//! AgentTurn 的模型续轮、CapabilityGrant 校验、工具执行、结果注入与终态结算。

use super::edit_runtime::ag_propose_edit;
use super::prompt_runtime::{
    build_prompt_audit_record, current_doc_edit_requires_prior_read, is_current_doc_read_tool,
    prompt_audit_enabled, provider_messages_for_model, FinalPromptAuditLog,
};
use super::task_runtime::{edit_intent_has_terminal_tool_result, settle_optional_task_run};
use super::turn_control::{
    bind_agent_turn_task_run, install_agent_turn_abort_handle, lock_agent_turn_effect,
    park_agent_turn_for_clarification, park_agent_turn_for_decision, AgentTurnRuntimeCheckpoint,
};
use super::turn_events::{
    emit_ag01, emit_ag02, emit_ag_turn_cancelled, emit_ag_turn_denied, emit_ag_turn_done,
    emit_ag_turn_error, emit_ag_turn_error_from, settle_and_emit_ag_turn_error,
    settle_and_emit_ag_turn_error_from, sm_error, TurnEventEmitter,
};
use super::turn_prompt::{prepare_turn_prompt, PreparedTurnPrompt};
use crate::ag;
use crate::ipc::{AgTurnEvent, AgTurnEventPayload, CommandRequest, IpcError};
use crate::workspace::routes::route_plan_workspace_operation;
use crate::{ent, AppState};
use std::collections::BTreeSet;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

fn apply_model_control(
    emitter: &TurnEventEmitter<'_>,
    latch: &ag::turn::TerminalLatch,
    current: &ag::context::IntentResultV2,
    control: &ag::model_output::ModelTurnControl,
) -> Result<ag::context::IntentResultV2, IpcError> {
    let source_event_seq = emitter
        .record_internal(
            "model_semantics_received",
            serde_json::json!({
                "nextAction": control.next_action,
                "intentType": control.intent.intent_type,
            }),
        )?
        .ok_or_else(|| {
            IpcError::new(
                "AG_TURN_ALREADY_TERMINAL",
                "模型语义到达时 AgentTurn 已终结",
                false,
                false,
            )
        })?;
    let next = control.intent_result(current.revision + 1, source_event_seq);
    emitter.emit(
        latch,
        AgTurnEventPayload::IntentUpdated {
            revision: next.revision,
            projection: serde_json::to_value(&next).map_err(|error| {
                IpcError::new(
                    "AG_MODEL_CONTROL_INVALID",
                    &format!("IntentResultV2 无法投影：{error}"),
                    false,
                    false,
                )
            })?,
        },
    )?;
    emitter.emit(
        latch,
        AgTurnEventPayload::RouteUpdated {
            revision: next.revision,
            summary: control.route.summary.clone(),
            steps: control
                .route
                .steps
                .iter()
                .cloned()
                .map(serde_json::Value::String)
                .collect(),
        },
    )?;
    Ok(next)
}

fn ensure_route_task_projection(
    state: &AppState,
    emitter: &TurnEventEmitter<'_>,
    session_id: &str,
    workspace_id: &str,
    turn_id: &str,
    explicit_edit_target_set: &ag::target_set::ExplicitEditTargetSet,
    task_run_id: &mut Option<String>,
    control: &ag::model_output::ModelTurnControl,
) -> Result<(), IpcError> {
    if task_run_id.is_some()
        || !ag::task::route_requires_task_projection(
            &control.route.steps,
            &control.intent.confirmation_points,
            &explicit_edit_target_set.paths(),
        )
    {
        return Ok(());
    }
    let created = {
        let conn = state.db.lock().map_err(|_| {
            IpcError::new(
                "WS_DB_BUSY",
                "数据库连接繁忙，无法建立任务投影身份",
                true,
                false,
            )
        })?;
        let created = ag::task::begin_for_model_route(
            &conn,
            workspace_id,
            session_id,
            &control.intent.goal,
            &control.route.steps,
            &control.intent.confirmation_points,
            &explicit_edit_target_set.paths(),
        )
        .map_err(|error| IpcError::new("AG_TASK_DB_ERROR", &error.to_string(), true, false))?
        .ok_or_else(|| {
            IpcError::new(
                "AG_TASK_PROJECTION_DRIFT",
                "复杂路线判据与 TaskRun 创建边界不一致",
                false,
                false,
            )
        })?;
        ag::task::link_turn(&conn, turn_id, &created)
            .map_err(|error| IpcError::new("AG_TASK_DB_ERROR", &error.to_string(), true, false))?;
        *task_run_id = Some(created.clone());
        ag::target_set::persist_for_task_run(&conn, &created, explicit_edit_target_set)
            .map_err(|error| IpcError::new("AG_TASK_DB_ERROR", &error.to_string(), true, false))?;
        created
    };
    bind_agent_turn_task_run(state, session_id, turn_id, &created)?;
    emitter.record_internal(
        "task_projected",
        serde_json::json!({
            "taskRunId": &created,
            "goal": &control.intent.goal,
            "reason": "complex_route",
        }),
    )?;
    Ok(())
}

fn terminal_tool_is_missing(
    required_terminal_tool: Option<&str>,
    proposed_diff: bool,
    edit_blocked_reported: bool,
) -> bool {
    match required_terminal_tool {
        Some("propose_edit") => {
            !edit_intent_has_terminal_tool_result(proposed_diff, edit_blocked_reported)
        }
        // 成功的资源计划会在 DecisionRequest 事件分支提前结束；仍在模型循环中即尚未满足。
        Some("plan_workspace_operation") => true,
        _ => false,
    }
}

fn required_terminal_tool_error(
    required_terminal_tool: Option<&str>,
    any_tool_called: bool,
) -> IpcError {
    if required_terminal_tool == Some("plan_workspace_operation") {
        return IpcError::new(
            "AG_WORKSPACE_PLAN_TOOL_REQUIRED",
            "本轮是工作区资源请求，但工具循环结束前未生成 WorkspaceOperationPlan；请调用 plan_workspace_operation 生成待确认计划。工作区没有被修改。",
            true,
            false,
        );
    }
    IpcError::new(
        "AG_EDIT_TOOL_REQUIRED",
        if any_tool_called {
            "本轮是编辑请求，模型读取了文档但未生成修改建议；请基于已读内容直接调用 propose_edit 生成 DiffCard，或明确说明为何无法修改。文档没有被修改。"
        } else {
            "本轮是编辑请求，但模型未调用任何工具；请直接调用 propose_edit 生成修改建议（可先调用 read_doc/list_blocks 读取目标内容，再在同一轮继续调用 propose_edit）。文档没有被修改，请重试。"
        },
        true,
        false,
    )
}

fn requested_report_scope(
    intent: &ag::context::IntentResultV2,
    active_doc_path: Option<&str>,
) -> Vec<String> {
    let mut scope = BTreeSet::new();
    for candidate in &intent.target_candidates {
        if let Some(path) = candidate.rel_path.as_deref() {
            scope.insert(path.to_string());
        } else if matches!(candidate.kind.as_str(), "current_document" | "selection") {
            if let Some(path) = active_doc_path {
                scope.insert(path.to_string());
            }
        } else if candidate.kind == "workspace" {
            scope.insert(".".to_string());
        }
    }
    if scope.is_empty() {
        scope.insert(active_doc_path.unwrap_or(".").to_string());
    }
    scope.into_iter().collect()
}

/// Channel/补流只公开工具身份、目标路径和参数键，不复制查询正文、文档块或修改载荷。
fn safe_tool_params(params: &serde_json::Value) -> serde_json::Value {
    let Some(object) = params.as_object() else {
        return serde_json::json!({});
    };
    let mut keys = object.keys().cloned().collect::<Vec<_>>();
    keys.sort();
    let rel_path = object
        .get("relPath")
        .or_else(|| object.get("path"))
        .and_then(|value| value.as_str());
    serde_json::json!({
        "parameterKeys": keys,
        "relPath": rel_path,
    })
}

/// 模型继续推理仍接收本地完整结果；TurnEvent 安全投影只保存不会复制正文的执行摘要。
fn safe_tool_result(tool_name: &str, success: bool) -> String {
    if success {
        format!("{tool_name} 执行成功")
    } else {
        format!("{tool_name} 执行失败")
    }
}

fn tool_is_visible_in_manifest(tools: &serde_json::Value, tool_name: &str) -> bool {
    tools.as_array().is_some_and(|definitions| {
        definitions
            .iter()
            .any(|definition| definition["function"]["name"].as_str() == Some(tool_name))
    })
}

/**
 * @GOV
 * codes:    BR-AG-STATE-002-EFFECT-AG-AG-TOOL-038
 * type:     EFFECT
 * chain:    AG-TOOL
 * rules:    BR-AG-STATE-002
 * term_ref: TERM-AG-002
 * boundary: in=CurrentToolLoopStateAndToolLoopEvent | out=TransitionValidatedToolLoopStateBroadcast | effect=SM-AG-02-state_changed | delegate=ag::tool::transition | does_not=broadcastUnregisteredTransitionOrSettleAgentTurn
 */
fn advance_tool_loop(
    app: &tauri::AppHandle,
    turn_id: &str,
    state: &mut ag::tool::ToolLoopState,
    event: ag::tool::ToolLoopEvent,
) -> Result<ag::tool::ToolLoopState, IpcError> {
    let next = ag::tool::transition(*state, event).ok_or_else(|| {
        IpcError::new(
            "AG_TOOL_LOOP_STATE_INVALID",
            &format!("SM-AG-02 非法转移：{:?} + {:?}", state, event),
            false,
            false,
        )
    })?;
    *state = next;
    emit_ag02(app, turn_id, next);
    Ok(next)
}

/**
 * @GOV
 * codes:    BR-AG-STATE-002-RB-AG-AG-TOOL-018, BR-SYS-STATE-008-RB-AG-AG-TOOL-018, BR-AG-OBS-001-RB-AG-AG-TOOL-018
 * type:     RB
 * chain:    AG-TURN, AG-TOOL
 * rules:    BR-AG-STATE-002, BR-SYS-STATE-008, BR-AG-OBS-001
 * term_ref: TERM-AG-014, TERM-AG-015, TERM-AG-016, TERM-AG-020, TERM-AG-021, TERM-AG-023
 * boundary: in=PreparedPromptAgentContextFactsPromptManifestCapabilityFactorsAndActiveAgentTurnWithOptionalTaskRun | out=PerToolCallCapabilityGrantValidatedToolResultsEvidenceLedgerDerivedProjectReportPendingDiffOrWorkspaceOperationDecisionRequestAndSingleAgentTurnTerminal | delegate=resolve_capability_grant_v2,validate_requested_tool_for_grant,advance_tool_loop,derive_project_report | consumer=OptionalTaskRunStatus | does_not=executeToolOutsideFreshGrantOrTreatPromptIntentProjectReportOrWorkspaceOperationPlanAsAuthorization
 */
#[allow(clippy::too_many_arguments)]
pub(super) async fn run_turn_tool_loop(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    on_event: tauri::ipc::Channel<AgTurnEvent>,
    cancelled: Arc<AtomicBool>,
    terminal_latch: ag::turn::TerminalLatch,
    turn_id: String,
    session_id: String,
    mut task_run_id: Option<String>,
    workspace_id_for_turn: String,
    provider: String,
    model: String,
    runtime_profile: String,
    mut prompt_package: ag::context::PromptPackageV2,
    agent_context_facts: crate::types::AgentContextFactsV1,
    mut current_doc_prompt_coverage: Option<&'static str>,
    mut messages: Vec<serde_json::Value>,
    mut tools: serde_json::Value,
    mut capability_factors: ag::tool_registry::CapabilityFactorsV2,
    read_ctx: ag::blocks::AgentReadContext,
    active_doc_path: Option<String>,
    explicit_edit_target_set: ag::target_set::ExplicitEditTargetSet,
    ws_root: Option<String>,
    mut current_doc_requires_tool_read: bool,
    mut intent_result: ag::context::IntentResultV2,
    mut continuation_messages: Vec<serde_json::Value>,
    history: Vec<ag::context::HistTurn>,
    raw_prompt: String,
    mention_block: Option<String>,
    reference_envelopes: Vec<ag::context::PromptDataEnvelopeV1>,
    active_snapshot: Option<ag::blocks::AgentEditableSnapshot>,
) -> Result<(), IpcError> {
    let event_emitter = TurnEventEmitter::new(&state, &on_event, &turn_id);
    let mut proposed_diff = false;
    let mut edit_blocked_reported = false;

    // SM-AG-02 工具循环（BR-AG-STATE-002：注册/权限校验 + 上限）。dev 上限 6 轮。
    // 循环经状态机驱动 + 广播 sm:SM-AG-02（ModelStreaming→ToolValidating→ToolExecuting→ResultRecording→
    // Recorded 后回归 SM-AG-01 并重新装配；FinalAnswer→LoopCompleted；达上限→LoopAborted。
    // 工具卡数据仍由 Channel 投影（互不替代）。
    use ag::tool::{ToolLoopEvent as TlEv, ToolLoopState as TlSt};
    const MAX_ROUNDS: usize = 6;
    let mut current_doc_read_confirmed = !current_doc_requires_tool_read;
    // AG-CTX-01：本轮是否调用过任何工具（用于区分"已读未提案"与"零工具调用"的失败文案）。
    let mut any_tool_called = false;
    // 完整 Prompt 审计只保留本次 AgentTurn 最后一次实际发给模型的请求；回合退出时由 Drop
    // 统一输出一次，避免工具续轮把前序 messages/tools 反复打印。
    let mut final_prompt_audit = prompt_audit_enabled().then(FinalPromptAuditLog::default);
    let mut tool_loop_state = TlSt::ModelStreaming;
    let mut capability_use_tracker = ag::tool_registry::CapabilityGrantUseTrackerV2::default();
    let mut remaining_tool_calls = capability_factors.remaining_tool_calls;
    let mut turn_state = ag::turn::TurnState::Streaming;
    macro_rules! advance_or_finish {
        ($event:expr) => {{
            match advance_tool_loop(&app, &turn_id, &mut tool_loop_state, $event) {
                Ok(next) => next,
                Err(error) => {
                    settle_and_emit_ag_turn_error(
                        &state,
                        task_run_id.as_deref(),
                        &event_emitter,
                        &terminal_latch,
                        &app,
                        &turn_id,
                        &error,
                    );
                    return Ok(());
                }
            }
        }};
    }
    for _round in 0..MAX_ROUNDS {
        if cancelled.load(Ordering::Acquire) {
            advance_or_finish!(TlEv::Cancel);
            if let Ok(conn) = state.db.lock() {
                settle_optional_task_run(
                    &conn,
                    task_run_id.as_deref(),
                    &turn_id,
                    ag::task::TurnOutcome::Cancelled,
                );
            }
            emit_ag_turn_cancelled(&event_emitter, &terminal_latch, &app, &turn_id);
            return Ok(());
        }
        if _round > 0 {
            debug_assert_eq!(turn_state, ag::turn::TurnState::AwaitingGate);
            event_emitter
                .record_internal("gate_requested", serde_json::json!({"attempt": _round + 1}))?;
            let gate_error = match ent::gateway::check_gate().await {
                Ok(ent::gateway::GateOutcome::Pass) => {
                    event_emitter.record_internal(
                        "gate_evaluated",
                        serde_json::json!({"attempt": _round + 1, "outcome": "passed"}),
                    )?;
                    None
                }
                Ok(ent::gateway::GateOutcome::Deny) => {
                    event_emitter.record_internal(
                        "gate_evaluated",
                        serde_json::json!({"attempt": _round + 1, "outcome": "denied"}),
                    )?;
                    Some(IpcError::new(
                        "AG_GATE_DENIED",
                        "续调门禁被服务端拒绝，本轮已停止。",
                        false,
                        true,
                    ))
                }
                Err(error) => {
                    event_emitter.record_internal(
                        "gate_evaluated",
                        serde_json::json!({
                            "attempt": _round + 1,
                            "outcome": "error",
                            "code": &error.code,
                        }),
                    )?;
                    Some(error)
                }
            };
            if let Some(error) = gate_error {
                if let Ok(conn) = state.db.lock() {
                    settle_optional_task_run(
                        &conn,
                        task_run_id.as_deref(),
                        &turn_id,
                        ag::task::TurnOutcome::Failed,
                    );
                }
                emit_ag_turn_denied(&event_emitter, &terminal_latch, &app, &turn_id, &error);
                return Ok(());
            }
            turn_state = ag::turn::transition(turn_state, ag::turn::TurnEvent::GatePassed)
                .ok_or_else(sm_error)?;
            emit_ag01(&app, &turn_id, turn_state);
            tool_loop_state = TlSt::ModelStreaming;
        }
        debug_assert_eq!(tool_loop_state, TlSt::ModelStreaming);
        // layer 是审计/装配内部元数据，发送 provider 前剥离（不进入模型输入）。
        let provider_messages = provider_messages_for_model(&messages);
        if let Some(audit_log) = final_prompt_audit.as_mut() {
            let audit = build_prompt_audit_record(
                &turn_id,
                &session_id,
                _round + 1,
                &runtime_profile,
                &provider,
                &model,
                &prompt_package,
                &tools,
                &provider_messages,
            );
            audit_log.capture(audit);
        }
        let msgs = serde_json::Value::Array(provider_messages);
        event_emitter.record_internal(
            "provider_started",
            serde_json::json!({
                "roundIndex": _round + 1,
                "provider": &provider,
                "model": &model,
            }),
        )?;
        let (abort_handle, abort_registration) = futures_util::future::AbortHandle::new_pair();
        if let Err(error) =
            install_agent_turn_abort_handle(&state, &session_id, &turn_id, abort_handle)
        {
            advance_or_finish!(TlEv::LoopError);
            settle_and_emit_ag_turn_error(
                &state,
                task_run_id.as_deref(),
                &event_emitter,
                &terminal_latch,
                &app,
                &turn_id,
                &error,
            );
            return Ok(());
        }
        let mut output_decoder = ag::model_output::ModelOutputDecoder::default();
        let mut visible_output_chars = 0usize;
        let round = match futures_util::future::Abortable::new(
            ent::gateway::stream_round(
                &turn_id,
                _round + 1,
                &provider,
                &model,
                &msgs,
                &tools,
                |text| {
                    for visible in output_decoder.push(&text)? {
                        visible_output_chars += visible.chars().count();
                        event_emitter
                            .emit(&terminal_latch, AgTurnEventPayload::Chunk { text: visible })?;
                    }
                    Ok(())
                },
            ),
            abort_registration,
        )
        .await
        {
            Err(_) => {
                event_emitter.record_internal(
                    "provider_finished",
                    serde_json::json!({"roundIndex": _round + 1, "outcome": "cancelled"}),
                )?;
                advance_or_finish!(TlEv::Cancel);
                if let Ok(conn) = state.db.lock() {
                    settle_optional_task_run(
                        &conn,
                        task_run_id.as_deref(),
                        &turn_id,
                        ag::task::TurnOutcome::Cancelled,
                    );
                }
                emit_ag_turn_cancelled(&event_emitter, &terminal_latch, &app, &turn_id);
                return Ok(());
            }
            Ok(Err(error)) => {
                event_emitter.record_internal(
                    "provider_finished",
                    serde_json::json!({
                        "roundIndex": _round + 1,
                        "outcome": "transport_error",
                        "code": &error.code,
                    }),
                )?;
                advance_or_finish!(TlEv::LoopError);
                if let Ok(conn) = state.db.lock() {
                    settle_optional_task_run(
                        &conn,
                        task_run_id.as_deref(),
                        &turn_id,
                        ag::task::TurnOutcome::Failed,
                    );
                }
                emit_ag_turn_error(&event_emitter, &terminal_latch, &app, &turn_id, &error);
                return Ok(());
            }
            Ok(Ok(outcome)) => {
                event_emitter.record_internal(
                    "provider_finished",
                    serde_json::json!({
                        "roundIndex": _round + 1,
                        "outcome": match &outcome {
                            ent::gateway::RoundOutcome::Failed(_) => "failed",
                            ent::gateway::RoundOutcome::Done { .. } => "done",
                            ent::gateway::RoundOutcome::ToolCalls(_) => "tool_calls",
                        },
                    }),
                )?;
                outcome
            }
        };
        match round {
            ent::gateway::RoundOutcome::Failed(error) => {
                advance_or_finish!(TlEv::LoopError);
                if let Ok(conn) = state.db.lock() {
                    settle_optional_task_run(
                        &conn,
                        task_run_id.as_deref(),
                        &turn_id,
                        ag::task::TurnOutcome::Failed,
                    );
                }
                emit_ag_turn_error(&event_emitter, &terminal_latch, &app, &turn_id, &error);
                return Ok(());
            }
            ent::gateway::RoundOutcome::Done { stop_reason } => {
                let control = match output_decoder.finish() {
                    Ok(control) => control,
                    Err(error) => {
                        advance_or_finish!(TlEv::LoopError);
                        settle_and_emit_ag_turn_error(
                            &state,
                            task_run_id.as_deref(),
                            &event_emitter,
                            &terminal_latch,
                            &app,
                            &turn_id,
                            &error,
                        );
                        return Ok(());
                    }
                };
                intent_result = match apply_model_control(
                    &event_emitter,
                    &terminal_latch,
                    &intent_result,
                    &control,
                ) {
                    Ok(intent) => intent,
                    Err(error) => {
                        advance_or_finish!(TlEv::LoopError);
                        settle_and_emit_ag_turn_error(
                            &state,
                            task_run_id.as_deref(),
                            &event_emitter,
                            &terminal_latch,
                            &app,
                            &turn_id,
                            &error,
                        );
                        return Ok(());
                    }
                };
                if let Err(error) = ensure_route_task_projection(
                    &state,
                    &event_emitter,
                    &session_id,
                    &workspace_id_for_turn,
                    &turn_id,
                    &explicit_edit_target_set,
                    &mut task_run_id,
                    &control,
                ) {
                    advance_or_finish!(TlEv::LoopError);
                    settle_and_emit_ag_turn_error(
                        &state,
                        task_run_id.as_deref(),
                        &event_emitter,
                        &terminal_latch,
                        &app,
                        &turn_id,
                        &error,
                    );
                    return Ok(());
                }
                match control.next_action {
                    ag::model_output::ModelNextAction::Clarify => {
                        let clarification_question = control
                            .clarification_question
                            .expect("validated clarification must include a question");
                        let mut checkpoint_messages = continuation_messages;
                        checkpoint_messages.push(serde_json::json!({
                            "role": "assistant",
                            "content": clarification_question,
                        }));
                        turn_state = ag::turn::transition(
                            turn_state,
                            ag::turn::TurnEvent::ModelClarification,
                        )
                        .ok_or_else(sm_error)?;
                        park_agent_turn_for_clarification(
                            &state,
                            &session_id,
                            &turn_id,
                            task_run_id.as_deref(),
                            AgentTurnRuntimeCheckpoint {
                                intent: intent_result,
                                initial_prompt: raw_prompt,
                                continuation_messages: checkpoint_messages,
                            },
                        )?;
                        emit_ag01(&app, &turn_id, turn_state);
                        event_emitter.emit(
                            &terminal_latch,
                            AgTurnEventPayload::ClarificationRequired {
                                text: clarification_question,
                            },
                        )?;
                        return Ok(());
                    }
                    ag::model_output::ModelNextAction::Tool => {
                        let error = IpcError::new(
                            "AG_MODEL_CONTROL_INVALID",
                            "无 ToolCall 的模型响应不能声明 nextAction=tool",
                            true,
                            false,
                        );
                        advance_or_finish!(TlEv::LoopError);
                        settle_and_emit_ag_turn_error(
                            &state,
                            task_run_id.as_deref(),
                            &event_emitter,
                            &terminal_latch,
                            &app,
                            &turn_id,
                            &error,
                        );
                        return Ok(());
                    }
                    ag::model_output::ModelNextAction::Final => {}
                }
                let required_terminal_tool = match intent_result.side_effect_intent {
                    ag::context::SideEffectIntentV2::ContentProposal => Some("propose_edit"),
                    ag::context::SideEffectIntentV2::StructurePlan => {
                        Some("plan_workspace_operation")
                    }
                    ag::context::SideEffectIntentV2::None => None,
                };
                if terminal_tool_is_missing(
                    required_terminal_tool,
                    proposed_diff,
                    edit_blocked_reported,
                ) {
                    let error =
                        required_terminal_tool_error(required_terminal_tool, any_tool_called);
                    advance_or_finish!(TlEv::LoopError);
                    if let Ok(conn) = state.db.lock() {
                        settle_optional_task_run(
                            &conn,
                            task_run_id.as_deref(),
                            &turn_id,
                            ag::task::TurnOutcome::Failed,
                        );
                    }
                    emit_ag_turn_error(&event_emitter, &terminal_latch, &app, &turn_id, &error);
                    return Ok(());
                }
                if visible_output_chars == 0 {
                    let error = IpcError::new(
                        "AG_MODEL_FINAL_EMPTY",
                        "模型声明 final 但没有提供可见回答",
                        true,
                        false,
                    );
                    advance_or_finish!(TlEv::LoopError);
                    settle_and_emit_ag_turn_error(
                        &state,
                        task_run_id.as_deref(),
                        &event_emitter,
                        &terminal_latch,
                        &app,
                        &turn_id,
                        &error,
                    );
                    return Ok(());
                }
                advance_or_finish!(TlEv::FinalAnswer);
                turn_state = ag::turn::transition(turn_state, ag::turn::TurnEvent::ModelFinal)
                    .ok_or_else(sm_error)?;
                emit_ag01(&app, &turn_id, turn_state);
                let report_result = (|| -> Result<bool, IpcError> {
                    let evidence_ledger = {
                        let conn = state.db.lock().map_err(|_| {
                            IpcError::new(
                                "WS_DB_BUSY",
                                "数据库连接繁忙，无法派生 ProjectReport",
                                true,
                                false,
                            )
                        })?;
                        super::turn_events::rebuild_evidence_ledger(
                            &conn,
                            &workspace_id_for_turn,
                            &turn_id,
                        )?
                    };
                    let project_report = ag::tool_exec::derive_project_report(
                        &turn_id,
                        requested_report_scope(&intent_result, active_doc_path.as_deref()),
                        &evidence_ledger,
                        &control.findings,
                    );
                    event_emitter.emit(
                        &terminal_latch,
                        AgTurnEventPayload::ProjectReport {
                            report: project_report,
                        },
                    )
                })();
                match report_result {
                    Ok(true) => {}
                    Ok(false) => return Ok(()),
                    Err(error) => {
                        if let Ok(conn) = state.db.lock() {
                            settle_optional_task_run(
                                &conn,
                                task_run_id.as_deref(),
                                &turn_id,
                                ag::task::TurnOutcome::Failed,
                            );
                        }
                        emit_ag_turn_error_from(
                            &event_emitter,
                            &terminal_latch,
                            &app,
                            &turn_id,
                            &error,
                            ag::turn::TurnState::Finalizing,
                        );
                        return Ok(());
                    }
                }
                if let Ok(conn) = state.db.lock() {
                    settle_optional_task_run(
                        &conn,
                        task_run_id.as_deref(),
                        &turn_id,
                        ag::task::TurnOutcome::Completed,
                    );
                }
                emit_ag_turn_done(
                    &event_emitter,
                    &terminal_latch,
                    &app,
                    &turn_id,
                    &stop_reason,
                    ag::turn::TurnState::Finalizing,
                );
                return Ok(());
            }
            ent::gateway::RoundOutcome::ToolCalls(calls) => {
                if calls.is_empty() {
                    let error = IpcError::new(
                        "AG_TOOL_CALLS_EMPTY",
                        "模型返回了空 ToolCall 集合，当前工具循环已中止。",
                        true,
                        false,
                    );
                    advance_or_finish!(TlEv::LoopError);
                    settle_and_emit_ag_turn_error(
                        &state,
                        task_run_id.as_deref(),
                        &event_emitter,
                        &terminal_latch,
                        &app,
                        &turn_id,
                        &error,
                    );
                    return Ok(());
                }
                let output_control = match output_decoder.finish_optional() {
                    Ok(control) => control,
                    Err(error) => {
                        advance_or_finish!(TlEv::LoopError);
                        settle_and_emit_ag_turn_error(
                            &state,
                            task_run_id.as_deref(),
                            &event_emitter,
                            &terminal_latch,
                            &app,
                            &turn_id,
                            &error,
                        );
                        return Ok(());
                    }
                };
                let mut parsed_params = Vec::with_capacity(calls.len());
                let mut batch_control: Option<ag::model_output::ModelTurnControl> = None;
                let mut control_error = None;
                for call in &calls {
                    let parsed = serde_json::from_str::<serde_json::Value>(&call.arguments)
                        .map_err(|error| {
                            IpcError::new(
                                "AG_TOOL_ARGUMENTS_INVALID",
                                &format!("工具 {} 参数不是合法 JSON（{}）。", call.name, error),
                                true,
                                false,
                            )
                        })
                        .and_then(|mut params| {
                            let control = ag::model_output::take_tool_control(&mut params)?;
                            if let Some(expected) = batch_control.as_ref() {
                                if expected != &control {
                                    return Err(IpcError::new(
                                        "AG_MODEL_CONTROL_CONFLICT",
                                        "同一模型响应内的 ToolCall 使用了不同意图或路线",
                                        true,
                                        false,
                                    ));
                                }
                            } else {
                                batch_control = Some(control);
                            }
                            Ok(params)
                        });
                    match parsed {
                        Ok(params) => parsed_params.push(params),
                        Err(error) => {
                            control_error = Some(error);
                            break;
                        }
                    }
                }
                if let Some(error) = control_error {
                    advance_or_finish!(TlEv::LoopError);
                    settle_and_emit_ag_turn_error(
                        &state,
                        task_run_id.as_deref(),
                        &event_emitter,
                        &terminal_latch,
                        &app,
                        &turn_id,
                        &error,
                    );
                    return Ok(());
                }
                let control =
                    batch_control.expect("non-empty calls must contain validated controls");
                if output_control
                    .as_ref()
                    .is_some_and(|header| header != &control)
                {
                    let error = IpcError::new(
                        "AG_MODEL_CONTROL_CONFLICT",
                        "模型文本封套与 ToolCall 语义封套不一致",
                        true,
                        false,
                    );
                    advance_or_finish!(TlEv::LoopError);
                    settle_and_emit_ag_turn_error(
                        &state,
                        task_run_id.as_deref(),
                        &event_emitter,
                        &terminal_latch,
                        &app,
                        &turn_id,
                        &error,
                    );
                    return Ok(());
                }
                intent_result = match apply_model_control(
                    &event_emitter,
                    &terminal_latch,
                    &intent_result,
                    &control,
                ) {
                    Ok(intent) => intent,
                    Err(error) => {
                        advance_or_finish!(TlEv::LoopError);
                        settle_and_emit_ag_turn_error(
                            &state,
                            task_run_id.as_deref(),
                            &event_emitter,
                            &terminal_latch,
                            &app,
                            &turn_id,
                            &error,
                        );
                        return Ok(());
                    }
                };
                if let Err(error) = ensure_route_task_projection(
                    &state,
                    &event_emitter,
                    &session_id,
                    &workspace_id_for_turn,
                    &turn_id,
                    &explicit_edit_target_set,
                    &mut task_run_id,
                    &control,
                ) {
                    advance_or_finish!(TlEv::LoopError);
                    settle_and_emit_ag_turn_error(
                        &state,
                        task_run_id.as_deref(),
                        &event_emitter,
                        &terminal_latch,
                        &app,
                        &turn_id,
                        &error,
                    );
                    return Ok(());
                }
                current_doc_requires_tool_read = matches!(
                    intent_result.side_effect_intent,
                    ag::context::SideEffectIntentV2::ContentProposal
                ) && current_doc_prompt_coverage
                    != Some("full_document")
                    && !current_doc_read_confirmed;
                any_tool_called = true;
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
                let assistant_tool_calls = serde_json::json!({ "role": "assistant", "content": null, "tool_calls": tc_json });
                messages.push(assistant_tool_calls.clone());
                continuation_messages.push(assistant_tool_calls);

                let mut stop_after_current_tool = false;
                for (call_index, c) in calls.iter().enumerate() {
                    if cancelled.load(Ordering::Acquire) {
                        advance_or_finish!(TlEv::Cancel);
                        if let Ok(conn) = state.db.lock() {
                            settle_optional_task_run(
                                &conn,
                                task_run_id.as_deref(),
                                &turn_id,
                                ag::task::TurnOutcome::Cancelled,
                            );
                        }
                        emit_ag_turn_cancelled(&event_emitter, &terminal_latch, &app, &turn_id);
                        return Ok(());
                    }
                    advance_or_finish!(TlEv::ToolRequest);
                    let params = parsed_params[call_index].clone();
                    let arg_parse_error: Option<String> = None;
                    event_emitter.emit(
                        &terminal_latch,
                        AgTurnEventPayload::ToolRequest {
                            tool_call_id: c.id.clone(),
                            tool_name: c.name.clone(),
                            params: safe_tool_params(&params),
                        },
                    )?;
                    // AG-1.0-TASKRUN-REWORK-IMP-RUNTIME-01：实时当前动作（信息流底部状态条，不伪造）。
                    event_emitter.emit(
                        &terminal_latch,
                        AgTurnEventPayload::StepProgress {
                            action: if c.name == "propose_decision"
                                || c.name == "plan_workspace_operation"
                            {
                                "deciding".to_string()
                            } else {
                                "tool".to_string()
                            },
                            label: c.name.clone(),
                            detail: None,
                        },
                    )?;
                    let mut diff_event_after_tool: Option<(String, String)> = None;
                    let mut decision_event_after_tool = None;
                    let mut preview_checkpoint_after_tool = false;
                    let mut end_after_diff_proposal = false;
                    let mut mutation_guard = None;
                    // 每个 ToolCall 在真实 GatePassed 后重新取得一次性 CapabilityGrantV2。Prompt Manifest
                    // 只决定模型当时可见的工具，不能拿来执行；TaskPlan/IntentResult/ToolResult 内容均不授权。
                    let computed_at_seq = event_emitter
                        .record_internal(
                            "capability_grant_boundary",
                            serde_json::json!({
                                "toolCallId": &c.id,
                                "gateAttempt": _round + 1,
                                "remainingToolCalls": remaining_tool_calls,
                            }),
                        )?
                        .ok_or_else(|| {
                            IpcError::new(
                                "AG_CAPABILITY_GRANT_TERMINAL",
                                "AgentTurn 已终结，不能再计算工具执行授权",
                                false,
                                false,
                            )
                        })?;
                    let mut action_factors = capability_factors.clone();
                    action_factors.has_task_run = task_run_id.is_some();
                    action_factors.gate_attempt = (_round + 1) as u32;
                    action_factors.gate_passed = true;
                    action_factors.remaining_tool_calls = remaining_tool_calls;
                    action_factors.cancelled = cancelled.load(Ordering::Acquire);
                    action_factors.terminal = terminal_latch.is_set();
                    let execution_grant = ag::tool_registry::resolve_capability_grant_v2(
                        &runtime_profile,
                        &turn_id,
                        &explicit_edit_target_set.paths(),
                        action_factors,
                        ag::tool_registry::CapabilityGrantPurposeV2::ToolCall {
                            tool_call_id: c.id.clone(),
                            computed_at_seq,
                        },
                    );
                    event_emitter.record_internal(
                        "capability_grant_computed",
                        serde_json::json!({
                            "grantId": &execution_grant.grant_id,
                            "toolCallId": &c.id,
                            "computedAtSeq": computed_at_seq,
                            "authorizationFactsHash": &execution_grant.authorization_facts_hash,
                            "allowedToolCount": execution_grant.allowed_tools.len(),
                        }),
                    )?;
                    remaining_tool_calls = remaining_tool_calls.saturating_sub(1);
                    let tool_was_visible = tool_is_visible_in_manifest(&tools, &c.name);
                    let tool_decision = arg_parse_error.is_none().then(|| {
                        let validated = ag::tool_registry::validate_requested_tool_for_grant(
                            &c.name,
                            &params,
                            &execution_grant,
                            &turn_id,
                            &c.id,
                            computed_at_seq,
                            ws_root.as_deref().map(std::path::Path::new),
                            &mut capability_use_tracker,
                        );
                        if matches!(validated, ag::tool::ToolDecision::Allow) && !tool_was_visible {
                            ag::tool::ToolDecision::Reject(format!(
                                "工具 {} 不在当前 Prompt Tool Manifest 中",
                                c.name
                            ))
                        } else {
                            validated
                        }
                    });
                    let scope_candidates =
                        if matches!(tool_decision.as_ref(), Some(ag::tool::ToolDecision::Allow))
                            && c.name == "propose_decision"
                            && params.get("kind").and_then(|value| value.as_str()) == Some("scope")
                        {
                            let raw_paths = params
                                .get("targetPaths")
                                .and_then(|value| value.as_array())
                                .map(|paths| {
                                    paths
                                        .iter()
                                        .filter_map(|path| path.as_str().map(str::to_string))
                                        .collect::<Vec<_>>()
                                })
                                .unwrap_or_default();
                            match ws_root.as_deref() {
                                Some(root) => ag::target_set::validated_scope_candidates(
                                    std::path::Path::new(root),
                                    &raw_paths,
                                    &explicit_edit_target_set,
                                )
                                .map(Some),
                                None => Err(IpcError::new(
                                    "WS_NO_ACTIVE",
                                    "缺少 active workspace，不能冻结范围追加目标",
                                    false,
                                    false,
                                )),
                            }
                        } else {
                            Ok(None)
                        };
                    // 简单回合保持无 TaskRun。只有已获准的任务级工具真的到达执行边界时，
                    // 才在创建 DecisionRequest / WorkspaceOperationPlan 之前取得并绑定 TaskRun。
                    let task_run_was_missing = task_run_id.is_none();
                    let mut task_run_prepare_error =
                        if matches!(tool_decision.as_ref(), Some(ag::tool::ToolDecision::Allow))
                            && scope_candidates.is_ok()
                            && ag::task::tool_requires_task_run(&c.name)
                            && task_run_id.is_none()
                        {
                            let created = match state.db.lock() {
                                Ok(conn) => ag::task::begin_for_tool_product(
                                    &conn,
                                    &workspace_id_for_turn,
                                    &session_id,
                                    &c.name,
                                )
                                .map_err(|error| {
                                    IpcError::new(
                                        "AG_TASK_DB_ERROR",
                                        &error.to_string(),
                                        true,
                                        false,
                                    )
                                })
                                .and_then(|task_run_id| {
                                    task_run_id.ok_or_else(|| {
                                        IpcError::new(
                                            "AG_TASK_REQUIREMENT_DRIFT",
                                            "任务级工具判据在 TaskRun 创建边界发生漂移",
                                            false,
                                            false,
                                        )
                                    })
                                }),
                                Err(_) => Err(IpcError::new(
                                    "WS_DB_BUSY",
                                    "数据库连接繁忙，无法创建任务级产物",
                                    true,
                                    false,
                                )),
                            };
                            match created {
                                Ok(created_task_run_id) => {
                                    task_run_id = Some(created_task_run_id.clone());
                                    match bind_agent_turn_task_run(
                                        &state,
                                        &session_id,
                                        &turn_id,
                                        &created_task_run_id,
                                    ) {
                                        Ok(()) => match state.db.lock() {
                                            Ok(conn) => ag::task::link_turn(
                                                &conn,
                                                &turn_id,
                                                &created_task_run_id,
                                            )
                                            .and_then(|_| {
                                                ag::target_set::persist_for_task_run(
                                                    &conn,
                                                    &created_task_run_id,
                                                    &explicit_edit_target_set,
                                                )
                                            })
                                            .err()
                                            .map(|error| {
                                                format!("TaskRun 无法固化当前回合范围：{error}")
                                            }),
                                            Err(_) => Some(
                                                "数据库连接繁忙，无法固化 TaskRun 回合范围"
                                                    .to_string(),
                                            ),
                                        },
                                        Err(error) => Some(format!(
                                            "TaskRun 无法绑定当前 AgentTurn：{}",
                                            error.message
                                        )),
                                    }
                                }
                                Err(error) => Some(error.message),
                            }
                        } else {
                            None
                        };
                    if task_run_prepare_error.is_none() && task_run_was_missing {
                        if let Some(created_task_run_id) = task_run_id.as_deref() {
                            if let Err(error) = event_emitter.record_internal(
                                "task_projected",
                                serde_json::json!({
                                    "taskRunId": created_task_run_id,
                                    "goal": &intent_result.goal,
                                    "reason": "task_level_waiting_point",
                                }),
                            ) {
                                task_run_prepare_error = Some(error.message);
                            }
                        }
                    }
                    let execution_guard_error = if let Err(error) = &scope_candidates {
                        Some(error.message.clone())
                    } else if task_run_prepare_error.is_some() {
                        task_run_prepare_error
                    } else {
                        None
                    };
                    let effect_lock_error = if execution_guard_error.is_none()
                        && matches!(
                            c.name.as_str(),
                            "propose_decision" | "plan_workspace_operation"
                        ) {
                        match lock_agent_turn_effect(&state, &session_id, &turn_id) {
                            Ok(guard) => {
                                mutation_guard = Some(guard);
                                None
                            }
                            Err(error) => Some(error.message),
                        }
                    } else {
                        None
                    };
                    let validation_allows_execution = arg_parse_error.is_none()
                        && execution_guard_error.is_none()
                        && effect_lock_error.is_none()
                        && matches!(tool_decision.as_ref(), Some(ag::tool::ToolDecision::Allow));
                    if validation_allows_execution {
                        advance_or_finish!(TlEv::ValidateOk);
                    } else {
                        advance_or_finish!(TlEv::ValidateFail);
                    }
                    let (result, success) = if let Some(error_msg) = arg_parse_error {
                        (error_msg, false)
                    } else if let Some(error_msg) = execution_guard_error {
                        (error_msg, false)
                    } else if let Some(error_msg) = effect_lock_error {
                        (error_msg, false)
                    } else {
                        match tool_decision
                            .expect("parsed tool arguments must have a validation decision")
                        {
                            ag::tool::ToolDecision::Reject(reason) => (reason, false),
                            ag::tool::ToolDecision::Allow if c.name == "edit_blocked" => {
                                match ag::tool_exec::exec_edit_blocked(&params) {
                                    Ok(result) => {
                                        edit_blocked_reported = true;
                                        (result, true)
                                    }
                                    Err(error) => {
                                        (format!("阻断报告无效：{}", error.message), false)
                                    }
                                }
                            }
                            // 任务级确认只创建 DecisionRequest，不修改文档或 Diff。成功后当前执行片段必须
                            // 停在 AwaitingDecision；由下方事件分派统一发送确认卡，但不得发送 Done。
                            ag::tool::ToolDecision::Allow if c.name == "propose_decision" => {
                                let current_task_run_id = task_run_id
                                    .as_deref()
                                    .expect("task-level decision tool must bind a TaskRun");
                                let kind = params.get("kind").and_then(|value| value.as_str());
                                let summary =
                                    params.get("summary").and_then(|value| value.as_str());
                                let detail_is_valid = params
                                    .get("detail")
                                    .map(|value| value.is_string())
                                    .unwrap_or(true);
                                let options_are_valid = params
                                    .get("options")
                                    .map(|value| {
                                        value.as_array().is_some_and(|options| {
                                            options.iter().all(|option| {
                                                option
                                                    .as_str()
                                                    .is_some_and(|option| !option.trim().is_empty())
                                            })
                                        })
                                    })
                                    .unwrap_or(true);
                                match (kind, summary) {
                                (Some(kind), Some(summary))
                                    if !kind.trim().is_empty()
                                        && !summary.trim().is_empty()
                                        && detail_is_valid
                                        && options_are_valid =>
                                {
                                    let payload = serde_json::json!({
                                        "summary": summary,
                                        "detail": params.get("detail").cloned(),
                                        "options": params.get("options").cloned(),
                                        "targetPaths": scope_candidates
                                            .as_ref()
                                            .ok()
                                            .and_then(|paths| paths.clone()),
                                    });
                                    match state.db.lock() {
                                        Ok(conn) => match ag::task::propose_decision(
                                            &conn,
                                            current_task_run_id,
                                            kind,
                                            payload,
                                        ) {
                                            Ok(decision) => {
                                                decision_event_after_tool = Some(decision.clone());
                                                (
                                                    serde_json::json!({
                                                        "summary": "已创建任务级确认请求；原 AgentTurn 正在等待用户确认或取消。",
                                                        "request_id": decision.request_id,
                                                        "status": "waiting_confirm",
                                                    })
                                                    .to_string(),
                                                    true,
                                                )
                                            }
                                            Err(error) => (
                                                format!("创建任务级确认请求失败：{error}"),
                                                false,
                                            ),
                                        },
                                        Err(_) => (
                                            "数据库连接繁忙，无法创建任务级确认请求".to_string(),
                                            false,
                                        ),
                                    }
                                }
                                _ => (
                                    "propose_decision 参数无效：kind、summary 必须为非空字符串；detail 必须为字符串；options 必须是非空字符串数组。".to_string(),
                                    false,
                                ),
                            }
                            }
                            // WorkspaceOperation 工具只冻结计划并创建独立 DecisionRequest。成功后与
                            // propose_decision 一样结束当前片段；真实磁盘副作用只能由确认入口触发。
                            ag::tool::ToolDecision::Allow
                                if c.name == "plan_workspace_operation" =>
                            {
                                let current_task_run_id = task_run_id
                                    .as_deref()
                                    .expect("workspace operation tool must bind a TaskRun");
                                let mut operation_params = params.clone();
                                if let Some(object) = operation_params.as_object_mut() {
                                    object.insert(
                                        "taskRunId".into(),
                                        serde_json::Value::String(current_task_run_id.to_string()),
                                    );
                                    object.insert(
                                        "sourceTurnId".into(),
                                        serde_json::Value::String(turn_id.clone()),
                                    );
                                }
                                let operation_request = CommandRequest {
                                    command: "ws.plan_operation".into(),
                                    correlation_id: format!("ag-wop-{turn_id}"),
                                    params: operation_params,
                                };
                                match route_plan_workspace_operation(&operation_request, &state) {
                                    Ok(value) => {
                                        match value
                                        .get("decision_request")
                                        .cloned()
                                        .and_then(|decision| serde_json::from_value(decision).ok())
                                    {
                                        Some(decision) => {
                                            decision_event_after_tool = Some(decision);
                                            (
                                                serde_json::json!({
                                                    "summary": "已生成工作区资源操作确认卡；确认前未修改磁盘，原 AgentTurn 正在等待裁决。",
                                                    "plan_id": value.get("plan").and_then(|plan| plan.get("plan_id")),
                                                    "status": "waiting_confirm",
                                                })
                                                .to_string(),
                                                true,
                                            )
                                        }
                                        None => (
                                            "资源操作计划已创建，但 DecisionRequest 回执缺失，已停止执行"
                                                .to_string(),
                                            false,
                                        ),
                                    }
                                    }
                                    Err(error) => (
                                        format!("生成工作区资源操作计划失败：{}", error.message),
                                        false,
                                    ),
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
                                    let guarded_result =
                                        match lock_agent_turn_effect(&state, &session_id, &turn_id)
                                        {
                                            Ok(guard) => {
                                                mutation_guard = Some(guard);
                                                ag_propose_edit(
                                                    &state,
                                                    &app,
                                                    &params,
                                                    &read_ctx,
                                                    active_doc_path.as_deref(),
                                                    &explicit_edit_target_set,
                                                )
                                            }
                                            Err(error) => Err(error),
                                        };
                                    match guarded_result {
                                        Ok(mut edit_result) => {
                                            let needs_preview_checkpoint = matches!(
                                                edit_result.projection.draft_mode.as_str(),
                                                "current_editor" | "open_session"
                                            );
                                            if needs_preview_checkpoint {
                                                if let Some(current_task_run_id) =
                                                    task_run_id.as_deref()
                                                {
                                                    let checkpoint_result = match state.db.lock() {
                                                        Ok(conn) => {
                                                            ag::task::create_preview_checkpoint(
                                                                &conn,
                                                                current_task_run_id,
                                                                &workspace_id_for_turn,
                                                                &session_id,
                                                                &turn_id,
                                                                &edit_result.diff_id,
                                                                &edit_result.rel_path,
                                                            )
                                                        }
                                                        Err(_) => {
                                                            Err(rusqlite::Error::InvalidQuery)
                                                        }
                                                    };
                                                    match checkpoint_result {
                                                        Ok(()) => {
                                                            preview_checkpoint_after_tool = true;
                                                        }
                                                        Err(error) => {
                                                            edit_result
                                                                .projection
                                                                .preview_checkpoint_required =
                                                                false;
                                                            // Diff 已经持久化。此处不得把它变成前端不可见的孤儿
                                                            // 提案，也绝不能在没有 checkpoint 的情况下续用旧 read_ctx。
                                                            eprintln!(
                                                                "[ag-preview] checkpoint unavailable for diff {}: {error}",
                                                                edit_result.diff_id
                                                            );
                                                        }
                                                    }
                                                } else {
                                                    // 简单局部编辑不创建 TaskRun，也不伪造依赖任务身份的
                                                    // 自动续片段。Diff 仍作为当前 AgentTurn 的待审提案可见。
                                                    edit_result
                                                        .projection
                                                        .preview_checkpoint_required = false;
                                                }
                                            }
                                            proposed_diff = true;
                                            // 无 LogicalState 的 background Diff 同样不得让模型在同一份旧 read_ctx
                                            // 上继续续编；它没有 PreviewAck checkpoint，需经既有迁移或裁决后由
                                            // 人类下一次显式操作开启新片段。
                                            end_after_diff_proposal = true;
                                            diff_event_after_tool = Some((
                                                edit_result.diff_id.clone(),
                                                edit_result.rel_path.clone(),
                                            ));
                                            // AG-TASK-01 F1b：结果携带批次元数据，供模型与前端展示"第 x/y 批"。
                                            let mut result = serde_json::json!({
                                                "summary": format!("已生成待确认修改建议（diff_id={}）。本结果仅确认建议卡已生成；尚未收到编辑器预览、用户接受或文件保存的回执。回复请使用「已提出建议，待确认」，不得声称已应用或已保存。", edit_result.diff_id),
                                                "diff_id": edit_result.diff_id,
                                                "rel_path": edit_result.rel_path,
                                                "projection": edit_result.projection,
                                                "proposal_status": "pending_user_review",
                                                "editor_preview_status": "not_reported",
                                                "disk_persistence_status": "not_saved",
                                                "preview_checkpoint_required": preview_checkpoint_after_tool,
                                            });
                                            if let Some(batch) = &edit_result.batch {
                                                result["batch"] = serde_json::json!({
                                                    "id": batch.batch_id,
                                                    "index": batch.index,
                                                    "total": batch.total,
                                                });
                                                result["summary"] = serde_json::Value::String(format!(
                                                "已生成第 {}/{} 批待确认修改建议（batch_id={}，diff_id={}）。本结果仅确认建议卡已生成；尚未收到编辑器预览、用户接受或文件保存的回执。回复请使用「已提出建议，待确认」。",
                                                batch.index, batch.total, batch.batch_id, edit_result.diff_id
                                            ));
                                            }
                                            (result.to_string(), true)
                                        }
                                        Err(e) => {
                                            (format!("生成修改建议失败：{}", e.message), false)
                                        }
                                    }
                                }
                            }
                            // 只读工具：经 WS/KB 执行（限 active workspace）。
                            ag::tool::ToolDecision::Allow => match ws_root.as_deref() {
                                None => ("无活动 workspace，无法执行工具".to_string(), false),
                                Some(_) => {
                                    match ag::tool_exec::exec_read_only(&read_ctx, &c.name, &params)
                                    {
                                        Ok(r) => (r, true),
                                        Err(e) => (format!("工具执行失败：{}", e.message), false),
                                    }
                                }
                            },
                        }
                    };
                    if success
                        && is_current_doc_read_tool(&c.name, &params, active_doc_path.as_deref())
                    {
                        current_doc_read_confirmed = true;
                    }
                    if cancelled.load(Ordering::Acquire) {
                        advance_or_finish!(TlEv::Cancel);
                        if let Ok(conn) = state.db.lock() {
                            settle_optional_task_run(
                                &conn,
                                task_run_id.as_deref(),
                                &turn_id,
                                ag::task::TurnOutcome::Cancelled,
                            );
                        }
                        emit_ag_turn_cancelled(&event_emitter, &terminal_latch, &app, &turn_id);
                        return Ok(());
                    }
                    if validation_allows_execution {
                        advance_or_finish!(if success {
                            TlEv::ExecOk
                        } else {
                            TlEv::ExecFail
                        });
                    }
                    let pending_evidence = if success {
                        ag::tool_exec::successful_tool_evidence(
                            &read_ctx, &c.name, &params, &result,
                        )
                    } else {
                        Vec::new()
                    };
                    let evidence_ids = ag::tool_exec::evidence_ids(
                        &workspace_id_for_turn,
                        &turn_id,
                        &c.id,
                        &pending_evidence,
                    );
                    let tool_result_event = event_emitter.emit_recorded(
                        &terminal_latch,
                        AgTurnEventPayload::ToolResult {
                            tool_call_id: c.id.clone(),
                            tool_name: c.name.clone(),
                            result: safe_tool_result(&c.name, success),
                            success,
                            evidence_ids: evidence_ids.clone(),
                        },
                    )?;
                    if let Some(source_event) = tool_result_event {
                        let evidence_items = ag::tool_exec::materialize_evidence(
                            &workspace_id_for_turn,
                            &turn_id,
                            &source_event.event_id,
                            source_event.seq,
                            &c.id,
                            pending_evidence,
                        );
                        if !evidence_items.is_empty() {
                            event_emitter.record_internal(
                                "evidence_recorded",
                                serde_json::json!({ "evidenceItems": evidence_items }),
                            )?;
                        }
                    } else {
                        return Ok(());
                    }
                    if let Some((diff_id, rel_path)) = diff_event_after_tool {
                        event_emitter.emit(
                            &terminal_latch,
                            AgTurnEventPayload::DiffProposed {
                                proposal_batch_id: diff_id.clone(),
                                group_index: 1,
                                group_count: 1,
                                diff_id,
                                rel_path,
                                task_run_id: task_run_id.clone(),
                            },
                        )?;
                    }
                    let model_tool_result = if evidence_ids.is_empty() {
                        result.clone()
                    } else {
                        serde_json::json!({
                            "toolResult": serde_json::from_str::<serde_json::Value>(&result)
                                .unwrap_or_else(|_| serde_json::Value::String(result.clone())),
                            "evidenceIds": evidence_ids,
                        })
                        .to_string()
                    };
                    let tool_message = serde_json::json!({
                        "role": "tool",
                        "tool_call_id": c.id,
                        "content": model_tool_result,
                    });
                    messages.push(tool_message.clone());
                    continuation_messages.push(tool_message);
                    if let Some(decision) = decision_event_after_tool {
                        let request_id = decision.request_id.clone();
                        advance_or_finish!(TlEv::DecisionHandoff);
                        turn_state =
                            ag::turn::transition(turn_state, ag::turn::TurnEvent::DecisionProposed)
                                .ok_or_else(sm_error)?;
                        let current_task_run_id = task_run_id
                            .as_deref()
                            .expect("DecisionRequest must retain its TaskRun");
                        drop(mutation_guard);
                        let park_result = park_agent_turn_for_decision(
                            &state,
                            &session_id,
                            &turn_id,
                            current_task_run_id,
                            &request_id,
                            AgentTurnRuntimeCheckpoint {
                                intent: intent_result,
                                initial_prompt: raw_prompt,
                                continuation_messages,
                            },
                        );
                        if let Err(error) = park_result {
                            if cancelled.load(Ordering::Acquire) {
                                if let Ok(conn) = state.db.lock() {
                                    settle_optional_task_run(
                                        &conn,
                                        task_run_id.as_deref(),
                                        &turn_id,
                                        ag::task::TurnOutcome::Cancelled,
                                    );
                                }
                                emit_ag_turn_cancelled(
                                    &event_emitter,
                                    &terminal_latch,
                                    &app,
                                    &turn_id,
                                );
                                return Ok(());
                            }
                            return Err(error);
                        }
                        let projected = event_emitter.emit(
                            &terminal_latch,
                            AgTurnEventPayload::DecisionRequestProposed {
                                request_id: decision.request_id,
                                kind: decision.kind,
                                payload: decision.payload,
                            },
                        )?;
                        if !projected {
                            return Ok(());
                        }
                        event_emitter.record_internal(
                            "decision_wait_started",
                            serde_json::json!({"requestId": request_id}),
                        )?;
                        emit_ag01(&app, &turn_id, turn_state);
                        return Ok(());
                    }
                    drop(mutation_guard);
                    let has_more_calls = call_index + 1 < calls.len();
                    let loop_budget_reached =
                        remaining_tool_calls == 0 || (_round + 1 == MAX_ROUNDS && !has_more_calls);
                    if loop_budget_reached {
                        advance_or_finish!(TlEv::LoopLimit);
                        stop_after_current_tool = true;
                    } else if end_after_diff_proposal && success {
                        advance_or_finish!(TlEv::ProposalRecorded);
                        stop_after_current_tool = true;
                    } else {
                        advance_or_finish!(TlEv::Recorded);
                    }
                    if stop_after_current_tool {
                        break;
                    }
                    if has_more_calls {
                        tool_loop_state = TlSt::ModelStreaming;
                    }
                }
                if _round + 1 == MAX_ROUNDS || remaining_tool_calls == 0 {
                    let error = IpcError::new(
                        "AG_TOOL_LOOP_LIMIT",
                        "工具循环达到上限，当前 AgentTurn 已中止；未完成步骤不会被标记为完成。",
                        true,
                        false,
                    );
                    if let Ok(conn) = state.db.lock() {
                        settle_optional_task_run(
                            &conn,
                            task_run_id.as_deref(),
                            &turn_id,
                            ag::task::TurnOutcome::Failed,
                        );
                    }
                    emit_ag_turn_error_from(
                        &event_emitter,
                        &terminal_latch,
                        &app,
                        &turn_id,
                        &error,
                        turn_state,
                    );
                    return Ok(());
                }
                // ToolResult/Proposal 已落事实后必须先回 SM-AG-01 Assembling；不能从 ResultRecording
                // 直接调用 provider，也不能复用首次 PromptPackage/CapabilityGrant。
                turn_state =
                    ag::turn::transition(turn_state, ag::turn::TurnEvent::ToolRoundCompleted)
                        .ok_or_else(sm_error)?;
                emit_ag01(&app, &turn_id, turn_state);
                let PreparedTurnPrompt {
                    current_doc_requires_tool_read: _,
                    current_doc_prompt_coverage: rebuilt_coverage,
                    messages: rebuilt_messages,
                    tools: rebuilt_tools,
                    capability_grant: _rebuilt_grant,
                    capability_factors: rebuilt_factors,
                    prompt_package: rebuilt_package,
                } = match prepare_turn_prompt(
                    &state,
                    &runtime_profile,
                    &turn_id,
                    &session_id,
                    task_run_id.as_deref(),
                    &intent_result,
                    &agent_context_facts,
                    &explicit_edit_target_set.paths(),
                    &read_ctx,
                    active_doc_path.as_deref(),
                    active_snapshot.as_ref(),
                    &history,
                    &raw_prompt,
                    mention_block.as_deref(),
                    &reference_envelopes,
                    proposed_diff,
                    &continuation_messages,
                    &capability_factors.decision_receipt_ids,
                    (_round + 2) as u32,
                    remaining_tool_calls,
                ) {
                    Ok(prepared) => prepared,
                    Err(error) => {
                        settle_and_emit_ag_turn_error_from(
                            &state,
                            task_run_id.as_deref(),
                            &event_emitter,
                            &terminal_latch,
                            &app,
                            &turn_id,
                            &error,
                            ag::turn::TurnState::Assembling,
                        );
                        return Ok(());
                    }
                };
                if ag::context::estimate_serialized_input_tokens(&rebuilt_messages, &rebuilt_tools)
                    > super::prompt_runtime::PROMPT_TOKEN_BUDGET
                {
                    let error = IpcError::new(
                        "AG_PROMPT_BUDGET_EXCEEDED",
                        "工具结果回注后的可信上下文超出 Prompt 预算，当前回合已停止。",
                        true,
                        false,
                    );
                    settle_and_emit_ag_turn_error_from(
                        &state,
                        task_run_id.as_deref(),
                        &event_emitter,
                        &terminal_latch,
                        &app,
                        &turn_id,
                        &error,
                        ag::turn::TurnState::Assembling,
                    );
                    return Ok(());
                }
                messages = rebuilt_messages;
                tools = rebuilt_tools;
                capability_factors = rebuilt_factors;
                prompt_package = rebuilt_package;
                current_doc_prompt_coverage = rebuilt_coverage;
                event_emitter.record_internal(
                    "prompt_rebuilt",
                    serde_json::json!({
                        "nextRound": _round + 2,
                        "intentRevision": intent_result.revision,
                        "contextTraceId": prompt_package.context_trace_id,
                        "toolManifestHash": prompt_package.tool_manifest_hash,
                    }),
                )?;
                turn_state = ag::turn::transition(turn_state, ag::turn::TurnEvent::Assembled)
                    .ok_or_else(sm_error)?;
                emit_ag01(&app, &turn_id, turn_state);
            }
        }
    }

    debug_assert!(false, "每个 provider 结果都应在循环内部返回或推进");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // covers: BR-SYS-STATE-007, BR-AG-PERSIST-001
    #[test]
    fn safe_tool_projection_keeps_identity_but_not_document_or_edit_body() {
        let params = serde_json::json!({
            "path": "notes/a.md",
            "query": "机密查询正文",
            "new_text": "整段文档改写内容",
        });
        let safe = safe_tool_params(&params);
        assert_eq!(safe["relPath"], "notes/a.md");
        assert_eq!(
            safe["parameterKeys"],
            serde_json::json!(["new_text", "path", "query"])
        );
        let serialized = safe.to_string();
        assert!(!serialized.contains("机密查询正文"));
        assert!(!serialized.contains("整段文档改写内容"));
        assert_eq!(safe_tool_result("read_doc", true), "read_doc 执行成功");
    }

    #[test]
    fn terminal_tool_requirement_distinguishes_content_and_workspace_side_effects() {
        assert!(!terminal_tool_is_missing(None, false, false));
        assert!(terminal_tool_is_missing(Some("propose_edit"), false, false,));
        assert!(!terminal_tool_is_missing(Some("propose_edit"), true, false,));
        assert!(!terminal_tool_is_missing(Some("propose_edit"), false, true,));
        assert!(terminal_tool_is_missing(
            Some("plan_workspace_operation"),
            false,
            false,
        ));
        assert_eq!(
            required_terminal_tool_error(Some("plan_workspace_operation"), false).code,
            "AG_WORKSPACE_PLAN_TOOL_REQUIRED"
        );
    }

    // covers: BR-AG-STATE-002
    // 生产状态广播只能由 advance_tool_loop 内真实转移成功后产生，防止回退为装饰性广播。
    #[test]
    fn production_tool_loop_has_one_transition_validated_state_broadcast_owner() {
        let source = include_str!("turn_tool_loop.rs");
        let broadcast_call = ["emit_ag02", "("].concat();
        let transition_call = ["ag::tool::transition", "("].concat();

        assert_eq!(source.matches(&broadcast_call).count(), 1);
        assert_eq!(source.matches(&transition_call).count(), 1);
    }

    // covers: BR-AG-STATE-001, BR-AG-STATE-002, BR-AG-VERIFY-001
    #[test]
    fn every_continuation_provider_call_is_reassembled_and_gate_checked() {
        let source = include_str!("turn_tool_loop.rs");
        let continuation_guard = source
            .find("if _round > 0")
            .expect("continuation rounds must have an explicit guard");
        let gate = source[continuation_guard..]
            .find("ent::gateway::check_gate().await")
            .map(|offset| continuation_guard + offset)
            .expect("continuation rounds must check Gate");
        let provider = source[gate..]
            .find("ent::gateway::stream_round")
            .map(|offset| gate + offset)
            .expect("Gate must precede the provider call");
        assert!(continuation_guard < gate && gate < provider);

        let tool_round_completed = source
            .rfind("TurnEvent::ToolRoundCompleted")
            .expect("ToolResult must return SM-AG-01 to Assembling");
        let rebuild = source[tool_round_completed..]
            .find("prepare_turn_prompt(")
            .map(|offset| tool_round_completed + offset)
            .expect("ToolResult must rebuild PromptPackage");
        let assembled = source[rebuild..]
            .find("TurnEvent::Assembled")
            .map(|offset| rebuild + offset)
            .expect("rebuilt PromptPackage must transition to AwaitingGate");
        assert!(tool_round_completed < rebuild && rebuild < assembled);
    }

    // covers: BR-AG-STATE-002, BR-SYS-STATE-007
    #[test]
    fn every_tool_call_recomputes_and_validates_a_bound_grant_before_executing() {
        let source = include_str!("turn_tool_loop.rs");
        let boundary = source
            .find("\"capability_grant_boundary\"")
            .expect("ToolCall must establish a fresh authorization boundary");
        let recompute = source[boundary..]
            .find("resolve_capability_grant_v2(")
            .map(|offset| boundary + offset)
            .expect("ToolCall must recompute CapabilityGrantV2");
        let validate = source[recompute..]
            .find("validate_requested_tool_for_grant(")
            .map(|offset| recompute + offset)
            .expect("fresh grant must be bound and validated");
        let manifest_guard = source[validate..]
            .find("tool_was_visible")
            .map(|offset| validate + offset)
            .expect("execution cannot expand beyond the current Prompt manifest");
        let executing = source[manifest_guard..]
            .find("TlEv::ValidateOk")
            .map(|offset| manifest_guard + offset)
            .expect("ToolExecuting must occur only after grant validation");
        assert!(boundary < recompute && recompute < validate && validate < executing);
    }
}
