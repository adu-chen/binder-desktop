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
    workspace_id: String,
    cancelled: Arc<AtomicBool>,
    abort_handle: Option<futures_util::future::AbortHandle>,
    phase: ActiveAgentTurnPhase,
    task_run_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActiveAgentTurnPhase {
    Running,
    AwaitingClarification,
}

pub(super) struct ActiveAgentTurnLease {
    active_turns: Arc<Mutex<HashMap<String, ActiveAgentTurn>>>,
    session_id: String,
    turn_id: String,
}

impl Drop for ActiveAgentTurnLease {
    fn drop(&mut self) {
        if let Ok(mut turns) = self.active_turns.lock() {
            let should_remove = turns.get(&self.session_id).is_some_and(|turn| {
                turn.turn_id == self.turn_id
                    && turn.phase != ActiveAgentTurnPhase::AwaitingClarification
            });
            if should_remove {
                turns.remove(&self.session_id);
            }
        }
    }
}

/**
 * @GOV
 * codes:    BR-SYS-STATE-007-GUARD-AG-AG-TURN-034, BR-AG-STATE-001-GUARD-AG-AG-TURN-034
 * type:     GUARD
 * chain:    AG-TURN
 * rules:    BR-SYS-STATE-007, BR-AG-STATE-001
 * term_ref: TERM-AG-001, TERM-AG-015
 * boundary: in=AgentTurnStartIdentity(sessionId,workspaceId,turnId,clarificationResume) | out=ExclusiveActiveAgentTurnLeaseAndOptionalOriginalTaskRunIdentity(rejectsWrongSessionWorkspaceTurnRepeatedOrTerminalResume)
 */
pub(super) fn register_active_agent_turn(
    state: &AppState,
    workspace_id: &str,
    session_id: &str,
    turn_id: &str,
    clarification_resume: bool,
) -> Result<(ActiveAgentTurnLease, Arc<AtomicBool>, Option<String>), IpcError> {
    let mut turns = state
        .active_agent_turns
        .lock()
        .map_err(|_| IpcError::new("AG_TURN_REGISTRY_BUSY", "Agent 回合注册表繁忙", true, false))?;
    if clarification_resume {
        let active = turns.get_mut(session_id).ok_or_else(|| {
            IpcError::new(
                "AG_CLARIFICATION_NOT_AWAITING",
                "当前会话没有可恢复的澄清等待回合",
                false,
                false,
            )
        })?;
        if active.turn_id != turn_id {
            return Err(IpcError::new(
                "AG_TURN_MISMATCH",
                "补充说明不属于当前澄清等待回合",
                false,
                false,
            ));
        }
        if active.workspace_id != workspace_id {
            return Err(IpcError::new(
                "AG_CLARIFICATION_WORKSPACE_MISMATCH",
                "澄清等待回合不属于当前 workspace",
                false,
                false,
            ));
        }
        if active.phase != ActiveAgentTurnPhase::AwaitingClarification {
            return Err(IpcError::new(
                "AG_CLARIFICATION_NOT_AWAITING",
                "AgentTurn 当前不处于澄清等待态，禁止重复或终态重入",
                false,
                false,
            ));
        }
        if active.cancelled.load(Ordering::Acquire) {
            return Err(IpcError::new(
                "AG_TURN_CANCELLED",
                "澄清等待回合已取消",
                false,
                false,
            ));
        }
        let task_run_id = active.task_run_id.clone();
        active.phase = ActiveAgentTurnPhase::Running;
        active.abort_handle = None;
        let cancelled = Arc::clone(&active.cancelled);
        drop(turns);
        return Ok((
            ActiveAgentTurnLease {
                active_turns: Arc::clone(&state.active_agent_turns),
                session_id: session_id.to_string(),
                turn_id: turn_id.to_string(),
            },
            cancelled,
            task_run_id,
        ));
    }

    if let Some(active) = turns.get(session_id) {
        // 已发送取消信号但旧 command 尚未释放 lease 时，允许新 turn_id 接管；未取消活动态与
        // AwaitingClarification 均须显式失败，不能把普通发送误当澄清恢复。
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
            workspace_id: workspace_id.to_string(),
            cancelled: Arc::clone(&cancelled),
            abort_handle: None,
            phase: ActiveAgentTurnPhase::Running,
            task_run_id: None,
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
        None,
    ))
}

/**
 * @GOV
 * codes:    BR-SYS-STATE-007-EFFECT-AG-AG-TURN-034, BR-AG-STATE-001-EFFECT-AG-AG-TURN-034
 * type:     EFFECT
 * chain:    AG-TURN
 * rules:    BR-SYS-STATE-007, BR-AG-STATE-001
 * term_ref: TERM-AG-001, TERM-AG-015
 * boundary: in=RunningAgentTurnWithBlockingIntentAndOptionalTaskRunIdentity | out=AwaitingClarificationAgentTurnIdentity(retainedAfterCommandReturnAndClaimableExactlyOnce)
 */
pub(super) fn park_agent_turn_for_clarification(
    state: &AppState,
    session_id: &str,
    turn_id: &str,
    task_run_id: Option<&str>,
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
            "澄清等待身份与当前活动回合不匹配",
            false,
            false,
        ));
    }
    if active.phase != ActiveAgentTurnPhase::Running || active.cancelled.load(Ordering::Acquire) {
        return Err(IpcError::new(
            "AG_TURN_CANCELLED",
            "Agent 回合已取消或不能进入澄清等待",
            false,
            false,
        ));
    }
    active.phase = ActiveAgentTurnPhase::AwaitingClarification;
    active.abort_handle = None;
    active.task_run_id = task_run_id.map(str::to_string);
    Ok(())
}

/// 在任务级产物首次出现时，把按需创建的 TaskRun 绑定到仍活动的原 AgentTurn。
/// 重复绑定同一身份幂等；不同身份失败关闭，避免动态工具把任务挂到其他回合。
pub(super) fn bind_agent_turn_task_run(
    state: &AppState,
    session_id: &str,
    turn_id: &str,
    task_run_id: &str,
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
            "TaskRun 绑定身份与当前活动回合不匹配",
            false,
            false,
        ));
    }
    if active.cancelled.load(Ordering::Acquire) {
        return Err(IpcError::new(
            "AG_TURN_CANCELLED",
            "已取消的 AgentTurn 不能绑定 TaskRun",
            false,
            false,
        ));
    }
    if active
        .task_run_id
        .as_deref()
        .is_some_and(|current| current != task_run_id)
    {
        return Err(IpcError::new(
            "AG_TASK_IDENTITY_MISMATCH",
            "AgentTurn 已绑定其他 TaskRun",
            false,
            false,
        ));
    }
    active.task_run_id = Some(task_run_id.to_string());
    Ok(())
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
    app: tauri::AppHandle,
) -> Result<(), IpcError> {
    let parked_task_run_id = cancel_active_agent_turn(&state, &session_id, &turn_id)?;
    if let Some(task_run_id) = parked_task_run_id {
        if let Ok(conn) = state.db.lock() {
            super::task_runtime::settle_task_run(
                &conn,
                &task_run_id,
                &turn_id,
                ag::task::TurnOutcome::Cancelled,
            );
        }
        super::turn_events::emit_ag01(&app, &turn_id, ag::turn::TurnState::Cancelled);
    }
    Ok(())
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
) -> Result<Option<String>, IpcError> {
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
    let parked_task_run_id = if active.phase == ActiveAgentTurnPhase::AwaitingClarification {
        active.task_run_id.clone()
    } else {
        None
    };
    if let Some(handle) = active.abort_handle.take() {
        handle.abort();
    }
    if parked_task_run_id.is_some() {
        turns.remove(session_id);
    }
    Ok(parked_task_run_id)
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

    fn error_code<T>(result: Result<T, IpcError>) -> String {
        match result {
            Err(error) => error.code,
            Ok(_) => panic!("预期身份校验失败"),
        }
    }

    // covers: BR-AG-STATE-001
    #[test]
    fn session_has_one_active_turn_and_cancel_reaches_backend_control() {
        let state = state();
        let (lease, cancelled, _) =
            register_active_agent_turn(&state, "ws1", "s1", "t1", false).unwrap();
        let second_error = match register_active_agent_turn(&state, "ws1", "s1", "t2", false) {
            Ok(_) => panic!("同 session 不得双活"),
            Err(error) => error,
        };
        assert_eq!(second_error.code, "AG_TURN_ACTIVE");
        let (abort_handle, _abort_registration) = futures_util::future::AbortHandle::new_pair();
        install_agent_turn_abort_handle(&state, "s1", "t1", abort_handle).unwrap();
        assert_eq!(cancel_active_agent_turn(&state, "s1", "t1").unwrap(), None);
        assert!(cancelled.load(Ordering::Acquire));

        drop(lease);
        let (_next, _, _) = register_active_agent_turn(&state, "ws1", "s1", "t2", false).unwrap();
    }

    // covers: BR-AG-STATE-001
    #[test]
    fn cancel_rejects_wrong_or_missing_turn() {
        let state = state();
        let (_lease, _, _) = register_active_agent_turn(&state, "ws1", "s1", "t1", false).unwrap();
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

    // covers: BR-AG-STATE-001
    // covers: BR-SYS-STATE-007
    #[test]
    fn clarification_resume_claims_same_turn_once_and_rejects_identity_drift() {
        let state = state();
        let (lease, _, _) = register_active_agent_turn(&state, "ws1", "s1", "t1", false).unwrap();
        park_agent_turn_for_clarification(&state, "s1", "t1", Some("task-1")).unwrap();
        drop(lease);

        assert_eq!(
            error_code(register_active_agent_turn(&state, "ws1", "s2", "t1", true)),
            "AG_CLARIFICATION_NOT_AWAITING"
        );
        assert_eq!(
            error_code(register_active_agent_turn(
                &state, "ws1", "s1", "wrong", true
            )),
            "AG_TURN_MISMATCH"
        );
        assert_eq!(
            error_code(register_active_agent_turn(&state, "ws2", "s1", "t1", true)),
            "AG_CLARIFICATION_WORKSPACE_MISMATCH"
        );

        let (resumed_lease, _, task_run_id) =
            register_active_agent_turn(&state, "ws1", "s1", "t1", true).unwrap();
        assert_eq!(task_run_id.as_deref(), Some("task-1"));
        assert_eq!(
            error_code(register_active_agent_turn(&state, "ws1", "s1", "t1", true)),
            "AG_CLARIFICATION_NOT_AWAITING"
        );
        drop(resumed_lease);
        assert_eq!(
            error_code(register_active_agent_turn(&state, "ws1", "s1", "t1", true)),
            "AG_CLARIFICATION_NOT_AWAITING"
        );
    }

    // covers: BR-AG-STATE-001
    // covers: BR-SYS-STATE-007
    // covers: BR-SYS-STATE-008
    #[test]
    fn simple_clarification_resumes_same_turn_without_task_run() {
        let state = state();
        let (lease, _, _) = register_active_agent_turn(&state, "ws1", "s1", "t1", false).unwrap();
        park_agent_turn_for_clarification(&state, "s1", "t1", None).unwrap();
        drop(lease);

        let (resumed_lease, _, task_run_id) =
            register_active_agent_turn(&state, "ws1", "s1", "t1", true).unwrap();
        assert_eq!(task_run_id, None);
        drop(resumed_lease);
    }

    // covers: BR-AG-STATE-001
    // covers: BR-SYS-STATE-007
    #[test]
    fn cancelling_parked_clarification_removes_resume_identity_and_returns_task_run() {
        let state = state();
        let (lease, _, _) = register_active_agent_turn(&state, "ws1", "s1", "t1", false).unwrap();
        park_agent_turn_for_clarification(&state, "s1", "t1", Some("task-1")).unwrap();
        drop(lease);

        assert_eq!(
            cancel_active_agent_turn(&state, "s1", "t1").unwrap(),
            Some("task-1".to_string())
        );
        assert_eq!(
            error_code(register_active_agent_turn(&state, "ws1", "s1", "t1", true)),
            "AG_CLARIFICATION_NOT_AWAITING"
        );
    }
}
