//! Binder 后端骨架（Tauri 2）。
//! 模块边界承接 A-SYS-C-T-01 §4 分层与各模块 Plan Item 的文件范围；
//! 各模块为 G3 脚手架占位，@GOV 块在 G4 实现时随实际逻辑落入对应模块。
//! 分层约束 X-SEQ-002：前端经 command 触达，后端经领域逻辑触达 FS/db。

#![recursion_limit = "512"]

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
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
pub mod ed; // PI-ED-* 编辑器后端会话路由
pub mod reference; // PI-REF-* 引用（@mention）

// —— P4 上层能力 ——
pub mod ast; // PI-AST-* 记忆
pub mod cbt; // PI-CBT-* 构建
pub mod fonts; // ED-FONT-LOCAL-LIBRARY-01 本地字体库只读枚举
pub mod kb; // PI-KB-* 知识库
pub mod prv;
pub mod tpl; // PI-TPL-* 模板/工作流 // PI-PRV-* 预览输出

// —— 商业化客户端侧 ——
pub mod acct; // PI-ACCT-* 账号会话
pub mod ent; // PI-ENT-* 订阅门控

use ag::runtime::*;
use ast::routes::*;
use de::routes::*;
use doc::routes::*;
use ed::routes::*;
use ent::commands::*;
use fonts::routes::*;
use ipc::{CommandRegistry, CommandRequest, CommandResponse, IpcError};
use prv::routes::*;
use reference::routes::*;
use rusqlite::{Connection, OptionalExtension};
use workspace::routes::*;

/// 应用运行时状态：workspace.db 连接 + command 注册表（PI-FE-00 联通后端）。
pub(crate) struct AppState {
    pub(crate) db: Mutex<Connection>,
    pub(crate) registry: CommandRegistry,
    pub(crate) assist_terminals: Mutex<HashMap<String, ag::assist::AssistState>>,
    pub(crate) active_agent_turns: Arc<Mutex<HashMap<String, ag::runtime::ActiveAgentTurn>>>,
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
    reg.register("ws.delete_folder");
    reg.register("ws.plan_operation");
    reg.register("ws.get_operation_plan");
    reg.register("ws.execute_operation");
    reg.register("ws.cancel_operation");
    reg.register("ws.list_operations");
    reg.register("ws.project_create_document_bundle");
    reg.register("ws.compensate_create_document_open_failure");
    reg.register("ws.compensate_create_document_projection_preflight_failure");
    reg.register("ws.compensate_create_document_preview_failure");
    reg.register("prv.preview");
    reg.register("prv.export");
    reg.register("prv.media_src");
    reg.register("prv.table_data");
    reg.register("prv.open_external");
    reg.register("fonts.list_fonts");
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
    reg.register("de.promote_pending_diff_on_open");
    reg.register("de.accept_diff");
    reg.register("de.reject_diff");
    reg.register("de.expire_diff");
    reg.register("de.list_diffs");
    reg.register("de.update_image_diff_snapshots");
    reg.register("ed.persist_session_snapshot");
    reg.register("ed.list_session_snapshots");
    reg.register("ed.persist_open_tabs");
    reg.register("ed.list_open_tabs");
    reg.register("ag.list_blocks");
    reg.register("ag.list_sessions");
    reg.register("ag.save_session");
    reg.register("ag.save_turn");
    reg.register("ag.list_turns");
    reg.register("ag.list_turn_events");
    reg.register("ag.delete_session");
    // AG-1.0-IMP-TASKRUN-UI-01：按会话列受限任务实例（SM-AG-04 状态/步骤，前端会话摘要呈现）。
    reg.register("ag.list_task_runs");
    // AG-1.0-TASKRUN-REWORK-IMP-RUNTIME-01：任务级裁决（DecisionRequest，三层分离）。
    reg.register("ag.propose_decision");
    reg.register("ag.resolve_decision");
    // TaskPlan 仅保留 TurnEvent 派生查询；不存在外部登记或执行入口。
    reg.register("ag.list_plans");
    reg.register("ref.list_candidates");
    reg.register("ref.resolve_anchor");
    reg.register("ref.verify_anchor");
    reg.register("ref.build_injection");
    reg.register("ast.list_entries");
    reg
}

/**
 * @GOV
 * codes:    BR-SYS-DATA-003-API-SYS-IPC-CALL-005
 * type:     API
 * chain:    IPC-CALL
 * rules:    BR-SYS-DATA-003
 * boundary: in=RegisteredEditPersistenceCommandRequest | out=SharedRustRouteResult(WryOrMockRuntimeSameStateMachineAndDiskEffects) | does_not=duplicate_test_only_route_or_bypass_command_registry
 */
/// 编辑持久化 G5 链共用的生产路由选择。Wry `dispatch` 与 Tauri Mock Webview 系统测试必须命中同一函数，
/// 避免测试复制一套“能过但生产未接线”的 command 映射；注册校验仍由调用方在进入本函数前执行。
fn route_edit_persistence_command<R: tauri::Runtime>(
    request: &CommandRequest,
    state: &AppState,
    app: &tauri::AppHandle<R>,
) -> Option<Result<serde_json::Value, IpcError>> {
    match request.command.as_str() {
        "ws.read_doc" => Some(route_read_doc(request, state)),
        "ws.save_doc" => Some(route_save_doc(request, state, app)),
        "de.generate_diff" => Some(route_generate_diff(request, state)),
        "de.accept_draft_diff" => Some(route_accept_draft_diff(request, state, app)),
        "de.promote_pending_diff_on_open" => {
            Some(route_promote_pending_diff_on_open(request, state))
        }
        "de.reject_diff" => Some(route_reject_diff(request, state, app)),
        _ => None,
    }
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
    let routed = route_edit_persistence_command(&request, &state, &app).unwrap_or_else(|| {
        match request.command.as_str() {
            "ws.open_workspace" => route_open_workspace(&request, &state),
            "ws.list_workspaces" => route_list_workspaces(&state),
            "ws.switch_workspace" => route_switch_workspace(&request, &state),
            "ws.close_workspace" => route_close_workspace(&request, &state),
            "ws.active_workspace" => route_active_workspace(&state),
            "ws.list_docs" => route_list_docs(&state),
            "ws.search" => route_search(&request, &state),
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
            "ws.delete_folder" => route_delete_folder(&request, &state),
            "ws.plan_operation" => route_plan_workspace_operation(&request, &state),
            "ws.get_operation_plan" => route_get_workspace_operation_plan(&request, &state),
            "ws.execute_operation" => route_execute_workspace_operation(&request, &state),
            "ws.cancel_operation" => route_cancel_workspace_operation(&request, &state),
            "ws.list_operations" => route_list_workspace_operations(&request, &state),
            "ws.project_create_document_bundle" => {
                route_project_create_document_bundle(&request, &state)
            }
            "ws.compensate_create_document_open_failure" => {
                route_compensate_create_document_open_failure(&request, &state)
            }
            "ws.compensate_create_document_projection_preflight_failure" => {
                route_compensate_create_document_projection_preflight_failure(&request, &state)
            }
            "ws.compensate_create_document_preview_failure" => {
                route_compensate_create_document_preview_failure(&request, &state)
            }
            "prv.preview" => route_preview(&request, &state, &app),
            "prv.export" => route_export(&request, &state),
            "prv.media_src" => route_media_src(&request, &state, &app),
            "prv.table_data" => route_table_data(&request, &state),
            "prv.open_external" => route_open_external(&request, &state),
            "fonts.list_fonts" => route_list_fonts(),
            "doc.create_draft" => route_create_draft(&request, &state),
            "doc.open_for_edit" => route_open_for_edit(&request, &state, &app),
            "doc.restore_edit_media" => route_restore_edit_media(&request, &state, &app),
            "doc.import_project_image" => route_import_project_image(&request, &state, &app),
            "doc.get_style_report" => route_get_style_report(&request, &state),
            "doc.save_from_edit" => route_save_from_edit(&request, &state, &app),
            "ws.list_timeline" => route_list_timeline(&request, &state),
            "ws.preview_node" => route_preview_node(&request, &state),
            "ws.restore_node" => route_restore_node(&request, &state, &app),
            "de.accept_diff" => route_accept_diff(&request, &state, &app),
            "de.expire_diff" => route_expire_diff(&request, &state, &app),
            "de.list_diffs" => route_list_diffs(&request, &state),
            "de.update_image_diff_snapshots" => route_update_image_diff_snapshots(&request, &state),
            "ed.persist_session_snapshot" => route_persist_session_snapshot(&request, &state),
            "ed.list_session_snapshots" => route_list_session_snapshots(&state),
            "ed.persist_open_tabs" => route_persist_open_tabs(&request, &state),
            "ed.list_open_tabs" => route_list_open_tabs(&state),
            "ag.list_blocks" => route_list_blocks(&request, &state),
            "ag.list_sessions" => route_list_agent_sessions(&state),
            "ag.save_session" => route_save_agent_session(&request, &state),
            "ag.save_turn" => route_save_agent_turn(&request, &state),
            "ag.list_turns" => route_list_agent_turns(&request, &state),
            "ag.list_turn_events" => {
                ag::history_routes::route_list_agent_turn_events(&request, &state)
            }
            "ag.list_task_runs" => route_list_agent_task_runs(&request, &state),
            "ag.propose_decision" => route_propose_decision(&request, &state),
            "ag.resolve_decision" => route_resolve_decision(&request, &state),
            "ag.list_plans" => route_list_plans(&request, &state),
            "ag.delete_session" => route_delete_agent_session(&request, &state),
            "ref.list_candidates" => route_ref_list_candidates(&request, &state),
            "ref.resolve_anchor" => route_ref_resolve_anchor(&request, &state),
            "ref.verify_anchor" => route_ref_verify_anchor(&request, &state),
            "ref.build_injection" => route_ref_build_injection(&request, &state),
            "ast.list_entries" => route_ast_list_entries(&state),
            // 注册表已放行但路由缺失：实现期补全各模块 command（视为内部错误）。
            other => Err(IpcError::new(
                "IPC_ROUTE_MISSING",
                &format!("command 已注册但未接线: {other}"),
                false,
                false,
            )),
        }
    });
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
            let interrupted_task_runs = ag::task::mark_all_stale_as_interrupted(&conn)?;
            if interrupted_task_runs > 0 {
                eprintln!(
                    "[agent-task] startup cleanup: interrupted_task_runs={interrupted_task_runs}"
                );
            }
            let interrupted_workspace_operations =
                workspace::operation::interrupt_nonterminal_plans(&conn)?;
            if interrupted_workspace_operations > 0 {
                eprintln!(
                    "[workspace-operation] startup cleanup: interrupted_plans={interrupted_workspace_operations}"
                );
            }
            app.manage(AppState {
                db: Mutex::new(conn),
                registry: build_registry(),
                assist_terminals: Mutex::new(HashMap::new()),
                active_agent_turns: Arc::new(Mutex::new(HashMap::new())),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            dispatch,
            ag_start_turn,
            ag_cancel_turn,
            ag_ack_preview,
            byok_upload,
            byok_list_providers,
            byok_delete_key,
            list_models,
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
