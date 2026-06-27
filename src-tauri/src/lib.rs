//! Binder 后端骨架（Tauri 2）。
//! 模块边界承接 A-SYS-C-T-01 §4 分层与各模块 Plan Item 的文件范围；
//! 各模块为 G3 脚手架占位，@GOV 块在 G4 实现时随实际逻辑落入对应模块。
//! 分层约束 X-SEQ-002：前端经 command 触达，后端经领域逻辑触达 FS/db。

use std::sync::Mutex;
use tauri::{Manager, State};

// —— 系统底座（PI-SYS-01~04）——
pub mod ipc; // PI-SYS-01 IPC 信封/错误/command 注册表
pub mod types; // PI-SYS-02 跨模块共享数据结构
pub mod db; // PI-SYS-03 workspace.db schema 与版本化迁移
pub mod session; // PI-SYS-04 会话与订阅门禁状态机骨架

// —— P2 编辑核心 ——
pub mod workspace; // PI-WS-* 工作台
pub mod doc; // PI-DOC-* 文档转换（编辑链/预览链）

// —— P3 对话编辑与 Agent ——
pub mod de; // PI-DE-* 对话编辑（diff 终态）
pub mod ag; // PI-AG-* 对话代理（门禁前置/工具/辅助）
pub mod reference; // PI-REF-* 引用（@mention）

// —— P4 上层能力 ——
pub mod cbt; // PI-CBT-* 构建
pub mod ast; // PI-AST-* 记忆
pub mod kb; // PI-KB-* 知识库
pub mod tpl; // PI-TPL-* 模板/工作流
pub mod prv; // PI-PRV-* 预览输出

// —— 商业化客户端侧 ——
pub mod acct; // PI-ACCT-* 账号会话
pub mod ent; // PI-ENT-* 订阅门控

use ipc::{AgTurnEvent, CommandRegistry, CommandRequest, CommandResponse, IpcError};
use rusqlite::Connection;

/// 应用运行时状态：workspace.db 连接 + command 注册表（PI-FE-00 联通后端）。
struct AppState {
    db: Mutex<Connection>,
    registry: CommandRegistry,
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
    reg.register("ws.check_external_change");
    reg.register("ws.create_doc");
    reg.register("ws.rename_doc");
    reg.register("ws.delete_doc");
    reg.register("prv.preview");
    reg.register("prv.export");
    reg.register("prv.media_src");
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
fn dispatch(app: tauri::AppHandle, request: CommandRequest, state: State<AppState>) -> CommandResponse {
    let correlation_id = request.correlation_id.clone();
    if let Err(err) = state.registry.validate(&request) {
        return CommandResponse { correlation_id, result: None, error: Some(err) };
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
        "ws.save_doc" => route_save_doc(&request, &state),
        "ws.persist_diff" => route_persist_diff(&request, &state),
        "ws.list_pending_diffs" => route_list_pending_diffs(&request, &state),
        "ws.apply_diff" => route_apply_diff(&request, &state),
        "ws.discard_diff" => route_discard_diff(&request, &state),
        "ws.check_external_change" => route_check_external_change(&request, &state),
        "ws.create_doc" => route_create_doc(&request, &state),
        "ws.rename_doc" => route_rename_doc(&request, &state),
        "ws.delete_doc" => route_delete_doc(&request, &state),
        "prv.preview" => route_preview(&request, &state),
        "prv.export" => route_export(&request, &state),
        "prv.media_src" => route_media_src(&request, &state, &app),
        "doc.create_draft" => route_create_draft(&request, &state),
        "doc.open_for_edit" => route_open_for_edit(&request, &state),
        "doc.save_from_edit" => route_save_from_edit(&request, &state),
        "ws.list_timeline" => route_list_timeline(&request, &state),
        "ws.preview_node" => route_preview_node(&request, &state),
        "ws.restore_node" => route_restore_node(&request, &state),
        "de.generate_diff" => route_generate_diff(&request, &state),
        "de.accept_diff" => route_accept_diff(&request, &state),
        "de.reject_diff" => route_reject_diff(&request, &state),
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
        Ok(result) => CommandResponse { correlation_id, result: Some(result), error: None },
        Err(error) => CommandResponse { correlation_id, result: None, error: Some(error) },
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
fn lock_db<'a>(
    state: &'a AppState,
) -> Result<std::sync::MutexGuard<'a, Connection>, IpcError> {
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
fn route_close_workspace(request: &CommandRequest, state: &AppState) -> Result<serde_json::Value, IpcError> {
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
    let query = request.params.get("query").and_then(|v| v.as_str()).unwrap_or("");
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

/// 解析活动 workspace 内相对路径为绝对路径（X-FORBID-004 守界）。
fn resolve_active(state: &AppState, rel_path: &str) -> Result<(crate::types::WorkspaceRef, std::path::PathBuf), IpcError> {
    let ws = active_ws(state)?;
    let abs = workspace::resolve_in_workspace(std::path::Path::new(&ws.root_path), rel_path)?;
    Ok((ws, abs))
}

/// 路由 `ws.read_doc`：读活动 workspace 内文档内容 + 加载基线（BR-WS-OBS-001 / X-FORBID-004）。
fn route_read_doc(request: &CommandRequest, state: &AppState) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let (_, abs) = resolve_active(state, &rel_path)?;
    let read = workspace::save::read_doc(&abs)?;
    ws_json(&read)
}

/// 路由 `ws.save_doc`：原子写入（带基线校验 + 显式 override），回新基线（BR-WS-PERSIST-001 / SM-WS-01）。
fn route_save_doc(request: &CommandRequest, state: &AppState) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let content = request
        .params
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| IpcError::new("WS_CONTENT_REQUIRED", "缺少 content 参数", false, false))?;
    let baseline = request.params.get("baseline").and_then(|v| v.as_u64());
    let override_external = request.params.get("override").and_then(|v| v.as_bool()).unwrap_or(false);
    let (_, abs) = resolve_active(state, &rel_path)?;
    let saved = workspace::save::save_atomic(&abs, content.as_bytes(), baseline, override_external)?;
    Ok(serde_json::json!({ "baseline": saved.baseline }))
}

/// 路由 `ws.persist_diff`：持久化 PendingDiff 为 status=pending（重启恢复，BR-WS-STATE-001）。
fn route_persist_diff(request: &CommandRequest, state: &AppState) -> Result<serde_json::Value, IpcError> {
    let diff: crate::types::PendingDiff = serde_json::from_value(request.params.clone())
        .map_err(|e| IpcError::new("WS_DIFF_MALFORMED", &format!("PendingDiff 解析失败: {e}"), false, false))?;
    let conn = lock_db(state)?;
    workspace::diff_apply::persist_diff(&conn, &diff)
        .map_err(|e| IpcError::new("WS_DB_ERROR", &e.to_string(), true, false))?;
    Ok(serde_json::json!({ "diff_id": diff.diff_id }))
}

/// 路由 `ws.list_pending_diffs`：回活动 workspace 未裁决 diff 列表（重启恢复入口）。
fn route_list_pending_diffs(request: &CommandRequest, state: &AppState) -> Result<serde_json::Value, IpcError> {
    let status = request.params.get("status").and_then(|v| v.as_str());
    let ws = active_ws(state)?;
    let conn = lock_db(state)?;
    let list = workspace::diff_apply::list_pending_diffs(&conn, &ws.workspace_id, status)
        .map_err(|e| IpcError::new("WS_DB_ERROR", &e.to_string(), true, false))?;
    ws_json(&list)
}

/// 路由 `ws.apply_diff`：accepted 唯一落盘路径——原子写 + 记一个时间轴节点 + 置 accepted（BR-WS-STATE-001）。
/// afterContent 由 DE 产出方提供（payload_ref→内容解析归 DE）；本路由执行 WS 侧落盘 effect。
fn route_apply_diff(request: &CommandRequest, state: &AppState) -> Result<serde_json::Value, IpcError> {
    let diff_id = str_param(request, "diffId", "WS_DIFF_ID_REQUIRED")?;
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let after_content = request
        .params
        .get("afterContent")
        .and_then(|v| v.as_str())
        .ok_or_else(|| IpcError::new("WS_CONTENT_REQUIRED", "缺少 afterContent 参数", false, false))?;
    let baseline = request.params.get("baseline").and_then(|v| v.as_u64());
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
    let entry = workspace::diff_apply::apply_diff(&conn, &diff_id, &doc, &abs, after_content.as_bytes(), baseline)?;
    ws_json(&entry)
}

/// 路由 `ws.check_external_change`：比对磁盘当前哈希与加载基线（BR-WS-OBS-001 / X-FORBID-004）。
/// 前端轮询此命令实现 external_change 逻辑契约（物理事件延 SY-02）。
fn route_check_external_change(request: &CommandRequest, state: &AppState) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let baseline = request
        .params
        .get("baseline")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| IpcError::new("WS_BASELINE_REQUIRED", "缺少 baseline 参数", false, false))?;
    let (_, abs) = resolve_active(state, &rel_path)?;
    let check = workspace::save::check_external_change(&abs, baseline)?;
    ws_json(&check)
}

/// 路由 `ws.create_doc`：workspace 内新建文档（守界，已存在拒绝；BR-WS-DATA-001）。
fn route_create_doc(request: &CommandRequest, state: &AppState) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let content = request.params.get("content").and_then(|v| v.as_str()).unwrap_or("");
    let (_, abs) = resolve_active(state, &rel_path)?;
    let baseline = workspace::fileops::create_doc(&abs, content.as_bytes())?;
    Ok(serde_json::json!({ "rel_path": rel_path, "baseline": baseline }))
}

/// 路由 `ws.rename_doc`：同 workspace 内重命名/移动（两端守界，目标已存在拒绝；BR-WS-DATA-001 / §8 move）。
fn route_rename_doc(request: &CommandRequest, state: &AppState) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let new_rel_path = str_param(request, "newRelPath", "WS_NEW_PATH_REQUIRED")?;
    let (_, from_abs) = resolve_active(state, &rel_path)?;
    let (_, to_abs) = resolve_active(state, &new_rel_path)?;
    workspace::fileops::rename_doc(&from_abs, &to_abs)?;
    Ok(serde_json::json!({ "rel_path": new_rel_path }))
}

/// 路由 `ws.delete_doc`：删除 workspace 内文档（守界，不存在拒绝；BR-WS-DATA-001）。
fn route_delete_doc(request: &CommandRequest, state: &AppState) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let (_, abs) = resolve_active(state, &rel_path)?;
    workspace::fileops::delete_doc(&abs)?;
    Ok(serde_json::json!({ "rel_path": rel_path, "deleted": true }))
}

/// 路由 `prv.preview`：docx/Excel/PPT 经 DOC 预览链出只读 PDF 字节（守界 X-FORBID-004，只读 PRV 硬边界）。
fn route_preview(request: &CommandRequest, state: &AppState) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let (_, abs) = resolve_active(state, &rel_path)?;
    let ext = std::path::Path::new(&rel_path).extension().and_then(|e| e.to_str()).unwrap_or("");
    let bytes = prv::render_preview(&abs, ext)?;
    Ok(serde_json::json!({ "format": "pdf", "bytes": bytes }))
}

/// 路由 `prv.media_src`：守界解析媒体文件绝对路径（X-FORBID-004）+ 仅把**该单个文件**加入 asset 协议
/// scope（最小暴露，不开放整目录）；前端 convertFileSrc(abs) 流式播放音视频（不进内存，只读）。
fn route_media_src(request: &CommandRequest, state: &AppState, app: &tauri::AppHandle) -> Result<serde_json::Value, IpcError> {
    use tauri::Manager;
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let (_, abs) = resolve_active(state, &rel_path)?;
    if !abs.is_file() {
        return Err(IpcError::new("PRV_NOT_FOUND", "媒体文件不存在", false, false));
    }
    app.asset_protocol_scope()
        .allow_file(&abs)
        .map_err(|e| IpcError::new("PRV_SCOPE_FAILED", &format!("授权媒体访问失败: {e}"), false, false))?;
    Ok(serde_json::json!({ "abs_path": abs.to_string_lossy() }))
}

/// 路由 `prv.export`：源文档导出为目标格式，落点 = 同目录同名换扩展（workspace 内，守界 X-FORBID-004）。
fn route_export(request: &CommandRequest, state: &AppState) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let format = str_param(request, "format", "PRV_FORMAT_REQUIRED")?;
    let src_ext = std::path::Path::new(&rel_path).extension().and_then(|e| e.to_str()).unwrap_or("").to_string();
    // 落点：源 rel_path 去扩展 + 目标扩展（同目录）。
    let stem = rel_path.rsplit_once('.').map(|(s, _)| s).unwrap_or(&rel_path);
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
fn route_create_draft(request: &CommandRequest, state: &AppState) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let p = std::path::Path::new(&rel_path);
    let stem = p
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| IpcError::new("DOC_PATH_INVALID", "无法解析文件名", false, false))?;
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("docx");
    let parent = p.parent().map(|x| x.to_string_lossy().to_string()).filter(|s| !s.is_empty());
    let draft_rel = match parent {
        Some(d) => format!("{d}/{stem}.draft.{ext}"),
        None => format!("{stem}.draft.{ext}"),
    };
    let (_, abs_src) = resolve_active(state, &rel_path)?;
    let (_, abs_draft) = resolve_active(state, &draft_rel)?;
    if !abs_src.exists() {
        return Err(IpcError::new("DOC_NOT_FOUND", "原文档不存在", false, false));
    }
    let _ = std::fs::remove_file(&abs_draft); // 已存在草稿先覆盖
    std::fs::copy(&abs_src, &abs_draft)
        .map_err(|e| IpcError::new("DOC_DRAFT_FAILED", &format!("创建草稿失败: {e}"), true, false))?;
    Ok(serde_json::json!({ "rel_path": draft_rel }))
}

fn route_open_for_edit(request: &CommandRequest, state: &AppState) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let (_, abs) = resolve_active(state, &rel_path)?;
    // 加载基线：docx 原始字节内容哈希（供 save_from_edit 校验外部变更，BR-WS-OBS-001）。
    let baseline = std::fs::read(&abs)
        .map(|b| workspace::save::content_hash(&b))
        .map_err(|e| IpcError::new("DOC_READ_FAILED", &format!("读 docx 失败: {e}"), true, false))?;
    // 编辑链：docx → HTML；extract-media 工作目录按文档隔离（图片解出供编辑器；缓存策略实现期）。
    let work_dir = std::env::temp_dir().join("binder_edit_media").join(format!("{baseline:x}"));
    let html = doc::edit_chain::docx_to_html(&abs, &work_dir)?;
    Ok(serde_json::json!({ "format": "html", "html": html, "baseline": baseline }))
}

/// 路由 `doc.save_from_edit`（A-DOC-I-P-01 §2 编辑链）：编辑 HTML → docx（--reference-doc 统一样式）→
/// 交 WS 原子落盘（SM-WS-01，baseline 校验外部变更、原文件不变保护）。转换失败回 convert_failed
/// （SM-DOC-01 SaveError，原 docx 不变）。
fn route_save_from_edit(request: &CommandRequest, state: &AppState) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let html = request
        .params
        .get("html")
        .and_then(|v| v.as_str())
        .ok_or_else(|| IpcError::new("DOC_HTML_REQUIRED", "缺少 html 参数", false, false))?;
    let baseline = request.params.get("baseline").and_then(|v| v.as_u64());
    let override_external = request.params.get("override").and_then(|v| v.as_bool()).unwrap_or(false);
    let (_, abs) = resolve_active(state, &rel_path)?;
    // 编辑链回写：HTML → docx 字节（失败原文件不变）。
    let docx_bytes = doc::edit_chain::html_to_docx(html)?;
    // WS 原子落盘 + 外部变更保护（SM-WS-01）。
    let saved = workspace::save::save_atomic(&abs, &docx_bytes, baseline, override_external)?;
    Ok(serde_json::json!({ "baseline": saved.baseline }))
}

/// 路由 `ws.list_timeline`：回文档时间轴节点（version_seq 升序，BR-WS-DATA-002）。
fn route_list_timeline(request: &CommandRequest, state: &AppState) -> Result<serde_json::Value, IpcError> {
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
fn route_preview_node(request: &CommandRequest, state: &AppState) -> Result<serde_json::Value, IpcError> {
    let entry_id = str_param(request, "entryId", "WS_ENTRY_ID_REQUIRED")?;
    let conn = lock_db(state)?;
    let bytes = workspace::timeline::preview_node(&conn, &entry_id)?;
    Ok(serde_json::json!({ "content": String::from_utf8_lossy(&bytes) }))
}

/// 路由 `ws.restore_node`：取回快照→原子写回文档（守界）→记一个新时间轴节点（前进式还原，BR-WS-STATE-002）。
fn route_restore_node(request: &CommandRequest, state: &AppState) -> Result<serde_json::Value, IpcError> {
    let entry_id = str_param(request, "entryId", "WS_ENTRY_ID_REQUIRED")?;
    let ws = active_ws(state)?;
    let conn = lock_db(state)?;
    let (rel_path, snapshot_ref) = workspace::timeline::node_locator(&conn, &entry_id)
        .map_err(|e| IpcError::new("WS_DB_ERROR", &e.to_string(), true, false))?
        .ok_or_else(|| IpcError::new("WS_NODE_NOT_FOUND", "时间轴节点不存在", false, false))?;
    let content = workspace::timeline::load_snapshot(&conn, &snapshot_ref)
        .map_err(|e| IpcError::new("WS_DB_ERROR", &e.to_string(), true, false))?
        .ok_or_else(|| IpcError::new("WS_SNAPSHOT_UNAVAILABLE", "快照不可取回", false, false))?;
    let abs = workspace::resolve_in_workspace(std::path::Path::new(&ws.root_path), &rel_path)?;
    // 原子写回（override：还原即以历史内容覆盖现状，用户显式动作）。
    workspace::save::save_atomic(&abs, &content, None, true)?;
    let doc = crate::types::DocRef {
        workspace_id: ws.workspace_id,
        doc_type: doc_type_of(&rel_path),
        rel_path,
    };
    // 前进式记新节点（不抹历史；复用同一快照内容，内容寻址去重）。
    let entry = workspace::timeline::append_node(&conn, &doc, "restore", &snapshot_ref, Some("还原"))
        .map_err(|e| IpcError::new("WS_DB_ERROR", &e.to_string(), true, false))?;
    ws_json(&entry)
}

/// 路由 `ws.discard_diff`：rejected/expired 仅置终态，不写文件、不记节点（BR-WS-STATE-001）。
fn route_discard_diff(request: &CommandRequest, state: &AppState) -> Result<serde_json::Value, IpcError> {
    let diff_id = str_param(request, "diffId", "WS_DIFF_ID_REQUIRED")?;
    let terminal = str_param(request, "terminal", "WS_TERMINAL_REQUIRED")?;
    if terminal != "rejected" && terminal != "expired" {
        return Err(IpcError::new("WS_TERMINAL_INVALID", "terminal 须为 rejected/expired", false, false));
    }
    let conn = lock_db(state)?;
    workspace::diff_apply::discard_diff(&conn, &diff_id, &terminal)
        .map_err(|e| IpcError::new("WS_DB_ERROR", &e.to_string(), true, false))?;
    Ok(serde_json::json!({ "diff_id": diff_id, "status": terminal }))
}

// —— DE diff 链路（PI-BE-02）：生成→裁决→落盘经 dispatch 接入 ——

fn now_millis() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// 不透明 id（diff_id / payload_ref 占位；与 lifecycle ID 同策略，A 类债务）。
fn gen_id(prefix: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    format!("{prefix}_{}_{}", now_millis(), N.fetch_add(1, Ordering::Relaxed))
}

fn doc_type_of(rel_path: &str) -> String {
    std::path::Path::new(rel_path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_else(|| "bin".to_string())
}

/// 路由 `de.generate_diff`：组装 ChangeProposal→锚校验生成 PendingDiff（BR-DE-DATA-001）→持久化
/// DiffPayload + pending_diff（重启恢复）；修改既有须锚 Located，否则 DE_ANCHOR_LOST（不产悬空 diff）。
fn route_generate_diff(request: &CommandRequest, state: &AppState) -> Result<serde_json::Value, IpcError> {
    use crate::de::generate::{build_payload, generate_diff, ChangeProposal};
    use crate::types::LocateResult;

    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let after_content = request
        .params
        .get("afterContent")
        .and_then(|v| v.as_str())
        .ok_or_else(|| IpcError::new("WS_CONTENT_REQUIRED", "缺少 afterContent 参数", false, false))?
        .to_string();
    let before_snapshot = request.params.get("beforeSnapshot").and_then(|v| v.as_str()).map(|s| s.to_string());
    let origin_module = request.params.get("originModule").and_then(|v| v.as_str()).unwrap_or("AG").to_string();
    let origin_task = request.params.get("originTask").and_then(|v| v.as_str()).unwrap_or("").to_string();
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
    let proposal = ChangeProposal {
        origin_task,
        target,
        after_content: after_content.into_bytes(),
        before_snapshot,
    };
    let diff_id = gen_id("diff");
    let payload_ref = format!("pl_{diff_id}");
    // 锚校验（修改既有须 Located；新建跳过）——失败即 GenFailed 回报，不持久化。
    let pending = generate_diff(&proposal, &diff_id, &origin_module, &payload_ref, now_millis(), anchor_locate)?;
    let payload = build_payload(&proposal);

    let conn = lock_db(state)?;
    crate::de::store::persist_payload(&conn, &payload_ref, &payload)
        .map_err(|e| IpcError::new("DE_DB_ERROR", &e.to_string(), true, false))?;
    workspace::diff_apply::persist_diff(&conn, &pending)
        .map_err(|e| IpcError::new("DE_DB_ERROR", &e.to_string(), true, false))?;
    Ok(serde_json::json!({ "diff_id": diff_id, "status": "pending" }))
}

/// 路由 `de.accept_diff`：取回 DiffPayload→ws.apply_diff 落盘（accepted 唯一触发，X-INTER-001）；回时间轴节点。
fn route_accept_diff(request: &CommandRequest, state: &AppState) -> Result<serde_json::Value, IpcError> {
    let diff_id = str_param(request, "diffId", "WS_DIFF_ID_REQUIRED")?;
    let ws = active_ws(state)?;
    let conn = lock_db(state)?;
    let stored = crate::de::store::load_payload_for_diff(&conn, &diff_id)?;
    let abs = workspace::resolve_in_workspace(std::path::Path::new(&ws.root_path), &stored.rel_path)?;
    let doc = crate::types::DocRef {
        workspace_id: ws.workspace_id,
        doc_type: doc_type_of(&stored.rel_path),
        rel_path: stored.rel_path,
    };
    let entry = workspace::diff_apply::apply_diff(&conn, &diff_id, &doc, &abs, &stored.after_content, None)?;
    ws_json(&entry)
}

/// 路由 `de.reject_diff`：ws.discard_diff(rejected)，不写文件、不记节点（终态不可逆，SM-DE-01）。
fn route_reject_diff(request: &CommandRequest, state: &AppState) -> Result<serde_json::Value, IpcError> {
    let diff_id = str_param(request, "diffId", "WS_DIFF_ID_REQUIRED")?;
    let conn = lock_db(state)?;
    workspace::diff_apply::discard_diff(&conn, &diff_id, "rejected")
        .map_err(|e| IpcError::new("DE_DB_ERROR", &e.to_string(), true, false))?;
    Ok(serde_json::json!({ "diff_id": diff_id, "status": "rejected" }))
}

/// 路由 `de.list_diffs`：活动 workspace 未裁决 diff 列表（重启恢复，复用 WS 持久化查询）。
fn route_list_diffs(request: &CommandRequest, state: &AppState) -> Result<serde_json::Value, IpcError> {
    let status = request.params.get("status").and_then(|v| v.as_str());
    let ws = active_ws(state)?;
    let conn = lock_db(state)?;
    let list = workspace::diff_apply::list_pending_diffs(&conn, &ws.workspace_id, status)
        .map_err(|e| IpcError::new("DE_DB_ERROR", &e.to_string(), true, false))?;
    ws_json(&list)
}

/**
 * @GOV
 * codes:    BR-AG-VERIFY-001-GUARD-AG-AG-TURN-002
 * type:     GUARD
 * chain:    AG-TURN
 * rules:    BR-AG-VERIFY-001
 * term_ref: TERM-AG-001, TERM-CLOUD-001
 * boundary: in=turnPrompt | out=AgentTurnReply(gateGatedBeforeModelStubbed)
 */
/// 路由 `ag.start_turn`（桩）：驱动 SM-AG-01（Idle→Assembling→AwaitingGate），经 ensure_gate_passed
/// （SubscriptionGate 桩=Passed，X-SEQ-001 门禁前置结构保留；真实门禁服务端权威 A-SYS-I-P-03）→Streaming，
/// 回桩化助手回复。模型实际调用与流式分片落实现期（云网关接通）；本桩保留"门禁先于模型"不变量。
fn route_start_turn(request: &CommandRequest) -> Result<serde_json::Value, IpcError> {
    use ag::turn::{ensure_gate_passed, transition, GateState, TurnEvent, TurnState};

    let prompt = request
        .params
        .get("prompt")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| IpcError::new("AG_PROMPT_REQUIRED", "缺少 prompt 参数", false, false))?;

    // 结构性走 SM-AG-01：唯一通往 Streaming（模型调用）的路径是 AwaitingGate + gatePassed。
    let s = TurnState::Idle;
    let s = transition(s, TurnEvent::Send).ok_or_else(sm_error)?; // → Assembling
    let s = transition(s, TurnEvent::Assembled).ok_or_else(sm_error)?; // → AwaitingGate
    // 门禁前置硬门（X-SEQ-001 / BR-AG-VERIFY-001）：桩 gate=Passed；无有效订阅时此处 Denied 即拒。
    ensure_gate_passed(GateState::Passed)?;
    let s = transition(s, TurnEvent::GatePassed).ok_or_else(sm_error)?; // → Streaming
    debug_assert_eq!(s, TurnState::Streaming);

    let reply = format!(
        "（桩回复）已收到：{}。模型网关将在实现期接通（A-SYS-I-P-03），届时此处为真实流式产出。",
        prompt.trim()
    );
    Ok(serde_json::json!({ "reply": reply, "turn_state": "streaming" }))
}

fn sm_error() -> IpcError {
    IpcError::new("AG_SM_INVALID", "SM-AG-01 非法转移", false, false)
}

/// 路由 `ag.list_blocks`：列指定文档的块（供块级 @mention 选择非当前文档的块，PI-AG-09G）。
/// 限 active workspace（X-FORBID-004）；read_doc 含 docx；is_html 自判 text/html 分块。
fn route_list_blocks(request: &CommandRequest, state: &AppState) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let ws = active_ws(state)?;
    let abs = workspace::resolve_in_workspace(std::path::Path::new(&ws.root_path), &rel_path)?;
    let read = workspace::save::read_doc(&abs)?;
    let is_html = ag::blocks::is_html(&read.content, None);
    let blocks: Vec<serde_json::Value> = ag::blocks::parse_blocks(&read.content, is_html)
        .into_iter()
        .map(|b| serde_json::json!({ "block_id": b.block_id, "text": b.text }))
        .collect();
    Ok(serde_json::json!({ "blocks": blocks }))
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
    text: String,
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

/// PI-AG-08C/08E/08F · propose_edit 写工具执行（复用 route_generate_diff 逻辑）。
///
/// AI 修改建议经 DE 生成 pending diff，**不直写文件**（X-FORBID-006）；限 active workspace（X-FORBID-004）。
/// 两模式：整篇（content）/ 块级（block_index+new_text，基准用 base_content 切块对齐块号，PI-AG-08E）。
/// 返回 (diff_id, rel_path)。
fn ag_propose_edit(
    state: &AppState,
    params: &serde_json::Value,
    base_content: Option<&str>,
    is_html: bool,
) -> Result<(String, String), IpcError> {
    use crate::de::generate::{build_payload, generate_diff, ChangeProposal};
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

    // 块模式（block_index+new_text）：基准用注入给模型的当前内容切块、替目标块、重建整篇（块号对齐）。
    // 整篇模式（content）：直接整篇替换。
    let after_content = if params.get("block_id").is_some() || params.get("block_index").is_some() {
        // 块模式：需 new_text + 基准内容（注入给模型的当前内容，块锚对齐）。
        let new_text = params
            .get("new_text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| IpcError::new("AG_TOOL_ARG", "propose_edit 块模式缺 new_text 参数", false, false))?;
        let base = base_content
            .map(|s| s.to_string())
            .or_else(|| std::fs::read_to_string(&abs).ok())
            .ok_or_else(|| IpcError::new("AG_DOC_READ", "无法读取文档内容以做块编辑", false, false))?;
        if let Some(block_id) = params.get("block_id").and_then(|v| v.as_str()).filter(|s| !s.trim().is_empty()) {
            // 真实 blockId 锚（HTML data-block-id，PI-AG-08G）：按 id 定位；锚失效报错不误改。
            crate::ag::blocks::replace_block_by_id(&base, block_id, new_text)
                .map_err(|m| IpcError::new("AG_ANCHOR_LOST", &m, true, false))?
        } else {
            // 位置块号（text 或回退）。
            let bi = params
                .get("block_index")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| IpcError::new("AG_TOOL_ARG", "propose_edit 缺 block_id/block_index", false, false))?;
            crate::ag::blocks::replace_block(&base, is_html, bi as usize, new_text)
                .map_err(|m| IpcError::new("AG_TOOL_ARG", &m, false, false))?
        }
    } else {
        params
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                IpcError::new("AG_TOOL_ARG", "propose_edit 缺 content 或 block_id/block_index 参数", false, false)
            })?
            .to_string()
    };

    // 既有文档 → before_snapshot=Some（modify）；不存在 → None（新建，is_new_doc 派生）。
    let before_snapshot = std::fs::read_to_string(&abs).ok();

    let proposal = ChangeProposal {
        origin_task: "ag_turn".to_string(),
        target: crate::types::DocRef {
            workspace_id: ws.workspace_id,
            doc_type: doc_type_of(&rel_path),
            rel_path: rel_path.clone(),
        },
        after_content: after_content.into_bytes(),
        before_snapshot,
    };
    let diff_id = gen_id("diff");
    let payload_ref = format!("pl_{diff_id}");
    // 锚校验（既有须 Located；新建跳过）——失败 GenFailed 回报，不持久化（BR-DE-DATA-001）。
    let pending = generate_diff(&proposal, &diff_id, "AG", &payload_ref, now_millis(), LocateResult::Located)?;
    let payload = build_payload(&proposal);

    let conn = lock_db(state)?;
    crate::de::store::persist_payload(&conn, &payload_ref, &payload)
        .map_err(|e| IpcError::new("DE_DB_ERROR", &e.to_string(), true, false))?;
    workspace::diff_apply::persist_diff(&conn, &pending)
        .map_err(|e| IpcError::new("DE_DB_ERROR", &e.to_string(), true, false))?;
    Ok((diff_id, rel_path))
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
    on_event: tauri::ipc::Channel<AgTurnEvent>,
    state: tauri::State<'_, AppState>,
) -> Result<(), IpcError> {
    use ag::turn::{ensure_gate_passed, transition, GateState, TurnEvent, TurnState};

    if prompt.trim().is_empty() {
        return Err(IpcError::new("AG_PROMPT_REQUIRED", "缺少 prompt", false, false));
    }

    // 结构性门禁前置：唯一通往 Streaming（模型调用）的路径是 AwaitingGate + gatePassed（X-SEQ-001）。
    let s = transition(TurnState::Idle, TurnEvent::Send).ok_or_else(sm_error)?; // → Assembling
    let s = transition(s, TurnEvent::Assembled).ok_or_else(sm_error)?; // → AwaitingGate
    ensure_gate_passed(GateState::Passed)?; // dev 桩=Passed；无订阅时此处 Denied 即拒
    let s = transition(s, TurnEvent::GatePassed).ok_or_else(sm_error)?; // → Streaming
    debug_assert_eq!(s, TurnState::Streaming);

    let provider = provider.unwrap_or_else(|| "deepseek".to_string());
    let model = model.unwrap_or_else(|| default_model(&provider));

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
            let body = aug.kept.iter().map(|i| i.content.as_str()).collect::<Vec<_>>().join("\n");
            Some(format!(
                "[补强信息] 以下来自记忆库/知识库，仅作背景补强，不覆盖当前文档与当前轮请求：\n{body}"
            ))
        }
    };

    // REF @mention（P1 显式引用，优先保留不静默丢弃，BR-AG-DATA-001，PI-AG-09E/09F）；限 workspace。
    const MENTION_CAP: usize = 4000;
    let mention_block: Option<String> = {
        let mut parts = Vec::new();
        // 整篇引用：读文档（需 ws_root，X-FORBID-004）。
        if let Some(root) = ws_root.as_deref() {
            for rel in &mentions {
                if let Ok(abs) = workspace::resolve_in_workspace(std::path::Path::new(root), rel) {
                    if let Ok(read) = workspace::save::read_doc(&abs) {
                        let capped = crate::ag::context::cap_text(
                            &read.content,
                            MENTION_CAP,
                            "[引用文档较长，已截取前部；需要完整内容可用 read_doc]",
                        );
                        parts.push(format!("引用文档「{rel}」：\n{capped}"));
                    }
                }
            }
        }
        // 块级引用（选区精确锚，PI-AG-09F）：text 已由前端选区带入，精确到块。
        for br in &block_refs {
            if br.text.trim().is_empty() {
                continue;
            }
            let capped = crate::ag::context::cap_text(&br.text, MENTION_CAP, "[引用选区较长，已截取前部]");
            parts.push(format!(
                "引用 文档「{}」第 [{}] 块的选中内容：\n{}",
                br.rel_path, br.block_id, capped
            ));
        }
        if parts.is_empty() {
            None
        } else {
            Some(format!(
                "[用户显式引用] 以下是用户本轮明确引用的内容（高优先，请据此回答相关问题）：\n{}",
                parts.join("\n\n")
            ))
        }
    };

    // 文档形态（PI-AG-08F）：editFormat=html 或内容以 < 开头 → 按 docx/html 块；否则 text 段落块。
    let doc_is_html = active_doc_content
        .as_deref()
        .map(|c| crate::ag::blocks::is_html(c, active_doc_format.as_deref()))
        .unwrap_or(false);

    // P0 当前文档块视图（PI-AG-08E/08F/08G）；超长封顶（层保留、内容可缩，BR-AG-DATA-005，PI-AG-09B）。
    const DOC_CTX_CAP: usize = 9000; // 字符（dev；G5 标定）
    let doc_ctx: Option<String> = active_doc_content
        .as_deref()
        .filter(|c| !c.trim().is_empty())
        .map(|content| {
            let path = active_doc_path.as_deref().unwrap_or("(未命名)");
            let blocks = crate::ag::blocks::parse_blocks(content, doc_is_html);
            let mut block_view = String::new();
            for (i, b) in blocks.iter().enumerate() {
                match &b.block_id {
                    Some(id) => block_view.push_str(&format!("Block {i} [id={id}]: {}\n", b.text)),
                    None => block_view.push_str(&format!("Block {i}: {}\n", b.text)),
                }
            }
            let ctx = format!(
                "[当前编辑器打开的文档] 路径：{path}\n以下按块列出该文档当前内容（本轮主事实，回答关于「当前文档/这篇/这段」的问题时直接据此作答）：\n\n{block_view}\n修改某一块用 propose_edit + new_text：有 [id=…] 的块**优先传 block_id**（稳定锚），否则传 block_index（块号）；改写整篇用 path + content。"
            );
            crate::ag::context::cap_text(&ctx, DOC_CTX_CAP, "[文档较长，已截取前部；其余内容可用 read_doc 分块读取]")
        });

    // P1 历史层：总预算扣除 P0（系统/文档/当前输入）后的余额内保留最近若干轮（drop 最旧，PI-AG-09B）。
    const CTX_BUDGET_CHARS: usize = 16000; // 总预算（~4000 tokens，dev；G5 标定）
    let aug_chars = aug_block.as_deref().map(|s| s.chars().count()).unwrap_or(0);
    let mention_chars = mention_block.as_deref().map(|s| s.chars().count()).unwrap_or(0);
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
    let intent_hint = match crate::ag::context::decide_intent_carryover(&prompt, !kept_history.is_empty()) {
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
    const MAX_ROUNDS: usize = 6;
    for _round in 0..MAX_ROUNDS {
        let msgs = serde_json::Value::Array(messages.clone());
        match ent::gateway::stream_round(&turn_id, &provider, &model, &msgs, &tools, &on_event).await? {
            ent::gateway::RoundOutcome::Done => return Ok(()),
            ent::gateway::RoundOutcome::ToolCalls(calls) => {
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
                    // 注册/权限校验（BR-AG-STATE-002）→ 执行（限 workspace，X-FORBID-004）。
                    let (result, success) = match ag::tool_registry::validate_requested_tool(&c.name) {
                        ag::tool::ToolDecision::Reject(reason) => (reason, false),
                        // 写工具 propose_edit：经 DE 生成 pending diff（X-FORBID-006，不直写）→ emit DiffProposed。
                        ag::tool::ToolDecision::Allow if c.name == "propose_edit" => {
                            match ag_propose_edit(&state, &params, active_doc_content.as_deref(), doc_is_html) {
                                Ok((diff_id, rel_path)) => {
                                    let _ = on_event.send(AgTurnEvent::DiffProposed {
                                        turn_id: turn_id.clone(),
                                        diff_id: diff_id.clone(),
                                        rel_path,
                                    });
                                    (format!("已生成修改建议（diff_id={diff_id}），界面已显示待确认卡片，等待用户接受后应用；请勿重复提交。"), true)
                                }
                                Err(e) => (format!("生成修改建议失败：{}", e.message), false),
                            }
                        }
                        // 只读工具：经 WS/KB 执行（限 active workspace）。
                        ag::tool::ToolDecision::Allow => match ws_root.as_deref() {
                            None => ("无活动 workspace，无法执行工具".to_string(), false),
                            Some(root) => {
                                match ag::tool_exec::exec_read_only(std::path::Path::new(root), &c.name, &params) {
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
                    // 结果回注模型（SM-AG-02 ResultInjecting）。
                    messages.push(serde_json::json!({ "role": "tool", "tool_call_id": c.id, "content": result }));
                }
                // 继续下一轮：带工具结果再调 GW。
            }
        }
    }

    // 达工具循环上限（BR-AG-STATE-002 LoopAborted）：收尾完成终态。
    let _ = on_event.send(AgTurnEvent::Done {
        turn_id: turn_id.clone(),
        stop_reason: "tool_loop_limit".to_string(),
    });
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // workspace.db 落应用数据目录（控制态，非用户正文；承 BR-SYS-PERSIST-001 迁移框架）。
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let conn = db::open_and_migrate(&data_dir.join("workspace.db"))?;
            app.manage(AppState { db: Mutex::new(conn), registry: build_registry() });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![dispatch, ag_start_turn, byok_upload])
        .run(tauri::generate_context!())
        .expect("error while running Binder");
}
