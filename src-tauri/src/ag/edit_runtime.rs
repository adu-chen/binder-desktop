//! Agent 内容修改建议到 PendingDiff 的执行编排。

use super::edit_image_runtime::image_ops_payload;
use super::edit_projection::build_edit_projection;
use super::edit_support::{
    capability_error_to_ipc, doc_type_of, gen_id, guard_propose_edit_target,
    is_active_editor_snapshot_source, is_docx_rel_path, now_millis, resolve_agent_edit_operation,
    style_ops_param,
};
use super::edit_table_runtime::table_ops_payload;
use super::edit_types::{AgentEditOperation, AgentProposeEditResult, EditProjectionDraft};
use crate::ipc::IpcError;
use crate::types::DiffProjection;
use crate::{active_ws, ag, doc, lock_db, workspace, AppState};

/// Adapter keeps document algorithms behind the owned execution port.
/**
 * @GOV
 * codes: BR-AG-STATE-002-EFFECT-AG-AG-TOOL-047, BR-AG-DATA-003-EFFECT-AG-AG-TOOL-047
 * type: EFFECT
 * chain: AG-TOOL, DE-DIFF
 * rules: BR-AG-STATE-002, BR-AG-DATA-003
 * boundary: in=OwnedAuthorizedProposalRequestAndFrozenAgentReadContext | out=BoundedProposalWaitAndPersistentCapabilityEffectReceipt | delegate=capability_runtime::execute,ag_propose_edit | does_not=ChangeDocumentAlgorithmsOrAcceptDiffOrSaveFile | consumer=BR-AG-STATE-002,BR-AG-DATA-003
 */
pub(super) async fn execute_proposal<R: tauri::Runtime>(
    state: AppState,
    app: tauri::AppHandle<R>,
    params: serde_json::Value,
    read_ctx: ag::blocks::AgentReadContext,
    current_rel: Option<String>,
    targets: ag::target_set::ExplicitEditTargetSet,
    identity: crate::types::ToolCallReceiptIdentityV1,
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Result<AgentProposeEditResult, IpcError> {
    let turn = identity.turn_id.clone();
    let call = identity.tool_call_id.clone();
    let effect_boundary = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let worker_boundary = effect_boundary.clone();
    super::capability_runtime::execute_with_effect_boundary(
        state,
        identity,
        cancelled,
        std::time::Duration::from_secs(30),
        Some(effect_boundary),
        move |state| {
            ag_propose_edit_with_effect_boundary(
                state,
                &app,
                &params,
                &read_ctx,
                current_rel.as_deref(),
                &targets,
                &turn,
                &call,
                Some(&worker_boundary),
            )
        },
        |result| {
            result
                .projections
                .iter()
                .map(|projection| crate::types::ToolCallReceiptReferenceV1 {
                    kind: crate::types::ToolCallReceiptReferenceKind::PendingDiff,
                    id: projection.diff_id.clone(),
                })
                .collect()
        },
    )
    .await
}

/**
 * @GOV
 * codes:    BR-AG-STATE-002-EFFECT-AG-AG-TOOL-017, BR-AG-DATA-003-EFFECT-AG-AG-TOOL-017, BR-DE-DATA-001-EFFECT-AG-AG-TOOL-017, BR-DE-DATA-002-EFFECT-AG-AG-TOOL-017
 * type:     EFFECT
 * chain:    AG-TOOL, DE-DIFF, ED-EDIT
 * rules:    BR-AG-STATE-002, BR-AG-DATA-003, BR-DE-DATA-001, BR-DE-DATA-002
 * boundary: in=ValidatedAgentEditOperationAndAgentReadContext | out=PendingDiffAndDiffProjection | delegate=de::generate::generate_diff | does_not=createWorkspaceResourceOrWriteLogicalStateOrDiskState | consumer=BR-AG-STATE-002,BR-AG-DATA-003
 */
/// PI-AG-08C/08E/08F + PI-AG-SEM-01 · propose_edit 写工具执行（复用 route_generate_diff 逻辑）。
///
/// AI 修改建议经 DE 生成 pending diff，**不直写文件**（X-FORBID-006）；限 active workspace（X-FORBID-004）；
/// target 须属 workspace；资源新建不进入本函数，统一由 WorkspaceOperationPlan 前置裁决。
/// 操作模式：纯文本块级（replace_block）、整篇（replace_document）、富文档块（block_ops）、
/// 样式（style_ops）、表格（table_ops）与图片（image_ops）。
/// 返回后端权威 DiffProjection；前端不得再从 tool_request 预测 before/after。
pub(super) fn ag_propose_edit<R: tauri::Runtime>(
    state: &AppState,
    app: &tauri::AppHandle<R>,
    params: &serde_json::Value,
    read_ctx: &ag::blocks::AgentReadContext,
    current_rel: Option<&str>,
    explicit_edit_target_set: &ag::target_set::ExplicitEditTargetSet,
    turn_id: &str,
    tool_call_id: &str,
) -> Result<AgentProposeEditResult, IpcError> {
    ag_propose_edit_with_effect_boundary(
        state,
        app,
        params,
        read_ctx,
        current_rel,
        explicit_edit_target_set,
        turn_id,
        tool_call_id,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
fn ag_propose_edit_with_effect_boundary<R: tauri::Runtime>(
    state: &AppState,
    app: &tauri::AppHandle<R>,
    params: &serde_json::Value,
    read_ctx: &ag::blocks::AgentReadContext,
    current_rel: Option<&str>,
    explicit_edit_target_set: &ag::target_set::ExplicitEditTargetSet,
    turn_id: &str,
    tool_call_id: &str,
    effect_boundary: Option<&std::sync::atomic::AtomicBool>,
) -> Result<AgentProposeEditResult, IpcError> {
    use crate::de::generate::{build_payload, generate_diff, ChangeProposal};
    use crate::types::LocateResult;

    let requested_rel_path = params
        .get("path")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| IpcError::new("AG_TOOL_ARG", "propose_edit 缺 path 参数", false, false))?
        .to_string();

    let ws = active_ws(state)?;
    if std::path::Path::new(&ws.root_path) != read_ctx.root() {
        return Err(IpcError::new(
            "AG_CONTEXT_WORKSPACE_CHANGED",
            "工作区身份已变化，未生成修改建议",
            false,
            false,
        ));
    }
    let rel_path = ag::target_set::normalize_existing_target(
        std::path::Path::new(&ws.root_path),
        &requested_rel_path,
    )?;
    // 路径限 workspace 内（X-FORBID-004）。
    let abs = workspace::resolve_in_workspace(std::path::Path::new(&ws.root_path), &rel_path)?;

    // 目标集双守（BR-AG-DATA-003）：既有文档必须属于 ExplicitEditTargetSet；未授权目标稳定拒绝，
    // 错误回注模型后只能由显式引用或 scope DecisionRequest 确认扩展。AG 与 DE 两侧均须通过。
    let target_exists = abs.exists();
    // WS-RESOURCE-WRITE-GUARD-01（F1，X-FORBID-006）：内容 Diff 不得创建资源。目标路径不存在即拒绝，
    // 与意图分类无关——此前 Edit/其他意图下 propose_edit 到不存在路径会经 ReplaceDocument + 接受
    // 落盘创建文件，已封闭；新建文档唯一入口是 plan_workspace_operation。
    guard_propose_edit_target(target_exists)?;
    // ExplicitEditTargetSet 是修改授权 SSOT；cross_doc 只保留集合内非当前文档的展示属性。
    ag::target_set::ensure_authorized(explicit_edit_target_set, &rel_path)?;
    crate::de::generate::ensure_agent_target_authorized(&rel_path, explicit_edit_target_set)?;
    let cross_doc = current_rel != Some(rel_path.as_str());
    let edit_operation = resolve_agent_edit_operation(params, target_exists)?;

    let target_is_current_editor = current_rel == Some(rel_path.as_str());

    let style_ops_json = style_ops_param(params).cloned();
    let snapshot = if target_exists {
        Some(read_ctx.snapshot(&rel_path)?)
    } else {
        None
    };
    let target_is_open_session = snapshot
        .as_ref()
        .map(|s| s.source == "open_editor_session")
        .unwrap_or(false);
    if target_is_current_editor
        && target_exists
        && snapshot
            .as_ref()
            .map(|s| is_active_editor_snapshot_source(&s.source))
            != Some(true)
    {
        return Err(IpcError::new(
            "AG_EDITOR_SNAPSHOT_REQUIRED",
            "当前打开文档缺少 active editor LogicalState 快照，已拒绝生成可能错位的修改建议。",
            true,
            false,
        ));
    }
    if target_exists
        && is_docx_rel_path(&rel_path)
        && !target_is_current_editor
        && !target_is_open_session
    {
        return Err(IpcError::new(
            "AG_BDOCX_EDIT_REQUIRES_OPEN_EDITOR",
            "b-docx 修改必须先打开为编辑器会话后生成，避免后台裁决把 HTML 当 docx 字节写入。",
            false,
            false,
        ));
    }
    if edit_operation == AgentEditOperation::TableOps
        && (!target_is_current_editor
            || !is_docx_rel_path(&rel_path)
            || snapshot.as_ref().map(|s| s.source.as_str()) != Some("active_editor"))
    {
        return Err(IpcError::new(
            "AG_TABLE_EDIT_REQUIRES_ACTIVE_EDITOR",
            "表格结构修改仅支持当前活动 BDocx 编辑器会话；请先打开目标 BDocx。",
            false,
            false,
        ));
    }
    if edit_operation == AgentEditOperation::ImageOps
        && (!target_is_current_editor
            || !is_docx_rel_path(&rel_path)
            || snapshot.as_ref().map(|current| current.source.as_str()) != Some("active_editor"))
    {
        return Err(IpcError::new(
            "AG_IMAGE_OPERATION_REQUIRES_ACTIVE_EDITOR",
            "图片插入/删除仅支持当前活动 BDocx 编辑器会话；请先打开目标 BDocx。",
            false,
            false,
        ));
    }
    if edit_operation == AgentEditOperation::BlockOps
        && (!target_is_current_editor
            || snapshot.as_ref().map(|current| current.source.as_str()) != Some("active_editor")
            || snapshot
                .as_ref()
                .map(|current| current.content_format.as_str())
                != Some("html"))
    {
        return Err(IpcError::new(
            "AG_BLOCK_OPERATION_REQUIRES_ACTIVE_EDITOR",
            "富文档结构化块修改仅支持当前活动编辑器 LogicalState；请先打开并激活目标文档。",
            false,
            false,
        ));
    }
    if edit_operation == AgentEditOperation::StyleOps {
        let current = snapshot.as_ref().ok_or_else(|| {
            IpcError::new(
                "AG_EDIT_BASE_REQUIRED",
                "样式修改必须基于既有文档快照",
                false,
                false,
            )
        })?;
        let style_ops = style_ops_json.as_ref().ok_or_else(|| {
            IpcError::new(
                "AG_TOOL_ARG",
                "operation=style_ops 必须提供 style_ops",
                false,
                false,
            )
        })?;
        ag::bdocx_capabilities::validate_style_ops(style_ops, current)
            .map_err(capability_error_to_ipc)?;
    }
    let base_content_for_diff = snapshot.as_ref().map(|s| s.content.as_str()).unwrap_or("");
    let base_is_html = snapshot
        .as_ref()
        .map(|s| s.content_format == "html")
        .unwrap_or(false);
    // AG-STRATEGY-04：既有富文档整篇替换在无表格、无图片的当前 BDocx 快照上可用；
    // 身份漂移与表格/图片结构守卫共同兜底。ReplaceBlock（HTML 字符串块替换）仍须走
    // block_ops（块级结构不可经 HTML 字符串表达）。
    if target_exists && base_is_html && edit_operation == AgentEditOperation::ReplaceBlock {
        return Err(IpcError::new(
            "AG_RICH_BLOCK_OPERATION_REQUIRED",
            "富文档块替换必须使用 operation=block_ops 和受控 ProseMirror node JSON；不接受 HTML 字符串块替换。",
            true,
            false,
        ));
    }
    let block_ops_groups = if edit_operation == AgentEditOperation::BlockOps {
        let current = snapshot.as_ref().ok_or_else(|| {
            IpcError::new(
                "AG_EDIT_BASE_REQUIRED",
                "结构化块修改必须基于当前活动编辑器快照",
                false,
                false,
            )
        })?;
        let canonical = ag::block_ops::validate_and_build(params, current)?;
        ag::block_ops::split_adjudication_groups(&canonical, current)?
            .into_iter()
            .map(|group| {
                serde_json::to_value(group).map_err(|error| {
                    IpcError::new(
                        "AG_BLOCK_OPERATION_INVALID",
                        &format!("结构化块载荷序列化失败: {error}"),
                        false,
                        false,
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?
    } else {
        vec![serde_json::Value::Null]
    };
    let table_ops_json = if edit_operation == AgentEditOperation::TableOps {
        let current = snapshot.as_ref().ok_or_else(|| {
            IpcError::new(
                "AG_TABLE_EDIT_REQUIRES_ACTIVE_EDITOR",
                "表格结构修改缺活动编辑器快照",
                false,
                false,
            )
        })?;
        Some(table_ops_payload(
            params,
            current,
            read_ctx.table_snapshots(&rel_path)?,
        )?)
    } else {
        None
    };
    let image_ops_json = if edit_operation == AgentEditOperation::ImageOps {
        let current = snapshot.as_ref().ok_or_else(|| {
            IpcError::new(
                "AG_IMAGE_OPERATION_REQUIRES_ACTIVE_EDITOR",
                "图片操作缺活动编辑器快照",
                false,
                false,
            )
        })?;
        let payload = image_ops_payload(
            params,
            current,
            read_ctx.image_snapshots(&rel_path)?,
            std::path::Path::new(&ws.root_path),
        )?;
        let baseline = current.baseline.ok_or_else(|| {
            IpcError::new(
                "AG_IMAGE_BASELINE_REQUIRED",
                "当前 BDocx 图片操作缺少媒体 baseline",
                true,
                false,
            )
        })?;
        doc::image_media::authorize_media_files(app, &doc::image_media::edit_media_dir(baseline))?;
        Some(payload)
    } else {
        None
    };
    let proposal_batch_id = gen_id("proposal_batch");
    let group_count = u32::try_from(block_ops_groups.len()).unwrap_or(u32::MAX);
    let created_at = now_millis();
    let mut persisted = Vec::with_capacity(block_ops_groups.len());
    let mut projections = Vec::with_capacity(block_ops_groups.len());
    for (offset, block_ops_value) in block_ops_groups.into_iter().enumerate() {
        let group_index = u32::try_from(offset + 1).unwrap_or(u32::MAX);
        let block_ops_json = (!block_ops_value.is_null()).then_some(block_ops_value);
        let EditProjectionDraft {
            after_content,
            block_id,
            block_index,
            before_text,
            after_text,
            location_label,
        } = build_edit_projection(
            params,
            edit_operation,
            snapshot.as_ref(),
            base_content_for_diff,
            base_is_html,
            &rel_path,
            target_exists,
            read_ctx,
            style_ops_json.as_ref(),
            block_ops_json.as_ref(),
            table_ops_json.as_ref(),
            image_ops_json.as_ref(),
        )?;
        let mut persisted_ops = if let Some(block_ops) = &block_ops_json {
            serde_json::json!({
                "kind": "block_ops",
                "payloadVersion": 1,
                "payload": block_ops
            })
        } else if let Some(table_ops) = &table_ops_json {
            serde_json::json!({ "kind": "table_ops", "payload": table_ops })
        } else if let Some(image_ops) = &image_ops_json {
            serde_json::json!({
                "kind": "image_ops",
                "payloadVersion": 1,
                "payload": image_ops
            })
        } else {
            style_ops_json
                .clone()
                .unwrap_or_else(|| serde_json::Value::Object(Default::default()))
        };
        if let Some(object) = persisted_ops.as_object_mut() {
            object.insert(
                "_projection".to_string(),
                serde_json::json!({
                    "patchKind": match edit_operation {
                        AgentEditOperation::ReplaceBlock => "replace_block",
                        AgentEditOperation::BlockOps => "block_ops",
                        AgentEditOperation::ReplaceDocument => "replace_document",
                        AgentEditOperation::StyleOps => "style_ops",
                        AgentEditOperation::TableOps => "table_ops",
                        AgentEditOperation::ImageOps => "image_ops",
                    },
                    "blockId": block_id.clone(),
                    "blockIndex": block_index,
                    "beforeText": before_text.clone(),
                    "afterText": after_text.clone(),
                    "locationLabel": location_label.clone(),
                    "draftMode": if target_is_current_editor { "current_editor" } else if target_is_open_session { "open_session" } else { "background" },
                    "snapshotSource": snapshot.as_ref().map(|item| item.source.as_str()).unwrap_or("disk_snapshot"),
                    "baseSessionVersion": snapshot.as_ref().and_then(|item| item.session_version),
                    "baseContentHash": snapshot.as_ref().and_then(|item| item.content_hash.clone()),
                }),
            );
        }
        let proposal = ChangeProposal {
            origin_task: "ag_turn".to_string(),
            target: crate::types::DocRef {
                workspace_id: ws.workspace_id.clone(),
                doc_type: doc_type_of(&rel_path),
                rel_path: rel_path.clone(),
            },
            after_content: after_content.into_bytes(),
            before_snapshot: target_exists.then(|| base_content_for_diff.to_string()),
            style_ops: Some(persisted_ops.to_string()),
        };
        let diff_id = gen_id("diff");
        let payload_ref = format!("pl_{diff_id}");
        let mut pending = generate_diff(
            &proposal,
            &diff_id,
            "AG",
            &payload_ref,
            created_at,
            LocateResult::Located,
        )?;
        pending.proposal_batch_id = proposal_batch_id.clone();
        pending.group_index = group_index;
        pending.group_count = group_count;
        persisted.push((payload_ref, build_payload(&proposal), pending));
        projections.push(DiffProjection {
            diff_id,
            rel_path: rel_path.clone(),
            patch_kind: match edit_operation {
                AgentEditOperation::ReplaceBlock => "replace_block",
                AgentEditOperation::BlockOps => "block_ops",
                AgentEditOperation::ReplaceDocument => "replace_document",
                AgentEditOperation::StyleOps => "style_ops",
                AgentEditOperation::TableOps => "table_ops",
                AgentEditOperation::ImageOps => "image_ops",
            }
            .to_string(),
            block_id,
            block_index,
            before_text,
            after_text,
            location_label,
            draft_mode: if target_is_current_editor {
                "current_editor"
            } else if target_is_open_session {
                "open_session"
            } else {
                "background"
            }
            .to_string(),
            snapshot_source: snapshot
                .as_ref()
                .map(|s| match s.source.as_str() {
                    "active_editor" => "active_editor",
                    "active_editor_session" => "active_editor_session",
                    "open_editor_session" => "open_editor_session",
                    _ => "disk_snapshot",
                })
                .unwrap_or("disk_snapshot")
                .to_string(),
            base_session_version: snapshot.as_ref().and_then(|s| s.session_version),
            base_content_hash: snapshot.as_ref().and_then(|s| s.content_hash.clone()),
            style_ops: style_ops_json.clone(),
            block_ops: block_ops_json,
            table_ops: table_ops_json.clone(),
            image_ops: image_ops_json.clone(),
            proposal_batch_id: proposal_batch_id.clone(),
            group_index,
            group_count,
            preview_checkpoint_required: target_is_current_editor || target_is_open_session,
            cross_doc: Some(cross_doc),
        });
    }
    let conn = lock_db(state)?;
    let batch = crate::types::ProposalBatch {
        proposal_batch_id: proposal_batch_id.clone(),
        workspace_id: ws.workspace_id,
        turn_id: Some(turn_id.to_string()),
        tool_call_id: Some(tool_call_id.to_string()),
        grouping_version: 1,
        created_at,
    };
    // Before this atomic transaction there is no persisted proposal or pending diff. Once it
    // starts, any failure remains effect-unknown until the original identity is inspected.
    if let Some(boundary) = effect_boundary {
        boundary.store(true, std::sync::atomic::Ordering::Release);
    }
    crate::de::store::persist_proposal_batch(&conn, &batch, &persisted)
        .map_err(|e| IpcError::new("DE_DB_ERROR", &e.to_string(), true, false))?;
    Ok(AgentProposeEditResult {
        proposal_batch_id,
        rel_path,
        projections,
    })
}
