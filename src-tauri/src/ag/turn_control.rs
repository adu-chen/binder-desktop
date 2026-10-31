//! AgentTurn 单活注册、取消控制和预览回执 command。

use crate::ipc::IpcError;
use crate::{ag, workspace, AppState};
use ag::target_set::ExplicitEditTargetSet;
#[cfg(test)]
use rusqlite::Connection;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
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
    decision_request_id: Option<String>,
    edit_target_set: ExplicitEditTargetSet,
    runtime_checkpoint: Option<AgentTurnRuntimeCheckpoint>,
    model_binding: Option<(String, String)>,
    effects_in_flight: Arc<AtomicUsize>,
    runtime_returned: bool,
}

#[derive(Debug, Clone)]
pub(super) struct AgentTurnRuntimeCheckpoint {
    pub(super) intent: ag::context::IntentResultV2,
    pub(super) initial_prompt: String,
    pub(super) continuation_messages: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActiveAgentTurnPhase {
    Running,
    AwaitingClarification,
    AwaitingDecision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AgentTurnResumeKind {
    New,
    Clarification,
    Decision,
}

pub(super) struct ActiveAgentTurnLease {
    active_turns: Arc<Mutex<HashMap<String, ActiveAgentTurn>>>,
    session_id: String,
    turn_id: String,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct CancelActiveTurnOutcome {
    parked_source: Option<ag::turn::TurnState>,
    task_run_id: Option<String>,
}

impl Drop for ActiveAgentTurnLease {
    fn drop(&mut self) {
        if let Ok(mut turns) = self.active_turns.lock() {
            if let Some(turn) = turns.get_mut(&self.session_id).filter(|turn| {
                turn.turn_id == self.turn_id && turn.phase == ActiveAgentTurnPhase::Running
            }) {
                turn.runtime_returned = true;
            }
            let should_remove = turns.get(&self.session_id).is_some_and(|turn| {
                turn.turn_id == self.turn_id
                    && turn.phase == ActiveAgentTurnPhase::Running
                    && turn.effects_in_flight.load(Ordering::Acquire) == 0
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
 * term_ref: TERM-AG-001, TERM-AG-015, TERM-AG-019
 * boundary: in=AgentTurnStartIdentity(sessionId,workspaceId,turnId,resumeKind) | out=ExclusiveActiveAgentTurnLeaseOptionalOriginalTaskRunAndExplicitEditTargetSet(rejectsWrongSessionWorkspaceTurnRepeatedOrTerminalResume) | consumer=BR-AG-STATE-001,BR-SYS-STATE-007
 */
pub(super) fn register_active_agent_turn(
    state: &AppState,
    workspace_id: &str,
    session_id: &str,
    turn_id: &str,
    resume_kind: AgentTurnResumeKind,
) -> Result<
    (
        ActiveAgentTurnLease,
        Arc<AtomicBool>,
        Option<String>,
        ExplicitEditTargetSet,
        Option<AgentTurnRuntimeCheckpoint>,
    ),
    IpcError,
> {
    let mut turns = state
        .active_agent_turns
        .lock()
        .map_err(|_| IpcError::new("AG_TURN_REGISTRY_BUSY", "Agent 回合注册表繁忙", true, false))?;
    if resume_kind != AgentTurnResumeKind::New {
        let active = turns.get_mut(session_id).ok_or_else(|| {
            IpcError::new(
                "AG_TURN_NOT_AWAITING",
                "当前会话没有可恢复的等待回合",
                false,
                false,
            )
        })?;
        if active.turn_id != turn_id {
            return Err(IpcError::new(
                "AG_TURN_MISMATCH",
                "恢复请求不属于当前等待回合",
                false,
                false,
            ));
        }
        if active.workspace_id != workspace_id {
            return Err(IpcError::new(
                "AG_TURN_RESUME_WORKSPACE_MISMATCH",
                "等待回合不属于当前 workspace",
                false,
                false,
            ));
        }
        let expected_phase = match resume_kind {
            AgentTurnResumeKind::Clarification => ActiveAgentTurnPhase::AwaitingClarification,
            AgentTurnResumeKind::Decision => ActiveAgentTurnPhase::AwaitingDecision,
            AgentTurnResumeKind::New => unreachable!(),
        };
        if active.phase != expected_phase {
            return Err(IpcError::new(
                "AG_TURN_RESUME_NOT_AWAITING",
                "AgentTurn 当前不处于对应等待态，禁止错类、重复或终态重入",
                false,
                false,
            ));
        }
        if active.cancelled.load(Ordering::Acquire) {
            return Err(IpcError::new(
                "AG_TURN_CANCELLED",
                "等待回合已取消",
                false,
                false,
            ));
        }
        let task_run_id = active.task_run_id.clone();
        let edit_target_set = active.edit_target_set.clone();
        let runtime_checkpoint = active.runtime_checkpoint.take();
        active.decision_request_id = None;
        active.phase = ActiveAgentTurnPhase::Running;
        active.runtime_returned = false;
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
            edit_target_set,
            runtime_checkpoint,
        ));
    }

    if let Some(active) = turns.get(session_id) {
        if active.effects_in_flight.load(Ordering::Acquire) != 0 {
            return Err(IpcError::new(
                "AG_EFFECT_PENDING",
                "原回合能力仍在结算；请查询执行事实，禁止重放或接管写操作",
                true,
                false,
            ));
        }
        // 已发送取消信号但旧 command 尚未释放 lease 时，允许新 turn_id 接管；未取消活动态与等待态
        // 均须显式失败，不能把普通发送误当恢复。
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
            decision_request_id: None,
            edit_target_set: ExplicitEditTargetSet::default(),
            runtime_checkpoint: None,
            model_binding: None,
            effects_in_flight: Arc::new(AtomicUsize::new(0)),
            runtime_returned: false,
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
        ExplicitEditTargetSet::default(),
        None,
    ))
}

/// The model is chosen once for a new turn. A resumed turn always uses its recorded choice,
/// irrespective of omitted or changed UI preferences.
pub(super) fn model_binding_for_resume(
    state: &AppState,
    workspace_id: &str,
    session_id: &str,
    turn_id: &str,
    requested_provider: Option<&str>,
    requested_model: Option<&str>,
) -> Result<(String, String), IpcError> {
    let turns = state
        .active_agent_turns
        .lock()
        .map_err(|_| IpcError::new("AG_TURN_REGISTRY_BUSY", "Agent 回合注册表繁忙", true, false))?;
    let active = turns
        .get(session_id)
        .filter(|active| active.workspace_id == workspace_id && active.turn_id == turn_id)
        .ok_or_else(|| IpcError::new("AG_TURN_MISMATCH", "恢复请求不属于原回合", false, false))?;
    let bound = active.model_binding.clone().ok_or_else(|| {
        IpcError::new(
            "AG_MODEL_BINDING_MISSING",
            "原回合缺少固定模型，禁止默认回退",
            false,
            false,
        )
    })?;
    match (requested_provider, requested_model) {
        (None, None) => Ok(bound),
        (Some(provider), Some(model)) if provider == bound.0 && model == bound.1 => Ok(bound),
        _ => Err(IpcError::new(
            "AG_MODEL_BINDING_MISMATCH",
            "续行模型与原回合不一致，禁止切换；请核对原回合状态",
            false,
            false,
        )),
    }
}

pub(super) fn bind_new_agent_turn_model(
    state: &AppState,
    session_id: &str,
    turn_id: &str,
    provider: &str,
    model: &str,
) -> Result<(), IpcError> {
    let mut turns = state
        .active_agent_turns
        .lock()
        .map_err(|_| IpcError::new("AG_TURN_REGISTRY_BUSY", "Agent 回合注册表繁忙", true, false))?;
    let active = turns
        .get_mut(session_id)
        .filter(|active| active.turn_id == turn_id)
        .ok_or_else(|| IpcError::new("AG_TURN_NOT_ACTIVE", "Agent 回合已不再活动", false, false))?;
    if active.model_binding.is_some() {
        return Err(IpcError::new(
            "AG_MODEL_BINDING_CONFLICT",
            "原回合模型不可重新绑定",
            false,
            false,
        ));
    }
    active.model_binding = Some((provider.to_string(), model.to_string()));
    Ok(())
}

/**
 * @GOV
 * codes:    BR-SYS-STATE-007-EFFECT-AG-AG-TURN-034, BR-AG-STATE-001-EFFECT-AG-AG-TURN-034
 * type:     EFFECT
 * chain:    AG-TURN
 * rules:    BR-SYS-STATE-007, BR-AG-STATE-001
 * term_ref: TERM-AG-001, TERM-AG-015
 * boundary: in=RunningAgentTurnWithBlockingIntentAndOptionalTaskRunIdentity | out=AwaitingClarificationAgentTurnIdentity(retainedAfterCommandReturnAndClaimableExactlyOnce) | does_not=callProviderOrToolOrWriteEditorOrCancelTurn | consumer=BR-AG-STATE-001,BR-SYS-STATE-007
 */
pub(super) fn park_agent_turn_for_clarification(
    state: &AppState,
    session_id: &str,
    turn_id: &str,
    task_run_id: Option<&str>,
    checkpoint: AgentTurnRuntimeCheckpoint,
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
    active.decision_request_id = None;
    active.runtime_checkpoint = Some(checkpoint);
    Ok(())
}

pub(super) fn park_agent_turn_for_decision(
    state: &AppState,
    session_id: &str,
    turn_id: &str,
    task_run_id: &str,
    decision_request_id: &str,
    checkpoint: AgentTurnRuntimeCheckpoint,
) -> Result<(), IpcError> {
    let mut turns = state
        .active_agent_turns
        .lock()
        .map_err(|_| IpcError::new("AG_TURN_REGISTRY_BUSY", "Agent 回合注册表繁忙", true, false))?;
    let active = turns
        .get_mut(session_id)
        .ok_or_else(|| IpcError::new("AG_TURN_NOT_ACTIVE", "Agent 回合已不再活动", false, false))?;
    if active.turn_id != turn_id || active.phase != ActiveAgentTurnPhase::Running {
        return Err(IpcError::new(
            "AG_TURN_MISMATCH",
            "裁决等待身份与当前活动回合不匹配",
            false,
            false,
        ));
    }
    if active.cancelled.load(Ordering::Acquire) {
        return Err(IpcError::new(
            "AG_TURN_CANCELLED",
            "已取消回合不能进入裁决等待",
            false,
            false,
        ));
    }
    active.phase = ActiveAgentTurnPhase::AwaitingDecision;
    active.abort_handle = None;
    active.task_run_id = Some(task_run_id.to_string());
    active.decision_request_id = Some(decision_request_id.to_string());
    active.runtime_checkpoint = Some(checkpoint);
    Ok(())
}

/// DecisionRequest 的具体身份必须在认领等待态之前校验。失败不改变 phase/checkpoint，避免错误重放
/// 消耗合法的同回合恢复权。
pub(crate) fn validate_agent_turn_decision_resume(
    state: &AppState,
    session_id: &str,
    turn_id: &str,
    decision_request_id: &str,
) -> Result<(), IpcError> {
    let turns = state
        .active_agent_turns
        .lock()
        .map_err(|_| IpcError::new("AG_TURN_REGISTRY_BUSY", "Agent 回合注册表繁忙", true, false))?;
    let active = turns.get(session_id).ok_or_else(|| {
        IpcError::new(
            "AG_TURN_NOT_AWAITING",
            "当前会话没有可恢复的裁决等待回合",
            false,
            false,
        )
    })?;
    if active.turn_id != turn_id
        || active.phase != ActiveAgentTurnPhase::AwaitingDecision
        || active.decision_request_id.as_deref() != Some(decision_request_id)
    {
        return Err(IpcError::new(
            "AG_TURN_DECISION_MISMATCH",
            "裁决恢复请求与当前 AgentTurn 正在等待的 DecisionRequest 不匹配",
            false,
            false,
        ));
    }
    if active.cancelled.load(Ordering::Acquire) {
        return Err(IpcError::new(
            "AG_TURN_CANCELLED",
            "裁决等待回合已取消",
            false,
            false,
        ));
    }
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

/**
 * @GOV
 * codes:    BR-AG-DATA-003-EFFECT-AG-AG-TURN-036, BR-SYS-STATE-007-EFFECT-AG-AG-TURN-036
 * type:     EFFECT
 * chain:    AG-TURN
 * rules:    BR-AG-DATA-003, BR-SYS-STATE-007
 * term_ref: TERM-AG-001, TERM-AG-019
 * boundary: in=ActiveAgentTurnAndMonotonicExplicitEditTargetSet | out=SameAgentTurnWithExpandedTargets(rejectsRemovalOrIdentityDrift) | effect=memory | does_not=removeTargetsOrRewriteTurnIdentityOrExecuteEdit | consumer=BR-AG-DATA-003,BR-SYS-STATE-007
 */
pub(super) fn bind_agent_turn_edit_target_set(
    state: &AppState,
    session_id: &str,
    turn_id: &str,
    targets: &ExplicitEditTargetSet,
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
            "ExplicitEditTargetSet 与当前活动 AgentTurn 不匹配",
            false,
            false,
        ));
    }
    if active.cancelled.load(Ordering::Acquire) {
        return Err(IpcError::new(
            "AG_TURN_CANCELLED",
            "已取消的 AgentTurn 不能更新 ExplicitEditTargetSet",
            false,
            false,
        ));
    }
    if !targets.is_superset_of(&active.edit_target_set) {
        return Err(IpcError::new(
            "AG_EDIT_TARGET_SET_SHRINK",
            "同一 AgentTurn 的 ExplicitEditTargetSet 只能单调追加",
            false,
            false,
        ));
    }
    active.edit_target_set = targets.clone();
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

/// Read-only asynchronous boundaries share the active turn's cancellation handle.
/// Dropping this future stops waiting; it never grants permission or retries effects.
pub(super) async fn await_turn_operation<T>(
    state: &AppState,
    session_id: &str,
    turn_id: &str,
    operation: impl std::future::Future<Output = Result<T, IpcError>>,
) -> Result<T, IpcError> {
    let (handle, registration) = futures_util::future::AbortHandle::new_pair();
    install_agent_turn_abort_handle(state, session_id, turn_id, handle)?;
    futures_util::future::Abortable::new(operation, registration)
        .await
        .map_err(|_| IpcError::new("AG_TURN_CANCELLED", "当前回合已取消。", false, false))?
}

pub(super) struct AgentTurnEffectLease {
    turns: Arc<Mutex<HashMap<String, ActiveAgentTurn>>>,
    session_id: String,
    turn_id: String,
    count: Arc<AtomicUsize>,
}

impl Drop for AgentTurnEffectLease {
    fn drop(&mut self) {
        self.count.fetch_sub(1, Ordering::AcqRel);
        if let Ok(mut turns) = self.turns.lock() {
            let remove = turns.get(&self.session_id).is_some_and(|turn| {
                turn.turn_id == self.turn_id
                    && (turn.cancelled.load(Ordering::Acquire) || turn.runtime_returned)
                    && turn.effects_in_flight.load(Ordering::Acquire) == 0
            });
            if remove {
                turns.remove(&self.session_id);
            }
        }
    }
}

pub(super) fn lock_agent_turn_effect(
    state: &AppState,
    session_id: &str,
    turn_id: &str,
) -> Result<AgentTurnEffectLease, IpcError> {
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
            "Agent 回合已取消，禁止产生新的提案或任务级控制事实",
            false,
            false,
        ));
    }
    active.effects_in_flight.fetch_add(1, Ordering::AcqRel);
    Ok(AgentTurnEffectLease {
        turns: Arc::clone(&state.active_agent_turns),
        session_id: session_id.to_string(),
        turn_id: turn_id.to_string(),
        count: Arc::clone(&active.effects_in_flight),
    })
}

/**
 * @GOV
 * codes:    BR-AG-STATE-001-API-AG-AG-TURN-019, BR-AG-PERSIST-001-API-AG-AG-TURN-019, BR-SYS-STATE-007-API-AG-AG-TURN-019
 * type:     API
 * chain:    AG-TURN
 * rules:    BR-AG-STATE-001, BR-AG-PERSIST-001, BR-SYS-STATE-007
 * term_ref: TERM-AG-001, TERM-AG-022
 * boundary: in=AgentTurnCancellationIdentity(sessionId,turnId) | out=CancelledAgentTurnWithPersistedTerminalEvent(abortedGatewayStreamOrClosedClarificationWaitAndNoFurtherToolStart) | consumer=BR-AG-STATE-001,BR-AG-PERSIST-001,BR-SYS-STATE-007
 */
#[tauri::command]
pub(crate) fn ag_cancel_turn(
    session_id: String,
    turn_id: String,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), IpcError> {
    let outcome = cancel_active_agent_turn(&state, &session_id, &turn_id)?;
    if let Some(source) = outcome.parked_source {
        super::turn_events::persist_parked_ag_turn_cancelled(&state, &app, &turn_id, source)?;
        if let Some(task_run_id) = outcome.task_run_id {
            if let Ok(conn) = state.db.lock() {
                super::task_runtime::settle_task_run(
                    &conn,
                    &task_run_id,
                    &turn_id,
                    ag::task::TurnOutcome::Cancelled,
                );
            }
        }
        finish_cancelled_parked_turn(&state, &session_id, &turn_id)?;
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
 * boundary: in=EditorAppliedLogicalStatePreviewIdentity | out=PreviewContinuationCheckpointAcknowledged(noModelCallNoDiffDecisionNoDiskWrite) | consumer=BR-AG-DATA-001,BR-SYS-STATE-008
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
    commit_preview_ack(&conn, &checkpoint)?;
    let awaiting_save: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM create_document_bundles b JOIN workspace_operation_plans p ON p.plan_id=b.plan_id WHERE b.initial_diff_id=?1 AND p.status='awaiting_content')", [&checkpoint.diff_id], |row|row.get(0))
        .map_err(|error| IpcError::new("AG_PREVIEW_ACK_DB_ERROR", &error.to_string(), true, false))?;
    Ok(
        serde_json::json!({ "task_run_id": task_run_id, "ready": true, "awaiting_save":awaiting_save }),
    )
}

// 同一数据库事务提交检查点和新建文档拒绝保护；后者失败不得留下不可重提的半份 ack。
pub(super) fn commit_preview_ack(
    conn: &rusqlite::Connection,
    checkpoint: &ag::task::PreviewCheckpoint,
) -> Result<(), IpcError> {
    let tx = conn.unchecked_transaction().map_err(|error| {
        IpcError::new("AG_PREVIEW_ACK_DB_ERROR", &error.to_string(), true, false)
    })?;
    ag::task::acknowledge_preview_checkpoint(&tx, checkpoint).map_err(|error| {
        IpcError::new(
            "AG_PREVIEW_ACK_REJECTED",
            &format!("预览回执未通过身份或状态校验：{error}"),
            false,
            false,
        )
    })?;
    workspace::operation::acknowledge_create_document_preview(
        &tx,
        &checkpoint.diff_id,
        checkpoint.session_version,
        &checkpoint.content_hash,
    )?;
    tx.commit()
        .map_err(|error| IpcError::new("AG_PREVIEW_ACK_DB_ERROR", &error.to_string(), true, false))
}

pub(super) fn cancel_active_agent_turn(
    state: &AppState,
    session_id: &str,
    turn_id: &str,
) -> Result<CancelActiveTurnOutcome, IpcError> {
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
    let parked_source = match active.phase {
        ActiveAgentTurnPhase::AwaitingClarification => {
            Some(ag::turn::TurnState::AwaitingClarification)
        }
        ActiveAgentTurnPhase::AwaitingDecision => Some(ag::turn::TurnState::AwaitingDecision),
        ActiveAgentTurnPhase::Running => None,
    };
    let task_run_id = parked_source
        .is_some()
        .then(|| active.task_run_id.clone())
        .flatten();
    if let Some(handle) = active.abort_handle.take() {
        handle.abort();
    }
    Ok(CancelActiveTurnOutcome {
        parked_source,
        task_run_id,
    })
}

pub(super) fn release_failed_parked_turn(
    state: &AppState,
    session_id: &str,
    turn_id: &str,
) -> Result<(), IpcError> {
    let mut turns = state
        .active_agent_turns
        .lock()
        .map_err(|_| IpcError::new("AG_TURN_REGISTRY_BUSY", "Agent回合注册表繁忙", true, false))?;
    if let Some(turn) = turns
        .get_mut(session_id)
        .filter(|turn| turn.turn_id == turn_id)
    {
        turn.cancelled.store(true, Ordering::Release);
        turn.runtime_returned = true;
        // Keep an in-flight lease registered until its real effect settles.
        turn.phase = ActiveAgentTurnPhase::Running;
        if let Some(abort) = turn.abort_handle.take() {
            abort.abort();
        }
    }
    if turns.get(session_id).is_some_and(|turn| {
        turn.turn_id == turn_id && turn.effects_in_flight.load(Ordering::Acquire) == 0
    }) {
        if let Some(turn) = turns.remove(session_id) {
            turn.cancelled.store(true, Ordering::Release);
            if let Some(abort) = turn.abort_handle {
                abort.abort();
            }
        }
    }
    Ok(())
}

fn finish_cancelled_parked_turn(
    state: &AppState,
    session_id: &str,
    turn_id: &str,
) -> Result<(), IpcError> {
    let mut turns = state
        .active_agent_turns
        .lock()
        .map_err(|_| IpcError::new("AG_TURN_REGISTRY_BUSY", "Agent 回合注册表繁忙", true, false))?;
    let should_remove = turns.get(session_id).is_some_and(|active| {
        active.turn_id == turn_id
            && active.phase != ActiveAgentTurnPhase::Running
            && active.cancelled.load(Ordering::Acquire)
    });
    if !should_remove {
        return Err(IpcError::new(
            "AG_TURN_MISMATCH",
            "已持久化取消终态，但等待身份已改变",
            false,
            false,
        ));
    }
    turns.remove(session_id);
    Ok(())
}

#[cfg(test)]
mod ag_execution_kernel_tests {
    use super::*;
    use crate::build_registry;

    // covers: BR-SYS-STATE-008, BR-AG-DATA-001
    #[test]
    fn preview_ack_rolls_back_checkpoint_when_bundle_storage_fails() {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::migrate(&mut conn).unwrap();
        conn.execute("INSERT INTO workspaces (workspace_id, root_path, is_active, opened_at) VALUES ('ws1','/w',1,0)", []).unwrap();
        let task_run_id = ag::task::begin_turn(&conn, "ws1", "s1", None).unwrap();
        conn.execute("INSERT INTO pending_diffs (diff_id, workspace_id, rel_path, origin_module, status, payload_ref, created_at) VALUES ('d1','ws1','a.txt','AG','pending','p1',1)", []).unwrap();
        ag::task::create_preview_checkpoint(&conn, &task_run_id, "ws1", "s1", "t1", "d1", "a.txt")
            .unwrap();
        let checkpoint = ag::task::PreviewCheckpoint {
            diff_id: "d1".into(),
            task_run_id: task_run_id.clone(),
            workspace_id: "ws1".into(),
            session_id: "s1".into(),
            turn_id: "t1".into(),
            rel_path: "a.txt".into(),
            session_version: 1,
            content_hash: "hash".into(),
        };
        // 隔离内存库注入存储故障；执行真实生产事务/检查点/组合查询，不替换被测逻辑。
        conn.execute(
            "ALTER TABLE create_document_bundles RENAME TO unavailable_bundles",
            [],
        )
        .unwrap();
        assert!(commit_preview_ack(&conn, &checkpoint).is_err());
        let status: String = conn
            .query_row(
                "SELECT status FROM task_preview_checkpoints WHERE diff_id='d1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(status, "awaiting_ack");
        assert!(
            ag::task::acknowledged_preview_checkpoint(&conn, &task_run_id, "ws1", "s1", "d1")
                .unwrap()
                .is_none()
        );
        conn.execute(
            "ALTER TABLE unavailable_bundles RENAME TO create_document_bundles",
            [],
        )
        .unwrap();
        commit_preview_ack(&conn, &checkpoint).unwrap();
        assert!(
            ag::task::acknowledged_preview_checkpoint(&conn, &task_run_id, "ws1", "s1", "d1")
                .unwrap()
                .is_some()
        );
        assert!(commit_preview_ack(&conn, &checkpoint).is_err());
    }

    fn state() -> AppState {
        AppState {
            db: std::sync::Arc::new(Mutex::new(Connection::open_in_memory().unwrap())),
            registry: build_registry(),
            assist_terminals: std::sync::Arc::new(Mutex::new(HashMap::new())),
            active_agent_turns: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn error_code<T>(result: Result<T, IpcError>) -> String {
        match result {
            Err(error) => error.code,
            Ok(_) => panic!("预期身份校验失败"),
        }
    }

    fn checkpoint() -> AgentTurnRuntimeCheckpoint {
        AgentTurnRuntimeCheckpoint {
            intent: ag::context::IntentResultV2::unresolved(),
            initial_prompt: "initial request".to_string(),
            continuation_messages: Vec::new(),
        }
    }

    // covers: BR-AG-DATA-007
    #[test]
    fn resumed_turn_keeps_initial_model_even_when_client_selection_changes() {
        let state = state();
        let (_lease, _, _, _, _) =
            register_active_agent_turn(&state, "ws1", "s1", "t1", AgentTurnResumeKind::New)
                .unwrap();
        bind_new_agent_turn_model(&state, "s1", "t1", "deepseek", "deepseek-v4-flash").unwrap();
        assert_eq!(
            model_binding_for_resume(&state, "ws1", "s1", "t1", None, None).unwrap(),
            ("deepseek".to_string(), "deepseek-v4-flash".to_string())
        );
        assert_eq!(
            error_code(bind_new_agent_turn_model(
                &state,
                "s1",
                "t1",
                "deepseek",
                "deepseek-chat"
            )),
            "AG_MODEL_BINDING_CONFLICT"
        );
        assert_eq!(
            error_code(model_binding_for_resume(
                &state, "ws2", "s1", "t1", None, None
            )),
            "AG_TURN_MISMATCH"
        );
        assert_eq!(
            error_code(model_binding_for_resume(
                &state,
                "ws1",
                "s1",
                "t1",
                Some("deepseek"),
                Some("deepseek-chat")
            )),
            "AG_MODEL_BINDING_MISMATCH"
        );
    }

    // covers: BR-AG-STATE-001
    #[test]
    fn session_has_one_active_turn_and_cancel_reaches_backend_control() {
        let state = state();
        let (lease, cancelled, _, _, _) =
            register_active_agent_turn(&state, "ws1", "s1", "t1", AgentTurnResumeKind::New)
                .unwrap();
        let second_error =
            match register_active_agent_turn(&state, "ws1", "s1", "t2", AgentTurnResumeKind::New) {
                Ok(_) => panic!("同 session 不得双活"),
                Err(error) => error,
            };
        assert_eq!(second_error.code, "AG_TURN_ACTIVE");
        let (abort_handle, _abort_registration) = futures_util::future::AbortHandle::new_pair();
        install_agent_turn_abort_handle(&state, "s1", "t1", abort_handle).unwrap();
        assert_eq!(
            cancel_active_agent_turn(&state, "s1", "t1").unwrap(),
            CancelActiveTurnOutcome {
                parked_source: None,
                task_run_id: None,
            }
        );
        assert!(cancelled.load(Ordering::Acquire));

        drop(lease);
        let (_next, _, _, _, _) =
            register_active_agent_turn(&state, "ws1", "s1", "t2", AgentTurnResumeKind::New)
                .unwrap();
    }

    // covers: BR-AG-STATE-001, BR-SYS-STATE-007
    #[test]
    fn pending_turn_operation_cancels_before_or_during_wait_without_replay() {
        use std::future::Future;
        use std::task::{Context, Poll};
        for cancel_before_poll in [false, true] {
            let state = state();
            let (lease, _, _, _, _) =
                register_active_agent_turn(&state, "ws1", "s1", "t1", AgentTurnResumeKind::New)
                    .unwrap();
            let waker = futures_util::task::noop_waker();
            let mut context = Context::from_waker(&waker);
            let mut operation = Box::pin(await_turn_operation(
                &state,
                "s1",
                "t1",
                futures_util::future::pending::<Result<(), IpcError>>(),
            ));
            if !cancel_before_poll {
                assert!(operation.as_mut().poll(&mut context).is_pending());
            }
            cancel_active_agent_turn(&state, "s1", "t1").unwrap();
            match operation.as_mut().poll(&mut context) {
                Poll::Ready(Err(error)) => assert_eq!(error.code, "AG_TURN_CANCELLED"),
                _ => panic!("cancelled operation must stop waiting immediately"),
            }
            drop(operation);
            drop(lease);
            let (_next, _, _, _, _) =
                register_active_agent_turn(&state, "ws1", "s1", "t2", AgentTurnResumeKind::New)
                    .unwrap();
        }
    }

    // covers: BR-AG-STATE-001
    #[test]
    fn cancel_rejects_wrong_or_missing_turn() {
        let state = state();
        let (_lease, _, _, _, _) =
            register_active_agent_turn(&state, "ws1", "s1", "t1", AgentTurnResumeKind::New)
                .unwrap();
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
        let (lease, _, _, _, _) =
            register_active_agent_turn(&state, "ws1", "s1", "t1", AgentTurnResumeKind::New)
                .unwrap();
        park_agent_turn_for_clarification(&state, "s1", "t1", Some("task-1"), checkpoint())
            .unwrap();
        drop(lease);

        assert_eq!(
            error_code(register_active_agent_turn(
                &state,
                "ws1",
                "s2",
                "t1",
                AgentTurnResumeKind::Clarification
            )),
            "AG_TURN_NOT_AWAITING"
        );
        assert_eq!(
            error_code(register_active_agent_turn(
                &state,
                "ws1",
                "s1",
                "wrong",
                AgentTurnResumeKind::Clarification
            )),
            "AG_TURN_MISMATCH"
        );
        assert_eq!(
            error_code(register_active_agent_turn(
                &state,
                "ws2",
                "s1",
                "t1",
                AgentTurnResumeKind::Clarification
            )),
            "AG_TURN_RESUME_WORKSPACE_MISMATCH"
        );

        let (resumed_lease, _, task_run_id, _, resumed_checkpoint) = register_active_agent_turn(
            &state,
            "ws1",
            "s1",
            "t1",
            AgentTurnResumeKind::Clarification,
        )
        .unwrap();
        assert_eq!(task_run_id.as_deref(), Some("task-1"));
        assert!(resumed_checkpoint.is_some());
        assert_eq!(
            error_code(register_active_agent_turn(
                &state,
                "ws1",
                "s1",
                "t1",
                AgentTurnResumeKind::Clarification
            )),
            "AG_TURN_RESUME_NOT_AWAITING"
        );
        drop(resumed_lease);
        assert_eq!(
            error_code(register_active_agent_turn(
                &state,
                "ws1",
                "s1",
                "t1",
                AgentTurnResumeKind::Clarification
            )),
            "AG_TURN_NOT_AWAITING"
        );
    }

    // covers: BR-AG-STATE-001
    // covers: BR-SYS-STATE-007
    // covers: BR-SYS-STATE-008
    #[test]
    fn simple_clarification_resumes_same_turn_without_task_run() {
        let state = state();
        let (lease, _, _, _, _) =
            register_active_agent_turn(&state, "ws1", "s1", "t1", AgentTurnResumeKind::New)
                .unwrap();
        park_agent_turn_for_clarification(&state, "s1", "t1", None, checkpoint()).unwrap();
        drop(lease);

        let (resumed_lease, _, task_run_id, _, resumed_checkpoint) = register_active_agent_turn(
            &state,
            "ws1",
            "s1",
            "t1",
            AgentTurnResumeKind::Clarification,
        )
        .unwrap();
        assert_eq!(task_run_id, None);
        assert!(resumed_checkpoint.is_some());
        drop(resumed_lease);
    }

    // covers: BR-AG-DATA-003
    // covers: BR-SYS-STATE-007
    #[test]
    fn clarification_keeps_a_monotonic_edit_target_set_without_creating_task_run() {
        let state = state();
        let (lease, _, _, _, _) =
            register_active_agent_turn(&state, "ws1", "s1", "t1", AgentTurnResumeKind::New)
                .unwrap();
        let mut initial = ExplicitEditTargetSet::default();
        initial.insert_authorized(
            "a.md".to_string(),
            "current_document",
            Some("t1".to_string()),
        );
        bind_agent_turn_edit_target_set(&state, "s1", "t1", &initial).unwrap();
        park_agent_turn_for_clarification(&state, "s1", "t1", None, checkpoint()).unwrap();
        drop(lease);

        let (resumed_lease, _, task_run_id, resumed_targets, resumed_checkpoint) =
            register_active_agent_turn(
                &state,
                "ws1",
                "s1",
                "t1",
                AgentTurnResumeKind::Clarification,
            )
            .unwrap();
        assert_eq!(task_run_id, None);
        assert_eq!(resumed_targets.paths(), vec!["a.md"]);
        assert!(resumed_checkpoint.is_some());

        let empty = ExplicitEditTargetSet::default();
        let error = bind_agent_turn_edit_target_set(&state, "s1", "t1", &empty).unwrap_err();
        assert_eq!(error.code, "AG_EDIT_TARGET_SET_SHRINK");
        drop(resumed_lease);
    }

    // covers: BR-AG-STATE-001
    // covers: BR-SYS-STATE-007
    #[test]
    fn cancelling_parked_clarification_removes_resume_identity_and_returns_task_run() {
        let state = state();
        let (lease, _, _, _, _) =
            register_active_agent_turn(&state, "ws1", "s1", "t1", AgentTurnResumeKind::New)
                .unwrap();
        park_agent_turn_for_clarification(&state, "s1", "t1", Some("task-1"), checkpoint())
            .unwrap();
        drop(lease);

        assert_eq!(
            cancel_active_agent_turn(&state, "s1", "t1").unwrap(),
            CancelActiveTurnOutcome {
                parked_source: Some(ag::turn::TurnState::AwaitingClarification),
                task_run_id: Some("task-1".to_string()),
            }
        );
        finish_cancelled_parked_turn(&state, "s1", "t1").unwrap();
        assert_eq!(
            error_code(register_active_agent_turn(
                &state,
                "ws1",
                "s1",
                "t1",
                AgentTurnResumeKind::Clarification
            )),
            "AG_TURN_NOT_AWAITING"
        );
    }

    // covers: BR-AG-STATE-001, BR-SYS-STATE-007
    #[test]
    fn cancelling_simple_parked_clarification_also_removes_resume_identity() {
        let state = state();
        let (lease, _, _, _, _) =
            register_active_agent_turn(&state, "ws1", "s1", "t1", AgentTurnResumeKind::New)
                .unwrap();
        park_agent_turn_for_clarification(&state, "s1", "t1", None, checkpoint()).unwrap();
        drop(lease);

        assert_eq!(
            cancel_active_agent_turn(&state, "s1", "t1").unwrap(),
            CancelActiveTurnOutcome {
                parked_source: Some(ag::turn::TurnState::AwaitingClarification),
                task_run_id: None,
            }
        );
        finish_cancelled_parked_turn(&state, "s1", "t1").unwrap();
        assert_eq!(
            error_code(register_active_agent_turn(
                &state,
                "ws1",
                "s1",
                "t1",
                AgentTurnResumeKind::Clarification
            )),
            "AG_TURN_NOT_AWAITING"
        );
    }

    // covers: BR-AG-STATE-001, BR-SYS-STATE-007, BR-SYS-STATE-008
    #[test]
    fn decision_resume_claims_the_same_turn_and_checkpoint_exactly_once() {
        let state = state();
        let (lease, _, _, _, _) =
            register_active_agent_turn(&state, "ws1", "s1", "t1", AgentTurnResumeKind::New)
                .unwrap();
        let expected = checkpoint();
        park_agent_turn_for_decision(&state, "s1", "t1", "task-1", "decision-1", expected.clone())
            .unwrap();
        drop(lease);

        assert_eq!(
            validate_agent_turn_decision_resume(&state, "s1", "t1", "decision-old")
                .unwrap_err()
                .code,
            "AG_TURN_DECISION_MISMATCH"
        );
        validate_agent_turn_decision_resume(&state, "s1", "t1", "decision-1").unwrap();

        assert_eq!(
            error_code(register_active_agent_turn(
                &state,
                "ws1",
                "s1",
                "t1",
                AgentTurnResumeKind::Clarification,
            )),
            "AG_TURN_RESUME_NOT_AWAITING"
        );
        let (resumed_lease, _, task_run_id, _, resumed_checkpoint) =
            register_active_agent_turn(&state, "ws1", "s1", "t1", AgentTurnResumeKind::Decision)
                .unwrap();
        assert_eq!(task_run_id.as_deref(), Some("task-1"));
        assert_eq!(resumed_checkpoint.unwrap().intent, expected.intent);
        assert_eq!(
            error_code(register_active_agent_turn(
                &state,
                "ws1",
                "s1",
                "t1",
                AgentTurnResumeKind::Decision,
            )),
            "AG_TURN_RESUME_NOT_AWAITING"
        );
        drop(resumed_lease);
    }

    // covers: BR-AG-STATE-001, BR-SYS-STATE-007
    #[test]
    fn cancelling_a_parked_decision_freezes_its_resume_identity() {
        let state = state();
        let (lease, _, _, _, _) =
            register_active_agent_turn(&state, "ws1", "s1", "t1", AgentTurnResumeKind::New)
                .unwrap();
        park_agent_turn_for_decision(&state, "s1", "t1", "task-1", "decision-1", checkpoint())
            .unwrap();
        drop(lease);

        assert_eq!(
            cancel_active_agent_turn(&state, "s1", "t1").unwrap(),
            CancelActiveTurnOutcome {
                parked_source: Some(ag::turn::TurnState::AwaitingDecision),
                task_run_id: Some("task-1".to_string()),
            }
        );
        finish_cancelled_parked_turn(&state, "s1", "t1").unwrap();
        assert_eq!(
            error_code(register_active_agent_turn(
                &state,
                "ws1",
                "s1",
                "t1",
                AgentTurnResumeKind::Decision,
            )),
            "AG_TURN_NOT_AWAITING"
        );
    }

    // covers: BR-AG-STATE-001, BR-SYS-STATE-007
    #[test]
    fn cancellation_signals_without_waiting_for_effect_but_blocks_takeover_until_settled() {
        let state = state();
        let (_lease, cancelled, _, _, _) =
            register_active_agent_turn(&state, "ws1", "s1", "t1", AgentTurnResumeKind::New)
                .unwrap();
        let effect_guard = lock_agent_turn_effect(&state, "s1", "t1").unwrap();

        std::thread::scope(|scope| {
            let (started_tx, started_rx) = std::sync::mpsc::channel();
            let (done_tx, done_rx) = std::sync::mpsc::channel();
            let state_ref = &state;
            scope.spawn(move || {
                started_tx.send(()).unwrap();
                let result = cancel_active_agent_turn(state_ref, "s1", "t1");
                done_tx.send(result).unwrap();
            });
            started_rx.recv().unwrap();
            assert_eq!(
                done_rx
                    .recv_timeout(std::time::Duration::from_millis(100))
                    .unwrap()
                    .unwrap(),
                CancelActiveTurnOutcome {
                    parked_source: None,
                    task_run_id: None,
                }
            );
            assert!(cancelled.load(Ordering::Acquire));
            assert_eq!(
                error_code(register_active_agent_turn(
                    &state,
                    "ws1",
                    "s1",
                    "t2",
                    AgentTurnResumeKind::New
                )),
                "AG_EFFECT_PENDING"
            );
            drop(effect_guard);
            let (new_lease, new_cancelled, _, _, _) =
                register_active_agent_turn(&state, "ws1", "s1", "t2", AgentTurnResumeKind::New)
                    .unwrap();
            assert!(!new_cancelled.load(Ordering::Acquire));
            drop(new_lease);
        });
        assert!(cancelled.load(Ordering::Acquire));
    }
}
