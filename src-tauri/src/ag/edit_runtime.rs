//! Agent 内容修改建议到 PendingDiff 的执行编排。

use super::edit_image_runtime::image_ops_payload;
use super::edit_projection::build_edit_projection;
use super::edit_support::{
    capability_error_to_ipc, doc_type_of, gen_id, guard_propose_edit_target,
    is_active_editor_snapshot_source, is_docx_rel_path, now_millis, resolve_agent_edit_operation,
    style_ops_param,
};
use super::edit_table_runtime::table_ops_payload;
use super::edit_types::{
    AgentEditOperation, AgentProposeEditResult, BatchMeta, EditProjectionDraft,
};
use crate::ipc::IpcError;
use crate::types::DiffProjection;
use crate::{active_ws, ag, doc, lock_db, workspace, AppState};

/**
 * @GOV
 * codes:    BR-AG-STATE-002-EFFECT-AG-AG-TOOL-017, BR-AG-DATA-003-EFFECT-AG-AG-TOOL-017
 * type:     EFFECT
 * chain:    AG-TOOL, DE-DIFF, ED-EDIT
 * rules:    BR-AG-STATE-002, BR-AG-DATA-003
 * boundary: in=ValidatedAgentEditOperationAndAgentReadContext | out=PendingDiffAndDiffProjection | delegate=de::generate::generate_diff | does_not=createWorkspaceResourceOrWriteLogicalStateOrDiskState
 */
/// PI-AG-08C/08E/08F + PI-AG-SEM-01 · propose_edit 写工具执行（复用 route_generate_diff 逻辑）。
///
/// AI 修改建议经 DE 生成 pending diff，**不直写文件**（X-FORBID-006）；限 active workspace（X-FORBID-004）；
/// target 须属 workspace；资源新建不进入本函数，统一由 WorkspaceOperationPlan 前置裁决。
/// 操作模式：纯文本块级（replace_block）、整篇（replace_document）、富文档块（block_ops）、
/// 样式（style_ops）、表格（table_ops）与图片（image_ops）。
/// 返回后端权威 DiffProjection；前端不得再从 tool_request 预测 before/after。
pub(super) fn ag_propose_edit(
    state: &AppState,
    app: &tauri::AppHandle,
    params: &serde_json::Value,
    read_ctx: &ag::blocks::AgentReadContext,
    current_rel: Option<&str>,
    declared_targets: &[String],
) -> Result<AgentProposeEditResult, IpcError> {
    use crate::de::generate::{build_payload, generate_diff, ChangeProposal};
    use crate::types::LocateResult;

    // AG-TASK-01 F1b：解析可选 batch 参数（{id,index,total}），校验 index<=total。
    let batch = params
        .get("batch")
        .and_then(|v| v.as_object())
        .map(|batch| {
            let batch_id = batch
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let index = batch.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let total = batch.get("total").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
            if batch_id.is_empty() || index < 1 || total < 1 || index > total {
                Err(IpcError::new(
                    "AG_TOOL_ARG",
                    "propose_edit batch 参数需为 {id,index,total}，且 1<=index<=total",
                    false,
                    false,
                ))
            } else {
                Ok(BatchMeta {
                    batch_id,
                    index,
                    total,
                })
            }
        })
        .transpose()?;

    let rel_path = params
        .get("path")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| IpcError::new("AG_TOOL_ARG", "propose_edit 缺 path 参数", false, false))?
        .to_string();

    let ws = active_ws(state)?;
    // 路径限 workspace 内（X-FORBID-004）。
    let abs = workspace::resolve_in_workspace(std::path::Path::new(&ws.root_path), &rel_path)?;

    // 目标集双守（BR-AG-DATA-003，2026-07-04 版）：当前文档/显式声明集，未声明既有文档拒绝
    // （错误回注模型，模型可提示用户显式 @引用后重试）。AG guard 与 DE 协同校验两侧均须通过。
    let target_exists = abs.exists();
    // WS-RESOURCE-WRITE-GUARD-01（F1，X-FORBID-006）：内容 Diff 不得创建资源。目标路径不存在即拒绝，
    // 与意图分类无关——此前 Edit/其他意图下 propose_edit 到不存在路径会经 ReplaceDocument + 接受
    // 落盘创建文件，已封闭；新建文档唯一入口是 plan_workspace_operation。
    guard_propose_edit_target(target_exists)?;
    // 未声明既有文档放行（AcceptCrossDocument），标注 cross_doc
    // 由 DiffCard「跨文档目标」+ 用户确认承担安全；不再硬拒（延续词/多文档联动不再被锁死）。
    let cross_doc = match crate::ag::context::guard_target_set(
        current_rel,
        declared_targets,
        &rel_path,
        target_exists,
    ) {
        crate::ag::context::TargetDecision::AcceptCrossDocument => true,
        crate::ag::context::TargetDecision::Accept => false,
    };
    let cross_doc = cross_doc
        || !crate::de::generate::target_declared(
            &rel_path,
            current_rel,
            declared_targets,
            target_exists,
        );
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
    let block_ops_json = if edit_operation == AgentEditOperation::BlockOps {
        let current = snapshot.as_ref().ok_or_else(|| {
            IpcError::new(
                "AG_EDIT_BASE_REQUIRED",
                "结构化块修改必须基于当前活动编辑器快照",
                false,
                false,
            )
        })?;
        Some(
            serde_json::to_value(ag::block_ops::validate_and_build(params, current)?).map_err(
                |error| {
                    IpcError::new(
                        "AG_BLOCK_OPERATION_INVALID",
                        &format!("结构化块载荷序列化失败: {error}"),
                        false,
                        false,
                    )
                },
            )?,
        )
    } else {
        None
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
    // 复用现有可持久化 JSON 字段承载语义操作，避免把 TableOperation/ImageOperation
    // 伪装为 after HTML；kind 负责在读取/执行侧禁止错误路径。
    let style_ops = if let Some(block_ops) = &block_ops_json {
        Some(
            serde_json::json!({
                "kind": "block_ops",
                "payloadVersion": 1,
                "payload": block_ops
            })
            .to_string(),
        )
    } else if let Some(table_ops) = &table_ops_json {
        Some(serde_json::json!({ "kind": "table_ops", "payload": table_ops }).to_string())
    } else if let Some(image_ops) = &image_ops_json {
        Some(
            serde_json::json!({
                "kind": "image_ops",
                "payloadVersion": 1,
                "payload": image_ops
            })
            .to_string(),
        )
    } else {
        style_ops_json.as_ref().map(|v| v.to_string())
    };
    // AG-TASK-01 F1b/F1c：批次元数据随 style_ops 落库（diff_payloads.style_ops 内 _batch 键），
    // 供接受路由查询剩余批次并给出衔接提示。
    let style_ops = if let Some(batch) = &batch {
        let mut envelope: serde_json::Value = style_ops
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or(serde_json::Value::Object(Default::default()));
        if let Some(object) = envelope.as_object_mut() {
            object.insert(
                "_batch".to_string(),
                serde_json::json!({
                    "id": batch.batch_id,
                    "index": batch.index,
                    "total": batch.total,
                }),
            );
        }
        Some(envelope.to_string())
    } else {
        style_ops
    };

    let EditProjectionDraft {
        after_content,
        block_id: projection_block_id,
        block_index: projection_block_index,
        before_text: projection_before,
        after_text: projection_after,
        location_label: projection_location,
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

    // 既有文档 → before_snapshot=Some（modify）；不存在 → None（新建，is_new_doc 派生）。
    // BDocx 磁盘是二进制 docx，必须用当前编辑器 HTML 或 DOC 转换 HTML 作为 diff 基准，不能因 UTF-8
    // 读取失败而误判为新文档。
    let before_snapshot = if target_exists {
        Some(base_content_for_diff.to_string())
    } else {
        None
    };

    let proposal = ChangeProposal {
        origin_task: "ag_turn".to_string(),
        target: crate::types::DocRef {
            workspace_id: ws.workspace_id,
            doc_type: doc_type_of(&rel_path),
            rel_path: rel_path.clone(),
        },
        after_content: after_content.into_bytes(),
        before_snapshot,
        style_ops,
    };
    let diff_id = gen_id("diff");
    let payload_ref = format!("pl_{diff_id}");
    // 锚校验（既有须 Located；新建跳过）——失败 GenFailed 回报，不持久化（BR-DE-DATA-001）。
    let pending = generate_diff(
        &proposal,
        &diff_id,
        "AG",
        &payload_ref,
        now_millis(),
        LocateResult::Located,
    )?;
    let payload = build_payload(&proposal);

    let conn = lock_db(state)?;
    crate::de::store::persist_payload(&conn, &payload_ref, &payload)
        .map_err(|e| IpcError::new("DE_DB_ERROR", &e.to_string(), true, false))?;
    workspace::diff_apply::persist_diff(&conn, &pending)
        .map_err(|e| IpcError::new("DE_DB_ERROR", &e.to_string(), true, false))?;
    let projection = DiffProjection {
        diff_id: diff_id.clone(),
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
        block_id: projection_block_id,
        block_index: projection_block_index,
        before_text: projection_before,
        after_text: projection_after,
        location_label: projection_location,
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
        style_ops: style_ops_json,
        block_ops: block_ops_json,
        table_ops: table_ops_json,
        image_ops: image_ops_json,
        batch: batch.as_ref().map(|b| {
            serde_json::json!({
                "id": b.batch_id,
                "index": b.index,
                "total": b.total,
            })
        }),
        preview_checkpoint_required: target_is_current_editor || target_is_open_session,
        // 跨文档内容目标标注（DiffCard「跨文档目标」提示 + 用户确认）。
        cross_doc: Some(cross_doc),
    };
    Ok(AgentProposeEditResult {
        diff_id,
        rel_path,
        projection,
        batch,
    })
}
