//! 从应用组合根迁入的模块运行时/路由；保持原 command 与状态机行为等价。

use crate::*;

fn now_millis() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

fn gen_id(prefix: &str) -> String {
    format!("{prefix}_{}", uuid::Uuid::new_v4())
}

fn doc_type_of(rel_path: &str) -> String {
    std::path::Path::new(rel_path)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
        .unwrap_or_else(|| "bin".to_string())
}

fn is_docx_rel_path(rel_path: &str) -> bool {
    matches!(doc_type_of(rel_path).as_str(), "docx" | "doc")
}

fn can_accept_background_diff(rel_path: &str, is_new_doc: bool) -> bool {
    !is_docx_rel_path(rel_path) || is_new_doc
}

/// 路由 `de.generate_diff`：组装 ChangeProposal→锚校验生成 PendingDiff（BR-DE-DATA-001）→持久化
/// DiffPayload + pending_diff（重启恢复）；修改既有须锚 Located，否则 DE_ANCHOR_LOST（不产悬空 diff）。
pub(crate) fn route_generate_diff(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    use crate::de::generate::{build_payload, generate_diff, ChangeProposal};
    use crate::types::LocateResult;

    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let after_content = request
        .params
        .get("afterContent")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            IpcError::new(
                "WS_CONTENT_REQUIRED",
                "缺少 afterContent 参数",
                false,
                false,
            )
        })?
        .to_string();
    let before_snapshot = request
        .params
        .get("beforeSnapshot")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let origin_module = request
        .params
        .get("originModule")
        .and_then(|v| v.as_str())
        .unwrap_or("AG")
        .to_string();
    let origin_task = request
        .params
        .get("originTask")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let anchor_locate = match request.params.get("anchorLocate").and_then(|v| v.as_str()) {
        Some("anchor_lost") => LocateResult::AnchorLost,
        Some("undetectable") => LocateResult::Undetectable,
        _ => LocateResult::Located,
    };

    let ws = active_ws(state)?;
    let target = crate::types::DocRef {
        workspace_id: ws.workspace_id,
        doc_type: doc_type_of(&rel_path),
        rel_path,
    };
    // 样式操作序列（PI-ED-11，可选）：StyleOp JSON（形状归 ED A-ED-M-T-01 §5.1），随载荷持久化。
    let style_ops = request
        .params
        .get("styleOps")
        .filter(|v| !v.is_null())
        .map(|v| v.to_string());
    let proposal = ChangeProposal {
        origin_task,
        target,
        after_content: after_content.into_bytes(),
        before_snapshot,
        style_ops,
    };
    let diff_id = gen_id("diff");
    let payload_ref = format!("pl_{diff_id}");
    // 锚校验（修改既有须 Located；新建跳过）——失败即 GenFailed 回报，不持久化。
    let pending = generate_diff(
        &proposal,
        &diff_id,
        &origin_module,
        &payload_ref,
        now_millis(),
        anchor_locate,
    )?;
    let payload = build_payload(&proposal);

    let conn = lock_db(state)?;
    crate::de::store::persist_payload(&conn, &payload_ref, &payload)
        .map_err(|e| IpcError::new("DE_DB_ERROR", &e.to_string(), true, false))?;
    workspace::diff_apply::persist_diff(&conn, &pending)
        .map_err(|e| IpcError::new("DE_DB_ERROR", &e.to_string(), true, false))?;
    Ok(serde_json::json!({ "diff_id": diff_id, "status": "pending" }))
}

/// 路由 `de.accept_draft_diff`：用于已在打开编辑器中应用为草稿内容的 diff。经 SM-DE-01 守护
/// Pending→Accepted，但不触发 SM-WS-02 落盘；可由用户显式接受或保存有效 pending 预览触发，
/// 后续由 SM-WS-01 写盘。
/**
 * @GOV
 * codes:    BR-CORE-STATE-001-EFFECT-DE-DE-DIFF-005
 * type:     EFFECT
 * chain:    DE-DIFF, ED-EDIT, WS-SAVE
 * rules:    BR-CORE-STATE-001
 * boundary: in=PendingDiffAlreadyAppliedToLogicalStateAndExplicitUserAcceptOrSave | out=AcceptedDiffCardLifecycleOnly(noLogicalContentMutationOrDiskWrite)
 */
pub(crate) fn route_accept_draft_diff<R: tauri::Runtime>(
    request: &CommandRequest,
    state: &AppState,
    app: &tauri::AppHandle<R>,
) -> Result<serde_json::Value, IpcError> {
    let diff_id = str_param(request, "diffId", "WS_DIFF_ID_REQUIRED")?;
    let apply_context = request
        .params
        .get("applyContext")
        .and_then(|v| v.as_str())
        .unwrap_or("current_editor");
    if apply_context == "open_session" {
        let has_base_version = request
            .params
            .get("baseSessionVersion")
            .and_then(|v| v.as_u64())
            .is_some();
        let has_base_hash = request
            .params
            .get("baseContentHash")
            .and_then(|v| v.as_str())
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        if !has_base_version || !has_base_hash {
            return Err(IpcError::new(
                "DE_OPEN_SESSION_BASE_REQUIRED",
                "open_session diff 确认必须携带 baseSessionVersion/baseContentHash",
                false,
                false,
            ));
        }
    }
    let ws = active_ws(state)?;
    let conn = lock_db(state)?;
    if workspace::operation::load_create_document_bundle_by_diff(&conn, &diff_id)
        .map_err(|error| IpcError::new("WS_OPERATION_DB_ERROR", &error.to_string(), true, false))?
        .is_some()
    {
        let logical_session_version = request
            .params
            .get("logicalSessionVersion")
            .and_then(|value| value.as_u64())
            .ok_or_else(|| {
                IpcError::new(
                    "WS_CREATE_BUNDLE_LOGICAL_BASE_REQUIRED",
                    "接受新建文档初始内容必须携带当前 LogicalState 版本",
                    false,
                    false,
                )
            })?;
        let logical_content_hash = str_param(
            request,
            "logicalContentHash",
            "WS_CREATE_BUNDLE_LOGICAL_BASE_REQUIRED",
        )?;
        match workspace::operation::validate_create_document_diff_guard(
            &conn,
            &ws,
            &diff_id,
            logical_session_version,
            &logical_content_hash,
        ) {
            Ok(true) => {}
            Ok(false) => {}
            Err(error) if error.code == "WS_CREATE_BUNDLE_BASELINE_ADVANCED" => {
                discard_diff_sm(app, &conn, &diff_id, "expired", Some(&error.code))?;
                sm_events::emit_sm_state(
                    app,
                    sm_events::SmStatePayload::new("SM-DE-01", &diff_id, "Expired", true),
                );
                return Ok(serde_json::json!({
                    "accepted": false,
                    "expired": true,
                    "reason": error.code,
                }));
            }
            Err(error) => return Err(error),
        }
    }
    let cur = crate::de::state_from_status(workspace::diff_apply::current_status(&conn, &diff_id)?);
    let next = crate::de::transition(cur, crate::de::DiffEvent::UserAccept).ok_or_else(|| {
        IpcError::new(
            "DE_ILLEGAL_TRANSITION",
            &format!("diff 当前态 {cur:?} 不可接受（SM-DE-01 三终态不可逆）"),
            false,
            false,
        )
    })?;
    // effect_status=awaiting_save（BR-SYS-DATA-005）：草稿/open_session 路径接受时内容尚未落盘，
    // 待用户实际保存成功后才由保存回调翻转为 saved；异常退出清算据此识别效果是否已丢失。
    conn.execute(
        "UPDATE pending_diffs SET status = 'accepted', effect_status = 'awaiting_save', decided_at = ?2 \
         WHERE diff_id = ?1",
        (&diff_id, now_millis()),
    )
    .map_err(|e| {
        IpcError::new(
            "DE_DB_ERROR",
            &format!("更新 diff 草稿裁决态失败: {e}"),
            true,
            false,
        )
    })?;
    workspace::operation::set_create_document_bundle_status_by_diff(
        &conn,
        &diff_id,
        "initial_diff_accepted",
    )
    .map_err(|error| IpcError::new("WS_OPERATION_DB_ERROR", &error.to_string(), true, false))?;
    sm_events::emit_sm_state(
        app,
        sm_events::SmStatePayload::new(
            "SM-DE-01",
            &diff_id,
            &format!("{next:?}"),
            crate::de::is_terminal(next),
        ),
    );
    Ok(serde_json::json!({ "accepted": true }))
}

/**
 * @GOV
 * codes:    BR-CORE-STATE-003-EFFECT-DE-DE-DIFF-006
 * type:     EFFECT
 * chain:    DE-DIFF, ED-EDIT, ED-DECOR, WS-SAVE
 * rules:    BR-CORE-STATE-003
 * boundary: in=PendingBackgroundDiffAndOpenedTargetLogicalState | out=PendingLogicalStateReviewRouteAcknowledgement(noTerminalTransitionOrDiskWrite)
 */
/// 目标文档在后台卡片待裁决期间被打开后，ED 已先以同一载荷建立 LogicalState 并展示 GreenReview。
/// 本路由只确认该卡仍为 Pending，供前端把分流从 background 切到 LogicalState 路径；不得接受、
/// 不得写盘，也不得把预览动作伪装成裁决动作。
pub(crate) fn route_promote_pending_diff_on_open(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let diff_id = str_param(request, "diffId", "WS_DIFF_ID_REQUIRED")?;
    let conn = lock_db(state)?;
    let current =
        crate::de::state_from_status(workspace::diff_apply::current_status(&conn, &diff_id)?);
    if current != crate::de::DiffState::Pending {
        return Err(IpcError::new(
            "DE_ILLEGAL_TRANSITION",
            &format!("diff 当前态 {current:?} 不可迁移到 LogicalState 审阅路径"),
            false,
            false,
        ));
    }
    Ok(serde_json::json!({ "diff_id": diff_id, "pending": true }))
}

/// 路由 `de.accept_diff`：后台/整篇 diff 经 SM-DE-01 守护（仅 Pending 可接受，三终态不可逆）
/// → ws.apply_diff 落盘 → 广播 SM-DE-01 转移；当前编辑器草稿路径使用 `de.accept_draft_diff`。
/// 既有 BDocx 不允许走后台接受，避免绕过当前编辑器 HTML 草稿与 DOC 保存链。
/**
 * @GOV
 * codes:    BR-CORE-STATE-002-EFFECT-DE-DE-DIFF-007
 * type:     EFFECT
 * chain:    DE-DIFF, WS-DIFF-APPLY
 * rules:    BR-CORE-STATE-002
 * boundary: in=PendingDiffWithoutLogicalStateAndExplicitUserAccept | out=AtomicDiskStateWriteAndAcceptedDiffLifecycle
 */
pub(crate) fn route_accept_diff(
    request: &CommandRequest,
    state: &AppState,
    app: &tauri::AppHandle,
) -> Result<serde_json::Value, IpcError> {
    let diff_id = str_param(request, "diffId", "WS_DIFF_ID_REQUIRED")?;
    let ws = active_ws(state)?;
    let conn = lock_db(state)?;
    // ① 集成：取当前持久态经 SM-DE-01 守护——非 Pending（已终态）不得接受，非法转移回错误不静默落盘。
    let cur = crate::de::state_from_status(workspace::diff_apply::current_status(&conn, &diff_id)?);
    let next = crate::de::transition(cur, crate::de::DiffEvent::UserAccept).ok_or_else(|| {
        IpcError::new(
            "DE_ILLEGAL_TRANSITION",
            &format!("diff 当前态 {cur:?} 不可接受（SM-DE-01 三终态不可逆）"),
            false,
            false,
        )
    })?;
    // ② 落盘：apply_diff 置 status=accepted，与 transition 给出的权威态 next=Accepted 一致。
    let stored = crate::de::store::load_payload_for_diff(&conn, &diff_id)?;
    let semantic_kind = stored
        .style_ops
        .as_deref()
        .and_then(|json| serde_json::from_str::<serde_json::Value>(json).ok())
        .and_then(|value| {
            value
                .get("kind")
                .and_then(|kind| kind.as_str())
                .map(str::to_string)
        });
    if semantic_kind.as_deref() == Some("image_ops") {
        return Err(IpcError::new(
            "DE_IMAGE_OPERATION_REQUIRES_LOGICAL_STATE",
            "ImageOperation 只能在当前活动 BDocx LogicalState 中应用，不能经后台 accept_diff 落盘。",
            false,
            false,
        ));
    }
    let rel_path = stored.rel_path.clone();
    if !can_accept_background_diff(&rel_path, stored.is_new_doc) {
        return Err(IpcError::new(
            "DE_BDOCX_DRAFT_REQUIRED",
            "既有 b-docx 修改必须先应用到当前编辑器草稿后接受，不能经后台 accept_diff 写入 docx。",
            false,
            false,
        ));
    }
    let abs = workspace::resolve_in_workspace(std::path::Path::new(&ws.root_path), &rel_path)?;
    let doc = crate::types::DocRef {
        workspace_id: ws.workspace_id,
        doc_type: doc_type_of(&rel_path),
        rel_path: rel_path.clone(),
    };
    let should_mark_bdocx = is_docx_rel_path(&rel_path)
        && (stored.is_new_doc
            || workspace::listing::is_bdocx(std::path::Path::new(&ws.root_path), &rel_path));
    let mut docx_saving = None;
    let apply_content = if is_docx_rel_path(&rel_path) {
        let saving = doc::convert::transition(
            doc::convert::DocConvertState::Ready,
            doc::convert::DocConvertEvent::SaveDocx,
        )
        .expect("SM-DOC-01: Ready+SaveDocx→SaveConverting");
        emit_doc01(app, &rel_path, saving);
        docx_saving = Some(saving);
        let html = std::str::from_utf8(&stored.after_content).map_err(|e| {
            let st =
                doc::convert::transition(saving, doc::convert::DocConvertEvent::SaveConvertFail)
                    .expect("SaveConverting+SaveConvertFail→SaveError");
            emit_doc01(app, &rel_path, st);
            IpcError::new(
                "DOC_HTML_INVALID",
                &format!("docx 修改载荷必须是 UTF-8 HTML: {e}"),
                false,
                false,
            )
        })?;
        match doc::edit_chain::html_to_docx(html, None) {
            Ok(bytes) => bytes,
            Err(e) => {
                let st = doc::convert::transition(
                    saving,
                    doc::convert::DocConvertEvent::SaveConvertFail,
                )
                .expect("SaveConverting+SaveConvertFail→SaveError");
                emit_doc01(app, &rel_path, st);
                return Err(e);
            }
        }
    } else {
        stored.after_content
    };
    if should_mark_bdocx {
        workspace::listing::mark_bdocx(std::path::Path::new(&ws.root_path), &rel_path)?;
    }
    // 经 SM-WS-02 落盘机驱动 apply（广播 Applying→Applied/ApplyError），与 SM-DE-01 同 diff 实例。
    let entry = match apply_diff_sm(app, &conn, &diff_id, &doc, &abs, &apply_content, None) {
        Ok(entry) => {
            if let Some(saving) = docx_saving {
                let st =
                    doc::convert::transition(saving, doc::convert::DocConvertEvent::SaveConvertOk)
                        .expect("SaveConverting+SaveConvertOk→Ready");
                emit_doc01(app, &rel_path, st);
            }
            entry
        }
        Err(e) => {
            if let Some(saving) = docx_saving {
                let st = doc::convert::transition(
                    saving,
                    doc::convert::DocConvertEvent::SaveConvertFail,
                )
                .expect("SaveConverting+SaveConvertFail→SaveError");
                emit_doc01(app, &rel_path, st);
            }
            return Err(e);
        }
    };
    // ③ 广播 SM-DE-01 转移（前端 deStore 投影，X-FORBID-007）。
    sm_events::emit_sm_state(
        app,
        sm_events::SmStatePayload::new(
            "SM-DE-01",
            &diff_id,
            &format!("{next:?}"),
            crate::de::is_terminal(next),
        ),
    );
    ws_json(&entry)
}

/// 路由 `de.reject_diff`：经 SM-DE-01 守护（仅 Pending 可拒绝）→ ws.discard_diff(rejected) 不写文件、
/// 不记节点 → 广播 SM-DE-01 转移（终态不可逆，A-CORE-X-L-04 §5.4 Task 2-B）。
pub(crate) fn route_reject_diff<R: tauri::Runtime>(
    request: &CommandRequest,
    state: &AppState,
    app: &tauri::AppHandle<R>,
) -> Result<serde_json::Value, IpcError> {
    let diff_id = str_param(request, "diffId", "WS_DIFF_ID_REQUIRED")?;
    let ws = active_ws(state)?;
    let conn = lock_db(state)?;
    let create_bundle = workspace::operation::load_create_document_bundle_by_diff(&conn, &diff_id)
        .map_err(|error| IpcError::new("WS_OPERATION_DB_ERROR", &error.to_string(), true, false))?;
    if create_bundle.is_some() {
        let logical_session_version = request
            .params
            .get("logicalSessionVersion")
            .and_then(|value| value.as_u64())
            .ok_or_else(|| {
                IpcError::new(
                    "WS_CREATE_BUNDLE_LOGICAL_BASE_REQUIRED",
                    "拒绝新建文档初始内容必须携带当前 LogicalState 版本",
                    false,
                    false,
                )
            })?;
        let logical_content_hash = str_param(
            request,
            "logicalContentHash",
            "WS_CREATE_BUNDLE_LOGICAL_BASE_REQUIRED",
        )?;
        match workspace::operation::discard_created_resource_if_guard_matches(
            &conn,
            &ws,
            &diff_id,
            logical_session_version,
            &logical_content_hash,
        ) {
            Ok(Some(entry)) => {
                discard_diff_sm(app, &conn, &diff_id, "rejected", None)?;
                sm_events::emit_sm_state(
                    app,
                    sm_events::SmStatePayload::new("SM-DE-01", &diff_id, "Rejected", true),
                );
                return Ok(serde_json::json!({
                    "diff_id": diff_id,
                    "status": "rejected",
                    "resource_discarded": true,
                    "workspace_timeline_entry_id": entry.entry_id,
                }));
            }
            Ok(None) => {}
            Err(error) if error.code == "WS_CREATE_BUNDLE_BASELINE_ADVANCED" => {
                discard_diff_sm(app, &conn, &diff_id, "expired", Some(&error.code))?;
                sm_events::emit_sm_state(
                    app,
                    sm_events::SmStatePayload::new("SM-DE-01", &diff_id, "Expired", true),
                );
                return Ok(serde_json::json!({
                    "diff_id": diff_id,
                    "status": "expired",
                    "resource_discarded": false,
                    "reason": error.code,
                }));
            }
            Err(error) => return Err(error),
        }
    }
    // ① 集成：经 SM-DE-01 守护当前态——已终态不得重复拒绝。
    let cur = crate::de::state_from_status(workspace::diff_apply::current_status(&conn, &diff_id)?);
    let next = crate::de::transition(cur, crate::de::DiffEvent::UserReject).ok_or_else(|| {
        IpcError::new(
            "DE_ILLEGAL_TRANSITION",
            &format!("diff 当前态 {cur:?} 不可拒绝（SM-DE-01 三终态不可逆）"),
            false,
            false,
        )
    })?;
    // ② 落终态：经 SM-WS-02 驱动 discard（rejected，不写文件、不记节点）+ 广播。
    discard_diff_sm(app, &conn, &diff_id, "rejected", None)?;
    // ③ 广播 SM-DE-01 转移。
    sm_events::emit_sm_state(
        app,
        sm_events::SmStatePayload::new(
            "SM-DE-01",
            &diff_id,
            &format!("{next:?}"),
            crate::de::is_terminal(next),
        ),
    );
    Ok(serde_json::json!({ "diff_id": diff_id, "status": "rejected" }))
}

/// 路由 `de.expire_diff`：自动应用失败、锚失效或会话基线推进时结束 PendingDiff。
/// 该路径不写用户文档、不记时间轴节点，只经 SM-DE-01 / SM-WS-02 落 Expired。
pub(crate) fn route_expire_diff(
    request: &CommandRequest,
    state: &AppState,
    app: &tauri::AppHandle,
) -> Result<serde_json::Value, IpcError> {
    let diff_id = str_param(request, "diffId", "WS_DIFF_ID_REQUIRED")?;
    let reason = request
        .params
        .get("reason")
        .and_then(|v| v.as_str())
        .unwrap_or("expired")
        .to_string();
    let conn = lock_db(state)?;
    let cur = crate::de::state_from_status(workspace::diff_apply::current_status(&conn, &diff_id)?);
    let next =
        crate::de::transition(cur, crate::de::DiffEvent::ExternalChange).ok_or_else(|| {
            IpcError::new(
                "DE_ILLEGAL_TRANSITION",
                &format!("diff 当前态 {cur:?} 不可过期（SM-DE-01 三终态不可逆）"),
                false,
                false,
            )
        })?;
    discard_diff_sm(app, &conn, &diff_id, "expired", Some(&reason))?;
    sm_events::emit_sm_state(
        app,
        sm_events::SmStatePayload::new(
            "SM-DE-01",
            &diff_id,
            &format!("{next:?}"),
            crate::de::is_terminal(next),
        ),
    );
    Ok(serde_json::json!({ "diff_id": diff_id, "status": "expired", "reason": reason }))
}

/// 路由 `de.update_image_diff_snapshots`：把 ED 实际应用得到的图片前后快照补入既有
/// ImageOperation DiffPayload。载荷只能首次写入或同值幂等重试，不改变文档内容或 Diff 终态。
pub(crate) fn route_update_image_diff_snapshots(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let diff_id = str_param(request, "diffId", "DE_DIFF_ID_REQUIRED")?;
    let before = request.params.get("before").ok_or_else(|| {
        IpcError::new(
            "DE_IMAGE_SNAPSHOT_REQUIRED",
            "缺少 before 图片快照",
            false,
            false,
        )
    })?;
    let after = request.params.get("after").ok_or_else(|| {
        IpcError::new(
            "DE_IMAGE_SNAPSHOT_REQUIRED",
            "缺少 after 图片快照",
            false,
            false,
        )
    })?;
    let affected_block_ids = request.params.get("affectedBlockIds").ok_or_else(|| {
        IpcError::new(
            "DE_IMAGE_SNAPSHOT_REQUIRED",
            "缺少 affectedBlockIds 图片锚列表",
            false,
            false,
        )
    })?;
    let ws = active_ws(state)?;
    let conn = lock_db(state)?;
    workspace::diff_apply::persist_image_diff_snapshots(
        &conn,
        &ws.workspace_id,
        &diff_id,
        before,
        after,
        affected_block_ids,
    )?;
    Ok(serde_json::json!({ "persisted": true }))
}

/// 路由 `de.list_diffs`：活动 workspace 未裁决 diff 列表（重启恢复，复用 WS 持久化查询）。
pub(crate) fn route_list_diffs(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    if request.params.get("diffIds").is_some() {
        return route_list_diff_details(request, state);
    }
    let status = request.params.get("status").and_then(|v| v.as_str());
    let ws = active_ws(state)?;
    let conn = lock_db(state)?;
    let list = workspace::diff_apply::list_pending_diffs(&conn, &ws.workspace_id, status)
        .map_err(|e| IpcError::new("DE_DB_ERROR", &e.to_string(), true, false))?;
    ws_json(&list)
}

/**
 * @GOV
 * codes:    BR-AG-PERSIST-001-QUERY-AG-AG-TURN-004
 * type:     QUERY
 * chain:    AG-TURN
 * rules:    BR-AG-PERSIST-001
 * term_ref: TERM-AG-001
 * boundary: in=agentHistoryDiffIds | out=diffDetailsForHistoryRestore(allStatusesNoPendingRevival)
 */
/// 历史 DiffCard 恢复查询：按 diff_id 返回 DE/WS 当前状态与展示投影，覆盖 pending/accepted/rejected/expired；
/// 找不到的 diff 由前端投只读不可用，不得默认复活为 pending。
pub(crate) fn route_list_diff_details(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let diff_ids = request
        .params
        .get("diffIds")
        .and_then(|v| v.as_array())
        .ok_or_else(|| IpcError::new("DE_DIFF_IDS_REQUIRED", "diffIds 必须是数组", false, false))?
        .iter()
        .filter_map(|v| v.as_str().map(str::trim))
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if diff_ids.is_empty() {
        return Ok(serde_json::Value::Array(Vec::new()));
    }
    let ws = active_ws(state)?;
    let conn = lock_db(state)?;
    let placeholders = (0..diff_ids.len())
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "SELECT d.diff_id, d.workspace_id, d.rel_path, d.status, d.effect_status, d.terminal_reason, d.created_at, \
                d.proposal_batch_id, d.group_index, d.group_count, \
                p.before_snapshot, p.after_content, p.style_ops \
         FROM pending_diffs d LEFT JOIN diff_payloads p ON p.payload_ref = d.payload_ref \
         WHERE d.workspace_id = ? AND d.diff_id IN ({placeholders}) \
         ORDER BY d.created_at ASC"
    );
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| IpcError::new("DE_DB_ERROR", &e.to_string(), true, false))?;
    let mut bind_values: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(diff_ids.len() + 1);
    bind_values.push(&ws.workspace_id);
    for id in &diff_ids {
        bind_values.push(id);
    }
    let rows = stmt
        .query_map(rusqlite::params_from_iter(bind_values), |r| {
            let diff_id: String = r.get(0)?;
            let workspace_id: String = r.get(1)?;
            let rel_path: String = r.get(2)?;
            let status: String = r.get(3)?;
            let effect_status: String = r.get(4)?;
            let terminal_reason: Option<String> = r.get(5)?;
            let created_at: i64 = r.get(6)?;
            let proposal_batch_id: String = r.get(7)?;
            let group_index: u32 = r.get(8)?;
            let group_count: u32 = r.get(9)?;
            let before_snapshot: Option<String> = r.get(10)?;
            let after_content: Option<Vec<u8>> = r.get(11)?;
            let style_ops_json: Option<String> = r.get(12)?;
            let style_ops = style_ops_json
                .as_deref()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
                .unwrap_or(serde_json::Value::Null);
            let table_ops = style_ops
                .get("kind")
                .and_then(|kind| (kind == "table_ops").then_some(()))
                .and_then(|_| style_ops.get("payload").cloned());
            let block_ops = style_ops
                .get("kind")
                .and_then(|kind| (kind == "block_ops").then_some(()))
                .and_then(|_| style_ops.get("payload").cloned());
            let image_ops = style_ops
                .get("kind")
                .and_then(|kind| (kind == "image_ops").then_some(()))
                .and_then(|_| style_ops.get("payload").cloned());
            let image_before = image_ops
                .as_ref()
                .and_then(|_| style_ops.get("imageBefore").cloned());
            let image_after = image_ops
                .as_ref()
                .and_then(|_| style_ops.get("imageAfter").cloned());
            let image_affected_block_ids = image_ops
                .as_ref()
                .and_then(|_| style_ops.get("imageAffectedBlockIds").cloned());
            let projection_meta = style_ops.get("_projection").cloned();
            let after_text = after_content
                .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
                .unwrap_or_default();
            let block_id = block_ops
                .as_ref()
                .and_then(|payload| payload.get("operations"))
                .and_then(serde_json::Value::as_array)
                .and_then(|operations| operations.first())
                .and_then(|operation| operation.get("blockId"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_string);
            let block_before_text = block_ops.as_ref().map(|payload| {
                payload["operations"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|operation| {
                        let id = operation["blockId"].as_str().unwrap_or("");
                        format!("{id}: {}", ag::block_ops::node_text(&operation["beforeNode"]))
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            });
            let block_after_text = block_ops.as_ref().map(|payload| {
                payload["operations"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|operation| {
                        let id = operation["blockId"].as_str().unwrap_or("");
                        format!("{id}: {}", ag::block_ops::node_text(&operation["afterNode"]))
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            });
            Ok(serde_json::json!({
                "diff_id": diff_id,
                "target": {
                    "workspace_id": workspace_id,
                    "doc_type": doc_type_of(&rel_path),
                    "rel_path": rel_path,
                },
                "status": status,
                "effect_status": effect_status,
                "terminal_reason": terminal_reason,
                "created_at": created_at,
                "projection": {
                    "proposal_batch_id": proposal_batch_id,
                    "group_index": group_index,
                    "group_count": group_count,
                    "patch_kind": projection_meta.as_ref().and_then(|meta| meta.get("patchKind")).cloned().unwrap_or_else(|| serde_json::json!(if block_ops.is_some() {
                        "block_ops"
                    } else if table_ops.is_some() {
                        "table_ops"
                    } else if image_ops.is_some() {
                        "image_ops"
                    } else {
                        "replace_document"
                    })),
                    "block_id": projection_meta.as_ref().and_then(|meta| meta.get("blockId")).cloned().unwrap_or_else(|| serde_json::json!(block_id)),
                    "block_index": projection_meta.as_ref().and_then(|meta| meta.get("blockIndex")).cloned(),
                    "before_text": projection_meta.as_ref().and_then(|meta| meta.get("beforeText")).cloned().unwrap_or_else(|| serde_json::json!(block_before_text.unwrap_or_else(|| before_snapshot.unwrap_or_default()))),
                    "after_text": projection_meta.as_ref().and_then(|meta| meta.get("afterText")).cloned().unwrap_or_else(|| serde_json::json!(block_after_text.unwrap_or(after_text))),
                    "location_label": projection_meta.as_ref().and_then(|meta| meta.get("locationLabel")).cloned().unwrap_or_else(|| serde_json::json!("历史修改建议")),
                    "draft_mode": projection_meta.as_ref().and_then(|meta| meta.get("draftMode")).cloned().unwrap_or_else(|| serde_json::json!("background")),
                    "snapshot_source": projection_meta.as_ref().and_then(|meta| meta.get("snapshotSource")).cloned().unwrap_or_else(|| serde_json::json!("disk_snapshot")),
                    "base_session_version": projection_meta.as_ref().and_then(|meta| meta.get("baseSessionVersion")).cloned(),
                    "base_content_hash": projection_meta.as_ref().and_then(|meta| meta.get("baseContentHash")).cloned(),
                    "style_ops": if block_ops.is_some() || table_ops.is_some() || image_ops.is_some() {
                        serde_json::Value::Null
                    } else {
                        style_ops
                    },
                    "block_ops": block_ops,
                    "table_ops": table_ops,
                    "image_ops": image_ops,
                    "image_before": image_before,
                    "image_after": image_after,
                    "image_affected_block_ids": image_affected_block_ids,
                }
            }))
        })
        .map_err(|e| IpcError::new("DE_DB_ERROR", &e.to_string(), true, false))?;
    let values = rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| IpcError::new("DE_DB_ERROR", &e.to_string(), true, false))?;
    Ok(serde_json::Value::Array(values))
}

#[cfg(test)]
mod image_diff_history_tests {
    use super::*;

    fn state_with_image_diff() -> AppState {
        let mut conn = Connection::open_in_memory().unwrap();
        db::migrate(&mut conn).unwrap();
        conn.execute(
            "INSERT INTO workspaces \
             (workspace_id, root_path, is_active, opened_at) VALUES ('ws-image','/w',1,0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO diff_payloads \
             (payload_ref, workspace_id, rel_path, after_content, before_snapshot, is_new_doc, style_ops, created_at) \
             VALUES ('p-image','ws-image','notes/a.docx',X'','',0,?1,0)",
            [serde_json::json!({
                "kind": "image_ops",
                "payloadVersion": 1,
                "payload": {
                    "base": { "sessionVersion": 1, "contentHash": "h1" },
                    "operations": [{
                        "op": "delete_image",
                        "imageId": "image_0_abcd",
                        "fingerprint": "abcd"
                    }]
                }
            })
            .to_string()],
        )
        .unwrap();
        let diff = crate::types::PendingDiff {
            diff_id: "d-image".into(),
            proposal_batch_id: "d-image".into(),
            group_index: 1,
            group_count: 1,
            target: crate::types::DocRef {
                workspace_id: "ws-image".into(),
                rel_path: "notes/a.docx".into(),
                doc_type: "docx".into(),
            },
            origin_module: "AG".into(),
            status: crate::types::DiffStatus::Pending,
            payload_ref: "p-image".into(),
            created_at: 1,
        };
        workspace::diff_apply::persist_diff(&conn, &diff).unwrap();
        AppState {
            db: std::sync::Arc::new(Mutex::new(conn)),
            registry: build_registry(),
            assist_terminals: std::sync::Arc::new(Mutex::new(HashMap::new())),
            active_agent_turns: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn request(command: &str, params: serde_json::Value) -> CommandRequest {
        CommandRequest {
            command: command.to_string(),
            correlation_id: "test-image-history".into(),
            params,
        }
    }

    // covers: BR-DE-DATA-002
    #[test]
    fn image_snapshot_update_is_projected_back_into_history_details() {
        let state = state_with_image_diff();
        let before = serde_json::json!([{
            "imageId": "image_0_abcd",
            "fingerprint": "abcd",
            "ordinal": 0,
            "blockId": "b-img",
            "imagePos": 2,
            "standalone": true,
            "alt": "旧图",
            "width": "240",
            "height": null,
            "mediaRef": "media/image.png",
            "projectSourcePath": "images/image.png",
            "projectSourceHash": "a".repeat(64)
        }]);
        route_update_image_diff_snapshots(
            &request(
                "de.update_image_diff_snapshots",
                serde_json::json!({
                    "diffId": "d-image",
                    "before": before,
                    "after": [],
                    "affectedBlockIds": ["b-img"]
                }),
            ),
            &state,
        )
        .unwrap();

        let details = route_list_diff_details(
            &request(
                "de.list_diffs",
                serde_json::json!({ "diffIds": ["d-image"] }),
            ),
            &state,
        )
        .unwrap();
        assert_eq!(details[0]["projection"]["patch_kind"], "image_ops");
        assert_eq!(details[0]["projection"]["image_before"], before);
        assert_eq!(
            details[0]["projection"]["image_after"],
            serde_json::json!([])
        );
        assert_eq!(
            details[0]["projection"]["image_affected_block_ids"],
            serde_json::json!(["b-img"])
        );
    }

    // covers: BR-DE-STATE-001
    // 既有 BDocx 的接受必须确认当前编辑器草稿；新建 docx 和纯文本仍可走后台落盘。
    #[test]
    fn background_accept_disallows_existing_bdocx_but_allows_new_docx_creation() {
        assert!(!can_accept_background_diff("notes/a.docx", false));
        assert!(!can_accept_background_diff("notes/a.draft.docx", false));
        assert!(can_accept_background_diff("notes/new.docx", true));
        assert!(can_accept_background_diff("notes/a.md", false));
    }
}
