//! Binder 后端骨架（Tauri 2）。
//! 模块边界承接 A-SYS-C-T-01 §4 分层与各模块 Plan Item 的文件范围；
//! 各模块为 G3 脚手架占位，@GOV 块在 G4 实现时随实际逻辑落入对应模块。
//! 分层约束 X-SEQ-002：前端经 command 触达，后端经领域逻辑触达 FS/db。

use std::{collections::HashMap, sync::Mutex};
use tauri::{Manager, State};

// —— 系统底座（PI-SYS-01~04）——
pub mod db; // PI-SYS-03 workspace.db schema 与版本化迁移
pub mod ipc; // PI-SYS-01 IPC 信封/错误/command 注册表
pub mod session;
pub mod sm_events; // A-CORE-X-L-04 §5 状态机运行时广播（sm:{id}:state_changed）
pub mod types; // PI-SYS-02 跨模块共享数据结构 // PI-SYS-04 会话与订阅门禁状态机骨架

// —— P2 编辑核心 ——
pub mod doc;
pub mod workspace; // PI-WS-* 工作台 // PI-DOC-* 文档转换（编辑链/预览链）

// —— P3 代理与 Diff Edit ——
pub mod ag; // PI-AG-* 代理（门禁前置/工具/辅助）
pub mod de; // PI-DE-* Diff Edit（diff 终态；旧称对话编辑，2026-07-04 更名）
pub mod reference; // PI-REF-* 引用（@mention）

// —— P4 上层能力 ——
pub mod ast; // PI-AST-* 记忆
pub mod cbt; // PI-CBT-* 构建
pub mod kb; // PI-KB-* 知识库
pub mod prv;
pub mod tpl; // PI-TPL-* 模板/工作流 // PI-PRV-* 预览输出

// —— 商业化客户端侧 ——
pub mod acct; // PI-ACCT-* 账号会话
pub mod ent; // PI-ENT-* 订阅门控

use ipc::{AgTurnEvent, CommandRegistry, CommandRequest, CommandResponse, IpcError};
use rusqlite::Connection;
use types::ReferencePayload;
#[cfg(test)]
use types::ReferenceSessionMessage;

/// 应用运行时状态：workspace.db 连接 + command 注册表（PI-FE-00 联通后端）。
struct AppState {
    db: Mutex<Connection>,
    registry: CommandRegistry,
    assist_terminals: Mutex<HashMap<String, ag::assist::AssistState>>,
}

/// 已注册 command 清单（实现期各模块在此登记；未注册即按 BR-SYS-DATA-003 拒绝）。
fn build_registry() -> CommandRegistry {
    let mut reg = CommandRegistry::new();
    reg.register("ws.open_workspace");
    reg.register("ws.list_workspaces");
    reg.register("ws.switch_workspace");
    reg.register("ws.close_workspace");
    reg.register("ws.active_workspace");
    reg.register("ws.list_docs");
    reg.register("ws.search");
    reg.register("ws.read_doc");
    reg.register("ws.save_doc");
    reg.register("ws.persist_diff");
    reg.register("ws.list_pending_diffs");
    reg.register("ws.apply_diff");
    reg.register("ws.discard_diff");
    reg.register("ws.create_doc");
    reg.register("ws.create_folder");
    reg.register("ws.rename_doc");
    reg.register("ws.delete_doc");
    reg.register("prv.preview");
    reg.register("prv.export");
    reg.register("prv.media_src");
    reg.register("prv.table_data");
    reg.register("doc.create_draft");
    reg.register("doc.open_for_edit");
    reg.register("doc.save_from_edit");
    reg.register("ws.list_timeline");
    reg.register("ws.preview_node");
    reg.register("ws.restore_node");
    reg.register("de.generate_diff");
    reg.register("de.accept_diff");
    reg.register("de.reject_diff");
    reg.register("de.list_diffs");
    reg.register("ag.start_turn");
    reg.register("ag.list_blocks");
    reg
}

/**
 * @GOV
 * codes:    BR-SYS-DATA-003-API-SYS-IPC-CALL-002
 * type:     API
 * chain:    IPC-CALL
 * rules:    BR-SYS-DATA-003
 * term_ref: TERM-WS-001
 * boundary: in=CommandRequest | out=CommandResponse(workspaceRefOrIpcError)
 */
/// 统一 command 分发器（BR-SYS-DATA-003）：前端经此单一 Tauri 入口投递 CommandRequest 信封，
/// 经注册表校验后路由到领域逻辑（X-SEQ-002）；未注册 command 一律按统一错误形状拒绝，不进入路由。
/// 物理传输选型（单 dispatch vs per-command，SY-02）为 A 类债务，本入口落 envelope 路由骨架。
#[tauri::command]
fn dispatch(
    app: tauri::AppHandle,
    request: CommandRequest,
    state: State<'_, AppState>,
) -> CommandResponse {
    let correlation_id = request.correlation_id.clone();
    if let Err(err) = state.registry.validate(&request) {
        return CommandResponse {
            correlation_id,
            result: None,
            error: Some(err),
        };
    }
    let routed = match request.command.as_str() {
        "ws.open_workspace" => route_open_workspace(&request, &state),
        "ws.list_workspaces" => route_list_workspaces(&state),
        "ws.switch_workspace" => route_switch_workspace(&request, &state),
        "ws.close_workspace" => route_close_workspace(&request, &state),
        "ws.active_workspace" => route_active_workspace(&state),
        "ws.list_docs" => route_list_docs(&state),
        "ws.search" => route_search(&request, &state),
        "ws.read_doc" => route_read_doc(&request, &state),
        "ws.save_doc" => route_save_doc(&request, &state, &app),
        "ws.persist_diff" => route_persist_diff(&request, &state),
        "ws.list_pending_diffs" => route_list_pending_diffs(&request, &state),
        "ws.apply_diff" => route_apply_diff(&request, &state, &app),
        "ws.discard_diff" => route_discard_diff(&request, &state, &app),
        "ws.create_doc" => route_create_doc(&request, &state),
        "ws.create_folder" => route_create_folder(&request, &state),
        "ws.rename_doc" => route_rename_doc(&request, &state),
        "ws.delete_doc" => route_delete_doc(&request, &state),
        "prv.preview" => route_preview(&request, &state),
        "prv.export" => route_export(&request, &state),
        "prv.media_src" => route_media_src(&request, &state, &app),
        "prv.table_data" => route_table_data(&request, &state),
        "doc.create_draft" => route_create_draft(&request, &state),
        "doc.open_for_edit" => route_open_for_edit(&request, &state, &app),
        "doc.save_from_edit" => route_save_from_edit(&request, &state, &app),
        "ws.list_timeline" => route_list_timeline(&request, &state),
        "ws.preview_node" => route_preview_node(&request, &state),
        "ws.restore_node" => route_restore_node(&request, &state, &app),
        "de.generate_diff" => route_generate_diff(&request, &state),
        "de.accept_draft_diff" => route_accept_draft_diff(&request, &state, &app),
        "de.accept_diff" => route_accept_diff(&request, &state, &app),
        "de.reject_diff" => route_reject_diff(&request, &state, &app),
        "de.list_diffs" => route_list_diffs(&request, &state),
        "ag.start_turn" => route_start_turn(&request),
        "ag.list_blocks" => route_list_blocks(&request, &state),
        // 注册表已放行但路由缺失：实现期补全各模块 command（视为内部错误）。
        other => Err(IpcError::new(
            "IPC_ROUTE_MISSING",
            &format!("command 已注册但未接线: {other}"),
            false,
            false,
        )),
    };
    match routed {
        Ok(result) => CommandResponse {
            correlation_id,
            result: Some(result),
            error: None,
        },
        Err(error) => CommandResponse {
            correlation_id,
            result: None,
            error: Some(error),
        },
    }
}

/// 路由 `ws.open_workspace` 到 WS 生命周期逻辑（PI-WS-05），回 WorkspaceRef 的 JSON 投影。
fn route_open_workspace(
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
fn ws_json<T: serde::Serialize>(value: &T) -> Result<serde_json::Value, IpcError> {
    serde_json::to_value(value)
        .map_err(|e| IpcError::new("WS_SERIALIZE_FAILED", &e.to_string(), false, false))
}

/// 锁定 workspace.db 连接（繁忙即可重试错误）。
fn lock_db<'a>(state: &'a AppState) -> Result<std::sync::MutexGuard<'a, Connection>, IpcError> {
    state
        .db
        .lock()
        .map_err(|_| IpcError::new("WS_DB_BUSY", "数据库连接繁忙", true, false))
}

/// 路由 `ws.list_workspaces`：回已注册 workspace 列表（最近列表入口，PI-WS-05）。
fn route_list_workspaces(state: &AppState) -> Result<serde_json::Value, IpcError> {
    let conn = lock_db(state)?;
    let list = workspace::lifecycle::list_workspaces(&conn)
        .map_err(|e| IpcError::new("WS_LIST_FAILED", &e.to_string(), true, false))?;
    ws_json(&list)
}

/// 路由 `ws.switch_workspace`：按 workspace_id 切活动态（单活动不变量，BR-WS-PERSIST-002）。
fn route_switch_workspace(
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
fn route_close_workspace(
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
fn route_active_workspace(state: &AppState) -> Result<serde_json::Value, IpcError> {
    let conn = lock_db(state)?;
    let active = workspace::lifecycle::active_workspace(&conn)
        .map_err(|e| IpcError::new("WS_ACTIVE_FAILED", &e.to_string(), true, false))?;
    ws_json(&active)
}

/// 路由 `ws.list_docs`：枚举当前活动 workspace root 下文件为 DocRef 列表（X-FORBID-004 守界）。
fn route_list_docs(state: &AppState) -> Result<serde_json::Value, IpcError> {
    let conn = lock_db(state)?;
    let active = workspace::lifecycle::active_workspace(&conn)
        .map_err(|e| IpcError::new("WS_ACTIVE_FAILED", &e.to_string(), true, false))?
        .ok_or_else(|| IpcError::new("WS_NO_ACTIVE", "无活动 workspace", false, false))?;
    drop(conn); // 枚举文件系统期间不持 db 锁
    let docs = workspace::listing::list_docs(std::path::Path::new(&active.root_path))?;
    ws_json(&docs)
}

/// 路由 `ws.search`：活动 workspace 内全文搜索（守界 X-FORBID-004，命中封顶）。
fn route_search(request: &CommandRequest, state: &AppState) -> Result<serde_json::Value, IpcError> {
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
fn active_ws(state: &AppState) -> Result<crate::types::WorkspaceRef, IpcError> {
    let conn = lock_db(state)?;
    workspace::lifecycle::active_workspace(&conn)
        .map_err(|e| IpcError::new("WS_ACTIVE_FAILED", &e.to_string(), true, false))?
        .ok_or_else(|| IpcError::new("WS_NO_ACTIVE", "无活动 workspace", false, false))
}

/// 取请求字符串参数（缺失/空回统一错误形状）。
fn str_param(request: &CommandRequest, key: &str, code: &str) -> Result<String, IpcError> {
    request
        .params
        .get(key)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .ok_or_else(|| IpcError::new(code, &format!("缺少 {key} 参数"), false, false))
}

/// 取可选 u64 参数；baseline 允许字符串形态，避免 JS number 精度损失。
fn optional_u64_param(
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
fn bool_param(request: &CommandRequest, key: &str, default: bool) -> bool {
    request
        .params
        .get(key)
        .and_then(|v| v.as_bool())
        .unwrap_or(default)
}

/// 解析活动 workspace 内相对路径为绝对路径（X-FORBID-004 守界）。
fn resolve_active(
    state: &AppState,
    rel_path: &str,
) -> Result<(crate::types::WorkspaceRef, std::path::PathBuf), IpcError> {
    let ws = active_ws(state)?;
    let abs = workspace::resolve_in_workspace(std::path::Path::new(&ws.root_path), rel_path)?;
    Ok((ws, abs))
}

/// 路由 `ws.read_doc`：读活动 workspace 内文档内容 + 加载基线（BR-WS-OBS-001 / X-FORBID-004）。
fn route_read_doc(
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

/// 路由 `ws.save_doc`：原子写入，回内容哈希（BR-WS-PERSIST-001 / SM-WS-01）。
fn route_save_doc(
    request: &CommandRequest,
    state: &AppState,
    app: &tauri::AppHandle,
) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let content = request
        .params
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| IpcError::new("WS_CONTENT_REQUIRED", "缺少 content 参数", false, false))?;
    let baseline = optional_u64_param(request, "baseline", "WS_BASELINE_INVALID")?;
    let override_confirmed = bool_param(request, "override", false);
    let (_, abs) = resolve_active(state, &rel_path)?;
    let saved = save_atomic_sm(
        app,
        &rel_path,
        &abs,
        content.as_bytes(),
        baseline,
        override_confirmed,
    )?;
    Ok(serde_json::json!({ "baseline": saved.baseline.to_string() }))
}

/// 路由 `ws.persist_diff`：持久化 PendingDiff 为 status=pending（重启恢复，BR-WS-STATE-001）。
fn route_persist_diff(
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
fn route_list_pending_diffs(
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
fn route_apply_diff(
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
fn route_create_doc(
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
        let docx_bytes = doc::edit_chain::html_to_docx(&html)?;
        let baseline = workspace::fileops::create_doc(&abs, &docx_bytes)?;
        workspace::listing::mark_bdocx(std::path::Path::new(&ws.root_path), &rel_path)?;
        baseline
    } else {
        workspace::fileops::create_doc(&abs, content.as_bytes())?
    };
    Ok(serde_json::json!({ "rel_path": rel_path, "baseline": baseline }))
}

/// 路由 `ws.create_folder`：workspace 内建目录（守界；目标为目录则幂等成功，目标为文件拒绝）。
fn route_create_folder(
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
fn route_rename_doc(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let new_rel_path = str_param(request, "newRelPath", "WS_NEW_PATH_REQUIRED")?;
    let (_, from_abs) = resolve_active(state, &rel_path)?;
    let (_, to_abs) = resolve_active(state, &new_rel_path)?;
    workspace::fileops::rename_doc(&from_abs, &to_abs)?;
    Ok(serde_json::json!({ "rel_path": new_rel_path }))
}

/// 路由 `ws.delete_doc`：删除 workspace 内文档（守界，不存在拒绝；BR-WS-DATA-001）。
fn route_delete_doc(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let (_, abs) = resolve_active(state, &rel_path)?;
    workspace::fileops::delete_doc(&abs)?;
    Ok(serde_json::json!({ "rel_path": rel_path, "deleted": true }))
}

/// 路由 `prv.preview`：docx/Excel/PPT 经 DOC 预览链出只读 PDF 字节（守界 X-FORBID-004，只读 PRV 硬边界）。
fn route_preview(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let (_, abs) = resolve_active(state, &rel_path)?;
    let ext = std::path::Path::new(&rel_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let bytes = prv::render_preview(&abs, ext)?;
    Ok(serde_json::json!({ "format": "pdf", "bytes": bytes }))
}

/// 路由 `prv.media_src`：守界解析媒体文件绝对路径（X-FORBID-004）+ 仅把**该单个文件**加入 asset 协议
/// scope（最小暴露，不开放整目录）；前端 convertFileSrc(abs) 流式播放音视频（不进内存，只读）。
fn route_media_src(
    request: &CommandRequest,
    state: &AppState,
    app: &tauri::AppHandle,
) -> Result<serde_json::Value, IpcError> {
    use tauri::Manager;
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let (_, abs) = resolve_active(state, &rel_path)?;
    if !abs.is_file() {
        return Err(IpcError::new(
            "PRV_NOT_FOUND",
            "媒体文件不存在",
            false,
            false,
        ));
    }
    app.asset_protocol_scope().allow_file(&abs).map_err(|e| {
        IpcError::new(
            "PRV_SCOPE_FAILED",
            &format!("授权媒体访问失败: {e}"),
            false,
            false,
        )
    })?;
    Ok(serde_json::json!({ "abs_path": abs.to_string_lossy() }))
}

/// 路由 `prv.table_data`（2026-07-03 #1 表格式预览）：xlsx/xls → 结构化表格数据（DOC calamine 管线，
/// X-SEQ-004；只读值投影，BR-PRV-DATA-001）。守界（X-FORBID-004）；csv/tsv 为文本格式由前端解析不经此。
fn route_table_data(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let (_, abs) = resolve_active(state, &rel_path)?;
    if !abs.is_file() {
        return Err(IpcError::new(
            "PRV_NOT_FOUND",
            "表格文件不存在",
            false,
            false,
        ));
    }
    let (sheets, truncated) = doc::table_preview::xlsx_to_sheets(&abs)?;
    Ok(serde_json::json!({ "sheets": sheets, "truncated": truncated }))
}

/// 路由 `prv.export`：源文档导出为目标格式，落点 = 同目录同名换扩展（workspace 内，守界 X-FORBID-004）。
fn route_export(request: &CommandRequest, state: &AppState) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let format = str_param(request, "format", "PRV_FORMAT_REQUIRED")?;
    let src_ext = std::path::Path::new(&rel_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_string();
    // 落点：源 rel_path 去扩展 + 目标扩展（同目录）。
    let stem = rel_path
        .rsplit_once('.')
        .map(|(s, _)| s)
        .unwrap_or(&rel_path);
    let dest_rel = format!("{stem}.{format}");
    let (_, abs_src) = resolve_active(state, &rel_path)?;
    let (_, abs_dest) = resolve_active(state, &dest_rel)?;
    prv::export(&abs_src, &src_ext, &format, &abs_dest)?;
    Ok(serde_json::json!({ "rel_path": dest_rel }))
}

/// 路由 `doc.open_for_edit`（A-DOC-I-P-01 §2 编辑链）：docx → 可编辑 HTML（DOC 编辑链 docx+styles +
/// preserve-styles.lua，保留样式子集供 TipTap）；回 baseline（docx 字节内容哈希）供保存冲突保护
/// （SM-WS-01）。转换失败回 convert_failed 不建会话（对接 SM-DOC-01 OpenError / ed loadFail）。
/// 路由 `doc.create_draft`（旧版成熟流程）：原生 docx → 同目录草稿副本 `{stem}.draft.{ext}`（守界
/// X-FORBID-004）。原生 docx 点击为只读预览，"创建草稿进行编辑"后草稿直接进编辑模式（doc.open_for_edit）。
/// 返回草稿 rel_path。已存在草稿先覆盖。
fn route_create_draft(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let p = std::path::Path::new(&rel_path);
    let stem = p
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| IpcError::new("DOC_PATH_INVALID", "无法解析文件名", false, false))?;
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("docx");
    let parent = p
        .parent()
        .map(|x| x.to_string_lossy().to_string())
        .filter(|s| !s.is_empty());
    let draft_rel = match parent {
        Some(d) => format!("{d}/{stem}.draft.{ext}"),
        None => format!("{stem}.draft.{ext}"),
    };
    let (ws, abs_src) = resolve_active(state, &rel_path)?;
    let (_, abs_draft) = resolve_active(state, &draft_rel)?;
    if !abs_src.exists() {
        return Err(IpcError::new("DOC_NOT_FOUND", "原文档不存在", false, false));
    }
    let _ = std::fs::remove_file(&abs_draft); // 已存在草稿先覆盖
    std::fs::copy(&abs_src, &abs_draft).map_err(|e| {
        IpcError::new(
            "DOC_DRAFT_FAILED",
            &format!("创建草稿失败: {e}"),
            true,
            false,
        )
    })?;
    workspace::listing::mark_bdocx(std::path::Path::new(&ws.root_path), &draft_rel)?;
    Ok(serde_json::json!({ "rel_path": draft_rel }))
}

/// 广播 SM-DOC-01 docx 开/存机转移（rel_path 为实例，A-CORE-X-L-04 Task 2-DOC）。循环机无终态。
fn emit_doc01(app: &tauri::AppHandle, rel_path: &str, st: doc::convert::DocConvertState) {
    sm_events::emit_sm_state(
        app,
        sm_events::SmStatePayload::new("SM-DOC-01", rel_path, &format!("{st:?}"), false),
    );
}

/// 递归授权 extract-media 工作目录下所有抽取媒体文件进入 asset 协议 scope（编辑链图文呈现）。
/// 逐文件最小暴露（不开放整目录，同 prv.media_src 纪律）；work_dir 按文档 baseline 隔离，仅本 docx 媒体。
/// 授权失败不阻断打开（个别图片无法呈现降级为 broken img，不影响文本编辑）。
fn authorize_media_files(app: &tauri::AppHandle, dir: &std::path::Path) {
    use tauri::Manager;
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            authorize_media_files(app, &p);
        } else if p.is_file() {
            let _ = app.asset_protocol_scope().allow_file(&p);
        }
    }
}

fn route_open_for_edit(
    request: &CommandRequest,
    state: &AppState,
    app: &tauri::AppHandle,
) -> Result<serde_json::Value, IpcError> {
    use doc::convert::{transition, DocConvertEvent, DocConvertState};
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let (ws, abs) = resolve_active(state, &rel_path)?;
    if is_docx_rel_path(&rel_path)
        && !workspace::listing::is_bdocx(std::path::Path::new(&ws.root_path), &rel_path)
    {
        return Err(IpcError::new(
            "DOC_NOT_BDOCX",
            "原生 docx 不能直接编辑，请先创建 b-docx",
            false,
            false,
        ));
    }
    // 加载基线：docx 原始字节内容哈希（供 save_from_edit 校验外部变更，BR-WS-OBS-001）。
    let baseline = std::fs::read(&abs)
        .map(|b| workspace::save::content_hash(&b))
        .map_err(|e| {
            IpcError::new(
                "DOC_READ_FAILED",
                &format!("读 docx 失败: {e}"),
                true,
                false,
            )
        })?;
    // ① 集成：开始打开转换 → OpenConverting（经 SM-DOC-01）。
    let opening = transition(DocConvertState::Idle, DocConvertEvent::OpenDocx)
        .expect("SM-DOC-01: Idle+OpenDocx→OpenConverting");
    emit_doc01(app, &rel_path, opening);
    // 编辑链：docx → HTML；extract-media 工作目录按文档隔离（图片解出供编辑器；缓存策略实现期）。
    let work_dir = std::env::temp_dir()
        .join("binder_edit_media")
        .join(format!("{baseline:x}"));
    match doc::edit_chain::docx_to_html(&abs, &work_dir) {
        Ok(html) => {
            let st = transition(opening, DocConvertEvent::ConvertOk)
                .expect("OpenConverting+ConvertOk→Ready");
            emit_doc01(app, &rel_path, st);
            // 图文呈现（BR-DOC-DATA-002 嵌入图片必需子集）：extract-media 把图片解到 work_dir（按 baseline
            // 隔离），HTML 内为相对 `media/…` 路径。逐文件授权入 asset 协议 scope（最小暴露，同 prv.media_src，
            // X-FORBID-004 守界由后端把关）；回 media_base 供前端将相对 <img src> 改写为 asset URL（只读呈现）。
            authorize_media_files(app, &work_dir);
            Ok(serde_json::json!({
                "format": "html",
                "html": html,
                "baseline": baseline.to_string(),
                "media_base": work_dir.to_string_lossy(),
            }))
        }
        Err(e) => {
            // 转换失败：不建编辑会话，OpenError（原文件不受影响）。
            let st = transition(opening, DocConvertEvent::ConvertFail)
                .expect("OpenConverting+ConvertFail→OpenError");
            emit_doc01(app, &rel_path, st);
            Err(e)
        }
    }
}

/// 路由 `doc.save_from_edit`（A-DOC-I-P-01 §2 编辑链）：编辑 HTML → docx（--reference-doc 统一样式）→
/// 交 WS 原子落盘（SM-WS-01，baseline 校验外部变更、原文件不变保护）。转换失败回 convert_failed
/// （SM-DOC-01 SaveError，原 docx 不变）。
fn route_save_from_edit(
    request: &CommandRequest,
    state: &AppState,
    app: &tauri::AppHandle,
) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let html = request
        .params
        .get("html")
        .and_then(|v| v.as_str())
        .ok_or_else(|| IpcError::new("DOC_HTML_REQUIRED", "缺少 html 参数", false, false))?;
    let baseline = optional_u64_param(request, "baseline", "DOC_BASELINE_INVALID")?;
    let override_confirmed = bool_param(request, "override", false);
    let (_, abs) = resolve_active(state, &rel_path)?;
    let saving = doc::convert::transition(
        doc::convert::DocConvertState::Ready,
        doc::convert::DocConvertEvent::SaveDocx,
    )
    .expect("SM-DOC-01: Ready+SaveDocx→SaveConverting");
    emit_doc01(app, &rel_path, saving);
    let docx_bytes = match doc::edit_chain::html_to_docx(html) {
        Ok(b) => b,
        Err(e) => {
            let st =
                doc::convert::transition(saving, doc::convert::DocConvertEvent::SaveConvertFail)
                    .expect("SaveConverting+SaveConvertFail→SaveError");
            emit_doc01(app, &rel_path, st);
            return Err(e);
        }
    };
    match save_atomic_sm(
        app,
        &rel_path,
        &abs,
        &docx_bytes,
        baseline,
        override_confirmed,
    ) {
        Ok(saved) => {
            let st = doc::convert::transition(saving, doc::convert::DocConvertEvent::SaveConvertOk)
                .expect("SaveConverting+SaveConvertOk→Ready");
            emit_doc01(app, &rel_path, st);
            Ok(serde_json::json!({ "baseline": saved.baseline.to_string() }))
        }
        Err(e) => {
            let st =
                doc::convert::transition(saving, doc::convert::DocConvertEvent::SaveConvertFail)
                    .expect("SaveConverting+SaveConvertFail→SaveError");
            emit_doc01(app, &rel_path, st);
            Err(e)
        }
    }
}

/// 路由 `ws.list_timeline`：回文档时间轴节点（version_seq 升序，BR-WS-DATA-002）。
fn route_list_timeline(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let ws = active_ws(state)?;
    let doc = crate::types::DocRef {
        workspace_id: ws.workspace_id,
        doc_type: doc_type_of(&rel_path),
        rel_path,
    };
    let conn = lock_db(state)?;
    let list = workspace::timeline::list_timeline(&conn, &doc)
        .map_err(|e| IpcError::new("WS_DB_ERROR", &e.to_string(), true, false))?;
    ws_json(&list)
}

/// 路由 `ws.preview_node`：经节点 snapshot_ref 取回该版本内容（SM-WS-03 Previewing，快照不可取回则 guard 拒）。
fn route_preview_node(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let entry_id = str_param(request, "entryId", "WS_ENTRY_ID_REQUIRED")?;
    let conn = lock_db(state)?;
    let bytes = workspace::timeline::preview_node(&conn, &entry_id)?;
    Ok(serde_json::json!({ "content": String::from_utf8_lossy(&bytes) }))
}

/// 广播 SM-WS-03 时间轴还原机转移（entry_id 为实例，A-CORE-X-L-04 Task 2-W3）。
fn emit_ws03(app: &tauri::AppHandle, entry_id: &str, st: workspace::timeline::RestoreState) {
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
fn route_restore_node(
    request: &CommandRequest,
    state: &AppState,
    app: &tauri::AppHandle,
) -> Result<serde_json::Value, IpcError> {
    use workspace::timeline::{transition, RestoreEvent, RestoreState};
    let entry_id = str_param(request, "entryId", "WS_ENTRY_ID_REQUIRED")?;
    let ws = active_ws(state)?;
    let conn = lock_db(state)?;
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
fn route_discard_diff(
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
    discard_diff_sm(app, &conn, &diff_id, &terminal)?;
    Ok(serde_json::json!({ "diff_id": diff_id, "status": terminal }))
}

// —— SM-WS-02 落盘机集成（A-CORE-X-L-04 Task 2-D）——

/// 广播 SM-WS-02 转移（diff_id 为实例）。
fn emit_ws02(app: &tauri::AppHandle, diff_id: &str, st: workspace::diff_apply::DiffApplyState) {
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
fn apply_diff_sm(
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
fn discard_diff_sm(
    app: &tauri::AppHandle,
    conn: &Connection,
    diff_id: &str,
    terminal: &str,
) -> Result<(), IpcError> {
    use workspace::diff_apply::{transition, DiffApplyEvent, DiffApplyState};
    let ev = if terminal == "rejected" {
        DiffApplyEvent::Rejected
    } else {
        DiffApplyEvent::Expired
    };
    let discarded = transition(DiffApplyState::AwaitingDecision, ev)
        .expect("SM-WS-02: AwaitingDecision+Rejected/Expired→Discarded");
    workspace::diff_apply::discard_diff(conn, diff_id, terminal)
        .map_err(|e| IpcError::new("WS_DB_ERROR", &e.to_string(), true, false))?;
    emit_ws02(app, diff_id, discarded);
    Ok(())
}

// —— SM-WS-01 保存机集成（A-CORE-X-L-04 Task 2-C）——
// 脏态（Idle/Dirty）留前端高频本地标记（人类决策 2026-06-27）；保存**动作**经状态机驱动 +
// 广播：Dirty→Save→Saving（落盘开始）→SaveOk→Saved / SaveFail→SaveError。

/// 广播 SM-WS-01 转移（rel_path 为实例，区分多文档会话）。保存机为循环机，无终态（terminal=false）。
fn emit_ws01(app: &tauri::AppHandle, rel_path: &str, st: workspace::save::SaveState) {
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
fn save_with_baseline_guard(
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
fn save_atomic_sm(
    app: &tauri::AppHandle,
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

fn now_millis() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// 不透明 id（diff_id / payload_ref）：前缀 + UUIDv4（G-4 定档，A-CORE-X-R-03 §7，与 lifecycle 同策略）。
fn gen_id(prefix: &str) -> String {
    format!("{prefix}_{}", uuid::Uuid::new_v4())
}

fn doc_type_of(rel_path: &str) -> String {
    std::path::Path::new(rel_path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_else(|| "bin".to_string())
}

fn is_docx_rel_path(rel_path: &str) -> bool {
    matches!(doc_type_of(rel_path).as_str(), "docx" | "doc")
}

fn can_accept_background_diff(rel_path: &str, is_new_doc: bool) -> bool {
    !is_docx_rel_path(rel_path) || is_new_doc
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

fn live_agent_edit_base<'a>(
    target_rel: &str,
    current_rel: Option<&str>,
    base_content: Option<&'a str>,
) -> Option<&'a str> {
    if current_rel == Some(target_rel) {
        base_content
    } else {
        None
    }
}

fn style_ops_param(params: &serde_json::Value) -> Option<&serde_json::Value> {
    params
        .get("style_ops")
        .or_else(|| params.get("styleOps"))
        .filter(|v| match v {
            serde_json::Value::Array(items) => !items.is_empty(),
            serde_json::Value::Null => false,
            _ => true,
        })
}

fn parse_block_index_param(value: Option<&serde_json::Value>) -> Option<usize> {
    match value {
        Some(serde_json::Value::Number(n)) => n.as_u64().map(|v| v as usize),
        Some(serde_json::Value::String(s)) if s.trim().chars().all(|c| c.is_ascii_digit()) => {
            s.trim().parse::<usize>().ok()
        }
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AgentEditOperation {
    ReplaceBlock,
    ReplaceDocument,
    StyleOps,
}

#[derive(Debug, Clone, serde::Serialize)]
struct AgentDiffProjection {
    diff_id: String,
    rel_path: String,
    patch_kind: String,
    block_id: Option<String>,
    block_index: Option<usize>,
    before_text: String,
    after_text: String,
    location_label: String,
    draft_mode: String,
    style_ops: Option<serde_json::Value>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct AgentProposeEditResult {
    diff_id: String,
    rel_path: String,
    projection: AgentDiffProjection,
}

fn resolve_agent_edit_operation(
    params: &serde_json::Value,
    target_exists: bool,
) -> Result<AgentEditOperation, IpcError> {
    let operation = params
        .get("operation")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let has_style_ops = style_ops_param(params).is_some();
    let has_block_selector =
        params.get("block_id").is_some() || params.get("block_index").is_some();
    let has_new_text = params.get("new_text").and_then(|v| v.as_str()).is_some();
    let has_content = params.get("content").and_then(|v| v.as_str()).is_some();

    let op = match operation {
        Some("replace_block") => AgentEditOperation::ReplaceBlock,
        Some("replace_document") => AgentEditOperation::ReplaceDocument,
        Some("style_ops") => AgentEditOperation::StyleOps,
        Some(other) => {
            return Err(IpcError::new(
                "AG_TOOL_ARG",
                &format!("propose_edit operation 不支持: {other}"),
                false,
                false,
            ));
        }
        None if has_block_selector && has_new_text => AgentEditOperation::ReplaceBlock,
        None if has_style_ops && !has_content => AgentEditOperation::StyleOps,
        None if has_content && !target_exists => AgentEditOperation::ReplaceDocument,
        None if has_content => {
            return Err(IpcError::new(
                "AG_PRECISE_TARGET_REQUIRED",
                "已有文档的修改必须使用 operation=replace_block + block_id/block_index + new_text；只有用户明确要求整篇重写时才可使用 operation=replace_document + content。",
                true,
                false,
            ));
        }
        None => {
            return Err(IpcError::new(
                "AG_TOOL_ARG",
                "propose_edit 缺 operation/content/new_text/style_ops 参数",
                false,
                false,
            ));
        }
    };

    match op {
        AgentEditOperation::ReplaceBlock if !has_block_selector || !has_new_text => {
            Err(IpcError::new(
                "AG_TOOL_ARG",
                "operation=replace_block 必须提供 block_id 或 block_index，并提供 new_text",
                false,
                false,
            ))
        }
        AgentEditOperation::ReplaceDocument if !has_content => Err(IpcError::new(
            "AG_TOOL_ARG",
            "operation=replace_document 必须提供 content",
            false,
            false,
        )),
        AgentEditOperation::StyleOps if !has_style_ops => Err(IpcError::new(
            "AG_TOOL_ARG",
            "operation=style_ops 必须提供 style_ops",
            false,
            false,
        )),
        _ => Ok(op),
    }
}

fn looks_like_edit_intent(prompt: &str) -> bool {
    let text = prompt.trim().to_ascii_lowercase();
    if text.is_empty() {
        return false;
    }
    for advisory in ["修改建议", "怎么修改", "如何修改", "哪些修改", "修改意见"]
    {
        if prompt.contains(advisory) {
            return false;
        }
    }
    let zh_markers = [
        "修改", "改写", "重写", "替换", "删除", "插入", "加入", "添加", "补充", "调整", "优化",
        "润色", "加粗", "斜体", "颜色", "格式", "样式", "换成", "变成", "扩写", "缩写",
    ];
    if zh_markers.iter().any(|marker| prompt.contains(marker)) {
        return true;
    }
    let en_markers = [
        "edit", "revise", "rewrite", "replace", "delete", "insert", "add ", "format", "style",
        "bold", "italic", "color",
    ];
    en_markers.iter().any(|marker| text.contains(marker))
}

/// 路由 `de.generate_diff`：组装 ChangeProposal→锚校验生成 PendingDiff（BR-DE-DATA-001）→持久化
/// DiffPayload + pending_diff（重启恢复）；修改既有须锚 Located，否则 DE_ANCHOR_LOST（不产悬空 diff）。
fn route_generate_diff(
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
/// Pending→Accepted，但不触发 SM-WS-02 落盘；后续由用户显式 save 走 SM-WS-01 写盘。
fn route_accept_draft_diff(
    request: &CommandRequest,
    state: &AppState,
    app: &tauri::AppHandle,
) -> Result<serde_json::Value, IpcError> {
    let diff_id = str_param(request, "diffId", "WS_DIFF_ID_REQUIRED")?;
    let conn = lock_db(state)?;
    let cur = crate::de::state_from_status(workspace::diff_apply::current_status(&conn, &diff_id)?);
    let next = crate::de::transition(cur, crate::de::DiffEvent::UserAccept).ok_or_else(|| {
        IpcError::new(
            "DE_ILLEGAL_TRANSITION",
            &format!("diff 当前态 {cur:?} 不可接受（SM-DE-01 三终态不可逆）"),
            false,
            false,
        )
    })?;
    conn.execute(
        "UPDATE pending_diffs SET status = 'accepted', decided_at = ?2 WHERE diff_id = ?1",
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

/// 路由 `de.accept_diff`：后台/整篇 diff 经 SM-DE-01 守护（仅 Pending 可接受，三终态不可逆）
/// → ws.apply_diff 落盘 → 广播 SM-DE-01 转移；当前编辑器草稿路径使用 `de.accept_draft_diff`。
/// 既有 BDocx 不允许走后台接受，避免绕过当前编辑器 HTML 草稿与 DOC 保存链。
fn route_accept_diff(
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
        match doc::edit_chain::html_to_docx(html) {
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
fn route_reject_diff(
    request: &CommandRequest,
    state: &AppState,
    app: &tauri::AppHandle,
) -> Result<serde_json::Value, IpcError> {
    let diff_id = str_param(request, "diffId", "WS_DIFF_ID_REQUIRED")?;
    let conn = lock_db(state)?;
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
    discard_diff_sm(app, &conn, &diff_id, "rejected")?;
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

/// 路由 `de.list_diffs`：活动 workspace 未裁决 diff 列表（重启恢复，复用 WS 持久化查询）。
fn route_list_diffs(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let status = request.params.get("status").and_then(|v| v.as_str());
    let ws = active_ws(state)?;
    let conn = lock_db(state)?;
    let list = workspace::diff_apply::list_pending_diffs(&conn, &ws.workspace_id, status)
        .map_err(|e| IpcError::new("DE_DB_ERROR", &e.to_string(), true, false))?;
    ws_json(&list)
}

/**
 * @GOV
 * codes:    BR-SYS-DATA-003-GUARD-SYS-IPC-CALL-004
 * type:     GUARD
 * chain:    IPC-CALL
 * rules:    BR-SYS-DATA-003
 * boundary: in=legacyDispatchAgentTurn | out=IpcError(useAgStartTurnChannelNoStubReply)
 */
/// 旧 dispatch `ag.start_turn` 已停用：Chat 回合唯一真实入口是 async `ag_start_turn` + Channel。
/// 继续保留注册是为了让残留调用得到明确错误，而不是落入未注册 command 或伪造助手回复。
fn route_start_turn(_request: &CommandRequest) -> Result<serde_json::Value, IpcError> {
    Err(IpcError::new(
        "AG_START_TURN_DEPRECATED",
        "dispatch ag.start_turn 已停用，请使用 ag_start_turn Channel 命令",
        false,
        false,
    ))
}

fn sm_error() -> IpcError {
    IpcError::new("AG_SM_INVALID", "SM-AG-01 非法转移", false, false)
}

/// 广播 SM-AG-01 回合机转移（turn_id 为实例，A-CORE-X-L-04 Task 2-E）。
fn emit_ag01(app: &tauri::AppHandle, turn_id: &str, st: ag::turn::TurnState) {
    sm_events::emit_sm_state(
        app,
        sm_events::SmStatePayload::new(
            "SM-AG-01",
            turn_id,
            &format!("{st:?}"),
            ag::turn::is_terminal(st),
        ),
    );
}

/// 广播 SM-AG-02 工具循环机转移（turn_id 为实例，A-CORE-X-L-04 Task 2-AG2）。
fn emit_ag02(app: &tauri::AppHandle, turn_id: &str, st: ag::tool::ToolLoopState) {
    sm_events::emit_sm_state(
        app,
        sm_events::SmStatePayload::new(
            "SM-AG-02",
            turn_id,
            &format!("{st:?}"),
            ag::tool::is_terminal(st),
        ),
    );
}

/// 广播 SM-AG-03 局部修改/辅助续写机转移（assist_id 为实例，A-CORE-X-L-04 Task 2-AG3）。
fn emit_ag03(app: &tauri::AppHandle, assist_id: &str, st: ag::assist::AssistState) {
    sm_events::emit_sm_state(
        app,
        sm_events::SmStatePayload::new(
            "SM-AG-03",
            assist_id,
            &format!("{st:?}"),
            ag::assist::is_terminal(st),
        ),
    );
}

fn mark_assist_terminal(state: &AppState, assist_id: &str, st: ag::assist::AssistState) {
    if ag::assist::is_terminal(st) {
        if let Ok(mut terminals) = state.assist_terminals.lock() {
            terminals.insert(assist_id.to_string(), st);
        }
    }
}

fn assist_is_terminal(state: &AppState, assist_id: &str) -> bool {
    state
        .assist_terminals
        .lock()
        .map(|terminals| terminals.contains_key(assist_id))
        .unwrap_or(false)
}

/// 广播回合完成（Streaming→HandingOff→Completed；无修改建议交接时直接收尾）。
fn emit_ag01_complete(app: &tauri::AppHandle, turn_id: &str) {
    use ag::turn::{transition, TurnEvent, TurnState};
    let st = transition(TurnState::Streaming, TurnEvent::StreamEnd)
        .and_then(|s| transition(s, TurnEvent::HandoffOk))
        .unwrap_or(TurnState::Completed);
    emit_ag01(app, turn_id, st);
}

/// 路由 `ag.list_blocks`：列指定文档的块（供块级 @mention 选择非当前文档的块，PI-AG-09G）。
/// 限 active workspace（X-FORBID-004）；read_doc 含 docx；is_html 自判 text/html 分块。
fn route_list_blocks(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let ws = active_ws(state)?;
    let abs = workspace::resolve_in_workspace(std::path::Path::new(&ws.root_path), &rel_path)?;
    let snapshot = ag::blocks::read_editable_snapshot(
        std::path::Path::new(&ws.root_path),
        &rel_path,
        &abs,
        None,
        None,
    )?;
    Ok(serde_json::json!({
        "rel_path": snapshot.rel_path,
        "editable_kind": snapshot.editable_kind,
        "content_format": snapshot.content_format,
        "source": snapshot.source,
        "blocks": snapshot.blocks,
    }))
}

/// 供应商默认模型（dev；清单延实现期运营配置）。
fn default_model(provider: &str) -> String {
    match provider {
        "openai" => "gpt-4o-mini",
        "anthropic" => "claude-sonnet-4-6",
        _ => "deepseek-chat",
    }
    .to_string()
}

/// 块级引用（选区精确锚，PI-AG-09F）：用户引用某文档某块的选中内容。serde camelCase 对接前端。
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct BlockRef {
    rel_path: String,
    block_id: String,
    start: Option<usize>,
    end: Option<usize>,
    text: String,
}

fn block_ref_range_label(br: &BlockRef) -> String {
    match (br.start, br.end) {
        (Some(start), Some(end)) if end > start => format!("字符区间 [{start}..{end}]"),
        _ => "字符区间未提供（按该块选中内容理解）".to_string(),
    }
}

fn format_block_ref_context(br: &BlockRef, capped_text: &str) -> String {
    format!(
        "引用文档「{}」块 [{}] 的选中内容，{}：\n{}",
        br.rel_path,
        br.block_id,
        block_ref_range_label(br),
        capped_text
    )
}

fn reference_target_path(payload: &ReferencePayload) -> Option<&str> {
    match payload {
        ReferencePayload::Doc { rel_path, .. } | ReferencePayload::Selection { rel_path, .. } => {
            Some(rel_path.as_str())
        }
        _ => None,
    }
}

/**
 * @GOV
 * codes:    BR-AG-DATA-001-AG-AG-TURN-009
 * type:     DATA
 * chain:    AG-TURN
 * rules:    BR-AG-DATA-001, BR-REF-DATA-003
 * term_ref: TERM-REF-003
 * boundary: in=ReferencePayloadAndWorkspaceRoot | out=explicitReferenceContextBlock(readOnlySnapshot)
 */
fn format_reference_payload_context(
    payload: &ReferencePayload,
    ws_root: Option<&str>,
    cap: usize,
) -> String {
    match payload {
        ReferencePayload::Doc { rel_path, .. } => match ws_root {
            Some(root) => {
                match workspace::resolve_in_workspace(std::path::Path::new(root), rel_path) {
                    Ok(abs) => match crate::ag::blocks::read_editable_snapshot(
                        std::path::Path::new(root),
                        rel_path,
                        &abs,
                        None,
                        None,
                    ) {
                        Ok(snapshot) => {
                            let capped = crate::ag::context::cap_text(
                                &snapshot.content,
                                cap,
                                "[引用文档较长，已截取前部；需要完整内容可用 read_doc]",
                            );
                            format!("引用文档「{rel_path}」：\n{capped}")
                        }
                        Err(e) => format!(
                            "引用文档「{rel_path}」读取失败（{}）：{}",
                            e.code, e.message
                        ),
                    },
                    Err(e) => format!(
                        "引用文档「{rel_path}」路径无效或超出 workspace（{}）：{}",
                        e.code, e.message
                    ),
                }
            }
            None => format!("引用文档「{rel_path}」未读取：当前没有 active workspace"),
        },
        ReferencePayload::Selection {
            rel_path,
            block_id,
            start,
            end,
            text,
            ..
        } => {
            let br = BlockRef {
                rel_path: rel_path.clone(),
                block_id: block_id.clone(),
                start: *start,
                end: *end,
                text: text.clone(),
            };
            let capped = crate::ag::context::cap_text(text, cap, "[引用选区较长，已截取前部]");
            format_block_ref_context(&br, &capped)
        }
        ReferencePayload::Session {
            title, messages, ..
        } => {
            let body = messages
                .iter()
                .filter(|m| !m.content.trim().is_empty())
                .map(|m| format!("{}: {}", m.role, m.content))
                .collect::<Vec<_>>()
                .join("\n");
            let capped = crate::ag::context::cap_text(&body, cap, "[引用会话较长，已截取前部]");
            format!("引用会话「{title}」：\n{capped}")
        }
        ReferencePayload::Memory { title, content, .. } => {
            let capped = crate::ag::context::cap_text(content, cap, "[引用记忆较长，已截取前部]");
            format!(
                "引用记忆「{}」：\n{}",
                title.as_deref().unwrap_or("未命名"),
                capped
            )
        }
        ReferencePayload::Knowledge {
            title,
            content,
            source_ref,
            ..
        } => {
            let capped = crate::ag::context::cap_text(content, cap, "[引用知识较长，已截取前部]");
            format!(
                "引用知识「{}」{}：\n{}",
                title.as_deref().unwrap_or("未命名"),
                source_ref
                    .as_deref()
                    .map(|s| format!("（来源：{s}）"))
                    .unwrap_or_default(),
                capped
            )
        }
        ReferencePayload::Template { name, content, .. } => {
            let capped = crate::ag::context::cap_text(content, cap, "[引用模板较长，已截取前部]");
            format!("引用模板「{name}」：\n{capped}")
        }
        ReferencePayload::Url { url, title, .. } => {
            format!("引用外部链接「{}」：{url}", title.as_deref().unwrap_or(url))
        }
        ReferencePayload::LongText {
            text, char_count, ..
        } => {
            let capped = crate::ag::context::cap_text(text, cap, "[引用长文本较长，已截取前部]");
            format!("引用长文本（{char_count} 字）：\n{capped}")
        }
    }
}

#[cfg(test)]
mod ag_reference_context_tests {
    use super::*;

    fn block_ref(start: Option<usize>, end: Option<usize>) -> BlockRef {
        BlockRef {
            rel_path: "chapter.md".to_string(),
            block_id: "block_1".to_string(),
            start,
            end,
            text: "引用文本".to_string(),
        }
    }

    #[test]
    fn block_ref_context_preserves_selection_range() {
        let out = format_block_ref_context(&block_ref(Some(2), Some(8)), "引用文本");
        assert!(out.contains("chapter.md"));
        assert!(out.contains("block_1"));
        assert!(out.contains("字符区间 [2..8]"));
        assert!(out.ends_with("引用文本"));
    }

    #[test]
    fn block_ref_context_degrades_invalid_range_without_dropping_text() {
        for br in [
            block_ref(None, None),
            block_ref(Some(3), None),
            block_ref(Some(8), Some(2)),
        ] {
            let out = format_block_ref_context(&br, "引用文本");
            assert!(out.contains("字符区间未提供"));
            assert!(out.ends_with("引用文本"));
        }
    }

    #[test]
    fn reference_payload_context_formats_url_and_long_text() {
        let url = ReferencePayload::Url {
            url: "https://example.com".to_string(),
            title: Some("Example".to_string()),
            label: None,
        };
        assert_eq!(
            format_reference_payload_context(&url, None, 1000),
            "引用外部链接「Example」：https://example.com"
        );

        let long = ReferencePayload::LongText {
            text: "abcdef".repeat(10),
            char_count: 60,
            label: None,
        };
        let out = format_reference_payload_context(&long, None, 20);
        assert!(out.contains("引用长文本（60 字）"));
        assert!(out.contains("[引用长文本较长"));
    }

    #[test]
    fn reference_payload_context_formats_session_snapshot() {
        let payload = ReferencePayload::Session {
            session_id: "s1".to_string(),
            title: "需求讨论".to_string(),
            messages: vec![
                ReferenceSessionMessage {
                    role: "user".to_string(),
                    content: "先做引用".to_string(),
                },
                ReferenceSessionMessage {
                    role: "assistant".to_string(),
                    content: "收到".to_string(),
                },
            ],
            label: None,
        };
        let out = format_reference_payload_context(&payload, None, 1000);
        assert!(out.contains("引用会话「需求讨论」"));
        assert!(out.contains("user: 先做引用"));
        assert!(out.contains("assistant: 收到"));
    }

    #[test]
    fn reference_payload_context_reads_doc_from_workspace() {
        let root = std::env::temp_dir().join(format!(
            "binder-ref-doc-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(root.join("notes")).unwrap();
        std::fs::write(root.join("notes/a.md"), "文档引用内容").unwrap();
        let payload = ReferencePayload::Doc {
            rel_path: "notes/a.md".to_string(),
            label: None,
        };

        let out = format_reference_payload_context(&payload, root.to_str(), 1000);

        assert!(out.contains("引用文档「notes/a.md」"));
        assert!(out.contains("文档引用内容"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn reference_payload_context_formats_selection_snapshot() {
        let payload = ReferencePayload::Selection {
            rel_path: "notes/a.md".to_string(),
            block_id: "block-1".to_string(),
            start: Some(2),
            end: Some(6),
            text: "选区引用内容".to_string(),
            label: None,
        };

        let out = format_reference_payload_context(&payload, None, 1000);

        assert!(out.contains("引用文档「notes/a.md」块 [block-1]"));
        assert!(out.contains("字符区间 [2..6]"));
        assert!(out.ends_with("选区引用内容"));
    }

    #[test]
    fn reference_payload_context_formats_asset_snapshots() {
        let memory = ReferencePayload::Memory {
            entry_id: Some("m1".to_string()),
            title: Some("偏好".to_string()),
            content: "用户偏好简洁回答".to_string(),
            label: None,
        };
        let knowledge = ReferencePayload::Knowledge {
            entry_id: Some("k1".to_string()),
            title: Some("术语表".to_string()),
            content: "ReferencePayload 是输入侧引用载荷".to_string(),
            source_ref: Some("kb/ref.md".to_string()),
            label: None,
        };
        let template = ReferencePayload::Template {
            template_id: Some("tpl1".to_string()),
            name: "周报模板".to_string(),
            content: "本周完成：\n下周计划：".to_string(),
            label: None,
        };

        let memory_out = format_reference_payload_context(&memory, None, 1000);
        let knowledge_out = format_reference_payload_context(&knowledge, None, 1000);
        let template_out = format_reference_payload_context(&template, None, 1000);

        assert!(memory_out.contains("引用记忆「偏好」"));
        assert!(memory_out.contains("用户偏好简洁回答"));
        assert!(knowledge_out.contains("引用知识「术语表」（来源：kb/ref.md）"));
        assert!(knowledge_out.contains("ReferencePayload 是输入侧引用载荷"));
        assert!(template_out.contains("引用模板「周报模板」"));
        assert!(template_out.contains("本周完成"));
    }

    #[test]
    fn reference_payload_declares_doc_targets() {
        let doc = ReferencePayload::Doc {
            rel_path: "a.md".to_string(),
            label: None,
        };
        let selection = ReferencePayload::Selection {
            rel_path: "b.md".to_string(),
            block_id: "block".to_string(),
            start: Some(1),
            end: Some(3),
            text: "bc".to_string(),
            label: None,
        };
        let memory = ReferencePayload::Memory {
            entry_id: None,
            title: None,
            content: "偏好".to_string(),
            label: None,
        };
        assert_eq!(reference_target_path(&doc), Some("a.md"));
        assert_eq!(reference_target_path(&selection), Some("b.md"));
        assert_eq!(reference_target_path(&memory), None);
    }
}

#[cfg(test)]
mod ag_edit_base_tests {
    use super::*;
    use serde_json::json;

    // covers: BR-AG-DATA-003
    // 当前编辑器实时快照只允许作为当前目标文档的 diff 基准；非当前目标必须另读自身内容，避免锚错位。
    #[test]
    fn live_editor_base_only_applies_to_current_target() {
        assert_eq!(
            live_agent_edit_base("notes/a.docx", Some("notes/a.docx"), Some("<p>live</p>")),
            Some("<p>live</p>")
        );
        assert_eq!(
            live_agent_edit_base(
                "notes/other.docx",
                Some("notes/a.docx"),
                Some("<p>live</p>")
            ),
            None
        );
        assert_eq!(
            live_agent_edit_base("notes/a.docx", None, Some("<p>live</p>")),
            None
        );
    }

    #[test]
    fn existing_document_rejects_legacy_bare_content() {
        let err = resolve_agent_edit_operation(
            &json!({ "path": "notes/a.docx", "content": "<p>全文</p>" }),
            true,
        )
        .unwrap_err();

        assert_eq!(err.code, "AG_PRECISE_TARGET_REQUIRED");
        assert!(err.message.contains("replace_block"));
        assert!(err.message.contains("replace_document"));
    }

    #[test]
    fn new_document_allows_legacy_content_creation() {
        let op = resolve_agent_edit_operation(
            &json!({ "path": "notes/new.md", "content": "新文档" }),
            false,
        )
        .unwrap();

        assert_eq!(op, AgentEditOperation::ReplaceDocument);
    }

    #[test]
    fn explicit_replace_document_requires_content() {
        let op = resolve_agent_edit_operation(
            &json!({ "path": "notes/a.docx", "operation": "replace_document", "content": "<p>全文</p>" }),
            true,
        )
        .unwrap();
        assert_eq!(op, AgentEditOperation::ReplaceDocument);

        let err = resolve_agent_edit_operation(
            &json!({ "path": "notes/a.docx", "operation": "replace_document" }),
            true,
        )
        .unwrap_err();
        assert_eq!(err.code, "AG_TOOL_ARG");
    }

    #[test]
    fn explicit_replace_block_requires_selector_and_new_text() {
        let op = resolve_agent_edit_operation(
            &json!({ "path": "notes/a.docx", "operation": "replace_block", "block_id": "b1", "new_text": "新块" }),
            true,
        )
        .unwrap();
        assert_eq!(op, AgentEditOperation::ReplaceBlock);

        let err = resolve_agent_edit_operation(
            &json!({ "path": "notes/a.docx", "operation": "replace_block", "new_text": "新块" }),
            true,
        )
        .unwrap_err();
        assert_eq!(err.code, "AG_TOOL_ARG");
    }

    // covers: BR-DE-STATE-001
    // 既有 BDocx 的接受必须确认当前编辑器草稿，不保留后台 accept_diff 平行落盘路径；新建 docx 仍可创建。
    #[test]
    fn background_accept_disallows_existing_bdocx_but_allows_new_docx_creation() {
        assert!(!can_accept_background_diff("notes/a.docx", false));
        assert!(!can_accept_background_diff("notes/a.draft.docx", false));
        assert!(can_accept_background_diff("notes/new.docx", true));
        assert!(can_accept_background_diff("notes/a.md", false));
    }
}

/// PI-BYOK-DEV-01 · BYOK 客户端入口：用户 provider 密钥一次性上送 dev 网关代持（零残留，客户端不持久化，
/// X-FORBID-001）。key 经 ent::byok 上送后即清零；不回显明文。
#[tauri::command]
async fn byok_upload(provider: String, key: String) -> Result<(), IpcError> {
    if key.trim().is_empty() {
        return Err(IpcError::new("ENT_BYOK_EMPTY", "密钥为空", false, false));
    }
    ent::byok::upload_to_gateway(&provider, key).await
}

/// PI-AG-08C/08E/08F + PI-AG-SEM-01 · propose_edit 写工具执行（复用 route_generate_diff 逻辑）。
///
/// AI 修改建议经 DE 生成 pending diff，**不直写文件**（X-FORBID-006）；限 active workspace（X-FORBID-004）；
/// target 须属回合声明目标集或为新建路径（BR-AG-DATA-003 2026-07-04 版，AG guard + DE 校验双侧把守）。
/// 三模式：块级（replace_block）/ 整篇（replace_document）/ 样式（style_ops）。
/// 返回后端权威 DiffProjection；前端不得再从 tool_request 预测 before/after。
fn ag_propose_edit(
    state: &AppState,
    params: &serde_json::Value,
    base_content: Option<&str>,
    is_html: bool,
    current_rel: Option<&str>,
    declared_targets: &[String],
) -> Result<AgentProposeEditResult, IpcError> {
    use crate::de::generate::{
        build_payload, ensure_target_declared, generate_diff, ChangeProposal,
    };
    use crate::types::LocateResult;

    let rel_path = params
        .get("path")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| IpcError::new("AG_TOOL_ARG", "propose_edit 缺 path 参数", false, false))?
        .to_string();

    let ws = active_ws(state)?;
    // 路径限 workspace 内（X-FORBID-004）。
    let abs = workspace::resolve_in_workspace(std::path::Path::new(&ws.root_path), &rel_path)?;

    // 目标集双守（BR-AG-DATA-003，2026-07-04 版）：当前文档/显式声明集/新建放行，未声明既有文档拒绝
    // （错误回注模型，模型可提示用户显式 @引用后重试）。AG guard 与 DE 协同校验两侧均须通过。
    let target_exists = abs.exists();
    if let crate::ag::context::TargetDecision::RejectUndeclared =
        crate::ag::context::guard_target_set(
            current_rel,
            declared_targets,
            &rel_path,
            target_exists,
        )
    {
        return Err(IpcError::new(
            "AG_TARGET_UNDECLARED",
            &format!("目标文档 {rel_path} 非当前文档且未被用户显式引用，已拒绝；请提示用户 @引用该文档后重试，或改为修改当前文档"),
            false,
            false,
        ));
    }
    ensure_target_declared(&rel_path, current_rel, declared_targets, target_exists)?;
    let edit_operation = resolve_agent_edit_operation(params, target_exists)?;

    let target_is_current_editor = current_rel == Some(rel_path.as_str());
    let live_base_content = live_agent_edit_base(&rel_path, current_rel, base_content);
    let live_is_html = target_is_current_editor && is_html;
    if target_is_current_editor && target_exists && live_base_content.is_none() {
        return Err(IpcError::new(
            "AG_EDITOR_SNAPSHOT_REQUIRED",
            "当前打开文档缺少编辑器实时快照，已拒绝生成可能错位的修改建议；请重新打开文档或重试。",
            true,
            false,
        ));
    }
    if target_exists && is_docx_rel_path(&rel_path) && !target_is_current_editor {
        return Err(IpcError::new(
            "AG_BDOCX_EDIT_REQUIRES_OPEN_EDITOR",
            "b-docx 修改必须先打开为当前编辑器草稿后生成，避免后台裁决把 HTML 当 docx 字节写入。",
            false,
            false,
        ));
    }

    let style_ops_json = style_ops_param(params).cloned();
    let style_ops = style_ops_json.as_ref().map(|v| v.to_string());
    let workspace_root = std::path::Path::new(&ws.root_path);
    let snapshot = if target_exists {
        Some(crate::ag::blocks::read_editable_snapshot(
            workspace_root,
            &rel_path,
            &abs,
            live_base_content,
            if live_is_html { Some("html") } else { None },
        )?)
    } else {
        None
    };
    let base_content_for_diff = snapshot.as_ref().map(|s| s.content.as_str()).unwrap_or("");
    let base_is_html = snapshot
        .as_ref()
        .map(|s| s.content_format == "html")
        .unwrap_or(false);

    // replace_block：基准用当前编辑器快照或 BDocx 转换 HTML 切块，替目标块并重建文档；
    // replace_document：仅显式整篇重写可用；style_ops：文本基准保持不变，样式语义由载荷进入 DE 裁决。
    let projection_block_id: Option<String>;
    let projection_block_index: Option<usize>;
    let projection_before: String;
    let projection_after: String;
    let projection_location: String;

    let after_content = match edit_operation {
        AgentEditOperation::ReplaceBlock => {
            let new_text = params
                .get("new_text")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    IpcError::new(
                        "AG_TOOL_ARG",
                        "propose_edit 块模式缺 new_text 参数",
                        false,
                        false,
                    )
                })?;
            let snapshot = snapshot.as_ref().ok_or_else(|| {
                IpcError::new(
                    "AG_EDIT_BASE_REQUIRED",
                    "块级修改必须基于既有文档快照",
                    false,
                    false,
                )
            })?;
            let raw_block_id = params
                .get("block_id")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty());
            let numeric_block_id_as_index = raw_block_id.and_then(|s| s.parse::<usize>().ok());
            let selected_block = if let Some(block_id) =
                raw_block_id.filter(|_| numeric_block_id_as_index.is_none())
            {
                snapshot
                    .blocks
                    .iter()
                    .find(|b| b.block_id.as_deref() == Some(block_id))
                    .ok_or_else(|| {
                        IpcError::new(
                            "AG_ANCHOR_LOST",
                            &format!("锚失效：未找到 block_id={block_id} 的块"),
                            true,
                            false,
                        )
                    })?
            } else {
                let bi = numeric_block_id_as_index
                    .or_else(|| parse_block_index_param(params.get("block_index")))
                    .ok_or_else(|| {
                        IpcError::new(
                            "AG_TOOL_ARG",
                            "propose_edit 缺 block_id/block_index",
                            false,
                            false,
                        )
                    })?;
                snapshot.blocks.get(bi).ok_or_else(|| {
                    IpcError::new(
                        "AG_TOOL_ARG",
                        &format!(
                            "block_index {bi} 越界（当前共 {} 块）",
                            snapshot.blocks.len()
                        ),
                        false,
                        false,
                    )
                })?
            };
            projection_block_id = selected_block.block_id.clone();
            projection_block_index = Some(selected_block.block_index);
            projection_before = selected_block.text.clone();
            projection_after = new_text.to_string();
            projection_location = format!("Block {}", selected_block.block_index);
            if let Some(block_id) = selected_block.block_id.as_deref() {
                // 真实 blockId 锚（HTML data-block-id，PI-AG-08G）：按 id 定位；锚失效报错不误改。
                crate::ag::blocks::replace_block_by_id(base_content_for_diff, block_id, new_text)
                    .map_err(|m| IpcError::new("AG_ANCHOR_LOST", &m, true, false))?
            } else {
                // 位置块号（text 或回退）。
                crate::ag::blocks::replace_block(
                    base_content_for_diff,
                    base_is_html,
                    selected_block.block_index,
                    new_text,
                )
                .map_err(|m| IpcError::new("AG_TOOL_ARG", &m, false, false))?
            }
        }
        AgentEditOperation::ReplaceDocument => {
            let content = params
                .get("content")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    IpcError::new(
                        "AG_TOOL_ARG",
                        "operation=replace_document 必须提供 content",
                        false,
                        false,
                    )
                })?;
            projection_block_id = None;
            projection_block_index = None;
            projection_before = base_content_for_diff.to_string();
            projection_after = content.to_string();
            projection_location = "整篇".to_string();
            content.to_string()
        }
        AgentEditOperation::StyleOps => {
            let snapshot = snapshot.as_ref().ok_or_else(|| {
                IpcError::new(
                    "AG_EDIT_BASE_REQUIRED",
                    "样式修改必须基于既有文档快照",
                    false,
                    false,
                )
            })?;
            let first_block_id =
                style_ops_json
                    .as_ref()
                    .and_then(|v| v.as_array())
                    .and_then(|items| {
                        items
                            .iter()
                            .find_map(|item| item.get("blockId").and_then(|v| v.as_str()))
                    });
            let selected_block = first_block_id.and_then(|id| {
                snapshot
                    .blocks
                    .iter()
                    .find(|b| b.block_id.as_deref() == Some(id))
            });
            if let Some(block) = selected_block {
                projection_block_id = block.block_id.clone();
                projection_block_index = Some(block.block_index);
                projection_before = block.text.clone();
            } else {
                projection_block_id = None;
                projection_block_index = None;
                projection_before = base_content_for_diff.to_string();
            }
            projection_after = "（样式修改）".to_string();
            projection_location = "样式修改".to_string();
            base_content_for_diff.to_string()
        }
    };

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
    let projection = AgentDiffProjection {
        diff_id: diff_id.clone(),
        rel_path: rel_path.clone(),
        patch_kind: match edit_operation {
            AgentEditOperation::ReplaceBlock => "replace_block",
            AgentEditOperation::ReplaceDocument => "replace_document",
            AgentEditOperation::StyleOps => "style_ops",
        }
        .to_string(),
        block_id: projection_block_id,
        block_index: projection_block_index,
        before_text: projection_before,
        after_text: projection_after,
        location_label: projection_location,
        draft_mode: if target_is_current_editor {
            "current_editor"
        } else {
            "background"
        }
        .to_string(),
        style_ops: style_ops_json,
    };
    Ok(AgentProposeEditResult {
        diff_id,
        rel_path,
        projection,
    })
}

/// PI-AG-WIRE-01 · `ag_start_turn`（独立 async tauri command，不走 dispatch 信封）。
///
/// 驱动 SM-AG-01 门禁前置（X-SEQ-001：唯一经 AwaitingGate+gatePassed 达 Streaming；dev 门禁桩=Passed，
/// 真实裁决经 GW 服务端 X-FORBID-002 归正式版）→ 组装 system(PI-AG-07)+user 消息 → 经 ENT 网关通道
/// `stream_invoke` 调本地 dev GW（X-FORBID-001：客户端不持密钥/不直连 provider）→ 归一化 SSE 逐事件
/// 经 `Channel<AgTurnEvent>` 流式回前端（替换 dispatch 内 ag.start_turn 桩）。
#[allow(clippy::too_many_arguments)]
#[tauri::command]
async fn ag_start_turn(
    turn_id: String,
    prompt: String,
    provider: Option<String>,
    model: Option<String>,
    active_doc_path: Option<String>,
    active_doc_content: Option<String>,
    active_doc_format: Option<String>,
    history: Vec<ag::context::HistTurn>,
    mentions: Vec<String>,
    block_refs: Vec<BlockRef>,
    references: Vec<ReferencePayload>,
    on_event: tauri::ipc::Channel<AgTurnEvent>,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), IpcError> {
    use ag::turn::{ensure_gate_passed, transition, GateState, TurnEvent, TurnState};

    if prompt.trim().is_empty() && references.is_empty() {
        return Err(IpcError::new(
            "AG_PROMPT_REQUIRED",
            "缺少 prompt",
            false,
            false,
        ));
    }

    // 结构性门禁前置：唯一通往 Streaming（模型调用）的路径是 AwaitingGate + gatePassed（X-SEQ-001）。
    // 每步转移广播 SM-AG-01（A-CORE-X-L-04 Task 2-E）：sending 等回合视图据此投影（X-FORBID-007）。
    let s = transition(TurnState::Idle, TurnEvent::Send).ok_or_else(sm_error)?; // → Assembling
    emit_ag01(&app, &turn_id, s);
    let s = transition(s, TurnEvent::Assembled).ok_or_else(sm_error)?; // → AwaitingGate
    emit_ag01(&app, &turn_id, s);
    ensure_gate_passed(GateState::Passed)?; // dev 桩=Passed；无订阅时此处 Denied 即拒
    let s = transition(s, TurnEvent::GatePassed).ok_or_else(sm_error)?; // → Streaming
    emit_ag01(&app, &turn_id, s);
    debug_assert_eq!(s, TurnState::Streaming);

    let provider = provider.unwrap_or_else(|| "deepseek".to_string());
    let model = model.unwrap_or_else(|| default_model(&provider));

    // 回合声明目标集（BR-AG-DATA-003，2026-07-04 版）：用户显式指向的文档 = @mention ∪ 块引用所在文档。
    // propose_edit 的 target 须属 当前文档 ∪ 本集合，或为新建路径；未声明既有文档拒绝（guard_target_set）。
    let mut declared_targets: Vec<String> = mentions.clone();
    for b in &block_refs {
        if !declared_targets.contains(&b.rel_path) {
            declared_targets.push(b.rel_path.clone());
        }
    }
    for r in &references {
        if let Some(rel_path) = reference_target_path(r) {
            if !declared_targets.iter().any(|p| p == rel_path) {
                declared_targets.push(rel_path.to_string());
            }
        }
    }

    // workspace 解析 + AST/KB 补强检索（P2，限当前 workspace X-FORBID-004，PI-AG-09D）。单锁，锁在 await 前释放。
    const AUG_BUDGET: u32 = 2500; // 补强层字符预算（dev；G5 标定）
    let mut ws_root: Option<String> = None;
    let mut aug_items: Vec<crate::ag::context::ContextItem> = Vec::new();
    if let Ok(conn) = state.db.lock() {
        if let Ok(Some(ws)) = workspace::lifecycle::active_workspace(&conn) {
            ws_root = Some(ws.root_path.clone());
            // AST 记忆供给。
            if let Ok(mems) = crate::ast::supply::supply(&conn, &ws.workspace_id) {
                for m in mems {
                    let cost = m.content.chars().count() as u32;
                    aug_items.push(crate::ag::context::ContextItem {
                        source: crate::ag::context::ContextSource::Memory,
                        content: format!("记忆：{}", m.content),
                        cost,
                    });
                }
            }
            // KB 知识检索（query=当前 prompt；本地匹配、无上行）。
            if let Ok(ks) = crate::kb::retrieve::retrieve(&conn, &ws.workspace_id, &prompt) {
                for k in ks {
                    let cost = k.content.chars().count() as u32;
                    aug_items.push(crate::ag::context::ContextItem {
                        source: crate::ag::context::ContextSource::Knowledge,
                        content: format!("知识（{}）：{}", k.source_ref, k.content),
                        cost,
                    });
                }
            }
        }
    }
    // P2 补强预算保留（assemble_context）+ 权重声明（BR-AG-DATA-005：仅补强，不覆盖当前文档/当前轮）。空集不注入。
    let aug_block: Option<String> = {
        let aug = crate::ag::context::assemble_context(aug_items, AUG_BUDGET);
        if aug.kept.is_empty() {
            None
        } else {
            let body = aug
                .kept
                .iter()
                .map(|i| i.content.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            Some(format!(
                "[补强信息] 以下来自记忆库/知识库，仅作背景补强，不覆盖当前文档与当前轮请求：\n{body}"
            ))
        }
    };

    // REF 显式输入引用（P1，优先保留不静默丢弃，BR-AG-DATA-001 / BR-REF-DATA-003）；限 workspace。
    const MENTION_CAP: usize = 4000;
    let mention_block: Option<String> = {
        let mut parts = Vec::new();
        // 整篇引用：读文档（需 ws_root，X-FORBID-004）。
        if let Some(root) = ws_root.as_deref() {
            for rel in &mentions {
                match workspace::resolve_in_workspace(std::path::Path::new(root), rel) {
                    Ok(abs) => match crate::ag::blocks::read_editable_snapshot(
                        std::path::Path::new(root),
                        rel,
                        &abs,
                        None,
                        None,
                    ) {
                        Ok(snapshot) => {
                            let capped = crate::ag::context::cap_text(
                                &snapshot.content,
                                MENTION_CAP,
                                "[引用文档较长，已截取前部；需要完整内容可用 read_doc]",
                            );
                            parts.push(format!("引用文档「{rel}」：\n{capped}"));
                        }
                        Err(e) => parts.push(format!(
                            "引用文档「{rel}」读取失败（{}）：{}",
                            e.code, e.message
                        )),
                    },
                    Err(e) => parts.push(format!(
                        "引用文档「{rel}」路径无效或超出 workspace（{}）：{}",
                        e.code, e.message
                    )),
                }
            }
        } else {
            for rel in &mentions {
                parts.push(format!(
                    "引用文档「{rel}」未读取：当前没有 active workspace"
                ));
            }
        }
        // 块级引用（选区精确锚，PI-AG-09F）：text 与 start/end 已由前端 SelectionRef 带入。
        for br in &block_refs {
            if br.text.trim().is_empty() {
                continue;
            }
            let capped =
                crate::ag::context::cap_text(&br.text, MENTION_CAP, "[引用选区较长，已截取前部]");
            parts.push(format_block_ref_context(br, &capped));
        }
        // 新主通道：ReferencePayload 覆盖 doc / selection / session / memory / knowledge / template / url / long_text。
        for reference in &references {
            parts.push(format_reference_payload_context(
                reference,
                ws_root.as_deref(),
                MENTION_CAP,
            ));
        }
        if parts.is_empty() {
            None
        } else {
            Some(format!(
                "[用户显式引用] 以下是用户本轮明确引用的内容（高优先，请据此回答相关问题；外部链接仅代表链接本身，不代表已抓取网页内容）：\n{}",
                parts.join("\n\n")
            ))
        }
    };

    // 文档形态（PI-AG-08F）：editFormat=html 或内容以 < 开头 → 按 docx/html 块；否则 text 段落块。
    let doc_is_html = active_doc_content
        .as_deref()
        .map(|c| crate::ag::blocks::is_html(c, active_doc_format.as_deref()))
        .unwrap_or(false);
    let requires_edit_tool =
        active_doc_path.as_deref().is_some() && looks_like_edit_intent(&prompt);
    let mut proposed_diff = false;

    // P0 当前文档块视图（PI-AG-08E/08F/08G）；超长封顶（层保留、内容可缩，BR-AG-DATA-005，PI-AG-09B）。
    const DOC_CTX_CAP: usize = 9000; // 字符（dev；G5 标定）
    let doc_ctx: Option<String> = active_doc_content
        .as_deref()
        .filter(|c| !c.trim().is_empty())
        .map(|content| {
            let path = active_doc_path.as_deref().unwrap_or("(未命名)");
            let blocks = crate::ag::blocks::project_blocks(content, doc_is_html);
            let mut block_view = String::new();
            for b in blocks.iter() {
                match &b.block_id {
                    Some(id) => block_view.push_str(&format!("Block {} [id={id}]: {}\n", b.block_index, b.text)),
                    None => block_view.push_str(&format!("Block {}: {}\n", b.block_index, b.text)),
                }
            }
            let ctx = format!(
                "[当前编辑器打开的文档] 路径：{path}\n以下按块列出该文档当前内容（本轮主事实，回答关于「当前文档/这篇/这段」的问题时直接据此作答）：\n\n{block_view}\n修改某一块必须用 propose_edit operation=replace_block + new_text：有 [id=…] 的块**优先传 block_id**（只能传方括号内完整 id，不要把数字块号填入 block_id），否则传 block_index（Block N 中的 0 起块号）；只有用户明确要求整篇重写时，才用 operation=replace_document + content。"
            );
            crate::ag::context::cap_text(&ctx, DOC_CTX_CAP, "[文档较长，已截取前部；其余内容可用 read_doc 分块读取]")
        });

    // P1 历史层：总预算扣除 P0（系统/文档/当前输入）后的余额内保留最近若干轮（drop 最旧，PI-AG-09B）。
    const CTX_BUDGET_CHARS: usize = 16000; // 总预算（~4000 tokens，dev；G5 标定）
    let aug_chars = aug_block.as_deref().map(|s| s.chars().count()).unwrap_or(0);
    let mention_chars = mention_block
        .as_deref()
        .map(|s| s.chars().count())
        .unwrap_or(0);
    let p0_chars = ag::prompts::SYSTEM_PROMPT_CHAT.chars().count()
        + doc_ctx.as_deref().map(|s| s.chars().count()).unwrap_or(0)
        + prompt.chars().count();
    let hist_budget = CTX_BUDGET_CHARS.saturating_sub(p0_chars + aug_chars + mention_chars);
    let (kept_history, _dropped) = crate::ag::context::select_recent_history(&history, hist_budget);

    // 组装顺序（M-T-02 §3 recency）：系统 → 补强(P2) → 历史(P1) → 显式引用(P1) → 当前文档(P0,贴末尾) → 意图 → 当前输入。
    let mut messages = vec![serde_json::json!({
        "role": "system", "content": ag::prompts::SYSTEM_PROMPT_CHAT
    })];
    if let Some(aug) = &aug_block {
        messages.push(serde_json::json!({ "role": "system", "content": aug }));
    }
    for t in &kept_history {
        messages.push(serde_json::json!({ "role": t.role, "content": t.content }));
    }
    if let Some(m) = &mention_block {
        messages.push(serde_json::json!({ "role": "system", "content": m }));
    }
    if let Some(ctx) = &doc_ctx {
        messages.push(serde_json::json!({ "role": "system", "content": ctx }));
    }
    // 意图沿用判定（BR-AG-DATA-004，PI-AG-09C）：默认弱化历史、显式沿用才提权——当前轮前注入信号。
    let intent_hint =
        match crate::ag::context::decide_intent_carryover(&prompt, !kept_history.is_empty()) {
            crate::ag::context::Carryover::None => {
                "【本轮意图】这是一个新请求，对话历史仅作背景，不要让历史意图凌驾当前请求。"
            }
            crate::ag::context::Carryover::Refine => {
                "【本轮意图】用户在修正/追加上文的产出，请据最近一轮的相关内容继续。"
            }
            crate::ag::context::Carryover::Select => {
                "【本轮意图】用户在选择/沿用上文给出的方案，请据上文相应选项继续。"
            }
            crate::ag::context::Carryover::ReferenceTarget => {
                "【本轮意图】用户在延续上文的话题/目标，请参考相关历史意图继续。"
            }
        };
    messages.push(serde_json::json!({ "role": "system", "content": intent_hint }));
    messages.push(serde_json::json!({ "role": "user", "content": prompt }));

    // 工具定义（PI-AG-08B 只读 3 工具）；ws_root 已在上文 workspace 解析（X-FORBID-004 工具限界，PI-AG-09D）。
    let tools = ag::tool_registry::tool_definitions_json();

    // SM-AG-02 工具循环（BR-AG-STATE-002：注册/权限校验 + 上限）。dev 上限 6 轮。
    // 循环经状态机驱动 + 广播 sm:SM-AG-02（ModelStreaming→ToolValidating→ToolExecuting→ResultInjecting→
    // 续调 / FinalAnswer→LoopCompleted / 达上限→LoopAborted）。工具卡数据仍由 Channel 投影（互不替代）。
    use ag::tool::ToolLoopState as TlSt;
    const MAX_ROUNDS: usize = 6;
    for _round in 0..MAX_ROUNDS {
        emit_ag02(&app, &turn_id, TlSt::ModelStreaming); // 本轮模型流式
        let msgs = serde_json::Value::Array(messages.clone());
        let round =
            match ent::gateway::stream_round(&turn_id, &provider, &model, &msgs, &tools, &on_event)
                .await
            {
                Ok(o) => o,
                Err(e) => {
                    emit_ag01(&app, &turn_id, ag::turn::TurnState::TurnError); // 流式错误：回合错误态（可重试）
                    return Err(e);
                }
            };
        match round {
            ent::gateway::RoundOutcome::Done => {
                if requires_edit_tool && !proposed_diff {
                    let _ = on_event.send(AgTurnEvent::Error {
                        turn_id: turn_id.clone(),
                        code: "AG_EDIT_TOOL_REQUIRED".to_string(),
                        message: "本轮是编辑请求，但模型未调用 propose_edit 生成修改建议；文档没有被修改，请重试。".to_string(),
                        retriable: true,
                    });
                    emit_ag02(&app, &turn_id, TlSt::LoopAborted);
                    emit_ag01(&app, &turn_id, TurnState::TurnError);
                    return Ok(());
                }
                emit_ag02(&app, &turn_id, TlSt::LoopCompleted); // ModelStreaming→FinalAnswer→LoopCompleted
                emit_ag01_complete(&app, &turn_id); // → Completed
                return Ok(());
            }
            ent::gateway::RoundOutcome::ToolCalls(calls) => {
                emit_ag02(&app, &turn_id, TlSt::ToolValidating); // ModelStreaming→ToolRequest→ToolValidating
                                                                 // 先把 assistant 的 tool_calls 入历史（OpenAI 要求 tool 结果前有对应 assistant.tool_calls）。
                let tc_json: Vec<serde_json::Value> = calls
                    .iter()
                    .map(|c| {
                        serde_json::json!({
                            "id": c.id, "type": "function",
                            "function": { "name": c.name, "arguments": c.arguments }
                        })
                    })
                    .collect();
                messages.push(serde_json::json!({ "role": "assistant", "content": null, "tool_calls": tc_json }));

                for c in &calls {
                    let params: serde_json::Value =
                        serde_json::from_str(&c.arguments).unwrap_or(serde_json::Value::Null);
                    let _ = on_event.send(AgTurnEvent::ToolRequest {
                        turn_id: turn_id.clone(),
                        tool_name: c.name.clone(),
                        params: params.clone(),
                    });
                    let mut diff_event_after_tool: Option<(String, String)> = None;
                    // 注册/权限校验（BR-AG-STATE-002）→ 执行（限 workspace，X-FORBID-004）。
                    let tool_def = ag::tool_registry::lookup(&c.name);
                    let (result, success) = match ag::tool_registry::validate_requested_tool(
                        &c.name,
                    ) {
                        ag::tool::ToolDecision::Reject(reason) => (reason, false),
                        // 写工具 propose_edit：经 DE 生成 pending diff（X-FORBID-006，不直写）→ emit DiffProposed。
                        ag::tool::ToolDecision::Allow if c.name == "propose_edit" => {
                            match ag_propose_edit(
                                &state,
                                &params,
                                active_doc_content.as_deref(),
                                doc_is_html,
                                active_doc_path.as_deref(),
                                &declared_targets,
                            ) {
                                Ok(edit_result) => {
                                    proposed_diff = true;
                                    diff_event_after_tool = Some((
                                        edit_result.diff_id.clone(),
                                        edit_result.rel_path.clone(),
                                    ));
                                    let result = serde_json::json!({
                                        "summary": format!("已生成修改建议（diff_id={}），界面已显示待确认卡片，等待用户接受后应用；请勿重复提交。", edit_result.diff_id),
                                        "diff_id": edit_result.diff_id,
                                        "rel_path": edit_result.rel_path,
                                        "projection": edit_result.projection,
                                    });
                                    (
                                        serde_json::to_string(&result)
                                            .unwrap_or_else(|_| "已生成修改建议".to_string()),
                                        true,
                                    )
                                }
                                Err(e) => (format!("生成修改建议失败：{}", e.message), false),
                            }
                        }
                        // 结构工具：经 WS 边界执行，只改变文件树，不修改文档内容。
                        ag::tool::ToolDecision::Allow
                            if tool_def
                                .map(|t| {
                                    t.permission
                                        == ag::tool_registry::ToolPermission::StructureWrite
                                })
                                .unwrap_or(false) =>
                        {
                            match ws_root.as_deref() {
                                None => ("无活动 workspace，无法执行工具".to_string(), false),
                                Some(root) => {
                                    match ag::tool_exec::exec_structure_tool(
                                        std::path::Path::new(root),
                                        &c.name,
                                        &params,
                                    ) {
                                        Ok(r) => (r, true),
                                        Err(e) => (format!("工具执行失败：{}", e.message), false),
                                    }
                                }
                            }
                        }
                        // 只读工具：经 WS/KB 执行（限 active workspace）。
                        ag::tool::ToolDecision::Allow => match ws_root.as_deref() {
                            None => ("无活动 workspace，无法执行工具".to_string(), false),
                            Some(root) => {
                                match ag::tool_exec::exec_read_only(
                                    std::path::Path::new(root),
                                    &c.name,
                                    &params,
                                ) {
                                    Ok(r) => (r, true),
                                    Err(e) => (format!("工具执行失败：{}", e.message), false),
                                }
                            }
                        },
                    };
                    let _ = on_event.send(AgTurnEvent::ToolResult {
                        turn_id: turn_id.clone(),
                        tool_name: c.name.clone(),
                        result: result.clone(),
                        success,
                    });
                    if let Some((diff_id, rel_path)) = diff_event_after_tool {
                        let _ = on_event.send(AgTurnEvent::DiffProposed {
                            turn_id: turn_id.clone(),
                            diff_id,
                            rel_path,
                        });
                    }
                    // 结果回注模型（SM-AG-02 ResultInjecting）。
                    messages.push(serde_json::json!({ "role": "tool", "tool_call_id": c.id, "content": result }));
                }
                emit_ag02(&app, &turn_id, TlSt::ToolExecuting); // ToolValidating→ValidateOk→ToolExecuting
                emit_ag02(&app, &turn_id, TlSt::ResultInjecting); // ToolExecuting→ExecOk→ResultInjecting
                                                                  // 继续下一轮（ResultInjecting→Injected→ModelStreaming，下轮顶部广播）。
            }
        }
    }

    // 达工具循环上限（BR-AG-STATE-002 LoopAborted）：收尾完成终态。
    if requires_edit_tool && !proposed_diff {
        let _ = on_event.send(AgTurnEvent::Error {
            turn_id: turn_id.clone(),
            code: "AG_EDIT_TOOL_REQUIRED".to_string(),
            message: "本轮是编辑请求，但工具循环结束前未生成修改建议；文档没有被修改，请重试。"
                .to_string(),
            retriable: true,
        });
        emit_ag02(&app, &turn_id, TlSt::LoopAborted);
        emit_ag01(&app, &turn_id, TurnState::TurnError);
        return Ok(());
    }
    let _ = on_event.send(AgTurnEvent::Done {
        turn_id: turn_id.clone(),
        stop_reason: "tool_loop_limit".to_string(),
    });
    emit_ag02(&app, &turn_id, TlSt::LoopAborted); // ResultInjecting→LoopLimit→LoopAborted
    emit_ag01_complete(&app, &turn_id); // → Completed
    Ok(())
}

/// 辅助产出（A-AG-I-P-01 `ag.request_assist`）：候选或空。
#[derive(serde::Serialize)]
struct AssistResult {
    /// 前端 requestId；用于 latest-wins，旧响应由前端丢弃。
    request_id: String,
    /// 候选文本；门禁不可用 / 失败 / 空产出 → None（前端静默不提供，不打断编辑）。
    candidate: Option<String>,
    /// 辅助续写候选列表；autocomplete 期望最多 3 个。
    candidates: Vec<String>,
    /// 内部诊断原因；前端不直接暴露给普通用户。
    failure_reason: Option<String>,
}

#[tauri::command]
fn ag_finish_assist(
    assist_id: String,
    event: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), IpcError> {
    use ag::assist::{transition, AssistEvent, AssistState};

    if assist_is_terminal(&state, &assist_id) {
        return Ok(());
    }

    let event = match event.as_str() {
        "accept" => AssistEvent::Accept,
        "dismiss" => AssistEvent::Dismiss,
        "anchor_lost" => AssistEvent::AnchorLost,
        _ => {
            return Err(IpcError::new(
                "AG_ASSIST_FINISH_EVENT_INVALID",
                "未知辅助终态事件：仅支持 accept/dismiss/anchor_lost",
                false,
                false,
            ))
        }
    };
    let st = transition(AssistState::Presenting, event).ok_or_else(|| {
        IpcError::new("AG_ASSIST_FINISH_INVALID", "辅助终态转移无效", false, false)
    })?;
    mark_assist_terminal(&state, &assist_id, st);
    emit_ag03(&app, &assist_id, st);
    Ok(())
}

/**
 * @GOV
 * codes:    BR-AG-STATE-003-EFFECT-AG-AG-ASSIST-001
 * type:     EFFECT
 * chain:    AG-ASSIST
 * rules:    BR-AG-STATE-003, BR-AG-VERIFY-001
 * boundary: in=AssistRequest | out=AssistCandidate(gateGatedSilentOnDenyOrFailNeverInterruptsEditing)
 */
/// 命令 `ag_request_assist`（A-AG-I-P-01 request_assist · SM-AG-03 · 独立 async，不走 dispatch 信封）：
/// 前端经 SM-AG-03 节流（Idle→Throttling）后调本命令承接 throttlePass 之后。**门禁前置**（BR-AG-VERIFY-001）：
/// 门禁不可用 → ThrottlePass{false}→Idle，静默返回空候选（不报错、不打断编辑）；可用 → Requesting → 经网关
/// 单次产出 → CandidateOk→Presenting（回候选）/ RequestFail→Idle（静默）。采纳=用户编辑（前端经 ED，非本命令）。
/// 节流参数/成本策略延实现期标定（AG-03，A-AG-M-S-01 §4）。
#[tauri::command]
async fn ag_request_assist(
    assist_id: String,
    kind: String, // "inline"（⌘K 改写）/ "autocomplete"（⌘J 续写）
    doc_path: Option<String>,
    load_seq: Option<i64>,
    block_id: Option<String>,
    start: Option<i64>,
    end: Option<i64>,
    instruction: Option<String>, // inline：改写指令
    selection: Option<String>,   // inline：选区原文
    prefix: Option<String>,      // autocomplete：光标前文本
    before: Option<String>,      // 触发锚点前文指纹
    after: Option<String>,       // 触发锚点后文指纹
    base_text: Option<String>,   // inline refine：上一轮候选
    provider: Option<String>,
    model: Option<String>,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<AssistResult, IpcError> {
    use ag::assist::{transition, AssistEvent, AssistState};
    use ent::gateway::GateOutcome;

    let empty = |failure_reason: Option<&'static str>| AssistResult {
        request_id: assist_id.clone(),
        candidate: None,
        candidates: Vec::new(),
        failure_reason: failure_reason.map(str::to_string),
    };

    if assist_is_terminal(&state, &assist_id) {
        eprintln!(
            "[ag-assist] id={} kind={} skipped=stale_request",
            assist_id, kind
        );
        return Ok(empty(Some("stale_request")));
    }

    if kind != "inline" && kind != "autocomplete" {
        return Err(IpcError::new(
            "AG_ASSIST_KIND_INVALID",
            "未知辅助类型：仅支持 inline 或 autocomplete",
            false,
            false,
        ));
    }

    // 门禁前置（BR-AG-VERIFY-001 / X-SEQ-001）：每次辅助调用均先提交令牌给服务端/GW 裁决。
    let gate_state = match ent::gateway::check_gate().await {
        Ok(GateOutcome::Pass) => {
            eprintln!("[ag-assist] id={} kind={} gate=pass", assist_id, kind);
            ag::turn::GateState::Passed
        }
        Ok(GateOutcome::Deny) => {
            eprintln!("[ag-assist] id={} kind={} gate=deny", assist_id, kind);
            ag::turn::GateState::Denied
        }
        Err(e) => {
            eprintln!(
                "[ag-assist] id={} kind={} gate=offline code={}",
                assist_id, kind, e.code
            );
            ag::turn::GateState::Offline
        }
    };
    let gate_ok = ag::turn::ensure_gate_passed(gate_state).is_ok();
    // 前端已 debounce 至 Throttling；本命令承接 throttlePass{gate_available}。
    let after_throttle = transition(
        AssistState::Throttling,
        AssistEvent::ThrottlePass {
            gate_available: gate_ok,
        },
    )
    .unwrap_or(AssistState::Idle);
    emit_ag03(&app, &assist_id, after_throttle);
    if after_throttle != AssistState::Requesting {
        // 门禁不可用：静默不提供（SM-AG-03 ThrottlePass{false}→Idle）。
        let reason = match gate_state {
            ag::turn::GateState::Denied => "gate_denied",
            ag::turn::GateState::Offline => "gate_offline",
            _ => "gate_unavailable",
        };
        return Ok(empty(Some(reason)));
    }

    // 组装轻量辅助消息（无对话历史；按 assist kind 选择专用 system prompt，避免 JSON/单正文契约混用）。
    let provider = provider.unwrap_or_else(|| "deepseek".to_string());
    let model = model.unwrap_or_else(|| default_model(&provider));
    let system_prompt = if kind == "autocomplete" {
        ag::prompts::SYSTEM_PROMPT_AUTOCOMPLETE
    } else {
        ag::prompts::SYSTEM_PROMPT_INLINE_ASSIST
    };
    let context = format!(
        "【定位上下文】doc={} load_seq={} block_id={} range=[{},{}]\n【锚点前文指纹】{}\n【锚点后文指纹】{}",
        doc_path.as_deref().unwrap_or(""),
        load_seq.unwrap_or_default(),
        block_id.as_deref().unwrap_or(""),
        start.unwrap_or_default(),
        end.unwrap_or_default(),
        before.as_deref().unwrap_or(""),
        after.as_deref().unwrap_or("")
    );
    let user = if kind == "inline" {
        let base = base_text
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| selection.clone().unwrap_or_default());
        if selection.as_deref().unwrap_or("").trim().is_empty() && base.trim().is_empty() {
            format!(
                "{context}\n【插入指令】{}\n【光标前文本】{}\n请只返回要插入到光标处的正文。",
                instruction.unwrap_or_default(),
                prefix.unwrap_or_default()
            )
        } else {
            format!(
                "{context}\n【修改指令】{}\n【基底文本】{}\n请只返回修改后的正文。",
                instruction.unwrap_or_default(),
                base
            )
        }
    } else {
        format!(
            "{context}\n【续写以下文本，只给续写部分】\n{}\n请生成 3 个不同候选，每个 50-100 字。只返回 JSON 字符串数组，不要解释。",
            prefix.unwrap_or_default()
        )
    };
    let messages = serde_json::json!([
        { "role": "system", "content": system_prompt },
        { "role": "user", "content": user },
    ]);

    match ent::gateway::assist_complete(&provider, &model, &messages).await {
        Ok(text) if !text.trim().is_empty() => {
            if assist_is_terminal(&state, &assist_id) {
                eprintln!(
                    "[ag-assist] id={} kind={} skipped=stale_request_after_response",
                    assist_id, kind
                );
                return Ok(empty(Some("stale_request")));
            }
            if kind == "autocomplete" {
                let candidates = ag::assist::parse_autocomplete_candidates(&text);
                if candidates.len() != 3 {
                    eprintln!(
                        "[ag-assist] id={} kind={} failure=parse_underflow raw_chars={} parsed_count={}",
                        assist_id,
                        kind,
                        text.chars().count(),
                        candidates.len()
                    );
                    let st = transition(AssistState::Requesting, AssistEvent::CandidateUnderflow)
                        .expect("Requesting+CandidateUnderflow→Presenting");
                    emit_ag03(&app, &assist_id, st);
                    return Ok(empty(Some("parse_underflow")));
                }
                eprintln!(
                    "[ag-assist] id={} kind={} result=candidate_ok raw_chars={} parsed_count=3",
                    assist_id,
                    kind,
                    text.chars().count()
                );
                let st = transition(AssistState::Requesting, AssistEvent::CandidateOk)
                    .expect("Requesting+CandidateOk→Presenting");
                emit_ag03(&app, &assist_id, st);
                return Ok(AssistResult {
                    request_id: assist_id,
                    candidate: candidates.first().cloned(),
                    candidates,
                    failure_reason: None,
                });
            }
            eprintln!(
                "[ag-assist] id={} kind={} result=candidate_ok raw_chars={}",
                assist_id,
                kind,
                text.chars().count()
            );
            let st = transition(AssistState::Requesting, AssistEvent::CandidateOk)
                .expect("Requesting+CandidateOk→Presenting");
            emit_ag03(&app, &assist_id, st);
            Ok(AssistResult {
                request_id: assist_id,
                candidate: Some(text.trim().to_string()),
                candidates: Vec::new(),
                failure_reason: None,
            })
        }
        Ok(_) => {
            eprintln!(
                "[ag-assist] id={} kind={} failure=empty_output",
                assist_id, kind
            );
            // 失败 / 空产出：静默回 Idle（不弹错误、不打断编辑，SM-AG-03 RequestFail）。
            if !assist_is_terminal(&state, &assist_id) {
                let st = transition(AssistState::Requesting, AssistEvent::RequestFail)
                    .expect("Requesting+RequestFail→Idle");
                emit_ag03(&app, &assist_id, st);
            }
            Ok(empty(Some("empty_output")))
        }
        Err(e) => {
            eprintln!(
                "[ag-assist] id={} kind={} failure=gateway_error code={}",
                assist_id, kind, e.code
            );
            if !assist_is_terminal(&state, &assist_id) {
                let st = transition(AssistState::Requesting, AssistEvent::RequestFail)
                    .expect("Requesting+RequestFail→Idle");
                emit_ag03(&app, &assist_id, st);
            }
            Ok(empty(Some("gateway_error")))
        }
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // workspace.db 落应用数据目录（控制态，非用户正文；承 BR-SYS-PERSIST-001 迁移框架）。
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let conn = db::open_and_migrate(&data_dir.join("workspace.db"))?;
            app.manage(AppState {
                db: Mutex::new(conn),
                registry: build_registry(),
                assist_terminals: Mutex::new(HashMap::new()),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            dispatch,
            ag_start_turn,
            byok_upload,
            ag_request_assist,
            ag_finish_assist
        ])
        .run(tauri::generate_context!())
        .expect("error while running Binder");
}
