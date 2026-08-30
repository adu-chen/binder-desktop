//! AgentTurn 的模型续轮、CapabilityGrant 校验、工具执行、结果注入与终态结算。

use super::edit_runtime::ag_propose_edit;
use super::prompt_runtime::{
    build_prompt_audit_record, current_doc_edit_requires_prior_read, is_current_doc_read_tool,
    prompt_audit_enabled, provider_messages_for_model, FinalPromptAuditLog,
};
use super::task_runtime::{edit_intent_has_terminal_tool_result, settle_optional_task_run};
use super::turn_control::{
    bind_agent_turn_task_run, install_agent_turn_abort_handle, lock_agent_turn_effect,
};
use super::turn_events::{
    emit_ag02, emit_ag_turn_cancelled, emit_ag_turn_done, emit_ag_turn_error,
    settle_and_emit_ag_turn_error,
};
use crate::ag;
use crate::ipc::{AgTurnEvent, CommandRequest, IpcError};
use crate::workspace::routes::route_plan_workspace_operation;
use crate::{ent, AppState};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

fn terminal_tool_is_missing(
    required_terminal_tool: Option<&str>,
    proposed_diff: bool,
    edit_blocked_reported: bool,
    project_reported: bool,
) -> bool {
    match required_terminal_tool {
        Some("propose_edit") => {
            !edit_intent_has_terminal_tool_result(proposed_diff, edit_blocked_reported)
        }
        // 成功的资源计划会在 DecisionRequest 事件分支提前结束；仍在模型循环中即尚未满足。
        Some("plan_workspace_operation") => true,
        Some("report_project") => !project_reported,
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
    if required_terminal_tool == Some("report_project") {
        return IpcError::new(
            "AG_PROJECT_REPORT_REQUIRED",
            "本轮是只读检查、搜索或分析请求，但尚未生成经读取证据核验的 ProjectReport；请先完成必要读取，再调用 report_project 如实列出已读、未读、来源与不确定性。",
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
 * term_ref: TERM-AG-014, TERM-AG-015, TERM-AG-016, TERM-AG-020, TERM-AG-021
 * boundary: in=PreparedPromptAgentContextFactsCurrentCapabilityGrantAndActiveAgentTurnWithOptionalTaskRun | out=ValidatedToolResultsProjectReportPendingDiffOrWorkspaceOperationDecisionRequestAndSingleAgentTurnTerminal | delegate=validate_requested_tool_for_grant,advance_tool_loop,exec_project_report | consumer=OptionalTaskRunStatus | does_not=executeToolOutsideGrantOrTreatProjectReportAsAuthorizationOrWorkspaceOperationPlanAsContentProposal
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
    prompt_package: ag::context::PromptPackageV1,
    agent_context_facts: crate::types::AgentContextFactsV1,
    current_doc_prompt_coverage: Option<&'static str>,
    mut messages: Vec<serde_json::Value>,
    tools: serde_json::Value,
    capability_grant: ag::tool_registry::CapabilityGrantV1,
    read_ctx: ag::blocks::AgentReadContext,
    active_doc_path: Option<String>,
    explicit_edit_target_set: ag::target_set::ExplicitEditTargetSet,
    ws_root: Option<String>,
    required_terminal_tool: Option<String>,
    current_doc_requires_tool_read: bool,
) -> Result<(), IpcError> {
    let mut proposed_diff = false;
    let mut edit_blocked_reported = false;
    let mut project_reported = false;
    let mut proposal_count = 0usize;

    // SM-AG-02 工具循环（BR-AG-STATE-002：注册/权限校验 + 上限）。dev 上限 6 轮。
    // 循环经状态机驱动 + 广播 sm:SM-AG-02（ModelStreaming→ToolValidating→ToolExecuting→ResultInjecting→
    // 续调 / FinalAnswer→LoopCompleted / 达上限→LoopAborted）。工具卡数据仍由 Channel 投影（互不替代）。
    use ag::tool::{ToolLoopEvent as TlEv, ToolLoopState as TlSt};
    const MAX_ROUNDS: usize = 6;
    // AG-TASK-01 F1a：单回合提案上限（整篇对齐分批最多 5 批；同轮可并行）。
    const MAX_PROPOSALS_PER_TURN: usize = 5;
    let mut current_doc_read_confirmed = !current_doc_requires_tool_read;
    let mut project_read_evidence = ag::tool_exec::ProjectReadEvidenceLedger::from_turn_start(
        &agent_context_facts,
        active_doc_path.as_deref(),
        current_doc_prompt_coverage,
    );
    // AG-CTX-01：本轮是否调用过任何工具（用于区分"已读未提案"与"零工具调用"的失败文案）。
    let mut any_tool_called = false;
    // 完整 Prompt 审计只保留本次 AgentTurn 最后一次实际发给模型的请求；回合退出时由 Drop
    // 统一输出一次，避免工具续轮把前序 messages/tools 反复打印。
    let mut final_prompt_audit = prompt_audit_enabled().then(FinalPromptAuditLog::default);
    let mut tool_loop_state = TlSt::ModelStreaming;
    macro_rules! advance_or_finish {
        ($event:expr) => {{
            match advance_tool_loop(&app, &turn_id, &mut tool_loop_state, $event) {
                Ok(next) => next,
                Err(error) => {
                    settle_and_emit_ag_turn_error(
                        &state,
                        task_run_id.as_deref(),
                        &on_event,
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
            emit_ag_turn_cancelled(&on_event, &terminal_latch, &app, &turn_id);
            return Ok(());
        }
        if _round > 0 {
            let gate_error = match ent::gateway::check_gate().await {
                Ok(ent::gateway::GateOutcome::Pass) => None,
                Ok(ent::gateway::GateOutcome::Deny) => Some(IpcError::new(
                    "AG_GATE_DENIED",
                    "续调门禁被服务端拒绝，本轮已停止。",
                    false,
                    true,
                )),
                Err(error) => Some(error),
            };
            if let Some(error) = gate_error {
                advance_or_finish!(TlEv::LoopError);
                if let Ok(conn) = state.db.lock() {
                    settle_optional_task_run(
                        &conn,
                        task_run_id.as_deref(),
                        &turn_id,
                        ag::task::TurnOutcome::Failed,
                    );
                }
                emit_ag_turn_error(&on_event, &terminal_latch, &app, &turn_id, &error);
                return Ok(());
            }
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
        let (abort_handle, abort_registration) = futures_util::future::AbortHandle::new_pair();
        if let Err(error) =
            install_agent_turn_abort_handle(&state, &session_id, &turn_id, abort_handle)
        {
            advance_or_finish!(TlEv::LoopError);
            settle_and_emit_ag_turn_error(
                &state,
                task_run_id.as_deref(),
                &on_event,
                &terminal_latch,
                &app,
                &turn_id,
                &error,
            );
            return Ok(());
        }
        let round = match futures_util::future::Abortable::new(
            ent::gateway::stream_round(
                &turn_id,
                _round + 1,
                &provider,
                &model,
                &msgs,
                &tools,
                &on_event,
            ),
            abort_registration,
        )
        .await
        {
            Err(_) => {
                advance_or_finish!(TlEv::Cancel);
                if let Ok(conn) = state.db.lock() {
                    settle_optional_task_run(
                        &conn,
                        task_run_id.as_deref(),
                        &turn_id,
                        ag::task::TurnOutcome::Cancelled,
                    );
                }
                emit_ag_turn_cancelled(&on_event, &terminal_latch, &app, &turn_id);
                return Ok(());
            }
            Ok(Err(error)) => {
                advance_or_finish!(TlEv::LoopError);
                if let Ok(conn) = state.db.lock() {
                    settle_optional_task_run(
                        &conn,
                        task_run_id.as_deref(),
                        &turn_id,
                        ag::task::TurnOutcome::Failed,
                    );
                }
                emit_ag_turn_error(&on_event, &terminal_latch, &app, &turn_id, &error);
                return Ok(());
            }
            Ok(Ok(outcome)) => outcome,
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
                emit_ag_turn_error(&on_event, &terminal_latch, &app, &turn_id, &error);
                return Ok(());
            }
            ent::gateway::RoundOutcome::Done { stop_reason } => {
                let plan_incomplete = match task_run_id.as_deref() {
                    None => Ok(false),
                    Some(task_run_id) => match state.db.lock() {
                        Ok(conn) => {
                            ag::task::has_incomplete_plan(&conn, task_run_id).map_err(|error| {
                                IpcError::new(
                                    "AG_TASK_PLAN_STATE_UNAVAILABLE",
                                    &format!("无法读取 TaskPlan 收束状态：{error}"),
                                    true,
                                    false,
                                )
                            })
                        }
                        Err(_) => Err(IpcError::new(
                            "AG_TASK_PLAN_STATE_UNAVAILABLE",
                            "TaskPlan 状态读取失败，不能将任务标记为完成",
                            true,
                            false,
                        )),
                    },
                };
                match plan_incomplete {
                    Ok(true) => {
                        let error = IpcError::new(
                            "AG_TASK_PLAN_INCOMPLETE",
                            "复杂任务的 TaskPlan 仍有未完成步骤；任务未被标记为完成。请按 RuntimeControl 的当前步骤继续执行。",
                            true,
                            false,
                        );
                        advance_or_finish!(TlEv::LoopError);
                        if let Ok(conn) = state.db.lock() {
                            settle_optional_task_run(
                                &conn,
                                task_run_id.as_deref(),
                                &turn_id,
                                ag::task::TurnOutcome::Failed,
                            );
                        }
                        emit_ag_turn_error(&on_event, &terminal_latch, &app, &turn_id, &error);
                        return Ok(());
                    }
                    Ok(false) => {}
                    Err(error) => {
                        advance_or_finish!(TlEv::LoopError);
                        if let Ok(conn) = state.db.lock() {
                            settle_optional_task_run(
                                &conn,
                                task_run_id.as_deref(),
                                &turn_id,
                                ag::task::TurnOutcome::Failed,
                            );
                        }
                        emit_ag_turn_error(&on_event, &terminal_latch, &app, &turn_id, &error);
                        return Ok(());
                    }
                }
                if terminal_tool_is_missing(
                    required_terminal_tool.as_deref(),
                    proposed_diff,
                    edit_blocked_reported,
                    project_reported,
                ) {
                    let error = required_terminal_tool_error(
                        required_terminal_tool.as_deref(),
                        any_tool_called,
                    );
                    advance_or_finish!(TlEv::LoopError);
                    if let Ok(conn) = state.db.lock() {
                        settle_optional_task_run(
                            &conn,
                            task_run_id.as_deref(),
                            &turn_id,
                            ag::task::TurnOutcome::Failed,
                        );
                    }
                    emit_ag_turn_error(&on_event, &terminal_latch, &app, &turn_id, &error);
                    return Ok(());
                }
                advance_or_finish!(TlEv::FinalAnswer);
                if let Ok(conn) = state.db.lock() {
                    settle_optional_task_run(
                        &conn,
                        task_run_id.as_deref(),
                        &turn_id,
                        ag::task::TurnOutcome::Completed,
                    );
                }
                emit_ag_turn_done(&on_event, &terminal_latch, &app, &turn_id, &stop_reason);
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
                        &on_event,
                        &terminal_latch,
                        &app,
                        &turn_id,
                        &error,
                    );
                    return Ok(());
                }
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
                messages.push(serde_json::json!({ "role": "assistant", "content": null, "tool_calls": tc_json }));

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
                        emit_ag_turn_cancelled(&on_event, &terminal_latch, &app, &turn_id);
                        return Ok(());
                    }
                    advance_or_finish!(TlEv::ToolRequest);
                    let params: serde_json::Value;
                    let arg_parse_error: Option<String> = match serde_json::from_str(&c.arguments) {
                        Ok(value) => {
                            params = value;
                            None
                        }
                        Err(err) => {
                            // AG-STRATEGY-01（A 修复）：arguments JSON 非法（如正文内未转义引号）→
                            // 回传明确错误，模型据此修正转义重发；此前伪装成"缺少目标 path"导致多轮空转。
                            params = serde_json::Value::Null;
                            Some(format!(
                                "工具 {} 参数不是合法 JSON（{}）。请检查引号与转义后重新调用。",
                                c.name, err
                            ))
                        }
                    };
                    let _ = on_event.send(AgTurnEvent::ToolRequest {
                        turn_id: turn_id.clone(),
                        tool_name: c.name.clone(),
                        params: params.clone(),
                    });
                    // AG-1.0-TASKRUN-REWORK-IMP-RUNTIME-01：实时当前动作（信息流底部状态条，不伪造）。
                    let _ = on_event.send(AgTurnEvent::StepProgress {
                        turn_id: turn_id.clone(),
                        action: if c.name == "propose_decision"
                            || c.name == "plan_workspace_operation"
                        {
                            "deciding".to_string()
                        } else {
                            "tool".to_string()
                        },
                        label: c.name.clone(),
                        detail: None,
                    });
                    let mut diff_event_after_tool: Option<(String, String)> = None;
                    let mut decision_event_after_tool = None;
                    let mut preview_checkpoint_after_tool = false;
                    let mut end_after_diff_proposal = false;
                    let mut mutation_guard = None;
                    // 注册 + 本轮 CapabilityGrant + 计划就绪步骤校验后方可执行。TaskPlan 只约束其
                    // 自己登记的工具步骤；辅助工具仍由 CapabilityGrant 决定且不会推进 cursor。
                    let tool_decision = arg_parse_error.is_none().then(|| {
                        ag::tool_registry::validate_requested_tool_for_grant(
                            &c.name,
                            &params,
                            &capability_grant,
                            &turn_id,
                        )
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
                    let task_run_prepare_error =
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
                                })
                                .and_then(|task_run_id| {
                                    ag::target_set::persist_for_task_run(
                                        &conn,
                                        &task_run_id,
                                        &explicit_edit_target_set,
                                    )
                                    .map(|_| task_run_id)
                                    .map_err(|error| {
                                        IpcError::new(
                                            "AG_TASK_DB_ERROR",
                                            &error.to_string(),
                                            true,
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
                                        Ok(()) => {
                                            if let Ok(conn) = state.db.lock() {
                                                let _ = ag::task::link_turn(
                                                    &conn,
                                                    &turn_id,
                                                    &created_task_run_id,
                                                );
                                            }
                                            None
                                        }
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
                    let mut plan_step_start = None;
                    let plan_guard_error = if let Err(error) = &scope_candidates {
                        Some(error.message.clone())
                    } else if task_run_prepare_error.is_some() {
                        task_run_prepare_error
                    } else if matches!(tool_decision.as_ref(), Some(ag::tool::ToolDecision::Allow))
                    {
                        if let Some(task_run_id) = task_run_id.as_deref() {
                            match state.db.lock() {
                                Ok(conn) => match ag::task::start_next_plan_step_for_tool(
                                    &conn,
                                    task_run_id,
                                    &c.name,
                                ) {
                                    Ok(ag::task::PlanToolStart::Untracked) => None,
                                    Ok(start @ ag::task::PlanToolStart::Started { .. }) => {
                                        plan_step_start = Some(start);
                                        None
                                    }
                                    Err(error) => {
                                        Some(format!("TaskPlan 阻止本次工具调用：{error}"))
                                    }
                                },
                                Err(_) => Some("TaskPlan 状态读取失败，已拒绝执行工具".to_string()),
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    let validation_allows_execution = arg_parse_error.is_none()
                        && plan_guard_error.is_none()
                        && matches!(tool_decision.as_ref(), Some(ag::tool::ToolDecision::Allow));
                    if validation_allows_execution {
                        advance_or_finish!(TlEv::ValidateOk);
                    } else {
                        advance_or_finish!(TlEv::ValidateFail);
                    }
                    let (mut result, mut success) = if let Some(error_msg) = arg_parse_error {
                        (error_msg, false)
                    } else if let Some(error_msg) = plan_guard_error {
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
                            ag::tool::ToolDecision::Allow if c.name == "report_project" => {
                                match ws_root.as_deref() {
                                    Some(root) => match ag::tool_exec::exec_project_report(
                                        std::path::Path::new(root),
                                        &params,
                                        &project_read_evidence,
                                    ) {
                                        Ok(result) => (result, true),
                                        Err(error) => (
                                            format!("ProjectReport 校验失败：{}", error.message),
                                            false,
                                        ),
                                    },
                                    None => (
                                        "无活动 workspace，无法核验 ProjectReport".to_string(),
                                        false,
                                    ),
                                }
                            }
                            // 任务级确认只创建 DecisionRequest，不修改文档或 Diff。成功后当前执行片段必须结束，
                            // 由下方事件分派统一发送确认卡和 Done，禁止进入下一轮模型循环。
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
                                                        "summary": "已创建任务级确认请求；当前执行片段已结束，正在等待用户确认或取消。",
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
                                                    "summary": "已生成工作区资源操作确认卡；确认前未修改磁盘，当前执行片段已结束。",
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
                                // AG-TASK-01 F1a：放开同回合多提案——整篇对齐等任务需分批提案（每批 ≤32 操作）。
                                // 保留上限防滥用：单回合 ≤MAX_PROPOSALS_PER_TURN。
                                if proposal_count >= MAX_PROPOSALS_PER_TURN {
                                    (
                                    serde_json::json!({
                                        "code": "AG_TOO_MANY_PROPOSALS",
                                        "message": format!("本回合已生成 {MAX_PROPOSALS_PER_TURN} 个修改建议，已达上限；请结束本回合，下一回合继续剩余批次。"),
                                        "retriable": false,
                                    })
                                    .to_string(),
                                    false,
                                )
                                } else if current_doc_edit_requires_prior_read(
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
                                            proposal_count += 1;
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
                    if let Some(ag::task::PlanToolStart::Started { plan_id, step_id }) =
                        plan_step_start.take()
                    {
                        match state.db.lock() {
                            Ok(conn) => {
                                if let Err(error) = ag::task::finish_plan_step_for_tool(
                                    &conn,
                                    task_run_id
                                        .as_deref()
                                        .expect("started plan step must retain its TaskRun"),
                                    &plan_id,
                                    &step_id,
                                    success,
                                ) {
                                    result = format!(
                                        "工具执行结果无法收束到 TaskPlan，已停止将其计为计划完成：{error}"
                                    );
                                    success = false;
                                }
                            }
                            Err(_) => {
                                result = "TaskPlan 状态写入失败，已停止将本次工具结果计为计划完成"
                                    .to_string();
                                success = false;
                            }
                        }
                    }
                    if success
                        && is_current_doc_read_tool(&c.name, &params, active_doc_path.as_deref())
                    {
                        current_doc_read_confirmed = true;
                    }
                    if success && c.name == "report_project" {
                        project_reported = true;
                    } else if success {
                        // report_project 是完成时证据；成功报告后若又发生任何工具活动，先前报告不再覆盖
                        // 当前最终读取边界，必须在结束前重新生成。
                        project_reported = false;
                        project_read_evidence
                            .record_successful_tool(&read_ctx, &c.name, &params, &result);
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
                        emit_ag_turn_cancelled(&on_event, &terminal_latch, &app, &turn_id);
                        return Ok(());
                    }
                    if validation_allows_execution {
                        advance_or_finish!(if success {
                            TlEv::ExecOk
                        } else {
                            TlEv::ExecFail
                        });
                    }
                    let _ = on_event.send(AgTurnEvent::ToolResult {
                        turn_id: turn_id.clone(),
                        tool_name: c.name.clone(),
                        result: result.clone(),
                        success,
                    });
                    // AG-1.0-IMP-TASKRUN-01：真实工具事件驱动步骤态（BR-SYS-STATE-008，best-effort）。
                    if let (Some(task_run_id), Ok(conn)) = (task_run_id.as_deref(), state.db.lock())
                    {
                        let _ = ag::task::record_tool_step(
                            &conn,
                            task_run_id,
                            &format!("tool-{}-{}", turn_id, c.id),
                            &c.name,
                            success,
                        );
                    }
                    if let Some((diff_id, rel_path)) = diff_event_after_tool {
                        // 提案＝W1/W2/W3 确认点（复用提案卡裁决，SM-AG-04 WaitingConfirm）。
                        if let (Some(task_run_id), Ok(conn)) =
                            (task_run_id.as_deref(), state.db.lock())
                        {
                            let _ = ag::task::record_proposal_step(
                                &conn,
                                task_run_id,
                                &format!("propose-{diff_id}"),
                                &format!("提案 {rel_path}"),
                            );
                        }
                        let _ = on_event.send(AgTurnEvent::DiffProposed {
                            turn_id: turn_id.clone(),
                            diff_id,
                            rel_path,
                            task_run_id: task_run_id.clone(),
                        });
                    }
                    if end_after_diff_proposal && success {
                        // 每张 Diff 都终止当前模型片段，避免在旧 read_ctx 上产生补丁栈。LogicalState
                        // 分支等待真实预览 Ack；background 分支等待既有迁移/裁决，均不改 DiffCard 或磁盘。
                        drop(mutation_guard);
                        if let (Some(task_run_id), Ok(conn)) =
                            (task_run_id.as_deref(), state.db.lock())
                        {
                            let _ = ag::task::link_turn(&conn, &turn_id, task_run_id);
                        }
                        advance_or_finish!(TlEv::Handoff);
                        emit_ag_turn_done(
                            &on_event,
                            &terminal_latch,
                            &app,
                            &turn_id,
                            if preview_checkpoint_after_tool {
                                "waiting_preview"
                            } else {
                                "waiting_diff_review"
                            },
                        );
                        return Ok(());
                    }
                    if let Some(decision) = decision_event_after_tool {
                        let _ = on_event.send(AgTurnEvent::DecisionRequestProposed {
                            turn_id: turn_id.clone(),
                            request_id: decision.request_id,
                            kind: decision.kind,
                            payload: decision.payload,
                        });
                        advance_or_finish!(TlEv::Handoff);
                        if let Ok(conn) = state.db.lock() {
                            // propose_decision 已将 TaskRun 置 WaitingConfirm；settle 只做同一真实状态的收束。
                            settle_optional_task_run(
                                &conn,
                                task_run_id.as_deref(),
                                &turn_id,
                                ag::task::TurnOutcome::Completed,
                            );
                        }
                        emit_ag_turn_done(
                            &on_event,
                            &terminal_latch,
                            &app,
                            &turn_id,
                            "waiting_confirm",
                        );
                        return Ok(());
                    }
                    drop(mutation_guard);
                    // 结果回注模型（SM-AG-02 ResultInjecting）。
                    messages.push(serde_json::json!({ "role": "tool", "tool_call_id": c.id, "content": result }));
                    let has_more_calls = call_index + 1 < calls.len();
                    let has_another_round = _round + 1 < MAX_ROUNDS;
                    if has_more_calls || has_another_round {
                        advance_or_finish!(TlEv::Injected);
                    }
                }
                // AG-CTX-01：工具结果不事后截断；同轮多个 ToolCall 已逐项回注并完成状态转移。
            }
        }
    }

    // 达工具循环上限（BR-AG-STATE-002 LoopAborted）：收束中止终态。
    let missing_terminal_tool = terminal_tool_is_missing(
        required_terminal_tool.as_deref(),
        proposed_diff,
        edit_blocked_reported,
        project_reported,
    );
    if missing_terminal_tool {
        let error =
            required_terminal_tool_error(required_terminal_tool.as_deref(), any_tool_called);
        advance_or_finish!(TlEv::LoopError);
        if let Ok(conn) = state.db.lock() {
            settle_optional_task_run(
                &conn,
                task_run_id.as_deref(),
                &turn_id,
                ag::task::TurnOutcome::Failed,
            );
        }
        emit_ag_turn_error(&on_event, &terminal_latch, &app, &turn_id, &error);
        return Ok(());
    }
    advance_or_finish!(TlEv::LoopLimit);
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
    emit_ag_turn_error(&on_event, &terminal_latch, &app, &turn_id, &error);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_tool_requirement_distinguishes_content_and_workspace_side_effects() {
        assert!(!terminal_tool_is_missing(None, false, false, false));
        assert!(terminal_tool_is_missing(
            Some("propose_edit"),
            false,
            false,
            false
        ));
        assert!(!terminal_tool_is_missing(
            Some("propose_edit"),
            true,
            false,
            false
        ));
        assert!(!terminal_tool_is_missing(
            Some("propose_edit"),
            false,
            true,
            false
        ));
        assert!(terminal_tool_is_missing(
            Some("plan_workspace_operation"),
            false,
            false,
            false,
        ));
        assert!(terminal_tool_is_missing(
            Some("report_project"),
            false,
            false,
            false
        ));
        assert!(!terminal_tool_is_missing(
            Some("report_project"),
            false,
            false,
            true
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
}
