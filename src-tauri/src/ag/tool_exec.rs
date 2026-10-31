//! PI-AG-08B · 工具执行（只读：list_docs / read_doc / search_text / list_blocks / list_tables /
//! list_images；受控复制：import_image_asset）。
//!
//! 经 WS 在 **active workspace** 内执行（X-FORBID-004 边界守卫），结果回注模型（SM-AG-02 ResultInjecting）。
//! 文档内容读取统一走 AgentEditableSnapshot；权限/注册校验归 tool_registry::validate_requested_tool
//! （BR-AG-STATE-002）。
//! 写意图工具 propose_edit 由 AG 回合专用路径生成 ChangeProposal→DE diff（X-FORBID-006），不进入本只读执行器。

use crate::ipc::IpcError;
use crate::types::{
    EvidenceItemV1, EvidenceLedgerV1, ProjectReportFindingV2, ProjectReportReadItemV2,
    ProjectReportUnreadItemV2, ProjectReportV2, PROJECT_REPORT_VERSION_V2,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/**
 * @GOV
 * codes:    BR-AG-STATE-002-DATA-AG-AG-TOOL-044, BR-SYS-DATA-008-DATA-AG-AG-TOOL-044
 * type:     DATA
 * chain:    AG-TOOL
 * rules:    BR-AG-STATE-002, BR-SYS-DATA-008
 * term_ref: TERM-AG-002, TERM-AG-014
 * boundary: in=ToolCallReceiptIdentityV1AndExecutionFactsAndArtifactReferences | out=ToolCallReceiptV1 | does_not=authorizeExecutionOrClaimAcceptanceOrSaveOrReplay | consumer=BR-AG-STATE-002
 */
pub(super) fn tool_call_receipt(
    identity: crate::types::ToolCallReceiptIdentityV1,
    validation_allowed: bool,
    success: bool,
    failure: Option<&IpcError>,
    references: Vec<crate::types::ToolCallReceiptReferenceV1>,
    evidence_ids: Vec<String>,
) -> crate::types::ToolCallReceiptV1 {
    use crate::types::*;
    let registered = super::tool_registry::lookup(&identity.tool_name);
    let read_only = registered
        .is_some_and(|tool| tool.permission == super::tool_registry::ToolPermission::ReadOnly);
    let execution_status = if !validation_allowed {
        ToolCallExecutionStatus::Rejected
    } else if success {
        ToolCallExecutionStatus::Succeeded
    } else {
        ToolCallExecutionStatus::Failed
    };
    let (completion_status, effect_status, next_action) = if !validation_allowed {
        (
            ToolCallCompletionStatus::Blocked,
            ToolCallEffectStatus::NotStarted,
            ToolCallNextAction::RespondBlocked,
        )
    } else if !success {
        if read_only || identity.tool_name == "edit_blocked" {
            (
                ToolCallCompletionStatus::Blocked,
                ToolCallEffectStatus::NotApplicable,
                if failure.is_some_and(|error| error.retriable) {
                    ToolCallNextAction::RetryExplicitly
                } else {
                    ToolCallNextAction::RespondBlocked
                },
            )
        } else {
            (
                ToolCallCompletionStatus::Blocked,
                ToolCallEffectStatus::Unknown,
                ToolCallNextAction::InspectEffect,
            )
        }
    } else if identity.tool_name == "edit_blocked" {
        (
            ToolCallCompletionStatus::Blocked,
            ToolCallEffectStatus::NotApplicable,
            ToolCallNextAction::RespondBlocked,
        )
    } else if matches!(
        identity.tool_name.as_str(),
        "propose_decision" | "plan_workspace_operation"
    ) && references.iter().any(|reference| {
        reference.kind == ToolCallReceiptReferenceKind::DecisionRequest && !reference.id.is_empty()
    }) {
        (
            ToolCallCompletionStatus::Waiting,
            ToolCallEffectStatus::NotStarted,
            ToolCallNextAction::AwaitUser,
        )
    } else if identity.tool_name == "propose_edit"
        && references.iter().any(|reference| {
            reference.kind == ToolCallReceiptReferenceKind::PendingDiff && !reference.id.is_empty()
        })
    {
        (
            ToolCallCompletionStatus::Waiting,
            ToolCallEffectStatus::ProposalCreated,
            ToolCallNextAction::AwaitUser,
        )
    } else if identity.tool_name == "import_image_asset" {
        (
            ToolCallCompletionStatus::Completed,
            ToolCallEffectStatus::ResourcePrepared,
            ToolCallNextAction::None,
        )
    } else if read_only {
        (
            ToolCallCompletionStatus::Completed,
            ToolCallEffectStatus::NotApplicable,
            ToolCallNextAction::None,
        )
    } else {
        // Missing adapter evidence is never upgraded to completed/saved from result text.
        (
            ToolCallCompletionStatus::Blocked,
            ToolCallEffectStatus::Unknown,
            ToolCallNextAction::InspectEffect,
        )
    };
    let failure = (!validation_allowed || !success).then(|| {
        let fallback = IpcError::new(
            if validation_allowed {
                "AG_TOOL_EXECUTION_FAILED"
            } else {
                "AG_TOOL_REJECTED"
            },
            "工具失败；没有可用的细分错误码",
            false,
            false,
        );
        let error = failure.unwrap_or(&fallback);
        ToolCallFailureV1 {
            code: error.code.clone(),
            retriable: error.retriable,
            reauth_required: error.reauth_required,
        }
    });
    ToolCallReceiptV1 {
        version: ToolCallReceiptVersion::V1,
        identity,
        execution_status,
        completion_status,
        effect_status,
        references,
        failure,
        evidence_ids,
        next_action,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingEvidenceItem {
    pub rel_path: String,
    pub source: String,
    pub coverage: String,
    pub version_or_hash: String,
}

fn snapshot_version(snapshot: &crate::ag::blocks::AgentEditableSnapshot) -> String {
    snapshot
        .content_hash
        .clone()
        .or_else(|| {
            snapshot
                .session_version
                .map(|version| format!("session:{version}"))
        })
        .or_else(|| {
            snapshot
                .baseline
                .map(|baseline| format!("baseline:{baseline}"))
        })
        .unwrap_or_else(|| crate::ag::context::sha256_text(&snapshot.content))
}

/**
 * @GOV
 * codes:    BR-AG-OBS-001-DATA-AG-AG-TOOL-043, BR-AG-DATA-008-DATA-AG-AG-TOOL-043, BR-SYS-DATA-008-DATA-AG-AG-TOOL-043
 * type:     DATA
 * chain:    AG-TURN, AG-TOOL
 * rules:    BR-AG-OBS-001, BR-AG-DATA-008, BR-SYS-DATA-008
 * term_ref: TERM-AG-020, TERM-AG-021, TERM-AG-023
 * boundary: in=VerifiedContextFrozenOrSuccessfulReadOnlyToolResult | out=CoverageDistinctPendingEvidenceItemsAndRuntimeMaterializedEvidenceLedger | does_not=recordFailedCrossWorkspaceOrModelDeclaredReadsOrUpgradeCoverage
 */
pub fn context_evidence(
    facts: &crate::types::AgentContextFacts,
    active_doc_path: Option<&str>,
    current_doc_prompt_coverage: Option<&str>,
) -> Vec<PendingEvidenceItem> {
    let documents = facts.ready_documents();
    let mut items = documents
        .iter()
        .map(|fact| PendingEvidenceItem {
            rel_path: fact.rel_path.clone(),
            source: fact.source.clone(),
            coverage: "session_metadata".to_string(),
            version_or_hash: fact.content_hash.clone(),
        })
        .collect::<Vec<_>>();
    if let (Some(path), Some(coverage)) = (active_doc_path, current_doc_prompt_coverage) {
        if let Some(fact) = documents.iter().find(|fact| fact.rel_path == path) {
            items.push(PendingEvidenceItem {
                rel_path: path.to_string(),
                source: fact.source.clone(),
                coverage: coverage.to_string(),
                version_or_hash: fact.content_hash.clone(),
            });
        }
    }
    items
}

pub fn successful_tool_evidence(
    ctx: &crate::ag::blocks::AgentReadContext,
    name: &str,
    args: &serde_json::Value,
    result: &str,
) -> Vec<PendingEvidenceItem> {
    let from_path = |path: &str, coverage: &str| {
        ctx.snapshot(path).ok().map(|snapshot| PendingEvidenceItem {
            rel_path: path.to_string(),
            source: snapshot.source.clone(),
            coverage: coverage.to_string(),
            version_or_hash: snapshot_version(&snapshot),
        })
    };
    // Read evidence binds the exact successful result, not a second disk read that may have changed.
    let from_returned_snapshot = |path: &str, coverage: &str| -> Option<PendingEvidenceItem> {
        let returned: serde_json::Value = serde_json::from_str(result).ok()?;
        let path = path.trim();
        if returned.get("rel_path")?.as_str()? != path {
            return None;
        }
        Some(PendingEvidenceItem {
            rel_path: path.to_string(),
            source: returned.get("source")?.as_str()?.to_string(),
            coverage: coverage.to_string(),
            version_or_hash: returned.get("version_hash")?.as_str()?.to_string(),
        })
    };
    match name {
        "read_doc" => args
            .get("path")
            .and_then(|value| value.as_str())
            .and_then(|path| {
                from_returned_snapshot(
                    path,
                    if args.get("blocks").is_some()
                        || serde_json::from_str::<serde_json::Value>(result)
                            .ok()
                            .is_some_and(|returned| {
                                returned.get("complete") == Some(&serde_json::Value::Bool(false))
                            })
                    {
                        "selected_blocks"
                    } else {
                        "full_document"
                    },
                )
            })
            .into_iter()
            .collect(),
        "list_blocks" => args
            .get("path")
            .and_then(|value| value.as_str())
            .and_then(|path| from_returned_snapshot(path, "structure_index"))
            .into_iter()
            .collect(),
        "list_tables" | "list_images" => args
            .get("path")
            .and_then(|value| value.as_str())
            .and_then(|path| from_path(path, "structure_index"))
            .into_iter()
            .collect(),
        "list_docs" => vec![PendingEvidenceItem {
            rel_path: args
                .get("path")
                .and_then(|value| value.as_str())
                .filter(|path| !path.trim().is_empty())
                .unwrap_or(".")
                .to_string(),
            source: "workspace_index".to_string(),
            coverage: "workspace_structure".to_string(),
            version_or_hash: crate::ag::context::sha256_text(result),
        }],
        "workspace_glob" => vec![PendingEvidenceItem {
            rel_path: ".".to_string(),
            source: "workspace_index".to_string(),
            coverage: "workspace_structure".to_string(),
            version_or_hash: crate::ag::context::sha256_text(result),
        }],
        "search_text" => serde_json::from_str::<Vec<serde_json::Value>>(result)
            .unwrap_or_default()
            .iter()
            .filter_map(|hit| hit.get("rel_path").and_then(|value| value.as_str()))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .filter_map(|path| from_path(path, "search_excerpt"))
            .collect(),
        _ => Vec::new(),
    }
}

pub fn materialize_evidence(
    workspace_id: &str,
    turn_id: &str,
    source_event_id: &str,
    seq: u64,
    identity_seed: &str,
    pending: Vec<PendingEvidenceItem>,
) -> Vec<EvidenceItemV1> {
    pending
        .into_iter()
        .enumerate()
        .map(|(index, item)| {
            let identity = format!(
                "{workspace_id}\n{turn_id}\n{identity_seed}\n{}\n{}\n{}\n{}\n{index}",
                item.rel_path, item.source, item.coverage, item.version_or_hash
            );
            EvidenceItemV1 {
                evidence_id: format!("evidence:{}", crate::ag::context::sha256_text(&identity)),
                workspace_id: workspace_id.to_string(),
                turn_id: turn_id.to_string(),
                rel_path: item.rel_path,
                source: item.source,
                coverage: item.coverage,
                version_or_hash: item.version_or_hash,
                source_event_id: source_event_id.to_string(),
                seq,
            }
        })
        .collect()
}

pub fn evidence_ids(
    workspace_id: &str,
    turn_id: &str,
    identity_seed: &str,
    pending: &[PendingEvidenceItem],
) -> Vec<String> {
    materialize_evidence(
        workspace_id,
        turn_id,
        "",
        0,
        identity_seed,
        pending.to_vec(),
    )
    .into_iter()
    .map(|item| item.evidence_id)
    .collect()
}

pub fn derive_project_report(
    turn_id: &str,
    requested_scope: Vec<String>,
    ledger: &EvidenceLedgerV1,
    model_findings: &[crate::ag::model_output::ModelFinalFindingV2],
) -> ProjectReportV2 {
    let mut grouped = BTreeMap::<(String, String, String), Vec<String>>::new();
    for item in &ledger.evidence_items {
        grouped
            .entry((
                item.rel_path.clone(),
                item.source.clone(),
                item.coverage.clone(),
            ))
            .or_default()
            .push(item.evidence_id.clone());
    }
    let read_items = grouped
        .into_iter()
        .map(
            |((rel_path, source, coverage), evidence_ids)| ProjectReportReadItemV2 {
                rel_path,
                source,
                coverage,
                evidence_ids,
            },
        )
        .collect::<Vec<_>>();
    let unread_items = requested_scope
        .iter()
        .filter(|scope| {
            !ledger
                .evidence_items
                .iter()
                .any(|item| item.rel_path == **scope || (*scope == "." && item.rel_path == "."))
        })
        .map(|scope| ProjectReportUnreadItemV2 {
            rel_path: scope.clone(),
            reason: "本轮没有成功读取该范围".to_string(),
        })
        .collect::<Vec<_>>();
    let by_id = ledger
        .evidence_items
        .iter()
        .map(|item| (item.evidence_id.as_str(), item))
        .collect::<BTreeMap<_, _>>();
    let mut findings = Vec::new();
    let mut uncertainties = Vec::new();
    for finding in model_findings {
        let referenced = finding
            .evidence_ids
            .iter()
            .filter_map(|id| by_id.get(id.as_str()).copied())
            .collect::<Vec<_>>();
        let all_known = referenced.len() == finding.evidence_ids.len();
        let content_supported = !referenced.is_empty()
            && referenced.iter().all(|item| {
                matches!(
                    item.coverage.as_str(),
                    "search_excerpt" | "selected_blocks" | "full_document"
                )
            });
        if all_known && content_supported {
            findings.push(ProjectReportFindingV2 {
                statement: finding.statement.clone(),
                evidence_ids: finding.evidence_ids.clone(),
                uncertainty: finding.uncertainty.clone(),
            });
        } else {
            uncertainties.push(format!(
                "结论“{}”引用了未知证据或仅有非正文覆盖，未登记为已证实 finding",
                finding.statement
            ));
        }
    }
    if ledger.evidence_items.is_empty() {
        uncertainties.push("本轮没有成功读取项目内容".to_string());
    }
    for item in &read_items {
        if matches!(
            item.coverage.as_str(),
            "session_metadata"
                | "workspace_structure"
                | "structure_index"
                | "search_excerpt"
                | "selected_blocks"
        ) {
            uncertainties.push(format!(
                "{} 仅覆盖 {}，不代表已读取全文",
                item.rel_path, item.coverage
            ));
        }
    }
    uncertainties.sort();
    uncertainties.dedup();
    ProjectReportV2 {
        version: PROJECT_REPORT_VERSION_V2.to_string(),
        turn_id: turn_id.to_string(),
        evidence_through_seq: ledger.through_seq,
        requested_scope,
        read_items,
        unread_items,
        findings,
        uncertainties,
    }
}

fn path_arg(args: &serde_json::Value, tool: &str) -> Result<String, IpcError> {
    args.get("path")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .ok_or_else(|| IpcError::new("AG_TOOL_ARG", &format!("{tool} 缺 path 参数"), false, false))
}

fn string_arg(args: &serde_json::Value, key: &str, tool: &str) -> Result<String, IpcError> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            IpcError::new(
                "AG_TOOL_ARG",
                &format!("{tool} 缺 {key} 参数"),
                false,
                false,
            )
        })
}

/**
 * @GOV
 * codes:    BR-AG-STATE-002-QUERY-AG-AG-TOOL-012, BR-AG-DATA-001-QUERY-AG-AG-TOOL-012, BR-WS-DATA-001-QUERY-AG-AG-TOOL-012
 * type:     QUERY
 * chain:    AG-TOOL
 * rules:    BR-AG-STATE-002, BR-AG-DATA-001, BR-WS-DATA-001
 * term_ref: TERM-ED-008, TERM-WS-001
 * boundary: in=ActiveBDocxLogicalStateAndWorkspaceProjectImages | out=ListImagesProjection(currentInstancesAndValidatedAssets) | consumer=BR-AG-STATE-002
 */
fn exec_list_images(
    ctx: &crate::ag::blocks::AgentReadContext,
    rel: &str,
) -> Result<String, IpcError> {
    let snapshot = ctx.snapshot(rel)?;
    let images = ctx
        .image_snapshots(rel)?
        .iter()
        .cloned()
        .map(|mut image| {
            let block_id = image.get("blockId").and_then(|value| value.as_str());
            let block_index = block_id.and_then(|id| {
                snapshot
                    .blocks
                    .iter()
                    .position(|block| block.block_id.as_deref() == Some(id))
            });
            let preview = |index: usize| -> Option<String> {
                snapshot
                    .blocks
                    .get(index)
                    .map(|block| block.text.trim().chars().take(160).collect::<String>())
                    .filter(|text| !text.is_empty())
            };
            let before_text = block_index.and_then(|index| (0..index).rev().find_map(preview));
            let after_text = block_index
                .and_then(|index| ((index + 1)..snapshot.blocks.len()).find_map(preview));
            if let Some(object) = image.as_object_mut() {
                object.insert(
                    "positionContext".to_string(),
                    serde_json::json!({
                        "blockIndex": block_index,
                        "precedingText": before_text,
                        "followingText": after_text,
                    }),
                );
            }
            image
        })
        .collect::<Vec<_>>();
    let mut assets = Vec::new();
    let mut rejected_candidates = 0usize;
    let catalog = crate::workspace::listing::load_project_image_catalog(ctx.root())?;
    for candidate in catalog.candidates {
        match crate::doc::image_media::inspect_project_image(ctx.root(), &candidate) {
            Ok(asset) => assets.push(serde_json::json!({
                "assetId": asset.asset_id,
                "sourcePath": asset.rel_path,
                "sourceHash": asset.content_hash,
                "mediaType": asset.media_type,
                "byteSize": asset.byte_size,
            })),
            Err(_) => rejected_candidates += 1,
        }
    }
    Ok(serde_json::to_string(&serde_json::json!({
        "relPath": snapshot.rel_path,
        "baseline": snapshot.baseline,
        "sessionVersion": snapshot.session_version,
        "contentHash": snapshot.content_hash,
        "images": images,
        "assets": assets,
        "imageRoots": catalog.image_roots,
        "catalogSource": catalog.config_source,
        "rejectedCandidates": rejected_candidates,
        "visualContentAvailable": false,
        "contextNotice": "图片内容未被视觉识别；precedingText/followingText 仅用于文档流定位，不代表图片画面。",
    }))
    .unwrap_or_else(|_| "{}".into()))
}

/// 执行白名单资源准备工具。普通创建、删除、重命名与移动必须走 WorkspaceOperationPlan，不能进入此执行器。
pub fn exec_structure_tool(
    root: &Path,
    name: &str,
    args: &serde_json::Value,
) -> Result<String, IpcError> {
    match name {
        "import_image_asset" => {
            let source_path = string_arg(args, "source_path", "import_image_asset")?;
            let source = crate::workspace::resolve_in_workspace(root, &source_path)?;
            let asset = crate::doc::image_media::import_project_image(root, &source)?;
            Ok(serde_json::to_string(&serde_json::json!({
                "operation": "import_image_asset",
                "copiedOrReused": true,
                "sourceFilePreserved": true,
                "assetId": asset.asset_id,
                "sourcePath": asset.rel_path,
                "sourceHash": asset.content_hash,
                "mediaType": asset.media_type,
                "byteSize": asset.byte_size,
                "summary": "图片已复制或复用到受控项目图片目录；源文件保持不变。请重新调用 list_images 后提出 image_ops 修改建议。"
            }))
            .unwrap_or_else(|_| "图片已准备".into()))
        }
        other => Err(IpcError::new(
            "AG_TOOL_UNKNOWN",
            &format!("结构工具执行器不支持: {other}"),
            false,
            false,
        )),
    }
}

pub fn exec_edit_blocked(args: &serde_json::Value) -> Result<String, IpcError> {
    let code = string_arg(args, "code", "edit_blocked")?;
    let reason = string_arg(args, "reason", "edit_blocked")?;
    if code.len() > 80 || reason.len() > 1000 {
        return Err(IpcError::new(
            "AG_TOOL_ARG",
            "edit_blocked 的 code/reason 过长",
            false,
            false,
        ));
    }
    Ok(serde_json::json!({
        "status": "edit_blocked",
        "code": code,
        "reason": reason,
        "documentModified": false,
    })
    .to_string())
}

/// 执行只读工具，返回回注模型的结果文本（JSON 序列化）。`ctx` 持有 active workspace 根与打开文档
/// LogicalState 覆盖层（X-FORBID-004）。
fn read_version(snapshot: &crate::ag::blocks::AgentEditableSnapshot) -> String {
    crate::ag::context::sha256_text(
        &serde_json::to_string(snapshot).expect("snapshot serialization"),
    )
}

/// A page token binds the next offset to the exact document identity and snapshot.
/// It is a continuation token, not an authorization credential.
fn next_read_cursor(rel_path: &str, offset: usize, version: &str) -> String {
    format!(
        "v1:{offset}:{version}:{}",
        crate::ag::context::sha256_text(rel_path)
    )
}

fn decode_read_cursor(
    args: &serde_json::Value,
    rel_path: &str,
) -> Result<Option<(usize, String)>, IpcError> {
    let Some(value) = args.get("cursor") else {
        return Ok(None);
    };
    let token = value.as_str().ok_or_else(|| {
        IpcError::new(
            "AG_READ_CURSOR_INVALID",
            "分页 cursor 必须是上一页返回的 next_cursor。",
            false,
            false,
        )
    })?;
    let parts = token.split(':').collect::<Vec<_>>();
    if parts.len() != 4
        || parts[0] != "v1"
        || parts[1].parse::<usize>().is_err()
        || parts[2].len() != 64
        || !parts[2].bytes().all(|byte| byte.is_ascii_hexdigit())
        || parts[3] != crate::ag::context::sha256_text(rel_path)
    {
        return Err(IpcError::new(
            "AG_READ_CURSOR_INVALID",
            "分页 cursor 无效或不属于此文档；请从第一页重新读取。",
            false,
            false,
        ));
    }
    Ok(Some((
        parts[1].parse().expect("checked offset"),
        parts[2].to_owned(),
    )))
}

fn read_page_offset(args: &serde_json::Value, rel_path: &str) -> Result<usize, IpcError> {
    match decode_read_cursor(args, rel_path)? {
        Some((offset, _)) => Ok(offset),
        None => page_integer(args, "offset", 0, usize::MAX),
    }
}

fn verify_read_version(
    args: &serde_json::Value,
    snapshot: &crate::ag::blocks::AgentEditableSnapshot,
) -> Result<(), IpcError> {
    if let Some((_, expected)) = decode_read_cursor(args, &snapshot.rel_path)? {
        if expected != read_version(snapshot) {
            return Err(IpcError::new(
                "AG_READ_VERSION_STALE",
                "文档版本已变化，请从当前版本重新读取，不得拼接旧页。",
                true,
                false,
            ));
        }
    }
    if let Some(expected) = args.get("expected_hash") {
        if expected.as_str() != Some(read_version(snapshot).as_str()) {
            return Err(IpcError::new(
                "AG_READ_VERSION_STALE",
                "文档版本已变化，请从当前版本重新读取，不得拼接旧页。",
                true,
                false,
            ));
        }
    }
    Ok(())
}

fn page_integer(
    args: &serde_json::Value,
    key: &str,
    default: usize,
    max: usize,
) -> Result<usize, IpcError> {
    match args.get(key) {
        None => Ok(default),
        Some(value) => value
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .filter(|n| *n <= max)
            .ok_or_else(|| {
                IpcError::new(
                    "AG_TOOL_ARG",
                    "分页参数必须为范围内的非负整数",
                    false,
                    false,
                )
            }),
    }
}

/// Snapshot-independent read contracts must agree at preflight and direct execution.
/// Do not repair arguments here: in particular, never invent a pagination version.
pub(super) fn validate_read_arguments(
    name: &str,
    args: &serde_json::Value,
) -> Result<(), IpcError> {
    if matches!(
        name,
        "read_doc" | "list_blocks" | "list_tables" | "list_images"
    ) {
        path_arg(args, name)?;
    }
    if args
        .get("expected_hash")
        .is_some_and(|hash| hash.as_str().is_none_or(|hash| hash.trim().is_empty()))
    {
        return Err(IpcError::new(
            "AG_TOOL_ARG",
            "expected_hash 必须是非空版本字符串；首次读取请省略，不得填空值。",
            false,
            false,
        ));
    }
    if args.get("cursor").is_some() {
        if args.get("offset").is_some() || args.get("expected_hash").is_some() {
            return Err(IpcError::new(
                "AG_TOOL_ARG",
                "cursor 不得与 offset/expected_hash 混用。",
                false,
                false,
            ));
        }
        if matches!(name, "read_doc" | "list_blocks") {
            let rel_path = path_arg(args, name)?;
            decode_read_cursor(args, &rel_path)?;
        }
    }
    if name == "read_doc"
        && args.get("blocks").is_some_and(|blocks| {
            blocks.as_array().is_none_or(|blocks| {
                blocks.is_empty()
                    || blocks
                        .iter()
                        .any(|block| block.as_str().is_none_or(|block| block.trim().is_empty()))
            })
        })
    {
        return Err(IpcError::new(
            "AG_TOOL_ARG",
            "read_doc.blocks 必须是非空块标识字符串数组；首次整篇读取请省略 blocks。",
            false,
            false,
        ));
    }
    if name == "read_doc" {
        let view = args.get("view").and_then(|value| value.as_str());
        if args.get("view").is_some() && !matches!(view, Some("text" | "structure")) {
            return Err(IpcError::new(
                "AG_TOOL_ARG",
                "read_doc.view 仅支持 text 或 structure。",
                false,
                false,
            ));
        }
        let structure = view == Some("structure") || args.get("blocks").is_some();
        if (view == Some("text") && args.get("blocks").is_some())
            || (structure
                && (args.get("offset").is_some()
                    || args.get("cursor").is_some()
                    || args.get("limit_chars").is_some()))
        {
            return Err(IpcError::new(
                "AG_TOOL_ARG",
                "正文分页与编辑结构读取不能混用；blocks/structure 不接受 cursor/offset/limit_chars。",
                false,
                false,
            ));
        }
        if !structure {
            let offset = read_page_offset(args, &path_arg(args, name)?)?;
            if page_integer(args, "limit_chars", 10_000, 10_000)? == 0 {
                return Err(IpcError::new(
                    "AG_TOOL_ARG",
                    "limit_chars 必须在 1..10000 内。",
                    false,
                    false,
                ));
            }
            if offset > 0 && args.get("cursor").is_none() && args.get("expected_hash").is_none() {
                return Err(IpcError::new("AG_READ_PAGE_VERSION_REQUIRED", "正文非首页缺少 expected_hash；请使用上一页返回的 next_offset 和 version_hash。offset 是阅读文本Unicode字符位置，不是块号。", false, false));
            }
        }
    }
    if name == "list_blocks" {
        let offset = read_page_offset(args, &path_arg(args, name)?)?;
        let limit = page_integer(args, "limit", 20, 50)?;
        if limit == 0 {
            return Err(IpcError::new(
                "AG_TOOL_ARG",
                "list_blocks.limit 必须在 1..50 内。",
                false,
                false,
            ));
        }
        if offset > 0 && args.get("cursor").is_none() && args.get("expected_hash").is_none() {
            return Err(IpcError::new("AG_READ_PAGE_VERSION_REQUIRED", "非首页分页缺少 expected_hash。请先调用 list_blocks(path, offset=0) 取得 version_hash 和 next_offset，再使用返回的 next_offset 与 version_hash 读取下一页；offset 不是块编号。", false, false));
        }
    }
    Ok(())
}

fn serialized_result_chars(result: &str) -> usize {
    serde_json::to_string(result)
        .expect("String serialization")
        .chars()
        .count()
}

/**
 * @GOV
 * codes: BR-AG-DATA-005-DATA-AG-AG-TOOL-059, BR-AG-OBS-001-DATA-AG-AG-TOOL-059
 * type: DATA
 * chain: AG-TOOL
 * rules: BR-AG-DATA-005, BR-AG-OBS-001
 * boundary: in=AgentEditableSnapshotAndValidatedReadingCursor | out=SerializedReadingPageWithVersionRangeAndNextCursor | does_not=TruncateEditNodesOrUpgradePartialCoverage
 */
fn reading_page(
    snapshot: &crate::ag::blocks::AgentEditableSnapshot,
    args: &serde_json::Value,
) -> Result<String, IpcError> {
    let text = crate::ag::blocks::reading_text(snapshot);
    let chars: Vec<char> = text.chars().collect();
    let offset = read_page_offset(args, &snapshot.rel_path)?;
    if offset > chars.len() {
        return Err(IpcError::new(
            "AG_READ_PAGE_OUT_OF_RANGE",
            "正文游标超出当前阅读投影；请从第一页重新读取，不得猜测游标。",
            true,
            false,
        ));
    }
    let limit = page_integer(args, "limit_chars", 10_000, 10_000)?;
    let mut page = serde_json::json!({
        "rel_path": snapshot.rel_path, "editable_kind": snapshot.editable_kind,
        "source": snapshot.source, "source_content_format": snapshot.content_format,
        "content_format": "text", "view": "text", "baseline": snapshot.baseline,
        "session_version": snapshot.session_version, "content_hash": snapshot.content_hash,
        "version_hash": read_version(snapshot), "offset": offset, "total_chars": chars.len(),
    });
    let mut render = |end: usize| {
        page["content"] = chars[offset..end].iter().collect::<String>().into();
        page["end_offset"] = end.into();
        page["next_offset"] = if end < chars.len() {
            end.into()
        } else {
            serde_json::Value::Null
        };
        page["next_cursor"] = if end < chars.len() {
            next_read_cursor(&snapshot.rel_path, end, &read_version(snapshot)).into()
        } else {
            serde_json::Value::Null
        };
        page["complete"] = (offset == 0 && end == chars.len()).into();
        page["has_more"] = (end < chars.len()).into();
        page.to_string()
    };
    let max_end = offset.saturating_add(limit).min(chars.len());
    let mut low = offset;
    let mut high = max_end;
    let mut best = None;
    // Bound the actual escaped result, not an assumed tokens/characters ratio.
    while low <= high {
        let end = low + (high - low) / 2;
        let candidate = render(end);
        if serialized_result_chars(&candidate)
            <= super::prompt_runtime::TOOL_RESULT_MAX_SERIALIZED_CHARS
        {
            best = Some((end, candidate));
            low = end + 1;
        } else if end == 0 {
            break;
        } else {
            high = end - 1;
        }
    }
    match best {
        Some((end, result)) if end > offset || offset == chars.len() => Ok(result),
        _ => Err(IpcError::new(
            "AG_TOOL_RESULT_TOO_LARGE",
            "正文页元数据已超过安全预算，无法产生前进页；本次不登记已读证据。",
            true,
            false,
        )),
    }
}

/**
 * @GOV
 * codes: BR-AG-DATA-005-GUARD-AG-AG-TOOL-057, BR-AG-OBS-001-GUARD-AG-AG-TOOL-057
 * type: GUARD
 * chain: AG-TOOL
 * rules: BR-AG-DATA-005, BR-AG-OBS-001
 * boundary: in=AuthorizedReadOnlyToolArgumentsAndAgentReadContext | out=CompleteBoundedToolResultOrExplicitFailure | does_not=TruncateStructuredBlocksOrClaimFailedReadsAsEvidence
 */
pub fn exec_read_only(
    ctx: &crate::ag::blocks::AgentReadContext,
    name: &str,
    args: &serde_json::Value,
) -> Result<String, IpcError> {
    validate_read_arguments(name, args)?;
    let result = exec_read_only_unbounded(ctx, name, args)?;
    let result_chars = serialized_result_chars(&result);
    if matches!(name, "read_doc" | "list_blocks") {
        eprintln!(
            "[AG-AG-TOOL] read_budget: {}",
            serde_json::json!({
                "toolName": name, "requestFingerprint": crate::ag::context::sha256_text(&args.to_string()),
                "resultChars": result_chars, "resultLimitChars": super::prompt_runtime::TOOL_RESULT_MAX_SERIALIZED_CHARS,
                "exceeded": result_chars > super::prompt_runtime::TOOL_RESULT_MAX_SERIALIZED_CHARS,
            })
        );
    }
    if result_chars > super::prompt_runtime::TOOL_RESULT_MAX_SERIALIZED_CHARS {
        return Err(IpcError::new("AG_TOOL_RESULT_TOO_LARGE", "结构或列表结果超过单次预算，未返回截断正文、JSON或编辑节点。阅读可改用 read_doc(view=text) 正文分页；编辑用 list_blocks 定位后缩小 blocks；本次不登记已读证据。", true, false));
    }
    Ok(result)
}

/**
 * @GOV
 * codes: BR-AG-STATE-002-GUARD-AG-AG-TOOL-045, BR-SYS-STATE-007-GUARD-AG-AG-TOOL-045
 * type: GUARD
 * chain: AG-TOOL
 * rules: BR-AG-STATE-002, BR-SYS-STATE-007
 * boundary: in=AuthorizedReadOnlyToolCallAndFrozenAgentReadContextAndCancellationSignal | out=ReadOnlyToolResultOrDeadlineCancellationWorkerFailure | does_not=ExecuteWritesOrInjectLateResultsOrRetryAutomatically | consumer=BR-AG-STATE-002
 */
pub(super) async fn exec_read_only_with_deadline(
    ctx: crate::ag::blocks::AgentReadContext,
    name: String,
    args: serde_json::Value,
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Result<String, IpcError> {
    if !super::tool_registry::lookup(&name)
        .is_some_and(|tool| tool.permission == super::tool_registry::ToolPermission::ReadOnly)
    {
        return Err(IpcError::new(
            "AG_TOOL_DEADLINE_UNSAFE",
            "写操作不可进入可丢弃结果的只读执行器",
            false,
            false,
        ));
    }
    await_read_operation(
        move || exec_read_only(&ctx, &name, &args),
        cancelled,
        std::time::Duration::from_secs(30),
    )
    .await
}

async fn await_read_operation(
    operation: impl FnOnce() -> Result<String, IpcError> + Send + 'static,
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
    timeout: std::time::Duration,
) -> Result<String, IpcError> {
    use std::sync::atomic::Ordering;
    if cancelled.load(Ordering::Acquire) {
        return Err(IpcError::new(
            "AG_TURN_CANCELLED",
            "当前回合已取消",
            false,
            false,
        ));
    }
    let deadline = std::time::Instant::now() + timeout;
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    tauri::async_runtime::spawn_blocking(move || {
        // 取消/期限结束后接收端销毁，迟到结果不进入事件或模型回注。
        let _ = sender.send(operation());
    });
    tauri::async_runtime::spawn_blocking(move || loop {
        if cancelled.load(Ordering::Acquire) {
            return Err(IpcError::new(
                "AG_TURN_CANCELLED",
                "当前回合已取消",
                false,
                false,
            ));
        }
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            return Err(IpcError::new(
                "AG_TOOL_TIMEOUT",
                "只读工具等待超过期限，未返回成功证据；可显式重试",
                true,
                false,
            ));
        }
        match receiver.recv_timeout(remaining.min(std::time::Duration::from_millis(20))) {
            Ok(result) => return result,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                return Err(IpcError::new(
                    "AG_TOOL_WORKER_FAILED",
                    "只读执行线程异常结束",
                    true,
                    false,
                ))
            }
        }
    })
    .await
    .map_err(|_| IpcError::new("AG_TOOL_WORKER_FAILED", "只读等待线程异常结束", true, false))?
}

fn exec_read_only_unbounded(
    ctx: &crate::ag::blocks::AgentReadContext,
    name: &str,
    args: &serde_json::Value,
) -> Result<String, IpcError> {
    match name {
        "list_docs" => {
            // AG-WS-LAYERED-01：无 path → L0 概览（豁免时全量）；path → L1 一层展开。
            // 树枚举只含索引（路径/类型/计数），永不携带正文；正文仅经 read_doc。
            let rel_dir = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            if rel_dir.trim().is_empty() {
                crate::workspace::listing::list_docs_overview_json(ctx.root())
            } else {
                crate::workspace::listing::list_docs_children_json(ctx.root(), rel_dir)
            }
        }
        "workspace_glob" => {
            // AG-STRATEGY-01：按文件名通配模式定位文档（只读，不含正文）。
            let pattern = args.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
            if pattern.trim().is_empty() {
                return Err(IpcError::new(
                    "WS_GLOB_PATTERN_MISSING",
                    "workspace_glob 缺少 pattern 参数（如 pattern=\"*寓言*.txt\"）",
                    false,
                    false,
                ));
            }
            let hits = crate::workspace::listing::glob_files(ctx.root(), pattern)?;
            Ok(serde_json::to_string(&hits).unwrap_or_else(|_| "[]".into()))
        }
        "search_text" => {
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            let needle = query.trim().to_lowercase();
            let mut hits: Vec<serde_json::Value> = Vec::new();
            if !needle.is_empty() {
                for entry in crate::workspace::listing::list_docs(ctx.root())? {
                    if entry.is_dir {
                        continue;
                    }
                    let Ok(snapshot) = ctx.snapshot(&entry.rel_path) else {
                        continue;
                    };
                    for block in snapshot.blocks {
                        if !block.text.to_lowercase().contains(&needle) {
                            continue;
                        }
                        let preview: String = block.text.trim().chars().take(160).collect();
                        hits.push(serde_json::json!({
                            "rel_path": entry.rel_path.clone(),
                            "block_index": block.block_index,
                            "block_id": block.block_id,
                            "preview": preview,
                        }));
                        if hits.len() >= 200 {
                            break;
                        }
                    }
                    if hits.len() >= 200 {
                        break;
                    }
                }
            }
            Ok(serde_json::to_string(&hits).unwrap_or_else(|_| "[]".into()))
        }
        "read_doc" => {
            let rel = path_arg(args, "read_doc")?;
            let snapshot = ctx.snapshot(&rel)?;
            verify_read_version(args, &snapshot)?;
            if args.get("blocks").is_none()
                && args.get("view").and_then(|value| value.as_str()) != Some("structure")
            {
                return reading_page(&snapshot, args);
            }
            // AG-CTX-01：read_doc 支持 blocks 范围参数（可选）——只返回指定块的完整投影
            // （含 node_json，供 block_ops 提案从 node_json 复制节点构造 afterNode）；
            // 省略 blocks 参数时只返回完整 content 与元数据，不重复块正文。轻量索引走 list_blocks。
            if let Some(requested) = args.get("blocks") {
                let blocks = requested.as_array().ok_or_else(|| {
                    IpcError::new("AG_TOOL_ARG", "blocks 必须是非空字符串数组", false, false)
                })?;
                let wanted: Vec<&str> = blocks.iter().filter_map(|value| value.as_str()).collect();
                if wanted.is_empty()
                    || wanted.len() != blocks.len()
                    || wanted.iter().any(|s| s.is_empty())
                {
                    return Err(IpcError::new(
                        "AG_TOOL_ARG",
                        "read_doc blocks 参数必须是块 id/索引数组且不能为空",
                        false,
                        false,
                    ));
                }
                let filtered = snapshot
                    .blocks
                    .iter()
                    .filter(|block| {
                        let id = block.block_id.as_deref().unwrap_or("");
                        wanted.contains(&id)
                            || wanted.contains(&block.block_index.to_string().as_str())
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                if wanted.iter().any(|wanted| {
                    !filtered.iter().any(|b| {
                        b.block_id.as_deref() == Some(*wanted)
                            || b.block_index.to_string() == *wanted
                    })
                }) {
                    return Err(IpcError::new(
                        "AG_READ_BLOCK_NOT_FOUND",
                        "请求的块不存在；请重新取得 list_blocks，不得将空结果视为已读。",
                        true,
                        false,
                    ));
                }
                let mut slim = serde_json::to_value(&snapshot).unwrap_or(serde_json::Value::Null);
                if let Some(object) = slim.as_object_mut() {
                    object.insert("version_hash".to_string(), read_version(&snapshot).into());
                    object["blocks"] = serde_json::to_value(&filtered)
                        .unwrap_or(serde_json::Value::Array(Vec::new()));
                    // 只返回目标块时省略整篇 content（块投影已含 text/node_json）。
                    object.remove("content");
                }
                return Ok(slim.to_string());
            }
            // A full read carries one complete body. Structured blocks are requested explicitly.
            let mut full = serde_json::to_value(&snapshot).expect("snapshot serialization");
            full.as_object_mut()
                .expect("snapshot object")
                .remove("blocks");
            full["version_hash"] = read_version(&snapshot).into();
            Ok(full.to_string())
        }
        "list_blocks" => {
            let rel = path_arg(args, "list_blocks")?;
            let snapshot = ctx.snapshot(&rel)?;
            verify_read_version(args, &snapshot)?;
            let offset = read_page_offset(args, &snapshot.rel_path)?;
            let limit = page_integer(args, "limit", 20, 50)?;
            if offset > snapshot.blocks.len() {
                return Err(IpcError::new("AG_READ_PAGE_OUT_OF_RANGE", "分页起点超出当前块目录；请从第一页重新取得 next_cursor，不得猜测页码或块编号。", true, false));
            }
            let total = snapshot.blocks.len();
            // AG-CTX-01：list_blocks 轻量投影——只返回块目录（无 node_json/全文/marks/attrs），
            // 结构性压缩（Claude Code Tool Result Budget 同思路），模型提案时经 read_doc(blocks) 按需取完整块。
            let directory = snapshot
                .blocks
                .iter()
                .skip(offset)
                .take(limit)
                .map(|block| {
                    let mut entry = serde_json::Map::new();
                    entry.insert(
                        "block_id".to_string(),
                        serde_json::Value::String(block.block_id.clone().unwrap_or_default()),
                    );
                    entry.insert(
                        "block_index".to_string(),
                        serde_json::json!(block.block_index),
                    );
                    if let Some(node_type) = &block.node_type {
                        entry.insert(
                            "node_type".to_string(),
                            serde_json::Value::String(node_type.clone()),
                        );
                    }
                    entry.insert(
                        "role".to_string(),
                        serde_json::Value::String(block.role.clone()),
                    );
                    if let Some(parent) = &block.parent_block_id {
                        entry.insert(
                            "parent_block_id".to_string(),
                            serde_json::Value::String(parent.clone()),
                        );
                    }
                    entry.insert("depth".to_string(), serde_json::json!(block.depth));
                    let text: String = block.text.chars().take(40).collect();
                    entry.insert("text".to_string(), serde_json::Value::String(text));
                    if let Some(fingerprint) = &block.fingerprint {
                        entry.insert(
                            "fingerprint".to_string(),
                            serde_json::Value::String(fingerprint.clone()),
                        );
                    }
                    entry.insert(
                        "capabilities".to_string(),
                        serde_json::Value::Array(
                            block
                                .capabilities
                                .iter()
                                .map(|c| serde_json::Value::String(c.clone()))
                                .collect(),
                        ),
                    );
                    serde_json::Value::Object(entry)
                })
                .collect::<Vec<_>>();
            let mut page = serde_json::json!({
                "rel_path": snapshot.rel_path,
                "editable_kind": snapshot.editable_kind,
                "content_format": snapshot.content_format,
                "source": snapshot.source,
                "version_hash": read_version(&snapshot),
                "offset": offset,
                "total_blocks": total,
                "next_offset": (offset + directory.len() < total).then_some(offset + directory.len()),
                "next_cursor": (offset + directory.len() < total).then(|| next_read_cursor(&snapshot.rel_path, offset + directory.len(), &read_version(&snapshot))),
                "complete": offset == 0 && directory.len() == total,
                "blocks": directory,
            });
            // limit is a maximum entry count, not a promise to overrun the byte
            // budget. Remove only whole directory entries and publish the real cursor.
            while serialized_result_chars(&page.to_string())
                > super::prompt_runtime::TOOL_RESULT_MAX_SERIALIZED_CHARS
            {
                let entries = page["blocks"].as_array_mut().expect("directory array");
                if entries.len() <= 1 {
                    break;
                }
                entries.pop();
                let end = offset + entries.len();
                page["next_offset"] = end.into();
                page["next_cursor"] =
                    next_read_cursor(&snapshot.rel_path, end, &read_version(&snapshot)).into();
                page["complete"] = false.into();
            }
            Ok(page.to_string())
        }
        "list_tables" => {
            let rel = path_arg(args, "list_tables")?;
            let snapshot = ctx.snapshot(&rel)?;
            if !matches!(
                snapshot.source.as_str(),
                "active_editor" | "active_editor_session"
            ) || snapshot.content_format != "html"
            {
                return Err(IpcError::new(
                    "AG_TABLE_EDIT_REQUIRES_ACTIVE_EDITOR",
                    "表格结构读取必须针对当前活动的 BDocx HTML 编辑器会话；原生预览或未打开文档不可编辑。",
                    false,
                    false,
                ));
            }
            let tables = ctx.table_snapshots(&rel)?;
            Ok(serde_json::to_string(&serde_json::json!({
                "rel_path": snapshot.rel_path,
                "session_version": snapshot.session_version,
                "content_hash": snapshot.content_hash,
                "tables": tables,
            }))
            .unwrap_or_else(|_| "{}".into()))
        }
        "list_images" => {
            let rel = path_arg(args, "list_images")?;
            exec_list_images(ctx, &rel)
        }
        other => Err(IpcError::new(
            "AG_TOOL_UNKNOWN",
            &format!(
                "只读工具执行器不支持: {other}（propose_edit 由 ChangeProposal→DE 专用路径处理）"
            ),
            false,
            false,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // covers: BR-AG-STATE-002
    // covers: BR-SYS-STATE-007
    #[test]
    fn read_deadline_cancellation_and_late_results_do_not_become_success() {
        use std::sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        };
        use std::time::Duration;
        tauri::async_runtime::block_on(async {
            let cancelled = Arc::new(AtomicBool::new(false));
            assert_eq!(
                await_read_operation(
                    || Ok("real result".into()),
                    cancelled.clone(),
                    Duration::from_secs(1)
                )
                .await
                .unwrap(),
                "real result"
            );
            let (started_tx, started_rx) = std::sync::mpsc::channel();
            let (finished_tx, finished_rx) = std::sync::mpsc::channel();
            let result = await_read_operation(
                move || {
                    started_tx.send(()).unwrap();
                    std::thread::sleep(Duration::from_millis(80));
                    finished_tx.send(()).unwrap();
                    Ok("late result".into())
                },
                cancelled.clone(),
                Duration::from_millis(10),
            )
            .await;
            started_rx.recv_timeout(Duration::from_secs(1)).unwrap();
            assert_eq!(result.unwrap_err().code, "AG_TOOL_TIMEOUT");
            finished_rx.recv_timeout(Duration::from_secs(1)).unwrap();
            let (started_tx, started_rx) = std::sync::mpsc::channel();
            let flag = cancelled.clone();
            let cancel = std::thread::spawn(move || {
                started_rx.recv_timeout(Duration::from_secs(1)).unwrap();
                flag.store(true, Ordering::Release);
            });
            let result = await_read_operation(
                move || {
                    started_tx.send(()).unwrap();
                    std::thread::sleep(Duration::from_millis(80));
                    Ok("cancelled result".into())
                },
                cancelled.clone(),
                Duration::from_secs(1),
            )
            .await;
            assert_eq!(result.unwrap_err().code, "AG_TURN_CANCELLED");
            cancel.join().unwrap();
            assert_eq!(
                await_read_operation(
                    || panic!("cancelled work must not start"),
                    cancelled,
                    Duration::from_secs(1)
                )
                .await
                .unwrap_err()
                .code,
                "AG_TURN_CANCELLED"
            );
            assert_eq!(
                await_read_operation(
                    || panic!("explicit worker failure"),
                    Arc::new(AtomicBool::new(false)),
                    Duration::from_secs(1)
                )
                .await
                .unwrap_err()
                .code,
                "AG_TOOL_WORKER_FAILED"
            );
        });
    }

    // covers: BR-AG-STATE-002, BR-SYS-DATA-008
    #[test]
    fn tool_receipts_distinguish_execution_effect_and_authorization() {
        use crate::types::*;
        let identity = |name: &str| ToolCallReceiptIdentityV1 {
            workspace_id: "ws".into(),
            session_id: "session".into(),
            turn_id: "turn".into(),
            tool_call_id: "call".into(),
            tool_name: name.into(),
            capability_id: "capability".into(),
            grant_id: "grant".into(),
            attempt: 1,
        };
        let failure = IpcError::new("AG_READ_FAILED", "PRIVATE_ERROR_CONTENT", true, false);
        let read = tool_call_receipt(
            identity("read_doc"),
            true,
            false,
            Some(&failure),
            vec![],
            vec![],
        );
        assert_eq!(read.execution_status, ToolCallExecutionStatus::Failed);
        assert_eq!(read.effect_status, ToolCallEffectStatus::NotApplicable);
        assert_eq!(read.next_action, ToolCallNextAction::RetryExplicitly);
        let serialized = serde_json::to_value(&read).unwrap();
        assert!(!serialized.to_string().contains("PRIVATE_ERROR_CONTENT"));
        assert_eq!(
            serde_json::from_value::<ToolCallReceiptV1>(serialized.clone()).unwrap(),
            read
        );
        let mut invalid = serialized.clone();
        invalid["version"] = serde_json::json!("tool-call-receipt-v2");
        assert!(serde_json::from_value::<ToolCallReceiptV1>(invalid).is_err());
        let mut invalid = serialized;
        invalid["effectStatus"] = serde_json::json!("saved");
        assert!(serde_json::from_value::<ToolCallReceiptV1>(invalid).is_err());
        let rejected =
            tool_call_receipt(identity("propose_edit"), false, false, None, vec![], vec![]);
        assert_eq!(rejected.execution_status, ToolCallExecutionStatus::Rejected);
        assert_eq!(rejected.effect_status, ToolCallEffectStatus::NotStarted);
        let failed_write = tool_call_receipt(
            identity("propose_edit"),
            true,
            false,
            Some(&failure),
            vec![],
            vec![],
        );
        assert_eq!(failed_write.effect_status, ToolCallEffectStatus::Unknown);
        assert_eq!(failed_write.next_action, ToolCallNextAction::InspectEffect);
        for (name, kind, expected_effect) in [
            (
                "propose_edit",
                ToolCallReceiptReferenceKind::PendingDiff,
                ToolCallEffectStatus::ProposalCreated,
            ),
            (
                "propose_decision",
                ToolCallReceiptReferenceKind::DecisionRequest,
                ToolCallEffectStatus::NotStarted,
            ),
        ] {
            let absent = tool_call_receipt(identity(name), true, true, None, vec![], vec![]);
            assert_eq!(absent.completion_status, ToolCallCompletionStatus::Blocked);
            let waiting = tool_call_receipt(
                identity(name),
                true,
                true,
                None,
                vec![ToolCallReceiptReferenceV1 {
                    kind,
                    id: "real-artifact".into(),
                }],
                vec![],
            );
            assert_eq!(waiting.completion_status, ToolCallCompletionStatus::Waiting);
            assert_eq!(waiting.effect_status, expected_effect);
            assert_eq!(waiting.next_action, ToolCallNextAction::AwaitUser);
        }
        let blocked = tool_call_receipt(identity("edit_blocked"), true, true, None, vec![], vec![]);
        assert_eq!(blocked.execution_status, ToolCallExecutionStatus::Succeeded);
        assert_eq!(blocked.completion_status, ToolCallCompletionStatus::Blocked);
    }

    // covers: BR-AG-DATA-005, BR-AG-OBS-001
    #[test]
    fn bounded_reads_preserve_whole_results_and_versioned_pages() {
        let dir = std::env::temp_dir().join(format!("ag-p05-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let body = (0..25)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(dir.join("a.txt"), &body).unwrap();
        let read_ctx = ctx(&dir);
        let call = |name: &str, args: serde_json::Value| -> serde_json::Value {
            serde_json::from_str(&exec_read_only(&read_ctx, name, &args).unwrap()).unwrap()
        };
        let full = call("read_doc", serde_json::json!({"path":"a.txt"}));
        assert_eq!(full["content"], body);
        assert!(full.get("blocks").is_none());
        let first = call("list_blocks", serde_json::json!({"path":"a.txt"}));
        assert_eq!(first["blocks"].as_array().unwrap().len(), 20);
        assert_eq!(first["total_blocks"], 25);
        assert_eq!(first["next_offset"], 20);
        assert!(first["next_cursor"].is_string());
        assert_eq!(first["complete"], false);
        assert_eq!(first["version_hash"], full["version_hash"]);
        let second = call(
            "list_blocks",
            serde_json::json!({"path":"a.txt","offset":20,"expected_hash":first["version_hash"]}),
        );
        assert_eq!(second["blocks"].as_array().unwrap().len(), 5);
        assert!(second["next_offset"].is_null());
        let cursor_second = call(
            "list_blocks",
            serde_json::json!({"path":"a.txt","cursor":first["next_cursor"]}),
        );
        assert_eq!(cursor_second["blocks"], second["blocks"]);
        assert!(cursor_second["next_cursor"].is_null());
        std::fs::write(dir.join("b.txt"), "different").unwrap();
        assert_eq!(
            exec_read_only(
                &read_ctx,
                "list_blocks",
                &serde_json::json!({"path":"b.txt","cursor":first["next_cursor"]})
            )
            .unwrap_err()
            .code,
            "AG_READ_CURSOR_INVALID"
        );
        assert_eq!(
            exec_read_only(
                &read_ctx,
                "list_blocks",
                &serde_json::json!({"path":"a.txt","cursor":first["next_cursor"],"offset":20})
            )
            .unwrap_err()
            .code,
            "AG_TOOL_ARG"
        );
        assert_eq!(second["complete"], false);
        let selected_args = serde_json::json!({"path":"a.txt","blocks":["24"],"expected_hash":first["version_hash"]});
        let selected = call("read_doc", selected_args.clone());
        assert_eq!(selected["blocks"][0]["text"], "line 24");
        assert!(selected.get("content").is_none());
        assert_eq!(
            successful_tool_evidence(&read_ctx, "read_doc", &selected_args, &selected.to_string())
                [0]
            .coverage,
            "selected_blocks"
        );
        assert_eq!(
            successful_tool_evidence(
                &read_ctx,
                "list_blocks",
                &serde_json::json!({"path":"a.txt"}),
                &first.to_string()
            )[0]
            .coverage,
            "structure_index"
        );
        for args in [
            serde_json::json!({"path":"a.txt","limit":0}),
            serde_json::json!({"path":"a.txt","limit":51}),
            serde_json::json!({"path":"a.txt","offset":-1}),
        ] {
            assert_eq!(
                exec_read_only(&read_ctx, "list_blocks", &args)
                    .unwrap_err()
                    .code,
                "AG_TOOL_ARG"
            );
        }
        assert_eq!(
            exec_read_only(
                &read_ctx,
                "list_blocks",
                &serde_json::json!({"path":"a.txt","offset":20})
            )
            .unwrap_err()
            .code,
            "AG_READ_PAGE_VERSION_REQUIRED"
        );
        assert_eq!(exec_read_only(&read_ctx,"list_blocks",&serde_json::json!({"path":"a.txt","offset":26,"expected_hash":first["version_hash"]})).unwrap_err().code,"AG_READ_PAGE_OUT_OF_RANGE");
        let end = call(
            "list_blocks",
            serde_json::json!({"path":"a.txt","offset":25,"expected_hash":first["version_hash"]}),
        );
        assert!(end["blocks"].as_array().unwrap().is_empty());
        assert!(end["next_offset"].is_null());
        std::fs::write(dir.join("a.txt"), format!("{body}\nchanged")).unwrap();
        assert_eq!(
            exec_read_only(
                &read_ctx,
                "list_blocks",
                &serde_json::json!({"path":"a.txt","cursor":first["next_cursor"]})
            )
            .unwrap_err()
            .code,
            "AG_READ_VERSION_STALE"
        );
        for blocks in [
            serde_json::json!([]),
            serde_json::json!([1]),
            serde_json::json!("1"),
        ] {
            assert_eq!(
                exec_read_only(
                    &read_ctx,
                    "read_doc",
                    &serde_json::json!({"path":"a.txt","blocks":blocks})
                )
                .unwrap_err()
                .code,
                "AG_TOOL_ARG"
            );
        }
        assert_eq!(
            exec_read_only(
                &read_ctx,
                "read_doc",
                &serde_json::json!({"path":"a.txt","blocks":["0","missing"]})
            )
            .unwrap_err()
            .code,
            "AG_READ_BLOCK_NOT_FOUND"
        );
        assert_eq!(
            successful_tool_evidence(&read_ctx, "read_doc", &selected_args, &selected.to_string())
                [0]
            .version_or_hash,
            first["version_hash"].as_str().unwrap()
        );
        for name in ["list_blocks", "read_doc"] {
            assert_eq!(
                exec_read_only(
                    &read_ctx,
                    name,
                    &serde_json::json!({"path":"a.txt","expected_hash":first["version_hash"]})
                )
                .unwrap_err()
                .code,
                "AG_READ_VERSION_STALE"
            );
        }
        std::fs::write(dir.join("huge.txt"), "\"\\汉".repeat(12_000)).unwrap();
        for args in [
            serde_json::json!({"path":"huge.txt","view":"structure"}),
            serde_json::json!({"path":"huge.txt","blocks":["0"]}),
        ] {
            assert_eq!(
                exec_read_only(&read_ctx, "read_doc", &args)
                    .unwrap_err()
                    .code,
                "AG_TOOL_RESULT_TOO_LARGE"
            );
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    fn ctx(root: &Path) -> crate::ag::blocks::AgentReadContext {
        crate::ag::blocks::AgentReadContext::new(root, Vec::new()).unwrap()
    }

    // covers: BR-AG-DATA-005, BR-AG-OBS-001
    #[test]
    fn directory_pages_fit_budget_and_return_actual_next_entry() {
        let dir =
            std::env::temp_dir().join(format!("ag-directory-budget-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let html = (0..80)
            .map(|index| {
                format!(
                    "<p data-block-id='{}-{index}'>目录条目{index}</p>",
                    "x".repeat(180)
                )
            })
            .collect::<String>();
        std::fs::write(dir.join("a.html"), html).unwrap();
        let read_ctx = ctx(&dir);
        let mut offset = 0;
        let mut hash: Option<String> = None;
        let mut indexes = Vec::new();
        loop {
            let mut args = serde_json::json!({"path":"a.html","offset":offset,"limit":50});
            if let Some(hash) = &hash {
                args["expected_hash"] = hash.clone().into();
            }
            let result = exec_read_only(&read_ctx, "list_blocks", &args).unwrap();
            assert!(
                serialized_result_chars(&result)
                    <= super::super::prompt_runtime::TOOL_RESULT_MAX_SERIALIZED_CHARS
            );
            let page: serde_json::Value = serde_json::from_str(&result).unwrap();
            let entries = page["blocks"].as_array().unwrap();
            assert!(!entries.is_empty() && entries.len() < 50);
            indexes.extend(
                entries
                    .iter()
                    .map(|block| block["block_index"].as_u64().unwrap()),
            );
            hash = Some(page["version_hash"].as_str().unwrap().into());
            if let Some(next) = page["next_offset"].as_u64() {
                assert_eq!(next as usize, offset + entries.len());
                offset = next as usize;
            } else {
                break;
            }
        }
        assert_eq!(indexes, (0..80).collect::<Vec<u64>>());
        std::fs::remove_dir_all(dir).unwrap();
    }

    // covers: BR-AG-DATA-005, BR-AG-OBS-001, BR-AG-STATE-002
    #[test]
    fn text_pages_advance_within_original_budget_without_false_full_read() {
        let dir = std::env::temp_dir().join(format!("ag-reading-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let body = "首行\n\"\\汉🙂\n".repeat(2400);
        std::fs::write(dir.join("long.txt"), &body).unwrap();
        let read_ctx = ctx(&dir);
        let mut offset = 0;
        let mut version: Option<String> = None;
        let mut cursor: Option<String> = None;
        let mut assembled = String::new();
        let mut pages = 0;
        loop {
            let mut args = serde_json::json!({"path":"long.txt"});
            if let Some(token) = &cursor {
                args["cursor"] = serde_json::Value::String(token.clone());
            }
            let result = exec_read_only(&read_ctx, "read_doc", &args).unwrap();
            assert!(
                serialized_result_chars(&result)
                    <= super::super::prompt_runtime::TOOL_RESULT_MAX_SERIALIZED_CHARS
            );
            let page: serde_json::Value = serde_json::from_str(&result).unwrap();
            assert_eq!(page["content_format"], "text");
            assert_eq!(page["complete"], false);
            assert_eq!(page["offset"], offset);
            assert_eq!(
                successful_tool_evidence(&read_ctx, "read_doc", &args, &result)[0].coverage,
                "selected_blocks"
            );
            assembled.push_str(page["content"].as_str().unwrap());
            if let Some(hash) = &version {
                assert_eq!(page["version_hash"], hash.as_str());
            } else {
                version = Some(page["version_hash"].as_str().unwrap().to_string());
            }
            pages += 1;
            assert!(pages < 20, "pagination must make progress");
            if let Some(next) = page["next_offset"].as_u64() {
                assert!(next as usize > offset);
                offset = next as usize;
                cursor = Some(page["next_cursor"].as_str().unwrap().to_owned());
            } else {
                assert_eq!(page["has_more"], false);
                assert!(page["next_cursor"].is_null());
                break;
            }
        }
        assert!(pages > 1);
        assert_eq!(
            assembled, body,
            "all Unicode and escaped characters must survive"
        );
        for args in [
            serde_json::json!({"path":"long.txt","offset":1}),
            serde_json::json!({"path":"long.txt","limit_chars":0}),
            serde_json::json!({"path":"long.txt","limit_chars":10001}),
            serde_json::json!({"path":"long.txt","view":"text","blocks":["0"]}),
            serde_json::json!({"path":"long.txt","view":"structure","offset":0}),
            serde_json::json!({"path":"long.txt","view":"unknown"}),
        ] {
            assert!(exec_read_only(&read_ctx, "read_doc", &args).is_err());
        }
        let args = serde_json::json!({"path":"long.txt","offset":999999,"expected_hash":version});
        assert_eq!(
            exec_read_only(&read_ctx, "read_doc", &args)
                .unwrap_err()
                .code,
            "AG_READ_PAGE_OUT_OF_RANGE"
        );
        let stale = serde_json::json!({"path":"long.txt","offset":offset,"expected_hash":version});
        std::fs::write(dir.join("long.txt"), "changed").unwrap();
        assert_eq!(
            exec_read_only(
                &read_ctx,
                "read_doc",
                &serde_json::json!({"path":"long.txt","cursor":cursor})
            )
            .unwrap_err()
            .code,
            "AG_READ_VERSION_STALE"
        );
        assert_eq!(
            exec_read_only(&read_ctx, "read_doc", &stale)
                .unwrap_err()
                .code,
            "AG_READ_VERSION_STALE"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    fn active_block(
        block_id: &str,
        block_index: usize,
        text: &str,
        role: &str,
    ) -> crate::ag::blocks::AgentEditableBlockSnapshot {
        crate::ag::blocks::AgentEditableBlockSnapshot {
            block_id: block_id.to_string(),
            block_index,
            block_type: "p".to_string(),
            node_type: "paragraph".to_string(),
            role: role.to_string(),
            parent_block_id: None,
            depth: 0,
            ordinal: block_index,
            text: text.to_string(),
            attrs: serde_json::json!({ "blockId": block_id }),
            marks: Vec::new(),
            node_json: serde_json::json!({
                "type": "paragraph",
                "attrs": { "blockId": block_id },
                "content": if text.is_empty() {
                    Vec::<serde_json::Value>::new()
                } else {
                    vec![serde_json::json!({ "type": "text", "text": text })]
                },
            }),
            fingerprint: format!("fp-{block_id}"),
            capabilities: if role == "content" {
                vec!["replace_node".to_string(), "style_ops".to_string()]
            } else {
                Vec::new()
            },
        }
    }

    #[test]
    fn unknown_tool_errors() {
        let root = Path::new("/tmp");
        let r = exec_read_only(&ctx(root), "delete_everything", &serde_json::Value::Null);
        assert_eq!(r.unwrap_err().code, "AG_TOOL_UNKNOWN");
    }

    #[test]
    fn read_doc_missing_path_errors() {
        let root = Path::new("/tmp");
        let r = exec_read_only(&ctx(root), "read_doc", &serde_json::json!({}));
        assert_eq!(r.unwrap_err().code, "AG_TOOL_ARG");
    }

    #[test]
    fn list_blocks_returns_unified_block_projection_for_text_file() {
        let dir = std::env::temp_dir().join(format!("ag-toolexec-blocks-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.md"), "一\n二").unwrap();
        let out = exec_read_only(
            &ctx(&dir),
            "list_blocks",
            &serde_json::json!({ "path": "a.md" }),
        )
        .unwrap();
        assert!(out.contains("\"block_index\":0"));
        assert!(out.contains("\"text\":\"一\""));
        // AG-CTX-01 F1：list_blocks 轻量投影不得包含 node_json/完整 marks。
        assert!(!out.contains("node_json"), "轻量投影应省略 node_json");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // covers: BR-AG-DATA-005
    // AG-CTX-01 F1：read_doc 支持 blocks 范围参数——只返回指定块完整投影，省略整篇 content。
    #[test]
    fn read_doc_blocks_scope_returns_only_selected_blocks_without_full_content() {
        let dir = std::env::temp_dir().join(format!("ag-toolexec-scope-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // 无 data-block-id 的文本文件：块索引 0/1。
        std::fs::write(dir.join("a.md"), "第一行\n第二行").unwrap();
        let out = exec_read_only(
            &ctx(&dir),
            "read_doc",
            &serde_json::json!({ "path": "a.md", "blocks": ["1"] }),
        )
        .unwrap();
        assert!(out.contains("第二行"), "应返回索引 1 的块: {out}");
        assert!(!out.contains("第一行"), "不应包含未选中的块 0");
        assert!(
            !out.contains("\"content\":"),
            "范围读取应省略整篇 content: {out}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn list_docs_on_temp_dir_returns_entries() {
        let dir = std::env::temp_dir().join(format!("ag-toolexec-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.md"), "# hi").unwrap();
        let out = exec_read_only(&ctx(&dir), "list_docs", &serde_json::Value::Null).unwrap();
        assert!(out.contains("a.md"), "list_docs 应含 a.md: {out}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // AG-WS-LAYERED-01：list_docs 无参数 → 大工作区返回概览（mode=overview，不含 entries/正文）；
    // path 参数 → 该目录一层直接子项（mode=children）。
    #[test]
    fn list_docs_layered_overview_and_children() {
        let dir = std::env::temp_dir().join(format!("ag-toolexec-layer-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("docs/sub")).unwrap();
        std::fs::write(dir.join("root.md"), "r").unwrap();
        for i in 0..35 {
            std::fs::write(dir.join(format!("docs/f{i}.txt")), "content").unwrap();
        }
        // 无 path：36 文件 > 30 豁免 → overview。
        let overview = exec_read_only(&ctx(&dir), "list_docs", &serde_json::json!({})).unwrap();
        let value: serde_json::Value = serde_json::from_str(&overview).unwrap();
        assert_eq!(value["mode"], "overview");
        assert_eq!(value["total_files"], 36);
        assert!(value.get("entries").is_none(), "概览不得携带全量 entries");
        assert!(value["top_level_dirs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rel_path"] == "docs" && d["file_count"] == 35));
        // path="docs"：一层直接子项（35 文件 + sub 目录，不递归）。
        let children = exec_read_only(
            &ctx(&dir),
            "list_docs",
            &serde_json::json!({ "path": "docs" }),
        )
        .unwrap();
        let child_value: serde_json::Value = serde_json::from_str(&children).unwrap();
        assert_eq!(child_value["mode"], "children");
        assert_eq!(child_value["total_items"], 36); // docs/f0..34 + docs/sub
        assert!(child_value["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["rel_path"] == "docs/sub" && i["file_count"] == 0));
        // 不存在目录 → 明确错误码。
        let err = exec_read_only(
            &ctx(&dir),
            "list_docs",
            &serde_json::json!({ "path": "nope" }),
        )
        .unwrap_err();
        assert_eq!(err.code, "WS_LIST_DIR_MISSING");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // AG-STRATEGY-01：workspace_glob 按模式返回匹配文件；缺 pattern → 明确错误。
    #[test]
    fn workspace_glob_lists_matching_files() {
        let dir = std::env::temp_dir().join(format!("ag-toolexec-glob-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("寓言")).unwrap();
        std::fs::write(dir.join("寓言/小马过河.txt"), "x").unwrap();
        std::fs::write(dir.join("readme.md"), "x").unwrap();
        let out = exec_read_only(
            &ctx(&dir),
            "workspace_glob",
            &serde_json::json!({ "pattern": "*寓言*" }),
        )
        .unwrap();
        assert!(
            out.contains("寓言/小马过河.txt"),
            "glob 应命中寓言文件: {out}"
        );
        assert!(!out.contains("readme.md"), "不应命中无关文件: {out}");
        let err = exec_read_only(&ctx(&dir), "workspace_glob", &serde_json::json!({})).unwrap_err();
        assert_eq!(err.code, "WS_GLOB_PATTERN_MISSING");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn search_text_prefers_open_logical_state() {
        let dir = std::env::temp_dir().join(format!("ag-toolexec-open-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.md"), "旧内容").unwrap();
        let ctx = crate::ag::blocks::AgentReadContext::new(
            &dir,
            vec![crate::ag::blocks::OpenDocSnapshot {
                rel_path: "a.md".to_string(),
                content: "最新 LogicalState".to_string(),
                baseline: None,
                edit_format: "text".to_string(),
                content_format: "text".to_string(),
                source: "open_editor_session".to_string(),
                dirty: true,
                session_version: 3,
                content_hash: "h3".to_string(),
                captured_at: 1,
                block_snapshots: Vec::new(),
                table_snapshots: Vec::new(),
                image_snapshots: Vec::new(),
            }],
        )
        .unwrap();
        let out =
            exec_read_only(&ctx, "search_text", &serde_json::json!({ "query": "最新" })).unwrap();
        assert!(out.contains("最新 LogicalState"));
        assert!(!out.contains("旧内容"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn list_tables_only_returns_active_html_bdocx_snapshot() {
        let dir = std::env::temp_dir().join(format!("ag-toolexec-tables-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let ctx = crate::ag::blocks::AgentReadContext::new(
            &dir,
            vec![crate::ag::blocks::OpenDocSnapshot {
                rel_path: "a.docx".to_string(),
                content: "<table data-table-id=\"table_1\"></table>".to_string(),
                baseline: Some(17),
                edit_format: "html".to_string(),
                content_format: "html".to_string(),
                source: "active_editor".to_string(),
                dirty: true,
                session_version: 7,
                content_hash: "h7".to_string(),
                captured_at: 1,
                block_snapshots: vec![active_block("table-context", 0, "", "container")],
                table_snapshots: vec![
                    serde_json::json!({ "tableId": "table_1", "rows": 1, "columns": 1 }),
                ],
                image_snapshots: Vec::new(),
            }],
        )
        .unwrap();
        let out = exec_read_only(
            &ctx,
            "list_tables",
            &serde_json::json!({ "path": "a.docx" }),
        )
        .unwrap();
        assert!(out.contains("table_1"));
        assert!(out.contains("session_version"));
        let selected: serde_json::Value = serde_json::from_str(
            &exec_read_only(
                &ctx,
                "read_doc",
                &serde_json::json!({"path":"a.docx","blocks":["table-context"]}),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            selected["blocks"][0]["node_json"],
            ctx.snapshot("a.docx").unwrap().blocks[0].node_json
        );
        assert!(selected.get("content").is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    // covers: BR-AG-STATE-002, BR-AG-DATA-001, BR-WS-DATA-001
    #[test]
    fn list_images_combines_active_image_snapshots_and_valid_project_assets() {
        let dir = std::env::temp_dir().join(format!("ag-toolexec-images-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("images")).unwrap();
        std::fs::write(dir.join("a.docx"), b"docx").unwrap();
        std::fs::write(
            dir.join("images/cover.png"),
            [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 1],
        )
        .unwrap();
        std::fs::write(dir.join("images/bad.png"), b"bad").unwrap();
        crate::workspace::listing::mark_bdocx(&dir, "a.docx").unwrap();
        let ctx = crate::ag::blocks::AgentReadContext::new(
            &dir,
            vec![crate::ag::blocks::OpenDocSnapshot {
                rel_path: "a.docx".to_string(),
                content: "<p data-block-id=\"before\">第一条说明</p><p data-block-id=\"b1\"><img src=\"media/a.png\"></p><p data-block-id=\"after\">第二条说明</p>".to_string(),
                baseline: Some(17),
                edit_format: "html".to_string(),
                content_format: "html".to_string(),
                source: "active_editor".to_string(),
                dirty: true,
                session_version: 8,
                content_hash: "h8".to_string(),
                captured_at: 1,
                block_snapshots: vec![
                    active_block("before", 0, "第一条说明", "content"),
                    active_block("b1", 1, "", "atom"),
                    active_block("after", 2, "第二条说明", "content"),
                ],
                table_snapshots: Vec::new(),
                image_snapshots: vec![serde_json::json!({
                    "imageId": "image_0",
                    "fingerprint": "fp_0",
                    "blockId": "b1",
                    "ordinal": 0,
                    "standalone": true
                })],
            }],
        )
        .unwrap();
        let output = exec_read_only(
            &ctx,
            "list_images",
            &serde_json::json!({ "path": "a.docx" }),
        )
        .unwrap();
        let value: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(value["sessionVersion"], 8);
        assert_eq!(value["images"][0]["imageId"], "image_0");
        assert_eq!(
            value["images"][0]["positionContext"]["precedingText"],
            "第一条说明"
        );
        assert_eq!(
            value["images"][0]["positionContext"]["followingText"],
            "第二条说明"
        );
        assert_eq!(value["visualContentAvailable"], false);
        assert_eq!(value["assets"].as_array().unwrap().len(), 1);
        assert!(value["assets"][0]["assetId"]
            .as_str()
            .is_some_and(|value| value.starts_with("asset_")));
        assert_eq!(value["assets"][0]["sourcePath"], "images/cover.png");
        assert_eq!(value["assets"][0]["sourceHash"].as_str().unwrap().len(), 64);
        assert_eq!(value["assets"][0]["mediaType"], "image/png");
        assert_eq!(value["imageRoots"], serde_json::json!(["images"]));
        assert_eq!(value["catalogSource"], "default");
        assert_eq!(value["rejectedCandidates"], 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn import_image_asset_copies_workspace_source_and_edit_blocked_is_structured() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("ag-import-image-{unique}"));
        std::fs::create_dir_all(dir.join("文档/多媒体")).unwrap();
        let source = dir.join("文档/多媒体/a.png");
        let png = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 1];
        std::fs::write(&source, png).unwrap();

        let result = exec_structure_tool(
            &dir,
            "import_image_asset",
            &serde_json::json!({ "source_path": "文档/多媒体/a.png" }),
        )
        .unwrap();
        let value: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(value["sourceFilePreserved"], true);
        assert!(value["sourcePath"]
            .as_str()
            .is_some_and(|path| path.starts_with("images/imported/")));
        assert_eq!(std::fs::read(&source).unwrap(), png);

        let blocked = exec_edit_blocked(&serde_json::json!({
            "code": "IMAGE_ASSET_UNAVAILABLE",
            "reason": "没有可用图片，请用户提供项目内来源。"
        }))
        .unwrap();
        let blocked: serde_json::Value = serde_json::from_str(&blocked).unwrap();
        assert_eq!(blocked["status"], "edit_blocked");
        assert_eq!(blocked["documentModified"], false);
        let _ = std::fs::remove_dir_all(dir);
    }

    // covers: BR-AG-OBS-001
    // covers: BR-AG-DATA-008
    // covers: BR-SYS-DATA-008
    #[test]
    fn project_report_is_derived_from_runtime_evidence_and_downgrades_unknown_claims() {
        let root = std::env::temp_dir().join(format!("ag-evidence-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("a.md"), "正文").unwrap();
        let read_ctx = ctx(&root);
        let cases = [
            ("list_docs", serde_json::json!({}), "workspace_structure"),
            (
                "workspace_glob",
                serde_json::json!({"pattern": "*.md"}),
                "workspace_structure",
            ),
            (
                "search_text",
                serde_json::json!({"query": "正文"}),
                "search_excerpt",
            ),
            (
                "list_blocks",
                serde_json::json!({"path": "a.md"}),
                "structure_index",
            ),
            (
                "read_doc",
                serde_json::json!({"path": "a.md", "blocks": ["0"]}),
                "selected_blocks",
            ),
            (
                "read_doc",
                serde_json::json!({"path": "a.md"}),
                "full_document",
            ),
        ];
        for (tool, args, expected_coverage) in cases {
            let result = exec_read_only(&read_ctx, tool, &args).unwrap();
            let evidence = successful_tool_evidence(&read_ctx, tool, &args, &result);
            assert!(
                evidence
                    .iter()
                    .all(|item| item.coverage == expected_coverage),
                "{tool} 必须保持独立 coverage"
            );
            assert!(!evidence.is_empty(), "{tool} 成功读取必须生成证据");
        }
        for tool in ["read_doc", "list_blocks"] {
            assert!(successful_tool_evidence(
                &read_ctx,
                tool,
                &serde_json::json!({"path":"a.md"}),
                "{}"
            )
            .is_empty());
        }
        assert!(successful_tool_evidence(
            &read_ctx,
            "propose_edit",
            &serde_json::json!({"path": "a.md"}),
            "{}"
        )
        .is_empty());

        let facts = crate::types::AgentContextFactsV1 {
            version: crate::types::AGENT_CONTEXT_FACTS_VERSION_V1.to_string(),
            workspace_id: "ws1".to_string(),
            turn_id: "t1".to_string(),
            active_doc: None,
            open_docs: vec![crate::types::AgentContextDocumentFactV1 {
                rel_path: "a.md".to_string(),
                source: "active_editor".to_string(),
                edit_format: "markdown".to_string(),
                content_format: "text".to_string(),
                dirty: true,
                baseline: Some("7".to_string()),
                session_version: 3,
                content_hash: "h3".to_string(),
            }],
            selections: Vec::new(),
            proposal_facts: Vec::new(),
        };
        let pending = context_evidence(
            &crate::types::AgentContextFacts::V1(facts),
            Some("a.md"),
            Some("full_document"),
        );
        let projected_ids = evidence_ids("ws1", "t1", "turn-event:t1:2", &pending);
        let items = materialize_evidence(
            "ws1",
            "t1",
            "turn-event:t1:2",
            2,
            "turn-event:t1:2",
            pending,
        );
        assert_eq!(
            projected_ids,
            items
                .iter()
                .map(|item| item.evidence_id.clone())
                .collect::<Vec<_>>()
        );
        let full_id = items
            .iter()
            .find(|item| item.coverage == "full_document")
            .unwrap()
            .evidence_id
            .clone();
        let ledger = EvidenceLedgerV1 {
            turn_id: "t1".to_string(),
            through_seq: 2,
            evidence_items: items,
        };
        let report = derive_project_report(
            "t1",
            vec!["a.md".to_string(), "missing.md".to_string()],
            &ledger,
            &[
                crate::ag::model_output::ModelFinalFindingV2 {
                    statement: "当前正文已读取".to_string(),
                    evidence_ids: vec![full_id],
                    uncertainty: "none".to_string(),
                },
                crate::ag::model_output::ModelFinalFindingV2 {
                    statement: "伪造结论".to_string(),
                    evidence_ids: vec!["evidence:unknown".to_string()],
                    uncertainty: "low".to_string(),
                },
            ],
        );
        assert_eq!(report.version, PROJECT_REPORT_VERSION_V2);
        assert_eq!(report.evidence_through_seq, 2);
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.unread_items[0].rel_path, "missing.md");
        assert!(report
            .uncertainties
            .iter()
            .any(|item| item.contains("伪造结论")));
        let _ = std::fs::remove_dir_all(root);
    }
}
