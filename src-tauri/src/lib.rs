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
    reg.register("ws.close_workspace");
    reg.register("ws.active_workspace");
    reg.register("ws.list_docs");
    reg.register("ws.read_doc");
    reg.register("ws.save_doc");
    reg.register("ws.persist_diff");
    reg.register("ws.list_pending_diffs");
    reg.register("ws.apply_diff");
    reg.register("ws.discard_diff");
    reg.register("de.generate_diff");
    reg.register("de.accept_diff");
    reg.register("de.reject_diff");
    reg.register("de.list_diffs");
    reg.register("ag.start_turn");
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
        "ws.close_workspace" => route_close_workspace(&request, &state),
        "ws.active_workspace" => route_active_workspace(&state),
        "ws.list_docs" => route_list_docs(&state),
        "ws.read_doc" => route_read_doc(&request, &state),
        "ws.save_doc" => route_save_doc(&request, &state),
        "ws.persist_diff" => route_persist_diff(&request, &state),
        "ws.list_pending_diffs" => route_list_pending_diffs(&request, &state),
        "ws.apply_diff" => route_apply_diff(&request, &state),
        "ws.discard_diff" => route_discard_diff(&request, &state),
        "de.generate_diff" => route_generate_diff(&request, &state),
        "de.accept_diff" => route_accept_diff(&request, &state),
        "de.reject_diff" => route_reject_diff(&request, &state),
        "de.list_diffs" => route_list_diffs(&request, &state),
        "ag.start_turn" => route_start_turn(&request),
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
    let docs = workspace::listing::list_docs(
        std::path::Path::new(&active.root_path),
        &active.workspace_id,
    )?;
    ws_json(&docs)
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
