//! 从应用组合根迁入的模块运行时/路由；保持原 command 与状态机行为等价。

use crate::types::DiffProjection;
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

fn escape_html_text(raw: &str) -> String {
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn initial_bdocx_html(content: &str) -> String {
    let trimmed = content.trim();
    if trimmed.starts_with('<') {
        return content.to_string();
    }
    if content.is_empty() {
        return "<p></p>".to_string();
    }
    content
        .split('\n')
        .map(|line| format!("<p>{}</p>", escape_html_text(line)))
        .collect::<Vec<_>>()
        .join("")
}

/// 路由 `ws.open_workspace` 到 WS 生命周期逻辑（PI-WS-05），回 WorkspaceRef 的 JSON 投影。
pub(crate) fn route_open_workspace(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let root_path = request
        .params
        .get("rootPath")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| IpcError::new("WS_ROOT_REQUIRED", "缺少 rootPath 参数", false, false))?;

    let mut conn = state
        .db
        .lock()
        .map_err(|_| IpcError::new("WS_DB_BUSY", "数据库连接繁忙", true, false))?;

    let ws = workspace::lifecycle::open_workspace(&mut conn, root_path)
        .map_err(|e| IpcError::new("WS_OPEN_FAILED", &e.to_string(), true, false))?;

    serde_json::to_value(&ws)
        .map_err(|e| IpcError::new("WS_SERIALIZE_FAILED", &e.to_string(), false, false))
}

/// 序列化 WS 生命周期结果为 JSON 投影（统一错误形状收口）。
pub(crate) fn ws_json<T: serde::Serialize>(value: &T) -> Result<serde_json::Value, IpcError> {
    serde_json::to_value(value)
        .map_err(|e| IpcError::new("WS_SERIALIZE_FAILED", &e.to_string(), false, false))
}

/// 锁定 workspace.db 连接（繁忙即可重试错误）。
pub(crate) fn lock_db<'a>(
    state: &'a AppState,
) -> Result<std::sync::MutexGuard<'a, Connection>, IpcError> {
    state
        .db
        .lock()
        .map_err(|_| IpcError::new("WS_DB_BUSY", "数据库连接繁忙", true, false))
}

/// 路由 `ws.list_workspaces`：回已注册 workspace 列表（最近列表入口，PI-WS-05）。
pub(crate) fn route_list_workspaces(state: &AppState) -> Result<serde_json::Value, IpcError> {
    let conn = lock_db(state)?;
    let list = workspace::lifecycle::list_workspaces(&conn)
        .map_err(|e| IpcError::new("WS_LIST_FAILED", &e.to_string(), true, false))?;
    ws_json(&list)
}

/// 路由 `ws.switch_workspace`：按 workspace_id 切活动态（单活动不变量，BR-WS-PERSIST-002）。
pub(crate) fn route_switch_workspace(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let workspace_id = request
        .params
        .get("workspaceId")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| IpcError::new("WS_ID_REQUIRED", "缺少 workspaceId 参数", false, false))?;
    let mut conn = lock_db(state)?;
    let ws = workspace::lifecycle::switch_workspace(&mut conn, workspace_id)
        .map_err(|e| IpcError::new("WS_SWITCH_FAILED", &e.to_string(), true, false))?;
    ws_json(&ws)
}

/// 路由 `ws.close_workspace`：置非活动（控制态保留可恢复，BR-WS-PERSIST-002）；前端据此回欢迎页。
pub(crate) fn route_close_workspace(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let workspace_id = request
        .params
        .get("workspaceId")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| IpcError::new("WS_ID_REQUIRED", "缺少 workspaceId 参数", false, false))?;
    let conn = lock_db(state)?;
    workspace::lifecycle::close_workspace(&conn, workspace_id)
        .map_err(|e| IpcError::new("WS_CLOSE_FAILED", &e.to_string(), true, false))?;
    Ok(serde_json::json!({ "workspace_id": workspace_id, "closed": true }))
}

/// 路由 `ws.active_workspace`：回当前活动 workspace（启动恢复入口；无则 null）。
pub(crate) fn route_active_workspace(state: &AppState) -> Result<serde_json::Value, IpcError> {
    let conn = lock_db(state)?;
    let active = workspace::lifecycle::active_workspace(&conn)
        .map_err(|e| IpcError::new("WS_ACTIVE_FAILED", &e.to_string(), true, false))?;
    ws_json(&active)
}

/// 路由 `ws.list_docs`：枚举当前活动 workspace root 下文件为 DocRef 列表（X-FORBID-004 守界）。
pub(crate) fn route_list_docs(state: &AppState) -> Result<serde_json::Value, IpcError> {
    let conn = lock_db(state)?;
    let active = workspace::lifecycle::active_workspace(&conn)
        .map_err(|e| IpcError::new("WS_ACTIVE_FAILED", &e.to_string(), true, false))?
        .ok_or_else(|| IpcError::new("WS_NO_ACTIVE", "无活动 workspace", false, false))?;
    drop(conn); // 枚举文件系统期间不持 db 锁
    let docs = workspace::listing::list_docs(std::path::Path::new(&active.root_path))?;
    ws_json(&docs)
}

/// 路由 `ws.search`：活动 workspace 内全文搜索（守界 X-FORBID-004，命中封顶）。
pub(crate) fn route_search(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let query = request
        .params
        .get("query")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let active = active_ws(state)?;
    let hits = workspace::listing::search(std::path::Path::new(&active.root_path), query);
    ws_json(&hits)
}

/// 取活动 workspace（无活动即拒）——文件 I/O 路由的公共前置。
pub(crate) fn active_ws(state: &AppState) -> Result<crate::types::WorkspaceRef, IpcError> {
    let conn = lock_db(state)?;
    workspace::lifecycle::active_workspace(&conn)
        .map_err(|e| IpcError::new("WS_ACTIVE_FAILED", &e.to_string(), true, false))?
        .ok_or_else(|| IpcError::new("WS_NO_ACTIVE", "无活动 workspace", false, false))
}

/// 取请求字符串参数（缺失/空回统一错误形状）。
pub(crate) fn str_param(
    request: &CommandRequest,
    key: &str,
    code: &str,
) -> Result<String, IpcError> {
    request
        .params
        .get(key)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .ok_or_else(|| IpcError::new(code, &format!("缺少 {key} 参数"), false, false))
}

/// 取必须存在的字符串参数，但保留合法空字符串（新建空文档的 LogicalState 基准即为空）。
pub(crate) fn str_param_allow_empty(
    request: &CommandRequest,
    key: &str,
    code: &str,
) -> Result<String, IpcError> {
    request
        .params
        .get(key)
        .and_then(|v| v.as_str())
        .map(str::to_owned)
        .ok_or_else(|| IpcError::new(code, &format!("缺少 {key} 参数"), false, false))
}

#[cfg(test)]
mod command_param_tests {
    use super::*;

    #[test]
    fn empty_document_snapshot_and_decision_replay_preserve_runtime_contracts() {
        let request = CommandRequest {
            command: "ws.project_create_document_bundle".into(),
            correlation_id: "empty-before-snapshot".into(),
            params: serde_json::json!({ "beforeSnapshot": "" }),
        };

        assert_eq!(
            str_param_allow_empty(&request, "beforeSnapshot", "WS_CONTENT_REQUIRED").unwrap(),
            ""
        );
        assert_eq!(
            str_param(&request, "beforeSnapshot", "WS_CONTENT_REQUIRED")
                .unwrap_err()
                .code,
            "WS_CONTENT_REQUIRED"
        );

        let mut conn = Connection::open_in_memory().unwrap();
        db::migrate(&mut conn).unwrap();
        conn.execute(
            "INSERT INTO workspaces (workspace_id, root_path, is_active, opened_at) VALUES ('ws-replay','/tmp/ws-replay',1,0)",
            [],
        )
        .unwrap();
        let task_run_id = ag::task::begin_turn(&conn, "ws-replay", "session-replay", None).unwrap();
        let decision = ag::task::propose_decision(
            &conn,
            &task_run_id,
            "permission",
            serde_json::json!({ "summary": "确认" }),
        )
        .unwrap();
        let state = AppState {
            db: Mutex::new(conn),
            registry: build_registry(),
            assist_terminals: Mutex::new(HashMap::new()),
            active_agent_turns: Arc::new(Mutex::new(HashMap::new())),
        };
        let resolve = |outcome: &str| CommandRequest {
            command: "ag.resolve_decision".into(),
            correlation_id: format!("decision-{outcome}"),
            params: serde_json::json!({
                "requestId": decision.request_id,
                "outcome": outcome,
            }),
        };

        let first = route_resolve_decision(&resolve("confirm"), &state).unwrap();
        assert_eq!(first["already_resolved"], false);
        let replay = route_resolve_decision(&resolve("confirm"), &state).unwrap();
        assert_eq!(replay["already_resolved"], true);
        assert_eq!(replay["task_run_id"], task_run_id);
        assert_eq!(
            route_resolve_decision(&resolve("cancel"), &state)
                .unwrap_err()
                .code,
            "AG_DECISION_ALREADY_RESOLVED"
        );
    }
}

/// 取可选 u64 参数；baseline 允许字符串形态，避免 JS number 精度损失。
pub(crate) fn optional_u64_param(
    request: &CommandRequest,
    key: &str,
    code: &str,
) -> Result<Option<u64>, IpcError> {
    let Some(v) = request.params.get(key) else {
        return Ok(None);
    };
    if v.is_null() {
        return Ok(None);
    }
    if let Some(n) = v.as_u64() {
        return Ok(Some(n));
    }
    if let Some(s) = v.as_str() {
        if s.trim().is_empty() {
            return Ok(None);
        }
        return s
            .parse::<u64>()
            .map(Some)
            .map_err(|_| IpcError::new(code, &format!("{key} 必须是 u64 内容哈希"), false, false));
    }
    Err(IpcError::new(
        code,
        &format!("{key} 必须是 u64 内容哈希"),
        false,
        false,
    ))
}

/// 取布尔参数；缺省时回 default。
pub(crate) fn bool_param(request: &CommandRequest, key: &str, default: bool) -> bool {
    request
        .params
        .get(key)
        .and_then(|v| v.as_bool())
        .unwrap_or(default)
}

/// 解析活动 workspace 内相对路径为绝对路径（X-FORBID-004 守界）。
pub(crate) fn resolve_active(
    state: &AppState,
    rel_path: &str,
) -> Result<(crate::types::WorkspaceRef, std::path::PathBuf), IpcError> {
    let ws = active_ws(state)?;
    let abs = workspace::resolve_in_workspace(std::path::Path::new(&ws.root_path), rel_path)?;
    Ok((ws, abs))
}

/// 路由 `ws.read_doc`：读活动 workspace 内文档内容 + 加载基线（BR-WS-OBS-001 / X-FORBID-004）。
pub(crate) fn route_read_doc(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let (_, abs) = resolve_active(state, &rel_path)?;
    let read = workspace::save::read_doc(&abs)?;
    Ok(serde_json::json!({
        "content": read.content,
        "baseline": read.baseline.to_string(),
    }))
}

/**
 * @GOV
 * codes:    BR-WS-PERSIST-001-EFFECT-WS-WS-SAVE-003, BR-WS-OBS-001-EFFECT-WS-WS-SAVE-003, BR-WS-DATA-001-EFFECT-WS-WS-SAVE-003
 * type:     EFFECT
 * chain:    WS-SAVE
 * rules:    BR-WS-PERSIST-001, BR-WS-OBS-001, BR-WS-DATA-001
 * boundary: in=saveRequestWithOptionalBaselineAndAbsoluteTarget | out=atomicDiskWriteOrConflictError(agentCreateSaveCreatesFileManuallyCreatedUnchanged)
 */
/// 路由 `ws.save_doc`：原子写入，回内容哈希（BR-WS-PERSIST-001 / SM-WS-01）。
/// **代理新建保存＝创建（D-NEW-001~008 / REQ-WS-006，A-WS-M-S-01 §2.1）**：baseline=null（未保存新文档
/// 身份，无磁盘基线）且目标不存在 → 创建语义（create_dir_all 父目录 → 原子落盘 → 时间轴节点 → docx 身份
/// 登记）；baseline=null 且目标已存在 → WS_EXISTS 冲突（V-N4，不覆盖外部创建）。手动新建（ws.create_doc
/// 立即建文件）不在此路径，逻辑不变。
pub(crate) fn route_save_doc<R: tauri::Runtime>(
    request: &CommandRequest,
    state: &AppState,
    app: &tauri::AppHandle<R>,
) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let content = request
        .params
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| IpcError::new("WS_CONTENT_REQUIRED", "缺少 content 参数", false, false))?;
    let baseline = optional_u64_param(request, "baseline", "WS_BASELINE_INVALID")?;
    let override_confirmed = bool_param(request, "override", false);
    let (ws, abs) = resolve_active(state, &rel_path)?;
    // 代理新建保存＝创建（D-NEW / REQ-WS-006，A-WS-M-S-01 §2.1）：未保存新文档身份 = baseline 为 null
    // （无磁盘基线；手动新建 openDocument 读盘后 baseline 非 null，不进入此分支）。
    // - 目标不存在 → 创建语义：父目录创建（代理新建目录可不存在），随后原子落盘；
    // - 目标已存在 → 外部创建介入（V-N4）：WS_EXISTS 冲突，不覆盖、保留未保存内容。
    let unsaved_create = baseline.is_none();
    if unsaved_create {
        if abs.exists() {
            return Err(IpcError::new(
                "WS_EXISTS",
                "保存目标已被外部创建，拒绝覆盖（请改名或合并后重试）",
                false,
                false,
            ));
        }
        if let Some(parent) = abs.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                IpcError::new("WS_IO_ERROR", &format!("创建父目录失败: {e}"), true, false)
            })?;
        }
    }
    let saved = save_atomic_sm(
        app,
        &rel_path,
        &abs,
        content.as_bytes(),
        baseline,
        override_confirmed,
    )?;
    // 文件已落盘后，必须同步确认对应 accepted diff 的效果态。这里不能吞错：否则调用方会把
    // "已保存但历史效果态仍 awaiting_save" 当作完整成功，后续异常清算会产生错误的历史结论。
    // 先记时间轴 save 节点（BR-WS-DATA-002：内容与最近节点快照相同则跳过，内容寻址幂等），
    // 再翻效果态；任一步失败都向调用方暴露（文件已落盘但控制态未完整，重试安全不重复记节点）。
    let conn = lock_db(state)?;
    let doc = crate::types::DocRef {
        workspace_id: ws.workspace_id,
        doc_type: doc_type_of(&rel_path),
        rel_path,
    };
    workspace::timeline::record_save_node(&conn, &doc, content.as_bytes()).map_err(|e| {
        IpcError::new(
            "WS_DB_ERROR",
            &format!("文件已保存，但时间轴节点记录失败: {e}"),
            true,
            false,
        )
    })?;
    // 代理新建 docx 保存＝创建后登记 BDocx 身份（文件树/列举识别；mark_bdocx 内部幂等）。
    if unsaved_create && doc.doc_type == "docx" {
        workspace::listing::mark_bdocx(std::path::Path::new(&ws.root_path), &doc.rel_path)
            .map_err(|e| {
                IpcError::new(
                    "WS_BDOCX_MARK_FAILED",
                    &format!(
                        "文件已保存，但 BDocx 身份登记失败: [{}] {}",
                        e.code, e.message
                    ),
                    true,
                    false,
                )
            })?;
    }
    de::lifecycle::flip_awaiting_save_to_saved(&conn, &doc.workspace_id, &doc.rel_path).map_err(
        |e| {
            IpcError::new(
                "DE_EFFECT_STATUS_SYNC_FAILED",
                &format!("文件已保存，但 diff 效果状态同步失败: {e}"),
                true,
                false,
            )
        },
    )?;
    workspace::operation::mark_create_document_bundles_saved_for_path(
        &conn,
        &doc.workspace_id,
        &doc.rel_path,
    )
    .map_err(|error| IpcError::new("WS_OPERATION_DB_ERROR", &error.to_string(), true, false))?;
    Ok(serde_json::json!({ "baseline": saved.baseline.to_string() }))
}

/// 路由 `ws.persist_diff`：持久化 PendingDiff 为 status=pending（重启恢复，BR-WS-STATE-001）。
pub(crate) fn route_persist_diff(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let diff: crate::types::PendingDiff =
        serde_json::from_value(request.params.clone()).map_err(|e| {
            IpcError::new(
                "WS_DIFF_MALFORMED",
                &format!("PendingDiff 解析失败: {e}"),
                false,
                false,
            )
        })?;
    let conn = lock_db(state)?;
    workspace::diff_apply::persist_diff(&conn, &diff)
        .map_err(|e| IpcError::new("WS_DB_ERROR", &e.to_string(), true, false))?;
    Ok(serde_json::json!({ "diff_id": diff.diff_id }))
}

/// 路由 `ws.list_pending_diffs`：回活动 workspace 未裁决 diff 列表（重启恢复入口）。
pub(crate) fn route_list_pending_diffs(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let status = request.params.get("status").and_then(|v| v.as_str());
    let ws = active_ws(state)?;
    let conn = lock_db(state)?;
    let list = workspace::diff_apply::list_pending_diffs(&conn, &ws.workspace_id, status)
        .map_err(|e| IpcError::new("WS_DB_ERROR", &e.to_string(), true, false))?;
    ws_json(&list)
}

/// 路由 `ws.apply_diff`：accepted 唯一落盘路径——原子写 + 记一个时间轴节点 + 置 accepted（BR-WS-STATE-001）。
/// afterContent 由 DE 产出方提供（payload_ref→内容解析归 DE）；本路由执行 WS 侧落盘 effect。
pub(crate) fn route_apply_diff(
    request: &CommandRequest,
    state: &AppState,
    app: &tauri::AppHandle,
) -> Result<serde_json::Value, IpcError> {
    let diff_id = str_param(request, "diffId", "WS_DIFF_ID_REQUIRED")?;
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
        })?;
    let baseline = optional_u64_param(request, "baseline", "WS_BASELINE_INVALID")?;
    let (ws, abs) = resolve_active(state, &rel_path)?;
    let doc = crate::types::DocRef {
        workspace_id: ws.workspace_id,
        doc_type: std::path::Path::new(&rel_path)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_else(|| "bin".to_string()),
        rel_path,
    };
    let conn = lock_db(state)?;
    // 经 SM-WS-02 落盘机驱动 + 广播（Applying→Applied/ApplyError）。
    let entry = apply_diff_sm(
        app,
        &conn,
        &diff_id,
        &doc,
        &abs,
        after_content.as_bytes(),
        baseline,
    )?;
    ws_json(&entry)
}

/// 路由 `ws.create_doc`：workspace 内新建文档（守界，已存在拒绝；BR-WS-DATA-001）。
pub(crate) fn route_create_doc(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let content = request
        .params
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let (ws, abs) = resolve_active(state, &rel_path)?;
    let baseline = if is_docx_rel_path(&rel_path) {
        let html = initial_bdocx_html(content);
        let docx_bytes = doc::edit_chain::html_to_docx(&html, None)?;
        let baseline = workspace::fileops::create_doc(&abs, &docx_bytes)?;
        workspace::listing::mark_bdocx(std::path::Path::new(&ws.root_path), &rel_path)?;
        baseline
    } else {
        workspace::fileops::create_doc(&abs, content.as_bytes())?
    };
    Ok(serde_json::json!({ "rel_path": rel_path, "baseline": baseline }))
}

/// 路由 `ws.create_folder`：workspace 内建目录（守界；目标为目录则幂等成功，目标为文件拒绝）。
pub(crate) fn route_create_folder(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let (_, abs) = resolve_active(state, &rel_path)?;
    let existed = abs.is_dir();
    workspace::fileops::create_folder(&abs)?;
    Ok(serde_json::json!({ "rel_path": rel_path, "created": !existed, "existed": existed }))
}

/// 路由 `ws.rename_doc`：同 workspace 内重命名/移动（两端守界，目标已存在拒绝；BR-WS-DATA-001 / §8 move）。
/// WS-RESOURCE-WRITE-GUARD-01（F2，BR-WS-STATE-009）：源 dirty 阻断；成功后写 WorkspaceTimeline
/// 条目（manual_rename，可反向恢复）并失效源路径受影响内容 Diff。
pub(crate) fn route_rename_doc(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let new_rel_path = str_param(request, "newRelPath", "WS_NEW_PATH_REQUIRED")?;
    let (ws, _from_abs) = resolve_active(state, &rel_path)?;
    let (_, _to_abs) = resolve_active(state, &new_rel_path)?;
    let conn = lock_db(state)?;
    let entry = crate::workspace::operation::record_manual_rename(
        &conn,
        std::path::Path::new(&ws.root_path),
        &ws.workspace_id,
        &rel_path,
        &new_rel_path,
    )?;
    Ok(serde_json::json!({ "rel_path": new_rel_path, "entry_id": entry.entry_id }))
}

/// 路由 `ws.delete_doc`：删除 workspace 内文档（守界，不存在拒绝；BR-WS-DATA-001）。
/// WS-RESOURCE-WRITE-GUARD-01（F2，BR-WS-STATE-009）：dirty 会话阻断（WS_DELETE_DIRTY_TARGET）；
/// 删除前保存可恢复快照并写 WorkspaceTimeline 条目（manual_delete），删除后失效受影响内容 Diff。
pub(crate) fn route_delete_doc(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let (ws, _abs) = resolve_active(state, &rel_path)?;
    let conn = lock_db(state)?;
    let entry = crate::workspace::operation::record_manual_delete(
        &conn,
        std::path::Path::new(&ws.root_path),
        &ws.workspace_id,
        &rel_path,
        crate::types::WorkspaceResourceKind::File,
        false,
    )?;
    Ok(serde_json::json!({ "rel_path": rel_path, "deleted": true, "entry_id": entry.entry_id }))
}

/// 路由 `ws.delete_folder`：workspace 内删除目录（守界；两级语义 2026-08-06——mode=empty 仅空目录、
/// mode=recursive 递归删除；缺失/文件冲突/非空拒绝；删除后清洗 bdocx_manifest）。
/// WS-RESOURCE-WRITE-GUARD-01（F2，BR-WS-STATE-009）：dirty 会话阻断 + 可恢复快照/时间轴条目 + Diff 失效。
pub(crate) fn route_delete_folder(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let mode = str_param(request, "mode", "WS_MODE_REQUIRED")?;
    if mode != "empty" && mode != "recursive" {
        return Err(IpcError::new(
            "WS_MODE_INVALID",
            "mode 仅支持 empty | recursive",
            false,
            false,
        ));
    }
    let (ws, _abs) = resolve_active(state, &rel_path)?;
    let conn = lock_db(state)?;
    let entry = crate::workspace::operation::record_manual_delete(
        &conn,
        std::path::Path::new(&ws.root_path),
        &ws.workspace_id,
        &rel_path,
        crate::types::WorkspaceResourceKind::Folder,
        mode == "recursive",
    )?;
    Ok(serde_json::json!({ "rel_path": rel_path, "deleted": true, "entry_id": entry.entry_id }))
}

/// 路由 `ws.list_timeline`：统一投影文档内容节点与命中当前路径的资源操作条目。
pub(crate) fn route_list_timeline(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let ws = active_ws(state)?;
    let rel_path = request
        .params
        .get("relPath")
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned);
    let conn = lock_db(state)?;
    let mut unified = Vec::new();
    if let Some(rel_path) = rel_path.as_deref() {
        let doc = crate::types::DocRef {
            workspace_id: ws.workspace_id.clone(),
            doc_type: doc_type_of(rel_path),
            rel_path: rel_path.to_string(),
        };
        let document_entries = workspace::timeline::list_timeline(&conn, &doc)
            .map_err(|e| IpcError::new("WS_DB_ERROR", &e.to_string(), true, false))?;
        unified.extend(document_entries.into_iter().map(|entry| {
            serde_json::json!({
                "entry_id": entry.entry_id,
                "entry_kind": "document",
                "target": entry.target,
                "version_seq": entry.version_seq,
                "label": entry.label,
                "created_at": entry.created_at,
                "recoverable": true,
            })
        }));
    }
    let operation_entries =
        workspace::operation::list_operation_entries(&conn, &ws.workspace_id)
            .map_err(|e| IpcError::new("WS_DB_ERROR", &e.to_string(), true, false))?;
    unified.extend(
        operation_entries
            .into_iter()
            .filter(|entry| {
                rel_path
                    .as_deref()
                    .map(|path| workspace::operation::entry_affects_path(entry, path))
                    .unwrap_or(true)
            })
            .map(|entry| {
                let recoverable = workspace::operation::restore_intents_from_entry(&conn, &entry)
                    .map(|intents| !intents.is_empty())
                    .unwrap_or(false);
                let status = serde_json::to_value(entry.final_status)
                    .ok()
                    .and_then(|value| value.as_str().map(str::to_owned))
                    .unwrap_or_else(|| "unknown".into());
                let target_path = entry
                    .after_manifest
                    .iter()
                    .chain(entry.before_manifest.iter())
                    .map(|baseline| baseline.rel_path.as_str())
                    .next()
                    .unwrap_or("")
                    .to_string();
                serde_json::json!({
                    "entry_id": entry.entry_id,
                    "entry_kind": "workspace_operation",
                    "target": {
                        "workspace_id": ws.workspace_id,
                        "doc_type": doc_type_of(&target_path),
                        "rel_path": target_path,
                    },
                    "version_seq": 0,
                    "label": format!("资源操作 · {status}"),
                    "created_at": entry.created_at,
                    "recoverable": recoverable,
                })
            }),
    );
    unified.sort_by_key(|entry| entry["created_at"].as_i64().unwrap_or(0));
    Ok(serde_json::Value::Array(unified))
}

/// 路由 `ws.preview_node`：经节点 snapshot_ref 取回该版本内容（SM-WS-03 Previewing，快照不可取回则 guard 拒）。
pub(crate) fn route_preview_node(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let entry_id = str_param(request, "entryId", "WS_ENTRY_ID_REQUIRED")?;
    let ws = active_ws(state)?;
    let conn = lock_db(state)?;
    if let Some(entry) = workspace::operation::load_operation_entry_by_entry_id(&conn, &entry_id)
        .map_err(|error| IpcError::new("WS_DB_ERROR", &error.to_string(), true, false))?
    {
        if entry.workspace_id != ws.workspace_id {
            return Err(IpcError::new(
                "WS_OPERATION_WORKSPACE_MISMATCH",
                "资源历史条目不属于当前 workspace",
                false,
                false,
            ));
        }
        return Ok(serde_json::json!({
            "content": serde_json::to_string_pretty(&serde_json::json!({
                "before": entry.before_manifest,
                "after": entry.after_manifest,
                "receipts": entry.item_receipts,
                "compensations": entry.compensation_receipts,
                "status": entry.final_status,
            })).unwrap_or_default(),
            "entry_kind": "workspace_operation",
        }));
    }
    let bytes = workspace::timeline::preview_node(&conn, &entry_id)?;
    Ok(serde_json::json!({
        "content": String::from_utf8_lossy(&bytes),
        "entry_kind": "document",
    }))
}

/// 广播 SM-WS-03 时间轴还原机转移（entry_id 为实例，A-CORE-X-L-04 Task 2-W3）。
pub(crate) fn emit_ws03(
    app: &tauri::AppHandle,
    entry_id: &str,
    st: workspace::timeline::RestoreState,
) {
    sm_events::emit_sm_state(
        app,
        sm_events::SmStatePayload::new(
            "SM-WS-03",
            entry_id,
            &format!("{st:?}"),
            workspace::timeline::is_terminal(st),
        ),
    );
}

/// 路由 `ws.restore_node`：经 SM-WS-03 还原机驱动（Previewing→ConfirmRestore→Restoring→RestoreOk/Fail）→
/// 取回快照→原子写回文档（守界）→记一个新时间轴节点（前进式还原，BR-WS-STATE-002）+ 广播。
pub(crate) fn route_restore_node(
    request: &CommandRequest,
    state: &AppState,
    app: &tauri::AppHandle,
) -> Result<serde_json::Value, IpcError> {
    use workspace::timeline::{transition, RestoreEvent, RestoreState};
    let entry_id = str_param(request, "entryId", "WS_ENTRY_ID_REQUIRED")?;
    let ws = active_ws(state)?;
    let conn = lock_db(state)?;
    if let Some(operation_entry) =
        workspace::operation::load_operation_entry_by_entry_id(&conn, &entry_id)
            .map_err(|error| IpcError::new("WS_DB_ERROR", &error.to_string(), true, false))?
    {
        if operation_entry.workspace_id != ws.workspace_id {
            return Err(IpcError::new(
                "WS_OPERATION_WORKSPACE_MISMATCH",
                "资源历史条目不属于当前 workspace",
                false,
                false,
            ));
        }
        let session_id = str_param(request, "sessionId", "AG_SESSION_ID_REQUIRED")?;
        let task_run_id = ag::task::begin_turn(
            &conn,
            &ws.workspace_id,
            &session_id,
            Some("恢复工作区资源历史"),
        )
        .map_err(|error| IpcError::new("AG_TASK_DB_ERROR", &error.to_string(), true, false))?;
        let intents = workspace::operation::restore_intents_from_entry(&conn, &operation_entry)?;
        drop(conn);
        let synthetic = CommandRequest {
            command: "ws.plan_operation".into(),
            correlation_id: request.correlation_id.clone(),
            params: serde_json::json!({
                "taskRunId": task_run_id,
                "sourceTurnId": format!("timeline-restore-{entry_id}"),
                "operations": intents,
            }),
        };
        return route_plan_workspace_operation(&synthetic, state);
    }
    let (rel_path, snapshot_ref) = workspace::timeline::node_locator(&conn, &entry_id)
        .map_err(|e| IpcError::new("WS_DB_ERROR", &e.to_string(), true, false))?
        .ok_or_else(|| IpcError::new("WS_NODE_NOT_FOUND", "时间轴节点不存在", false, false))?;
    // ① 集成：用户确认还原 → Restoring（经状态机；前端预览态为 Previewing）。
    let restoring = transition(RestoreState::Previewing, RestoreEvent::ConfirmRestore)
        .expect("SM-WS-03: Previewing+ConfirmRestore→Restoring");
    emit_ws03(app, &entry_id, restoring);
    // ② 还原副作用：取快照→原子写回→记新节点；任一失败 → RestoreFail→RestoreError 广播并上抛（现状不破坏）。
    let restore = || -> Result<crate::types::TimelineEntry, IpcError> {
        let content = workspace::timeline::load_snapshot(&conn, &snapshot_ref)
            .map_err(|e| IpcError::new("WS_DB_ERROR", &e.to_string(), true, false))?
            .ok_or_else(|| {
                IpcError::new("WS_SNAPSHOT_UNAVAILABLE", "快照不可取回", false, false)
            })?;
        let abs = workspace::resolve_in_workspace(std::path::Path::new(&ws.root_path), &rel_path)?;
        workspace::save::save_atomic(&abs, &content)?;
        let doc = crate::types::DocRef {
            workspace_id: ws.workspace_id.clone(),
            doc_type: doc_type_of(&rel_path),
            rel_path: rel_path.clone(),
        };
        // 前进式记新节点（不抹历史；复用同一快照内容，内容寻址去重）。
        workspace::timeline::append_node(&conn, &doc, "restore", &snapshot_ref, Some("还原"))
            .map_err(|e| IpcError::new("WS_DB_ERROR", &e.to_string(), true, false))
    };
    match restore() {
        Ok(entry) => {
            let st = transition(restoring, RestoreEvent::RestoreOk)
                .expect("Restoring+RestoreOk→Restored");
            emit_ws03(app, &entry_id, st);
            ws_json(&entry)
        }
        Err(e) => {
            let st = transition(restoring, RestoreEvent::RestoreFail)
                .expect("Restoring+RestoreFail→RestoreError");
            emit_ws03(app, &entry_id, st);
            Err(e)
        }
    }
}

/// 路由 `ws.discard_diff`：rejected/expired 仅置终态，不写文件、不记节点（BR-WS-STATE-001）。
pub(crate) fn route_discard_diff(
    request: &CommandRequest,
    state: &AppState,
    app: &tauri::AppHandle,
) -> Result<serde_json::Value, IpcError> {
    let diff_id = str_param(request, "diffId", "WS_DIFF_ID_REQUIRED")?;
    let terminal = str_param(request, "terminal", "WS_TERMINAL_REQUIRED")?;
    if terminal != "rejected" && terminal != "expired" {
        return Err(IpcError::new(
            "WS_TERMINAL_INVALID",
            "terminal 须为 rejected/expired",
            false,
            false,
        ));
    }
    let conn = lock_db(state)?;
    // 经 SM-WS-02 驱动 + 广播（AwaitingDecision→Discarded，不写文件）。
    discard_diff_sm(app, &conn, &diff_id, &terminal, None)?;
    Ok(serde_json::json!({ "diff_id": diff_id, "status": terminal }))
}

/**
 * @GOV
 * codes:    BR-SYS-STATE-004-QUERY-SYS-APP-CLOSE-001
 * type:     QUERY
 * chain:    APP-CLOSE
 * rules:    BR-SYS-STATE-004
 * boundary: in=activeWorkspace | out=pendingDiffCountAwaitingSaveCountAndDirtyDocuments(dbTruthNotCachedCount)
 */
/// 路由 `sys.inspect_close`：SM-SYS-02 Inspecting/Verifying 态复用的只读清点——当前活动 workspace 的
/// 待裁决 diff 数、未确认写盘的 effect 数与未保存文档列表（含各自是否存在 accepted+awaiting_save 的 diff，
/// 供弹窗详情展示）。
/// 直接查库，不信任调用方传入的缓存计数（BR-SYS-STATE-004：复核必须基于真值）。
pub(crate) fn route_inspect_close(state: &AppState) -> Result<serde_json::Value, IpcError> {
    let ws = active_ws(state)?;
    let conn = lock_db(state)?;
    let pending_diff_count = conn
        .query_row(
            "SELECT COUNT(*) FROM pending_diffs WHERE workspace_id=?1 AND status='pending'",
            [&ws.workspace_id],
            |r| r.get::<_, i64>(0),
        )
        .map_err(|e| IpcError::new("WS_DB_ERROR", &e.to_string(), true, false))?;
    let awaiting_save_count = conn
        .query_row(
            "SELECT COUNT(*) FROM pending_diffs \
             WHERE workspace_id=?1 AND status='accepted' AND effect_status='awaiting_save'",
            [&ws.workspace_id],
            |r| r.get::<_, i64>(0),
        )
        .map_err(|e| IpcError::new("WS_DB_ERROR", &e.to_string(), true, false))?;

    let mut stmt = conn
        .prepare("SELECT rel_path FROM ed_session_snapshots WHERE workspace_id=?1 AND dirty=1")
        .map_err(|e| IpcError::new("WS_DB_ERROR", &e.to_string(), true, false))?;
    let dirty_rel_paths: Vec<String> = stmt
        .query_map([&ws.workspace_id], |r| r.get::<_, String>(0))
        .map_err(|e| IpcError::new("WS_DB_ERROR", &e.to_string(), true, false))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| IpcError::new("WS_DB_ERROR", &e.to_string(), true, false))?;
    drop(stmt);

    let dirty_documents: Vec<serde_json::Value> = dirty_rel_paths
        .iter()
        .map(|rel_path| -> Result<serde_json::Value, IpcError> {
            let has_awaiting: bool = conn
                .query_row(
                    "SELECT COUNT(*) FROM pending_diffs \
                     WHERE workspace_id=?1 AND rel_path=?2 AND status='accepted' AND effect_status='awaiting_save'",
                    rusqlite::params![&ws.workspace_id, rel_path],
                    |r| r.get::<_, i64>(0),
                )
                .map_err(|e| IpcError::new("WS_DB_ERROR", &e.to_string(), true, false))?
                > 0;
            Ok(serde_json::json!({ "relPath": rel_path, "hasAcceptedAwaitingSave": has_awaiting }))
        })
        .collect::<Result<Vec<_>, IpcError>>()?;

    Ok(serde_json::json!({
        "pendingDiffCount": pending_diff_count,
        "awaitingSaveCount": awaiting_save_count,
        "dirtyDocuments": dirty_documents,
    }))
}

/**
 * @GOV
 * codes:    BR-SYS-STATE-004-EFFECT-SYS-APP-CLOSE-002
 * type:     EFFECT
 * chain:    APP-CLOSE
 * rules:    BR-SYS-STATE-004
 * boundary: in=activeWorkspace | out=dirtySnapshotsAndAwaitingSaveEffectsDiscarded(explicitUserChoiceNotAutomatic)
 */
/// 路由 `ws.discard_unsaved_sessions`：关闭前决策入口"放弃未保存修改"分支——用户显式选择后，清空
/// 当前活动 workspace 全部 `ed_session_snapshots.dirty=1` 行，不落盘；并把该 workspace 全部
/// accepted+awaiting_save diff 同步标记效果丢失（BR-SYS-DATA-005）。与异常退出清算共享同一丢弃步骤（`terminal_reason`
/// 标注为 `user_discarded_on_close` 以区分触发源），但本命令由正常关闭流程内的用户显式动作触发。
pub(crate) fn route_discard_unsaved_sessions(
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let ws = active_ws(state)?;
    let conn = lock_db(state)?;
    let (discarded_dirty_snapshots, discarded_awaiting_save_diffs) =
        de::lifecycle::discard_dirty_sessions(&conn, &ws.workspace_id, "user_discarded_on_close")
            .map_err(|e| IpcError::new("WS_DB_ERROR", &e.to_string(), true, false))?;
    Ok(serde_json::json!({
        "discardedDirtySnapshots": discarded_dirty_snapshots,
        "discardedAwaitingSaveDiffs": discarded_awaiting_save_diffs,
    }))
}

// —— SM-WS-02 落盘机集成（A-CORE-X-L-04 Task 2-D）——

/// 广播 SM-WS-02 转移（diff_id 为实例）。
pub(crate) fn emit_ws02<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    diff_id: &str,
    st: workspace::diff_apply::DiffApplyState,
) {
    sm_events::emit_sm_state(
        app,
        sm_events::SmStatePayload::new(
            "SM-WS-02",
            diff_id,
            &format!("{st:?}"),
            workspace::diff_apply::is_terminal(st),
        ),
    );
}

/// 经 SM-WS-02 落盘机驱动 apply：AwaitingDecision→Applying（广播）→ apply_diff effect →
/// ApplyOk→Applied / ApplyFail→ApplyError（广播）。
pub(crate) fn apply_diff_sm(
    app: &tauri::AppHandle,
    conn: &Connection,
    diff_id: &str,
    doc: &crate::types::DocRef,
    abs: &std::path::Path,
    after: &[u8],
    loaded_baseline: Option<u64>,
) -> Result<crate::types::TimelineEntry, IpcError> {
    use workspace::diff_apply::{transition, DiffApplyEvent, DiffApplyState};
    let applying = transition(DiffApplyState::AwaitingDecision, DiffApplyEvent::Accepted)
        .expect("SM-WS-02: AwaitingDecision+Accepted→Applying");
    emit_ws02(app, diff_id, applying);
    match workspace::diff_apply::apply_diff(conn, diff_id, doc, abs, after, loaded_baseline) {
        Ok(entry) => {
            let applied =
                transition(applying, DiffApplyEvent::ApplyOk).expect("Applying+ApplyOk→Applied");
            emit_ws02(app, diff_id, applied);
            Ok(entry)
        }
        Err(e) => {
            let errored = transition(applying, DiffApplyEvent::ApplyFail)
                .expect("Applying+ApplyFail→ApplyError");
            emit_ws02(app, diff_id, errored);
            Err(e)
        }
    }
}

/// 经 SM-WS-02 驱动 discard：AwaitingDecision→Discarded（rejected/expired，不写文件）→广播。
/// `terminal_reason` 可空（BR-SYS-DATA-005）：过期原因/异常退出清算标注，供历史只读展示区分。
pub(crate) fn discard_diff_sm<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    conn: &Connection,
    diff_id: &str,
    terminal: &str,
    terminal_reason: Option<&str>,
) -> Result<(), IpcError> {
    use workspace::diff_apply::{transition, DiffApplyEvent, DiffApplyState};
    let ev = if terminal == "rejected" {
        DiffApplyEvent::Rejected
    } else {
        DiffApplyEvent::Expired
    };
    let discarded = transition(DiffApplyState::AwaitingDecision, ev)
        .expect("SM-WS-02: AwaitingDecision+Rejected/Expired→Discarded");
    workspace::diff_apply::discard_diff(conn, diff_id, terminal, terminal_reason)
        .map_err(|e| IpcError::new("WS_DB_ERROR", &e.to_string(), true, false))?;
    emit_ws02(app, diff_id, discarded);
    Ok(())
}

// —— SM-WS-01 保存机集成（A-CORE-X-L-04 Task 2-C）——
// 脏态（Idle/Dirty）留前端高频本地标记（人类决策 2026-06-27）；保存**动作**经状态机驱动 +
// 广播：Dirty→Save→Saving（落盘开始）→SaveOk→Saved / SaveFail→SaveError。

/// 广播 SM-WS-01 转移（rel_path 为实例，区分多文档会话）。保存机为循环机，无终态（terminal=false）。
pub(crate) fn emit_ws01<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    rel_path: &str,
    st: workspace::save::SaveState,
) {
    sm_events::emit_sm_state(
        app,
        sm_events::SmStatePayload::new("SM-WS-01", rel_path, &format!("{st:?}"), false),
    );
}

/**
 * @GOV
 * codes:    BR-WS-OBS-001-RB-WS-WS-FS-001
 * type:     RB
 * chain:    WS-FS
 * rules:    BR-WS-OBS-001
 * boundary: in=saveRequestWithLoadedBaselineAndCurrentDiskBytes | out=saveAllowedOrConflictError(noSilentOverwrite)
 */
/// 保存前外部变更守卫（BR-WS-OBS-001）：baseline 存在且未 override 时，磁盘当前 hash 必须仍等于
/// 加载基线；不一致即保存冲突，返回统一错误码并保持原文件内容。override=true 只由前端用户显式确认触发。
pub(crate) fn save_with_baseline_guard(
    abs: &std::path::Path,
    content: &[u8],
    loaded_baseline: Option<u64>,
    override_confirmed: bool,
) -> Result<workspace::save::Saved, IpcError> {
    if let Some(expected) = loaded_baseline {
        if !override_confirmed {
            let current = std::fs::read(abs).map_err(|e| {
                IpcError::new(
                    "WS_READ_FAILED",
                    &format!("保存前读取当前磁盘内容失败: {e}"),
                    true,
                    false,
                )
            })?;
            let current_hash = workspace::save::content_hash(&current);
            if current_hash != expected {
                return Err(IpcError::new(
                    "WS_SAVE_CONFLICT",
                    "磁盘内容已在打开后变化，请确认覆盖或重载",
                    false,
                    false,
                ));
            }
        }
    }
    workspace::save::save_atomic(abs, content)
}

/// 经 SM-WS-01 保存机驱动原子落盘：Saving（广播）→ save_atomic effect → Saved / SaveError（广播）。
pub(crate) fn save_atomic_sm<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    rel_path: &str,
    abs: &std::path::Path,
    content: &[u8],
    loaded_baseline: Option<u64>,
    override_confirmed: bool,
) -> Result<workspace::save::Saved, IpcError> {
    use workspace::save::{transition, SaveEvent, SaveState};
    let saving =
        transition(SaveState::Dirty, SaveEvent::Save).expect("SM-WS-01: Dirty+Save→Saving");
    emit_ws01(app, rel_path, saving);
    match save_with_baseline_guard(abs, content, loaded_baseline, override_confirmed) {
        Ok(saved) => {
            let st = transition(saving, SaveEvent::SaveOk).expect("Saving+SaveOk→Saved");
            emit_ws01(app, rel_path, st);
            Ok(saved)
        }
        Err(e) => {
            let st = transition(saving, SaveEvent::SaveFail).expect("Saving+SaveFail→SaveError");
            emit_ws01(app, rel_path, st);
            Err(e)
        }
    }
}

#[cfg(test)]
mod save_guard_tests {
    use super::*;

    fn tmp_path(tag: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "binder_save_guard_{}_{}.txt",
            tag,
            std::process::id()
        ));
        let _ = std::fs::remove_file(&p);
        p
    }

    // covers: BR-WS-OBS-001
    // 保存前磁盘内容偏离加载基线时，不得静默覆盖；用户显式 override 后才允许写入并刷新基线。
    #[test]
    fn save_guard_rejects_external_change_until_override() {
        let path = tmp_path("conflict");
        std::fs::write(&path, b"loaded").unwrap();
        let baseline = workspace::save::content_hash(b"loaded");
        std::fs::write(&path, b"external").unwrap();

        let err = save_with_baseline_guard(&path, b"mine", Some(baseline), false).unwrap_err();
        assert_eq!(err.code, "WS_SAVE_CONFLICT");
        assert_eq!(std::fs::read(&path).unwrap(), b"external");

        let saved = save_with_baseline_guard(&path, b"mine", Some(baseline), true).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"mine");
        assert_eq!(saved.baseline, workspace::save::content_hash(b"mine"));
        let _ = std::fs::remove_file(&path);
    }
}

// —— DE diff 链路（PI-BE-02）：生成→裁决→落盘经 dispatch 接入 ——

/**
 * @GOV
 * codes:    BR-WS-STATE-003-API-WS-WS-OPERATION-002, BR-WS-DATA-003-API-WS-WS-OPERATION-002, BR-WS-OBS-002-API-WS-WS-OPERATION-002
 * type:     API
 * chain:    WS-OPERATION
 * rules:    BR-WS-STATE-003, BR-WS-DATA-003, BR-WS-OBS-002
 * term_ref: TERM-WS-003, TERM-WS-004
 * boundary: in=WorkspaceOperationIntentListAndExecutingTaskRun | out=WorkspaceOperationPlanBoundToDecisionRequest(noDiskMutationBeforeDecision)
 */
pub(crate) fn route_plan_workspace_operation(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let ws = active_ws(state)?;
    let task_run_id = str_param(request, "taskRunId", "AG_TASK_RUN_ID_REQUIRED")?;
    let source_turn_id = request
        .params
        .get("sourceTurnId")
        .and_then(|value| value.as_str());
    let supplied_initial_content_ref = request
        .params
        .get("initialContentRef")
        .and_then(|value| value.as_str());
    let initial_content = request
        .params
        .get("initialContent")
        .and_then(|value| value.as_str());
    let generated_initial_content_ref = initial_content.map(|_| {
        format!(
            "wop-initial-{}-{}",
            source_turn_id.unwrap_or(&task_run_id),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|value| value.as_nanos())
                .unwrap_or(0)
        )
    });
    let initial_content_ref = supplied_initial_content_ref
        .map(str::to_owned)
        .or(generated_initial_content_ref);
    let intents: Vec<workspace::operation::WorkspaceOperationIntent> = request
        .params
        .get("operations")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .ok_or_else(|| {
            IpcError::new(
                "WS_OPERATION_INTENTS_INVALID",
                "operations 必须是受支持的 WorkspaceOperation 意图数组",
                false,
                false,
            )
        })?;
    let conn = lock_db(state)?;
    let tx = conn
        .unchecked_transaction()
        .map_err(|error| IpcError::new("WS_OPERATION_DB_ERROR", &error.to_string(), true, false))?;
    let task_workspace: Option<String> = tx
        .query_row(
            "SELECT workspace_id FROM task_runs WHERE task_run_id=?1",
            [&task_run_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| IpcError::new("AG_TASK_DB_ERROR", &error.to_string(), true, false))?;
    if task_workspace.as_deref() != Some(ws.workspace_id.as_str()) {
        return Err(IpcError::new(
            "WS_OPERATION_WORKSPACE_MISMATCH",
            "TaskRun 与当前 workspace 不匹配",
            false,
            false,
        ));
    }
    if let (Some(payload_ref), Some(content)) = (initial_content_ref.as_deref(), initial_content) {
        workspace::operation::store_payload(
            &tx,
            payload_ref,
            &ws.workspace_id,
            "initial_document_content",
            &content.to_string(),
        )
        .map_err(|error| IpcError::new("WS_OPERATION_DB_ERROR", &error.to_string(), true, false))?;
    }
    let plan = workspace::operation::plan_operation(
        &tx,
        &ws,
        &intents,
        Some(&task_run_id),
        source_turn_id,
        initial_content_ref.as_deref(),
    )?;
    if let Some(initial_content_ref) = plan.initial_content_ref.as_ref() {
        let create_item = plan
            .operation_items
            .iter()
            .find(|item| item.kind == crate::types::WorkspaceOperationKind::CreateDocument)
            .ok_or_else(|| {
                IpcError::new(
                    "WS_CREATE_BUNDLE_INVALID",
                    "initialContent 只能绑定 create_document 操作",
                    false,
                    false,
                )
            })?;
        let rel_path = create_item
            .target_rel_path
            .as_ref()
            .expect("validated create target")
            .clone();
        let bundle = crate::types::CreateDocumentBundle {
            plan_id: plan.plan_id.clone(),
            workspace_id: ws.workspace_id.clone(),
            target: crate::types::DocRef {
                workspace_id: ws.workspace_id.clone(),
                doc_type: doc_type_of(&rel_path),
                rel_path,
            },
            initial_content_ref: initial_content_ref.clone(),
            operation_id: None,
            initial_diff_id: None,
            rejection_guard_ref: None,
            status: "waiting_confirm".into(),
        };
        workspace::operation::upsert_create_document_bundle(&tx, &bundle).map_err(|error| {
            IpcError::new("WS_OPERATION_DB_ERROR", &error.to_string(), true, false)
        })?;
    }
    let operation_summary = plan
        .operation_items
        .iter()
        .map(|item| {
            serde_json::json!({
                "itemId": item.item_id,
                "kind": item.kind,
                "sourcePath": item.source_rel_path,
                "targetPath": item.target_rel_path,
                "resourceKind": item.resource_kind,
                "recursive": item.recursive,
            })
        })
        .collect::<Vec<_>>();
    let decision = ag::task::propose_decision(
        &tx,
        &task_run_id,
        "permission",
        serde_json::json!({
            "workspaceOperationPlanId": plan.plan_id,
            "summary": format!("确认执行 {} 项工作区资源操作", plan.operation_items.len()),
            "detail": "确认后系统将重新核验目标；核验通过后直接修改磁盘，不会再生成资源 DiffCard。",
            "operations": operation_summary,
            "consequences": plan.consequences,
            "failurePolicy": plan.compensation_policy,
            "recoverable": plan.consequences.recoverable,
        }),
    )
    .map_err(|error| IpcError::new("AG_TASK_DB_ERROR", &error.to_string(), true, false))?;
    let plan =
        workspace::operation::bind_decision_request(&tx, &plan.plan_id, &decision.request_id)
            .map_err(|error| {
                IpcError::new("WS_OPERATION_DB_ERROR", &error.to_string(), true, false)
            })?;
    tx.commit()
        .map_err(|error| IpcError::new("WS_OPERATION_DB_ERROR", &error.to_string(), true, false))?;
    eprintln!(
        "[workspace-operation] {}",
        serde_json::json!({
            "event": "decision_proposed",
            "plan_id": &plan.plan_id,
            "request_id": &decision.request_id,
            "item_count": plan.operation_items.len(),
        })
    );
    Ok(serde_json::json!({ "plan": plan, "decision_request": decision }))
}

pub(crate) fn route_get_workspace_operation_plan(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let ws = active_ws(state)?;
    let plan_id = str_param(request, "planId", "WS_OPERATION_PLAN_ID_REQUIRED")?;
    let conn = lock_db(state)?;
    let plan = workspace::operation::load_plan(&conn, &plan_id)
        .map_err(|error| IpcError::new("WS_OPERATION_DB_ERROR", &error.to_string(), true, false))?
        .ok_or_else(|| {
            IpcError::new("WS_OPERATION_NOT_FOUND", "资源操作计划不存在", false, false)
        })?;
    if plan.workspace_id != ws.workspace_id {
        return Err(IpcError::new(
            "WS_OPERATION_WORKSPACE_MISMATCH",
            "资源操作计划不属于当前 workspace",
            false,
            false,
        ));
    }
    ws_json(&plan)
}

pub(crate) fn route_execute_workspace_operation(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let ws = active_ws(state)?;
    let plan_id = str_param(request, "planId", "WS_OPERATION_PLAN_ID_REQUIRED")?;
    let request_id = str_param(request, "requestId", "AG_DECISION_REQUEST_ID_REQUIRED")?;
    let conn = lock_db(state)?;
    let receipt = workspace::operation::execute_operation_plan(&conn, &ws, &plan_id, &request_id)?;
    ws_json(&receipt)
}

pub(crate) fn route_cancel_workspace_operation(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let ws = active_ws(state)?;
    let plan_id = str_param(request, "planId", "WS_OPERATION_PLAN_ID_REQUIRED")?;
    let request_id = str_param(request, "requestId", "AG_DECISION_REQUEST_ID_REQUIRED")?;
    let conn = lock_db(state)?;
    let plan = workspace::operation::cancel_operation_plan(
        &conn,
        &ws.workspace_id,
        &plan_id,
        &request_id,
    )?;
    ws_json(&plan)
}

pub(crate) fn route_list_workspace_operations(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let ws = active_ws(state)?;
    let status = request
        .params
        .get("status")
        .and_then(|value| value.as_str())
        .map(|value| {
            serde_json::from_value::<crate::types::WorkspaceOperationStatus>(
                serde_json::Value::String(value.to_string()),
            )
            .map_err(|_| {
                IpcError::new(
                    "WS_OPERATION_STATUS_INVALID",
                    "未知的 WorkspaceOperation 状态",
                    false,
                    false,
                )
            })
        })
        .transpose()?;
    let conn = lock_db(state)?;
    let plans = workspace::operation::list_plans(&conn, &ws.workspace_id, status)
        .map_err(|error| IpcError::new("WS_OPERATION_DB_ERROR", &error.to_string(), true, false))?;
    ws_json(&plans)
}

/**
 * @GOV
 * codes:    BR-WS-STATE-003-API-WS-WS-OPERATION-004, BR-WS-DATA-003-API-WS-WS-OPERATION-004, BR-WS-OBS-002-API-WS-WS-OPERATION-004
 * type:     API
 * chain:    WS-OPERATION, DE-DIFF, ED-EDIT
 * rules:    BR-WS-STATE-003, BR-WS-DATA-003, BR-WS-OBS-002
 * term_ref: TERM-WS-003, TERM-WS-007, TERM-ED-008, TERM-DE-003
 * boundary: in=CompletedCreateDocumentOperationAndOpenedLogicalStateIdentity | out=BoundInitialContentPendingDiffWithRejectionGuard(noSecondResourceDecisionOrDiskContentWrite)
 */
pub(crate) fn route_project_create_document_bundle(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    use crate::de::generate::{build_payload, generate_diff, ChangeProposal};
    use crate::types::{CreateDocumentRejectionGuard, LocateResult, WorkspaceOperationStatus};

    let ws = active_ws(state)?;
    let plan_id = str_param(request, "planId", "WS_OPERATION_PLAN_ID_REQUIRED")?;
    let session_id = str_param(request, "sessionId", "AG_SESSION_ID_REQUIRED")?;
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let before_snapshot = str_param_allow_empty(request, "beforeSnapshot", "WS_CONTENT_REQUIRED")?;
    let base_session_version = request
        .params
        .get("baseSessionVersion")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| {
            IpcError::new(
                "DE_OPEN_SESSION_BASE_REQUIRED",
                "新建文档内容投影缺少 LogicalState 版本",
                false,
                false,
            )
        })?;
    let base_content_hash = str_param(request, "baseContentHash", "DE_OPEN_SESSION_BASE_REQUIRED")?;
    let conn = lock_db(state)?;
    let bundle = workspace::operation::load_create_document_bundle(&conn, &plan_id)
        .map_err(|error| IpcError::new("WS_OPERATION_DB_ERROR", &error.to_string(), true, false))?
        .ok_or_else(|| {
            IpcError::new(
                "WS_CREATE_BUNDLE_NOT_FOUND",
                "新建文档组合不存在",
                false,
                false,
            )
        })?;
    if bundle.workspace_id != ws.workspace_id || bundle.target.rel_path != rel_path {
        return Err(IpcError::new(
            "WS_CREATE_BUNDLE_IDENTITY_MISMATCH",
            "新建文档组合与当前 workspace 或目标路径不匹配",
            false,
            false,
        ));
    }
    if let Some(diff_id) = bundle.initial_diff_id.as_deref() {
        return Ok(serde_json::json!({
            "bundle": bundle,
            "diff_id": diff_id,
            "already_projected": true,
        }));
    }
    let plan = workspace::operation::load_plan(&conn, &plan_id)
        .map_err(|error| IpcError::new("WS_OPERATION_DB_ERROR", &error.to_string(), true, false))?
        .ok_or_else(|| {
            IpcError::new("WS_OPERATION_NOT_FOUND", "资源操作计划不存在", false, false)
        })?;
    if plan.status != WorkspaceOperationStatus::Completed {
        return Err(IpcError::new(
            "WS_CREATE_BUNDLE_OPERATION_INCOMPLETE",
            "只有已完成的创建资源操作可以投影初始内容",
            false,
            false,
        ));
    }
    let operation_id = bundle.operation_id.clone().ok_or_else(|| {
        IpcError::new(
            "WS_CREATE_BUNDLE_OPERATION_MISSING",
            "新建文档组合缺少资源操作身份",
            false,
            false,
        )
    })?;
    let task_run_id = plan.task_run_id.as_deref().ok_or_else(|| {
        IpcError::new(
            "AG_TASK_RUN_ID_REQUIRED",
            "新建文档组合缺少 TaskRun 身份",
            false,
            false,
        )
    })?;
    let source_turn_id = plan.source_turn_id.as_deref().ok_or_else(|| {
        IpcError::new(
            "AG_TURN_ID_REQUIRED",
            "新建文档组合缺少来源回合身份",
            false,
            false,
        )
    })?;
    let initial_content: String =
        workspace::operation::load_payload(&conn, &bundle.initial_content_ref)
            .map_err(|error| {
                IpcError::new("WS_OPERATION_DB_ERROR", &error.to_string(), true, false)
            })?
            .ok_or_else(|| {
                IpcError::new(
                    "WS_CREATE_BUNDLE_CONTENT_MISSING",
                    "确认前冻结的初始内容不存在",
                    false,
                    false,
                )
            })?;
    let disk_baseline =
        workspace::operation::capture_baseline(std::path::Path::new(&ws.root_path), &rel_path)?;
    if !disk_baseline.exists {
        return Err(IpcError::new(
            "WS_CREATE_BUNDLE_TARGET_MISSING",
            "资源创建完成后目标文件不存在",
            false,
            false,
        ));
    }

    let proposal = ChangeProposal {
        origin_task: task_run_id.to_string(),
        target: bundle.target.clone(),
        after_content: initial_content.as_bytes().to_vec(),
        before_snapshot: Some(before_snapshot.clone()),
        style_ops: None,
    };
    let diff_id = gen_id("diff");
    let payload_ref = format!("pl_{diff_id}");
    let guard_ref = format!("wop-create-guard-{plan_id}");
    let guard = CreateDocumentRejectionGuard {
        plan_id: plan_id.clone(),
        operation_id,
        target: bundle.target.clone(),
        disk_baseline,
        logical_session_version: base_session_version,
        logical_content_hash: base_content_hash.clone(),
        created_at: now_millis(),
    };
    workspace::operation::store_payload(
        &conn,
        &guard_ref,
        &ws.workspace_id,
        "create_document_rejection_guard",
        &guard,
    )
    .map_err(|error| IpcError::new("WS_OPERATION_DB_ERROR", &error.to_string(), true, false))?;
    let mut guarded_bundle = bundle.clone();
    guarded_bundle.rejection_guard_ref = Some(guard_ref.clone());
    guarded_bundle.status = "projecting_initial_diff".into();
    workspace::operation::upsert_create_document_bundle(&conn, &guarded_bundle)
        .map_err(|error| IpcError::new("WS_OPERATION_DB_ERROR", &error.to_string(), true, false))?;

    let projection_write = (|| -> Result<crate::types::CreateDocumentBundle, IpcError> {
        let pending = generate_diff(
            &proposal,
            &diff_id,
            "AG",
            &payload_ref,
            now_millis(),
            LocateResult::Located,
        )?;
        let payload = build_payload(&proposal);
        let tx = conn.unchecked_transaction().map_err(|error| {
            IpcError::new("WS_OPERATION_DB_ERROR", &error.to_string(), true, false)
        })?;
        crate::de::store::persist_payload(&tx, &payload_ref, &payload)
            .map_err(|error| IpcError::new("DE_DB_ERROR", &error.to_string(), true, false))?;
        workspace::diff_apply::persist_diff(&tx, &pending)
            .map_err(|error| IpcError::new("DE_DB_ERROR", &error.to_string(), true, false))?;
        ag::task::create_preview_checkpoint(
            &tx,
            task_run_id,
            &ws.workspace_id,
            &session_id,
            source_turn_id,
            &diff_id,
            &rel_path,
        )
        .map_err(|error| IpcError::new("AG_TASK_DB_ERROR", &error.to_string(), true, false))?;
        let mut updated_bundle = guarded_bundle.clone();
        updated_bundle.initial_diff_id = Some(diff_id.clone());
        updated_bundle.status = "initial_diff_pending".into();
        workspace::operation::upsert_create_document_bundle(&tx, &updated_bundle).map_err(
            |error| IpcError::new("WS_OPERATION_DB_ERROR", &error.to_string(), true, false),
        )?;
        tx.commit().map_err(|error| {
            IpcError::new("WS_OPERATION_DB_ERROR", &error.to_string(), true, false)
        })?;
        Ok(updated_bundle)
    })();
    let updated_bundle = match projection_write {
        Ok(bundle) => bundle,
        Err(projection_error) => {
            match workspace::operation::compensate_create_document_projection_failure(
                &conn,
                &ws,
                &plan_id,
                base_session_version,
                &base_content_hash,
            ) {
                Ok(entry) => {
                    let _ =
                        ag::task::settle_turn(&conn, task_run_id, ag::task::TurnOutcome::Failed);
                    return Ok(serde_json::json!({
                        "bundle": workspace::operation::load_create_document_bundle(&conn, &plan_id)
                            .ok().flatten(),
                        "projection_failed_compensated": true,
                        "workspace_timeline_entry_id": entry.entry_id,
                        "error": projection_error,
                    }));
                }
                Err(compensation_error) => return Err(compensation_error),
            }
        }
    };
    let projection = DiffProjection {
        diff_id: diff_id.clone(),
        rel_path: rel_path.clone(),
        patch_kind: "replace_document".into(),
        block_id: None,
        block_index: None,
        before_text: before_snapshot,
        after_text: initial_content,
        location_label: "新文档初始内容".into(),
        draft_mode: "current_editor".into(),
        snapshot_source: "active_editor_session".into(),
        base_session_version: Some(base_session_version),
        base_content_hash: Some(base_content_hash),
        style_ops: None,
        block_ops: None,
        table_ops: None,
        image_ops: None,
        proposal_batch_id: diff_id.clone(),
        group_index: 1,
        group_count: 1,
        preview_checkpoint_required: true,
        cross_doc: Some(false),
    };
    Ok(serde_json::json!({
        "bundle": updated_bundle,
        "diff_id": diff_id,
        "projection": projection,
        "task_run_id": task_run_id,
        "source_turn_id": source_turn_id,
        "already_projected": false,
    }))
}

pub(crate) fn route_compensate_create_document_open_failure(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let ws = active_ws(state)?;
    let plan_id = str_param(request, "planId", "WS_OPERATION_PLAN_ID_REQUIRED")?;
    let conn = lock_db(state)?;
    let entry =
        workspace::operation::compensate_create_document_open_failure(&conn, &ws, &plan_id)?;
    if let Some(plan) = workspace::operation::load_plan(&conn, &plan_id)
        .map_err(|error| IpcError::new("WS_OPERATION_DB_ERROR", &error.to_string(), true, false))?
    {
        if let Some(task_run_id) = plan.task_run_id.as_deref() {
            let _ = ag::task::settle_turn(&conn, task_run_id, ag::task::TurnOutcome::Failed);
        }
    }
    ws_json(&entry)
}

pub(crate) fn route_compensate_create_document_projection_preflight_failure(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let ws = active_ws(state)?;
    let plan_id = str_param(request, "planId", "WS_OPERATION_PLAN_ID_REQUIRED")?;
    let conn = lock_db(state)?;
    let entry = workspace::operation::compensate_create_document_projection_preflight_failure(
        &conn, &ws, &plan_id,
    )?;
    if let Some(plan) = workspace::operation::load_plan(&conn, &plan_id)
        .map_err(|error| IpcError::new("WS_OPERATION_DB_ERROR", &error.to_string(), true, false))?
    {
        if let Some(task_run_id) = plan.task_run_id.as_deref() {
            let _ = ag::task::settle_turn(&conn, task_run_id, ag::task::TurnOutcome::Failed);
        }
    }
    ws_json(&entry)
}

pub(crate) fn route_compensate_create_document_preview_failure(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let ws = active_ws(state)?;
    let diff_id = str_param(request, "diffId", "WS_DIFF_ID_REQUIRED")?;
    let logical_session_version = request
        .params
        .get("logicalSessionVersion")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| {
            IpcError::new(
                "WS_CREATE_BUNDLE_LOGICAL_BASE_REQUIRED",
                "新建文档预览失败补偿缺少 LogicalState 版本",
                false,
                false,
            )
        })?;
    let logical_content_hash = str_param(
        request,
        "logicalContentHash",
        "WS_CREATE_BUNDLE_LOGICAL_BASE_REQUIRED",
    )?;
    let conn = lock_db(state)?;
    let Some(bundle) = workspace::operation::load_create_document_bundle_by_diff(&conn, &diff_id)
        .map_err(|error| {
        IpcError::new("WS_OPERATION_DB_ERROR", &error.to_string(), true, false)
    })?
    else {
        return Ok(serde_json::json!({ "compensated": false, "not_create_bundle": true }));
    };
    let entry = workspace::operation::compensate_create_document_projection_failure(
        &conn,
        &ws,
        &bundle.plan_id,
        logical_session_version,
        &logical_content_hash,
    )?;
    if let Some(plan) = workspace::operation::load_plan(&conn, &bundle.plan_id)
        .map_err(|error| IpcError::new("WS_OPERATION_DB_ERROR", &error.to_string(), true, false))?
    {
        if let Some(task_run_id) = plan.task_run_id.as_deref() {
            let _ = ag::task::settle_turn(&conn, task_run_id, ag::task::TurnOutcome::Failed);
        }
    }
    Ok(serde_json::json!({
        "compensated": true,
        "workspace_timeline_entry_id": entry.entry_id,
    }))
}
