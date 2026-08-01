//! Binder 后端骨架（Tauri 2）。
//! 模块边界承接 A-SYS-C-T-01 §4 分层与各模块 Plan Item 的文件范围；
//! 各模块为 G3 脚手架占位，@GOV 块在 G4 实现时随实际逻辑落入对应模块。
//! 分层约束 X-SEQ-002：前端经 command 触达，后端经领域逻辑触达 FS/db。

#![recursion_limit = "512"]

use std::{collections::HashMap, sync::Mutex};
use tauri::{Emitter, Manager, State};

/// macOS 的 Cmd+Q 不会在当前 Tao 运行时版本中转换为 Tauri `RunEvent::ExitRequested`：
/// Tao 直接在 `applicationWillTerminate:` 才结束 event loop，届时已无法取消。这里在 Tauri
/// 已安装的 app delegate 外再包一层无状态子类，为 `applicationShouldTerminate:` 提供可取消
/// 的原生入口，并复用前端的 `sys:exit-requested` 关闭编排。
///
/// 不添加 ivar、只覆写一个 ABI 兼容的方法；因此 `object_setClass` 满足 Objective-C 运行时
/// 的对象布局要求。非 macOS 平台不会编译此模块。
#[cfg(target_os = "macos")]
mod macos_exit_guard {
    use std::{
        ffi::{c_char, c_void},
        sync::OnceLock,
    };
    use tauri::{AppHandle, Emitter, Manager};

    type ObjcId = *mut c_void;

    static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

    #[link(name = "objc")]
    unsafe extern "C" {
        fn objc_getClass(name: *const c_char) -> ObjcId;
        fn object_getClass(object: ObjcId) -> ObjcId;
        fn objc_allocateClassPair(
            superclass: ObjcId,
            name: *const c_char,
            extra_bytes: usize,
        ) -> ObjcId;
        fn objc_registerClassPair(class: ObjcId);
        fn class_addMethod(
            class: ObjcId,
            selector: ObjcId,
            implementation: unsafe extern "C" fn(ObjcId, ObjcId, ObjcId) -> usize,
            types: *const c_char,
        ) -> bool;
        fn object_setClass(object: ObjcId, class: ObjcId) -> ObjcId;
        #[link_name = "objc_msgSend"]
        fn objc_msg_send_id(receiver: ObjcId, selector: ObjcId) -> ObjcId;
        fn sel_registerName(name: *const c_char) -> ObjcId;
    }

    /// `NSApplicationTerminateReply`: `NSTerminateCancel = 0`, `NSTerminateNow = 1`.
    /// 有主窗口时必须取消原生终止并进入可交互的决策流；窗口已销毁时让 Tauri 正常收束
    /// event loop，避免留下无窗口后台进程。
    unsafe extern "C" fn application_should_terminate(
        _delegate: ObjcId,
        _selector: ObjcId,
        _sender: ObjcId,
    ) -> usize {
        let Some(app_handle) = APP_HANDLE.get() else {
            return 1;
        };
        if app_handle.get_webview_window("main").is_none() {
            return 1;
        }

        match app_handle.emit("sys:exit-requested", ()) {
            Ok(()) => eprintln!(
                "[app-close] macOS Cmd+Q 已在 applicationShouldTerminate 拦截，已向前端广播 sys:exit-requested"
            ),
            Err(error) => eprintln!(
                "[app-close] macOS Cmd+Q 已拦截，但广播 sys:exit-requested 失败: {error}"
            ),
        }
        0
    }

    /// 将 Tao 的 app delegate 运行时替换为零 ivar 的子类，保留其余所有 delegate 方法的
    /// superclass 分派。此操作只在 Tauri setup 后执行一次，此时 delegate 已创建且仍在主线程。
    pub fn install(app_handle: &AppHandle) -> Result<(), String> {
        APP_HANDLE
            .set(app_handle.clone())
            .map_err(|_| "macOS exit guard 重复安装".to_string())?;

        // SAFETY: setup 在 macOS 主线程执行；新类不新增 ivar，并且仅覆写与 AppKit 约定一致的
        // `applicationShouldTerminate:` 签名。Tao 创建的 delegate 在应用生命周期内保持有效。
        unsafe {
            let app_class = objc_getClass(c"NSApplication".as_ptr());
            if app_class.is_null() {
                return Err("无法找到 NSApplication 类".to_string());
            }
            let shared_application =
                objc_msg_send_id(app_class, sel_registerName(c"sharedApplication".as_ptr()));
            if shared_application.is_null() {
                return Err("无法获取 NSApplication 实例".to_string());
            }
            let delegate =
                objc_msg_send_id(shared_application, sel_registerName(c"delegate".as_ptr()));
            if delegate.is_null() {
                return Err("Tauri app delegate 尚未就绪".to_string());
            }
            let superclass = object_getClass(delegate);
            if superclass.is_null() {
                return Err("无法读取 Tauri app delegate 类".to_string());
            }

            let guard_class =
                objc_allocateClassPair(superclass, c"BinderMacExitGuardDelegate".as_ptr(), 0);
            if guard_class.is_null() {
                return Err("无法创建 macOS 退出守卫 delegate 类".to_string());
            }
            if !class_addMethod(
                guard_class,
                sel_registerName(c"applicationShouldTerminate:".as_ptr()),
                application_should_terminate,
                c"Q@:@".as_ptr(),
            ) {
                return Err("无法安装 applicationShouldTerminate: 拦截器".to_string());
            }
            objc_registerClassPair(guard_class);
            let previous_class = object_setClass(delegate, guard_class);
            if previous_class != superclass {
                return Err("Tauri app delegate 在安装退出守卫时发生并发变化".to_string());
            }
        }
        Ok(())
    }
}

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
    reg.register("ws.discard_unsaved_sessions");
    reg.register("sys.inspect_close");
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
    reg.register("doc.restore_edit_media");
    reg.register("doc.import_project_image");
    reg.register("doc.get_style_report");
    reg.register("doc.save_from_edit");
    reg.register("ws.list_timeline");
    reg.register("ws.preview_node");
    reg.register("ws.restore_node");
    reg.register("de.generate_diff");
    reg.register("de.accept_draft_diff");
    reg.register("de.accept_diff");
    reg.register("de.reject_diff");
    reg.register("de.expire_diff");
    reg.register("de.list_diffs");
    reg.register("de.update_image_diff_snapshots");
    reg.register("ed.persist_session_snapshot");
    reg.register("ed.list_session_snapshots");
    reg.register("ed.persist_open_tabs");
    reg.register("ed.list_open_tabs");
    reg.register("ag.start_turn");
    reg.register("ag.list_blocks");
    reg.register("ag.list_sessions");
    reg.register("ag.save_session");
    reg.register("ag.save_turn");
    reg.register("ag.list_turns");
    reg.register("ag.delete_session");
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
        "ws.discard_unsaved_sessions" => route_discard_unsaved_sessions(&state),
        "sys.inspect_close" => route_inspect_close(&state),
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
        "doc.restore_edit_media" => route_restore_edit_media(&request, &state, &app),
        "doc.import_project_image" => route_import_project_image(&request, &state, &app),
        "doc.get_style_report" => route_get_style_report(&request, &state),
        "doc.save_from_edit" => route_save_from_edit(&request, &state, &app),
        "ws.list_timeline" => route_list_timeline(&request, &state),
        "ws.preview_node" => route_preview_node(&request, &state),
        "ws.restore_node" => route_restore_node(&request, &state, &app),
        "de.generate_diff" => route_generate_diff(&request, &state),
        "de.accept_draft_diff" => route_accept_draft_diff(&request, &state, &app),
        "de.accept_diff" => route_accept_diff(&request, &state, &app),
        "de.reject_diff" => route_reject_diff(&request, &state, &app),
        "de.expire_diff" => route_expire_diff(&request, &state, &app),
        "de.list_diffs" => route_list_diffs(&request, &state),
        "de.update_image_diff_snapshots" => route_update_image_diff_snapshots(&request, &state),
        "ed.persist_session_snapshot" => route_persist_session_snapshot(&request, &state),
        "ed.list_session_snapshots" => route_list_session_snapshots(&state),
        "ed.persist_open_tabs" => route_persist_open_tabs(&request, &state),
        "ed.list_open_tabs" => route_list_open_tabs(&state),
        "ag.start_turn" => route_start_turn(&request),
        "ag.list_blocks" => route_list_blocks(&request, &state),
        "ag.list_sessions" => route_list_agent_sessions(&state),
        "ag.save_session" => route_save_agent_session(&request, &state),
        "ag.save_turn" => route_save_agent_turn(&request, &state),
        "ag.list_turns" => route_list_agent_turns(&request, &state),
        "ag.delete_session" => route_delete_agent_session(&request, &state),
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
    let (ws, abs) = resolve_active(state, &rel_path)?;
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
    let conn = lock_db(state)?;
    de::lifecycle::flip_awaiting_save_to_saved(&conn, &ws.workspace_id, &rel_path).map_err(
        |e| {
            IpcError::new(
                "DE_EFFECT_STATUS_SYNC_FAILED",
                &format!("文件已保存，但 diff 效果状态同步失败: {e}"),
                true,
                false,
            )
        },
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
    let (ws, from_abs) = resolve_active(state, &rel_path)?;
    let (_, to_abs) = resolve_active(state, &new_rel_path)?;
    workspace::fileops::rename_doc(&from_abs, &to_abs)?;
    // 审计修复（2026-07-24）：rename 后同步更新 bdocx_manifest
    let _ = workspace::listing::update_bdocx_for_rename(
        std::path::Path::new(&ws.root_path),
        &rel_path,
        &new_rel_path,
    );
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
/// 授权失败必须阻断打开/恢复：不可显示的图片不能被伪装成完整可编辑 LogicalState。
fn authorize_media_files(app: &tauri::AppHandle, dir: &std::path::Path) -> Result<(), IpcError> {
    use tauri::Manager;
    let entries = std::fs::read_dir(dir).map_err(|error| {
        IpcError::new(
            "DOC_IMAGE_MEDIA_AUTHORIZE_FAILED",
            &format!("读取 BDocx 编辑媒体目录失败: {error}"),
            true,
            false,
        )
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            IpcError::new(
                "DOC_IMAGE_MEDIA_AUTHORIZE_FAILED",
                &format!("枚举 BDocx 编辑媒体失败: {error}"),
                true,
                false,
            )
        })?;
        let p = entry.path();
        if p.is_dir() {
            authorize_media_files(app, &p)?;
        } else if p.is_file() {
            app.asset_protocol_scope().allow_file(&p).map_err(|error| {
                IpcError::new(
                    "DOC_IMAGE_MEDIA_AUTHORIZE_FAILED",
                    &format!("授权 BDocx 编辑媒体失败: {error}"),
                    true,
                    false,
                )
            })?;
        }
    }
    Ok(())
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
    let work_dir = doc::image_media::edit_media_dir(baseline);
    match doc::edit_chain::docx_to_edit_html(&abs, &work_dir) {
        Ok(edit_model) => {
            if let Err(error) = authorize_media_files(app, &work_dir) {
                let st = transition(opening, DocConvertEvent::ConvertFail)
                    .expect("OpenConverting+ConvertFail→OpenError");
                emit_doc01(app, &rel_path, st);
                return Err(error);
            }
            let st = transition(opening, DocConvertEvent::ConvertOk)
                .expect("OpenConverting+ConvertOk→Ready");
            emit_doc01(app, &rel_path, st);
            // 图文呈现（BR-DOC-DATA-002 嵌入图片必需子集）：extract-media 把图片解到 work_dir（按 baseline
            // 隔离），HTML 内为相对 `media/…` 路径。逐文件授权入 asset 协议 scope（最小暴露，同 prv.media_src，
            // X-FORBID-004 守界由后端把关）；回 media_base 供前端将相对 <img src> 改写为 asset URL（只读呈现）。
            Ok(serde_json::json!({
                "format": "html",
                "html": edit_model.html,
                "baseline": baseline.to_string(),
                "media_base": work_dir.to_string_lossy(),
                "style_report": edit_model.style_report,
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

/// 恢复持久化 BDocx LogicalState 的显示媒体：前端只回稳定 HTML 相对引用和原加载 baseline，
/// 后端据此授权同一媒体目录。缓存缺失时，只有磁盘仍与该 baseline 相同才允许重新抽取；磁盘已变化
/// 则保留保存冲突，不用新文档媒体替换旧草稿媒体。
fn route_restore_edit_media(
    request: &CommandRequest,
    state: &AppState,
    app: &tauri::AppHandle,
) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let html = request
        .params
        .get("html")
        .and_then(|value| value.as_str())
        .ok_or_else(|| IpcError::new("DOC_HTML_REQUIRED", "缺少 html 参数", false, false))?;
    let requested_baseline = optional_u64_param(request, "baseline", "DOC_BASELINE_INVALID")?;
    let (ws, abs) = resolve_active(state, &rel_path)?;
    if !is_docx_rel_path(&rel_path)
        || !workspace::listing::is_bdocx(std::path::Path::new(&ws.root_path), &rel_path)
    {
        return Err(IpcError::new(
            "DOC_NOT_BDOCX",
            "只有 BDocx 编辑会话可以恢复编辑媒体",
            false,
            false,
        ));
    }
    let current = std::fs::read(&abs).map_err(|error| {
        IpcError::new(
            "DOC_READ_FAILED",
            &format!("读 BDocx 失败: {error}"),
            true,
            false,
        )
    })?;
    let current_baseline = workspace::save::content_hash(&current);
    let baseline = requested_baseline.unwrap_or(current_baseline);
    let work_dir = doc::image_media::edit_media_dir(baseline);
    doc::image_media::materialize_project_images_for_html(
        std::path::Path::new(&ws.root_path),
        baseline,
        html,
    )?;
    let initial = doc::image_media::inspect_html_resources(html, Some(&work_dir))?;
    if !initial.missing_resources.is_empty() {
        if current_baseline != baseline {
            return Err(IpcError::new(
                "DOC_IMAGE_MEDIA_RESTORE_CONFLICT",
                &format!(
                    "旧编辑媒体已不存在，且磁盘 BDocx 已偏离会话基线；缺失资源：{}",
                    initial.missing_resources.join("、")
                ),
                false,
                false,
            ));
        }
        // 只借同基线源文档重建媒体目录；持久化草稿 HTML 仍是 LogicalState，不被转换结果替换。
        let _ = doc::edit_chain::docx_to_edit_html(&abs, &work_dir)?;
        // docx 重抽取可能重建 media 子目录；项目来源图片按持久化 path/hash 再物化一次。
        doc::image_media::materialize_project_images_for_html(
            std::path::Path::new(&ws.root_path),
            baseline,
            html,
        )?;
        let rebuilt = doc::image_media::inspect_html_resources(html, Some(&work_dir))?;
        if !rebuilt.missing_resources.is_empty() {
            return Err(IpcError::new(
                "DOC_IMAGE_RESOURCE_MISSING",
                &format!(
                    "重建同基线编辑媒体后仍缺失资源：{}",
                    rebuilt.missing_resources.join("、")
                ),
                true,
                false,
            ));
        }
    } else {
        std::fs::create_dir_all(&work_dir).map_err(|error| {
            IpcError::new(
                "DOC_IMAGE_MEDIA_RESTORE_FAILED",
                &format!("创建编辑媒体目录失败: {error}"),
                true,
                false,
            )
        })?;
    }
    authorize_media_files(app, &work_dir)?;
    Ok(serde_json::json!({
        "baseline": baseline.to_string(),
        "media_base": work_dir.to_string_lossy(),
    }))
}

fn route_import_project_image(
    request: &CommandRequest,
    state: &AppState,
    app: &tauri::AppHandle,
) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let baseline =
        optional_u64_param(request, "baseline", "DOC_BASELINE_INVALID")?.ok_or_else(|| {
            IpcError::new(
                "DOC_BASELINE_REQUIRED",
                "导入 BDocx 图片必须携带当前编辑会话 baseline",
                false,
                false,
            )
        })?;
    let (workspace, _) = resolve_active(state, &rel_path)?;
    let workspace_root = std::path::Path::new(&workspace.root_path);
    if !is_docx_rel_path(&rel_path) || !workspace::listing::is_bdocx(workspace_root, &rel_path) {
        return Err(IpcError::new(
            "DOC_NOT_BDOCX",
            "只有 BDocx 编辑会话可以受控导入本地图片",
            false,
            false,
        ));
    }
    let source_path = request
        .params
        .get("sourcePath")
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty());
    let source_data_url = request
        .params
        .get("sourceDataUrl")
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty());
    let imported = match (source_path, source_data_url) {
        (Some(source_path), None) => {
            let source = std::path::Path::new(source_path);
            let resolved_source = if source.is_absolute() {
                source.to_path_buf()
            } else {
                if !source
                    .components()
                    .all(|component| matches!(component, std::path::Component::Normal(_)))
                {
                    return Err(IpcError::new(
                        "DOC_PROJECT_IMAGE_IMPORT_SOURCE_INVALID",
                        "项目文件树图片必须使用规范 workspace 相对路径",
                        false,
                        false,
                    ));
                }
                workspace_root.join(source)
            };
            doc::image_media::import_project_image(workspace_root, &resolved_source)?
        }
        (None, Some(source_data_url)) => {
            let source_name = request
                .params
                .get("sourceName")
                .and_then(|value| value.as_str())
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| {
                    IpcError::new(
                        "DOC_PROJECT_IMAGE_IMPORT_SOURCE_REQUIRED",
                        "剪贴板图片缺少文件名",
                        false,
                        false,
                    )
                })?;
            doc::image_media::import_project_image_data_url(
                workspace_root,
                source_name,
                source_data_url,
            )?
        }
        _ => {
            return Err(IpcError::new(
                "DOC_PROJECT_IMAGE_IMPORT_SOURCE_REQUIRED",
                "必须且只能提供 sourcePath 或 sourceDataUrl",
                false,
                false,
            ))
        }
    };
    let materialized = doc::image_media::materialize_project_image(
        workspace_root,
        &imported.rel_path,
        &imported.content_hash,
        baseline,
    )?;
    let media_base = doc::image_media::edit_media_dir(baseline);
    authorize_media_files(app, &media_base)?;
    Ok(serde_json::json!({
        "assetId": imported.asset_id,
        "sourcePath": imported.rel_path,
        "sourceHash": imported.content_hash,
        "mediaType": imported.media_type,
        "byteSize": imported.byte_size,
        "mediaRef": materialized.media_ref,
        "mediaBase": media_base.to_string_lossy(),
        "baseline": baseline.to_string(),
    }))
}

/// 路由样式报告：复用与打开链完全相同的 DOCX→HTML 颜色恢复与实际检测，不返回正文或授权媒体。
fn route_get_style_report(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let rel_path = str_param(request, "relPath", "WS_REL_PATH_REQUIRED")?;
    let (ws, abs) = resolve_active(state, &rel_path)?;
    if is_docx_rel_path(&rel_path)
        && !workspace::listing::is_bdocx(std::path::Path::new(&ws.root_path), &rel_path)
    {
        return Err(IpcError::new(
            "DOC_NOT_BDOCX",
            "原生 docx 不能进入可编辑样式报告，请先创建 b-docx",
            false,
            false,
        ));
    }
    let source = std::fs::read(&abs).map_err(|e| {
        IpcError::new(
            "DOC_READ_FAILED",
            &format!("读 docx 失败: {e}"),
            true,
            false,
        )
    })?;
    let work_dir = std::env::temp_dir()
        .join("binder_style_report")
        .join(format!("{:x}", workspace::save::content_hash(&source)));
    let edit_model = doc::edit_chain::docx_to_edit_html(&abs, &work_dir)?;
    Ok(serde_json::json!({ "style_report": edit_model.style_report }))
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
    let baseline =
        optional_u64_param(request, "baseline", "DOC_BASELINE_INVALID")?.ok_or_else(|| {
            IpcError::new(
                "DOC_BASELINE_REQUIRED",
                "保存 BDocx 必须携带打开或持久恢复时的加载基线",
                false,
                false,
            )
        })?;
    let override_confirmed = bool_param(request, "override", false);
    let (ws, abs) = resolve_active(state, &rel_path)?;
    let saving = doc::convert::transition(
        doc::convert::DocConvertState::Ready,
        doc::convert::DocConvertEvent::SaveDocx,
    )
    .expect("SM-DOC-01: Ready+SaveDocx→SaveConverting");
    emit_doc01(app, &rel_path, saving);
    let media_dir = doc::image_media::edit_media_dir(baseline);
    if let Err(error) = doc::image_media::materialize_project_images_for_html(
        std::path::Path::new(&ws.root_path),
        baseline,
        html,
    ) {
        let st = doc::convert::transition(saving, doc::convert::DocConvertEvent::SaveConvertFail)
            .expect("SaveConverting+SaveConvertFail→SaveError");
        emit_doc01(app, &rel_path, st);
        return Err(error);
    }
    let docx_bytes = match doc::edit_chain::html_to_docx(html, Some(&media_dir)) {
        Ok(b) => b,
        Err(e) => {
            let st =
                doc::convert::transition(saving, doc::convert::DocConvertEvent::SaveConvertFail)
                    .expect("SaveConverting+SaveConvertFail→SaveError");
            emit_doc01(app, &rel_path, st);
            return Err(e);
        }
    };
    let next_baseline = workspace::save::content_hash(&docx_bytes);
    if let Err(error) = doc::image_media::carry_html_media_forward(html, baseline, next_baseline) {
        let st = doc::convert::transition(saving, doc::convert::DocConvertEvent::SaveConvertFail)
            .expect("SaveConverting+SaveConvertFail→SaveError");
        emit_doc01(app, &rel_path, st);
        return Err(error);
    }
    let next_media_dir = doc::image_media::edit_media_dir(next_baseline);
    if let Err(error) = std::fs::create_dir_all(&next_media_dir)
        .map_err(|cause| {
            IpcError::new(
                "DOC_IMAGE_MEDIA_CARRY_FAILED",
                &format!("创建新 baseline 编辑媒体目录失败: {cause}"),
                true,
                false,
            )
        })
        .and_then(|_| authorize_media_files(app, &next_media_dir))
    {
        let st = doc::convert::transition(saving, doc::convert::DocConvertEvent::SaveConvertFail)
            .expect("SaveConverting+SaveConvertFail→SaveError");
        emit_doc01(app, &rel_path, st);
        return Err(error);
    }
    match save_atomic_sm(
        app,
        &rel_path,
        &abs,
        &docx_bytes,
        Some(baseline),
        override_confirmed,
    ) {
        Ok(saved) => {
            let st = doc::convert::transition(saving, doc::convert::DocConvertEvent::SaveConvertOk)
                .expect("SaveConverting+SaveConvertOk→Ready");
            emit_doc01(app, &rel_path, st);
            // 与文本保存路径相同：文件成功转换并写入后，效果态同步失败必须向调用方暴露，不能
            // 静默把历史留在 awaiting_save。
            let conn = lock_db(state)?;
            de::lifecycle::flip_awaiting_save_to_saved(&conn, &ws.workspace_id, &rel_path)
                .map_err(|e| {
                    IpcError::new(
                        "DE_EFFECT_STATUS_SYNC_FAILED",
                        &format!("文件已保存，但 diff 效果状态同步失败: {e}"),
                        true,
                        false,
                    )
                })?;
            Ok(serde_json::json!({
                "baseline": saved.baseline.to_string(),
                "media_base": next_media_dir.to_string_lossy(),
            }))
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
fn route_inspect_close(state: &AppState) -> Result<serde_json::Value, IpcError> {
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
fn route_discard_unsaved_sessions(state: &AppState) -> Result<serde_json::Value, IpcError> {
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
/// `terminal_reason` 可空（BR-SYS-DATA-005）：过期原因/异常退出清算标注，供历史只读展示区分。
fn discard_diff_sm(
    app: &tauri::AppHandle,
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

#[cfg(test)]
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

fn capability_error_to_ipc(error: ag::bdocx_capabilities::CapabilityValidationError) -> IpcError {
    IpcError::new(error.code, &error.message, error.retriable, false)
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
    BlockOps,
    ReplaceDocument,
    StyleOps,
    TableOps,
    ImageOps,
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
    snapshot_source: String,
    base_session_version: Option<u64>,
    base_content_hash: Option<String>,
    style_ops: Option<serde_json::Value>,
    block_ops: Option<serde_json::Value>,
    table_ops: Option<serde_json::Value>,
    image_ops: Option<serde_json::Value>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct AgentProposeEditResult {
    diff_id: String,
    rel_path: String,
    projection: AgentDiffProjection,
}

fn is_active_editor_snapshot_source(source: &str) -> bool {
    matches!(source, "active_editor" | "active_editor_session")
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
    let has_style_payload = params.get("style_ops").is_some() || params.get("styleOps").is_some();
    let has_block_selector =
        params.get("block_id").is_some() || params.get("block_index").is_some();
    let has_new_text_payload = params.get("new_text").is_some();
    let has_new_text = params.get("new_text").and_then(|v| v.as_str()).is_some();
    let has_content_payload = params.get("content").is_some();
    let has_content = params.get("content").and_then(|v| v.as_str()).is_some();
    let has_table_payload = params.get("base").is_some()
        || params.get("target").is_some()
        || params.get("operations").is_some();
    let has_table_target = params.get("target").is_some();
    let has_table_ops = params
        .get("operations")
        .and_then(|v| v.as_array())
        .map(|v| !v.is_empty())
        .unwrap_or(false);
    let has_structured_base = params.get("base").is_some();

    let op = match operation {
        Some("replace_block") => AgentEditOperation::ReplaceBlock,
        Some("block_ops") => AgentEditOperation::BlockOps,
        Some("replace_document") => AgentEditOperation::ReplaceDocument,
        Some("style_ops") => AgentEditOperation::StyleOps,
        Some("table_ops") => AgentEditOperation::TableOps,
        Some("image_ops") => AgentEditOperation::ImageOps,
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
                "已有文本文件按块修改使用 replace_block；富文档必须使用 block_ops；只有用户明确要求整篇重写时才可使用 replace_document。",
                true,
                false,
            ));
        }
        None => {
            return Err(IpcError::new(
                "AG_TOOL_ARG",
                "propose_edit 缺 operation/content/new_text/style_ops/operations 参数",
                false,
                false,
            ));
        }
    };

    let mixed_payload = match op {
        AgentEditOperation::ReplaceBlock => {
            has_content_payload || has_style_payload || has_table_payload
        }
        AgentEditOperation::BlockOps => {
            has_block_selector
                || has_new_text_payload
                || has_content_payload
                || has_style_payload
                || has_table_target
        }
        AgentEditOperation::ReplaceDocument => {
            has_block_selector || has_new_text_payload || has_style_payload || has_table_payload
        }
        AgentEditOperation::StyleOps => {
            has_block_selector || has_new_text_payload || has_content_payload || has_table_payload
        }
        AgentEditOperation::TableOps => {
            has_block_selector || has_new_text_payload || has_content_payload || has_style_payload
        }
        AgentEditOperation::ImageOps => {
            has_block_selector
                || has_new_text_payload
                || has_content_payload
                || has_style_payload
                || has_table_target
        }
    };
    if mixed_payload {
        return Err(IpcError::new(
            "AG_TOOL_ARG",
            "propose_edit 不允许混合不同 operation 的载荷字段",
            false,
            false,
        ));
    }

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
        AgentEditOperation::BlockOps if !has_structured_base || !has_table_ops => {
            Err(IpcError::new(
                "AG_TOOL_ARG",
                "operation=block_ops 必须提供 base 与非空 operations 数组",
                false,
                false,
            ))
        }
        AgentEditOperation::StyleOps if !has_style_ops => Err(IpcError::new(
            "AG_TOOL_ARG",
            "operation=style_ops 必须提供 style_ops",
            false,
            false,
        )),
        AgentEditOperation::TableOps if !has_table_ops => Err(IpcError::new(
            "AG_TOOL_ARG",
            "operation=table_ops 必须提供非空 operations 数组",
            false,
            false,
        )),
        AgentEditOperation::ImageOps if !has_structured_base || !has_table_ops => {
            Err(IpcError::new(
                "AG_TOOL_ARG",
                "operation=image_ops 必须提供 base 与非空 operations 数组",
                false,
                false,
            ))
        }
        _ => Ok(op),
    }
}

fn html_image_tags(html: &str) -> Vec<String> {
    let lower = html.to_ascii_lowercase();
    let mut tags = Vec::new();
    let mut cursor = 0usize;
    while let Some(relative) = lower[cursor..].find("<img") {
        let start = cursor + relative;
        let name_end = start + 4;
        if lower
            .as_bytes()
            .get(name_end)
            .is_some_and(|byte| !byte.is_ascii_whitespace() && *byte != b'>' && *byte != b'/')
        {
            cursor = name_end;
            continue;
        }
        let Some(end_relative) = lower[start..].find('>') else {
            break;
        };
        let end = start + end_relative + 1;
        // 标签名匹配大小写不敏感，但防绕过比较必须保留属性和值的原始字节；
        // 否则 case-only 的 src/hash/alt 变化会被误认为“图片未变”。
        tags.push(html[start..end].to_string());
        cursor = end;
    }
    tags
}

fn html_tag_attribute(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let needle = format!("{}=", name.to_ascii_lowercase());
    let mut cursor = 0usize;
    while let Some(relative) = lower[cursor..].find(&needle) {
        let start = cursor + relative;
        if start > 0 && !lower.as_bytes()[start - 1].is_ascii_whitespace() {
            cursor = start + needle.len();
            continue;
        }
        let quote_pos = start + needle.len();
        let quote = *tag.as_bytes().get(quote_pos)?;
        if quote != b'\'' && quote != b'"' {
            cursor = quote_pos + 1;
            continue;
        }
        let value_start = quote_pos + 1;
        let value_end = value_start + tag[value_start..].find(quote as char)?;
        return Some(tag[value_start..value_end].to_string());
    }
    None
}

fn top_level_html_block_ids(html: &str) -> std::collections::BTreeSet<String> {
    let void_tags = [
        "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param",
        "source", "track", "wbr",
    ];
    let mut ids = std::collections::BTreeSet::new();
    let mut depth = 0usize;
    let mut cursor = 0usize;
    while let Some(relative) = html[cursor..].find('<') {
        let start = cursor + relative;
        let Some(end_relative) = html[start..].find('>') else {
            break;
        };
        let end = start + end_relative + 1;
        let tag = &html[start..end];
        let inner = tag[1..tag.len() - 1].trim();
        if inner.starts_with('!') || inner.starts_with('?') {
            cursor = end;
            continue;
        }
        if inner.starts_with('/') {
            depth = depth.saturating_sub(1);
            cursor = end;
            continue;
        }
        let name = inner
            .trim_start_matches('/')
            .split(|character: char| character.is_ascii_whitespace() || character == '/')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        if depth == 0 {
            if let Some(block_id) = html_tag_attribute(tag, "data-block-id") {
                ids.insert(block_id);
            }
        }
        if !inner.ends_with('/') && !void_tags.contains(&name.as_str()) {
            depth = depth.saturating_add(1);
        }
        cursor = end;
    }
    ids
}

fn guard_bdocx_raw_image_edit(
    rel_path: &str,
    before: &str,
    after: &str,
    replace_document: bool,
) -> Result<(), IpcError> {
    if !is_docx_rel_path(rel_path) {
        return Ok(());
    }
    let before_images = html_image_tags(before);
    let after_images = html_image_tags(after);
    if (replace_document && (!before_images.is_empty() || !after_images.is_empty()))
        || (!replace_document && before_images != after_images)
    {
        return Err(IpcError::new(
            "AG_IMAGE_OPERATION_REQUIRED",
            if replace_document {
                "含图片的 BDocx 不允许通过 replace_document 原始 HTML 重写；图片增删必须使用 image_ops。"
            } else {
                "BDocx 块替换不得改变图片节点；请先调用 list_images，再使用 image_ops。"
            },
            false,
            false,
        ));
    }
    Ok(())
}

/**
 * @GOV
 * codes:    BR-SYS-DATA-006-GUARD-AG-AG-TOOL-004, BR-ED-DATA-001-GUARD-AG-AG-TOOL-002, BR-ED-DATA-003-GUARD-AG-AG-TOOL-002
 * type:     GUARD
 * chain:    AG-TOOL
 * rules:    BR-SYS-DATA-006, BR-ED-DATA-001, BR-ED-DATA-003
 * term_ref: TERM-SYS-002, TERM-ED-008
 * boundary: in=UntrustedImageOperationAndActiveBDocxSnapshot | out=ValidatedMaterializedImageOperationPayloadOrPreDiffRejection
 */
fn image_ops_payload(
    params: &serde_json::Value,
    snapshot: &ag::blocks::AgentEditableSnapshot,
    images: &[serde_json::Value],
    workspace_root: &std::path::Path,
) -> Result<serde_json::Value, IpcError> {
    let base = params
        .get("base")
        .and_then(|value| value.as_object())
        .ok_or_else(|| {
            IpcError::new(
                "AG_TOOL_ARG",
                "image_ops 缺 base（list_images 的 sessionVersion/contentHash）",
                false,
                false,
            )
        })?;
    let base_version = base
        .get("sessionVersion")
        .or_else(|| base.get("session_version"))
        .and_then(|value| value.as_u64());
    let base_hash = base
        .get("contentHash")
        .or_else(|| base.get("content_hash"))
        .and_then(|value| value.as_str());
    if base_version != snapshot.session_version || base_hash != snapshot.content_hash.as_deref() {
        return Err(IpcError::new(
            "AG_IMAGE_STALE_SNAPSHOT",
            "图片快照已过期；请重新调用 list_images 后再提出修改建议。",
            true,
            false,
        ));
    }
    let baseline = snapshot.baseline.ok_or_else(|| {
        IpcError::new(
            "AG_IMAGE_BASELINE_REQUIRED",
            "当前 BDocx 图片操作缺少媒体 baseline；请重新打开文档后重试。",
            true,
            false,
        )
    })?;
    let operations = params
        .get("operations")
        .and_then(|value| value.as_array())
        .filter(|operations| !operations.is_empty())
        .ok_or_else(|| IpcError::new("AG_TOOL_ARG", "image_ops 缺非空 operations", false, false))?;
    ag::bdocx_capabilities::validate_image_operation_shapes(operations)
        .map_err(capability_error_to_ipc)?;

    let top_level_block_ids = top_level_html_block_ids(&snapshot.content);
    let catalog = workspace::listing::load_project_image_catalog(workspace_root)?;
    let catalog_candidates = catalog
        .candidates
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    let mut image_targets =
        std::collections::BTreeMap::<String, std::collections::BTreeSet<String>>::new();
    let mut insert_anchors = std::collections::BTreeSet::new();
    let mut removed_block_anchors = std::collections::BTreeSet::new();
    for operation in operations {
        match operation.get("op").and_then(|value| value.as_str()) {
            Some("insert_image") => {
                let anchor_block_id = operation["anchorBlockId"].as_str().unwrap_or_default();
                if !top_level_block_ids.contains(anchor_block_id) {
                    return Err(IpcError::new(
                        "AG_IMAGE_ANCHOR_LOST",
                        &format!(
                            "未找到 anchorBlockId={anchor_block_id} 的当前 Block；请重新调用 list_blocks/list_images。"
                        ),
                        true,
                        false,
                    ));
                }
                insert_anchors.insert(anchor_block_id.to_string());
                let source_path = operation["sourcePath"].as_str().unwrap_or_default();
                let source_hash = operation["sourceHash"].as_str().unwrap_or_default();
                let asset_id = operation["assetId"].as_str().unwrap_or_default();
                if !catalog_candidates.contains(source_path) {
                    return Err(IpcError::new(
                        "AG_IMAGE_SOURCE_UNAVAILABLE",
                        &format!(
                            "项目图片不在当前配置 imageRoots 中：{source_path}；请重新调用 list_images。"
                        ),
                        true,
                        false,
                    ));
                }
                let asset = doc::image_media::inspect_project_image(workspace_root, source_path)?;
                if asset.content_hash != source_hash || asset.asset_id != asset_id {
                    return Err(IpcError::new(
                        "AG_IMAGE_SOURCE_CHANGED",
                        &format!(
                            "项目图片身份或内容已变化：{source_path}；请重新调用 list_images 获取最新 assetId/sourceHash。"
                        ),
                        true,
                        false,
                    ));
                }
            }
            Some(op @ ("delete_image" | "resize_image" | "move_image" | "set_image_layout")) => {
                let image_id = operation["imageId"].as_str().unwrap_or_default();
                let fingerprint = operation["fingerprint"].as_str().unwrap_or_default();
                let planned = image_targets.entry(image_id.to_string()).or_default();
                if planned.contains(op) {
                    return Err(IpcError::new(
                        "AG_IMAGE_OPERATION_INVALID",
                        &format!("同一图片 {image_id} 在单批中不能重复执行 {op}"),
                        false,
                        false,
                    ));
                }
                if planned.contains("delete_image") || (op == "delete_image" && !planned.is_empty())
                {
                    return Err(IpcError::new(
                        "AG_IMAGE_OPERATION_INVALID",
                        &format!(
                            "同一图片 {image_id} 的 delete_image 必须独占，不能与其他 ImageOperation 组合"
                        ),
                        false,
                        false,
                    ));
                }
                planned.insert(op.to_string());
                let matched = images.iter().find(|image| {
                    image.get("imageId").and_then(|value| value.as_str()) == Some(image_id)
                        && image.get("fingerprint").and_then(|value| value.as_str())
                            == Some(fingerprint)
                });
                let Some(matched) = matched else {
                    return Err(IpcError::new(
                        "AG_IMAGE_ANCHOR_LOST",
                        &format!(
                            "未找到 imageId={image_id} 且 fingerprint 匹配的当前图片；请重新调用 list_images。"
                        ),
                        true,
                        false,
                    ));
                };
                let capability_available = matched
                    .get("capabilities")
                    .and_then(|value| value.as_array())
                    .is_some_and(|capabilities| {
                        capabilities.iter().any(|value| value.as_str() == Some(op))
                    });
                if !capability_available {
                    return Err(IpcError::new(
                        "AG_IMAGE_OPERATION_UNAVAILABLE",
                        &format!("当前图片 {image_id} 不允许操作 {op}；请重新调用 list_images。"),
                        true,
                        false,
                    ));
                }
                if op == "resize_image" {
                    let width = operation["width"].as_u64().unwrap_or_default();
                    let bounds = matched
                        .get("resizeBounds")
                        .and_then(|value| value.as_object());
                    let min = bounds
                        .and_then(|value| value.get("min"))
                        .and_then(|value| value.as_u64())
                        .unwrap_or(1);
                    let max = bounds
                        .and_then(|value| value.get("max"))
                        .and_then(|value| value.as_u64())
                        .unwrap_or(0);
                    if width < min || width > max {
                        return Err(IpcError::new(
                            "AG_IMAGE_OPERATION_UNAVAILABLE",
                            &format!("图片宽度 {width} 超出当前 resizeBounds {min}..={max}"),
                            false,
                            false,
                        ));
                    }
                }
                if op == "move_image" {
                    let direction = operation["direction"].as_str().unwrap_or_default();
                    let allowed = matched
                        .get("moveDirections")
                        .and_then(|value| value.as_array())
                        .is_some_and(|directions| {
                            directions
                                .iter()
                                .any(|value| value.as_str() == Some(direction))
                        });
                    if !allowed {
                        return Err(IpcError::new(
                            "AG_IMAGE_OPERATION_UNAVAILABLE",
                            &format!("当前图片 {image_id} 不能向 {direction} 移动"),
                            true,
                            false,
                        ));
                    }
                }
                if op == "set_image_layout"
                    && matched.get("standalone").and_then(|value| value.as_bool()) != Some(true)
                {
                    return Err(IpcError::new(
                        "AG_IMAGE_OPERATION_UNAVAILABLE",
                        "只有独立顶层图片段落可设置 left/center/right 对齐",
                        false,
                        false,
                    ));
                }
                if op == "delete_image"
                    && top_level_block_ids.len() > 1
                    && matched.get("standalone").and_then(|value| value.as_bool()) == Some(true)
                {
                    if let Some(block_id) = matched.get("blockId").and_then(|value| value.as_str())
                    {
                        removed_block_anchors.insert(block_id.to_string());
                    }
                }
            }
            _ => unreachable!("static ImageOperation validation rejects unknown operations"),
        }
    }
    if let Some(anchor) = insert_anchors.intersection(&removed_block_anchors).next() {
        return Err(IpcError::new(
            "AG_IMAGE_OPERATION_INVALID",
            &format!(
                "同一批 ImageOperation 不能一边以 Block {anchor} 为插入锚，一边删除会移除该 Block 的独立图片。"
            ),
            false,
            false,
        ));
    }

    let mut materialized_operations = Vec::with_capacity(operations.len());
    for operation in operations {
        let mut materialized = operation.clone();
        if operation.get("op").and_then(|value| value.as_str()) == Some("insert_image") {
            let source_path = operation["sourcePath"].as_str().unwrap_or_default();
            let source_hash = operation["sourceHash"].as_str().unwrap_or_default();
            let image = doc::image_media::materialize_project_image(
                workspace_root,
                source_path,
                source_hash,
                baseline,
            )?;
            materialized
                .as_object_mut()
                .expect("validated image operation object")
                .insert(
                    "mediaRef".to_string(),
                    serde_json::Value::String(image.media_ref),
                );
        }
        materialized_operations.push(materialized);
    }
    Ok(serde_json::json!({
        "base": {
            "sessionVersion": base_version,
            "contentHash": base_hash,
        },
        "operations": materialized_operations,
    }))
}

fn table_ops_payload(
    params: &serde_json::Value,
    snapshot: &ag::blocks::AgentEditableSnapshot,
    tables: &[serde_json::Value],
) -> Result<serde_json::Value, IpcError> {
    let base = params
        .get("base")
        .and_then(|v| v.as_object())
        .ok_or_else(|| {
            IpcError::new(
                "AG_TOOL_ARG",
                "table_ops 缺 base（list_tables 的 sessionVersion/contentHash）",
                false,
                false,
            )
        })?;
    let base_version = base
        .get("sessionVersion")
        .or_else(|| base.get("session_version"))
        .and_then(|v| v.as_u64());
    let base_hash = base
        .get("contentHash")
        .or_else(|| base.get("content_hash"))
        .and_then(|v| v.as_str());
    if base_version != snapshot.session_version || base_hash != snapshot.content_hash.as_deref() {
        return Err(IpcError::new(
            "AG_TABLE_STALE_SNAPSHOT",
            "表格快照已过期；请重新调用 list_tables 后再提出修改建议。",
            true,
            false,
        ));
    }
    let target = params
        .get("target")
        .and_then(|v| v.as_object())
        .ok_or_else(|| {
            IpcError::new(
                "AG_TOOL_ARG",
                "table_ops 缺 target（tableId/fingerprint）",
                false,
                false,
            )
        })?;
    let table_id = target
        .get("tableId")
        .or_else(|| target.get("table_id"))
        .and_then(|v| v.as_str())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| IpcError::new("AG_TOOL_ARG", "table_ops target 缺 tableId", false, false))?;
    let fingerprint = target
        .get("fingerprint")
        .and_then(|v| v.as_str())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| {
            IpcError::new(
                "AG_TOOL_ARG",
                "table_ops target 缺 fingerprint",
                false,
                false,
            )
        })?;
    let known = tables.iter().find(|table| {
        table.get("tableId").and_then(|v| v.as_str()) == Some(table_id)
            && table.get("fingerprint").and_then(|v| v.as_str()) == Some(fingerprint)
    });
    let Some(table) = known else {
        return Err(IpcError::new(
            "AG_TABLE_ANCHOR_LOST",
            "未找到匹配 tableId/fingerprint 的当前表格；请重新调用 list_tables。",
            true,
            false,
        ));
    };
    let operations = params
        .get("operations")
        .and_then(|v| v.as_array())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| IpcError::new("AG_TOOL_ARG", "table_ops 缺非空 operations", false, false))?;
    ag::bdocx_capabilities::validate_table_operation_shapes(operations)
        .map_err(capability_error_to_ipc)?;
    let rows = table.get("rows").and_then(|v| v.as_u64()).ok_or_else(|| {
        IpcError::new("AG_TABLE_SNAPSHOT_INVALID", "表格快照缺 rows", true, false)
    })?;
    let columns = table
        .get("columns")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| {
            IpcError::new(
                "AG_TABLE_SNAPSHOT_INVALID",
                "表格快照缺 columns",
                true,
                false,
            )
        })?;
    let capabilities = table
        .get("capabilities")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            IpcError::new(
                "AG_TABLE_SNAPSHOT_INVALID",
                "表格快照缺 capabilities",
                true,
                false,
            )
        })?;
    let has_capability = |operation: &str| {
        capabilities
            .iter()
            .any(|value| value.as_str() == Some(operation))
    };
    let coordinate = |operation: &serde_json::Value, field: &str| -> Result<(u64, u64), IpcError> {
        let object = operation
            .get(field)
            .and_then(|v| v.as_object())
            .ok_or_else(|| {
                IpcError::new(
                    "AG_TABLE_OPERATION_INVALID",
                    &format!("{field} 必须是 {{row,column}} 坐标"),
                    false,
                    false,
                )
            })?;
        let row = object.get("row").and_then(|v| v.as_u64()).ok_or_else(|| {
            IpcError::new(
                "AG_TABLE_OPERATION_INVALID",
                &format!("{field}.row 必须是非负整数"),
                false,
                false,
            )
        })?;
        let column = object
            .get("column")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| {
                IpcError::new(
                    "AG_TABLE_OPERATION_INVALID",
                    &format!("{field}.column 必须是非负整数"),
                    false,
                    false,
                )
            })?;
        if row >= rows || column >= columns {
            return Err(IpcError::new(
                "AG_TABLE_OPERATION_INVALID",
                &format!("{field} 超出当前表格范围"),
                false,
                false,
            ));
        }
        Ok((row, column))
    };
    for operation in operations {
        let name = operation.get("op").and_then(|v| v.as_str()).unwrap_or("");
        if operation.get("html").is_some() || operation.get("content").is_some() {
            return Err(IpcError::new(
                "AG_TABLE_OPERATION_INVALID",
                "table_ops 只允许受限 TableOperation，禁止 HTML/content。",
                false,
                false,
            ));
        }
        let require_capability = |capability: &str| -> Result<(), IpcError> {
            if has_capability(capability) {
                Ok(())
            } else {
                Err(IpcError::new(
                    "AG_TABLE_OPERATION_UNAVAILABLE",
                    &format!("当前表格不可执行 {capability}"),
                    false,
                    false,
                ))
            }
        };
        match name {
            "insert_row" => {
                require_capability(name)?;
                if operation
                    .get("at")
                    .and_then(|v| v.as_u64())
                    .filter(|at| *at <= rows)
                    .is_none()
                {
                    return Err(IpcError::new(
                        "AG_TABLE_OPERATION_INVALID",
                        "insert_row.at 必须是 0..=rows 的整数",
                        false,
                        false,
                    ));
                }
            }
            "delete_row" => {
                require_capability(name)?;
                if operation
                    .get("row")
                    .and_then(|v| v.as_u64())
                    .filter(|row| *row < rows)
                    .is_none()
                {
                    return Err(IpcError::new(
                        "AG_TABLE_OPERATION_INVALID",
                        "delete_row.row 必须是表内行号",
                        false,
                        false,
                    ));
                }
            }
            "insert_column" => {
                require_capability(name)?;
                if operation
                    .get("at")
                    .and_then(|v| v.as_u64())
                    .filter(|at| *at <= columns)
                    .is_none()
                {
                    return Err(IpcError::new(
                        "AG_TABLE_OPERATION_INVALID",
                        "insert_column.at 必须是 0..=columns 的整数",
                        false,
                        false,
                    ));
                }
            }
            "delete_column" => {
                require_capability(name)?;
                if operation
                    .get("column")
                    .and_then(|v| v.as_u64())
                    .filter(|column| *column < columns)
                    .is_none()
                {
                    return Err(IpcError::new(
                        "AG_TABLE_OPERATION_INVALID",
                        "delete_column.column 必须是表内列号",
                        false,
                        false,
                    ));
                }
            }
            "set_header_row" => {
                require_capability(name)?;
                if operation.get("enabled").and_then(|v| v.as_bool()).is_none() {
                    return Err(IpcError::new(
                        "AG_TABLE_OPERATION_INVALID",
                        "set_header_row.enabled 必须是布尔值",
                        false,
                        false,
                    ));
                }
            }
            "merge_cells" => {
                require_capability(name)?;
                let from = coordinate(operation, "from")?;
                let to = coordinate(operation, "to")?;
                if from == to {
                    return Err(IpcError::new(
                        "AG_TABLE_OPERATION_INVALID",
                        "merge_cells 必须覆盖至少两个单元格",
                        false,
                        false,
                    ));
                }
            }
            "split_cell" => {
                require_capability(name)?;
                coordinate(operation, "at")?;
            }
            "set_cell_text" => {
                require_capability(name)?;
                let (row, column) = coordinate(operation, "at")?;
                if operation.get("text").and_then(|v| v.as_str()).is_none() {
                    return Err(IpcError::new(
                        "AG_TABLE_OPERATION_INVALID",
                        "set_cell_text.text 必须是字符串",
                        false,
                        false,
                    ));
                }
                let editable = table
                    .get("cells")
                    .and_then(|v| v.as_array())
                    .map(|cells| {
                        cells.iter().any(|cell| {
                            let start_row =
                                cell.get("row").and_then(|v| v.as_u64()).unwrap_or(u64::MAX);
                            let start_column = cell
                                .get("column")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(u64::MAX);
                            let rowspan = cell.get("rowspan").and_then(|v| v.as_u64()).unwrap_or(1);
                            let colspan = cell.get("colspan").and_then(|v| v.as_u64()).unwrap_or(1);
                            row >= start_row
                                && row < start_row.saturating_add(rowspan)
                                && column >= start_column
                                && column < start_column.saturating_add(colspan)
                                && cell.get("contentKind").and_then(|v| v.as_str())
                                    == Some("plain_text")
                        })
                    })
                    .unwrap_or(false);
                if !editable {
                    return Err(IpcError::new(
                        "AG_TABLE_RICH_CELL_UNSUPPORTED",
                        "代理只能改写单一纯文本段落单元格；富内容单元格请保留人工编辑。",
                        false,
                        false,
                    ));
                }
            }
            "set_cell_background" => {
                require_capability(name)?;
                let (row, column) = coordinate(operation, "at")?;
                let anchored = table
                    .get("cells")
                    .and_then(|v| v.as_array())
                    .map(|cells| {
                        cells.iter().any(|cell| {
                            cell.get("row").and_then(|v| v.as_u64()) == Some(row)
                                && cell.get("column").and_then(|v| v.as_u64()) == Some(column)
                        })
                    })
                    .unwrap_or(false);
                if !anchored {
                    return Err(IpcError::new(
                        "AG_TABLE_OPERATION_INVALID",
                        "set_cell_background.at 必须是单元格左上锚；合并单元格内部坐标不允许使用",
                        false,
                        false,
                    ));
                }
                if let Some(color) = operation.get("color").filter(|value| !value.is_null()) {
                    let valid = color
                        .as_str()
                        .map(|value| {
                            value.len() == 7
                                && value.starts_with('#')
                                && value[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
                        })
                        .unwrap_or(false);
                    if !valid {
                        return Err(IpcError::new(
                            "AG_TABLE_OPERATION_INVALID",
                            "set_cell_background.color 只能是 #RRGGBB 或 null",
                            false,
                            false,
                        ));
                    }
                } else if operation.get("color").is_none() {
                    return Err(IpcError::new(
                        "AG_TABLE_OPERATION_INVALID",
                        "set_cell_background 必须显式提供 color（#RRGGBB 或 null）",
                        false,
                        false,
                    ));
                }
            }
            "move_table" => {
                require_capability(name)?;
                let direction = operation.get("direction").and_then(|v| v.as_str());
                if !matches!(direction, Some("up" | "down")) {
                    return Err(IpcError::new(
                        "AG_TABLE_OPERATION_INVALID",
                        "move_table.direction 只能是 up 或 down",
                        false,
                        false,
                    ));
                }
                let available = table
                    .get("moveDirections")
                    .and_then(|v| v.as_array())
                    .map(|directions| directions.iter().any(|value| value.as_str() == direction))
                    .unwrap_or(false);
                if !available {
                    return Err(IpcError::new(
                        "AG_TABLE_OPERATION_UNAVAILABLE",
                        "表格在当前文档流位置不能向该方向移动",
                        false,
                        false,
                    ));
                }
            }
            _ => {
                return Err(IpcError::new(
                    "AG_TABLE_OPERATION_INVALID",
                    "table_ops 包含不支持的表格操作",
                    false,
                    false,
                ))
            }
        }
    }
    Ok(
        serde_json::json!({ "tableId": table_id, "fingerprint": fingerprint, "operations": operations }),
    )
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
fn route_expire_diff(
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

fn route_persist_session_snapshot(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let ws = active_ws(state)?;
    let snapshot_value = request.params.get("snapshot").cloned().ok_or_else(|| {
        IpcError::new(
            "ED_SESSION_SNAPSHOT_REQUIRED",
            "缺少 snapshot 参数",
            false,
            false,
        )
    })?;
    let snapshot_baseline = match snapshot_value.get("baseline") {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::Number(value)) => Some(value.as_u64().ok_or_else(|| {
            IpcError::new(
                "ED_SESSION_SNAPSHOT_INVALID",
                "BDocx snapshot baseline 必须是 u64",
                false,
                false,
            )
        })?),
        Some(serde_json::Value::String(value)) => Some(value.parse::<u64>().map_err(|_| {
            IpcError::new(
                "ED_SESSION_SNAPSHOT_INVALID",
                "BDocx snapshot baseline 必须是 u64 字符串",
                false,
                false,
            )
        })?),
        Some(_) => {
            return Err(IpcError::new(
                "ED_SESSION_SNAPSHOT_INVALID",
                "BDocx snapshot baseline 类型无效",
                false,
                false,
            ))
        }
    };
    let snapshot: ag::blocks::OpenDocSnapshot =
        serde_json::from_value(snapshot_value).map_err(|e| {
            IpcError::new(
                "ED_SESSION_SNAPSHOT_INVALID",
                &format!("OpenDocSnapshot 解析失败: {e}"),
                false,
                false,
            )
        })?;
    if snapshot.content_hash.trim().is_empty() {
        return Err(IpcError::new(
            "ED_SESSION_SNAPSHOT_INVALID",
            "OpenDocSnapshot 缺少 content_hash",
            false,
            false,
        ));
    }
    workspace::resolve_in_workspace(std::path::Path::new(&ws.root_path), &snapshot.rel_path)?;
    let session_version = i64::try_from(snapshot.session_version).map_err(|_| {
        IpcError::new(
            "ED_SESSION_SNAPSHOT_INVALID",
            "session_version 超出持久化范围",
            false,
            false,
        )
    })?;
    let persisted_content = if snapshot.edit_format == "html" {
        doc::image_media::encode_snapshot_content(&snapshot.content, snapshot_baseline)
    } else {
        snapshot.content.clone()
    };
    let conn = lock_db(state)?;
    conn.execute(
        "INSERT OR REPLACE INTO ed_session_snapshots \
         (workspace_id, rel_path, content, edit_format, content_format, dirty, session_version, content_hash, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            ws.workspace_id,
            snapshot.rel_path,
            persisted_content,
            snapshot.edit_format,
            snapshot.content_format,
            if snapshot.dirty { 1 } else { 0 },
            session_version,
            snapshot.content_hash,
            now_millis(),
        ],
    )
    .map_err(|e| IpcError::new("ED_SESSION_DB_ERROR", &e.to_string(), true, false))?;
    Ok(serde_json::json!({ "ok": true }))
}

fn route_list_session_snapshots(state: &AppState) -> Result<serde_json::Value, IpcError> {
    let ws = active_ws(state)?;
    let conn = lock_db(state)?;
    let mut stmt = conn
        .prepare(
            "SELECT rel_path, content, edit_format, content_format, dirty, session_version, content_hash, updated_at \
             FROM ed_session_snapshots WHERE workspace_id = ?1 ORDER BY updated_at ASC",
        )
        .map_err(|e| IpcError::new("ED_SESSION_DB_ERROR", &e.to_string(), true, false))?;
    let rows = stmt
        .query_map([ws.workspace_id], |r| {
            let stored_content: String = r.get(1)?;
            let (content, baseline) = doc::image_media::decode_snapshot_content(&stored_content);
            let dirty: i64 = r.get(4)?;
            let session_version: i64 = r.get(5)?;
            let updated_at: i64 = r.get(7)?;
            Ok(serde_json::json!({
                "relPath": r.get::<_, String>(0)?,
                "content": content,
                "baseline": baseline.map(|value| value.to_string()),
                "editFormat": r.get::<_, String>(2)?,
                "contentFormat": r.get::<_, String>(3)?,
                "source": "open_editor_session",
                "dirty": dirty != 0,
                "sessionVersion": session_version.max(0),
                "contentHash": r.get::<_, String>(6)?,
                "capturedAt": updated_at,
            }))
        })
        .map_err(|e| IpcError::new("ED_SESSION_DB_ERROR", &e.to_string(), true, false))?;
    let snapshots = rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| IpcError::new("ED_SESSION_DB_ERROR", &e.to_string(), true, false))?;
    Ok(serde_json::json!({ "snapshots": snapshots }))
}

fn route_persist_open_tabs(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let ws = active_ws(state)?;
    let tabs = request
        .params
        .get("tabs")
        .and_then(|v| v.as_array())
        .ok_or_else(|| IpcError::new("ED_OPEN_TABS_REQUIRED", "tabs 必须是数组", false, false))?;
    let mut rows: Vec<(String, String, i64, bool)> = Vec::with_capacity(tabs.len());
    let mut active_count = 0usize;
    for (idx, tab) in tabs.iter().enumerate() {
        let rel_path = tab
            .get("relPath")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| {
                IpcError::new("ED_OPEN_TAB_INVALID", "打开标签缺少 relPath", false, false)
            })?
            .to_string();
        workspace::resolve_in_workspace(std::path::Path::new(&ws.root_path), &rel_path)?;
        let kind = tab
            .get("kind")
            .and_then(|v| v.as_str())
            .unwrap_or("doc")
            .to_string();
        if kind != "doc" {
            return Err(IpcError::new(
                "ED_OPEN_TAB_INVALID",
                "当前仅支持持久化 doc 类型编辑标签",
                false,
                false,
            ));
        }
        let open_index = tab
            .get("openIndex")
            .and_then(|v| v.as_i64())
            .unwrap_or(idx as i64);
        if open_index < 0 {
            return Err(IpcError::new(
                "ED_OPEN_TAB_INVALID",
                "openIndex 不能为负数",
                false,
                false,
            ));
        }
        let is_active = tab
            .get("isActive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if is_active {
            active_count += 1;
        }
        rows.push((rel_path, kind, open_index, is_active));
    }
    if rows.len() > 1 && active_count != 1 {
        return Err(IpcError::new(
            "ED_OPEN_TABS_ACTIVE_INVALID",
            "多个打开文档必须且只能有一个当前文档",
            false,
            false,
        ));
    }
    if rows.len() == 1 && active_count == 0 {
        rows[0].3 = true;
    }

    let mut conn = lock_db(state)?;
    let tx = conn
        .transaction()
        .map_err(|e| IpcError::new("ED_OPEN_TABS_DB_ERROR", &e.to_string(), true, false))?;
    tx.execute(
        "DELETE FROM ed_open_tabs WHERE workspace_id = ?1",
        [ws.workspace_id.as_str()],
    )
    .map_err(|e| IpcError::new("ED_OPEN_TABS_DB_ERROR", &e.to_string(), true, false))?;
    let updated_at = now_millis();
    for (rel_path, kind, open_index, is_active) in rows {
        tx.execute(
            "INSERT INTO ed_open_tabs \
             (workspace_id, rel_path, kind, open_index, is_active, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                ws.workspace_id.as_str(),
                rel_path,
                kind,
                open_index,
                if is_active { 1 } else { 0 },
                updated_at,
            ],
        )
        .map_err(|e| IpcError::new("ED_OPEN_TABS_DB_ERROR", &e.to_string(), true, false))?;
    }
    tx.commit()
        .map_err(|e| IpcError::new("ED_OPEN_TABS_DB_ERROR", &e.to_string(), true, false))?;
    Ok(serde_json::json!({ "ok": true }))
}

fn route_list_open_tabs(state: &AppState) -> Result<serde_json::Value, IpcError> {
    let ws = active_ws(state)?;
    let conn = lock_db(state)?;
    let mut stmt = conn
        .prepare(
            "SELECT rel_path, kind, open_index, is_active, updated_at \
             FROM ed_open_tabs WHERE workspace_id = ?1 ORDER BY open_index ASC",
        )
        .map_err(|e| IpcError::new("ED_OPEN_TABS_DB_ERROR", &e.to_string(), true, false))?;
    let rows = stmt
        .query_map([ws.workspace_id], |r| {
            let is_active: i64 = r.get(3)?;
            Ok(serde_json::json!({
                "relPath": r.get::<_, String>(0)?,
                "kind": r.get::<_, String>(1)?,
                "openIndex": r.get::<_, i64>(2)?,
                "isActive": is_active != 0,
                "updatedAt": r.get::<_, i64>(4)?,
            }))
        })
        .map_err(|e| IpcError::new("ED_OPEN_TABS_DB_ERROR", &e.to_string(), true, false))?;
    let tabs = rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| IpcError::new("ED_OPEN_TABS_DB_ERROR", &e.to_string(), true, false))?;
    Ok(serde_json::json!({ "tabs": tabs }))
}

/// 路由 `de.update_image_diff_snapshots`：把 ED 实际应用得到的图片前后快照补入既有
/// ImageOperation DiffPayload。载荷只能首次写入或同值幂等重试，不改变文档内容或 Diff 终态。
fn route_update_image_diff_snapshots(
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
fn route_list_diffs(
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
fn route_list_diff_details(
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
            let before_snapshot: Option<String> = r.get(7)?;
            let after_content: Option<Vec<u8>> = r.get(8)?;
            let style_ops_json: Option<String> = r.get(9)?;
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
                    "patch_kind": if block_ops.is_some() {
                        "block_ops"
                    } else if table_ops.is_some() {
                        "table_ops"
                    } else if image_ops.is_some() {
                        "image_ops"
                    } else {
                        "replace_document"
                    },
                    "block_id": block_id,
                    "block_index": null,
                    "before_text": block_before_text.unwrap_or_else(|| before_snapshot.unwrap_or_default()),
                    "after_text": block_after_text.unwrap_or(after_text),
                    "location_label": "历史修改建议",
                    "draft_mode": "background",
                    "snapshot_source": "disk_snapshot",
                    "base_session_version": null,
                    "base_content_hash": null,
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
            db: Mutex::new(conn),
            registry: build_registry(),
            assist_terminals: Mutex::new(HashMap::new()),
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

fn route_list_agent_sessions(state: &AppState) -> Result<serde_json::Value, IpcError> {
    let ws = active_ws(state)?;
    let conn = lock_db(state)?;
    let sessions = ag::history::list_sessions(&conn, &ws.workspace_id)
        .map_err(|e| IpcError::new("AG_HISTORY_DB_ERROR", &e.to_string(), true, false))?;
    ws_json(&sessions)
}

fn route_save_agent_session(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let ws = active_ws(state)?;
    let input = ag::history::AgentSessionInput {
        session_id: str_param(request, "sessionId", "AG_SESSION_ID_REQUIRED")?,
        title: request
            .params
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("新对话")
            .to_string(),
        mode: request
            .params
            .get("mode")
            .and_then(|v| v.as_str())
            .unwrap_or("edit")
            .to_string(),
        archived: request
            .params
            .get("archived")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        open_index: request
            .params
            .get("openIndex")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        is_active: request
            .params
            .get("isActive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    };
    let conn = lock_db(state)?;
    ag::history::upsert_session(&conn, &ws.workspace_id, &input)
        .map_err(|e| IpcError::new("AG_HISTORY_DB_ERROR", &e.to_string(), true, false))?;
    Ok(serde_json::json!({ "ok": true }))
}

fn route_save_agent_turn(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let ws = active_ws(state)?;
    let session_id = str_param(request, "sessionId", "AG_SESSION_ID_REQUIRED")?;
    let turn_id = str_param(request, "turnId", "AG_TURN_ID_REQUIRED")?;
    let status = str_param(request, "status", "AG_TURN_STATUS_REQUIRED")?;
    if !ag::history::is_valid_status(&status) {
        return Err(IpcError::new(
            "AG_TURN_STATUS_INVALID",
            "未知代理回合历史状态",
            false,
            false,
        ));
    }
    let payload_value = request.params.get("payload").cloned().ok_or_else(|| {
        IpcError::new(
            "AG_TURN_PAYLOAD_REQUIRED",
            "缺少 payload 参数",
            false,
            false,
        )
    })?;
    let payload = serde_json::from_value::<ag::history::AgentTurnPayload>(payload_value)
        .map_err(|e| IpcError::new("AG_TURN_PAYLOAD_INVALID", &e.to_string(), false, false))?;
    let rel_path = request.params.get("relPath").and_then(|v| v.as_str());
    let conn = lock_db(state)?;
    ag::history::save_turn(
        &conn,
        &ws.workspace_id,
        &session_id,
        &turn_id,
        rel_path,
        &status,
        &payload,
    )
    .map_err(|e| IpcError::new("AG_HISTORY_DB_ERROR", &e.to_string(), true, false))?;
    Ok(serde_json::json!({ "ok": true }))
}

fn route_list_agent_turns(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let ws = active_ws(state)?;
    let session_id = str_param(request, "sessionId", "AG_SESSION_ID_REQUIRED")?;
    let conn = lock_db(state)?;
    let turns = ag::history::list_turns(&conn, &ws.workspace_id, &session_id)
        .map_err(|e| IpcError::new("AG_HISTORY_DB_ERROR", &e.to_string(), true, false))?;
    ws_json(&turns)
}

fn route_delete_agent_session(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let ws = active_ws(state)?;
    let session_id = str_param(request, "sessionId", "AG_SESSION_ID_REQUIRED")?;
    let mut conn = lock_db(state)?;
    let result = ag::history::delete_session(&mut conn, &ws.workspace_id, &session_id)
        .map_err(|e| IpcError::new("AG_HISTORY_DB_ERROR", &e.to_string(), true, false))?;
    ws_json(&result)
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
    let open_doc_snapshots: Vec<ag::blocks::OpenDocSnapshot> = request
        .params
        .get("openDocSnapshots")
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|e| {
            IpcError::new(
                "AG_OPEN_DOC_SNAPSHOT_INVALID",
                &format!("openDocSnapshots 解析失败: {e}"),
                false,
                false,
            )
        })?
        .unwrap_or_default();
    let read_ctx =
        ag::blocks::AgentReadContext::new(std::path::Path::new(&ws.root_path), open_doc_snapshots)?;
    let snapshot = read_ctx.snapshot(&rel_path)?;
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

#[cfg(test)]
fn block_ref_range_label(br: &BlockRef) -> String {
    match (br.start, br.end) {
        (Some(start), Some(end)) if end > start => format!("字符区间 [{start}..{end}]"),
        _ => "字符区间未提供（按该块选中内容理解）".to_string(),
    }
}

#[cfg(test)]
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
 * boundary: in=ReferencePayloadAndAgentReadContext | out=explicitReferenceContextBlock(validatedReadSnapshot)
 */
fn format_reference_payload_context(
    payload: &ReferencePayload,
    read_ctx: Option<&ag::blocks::AgentReadContext>,
    cap: usize,
) -> String {
    match payload {
        ReferencePayload::Doc { rel_path, .. } => match read_ctx {
            Some(ctx) => match ctx.snapshot(rel_path) {
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
            let stale = || {
                let capped = crate::ag::context::cap_text(text, cap, "[引用选区较长，已截取前部]");
                format!(
                    "引用选区「{rel_path}」块 [{block_id}] 锚未校验或已失效，以下仅为用户原始选区快照（不得作为当前文档事实）：\n{capped}"
                )
            };
            let Some(ctx) = read_ctx else {
                return stale();
            };
            let snapshot = match ctx.snapshot(rel_path) {
                Ok(snapshot) => snapshot,
                Err(e) => {
                    let capped =
                        crate::ag::context::cap_text(text, cap, "[引用选区较长，已截取前部]");
                    return format!(
                        "引用选区「{rel_path}」读取失败（{}）：{}。以下仅为用户原始选区快照（不得作为当前文档事实）：\n{capped}",
                        e.code, e.message
                    );
                }
            };
            let Some(block) = snapshot
                .blocks
                .iter()
                .find(|b| b.block_id.as_deref() == Some(block_id.as_str()))
            else {
                return stale();
            };
            let (Some(start), Some(end)) = (*start, *end) else {
                return stale();
            };
            if end <= start {
                return stale();
            }
            let chars: Vec<char> = block.text.chars().collect();
            if end > chars.len() {
                return stale();
            }
            let current: String = chars[start..end].iter().collect();
            let capped = crate::ag::context::cap_text(&current, cap, "[引用选区较长，已截取前部]");
            if current == *text {
                format!(
                    "引用选区（已校验）「{rel_path}」块 [{block_id}] 字符区间 [{start}..{end}]：\n{capped}"
                )
            } else {
                let original =
                    crate::ag::context::cap_text(text, cap, "[原始选区较长，已截取前部]");
                format!(
                    "引用选区「{rel_path}」块 [{block_id}] 字符区间 [{start}..{end}] 已按当前 LogicalState 重新校验，当前内容：\n{capped}\n\n原始选区快照仅作背景（不得作为当前文档事实）：\n{original}"
                )
            }
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

        let ctx = ag::blocks::AgentReadContext::new(&root, Vec::new()).unwrap();
        let out = format_reference_payload_context(&payload, Some(&ctx), 1000);

        assert!(out.contains("引用文档「notes/a.md」"));
        assert!(out.contains("文档引用内容"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn reference_payload_context_reads_doc_from_open_logical_state() {
        let root = std::env::temp_dir().join(format!(
            "binder-ref-open-doc-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(root.join("notes")).unwrap();
        std::fs::write(root.join("notes/a.md"), "磁盘旧内容").unwrap();
        let ctx = ag::blocks::AgentReadContext::new(
            &root,
            vec![ag::blocks::OpenDocSnapshot {
                rel_path: "notes/a.md".to_string(),
                content: "打开后的最新内容".to_string(),
                baseline: None,
                edit_format: "text".to_string(),
                content_format: "text".to_string(),
                source: "open_editor_session".to_string(),
                dirty: true,
                session_version: 2,
                content_hash: "h2".to_string(),
                captured_at: 1,
                block_snapshots: Vec::new(),
                table_snapshots: Vec::new(),
                image_snapshots: Vec::new(),
            }],
        )
        .unwrap();
        let payload = ReferencePayload::Doc {
            rel_path: "notes/a.md".to_string(),
            label: None,
        };

        let out = format_reference_payload_context(&payload, Some(&ctx), 1000);

        assert!(out.contains("打开后的最新内容"));
        assert!(!out.contains("磁盘旧内容"));
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

        assert!(out.contains("锚未校验或已失效"));
        assert!(out.contains("不得作为当前文档事实"));
        assert!(out.ends_with("选区引用内容"));
    }

    #[test]
    fn reference_payload_context_validates_selection_against_open_snapshot() {
        let root = std::env::temp_dir().join(format!(
            "binder-ref-selection-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("a.html"),
            "<p data-block-id=\"b1\">0123456789</p>",
        )
        .unwrap();
        let ctx = ag::blocks::AgentReadContext::new(
            &root,
            vec![ag::blocks::OpenDocSnapshot {
                rel_path: "a.html".to_string(),
                content: "<p data-block-id=\"b1\">0123456789</p>".to_string(),
                baseline: None,
                edit_format: "html".to_string(),
                content_format: "html".to_string(),
                source: "active_editor".to_string(),
                dirty: true,
                session_version: 4,
                content_hash: "h4".to_string(),
                captured_at: 1,
                block_snapshots: vec![ag::blocks::AgentEditableBlockSnapshot {
                    block_id: "b1".to_string(),
                    block_index: 0,
                    block_type: "p".to_string(),
                    node_type: "paragraph".to_string(),
                    role: "content".to_string(),
                    parent_block_id: None,
                    depth: 0,
                    ordinal: 0,
                    text: "0123456789".to_string(),
                    attrs: serde_json::json!({"blockId": "b1"}),
                    marks: Vec::new(),
                    node_json: serde_json::json!({
                        "type": "paragraph",
                        "attrs": {"blockId": "b1"},
                        "content": [{"type": "text", "text": "0123456789"}]
                    }),
                    fingerprint: "fp-b1".to_string(),
                    capabilities: vec!["replace_node".to_string()],
                }],
                table_snapshots: Vec::new(),
                image_snapshots: Vec::new(),
            }],
        )
        .unwrap();
        let payload = ReferencePayload::Selection {
            rel_path: "a.html".to_string(),
            block_id: "b1".to_string(),
            start: Some(2),
            end: Some(6),
            text: "2345".to_string(),
            label: None,
        };

        let out = format_reference_payload_context(&payload, Some(&ctx), 1000);

        assert!(out.contains("引用选区（已校验）"));
        assert!(out.contains("2345"));
        let _ = std::fs::remove_dir_all(root);
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

    fn table_editable_snapshot() -> ag::blocks::AgentEditableSnapshot {
        ag::blocks::AgentEditableSnapshot {
            rel_path: "notes/a.docx".to_string(),
            editable_kind: "bdocx".to_string(),
            source: "active_editor".to_string(),
            content_format: "html".to_string(),
            content: "<table></table>".to_string(),
            baseline: None,
            session_version: Some(7),
            content_hash: Some("h7".to_string()),
            blocks: Vec::new(),
        }
    }

    fn table_snapshot(content_kind: &str) -> serde_json::Value {
        json!({
            "tableId": "table_1",
            "fingerprint": "fp_1",
            "rows": 2,
            "columns": 2,
            "moveDirections": ["up", "down"],
            "capabilities": ["insert_row", "delete_row", "insert_column", "delete_column", "set_header_row", "merge_cells", "split_cell", "set_cell_text", "set_cell_background", "move_table"],
            "cells": [{ "row": 0, "column": 0, "rowspan": 1, "colspan": 1, "backgroundColor": null, "contentKind": content_kind }]
        })
    }

    fn image_editable_snapshot(baseline: u64) -> ag::blocks::AgentEditableSnapshot {
        ag::blocks::AgentEditableSnapshot {
            rel_path: "notes/a.docx".to_string(),
            editable_kind: "bdocx".to_string(),
            source: "active_editor".to_string(),
            content_format: "html".to_string(),
            content: r#"<p data-block-id="b1">正文</p><p data-block-id="img1"><img src="media/image1.png"></p>"#.to_string(),
            baseline: Some(baseline),
            session_version: Some(8),
            content_hash: Some("h8".to_string()),
            blocks: vec![
                ag::blocks::AgentEditableBlock {
                    block_id: Some("b1".to_string()),
                    block_index: 0,
                    block_type: Some("p".to_string()),
                    text: "正文".to_string(),
                    marks: Vec::new(),
                    block_attrs: json!({}),
                    ..Default::default()
                },
                ag::blocks::AgentEditableBlock {
                    block_id: Some("img1".to_string()),
                    block_index: 1,
                    block_type: Some("p".to_string()),
                    text: "\u{fffc}".to_string(),
                    marks: Vec::new(),
                    block_attrs: json!({}),
                    ..Default::default()
                },
            ],
        }
    }

    #[test]
    fn structured_edit_blocked_satisfies_edit_tool_terminal_without_a_diff() {
        assert!(edit_intent_has_terminal_tool_result(true, false));
        assert!(edit_intent_has_terminal_tool_result(false, true));
        assert!(!edit_intent_has_terminal_tool_result(false, false));
    }

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

        let err = resolve_agent_edit_operation(
            &json!({
                "path": "notes/a.docx",
                "operation": "block_ops",
                "operations": [{
                    "op": "replace_node",
                    "blockId": "p-1",
                    "fingerprint": "fp-1",
                    "expectedNodeType": "paragraph",
                    "afterNode": { "type": "paragraph", "attrs": { "blockId": "p-1" } }
                }]
            }),
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

    #[test]
    fn explicit_block_ops_requires_base_and_non_empty_operations() {
        let op = resolve_agent_edit_operation(
            &json!({
                "path": "notes/a.docx",
                "operation": "block_ops",
                "base": { "sessionVersion": 1, "contentHash": "h1" },
                "operations": [{
                    "op": "replace_node",
                    "blockId": "p-1",
                    "fingerprint": "fp-1",
                    "expectedNodeType": "paragraph",
                    "afterNode": { "type": "paragraph", "attrs": { "blockId": "p-1" } }
                }]
            }),
            true,
        )
        .unwrap();
        assert_eq!(op, AgentEditOperation::BlockOps);

        let err = resolve_agent_edit_operation(
            &json!({
                "path": "notes/a.docx",
                "operation": "block_ops",
                "base": { "sessionVersion": 1, "contentHash": "h1" },
                "operations": []
            }),
            true,
        )
        .unwrap_err();
        assert_eq!(err.code, "AG_TOOL_ARG");
    }

    #[test]
    fn propose_edit_rejects_mixed_operation_payloads_before_execution() {
        let err = resolve_agent_edit_operation(
            &json!({
                "path": "notes/a.docx",
                "operation": "replace_block",
                "block_id": "b1",
                "new_text": "新块",
                "style_ops": [
                    { "op": "set_mark", "blockId": "b1", "start": 0, "end": 1, "mark": { "type": "bold" } }
                ]
            }),
            true,
        )
        .unwrap_err();
        assert_eq!(err.code, "AG_TOOL_ARG");
        assert!(err.message.contains("不允许混合"));

        let err = resolve_agent_edit_operation(
            &json!({
                "path": "notes/a.docx",
                "operation": "replace_block",
                "block_id": "b1",
                "new_text": "新块",
                "base": { "sessionVersion": 1, "contentHash": "h1" }
            }),
            true,
        )
        .unwrap_err();
        assert_eq!(err.code, "AG_TOOL_ARG");
        assert!(err.message.contains("不允许混合"));
    }

    #[test]
    fn image_ops_is_a_distinct_typed_operation_and_html_tag_projection_is_stable() {
        let op = resolve_agent_edit_operation(
            &json!({
                "path": "notes/a.docx",
                "operation": "image_ops",
                "base": { "sessionVersion": 8, "contentHash": "h8" },
                "operations": [{ "op": "delete_image", "imageId": "image_0", "fingerprint": "fp0" }]
            }),
            true,
        )
        .unwrap();
        assert_eq!(op, AgentEditOperation::ImageOps);
        assert_eq!(
            html_image_tags(r#"<p>x</p><IMG src="media/a.png"><p>y</p>"#),
            vec![r#"<IMG src="media/a.png">"#]
        );
        assert_eq!(
            top_level_html_block_ids(
                r#"<p data-block-id="top">正文</p><ul data-block-id="list"><li data-block-id="nested">项</li></ul>"#
            ),
            std::collections::BTreeSet::from(["list".to_string(), "top".to_string()])
        );
        assert!(guard_bdocx_raw_image_edit(
            "notes/a.docx",
            r#"<p><img src="media/a.png"></p><p>旧</p>"#,
            r#"<p><img src="media/a.png"></p><p>新</p>"#,
            false,
        )
        .is_ok());
        assert_eq!(
            guard_bdocx_raw_image_edit(
                "notes/a.docx",
                r#"<p><img src="media/a.png"></p>"#,
                r#"<p><img src="MEDIA/A.PNG"></p>"#,
                false,
            )
            .unwrap_err()
            .code,
            "AG_IMAGE_OPERATION_REQUIRED"
        );
        assert_eq!(
            guard_bdocx_raw_image_edit(
                "notes/a.docx",
                r#"<p><img src="media/a.png"></p>"#,
                "<p>删除图片</p>",
                false,
            )
            .unwrap_err()
            .code,
            "AG_IMAGE_OPERATION_REQUIRED"
        );
        assert_eq!(
            guard_bdocx_raw_image_edit(
                "notes/a.docx",
                r#"<p><img src="media/a.png"></p>"#,
                r#"<p><img src="media/a.png"></p>"#,
                true,
            )
            .unwrap_err()
            .code,
            "AG_IMAGE_OPERATION_REQUIRED"
        );
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

    #[test]
    fn table_ops_requires_complete_typed_operations_and_plain_text_cells() {
        let snapshot = table_editable_snapshot();
        let base = json!({ "sessionVersion": 7, "contentHash": "h7" });
        let target = json!({ "tableId": "table_1", "fingerprint": "fp_1" });
        let malformed = json!({ "base": base, "target": target, "operations": [{ "op": "merge_cells", "from": { "row": 0, "column": 0 } }] });
        assert_eq!(
            table_ops_payload(&malformed, &snapshot, &[table_snapshot("plain_text")])
                .unwrap_err()
                .code,
            "AG_TABLE_OPERATION_INVALID"
        );

        let rich = json!({
            "base": { "sessionVersion": 7, "contentHash": "h7" },
            "target": { "tableId": "table_1", "fingerprint": "fp_1" },
            "operations": [{ "op": "set_cell_text", "at": { "row": 0, "column": 0 }, "text": "覆盖" }]
        });
        assert_eq!(
            table_ops_payload(&rich, &snapshot, &[table_snapshot("rich_content")])
                .unwrap_err()
                .code,
            "AG_TABLE_RICH_CELL_UNSUPPORTED"
        );

        let valid = json!({
            "base": { "sessionVersion": 7, "contentHash": "h7" },
            "target": { "tableId": "table_1", "fingerprint": "fp_1" },
            "operations": [{ "op": "set_cell_text", "at": { "row": 0, "column": 0 }, "text": "安全" }]
        });
        assert!(table_ops_payload(&valid, &snapshot, &[table_snapshot("plain_text")]).is_ok());

        let rich_background = json!({
            "base": { "sessionVersion": 7, "contentHash": "h7" },
            "target": { "tableId": "table_1", "fingerprint": "fp_1" },
            "operations": [{ "op": "set_cell_background", "at": { "row": 0, "column": 0 }, "color": "#FFE699" }]
        });
        assert!(table_ops_payload(
            &rich_background,
            &snapshot,
            &[table_snapshot("rich_content")]
        )
        .is_ok());

        let invalid_color = json!({
            "base": { "sessionVersion": 7, "contentHash": "h7" },
            "target": { "tableId": "table_1", "fingerprint": "fp_1" },
            "operations": [{ "op": "set_cell_background", "at": { "row": 0, "column": 0 }, "color": "yellow" }]
        });
        assert_eq!(
            table_ops_payload(&invalid_color, &snapshot, &[table_snapshot("plain_text")])
                .unwrap_err()
                .code,
            "AG_TABLE_OPERATION_INVALID"
        );
    }

    // covers: BR-SYS-DATA-006, BR-ED-DATA-001, BR-ED-DATA-003, BR-WS-DATA-001
    #[test]
    fn image_ops_validates_base_anchors_hash_and_materializes_only_after_full_batch_validation() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let baseline = unique as u64;
        let root = std::env::temp_dir().join(format!("binder-agent-image-{}", unique));
        std::fs::create_dir_all(root.join("images")).unwrap();
        let source = root.join("images/new.png");
        let bytes = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 1, 2, 3];
        std::fs::write(&source, bytes).unwrap();
        let asset = doc::image_media::inspect_project_image(&root, "images/new.png").unwrap();
        let snapshot = image_editable_snapshot(baseline);
        let images = vec![json!({
            "imageId": "image_stable",
            "fingerprint": "fp0",
            "blockId": "img1",
            "standalone": true,
            "capabilities": ["delete_image", "resize_image", "move_image", "set_image_layout"],
            "resizeBounds": { "min": 40, "max": 800 },
            "moveDirections": ["up", "down"]
        })];

        let invalid_batch = json!({
            "base": { "sessionVersion": 8, "contentHash": "h8" },
            "operations": [
                {
                    "op": "insert_image",
                    "anchorBlockId": "b1",
                    "position": "after",
                    "assetId": asset.asset_id,
                    "sourcePath": "images/new.png",
                    "sourceHash": asset.content_hash
                },
                { "op": "delete_image", "imageId": "missing", "fingerprint": "missing" }
            ]
        });
        assert_eq!(
            image_ops_payload(&invalid_batch, &snapshot, &images, &root)
                .unwrap_err()
                .code,
            "AG_IMAGE_ANCHOR_LOST"
        );
        assert!(!doc::image_media::edit_media_dir(baseline)
            .join(format!("media/binder-project-{}.png", asset.content_hash))
            .exists());

        let valid = json!({
            "base": { "sessionVersion": 8, "contentHash": "h8" },
            "operations": [
                {
                    "op": "insert_image",
                    "anchorBlockId": "b1",
                    "position": "after",
                    "assetId": asset.asset_id,
                    "sourcePath": "images/new.png",
                    "sourceHash": asset.content_hash,
                    "alt": "项目图",
                    "width": 240
                },
                { "op": "delete_image", "imageId": "image_stable", "fingerprint": "fp0" }
            ]
        });
        let payload = image_ops_payload(&valid, &snapshot, &images, &root).unwrap();
        let media_ref = payload["operations"][0]["mediaRef"].as_str().unwrap();
        assert!(doc::image_media::edit_media_dir(baseline)
            .join(media_ref)
            .is_file());
        assert_eq!(std::fs::read(&source).unwrap(), bytes);

        let stale = json!({
            "base": { "sessionVersion": 7, "contentHash": "h8" },
            "operations": [{ "op": "delete_image", "imageId": "image_stable", "fingerprint": "fp0" }]
        });
        assert_eq!(
            image_ops_payload(&stale, &snapshot, &images, &root)
                .unwrap_err()
                .code,
            "AG_IMAGE_STALE_SNAPSHOT"
        );
        for operation in [
            json!({ "op": "resize_image", "imageId": "image_stable", "fingerprint": "fp0", "width": 320 }),
            json!({ "op": "move_image", "imageId": "image_stable", "fingerprint": "fp0", "direction": "up" }),
            json!({ "op": "set_image_layout", "imageId": "image_stable", "fingerprint": "fp0", "alignment": "right" }),
        ] {
            let payload = json!({
                "base": { "sessionVersion": 8, "contentHash": "h8" },
                "operations": [operation]
            });
            assert!(image_ops_payload(&payload, &snapshot, &images, &root).is_ok());
        }
        let composite = json!({
            "base": { "sessionVersion": 8, "contentHash": "h8" },
            "operations": [
                { "op": "move_image", "imageId": "image_stable", "fingerprint": "fp0", "direction": "up" },
                { "op": "set_image_layout", "imageId": "image_stable", "fingerprint": "fp0", "alignment": "center" },
                { "op": "resize_image", "imageId": "image_stable", "fingerprint": "fp0", "width": 320 }
            ]
        });
        assert!(image_ops_payload(&composite, &snapshot, &images, &root).is_ok());

        for invalid in [
            json!({
                "base": { "sessionVersion": 8, "contentHash": "h8" },
                "operations": [
                    { "op": "resize_image", "imageId": "image_stable", "fingerprint": "fp0", "width": 320 },
                    { "op": "resize_image", "imageId": "image_stable", "fingerprint": "fp0", "width": 360 }
                ]
            }),
            json!({
                "base": { "sessionVersion": 8, "contentHash": "h8" },
                "operations": [
                    { "op": "delete_image", "imageId": "image_stable", "fingerprint": "fp0" },
                    { "op": "set_image_layout", "imageId": "image_stable", "fingerprint": "fp0", "alignment": "center" }
                ]
            }),
        ] {
            assert_eq!(
                image_ops_payload(&invalid, &snapshot, &images, &root)
                    .unwrap_err()
                    .code,
                "AG_IMAGE_OPERATION_INVALID"
            );
        }
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(doc::image_media::edit_media_dir(baseline));
    }
}

#[cfg(test)]
mod ag_turn_prompt_tests {
    use super::*;

    fn hist(rel_path: Option<&str>, role: &str, content: &str) -> ag::context::HistTurn {
        ag::context::HistTurn {
            role: role.to_string(),
            content: content.to_string(),
            rel_path: rel_path.map(str::to_string),
            turn_id: Some(format!("turn-{role}")),
        }
    }

    fn snapshot(path: &str, content: &str) -> ag::blocks::AgentEditableSnapshot {
        ag::blocks::AgentEditableSnapshot {
            rel_path: path.to_string(),
            editable_kind: "text".to_string(),
            source: "active_editor".to_string(),
            content_format: "text".to_string(),
            content: content.to_string(),
            baseline: None,
            session_version: Some(1),
            content_hash: Some("h1".to_string()),
            blocks: content
                .lines()
                .enumerate()
                .map(|(block_index, text)| ag::blocks::AgentEditableBlock {
                    block_id: None,
                    block_index,
                    block_type: None,
                    text: text.to_string(),
                    marks: Vec::new(),
                    block_attrs: serde_json::json!({}),
                    ..Default::default()
                })
                .collect(),
        }
    }

    fn message_text(messages: &[serde_json::Value]) -> String {
        messages
            .iter()
            .filter_map(|m| m.get("content").and_then(|v| v.as_str()))
            .collect::<Vec<_>>()
            .join("\n")
    }

    // covers: BR-AG-DATA-001, BR-AG-DATA-002, BR-AG-DATA-004, BR-AG-DATA-005
    #[test]
    fn current_doc_turn_drops_cross_doc_history_without_explicit_carryover() {
        let doc_ctx = "[当前编辑器打开的文档] 路径：b.md\n农夫与蛇当前正文";
        let history = vec![
            hist(Some("a.md"), "user", "把小马过河改短"),
            hist(Some("a.md"), "assistant", "小马过河改写结果"),
            hist(Some("b.md"), "user", "农夫与蛇上一轮同文档问题"),
        ];

        let assembled = assemble_agent_turn_messages(
            "system",
            None,
            &history,
            Some("b.md"),
            10_000,
            None,
            Some(doc_ctx),
            "整理当前文档",
        );
        let text = message_text(&assembled.messages);

        assert_eq!(assembled.carryover, ag::context::Carryover::None);
        assert!(text.contains("农夫与蛇当前正文"));
        assert!(text.contains("农夫与蛇上一轮同文档问题"));
        assert!(!text.contains("小马过河"));
        assert!(text.contains("已忽略 2 条非当前文档历史"));
    }

    // covers: BR-AG-DATA-004
    #[test]
    fn explicit_carryover_injects_cross_doc_history_only_as_weak_labeled_block() {
        let history = vec![hist(Some("a.md"), "assistant", "小马过河上一轮方案")];

        let assembled = assemble_agent_turn_messages(
            "system",
            None,
            &history,
            Some("b.md"),
            10_000,
            None,
            Some("[当前编辑器打开的文档] 路径：b.md\n农夫与蛇当前正文"),
            "把刚才那段再短一点",
        );
        let text = message_text(&assembled.messages);

        assert_eq!(assembled.carryover, ag::context::Carryover::Refine);
        assert!(text.contains("[其他文档历史]"));
        assert!(text.contains("doc=a.md"));
        assert!(text.contains("小马过河上一轮方案"));
        assert!(text.contains("农夫与蛇当前正文"));
        assert!(text.contains("不得把其中正文当作当前文档事实"));
    }

    // covers: BR-AG-DATA-005
    #[test]
    fn long_current_doc_uses_manifest_and_requires_tool_read_before_edit() {
        let snapshot = snapshot(
            "long.md",
            &format!("第一段\n{}\n最后一段", "正文".repeat(4000)),
        );

        let ctx = build_current_doc_prompt_context("long.md", &snapshot, 900).unwrap();

        let text = ctx.text.expect("long doc context");
        assert!(ctx.requires_read_before_edit);
        assert!(text.contains("当前文档较长"));
        assert!(text.contains("块目录"));
        assert!(text.contains("必须先调用 read_doc 或 list_blocks"));
        assert!(!text.contains("已截取前部"));
    }

    // covers: BR-SYS-DATA-006, BR-AG-DATA-005
    #[test]
    fn bdocx_current_doc_context_injects_manifest_version_hash_and_real_capabilities() {
        let snapshot = ag::blocks::AgentEditableSnapshot {
            rel_path: "notes/a.docx".to_string(),
            editable_kind: "bdocx".to_string(),
            source: "active_editor".to_string(),
            content_format: "html".to_string(),
            content: r#"<p data-block-id="b1">正文</p>"#.to_string(),
            baseline: None,
            session_version: Some(1),
            content_hash: Some("h1".to_string()),
            blocks: vec![ag::blocks::AgentEditableBlock {
                block_id: Some("b1".to_string()),
                block_index: 0,
                block_type: Some("p".to_string()),
                text: "正文".to_string(),
                marks: Vec::new(),
                block_attrs: serde_json::json!({}),
                node_type: Some("paragraph".to_string()),
                role: "content".to_string(),
                node_json: serde_json::json!({
                    "type": "paragraph",
                    "attrs": {"blockId": "b1"},
                    "content": [{"type": "text", "text": "正文"}]
                }),
                fingerprint: Some("fp-b1".to_string()),
                capabilities: vec!["replace_node".to_string()],
                ..Default::default()
            }],
        };

        let ctx = build_current_doc_prompt_context("notes/a.docx", &snapshot, 20_000).unwrap();
        let text = ctx.text.unwrap();
        assert!(text.contains("editable_kind=bdocx"));
        assert!(text.contains("manifestId=binder.bdocx.agent-edit"));
        assert!(text.contains("sha256="));
        assert!(text.contains("subscript"));
        assert!(text.contains("set_cell_background"));
        assert!(text.contains("人工专属"));
        assert!(text.contains("type=p"));
    }

    // covers: BR-AG-DATA-005
    #[test]
    fn long_current_doc_edit_guard_allows_after_read_tool() {
        let params = serde_json::json!({ "path": "long.md" });

        assert!(current_doc_edit_requires_prior_read(
            &params,
            Some("long.md"),
            true,
            false
        ));
        assert!(!current_doc_edit_requires_prior_read(
            &params,
            Some("long.md"),
            true,
            true
        ));
        assert!(is_current_doc_read_tool(
            "read_doc",
            &params,
            Some("long.md")
        ));
        assert!(is_current_doc_read_tool(
            "list_blocks",
            &params,
            Some("long.md")
        ));
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
/// Agent 工具 create_doc：workspace 内新建文档（守界，已存在拒绝）。
/// 不经 DE diff，直接写入文件系统；用于用户明确要求「新建文件」的场景。
fn ag_create_doc(state: &AppState, params: &serde_json::Value) -> Result<String, IpcError> {
    let rel_path = params
        .get("path")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| IpcError::new("AG_TOOL_ARG", "create_doc 缺 path 参数", false, false))?
        .to_string();
    let content = params.get("content").and_then(|v| v.as_str()).unwrap_or("");
    let ws = active_ws(state)?;
    let abs = workspace::resolve_in_workspace(std::path::Path::new(&ws.root_path), &rel_path)?;
    if abs.exists() {
        return Err(IpcError::new(
            "WS_EXISTS",
            &format!("目标文件已存在: {rel_path}"),
            false,
            false,
        ));
    }
    let baseline = if is_docx_rel_path(&rel_path) {
        let html = initial_bdocx_html(content);
        let docx_bytes = doc::edit_chain::html_to_docx(&html, None)?;
        let baseline = workspace::fileops::create_doc(&abs, &docx_bytes)?;
        workspace::listing::mark_bdocx(std::path::Path::new(&ws.root_path), &rel_path)?;
        baseline
    } else {
        workspace::fileops::create_doc(&abs, content.as_bytes())?
    };
    Ok(serde_json::to_string(&serde_json::json!({
        "rel_path": rel_path,
        "baseline": baseline,
        "summary": format!("已创建文档 {rel_path}")
    }))
    .unwrap_or_else(|_| format!("已创建文档 {rel_path}")))
}

/// target 须属回合声明目标集或为新建路径（BR-AG-DATA-003 2026-07-04 版，AG guard + DE 校验双侧把守）。
/// 三模式：块级（replace_block）/ 整篇（replace_document）/ 样式（style_ops）。
/// 返回后端权威 DiffProjection；前端不得再从 tool_request 预测 before/after。
fn ag_propose_edit(
    state: &AppState,
    app: &tauri::AppHandle,
    params: &serde_json::Value,
    read_ctx: &ag::blocks::AgentReadContext,
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
    if target_exists
        && base_is_html
        && matches!(
            edit_operation,
            AgentEditOperation::ReplaceBlock | AgentEditOperation::ReplaceDocument
        )
    {
        return Err(IpcError::new(
            "AG_RICH_BLOCK_OPERATION_REQUIRED",
            "富文档内容修改必须使用 operation=block_ops 和受控 ProseMirror node JSON；不再接受 HTML 字符串块替换或整篇替换。",
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
        authorize_media_files(app, &doc::image_media::edit_media_dir(baseline))?;
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
            let replaced = if let Some(block_id) = selected_block.block_id.as_deref() {
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
            };
            guard_bdocx_raw_image_edit(&rel_path, base_content_for_diff, &replaced, false)?;
            replaced
        }
        AgentEditOperation::BlockOps => {
            let payload = block_ops_json
                .as_ref()
                .expect("block_ops validated before match");
            let operations = payload["operations"]
                .as_array()
                .expect("validated block operations");
            let first_block_id = operations
                .first()
                .and_then(|operation| operation.get("blockId"))
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| {
                    IpcError::new(
                        "AG_BLOCK_OPERATION_INVALID",
                        "结构化块载荷缺少首个 blockId",
                        false,
                        false,
                    )
                })?;
            let current = snapshot
                .as_ref()
                .expect("block_ops requires existing snapshot");
            let first_block = current
                .blocks
                .iter()
                .find(|block| block.block_id.as_deref() == Some(first_block_id))
                .ok_or_else(|| {
                    IpcError::new("AG_ANCHOR_LOST", "结构化块首个锚已失效", true, false)
                })?;
            projection_block_id = Some(first_block_id.to_string());
            projection_block_index = Some(first_block.block_index);
            projection_before = operations
                .iter()
                .map(|operation| {
                    let block_id = operation["blockId"].as_str().unwrap_or("");
                    let text = ag::block_ops::node_text(&operation["beforeNode"]);
                    format!("{block_id}: {text}")
                })
                .collect::<Vec<_>>()
                .join("\n");
            projection_after = operations
                .iter()
                .map(|operation| {
                    let block_id = operation["blockId"].as_str().unwrap_or("");
                    let text = ag::block_ops::node_text(&operation["afterNode"]);
                    format!("{block_id}: {text}")
                })
                .collect::<Vec<_>>()
                .join("\n");
            projection_location = if operations.len() == 1 {
                format!("Block {}", first_block.block_index)
            } else {
                format!("{} 个独立内容块", operations.len())
            };
            base_content_for_diff.to_string()
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
            if target_exists
                && !crate::ag::blocks::rewrite_preserves_document_identity(
                    base_content_for_diff,
                    content,
                )
            {
                return Err(IpcError::new(
                    "AG_REPLACE_DOCUMENT_IDENTITY_DRIFT",
                    "整篇替换内容与原文主题/实体重合过低，疑似错文档或历史上下文污染，已拒绝生成修改建议。请先重新读取当前文档并优先按块修改；若用户明确要求改成全新主题，需要重新确认后再执行。",
                    true,
                    false,
                ));
            }
            guard_bdocx_raw_image_edit(&rel_path, base_content_for_diff, content, true)?;
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
        AgentEditOperation::TableOps => {
            let payload = table_ops_json
                .as_ref()
                .expect("table_ops validated before match");
            projection_block_id = None;
            projection_block_index = None;
            projection_before = format!(
                "表格 {} 当前结构",
                payload["tableId"].as_str().unwrap_or("")
            );
            projection_after = format!(
                "表格结构修改（{} 项）",
                payload["operations"]
                    .as_array()
                    .map(|v| v.len())
                    .unwrap_or(0)
            );
            projection_location = "表格结构".to_string();
            base_content_for_diff.to_string()
        }
        AgentEditOperation::ImageOps => {
            let payload = image_ops_json
                .as_ref()
                .expect("image_ops validated before match");
            let operations = payload["operations"]
                .as_array()
                .expect("validated image operations");
            let inserted = operations
                .iter()
                .filter(|operation| operation["op"] == "insert_image")
                .count();
            let deleted = operations
                .iter()
                .filter(|operation| operation["op"] == "delete_image")
                .count();
            let resized = operations
                .iter()
                .filter(|operation| operation["op"] == "resize_image")
                .count();
            let moved = operations
                .iter()
                .filter(|operation| operation["op"] == "move_image")
                .count();
            let aligned = operations
                .iter()
                .filter(|operation| operation["op"] == "set_image_layout")
                .count();
            projection_block_id = operations
                .iter()
                .find_map(|operation| {
                    operation
                        .get("anchorBlockId")
                        .and_then(|value| value.as_str())
                        .map(str::to_string)
                })
                .or_else(|| {
                    let images = read_ctx.image_snapshots(&rel_path).ok()?;
                    operations.iter().find_map(|operation| {
                        let image_id = operation.get("imageId")?.as_str()?;
                        let fingerprint = operation.get("fingerprint")?.as_str()?;
                        images.iter().find_map(|image| {
                            (image.get("imageId").and_then(|value| value.as_str())
                                == Some(image_id)
                                && image.get("fingerprint").and_then(|value| value.as_str())
                                    == Some(fingerprint))
                            .then(|| {
                                image
                                    .get("blockId")
                                    .and_then(|value| value.as_str())
                                    .map(str::to_string)
                            })
                            .flatten()
                        })
                    })
                });
            projection_block_index = projection_block_id.as_ref().and_then(|block_id| {
                snapshot
                    .as_ref()
                    .and_then(|current| {
                        current
                            .blocks
                            .iter()
                            .find(|block| block.block_id.as_ref() == Some(block_id))
                    })
                    .map(|block| block.block_index)
            });
            projection_before =
                format!("当前图片 {} 张", read_ctx.image_snapshots(&rel_path)?.len());
            projection_after = format!(
                "图片操作：插入 {inserted}，删除 {deleted}，缩放 {resized}，移动 {moved}，对齐 {aligned}"
            );
            projection_location = "图片结构".to_string();
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
    };
    Ok(AgentProposeEditResult {
        diff_id,
        rel_path,
        projection,
    })
}

#[derive(Debug)]
struct CurrentDocPromptContext {
    text: Option<String>,
    requires_read_before_edit: bool,
}

fn block_line_for_prompt(block: &ag::blocks::AgentEditableBlock, full_text: bool) -> String {
    let text = if full_text {
        block.text.clone()
    } else {
        let preview: String = block.text.chars().take(180).collect();
        if block.text.chars().count() > 180 {
            format!("{preview}...")
        } else {
            preview
        }
    };
    let char_count = block.text.encode_utf16().count();
    let block_type = block
        .block_type
        .as_deref()
        .map(|value| format!(" type={value}"))
        .unwrap_or_default();
    let structured = block.node_type.as_deref().map(|node_type| {
        let parent = block.parent_block_id.as_deref().unwrap_or("-");
        let capabilities = if block.capabilities.is_empty() {
            "readonly".to_string()
        } else {
            block.capabilities.join(",")
        };
        let node_json = if full_text && !block.node_json.is_null() {
            format!(" nodeJson={}", block.node_json)
        } else {
            String::new()
        };
        format!(
            " nodeType={node_type} role={} parent={parent} depth={} fingerprint={} capabilities={capabilities}{node_json}",
            block.role,
            block.depth,
            block.fingerprint.as_deref().unwrap_or("-"),
        )
    }).unwrap_or_default();
    match &block.block_id {
        Some(id) => format!(
            "Block {} [id={id}]{block_type}{structured} chars={char_count}: {text}\n",
            block.block_index,
        ),
        None => format!(
            "Block {}{block_type} chars={char_count}: {text}\n",
            block.block_index
        ),
    }
}

fn build_current_doc_prompt_context(
    path: &str,
    snapshot: &ag::blocks::AgentEditableSnapshot,
    inline_cap: usize,
) -> Result<CurrentDocPromptContext, IpcError> {
    let mut runtime_declaration = format!(
        "[当前文档运行时声明] editable_kind={} content_format={} source={}\n",
        snapshot.editable_kind, snapshot.content_format, snapshot.source
    );
    if snapshot.editable_kind == "bdocx" {
        runtime_declaration
            .push_str(&ag::prompts::bdocx_capability_prompt().map_err(capability_error_to_ipc)?);
        runtime_declaration.push('\n');
    }
    if snapshot.content.trim().is_empty() {
        return Ok(CurrentDocPromptContext {
            text: Some(format!(
                "{runtime_declaration}[当前编辑器打开的文档] 路径：{path}\n当前内容为空。"
            )),
            requires_read_before_edit: false,
        });
    }
    let edit_instruction = if snapshot.content_format == "html" {
        "富文档内容块只能使用 propose_edit operation=block_ops + base + operations。只能选择 capabilities 含 replace_node 的 role=content 块；role=container/atom 或 capabilities=readonly 仅作层级语境，不得直接替换。每个 operation 必须把块投影的 block_id、fingerprint、node_type 分别原样带回为 blockId、fingerprint、expectedNodeType，并从 node_json 复制节点，只修改受控 inline content 形成 afterNode；一批相关块必须在一次 block_ops 中提交。不得使用 replace_block/replace_document 或 HTML 字符串改变富文档结构。"
    } else {
        "文本文件修改某一块使用 propose_edit operation=replace_block + new_text：有 [id=…] 的块优先传 block_id，否则传 block_index；只有用户明确要求整篇重写时，才用 operation=replace_document + content。"
    };
    let mut full_block_view = String::new();
    if snapshot.blocks.is_empty() {
        full_block_view.push_str(&format!("完整内容：\n{}\n", snapshot.content));
    } else {
        for block in &snapshot.blocks {
            full_block_view.push_str(&block_line_for_prompt(block, true));
        }
    }
    let full_ctx = format!(
        "{runtime_declaration}[当前编辑器打开的文档] 路径：{path}\n以下按块列出该文档当前内容（本轮主事实，回答关于「当前文档/这篇/这段」的问题时直接据此作答）：\n\n{full_block_view}\n{edit_instruction}"
    );
    if full_ctx.chars().count() <= inline_cap {
        return Ok(CurrentDocPromptContext {
            text: Some(full_ctx),
            requires_read_before_edit: false,
        });
    }

    let mut manifest = String::new();
    if snapshot.blocks.is_empty() {
        let char_count = snapshot.content.encode_utf16().count();
        let preview: String = snapshot.content.chars().take(500).collect();
        manifest.push_str(&format!("全文 chars={char_count} preview={preview}...\n"));
    } else {
        for block in &snapshot.blocks {
            manifest.push_str(&block_line_for_prompt(block, false));
        }
    }
    let text = format!(
        "{runtime_declaration}[当前编辑器打开的文档] 路径：{path}\n当前文档较长，本轮不注入截断正文，改注入可追踪块目录（本轮主事实的索引）：\n\n{manifest}\n编辑前必须先调用 read_doc 或 list_blocks 读取当前文档的目标内容；不得根据历史对话或前部截断片段直接 propose_edit。\n{edit_instruction}"
    );
    Ok(CurrentDocPromptContext {
        text: Some(text),
        requires_read_before_edit: true,
    })
}

#[derive(Debug)]
struct AgentTurnMessages {
    messages: Vec<serde_json::Value>,
    carryover: ag::context::Carryover,
    dropped_cross_doc_history: usize,
}

fn weak_cross_doc_history_block(history: &[ag::context::HistTurn]) -> Option<String> {
    if history.is_empty() {
        return None;
    }
    let mut lines = Vec::new();
    for turn in history {
        let doc = turn.rel_path.as_deref().unwrap_or("(未记录文档)");
        let turn_id = turn.turn_id.as_deref().unwrap_or("-");
        lines.push(format!(
            "- doc={doc} turn={turn_id} role={}: {}",
            turn.role, turn.content
        ));
    }
    Some(format!(
        "[其他文档历史] 以下历史来自非当前文档或旧版本无归属消息，仅用于理解用户显式回指；不得把其中正文当作当前文档事实，不得据此覆盖当前文档：\n{}",
        lines.join("\n")
    ))
}

fn tool_path_arg<'a>(params: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    params
        .get(key)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

fn tool_targets_current_doc(
    params: &serde_json::Value,
    current_rel: Option<&str>,
    key: &str,
) -> bool {
    let Some(current) = current_rel else {
        return false;
    };
    tool_path_arg(params, key) == Some(current)
}

fn is_current_doc_read_tool(
    tool_name: &str,
    params: &serde_json::Value,
    current_rel: Option<&str>,
) -> bool {
    matches!(tool_name, "read_doc" | "list_blocks")
        && tool_targets_current_doc(params, current_rel, "path")
}

fn current_doc_edit_requires_prior_read(
    params: &serde_json::Value,
    current_rel: Option<&str>,
    requires_read: bool,
    already_read: bool,
) -> bool {
    requires_read && !already_read && tool_targets_current_doc(params, current_rel, "path")
}

fn intent_hint_for_prompt(carryover: ag::context::Carryover, dropped_cross_doc: usize) -> String {
    match carryover {
        ag::context::Carryover::None => {
            if dropped_cross_doc > 0 {
                format!(
                    "【本轮意图】这是一个新请求，对话历史仅作同文档背景；已忽略 {dropped_cross_doc} 条非当前文档历史，不要让历史意图凌驾当前请求。"
                )
            } else {
                "【本轮意图】这是一个新请求，对话历史仅作背景，不要让历史意图凌驾当前请求。"
                    .to_string()
            }
        }
        ag::context::Carryover::Refine => {
            "【本轮意图】用户显式修正/追加上文产出；若历史来自其他文档，只能用于理解回指，当前文档事实仍以当前 LogicalState 为准。".to_string()
        }
        ag::context::Carryover::Select => {
            "【本轮意图】用户显式选择/沿用上文方案；被选历史不能替代当前文档事实，修改必须落在当前/声明目标的最新 LogicalState 上。".to_string()
        }
        ag::context::Carryover::ReferenceTarget => {
            "【本轮意图】用户显式延续上文话题/目标；历史只辅助解析回指，当前文档内容以当前 LogicalState 为唯一事实。".to_string()
        }
    }
}

fn edit_intent_has_terminal_tool_result(proposed_diff: bool, edit_blocked_reported: bool) -> bool {
    proposed_diff || edit_blocked_reported
}

/**
 * @GOV
 * codes:    BR-AG-DATA-001-DATA-AG-AG-TURN-011, BR-AG-DATA-002-DATA-AG-AG-TURN-011, BR-AG-DATA-004-DATA-AG-AG-TURN-011, BR-AG-DATA-005-DATA-AG-AG-TURN-011
 * type:     DATA
 * chain:    AG-TURN
 * rules:    BR-AG-DATA-001, BR-AG-DATA-002, BR-AG-DATA-004, BR-AG-DATA-005
 * term_ref: TERM-AG-005
 * boundary: in=currentDocPromptAndDocScopedHistory | out=AgentTurnMessages(currentDocP0AfterHistoryCrossDocHistoryWeak)
 */
/// 真实模型消息装配：系统 → 补强(P2) → 同文档历史(P1) → 跨文档弱历史(仅显式沿用) →
/// 显式引用(P1) → 当前文档(P0，贴近末尾) → 意图 → 当前输入。当前文档永远晚于历史注入。
fn assemble_agent_turn_messages(
    system_prompt: &str,
    aug_block: Option<&str>,
    history: &[ag::context::HistTurn],
    active_doc_path: Option<&str>,
    history_budget_chars: usize,
    mention_block: Option<&str>,
    doc_ctx: Option<&str>,
    prompt: &str,
) -> AgentTurnMessages {
    let carryover = crate::ag::context::decide_intent_carryover(prompt, !history.is_empty());
    let scoped =
        crate::ag::context::scope_history_for_current_doc(history, active_doc_path, carryover);
    let (kept_same_history, dropped_same_history) =
        crate::ag::context::select_recent_history(&scoped.same_doc, history_budget_chars);
    let used_same: usize = kept_same_history
        .iter()
        .map(|turn| turn.content.chars().count())
        .sum();
    let other_budget = history_budget_chars.saturating_sub(used_same).min(2500);
    let (kept_other_history, dropped_other_history) =
        crate::ag::context::select_recent_history(&scoped.other_doc, other_budget);
    let dropped_cross_doc_history =
        scoped.dropped_cross_doc + dropped_same_history + dropped_other_history;

    let mut messages = vec![serde_json::json!({
        "role": "system", "content": system_prompt
    })];
    if let Some(aug) = aug_block {
        messages.push(serde_json::json!({ "role": "system", "content": aug }));
    }
    for turn in &kept_same_history {
        messages.push(serde_json::json!({ "role": turn.role, "content": turn.content }));
    }
    if let Some(other_history) = weak_cross_doc_history_block(&kept_other_history) {
        messages.push(serde_json::json!({ "role": "system", "content": other_history }));
    }
    if let Some(mention) = mention_block {
        messages.push(serde_json::json!({ "role": "system", "content": mention }));
    }
    if let Some(ctx) = doc_ctx {
        messages.push(serde_json::json!({ "role": "system", "content": ctx }));
    }
    messages.push(serde_json::json!({
        "role": "system",
        "content": intent_hint_for_prompt(carryover, dropped_cross_doc_history),
    }));
    messages.push(serde_json::json!({ "role": "user", "content": prompt }));
    AgentTurnMessages {
        messages,
        carryover,
        dropped_cross_doc_history,
    }
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
    session_id: String,
    prompt: String,
    provider: Option<String>,
    model: Option<String>,
    active_doc_path: Option<String>,
    active_doc_content: Option<String>,
    active_doc_format: Option<String>,
    open_doc_snapshots: Vec<ag::blocks::OpenDocSnapshot>,
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

    let (workspace_id_for_turn, workspace_root_path) = {
        let conn = state
            .db
            .lock()
            .map_err(|_| IpcError::new("WS_DB_BUSY", "数据库连接繁忙", true, false))?;
        let ws = workspace::lifecycle::active_workspace(&conn)
            .map_err(|e| IpcError::new("WS_ACTIVE_FAILED", &e.to_string(), true, false))?
            .ok_or_else(|| IpcError::new("WS_NO_ACTIVE", "无活动 workspace", false, false))?;
        ag::history::ensure_session_exists(&conn, &ws.workspace_id, &session_id)
            .map_err(|e| IpcError::new("AG_HISTORY_DB_ERROR", &e.to_string(), true, false))?;
        let draft_payload = ag::history::AgentTurnPayload {
            version: 1,
            user_message: ag::history::PersistedChatMessage {
                id: format!("u-{turn_id}"),
                role: "user".to_string(),
                content: prompt.trim().to_string(),
                parts: None,
                streaming: None,
                error: None,
            },
            assistant_message: None,
            tool_summaries: Vec::new(),
            diff_refs: Vec::new(),
            failure: None,
            incomplete_reason: Some("stream_interrupted".to_string()),
        };
        ag::history::save_turn(
            &conn,
            &ws.workspace_id,
            &session_id,
            &turn_id,
            active_doc_path.as_deref(),
            "draft",
            &draft_payload,
        )
        .map_err(|e| IpcError::new("AG_HISTORY_DB_ERROR", &e.to_string(), true, false))?;
        (ws.workspace_id.clone(), ws.root_path.clone())
    };

    let read_ctx = ag::blocks::AgentReadContext::new(
        std::path::Path::new(&workspace_root_path),
        open_doc_snapshots,
    )?;
    let active_snapshot = match active_doc_path.as_deref() {
        Some(path) => {
            let open = read_ctx.open_snapshot(path).ok_or_else(|| {
                IpcError::new(
                    "AG_OPEN_DOC_SNAPSHOT_REQUIRED",
                    "当前打开文档缺少 LogicalState 快照，已阻断代理回合",
                    true,
                    false,
                )
            })?;
            if !is_active_editor_snapshot_source(&open.source) {
                return Err(IpcError::new(
                    "AG_ACTIVE_SNAPSHOT_REQUIRED",
                    "当前打开文档必须来自 active editor LogicalState 快照",
                    true,
                    false,
                ));
            }
            if let Some(format) = active_doc_format.as_deref() {
                if !format.is_empty() && format != open.edit_format {
                    return Err(IpcError::new(
                        "AG_ACTIVE_FORMAT_MISMATCH",
                        "active_doc_format 与 OpenDocSnapshot.editFormat 不一致，已阻断代理回合",
                        true,
                        false,
                    ));
                }
            }
            if let Some(legacy_content) = active_doc_content.as_deref() {
                if legacy_content != open.content {
                    return Err(IpcError::new(
                        "AG_ACTIVE_SNAPSHOT_MISMATCH",
                        "active_doc_content 与 OpenDocSnapshot 不一致，已阻断代理回合",
                        true,
                        false,
                    ));
                }
            }
            Some(read_ctx.snapshot(path)?)
        }
        None => None,
    };

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
    let ws_root: Option<String> = Some(workspace_root_path.clone());
    let mut aug_items: Vec<crate::ag::context::ContextItem> = Vec::new();
    if let Ok(conn) = state.db.lock() {
        // AST 记忆供给。
        if let Ok(mems) = crate::ast::supply::supply(&conn, &workspace_id_for_turn) {
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
        if let Ok(ks) = crate::kb::retrieve::retrieve(&conn, &workspace_id_for_turn, &prompt) {
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
        // 整篇引用：经 AgentReadContext 读取，已打开文档优先 LogicalState。
        for rel in &mentions {
            match read_ctx.snapshot(rel) {
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
            }
        }
        // 块级引用（选区精确锚，PI-AG-09F）：text 与 start/end 已由前端 SelectionRef 带入；消费前重新校验。
        for br in &block_refs {
            if br.text.trim().is_empty() {
                continue;
            }
            let payload = ReferencePayload::Selection {
                rel_path: br.rel_path.clone(),
                block_id: br.block_id.clone(),
                start: br.start,
                end: br.end,
                text: br.text.clone(),
                label: None,
            };
            parts.push(format_reference_payload_context(
                &payload,
                Some(&read_ctx),
                MENTION_CAP,
            ));
        }
        // 新主通道：ReferencePayload 覆盖 doc / selection / session / memory / knowledge / template / url / long_text。
        for reference in &references {
            parts.push(format_reference_payload_context(
                reference,
                Some(&read_ctx),
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

    let requires_edit_tool =
        active_doc_path.as_deref().is_some() && looks_like_edit_intent(&prompt);
    if requires_edit_tool {
        if let Some(snapshot) = &active_snapshot {
            if !snapshot.content.trim().is_empty() && snapshot.blocks.is_empty() {
                let _ = on_event.send(AgTurnEvent::Error {
                    turn_id: turn_id.clone(),
                    code: "AG_ACTIVE_BLOCKS_REQUIRED".to_string(),
                    message: "当前文档块投影为空，已阻断编辑请求，请重新加载文档后重试。"
                        .to_string(),
                    retriable: true,
                });
                emit_ag02(&app, &turn_id, ag::tool::ToolLoopState::LoopAborted);
                emit_ag01(&app, &turn_id, TurnState::TurnError);
                return Ok(());
            }
        }
    }
    let mut proposed_diff = false;
    let mut edit_blocked_reported = false;

    // P0 当前文档块视图（PI-AG-08E/08F/08G）。短文档注入完整块内容；超长文档注入块目录并要求工具读取。
    const DOC_CTX_CAP: usize = 9000; // 字符（dev；G5 标定）
    let current_doc_prompt_context = match (active_doc_path.as_deref(), active_snapshot.as_ref()) {
        (Some(path), Some(snapshot)) => {
            build_current_doc_prompt_context(path, snapshot, DOC_CTX_CAP)?
        }
        _ => CurrentDocPromptContext {
            text: None,
            requires_read_before_edit: false,
        },
    };
    let current_doc_requires_tool_read =
        requires_edit_tool && current_doc_prompt_context.requires_read_before_edit;
    let doc_ctx = current_doc_prompt_context.text;

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
    let assembled_messages = assemble_agent_turn_messages(
        ag::prompts::SYSTEM_PROMPT_CHAT,
        aug_block.as_deref(),
        &history,
        active_doc_path.as_deref(),
        hist_budget,
        mention_block.as_deref(),
        doc_ctx.as_deref(),
        &prompt,
    );
    let mut messages = assembled_messages.messages;
    let _carryover = assembled_messages.carryover;
    let _dropped_cross_doc_history = assembled_messages.dropped_cross_doc_history;

    // 工具定义（PI-AG-08B 只读 3 工具）；ws_root 已在上文 workspace 解析（X-FORBID-004 工具限界，PI-AG-09D）。
    let tools = ag::tool_registry::tool_definitions_json().map_err(capability_error_to_ipc)?;

    // SM-AG-02 工具循环（BR-AG-STATE-002：注册/权限校验 + 上限）。dev 上限 6 轮。
    // 循环经状态机驱动 + 广播 sm:SM-AG-02（ModelStreaming→ToolValidating→ToolExecuting→ResultInjecting→
    // 续调 / FinalAnswer→LoopCompleted / 达上限→LoopAborted）。工具卡数据仍由 Channel 投影（互不替代）。
    use ag::tool::ToolLoopState as TlSt;
    const MAX_ROUNDS: usize = 6;
    let mut current_doc_read_confirmed = !current_doc_requires_tool_read;
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
                if requires_edit_tool
                    && !edit_intent_has_terminal_tool_result(proposed_diff, edit_blocked_reported)
                {
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
                        ag::tool::ToolDecision::Allow if c.name == "edit_blocked" => {
                            match ag::tool_exec::exec_edit_blocked(&params) {
                                Ok(result) => {
                                    edit_blocked_reported = true;
                                    (result, true)
                                }
                                Err(error) => (format!("阻断报告无效：{}", error.message), false),
                            }
                        }
                        // 写工具 propose_edit：经 DE 生成 pending diff（X-FORBID-006，不直写）→ emit DiffProposed。
                        ag::tool::ToolDecision::Allow if c.name == "propose_edit" => {
                            if current_doc_edit_requires_prior_read(
                                &params,
                                active_doc_path.as_deref(),
                                current_doc_requires_tool_read,
                                current_doc_read_confirmed,
                            ) {
                                (
                                    serde_json::json!({
                                        "code": "AG_CURRENT_DOC_READ_REQUIRED",
                                        "message": "当前文档较长，本轮只注入了块目录；请先调用 read_doc 或 list_blocks 读取当前文档目标内容，再重新调用 propose_edit。",
                                        "retriable": true,
                                    })
                                    .to_string(),
                                    false,
                                )
                            } else {
                                match ag_propose_edit(
                                    &state,
                                    &app,
                                    &params,
                                    &read_ctx,
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
                        }
                        // 新建文档工具 create_doc：不经 DE diff，直接创建文件（用户明确要求新建时使用）。
                        ag::tool::ToolDecision::Allow if c.name == "create_doc" => {
                            match ws_root.as_deref() {
                                None => ("无活动 workspace，无法创建文件".to_string(), false),
                                Some(_) => match ag_create_doc(&state, &params) {
                                    Ok(r) => (r, true),
                                    Err(e) => (format!("创建文件失败：{}", e.message), false),
                                },
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
                            Some(_) => {
                                match ag::tool_exec::exec_read_only(&read_ctx, &c.name, &params) {
                                    Ok(r) => (r, true),
                                    Err(e) => (format!("工具执行失败：{}", e.message), false),
                                }
                            }
                        },
                    };
                    if success
                        && is_current_doc_read_tool(&c.name, &params, active_doc_path.as_deref())
                    {
                        current_doc_read_confirmed = true;
                    }
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
    if requires_edit_tool
        && !edit_intent_has_terminal_tool_result(proposed_diff, edit_blocked_reported)
    {
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
            doc::tool_runtime::initialize(&app.handle());
            #[cfg(target_os = "macos")]
            macos_exit_guard::install(&app.handle())
                .map_err(std::io::Error::other)?;
            // workspace.db 落应用数据目录（控制态，非用户正文；承 BR-SYS-PERSIST-001 迁移框架）。
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let conn = db::open_and_migrate(&data_dir.join("workspace.db"))?;
            // 异常退出清算（BR-SYS-STATE-005）：必须先于任何 workspace/会话/历史恢复执行——残留
            // pending diff 一律过期，残留未保存内容一律丢弃，不得被当作可信态复活（APP-CLOSE-DIFF-LIFECYCLE-01）。
            for (workspace_id, report) in de::lifecycle::sweep_all_workspaces_on_startup(&conn)?
            {
                if report.expired_pending > 0
                    || report.discarded_dirty_snapshots > 0
                    || report.discarded_awaiting_save_diffs > 0
                {
                    eprintln!(
                        "[app-close] workspace_id={workspace_id} 异常退出清算：expired_pending={} discarded_dirty_snapshots={} discarded_awaiting_save_diffs={}",
                        report.expired_pending,
                        report.discarded_dirty_snapshots,
                        report.discarded_awaiting_save_diffs
                    );
                }
            }
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
        .build(tauri::generate_context!())
        .expect("error while building Binder")
        .run(|app_handle, event| {
            /*
             * @GOV
             * codes:    BR-SYS-STATE-004-GUARD-SYS-APP-CLOSE-004
             * type:     GUARD
             * chain:    APP-CLOSE
             * rules:    BR-SYS-STATE-004
             * boundary: in=ExitRequestedAndMainWindowPresence | out=frontendCloseRequestEmittedOrRuntimeExitAllowed(noHeadlessProcessResidue)
             */
            // BR-SYS-STATE-004（APP-CLOSE-DIFF-LIFECYCLE-01）：Cmd+Q / Dock 退出 / 系统关机到达的是
            // 应用级 RunEvent::ExitRequested，不经过窗口的 WindowEvent::CloseRequested。此前只有
            // App.tsx 的 onCloseRequested 拦截窗口关闭按钮一条路径，Cmd+Q 完全绕过关闭前清点/弹窗。
            // 主窗口仍存在时拦截默认退出，并显式广播给 React 进入 requestClose()。不能在此直接
            // window.close()：实测该路径不会可靠触发 WebView 的 onCloseRequested，因此会绕过弹窗。
            // 窗口已经销毁时不能再 prevent_exit：Tauri 会因最后一个窗口销毁再次派发 ExitRequested；
            // 若仍拦截，会留下无窗口的后台进程。
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                if app_handle.get_webview_window("main").is_some() {
                    api.prevent_exit();
                    match app_handle.emit("sys:exit-requested", ()) {
                        Ok(()) => eprintln!(
                            "[app-close] 原生退出已拦截，已向前端广播 sys:exit-requested"
                        ),
                        Err(error) => eprintln!(
                            "[app-close] 原生退出已拦截，但广播 sys:exit-requested 失败: {error}"
                        ),
                    }
                } else {
                    eprintln!(
                        "[app-close] 原生退出未拦截：main 窗口已不存在，允许运行时退出"
                    );
                }
            }
        });
}
