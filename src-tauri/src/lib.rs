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

use ipc::{CommandRegistry, CommandRequest, CommandResponse, IpcError};
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
    reg.register("ws.active_workspace");
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
fn dispatch(request: CommandRequest, state: State<AppState>) -> CommandResponse {
    let correlation_id = request.correlation_id.clone();
    if let Err(err) = state.registry.validate(&request) {
        return CommandResponse { correlation_id, result: None, error: Some(err) };
    }
    let routed = match request.command.as_str() {
        "ws.open_workspace" => route_open_workspace(&request, &state),
        "ws.list_workspaces" => route_list_workspaces(&state),
        "ws.switch_workspace" => route_switch_workspace(&request, &state),
        "ws.active_workspace" => route_active_workspace(&state),
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

/// 路由 `ws.active_workspace`：回当前活动 workspace（启动恢复入口；无则 null）。
fn route_active_workspace(state: &AppState) -> Result<serde_json::Value, IpcError> {
    let conn = lock_db(state)?;
    let active = workspace::lifecycle::active_workspace(&conn)
        .map_err(|e| IpcError::new("WS_ACTIVE_FAILED", &e.to_string(), true, false))?;
    ws_json(&active)
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
        .invoke_handler(tauri::generate_handler![dispatch])
        .run(tauri::generate_context!())
        .expect("error while running Binder");
}
