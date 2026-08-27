//! AgentTurn 单活注册、取消控制和预览回执 command。

use crate::ipc::IpcError;
use crate::{ag, workspace, AppState};
#[cfg(test)]
use rusqlite::Connection;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

pub(crate) struct ActiveAgentTurn {
    turn_id: String,
    cancelled: Arc<AtomicBool>,
    abort_handle: Option<futures_util::future::AbortHandle>,
}

pub(super) struct ActiveAgentTurnLease {
    active_turns: Arc<Mutex<HashMap<String, ActiveAgentTurn>>>,
    session_id: String,
    turn_id: String,
}

impl Drop for ActiveAgentTurnLease {
    fn drop(&mut self) {
        if let Ok(mut turns) = self.active_turns.lock() {
            if turns
                .get(&self.session_id)
                .map(|turn| turn.turn_id.as_str())
                == Some(self.turn_id.as_str())
            {
                turns.remove(&self.session_id);
            }
        }
    }
}

pub(super) fn register_active_agent_turn(
    state: &AppState,
    session_id: &str,
    turn_id: &str,
) -> Result<(ActiveAgentTurnLease, Arc<AtomicBool>), IpcError> {
    let mut turns = state
        .active_agent_turns
        .lock()
        .map_err(|_| IpcError::new("AG_TURN_REGISTRY_BUSY", "Agent 回合注册表繁忙", true, false))?;
    if let Some(active) = turns.get(session_id) {
        // 澄清等待（非终态）被取消后用户补充输入发起新回合：已取消的旧回合允许被取代
        // （BR-SYS-STATE-007，澄清不阻塞新回合；BR-AG-STATE-001 单活守卫仅约束未取消活动回合）。
        if !active.cancelled.load(Ordering::Acquire) {
            return Err(IpcError::new(
                "AG_TURN_ACTIVE",
                &format!("会话已有进行中的回合: {}", active.turn_id),
                true,
                false,
            ));
        }
        turns.remove(session_id);
    }
    let cancelled = Arc::new(AtomicBool::new(false));
    turns.insert(
        session_id.to_string(),
        ActiveAgentTurn {
            turn_id: turn_id.to_string(),
            cancelled: Arc::clone(&cancelled),
            abort_handle: None,
        },
    );
    drop(turns);
    Ok((
        ActiveAgentTurnLease {
            active_turns: Arc::clone(&state.active_agent_turns),
            session_id: session_id.to_string(),
            turn_id: turn_id.to_string(),
        },
        cancelled,
    ))
}

pub(super) fn install_agent_turn_abort_handle(
    state: &AppState,
    session_id: &str,
    turn_id: &str,
    handle: futures_util::future::AbortHandle,
) -> Result<(), IpcError> {
    let mut turns = state
        .active_agent_turns
        .lock()
        .map_err(|_| IpcError::new("AG_TURN_REGISTRY_BUSY", "Agent 回合注册表繁忙", true, false))?;
    let active = turns
        .get_mut(session_id)
        .ok_or_else(|| IpcError::new("AG_TURN_NOT_ACTIVE", "Agent 回合已不再活动", false, false))?;
    if active.turn_id != turn_id {
        return Err(IpcError::new(
            "AG_TURN_MISMATCH",
            "取消控制器与当前活动回合不匹配",
            false,
            false,
        ));
    }
    if active.cancelled.load(Ordering::Acquire) {
        handle.abort();
    }
    active.abort_handle = Some(handle);
    Ok(())
}

pub(super) fn lock_agent_turn_effect<'a>(
    state: &'a AppState,
    session_id: &str,
    turn_id: &str,
) -> Result<std::sync::MutexGuard<'a, HashMap<String, ActiveAgentTurn>>, IpcError> {
    let turns = state
        .active_agent_turns
        .lock()
        .map_err(|_| IpcError::new("AG_TURN_REGISTRY_BUSY", "Agent 回合注册表繁忙", true, false))?;
    let active = turns
        .get(session_id)
        .ok_or_else(|| IpcError::new("AG_TURN_NOT_ACTIVE", "Agent 回合已不再活动", false, false))?;
    if active.turn_id != turn_id || active.cancelled.load(Ordering::Acquire) {
        return Err(IpcError::new(
            "AG_TURN_CANCELLED",
            "Agent 回合已取消，禁止产生新的文档修改建议",
            false,
            false,
        ));
    }
    Ok(turns)
}

/**
 * @GOV
 * codes:    BR-AG-STATE-001-API-AG-AG-TURN-019
 * type:     API
 * chain:    AG-TURN
 * rules:    BR-AG-STATE-001
 * boundary: in=AgentTurnCancellationIdentity(sessionId,turnId) | out=CancelledAgentTurn(abortedGatewayStreamAndNoFurtherToolStart)
 */
#[tauri::command]
pub(crate) fn ag_cancel_turn(
    session_id: String,
    turn_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), IpcError> {
    cancel_active_agent_turn(&state, &session_id, &turn_id)
}

/**
 * @GOV
 * codes:    BR-SYS-STATE-008-API-AG-AG-TURN-003, BR-AG-DATA-001-API-AG-AG-TURN-003
 * type:     API
 * chain:    AG-TURN, DE-DIFF, ED-EDIT
 * rules:    BR-SYS-STATE-008, BR-AG-DATA-001
 * term_ref: TERM-AG-015, TERM-ED-008
 * boundary: in=EditorAppliedLogicalStatePreviewIdentity | out=PreviewContinuationCheckpointAcknowledged(noModelCallNoDiffDecisionNoDiskWrite)
 */
/// 编辑器只在受控预览已真实写入 LogicalState 后回报基线。该命令不接受 DiffCard、不写盘，也不
/// 自行调用模型；下一 AgentTurn 必须由前端显式发起。
#[tauri::command]
pub(crate) fn ag_ack_preview(
    task_run_id: String,
    session_id: String,
    turn_id: String,
    diff_id: String,
    rel_path: String,
    session_version: u64,
    content_hash: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, IpcError> {
    if task_run_id.is_empty()
        || session_id.is_empty()
        || turn_id.is_empty()
        || diff_id.is_empty()
        || rel_path.is_empty()
        || content_hash.is_empty()
    {
        return Err(IpcError::new(
            "AG_PREVIEW_ACK_INVALID",
            "预览回执缺少任务、回合、Diff、路径或内容基线标识",
            false,
            false,
        ));
    }
    let conn = state
        .db
        .lock()
        .map_err(|_| IpcError::new("WS_DB_BUSY", "数据库连接繁忙", true, false))?;
    let workspace = workspace::lifecycle::active_workspace(&conn)
        .map_err(|error| IpcError::new("WS_ACTIVE_FAILED", &error.to_string(), true, false))?
        .ok_or_else(|| IpcError::new("WS_NO_ACTIVE", "无活动 workspace", false, false))?;
    let checkpoint = ag::task::PreviewCheckpoint {
        diff_id,
        task_run_id: task_run_id.clone(),
        workspace_id: workspace.workspace_id,
        session_id,
        turn_id,
        rel_path,
        session_version,
        content_hash,
    };
    ag::task::acknowledge_preview_checkpoint(&conn, &checkpoint).map_err(|error| {
        IpcError::new(
            "AG_PREVIEW_ACK_REJECTED",
            &format!("预览回执未通过身份或状态校验：{error}"),
            false,
            false,
        )
    })?;
    workspace::operation::acknowledge_create_document_preview(
        &conn,
        &checkpoint.diff_id,
        checkpoint.session_version,
        &checkpoint.content_hash,
    )?;
    Ok(serde_json::json!({ "task_run_id": task_run_id, "ready": true }))
}

pub(super) fn cancel_active_agent_turn(
    state: &AppState,
    session_id: &str,
    turn_id: &str,
) -> Result<(), IpcError> {
    let mut turns = state
        .active_agent_turns
        .lock()
        .map_err(|_| IpcError::new("AG_TURN_REGISTRY_BUSY", "Agent 回合注册表繁忙", true, false))?;
    let active = turns.get_mut(session_id).ok_or_else(|| {
        IpcError::new(
            "AG_TURN_NOT_ACTIVE",
            "Agent 回合已结束或不存在",
            false,
            false,
        )
    })?;
    if active.turn_id != turn_id {
        return Err(IpcError::new(
            "AG_TURN_MISMATCH",
            "取消请求不属于当前活动回合",
            false,
            false,
        ));
    }
    active.cancelled.store(true, Ordering::Release);
    if let Some(handle) = active.abort_handle.take() {
        handle.abort();
    }
    Ok(())
}

#[cfg(test)]
mod ag_execution_kernel_tests {
    use super::*;
    use crate::build_registry;

    fn state() -> AppState {
        AppState {
            db: Mutex::new(Connection::open_in_memory().unwrap()),
            registry: build_registry(),
            assist_terminals: Mutex::new(HashMap::new()),
            active_agent_turns: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    // covers: BR-AG-STATE-001
    #[test]
    fn session_has_one_active_turn_and_cancel_reaches_backend_control() {
        let state = state();
        let (lease, cancelled) = register_active_agent_turn(&state, "s1", "t1").unwrap();
        let second_error = match register_active_agent_turn(&state, "s1", "t2") {
            Ok(_) => panic!("同 session 不得双活"),
            Err(error) => error,
        };
        assert_eq!(second_error.code, "AG_TURN_ACTIVE");
        let (abort_handle, _abort_registration) = futures_util::future::AbortHandle::new_pair();
        install_agent_turn_abort_handle(&state, "s1", "t1", abort_handle).unwrap();
        cancel_active_agent_turn(&state, "s1", "t1").unwrap();
        assert!(cancelled.load(Ordering::Acquire));

        drop(lease);
        let (_next, _) = register_active_agent_turn(&state, "s1", "t2").unwrap();
    }

    // covers: BR-AG-STATE-001
    #[test]
    fn cancel_rejects_wrong_or_missing_turn() {
        let state = state();
        let (_lease, _) = register_active_agent_turn(&state, "s1", "t1").unwrap();
        assert_eq!(
            cancel_active_agent_turn(&state, "s1", "wrong")
                .unwrap_err()
                .code,
            "AG_TURN_MISMATCH"
        );
        assert_eq!(
            cancel_active_agent_turn(&state, "missing", "t1")
                .unwrap_err()
                .code,
            "AG_TURN_NOT_ACTIVE"
        );
    }
}
