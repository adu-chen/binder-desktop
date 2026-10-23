//! AgentTurn 的模型续轮、CapabilityGrant 校验、工具执行、结果注入与终态结算。

use super::edit_runtime::execute_proposal;
use super::prompt_runtime::{
    build_prompt_audit_record, current_doc_edit_requires_prior_read, is_current_doc_read_tool,
    prompt_audit_enabled, provider_messages_for_model, FinalPromptAuditLog,
};
use super::task_runtime::{edit_intent_has_terminal_tool_result, settle_optional_task_run};
use super::turn_control::{
    bind_agent_turn_task_run, install_agent_turn_abort_handle, park_agent_turn_for_clarification,
    park_agent_turn_for_decision, AgentTurnRuntimeCheckpoint,
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

pub(super) const MAX_MODEL_ROUNDS: usize = 6;
/// Malformed model-control batches may be corrected, but never consume unbounded rounds.
const MAX_PROTOCOL_CORRECTION_ROUNDS: usize = 2;

// A protocol repair must never turn an already issued non-read operation into a new call.
// Keep this conservative even for rejected/unknown receipts; ordinary successful runs do not
// enter this restriction. The trusted correction marker survives checkpoint continuation.
fn correction_final_only(messages: &[serde_json::Value]) -> bool {
    super::turn_prompt::final_only_continuation(messages) || has_non_read_calls(messages)
}

fn has_non_read_calls(messages: &[serde_json::Value]) -> bool {
    messages.iter().any(|message| {
        message
            .get("tool_calls")
            .and_then(|v| v.as_array())
            .is_some_and(|calls| {
                calls.iter().any(|call| {
                    let name = call
                        .pointer("/function/name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    !ag::tool_registry::lookup(name).is_some_and(|tool| {
                        tool.permission == ag::tool_registry::ToolPermission::ReadOnly
                    })
                })
            })
    })
}

fn checkpoint_after_read_clarification(messages: &mut Vec<serde_json::Value>) {
    // Parking cannot execute anything. A validated user clarification may choose a new
    // read scope on resume, under the same cumulative budgets. Never release a guard
    // protecting any prior non-read operation (including uncertain/rejected receipts).
    let read_guard = messages.iter().any(|message| {
        message["role"] == "system" && message["layer"] == "read_failure_final_only"
    });
    if read_guard && !has_non_read_calls(messages) {
        messages.retain(|message| {
            !(message["role"] == "system"
                && matches!(
                    message["layer"].as_str(),
                    Some("read_failure_final_only" | "protocol_correction_final_only")
                ))
        });
    }
}

fn correction_instruction(final_only: bool) -> serde_json::Value {
    serde_json::json!({
        "role":"system",
        "layer": if final_only { "protocol_correction_final_only" } else { "protocol_correction" },
        "content": if final_only {
            "上一响应的回合控制协议被拒绝，本批未执行。已有工具回执和操作证据保留，禁止重新提交工具或重放操作。只重新输出合法 final 或 clarify 控制封套及回答；只能陈述既有回执支持的事实。"
        } else {
            "上一响应的回合控制协议被拒绝，本批未执行。重新提交完整合法的响应：直接回答/澄清须以控制封套开头；原生工具在首个参数的 _binderTurn 提交响应级控制。不要输出代码围栏或前置说明，不重复已执行操作。"
        }
    })
}

fn rejected_tool_call_ids(messages: &[serde_json::Value]) -> BTreeSet<String> {
    messages
        .iter()
        .filter(|message| message.get("role").and_then(serde_json::Value::as_str) == Some("tool"))
        .filter_map(|message| {
            let id = message
                .get("tool_call_id")
                .and_then(serde_json::Value::as_str)?;
            let content = message.get("content").and_then(serde_json::Value::as_str)?;
            let value = serde_json::from_str::<serde_json::Value>(content).ok()?;
            (value
                .get("executionReceipt")
                .and_then(|receipt| receipt.get("executionStatus"))
                .and_then(serde_json::Value::as_str)
                == Some("rejected"))
            .then(|| id.to_owned())
        })
        .collect()
}

fn repeated_read_failure(
    seen: &mut BTreeSet<String>,
    tool_name: &str,
    params: &serde_json::Value,
    failure_code: &str,
) -> bool {
    let params_fingerprint =
        ag::context::sha256_text(&serde_json::to_string(params).unwrap_or_default());
    let signature = format!("{tool_name}:{failure_code}:{params_fingerprint}");
    !seen.insert(signature)
}

pub(super) fn continuation_execution_budget(
    messages: &[serde_json::Value],
    tool_limit: u32,
    round_limit: usize,
) -> (u32, usize) {
    let previous_batches = messages
        .iter()
        .filter_map(|message| {
            message
                .get("tool_calls")
                .and_then(serde_json::Value::as_array)
        })
        .collect::<Vec<_>>();
    // A rejected pre-execution call is a model correction attempt, not an executed tool.
    // Missing receipts remain conservatively counted for legacy/interrupted histories.
    let rejected_ids = rejected_tool_call_ids(messages);
    let used_tools = previous_batches
        .iter()
        .flat_map(|calls| calls.iter())
        .filter(|call| {
            !call
                .get("id")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|id| rejected_ids.contains(id))
        })
        .count();
    (
        tool_limit.saturating_sub(u32::try_from(used_tools).unwrap_or(u32::MAX)),
        round_limit.saturating_sub(previous_batches.len()),
    )
}

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

// Diagnostic overlaps are observations, not proof that the model used a source or that a path is authorized.
fn target_source_overlaps(
    value: &str,
    initial_user_input: &str,
    history: &[ag::context::HistTurn],
    model_messages: &[serde_json::Value],
    active_doc_path: Option<&str>,
    references: &[ag::context::PromptDataEnvelopeV1],
    continuation: &[serde_json::Value],
) -> serde_json::Value {
    serde_json::json!({
        "initialUserInput": initial_user_input.contains(value),
        "followupUserInput": continuation.iter().any(|item| {
            item.get("role").and_then(serde_json::Value::as_str) == Some("user")
                && item.get("content").and_then(serde_json::Value::as_str)
                    .is_some_and(|content| content.contains(value))
        }),
        "sameSessionHistorySubmitted": history.iter().any(|item| item.content.contains(value)),
        "sameSessionHistoryVisible": model_messages.iter().any(|item| {
            matches!(item.get("layer").and_then(serde_json::Value::as_str), Some("history" | "cross_doc_history"))
                && item.get("content").and_then(serde_json::Value::as_str)
                    .is_some_and(|content| content.contains(value))
        }),
        "activeTarget": active_doc_path == Some(value),
        "explicitReference": references.iter().any(|item| item.source_id.contains(value) || item.payload.contains(value)),
        "priorToolResult": continuation.iter().any(|item| {
            item.get("role").and_then(serde_json::Value::as_str) == Some("tool")
                && item.get("content").and_then(serde_json::Value::as_str)
                    .is_some_and(|content| content.contains(value))
        }),
    })
}

fn target_candidate_observation(
    intent: &ag::context::IntentResultV2,
    current_input: &str,
    history: &[ag::context::HistTurn],
    model_messages: &[serde_json::Value],
    active_doc_path: Option<&str>,
    references: &[ag::context::PromptDataEnvelopeV1],
    continuation: &[serde_json::Value],
) -> serde_json::Value {
    serde_json::json!({
        "revision": intent.revision,
        "ambiguity": intent.ambiguity,
        "candidates": intent.target_candidates.iter().map(|candidate| {
            serde_json::json!({
                "kind": candidate.kind,
                "hasPath": candidate.rel_path.is_some(),
                "pathHash": candidate.rel_path.as_deref().map(ag::context::sha256_text),
                "sourceOverlaps": candidate.rel_path.as_deref().map(|path| target_source_overlaps(
                    path, current_input, history, model_messages, active_doc_path, references, continuation,
                )),
            })
        }).collect::<Vec<_>>(),
    })
}

fn workspace_glob_query_observation(
    params: &serde_json::Value,
    current_input: &str,
    history: &[ag::context::HistTurn],
    model_messages: &[serde_json::Value],
    active_doc_path: Option<&str>,
    references: &[ag::context::PromptDataEnvelopeV1],
    continuation: &[serde_json::Value],
) -> serde_json::Value {
    let pattern = params.get("pattern").and_then(serde_json::Value::as_str);
    serde_json::json!({
        "hasPattern": pattern.is_some(),
        "patternHash": pattern.map(ag::context::sha256_text),
        "patternChars": pattern.map(|value| value.chars().count()),
        "patternShape": pattern.map(|value| if value.chars().any(|ch| matches!(ch, '*' | '?' | '[')) { "glob" } else { "exact" }),
        "sourceOverlaps": pattern.map(|value| target_source_overlaps(
            value, current_input, history, model_messages, active_doc_path, references, continuation,
        )),
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

fn safe_tool_receipt_summary(
    tool_name: &str,
    validation_allows_execution: bool,
    success: bool,
) -> String {
    if validation_allows_execution {
        safe_tool_result(tool_name, success)
    } else {
        format!("{tool_name} 请求受阻，工具未执行")
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
fn advance_tool_loop<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
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
pub(super) async fn run_turn_tool_loop<R: tauri::Runtime>(
    state: &AppState,
    app: tauri::AppHandle<R>,
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
    mut agent_context_facts: crate::types::AgentContextFacts,
    mut current_doc_prompt_coverage: Option<&'static str>,
    mut messages: Vec<serde_json::Value>,
    mut tools: serde_json::Value,
    mut capability_factors: ag::tool_registry::CapabilityFactorsV2,
    mut read_ctx: ag::blocks::AgentReadContext,
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
    mut active_snapshot: Option<ag::blocks::AgentEditableSnapshot>,
    on_context_request: Option<tauri::ipc::Channel<super::turn_runtime::AgentContextRequest>>,
) -> Result<(), IpcError> {
    let event_emitter = TurnEventEmitter::new(&state, &on_event, &turn_id);
    let (open_resource_path_hashes, selection_path_hashes) = match &agent_context_facts {
        crate::types::AgentContextFacts::V1(facts) => (
            facts
                .open_docs
                .iter()
                .map(|item| ag::context::sha256_text(&item.rel_path))
                .collect::<Vec<_>>(),
            facts
                .selections
                .iter()
                .map(|item| ag::context::sha256_text(&item.rel_path))
                .collect::<Vec<_>>(),
        ),
        crate::types::AgentContextFacts::V2(facts) => (
            facts
                .open_resources
                .iter()
                .map(|item| ag::context::sha256_text(&item.rel_path))
                .collect::<Vec<_>>(),
            facts
                .selections
                .iter()
                .map(|item| ag::context::sha256_text(&item.rel_path))
                .collect::<Vec<_>>(),
        ),
    };
    event_emitter.record_internal(
        "target_resolution_inputs",
        serde_json::json!({
            "sameSessionHistorySubmittedCount": history.len(),
            "sameSessionHistoryVisible": messages.iter().any(|item| {
                matches!(item.get("layer").and_then(serde_json::Value::as_str), Some("history" | "cross_doc_history"))
            }),
            "activeTargetPresent": active_doc_path.is_some(),
            "activeTargetPathHash": active_doc_path.as_deref().map(ag::context::sha256_text),
            "openResourcePathHashes": open_resource_path_hashes,
            "selectionPathHashes": selection_path_hashes,
            "explicitReferenceEnvelopeCount": reference_envelopes.len(),
            "selectionCount": agent_context_facts.selection_count(),
            "contextRevision": agent_context_facts.revision(),
        }),
    )?;
    let mut proposed_diff = false;
    let mut edit_blocked_reported = false;

    // SM-AG-02 工具循环（BR-AG-STATE-002：注册/权限校验 + 上限）。dev 上限 6 轮。
    // 循环经状态机驱动 + 广播 sm:SM-AG-02（ModelStreaming→ToolValidating→ToolExecuting→ResultRecording→
    // Recorded 后回归 SM-AG-01 并重新装配；FinalAnswer→LoopCompleted；达上限→LoopAborted。
    // 工具卡数据仍由 Channel 投影（互不替代）。
    use ag::tool::{ToolLoopEvent as TlEv, ToolLoopState as TlSt};
    let mut current_doc_read_confirmed = !current_doc_requires_tool_read;
    // AG-CTX-01：本轮是否调用过任何工具（用于区分"已读未提案"与"零工具调用"的失败文案）。
    let mut any_tool_called = false;
    let (previous_model_calls, mut protocol_correction_rounds) =
        event_emitter.protocol_attempt_counts()?;
    let mut failed_read_signatures = BTreeSet::new();
    // 完整 Prompt 审计只保留本次 AgentTurn 最后一次实际发给模型的请求；回合退出时由 Drop
    // 统一输出一次，避免工具续轮把前序 messages/tools 反复打印。
    let mut final_prompt_audit = prompt_audit_enabled().then(FinalPromptAuditLog::default);
    let mut tool_loop_state = TlSt::ModelStreaming;
    let mut capability_use_tracker = ag::tool_registry::CapabilityGrantUseTrackerV2::default();
    let (mut remaining_tool_calls, continuation_rounds) = continuation_execution_budget(
        &continuation_messages,
        capability_factors.remaining_tool_calls,
        MAX_MODEL_ROUNDS,
    );
    let remaining_rounds =
        continuation_rounds.min(MAX_MODEL_ROUNDS.saturating_sub(previous_model_calls));
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
    if remaining_tool_calls == 0 || remaining_rounds == 0 {
        let error = IpcError::new(
            "AG_TOOL_LOOP_LIMIT",
            "原回合执行预算已用尽；确认或恢复不会重置预算。此前结果保留，未启动下一次调用。",
            true,
            false,
        );
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
    for _round in 0..remaining_rounds {
        let model_call_ordinal = previous_model_calls + _round + 1;
        if cancelled.load(Ordering::Acquire) {
            // Between rounds SM-AG-02 is already frozen; only the outer AgentTurn is active.
            if !ag::tool::is_terminal(tool_loop_state) {
                advance_or_finish!(TlEv::Cancel);
            }
            if let Ok(conn) = state.db.lock() {
                settle_optional_task_run(
                    &conn,
                    task_run_id.as_deref(),
                    &turn_id,
                    ag::task::TurnOutcome::Cancelled,
                );
            }
            super::turn_events::emit_ag_turn_cancelled_from(
                &event_emitter,
                &terminal_latch,
                &app,
                &turn_id,
                turn_state,
            );
            return Ok(());
        }
        if _round > 0 {
            debug_assert_eq!(turn_state, ag::turn::TurnState::AwaitingGate);
            event_emitter.record_internal(
                "gate_requested",
                serde_json::json!({"attempt": model_call_ordinal}),
            )?;
            let gate_error = match super::turn_control::await_turn_operation(
                &state,
                &session_id,
                &turn_id,
                ent::gateway::check_gate(),
            )
            .await
            {
                Ok(ent::gateway::GateOutcome::Pass) => {
                    event_emitter.record_internal(
                        "gate_evaluated",
                        serde_json::json!({"attempt": model_call_ordinal, "outcome": "passed"}),
                    )?;
                    None
                }
                Ok(ent::gateway::GateOutcome::Deny) => {
                    event_emitter.record_internal(
                        "gate_evaluated",
                        serde_json::json!({"attempt": model_call_ordinal, "outcome": "denied"}),
                    )?;
                    Some(IpcError::new(
                        "AG_GATE_DENIED",
                        "续调门禁被服务端拒绝，本轮已停止。",
                        false,
                        true,
                    ))
                }
                Err(error) if error.code == "AG_TURN_CANCELLED" => {
                    event_emitter.record_internal(
                        "gate_evaluated",
                        serde_json::json!({"attempt": model_call_ordinal, "outcome": "cancelled"}),
                    )?;
                    // The preceding nested loop completed before this Gate wait.
                    if let Ok(conn) = state.db.lock() {
                        settle_optional_task_run(
                            &conn,
                            task_run_id.as_deref(),
                            &turn_id,
                            ag::task::TurnOutcome::Cancelled,
                        );
                    }
                    super::turn_events::emit_ag_turn_cancelled_from(
                        &event_emitter,
                        &terminal_latch,
                        &app,
                        &turn_id,
                        turn_state,
                    );
                    return Ok(());
                }
                Err(error) => {
                    event_emitter.record_internal(
                        "gate_evaluated",
                        serde_json::json!({
                            "attempt": model_call_ordinal,
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
                if error.code == "AG_GATE_DENIED" {
                    emit_ag_turn_denied(&event_emitter, &terminal_latch, &app, &turn_id, &error);
                } else {
                    emit_ag_turn_error_from(
                        &event_emitter,
                        &terminal_latch,
                        &app,
                        &turn_id,
                        &error,
                        turn_state,
                    );
                }
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
                model_call_ordinal,
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
                "roundIndex": model_call_ordinal,
                "provider": &provider,
                "model": &model,
                "protocolVersion": "binder-response-v2",
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
                model_call_ordinal,
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
                    serde_json::json!({"roundIndex": model_call_ordinal, "outcome": "cancelled"}),
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
                        "roundIndex": model_call_ordinal,
                        "outcome": if error.code.starts_with("AG_MODEL_CONTROL_") {
                            "model_protocol_rejected"
                        } else { "transport_error" },
                        "code": &error.code,
                        "prefixKind": output_decoder.prefix_kind(),
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
                        "roundIndex": model_call_ordinal,
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
        // A provider completion and an accepted Binder response are separate facts. Text
        // protocol errors are collected until the native response kind is known, before any
        // intent/task projection or tool execution. Network failures never enter correction.
        let final_only_after_rejection = correction_final_only(&continuation_messages);
        let response_control = match &round {
            ent::gateway::RoundOutcome::Done { .. } => {
                output_decoder.clone().finish().and_then(|control| {
                    if control.next_action == ag::model_output::ModelNextAction::Tool {
                        return Err(IpcError::new(
                            "AG_MODEL_CONTROL_INVALID",
                            "tool 动作缺少原生工具调用",
                            true,
                            false,
                        ));
                    }
                    if control.next_action == ag::model_output::ModelNextAction::Final
                        && visible_output_chars == 0
                    {
                        return Err(IpcError::new(
                            "AG_MODEL_FINAL_EMPTY",
                            "final 缺少可见回答",
                            true,
                            false,
                        ));
                    }
                    Ok(Some(control))
                })
            }
            ent::gateway::RoundOutcome::ToolCalls(_) => {
                if super::turn_prompt::final_only_continuation(&continuation_messages) {
                    Err(IpcError::new(
                        "AG_MODEL_CONTROL_INVALID",
                        "结果纠错禁止重新提交工具；请依据已保留回执回答或澄清",
                        true,
                        false,
                    ))
                } else {
                    output_decoder.clone().finish_optional()
                }
            }
            ent::gateway::RoundOutcome::Failed(_) => Ok(None),
        };
        let response_rejected = response_control.is_err();
        let mut final_only_correction_pending = false;
        let mut read_failure_final_pending = false;
        if let Err(error) = &response_control {
            protocol_correction_rounds += 1;
            let final_only = final_only_after_rejection;
            event_emitter.record_internal("model_control_rejected", serde_json::json!({
                "code": error.code, "prefixKind": output_decoder.prefix_kind(),
                "rejectionStage":"response_completion", "correctionAttempt":protocol_correction_rounds,
                "providerCompletionSeen":true, "finalOnly":final_only,
            }))?;
            // Once visible body has streamed, do not append a second answer to the same body.
            // Oversized quarantine failures also stop immediately in the callback above.
            if protocol_correction_rounds > MAX_PROTOCOL_CORRECTION_ROUNDS
                || _round + 1 == remaining_rounds
                || visible_output_chars > 0
            {
                let terminal_error = if visible_output_chars > 0 {
                    error.clone()
                } else {
                    IpcError::new(
                        "AG_MODEL_CORRECTION_LIMIT",
                        "模型响应协议纠错达到上限；本批未执行，此前证据保留，请显式重试。",
                        true,
                        false,
                    )
                };
                advance_or_finish!(TlEv::LoopError);
                settle_and_emit_ag_turn_error(
                    &state,
                    task_run_id.as_deref(),
                    &event_emitter,
                    &terminal_latch,
                    &app,
                    &turn_id,
                    &terminal_error,
                );
                return Ok(());
            }
            let instruction = correction_instruction(final_only);
            continuation_messages.push(instruction);
            advance_or_finish!(TlEv::ProtocolRejected);
        }
        match round {
            _ if response_rejected => {}
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
                let control = response_control
                    .expect("validated response")
                    .expect("done control");
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
                event_emitter.record_internal(
                    "target_candidate_observed",
                    target_candidate_observation(
                        &intent_result,
                        &raw_prompt,
                        &history,
                        &messages,
                        active_doc_path.as_deref(),
                        &reference_envelopes,
                        &continuation_messages,
                    ),
                )?;
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
                        checkpoint_after_read_clarification(&mut checkpoint_messages);
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
                // 直接回答且没有工具或证据结论时，不把打开资源的 session_metadata
                // 强行包装成项目调查报告；专业事实仍保留在 EvidenceLedger 中。
                let report_result = (|| -> Result<bool, IpcError> {
                    if intent_result.intent_type == ag::context::IntentTypeV2::Answer
                        && !any_tool_called
                        && control.findings.is_empty()
                    {
                        return Ok(true);
                    }
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
                let output_control = response_control.expect("validated optional control");
                // Validate the whole batch before creating any task/tool product. Continuation
                // messages belong to this AgentTurn and survive decision parking/resumption.
                let mut call_ids = continuation_messages
                    .iter()
                    .filter_map(|message| message.get("tool_calls").and_then(|v| v.as_array()))
                    .flatten()
                    .filter_map(|call| call.get("id").and_then(|v| v.as_str()))
                    .map(str::to_owned)
                    .collect::<BTreeSet<_>>();
                if calls
                    .iter()
                    .any(|call| call.id.trim().is_empty() || !call_ids.insert(call.id.clone()))
                {
                    let error = IpcError::new("AG_MODEL_TOOL_ID_INVALID",
                        "模型工具调用标识为空或重复；本批未执行。此前执行证据保留，请核对结果后显式重试。", true, false);
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
                let arguments = calls
                    .iter()
                    .map(|call| call.arguments.as_str())
                    .collect::<Vec<_>>();
                let (control, parsed_params, control_error) =
                    match ag::model_output::parse_tool_batch(&arguments, output_control) {
                        Ok((control, params)) => (Some(control), params, None),
                        Err(error) => {
                            if final_only_after_rejection {
                                continuation_messages.push(correction_instruction(true));
                            }
                            protocol_correction_rounds += 1;
                            event_emitter.record_internal(
                                "model_control_rejected",
                                serde_json::json!({
                                    "code": &error.code,
                                    "toolCallCount": calls.len(),
                                    "correctionAttempt": protocol_correction_rounds,
                                    "rejectionStage":"tool_batch",
                                    "conflictFieldsOnly": (error.code == "AG_MODEL_CONTROL_CONFLICT")
                                        .then_some(error.message.as_str()),
                                }),
                            )?;
                            if protocol_correction_rounds > MAX_PROTOCOL_CORRECTION_ROUNDS {
                                let terminal_error = IpcError::new(
                                    "AG_MODEL_CORRECTION_LIMIT",
                                    "模型连续返回无效工具协议；已达到有限纠错上限，本批未执行。请检查模型协议或显式重试。",
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
                                    &terminal_error,
                                );
                                return Ok(());
                            }
                            // 一整批均未执行；逐个回注失败，复用 ValidateFail→Recorded→Assembling→Gate。
                            // 不从工具名猜测意图，不复用旧语义替代本次模型结论。
                            (None, vec![serde_json::json!({}); calls.len()], Some(error))
                        }
                    };
                let mut correction_counted_this_round = control_error.is_some();
                if let Some(control) = control {
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
                    event_emitter.record_internal(
                        "target_candidate_observed",
                        target_candidate_observation(
                            &intent_result,
                            &raw_prompt,
                            &history,
                            &messages,
                            active_doc_path.as_deref(),
                            &reference_envelopes,
                            &continuation_messages,
                        ),
                    )?;
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
                    let arg_parse_error = control_error.as_ref().map(|error| {
                        serde_json::json!({
                            "code": &error.code,
                            "message": format!("{}；本批没有执行任何工具。请修正后重新提交，使用首个工具的 _binderTurn 或单个文本封套。", error.message),
                            "retriable": true,
                        }).to_string()
                    });
                    if c.name == "workspace_glob" && arg_parse_error.is_none() {
                        event_emitter.record_internal(
                            "workspace_glob_query_observed",
                            serde_json::json!({
                                "toolCallId": &c.id,
                                "query": workspace_glob_query_observation(
                                    &params,
                                    &raw_prompt,
                                    &history,
                                    &messages,
                                    active_doc_path.as_deref(),
                                    &reference_envelopes,
                                    &continuation_messages,
                                ),
                            }),
                        )?;
                    }
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
                    let mut diff_events_after_tool: Vec<(String, u32, u32, String, String)> =
                        Vec::new();
                    let mut decision_event_after_tool = None;
                    let mut preview_checkpoint_after_tool = false;
                    let mut end_after_diff_proposal = false;
                    // Preflight acquisition under the same hard boundaries; this temporary grant never executes a tool.
                    // The real one-use execution grant is computed below after acquisition.
                    let mut acquisition_factors = capability_factors.clone();
                    acquisition_factors.gate_passed = true;
                    acquisition_factors.gate_attempt = model_call_ordinal as u32;
                    acquisition_factors.remaining_tool_calls = remaining_tool_calls;
                    acquisition_factors.cancelled = cancelled.load(Ordering::Acquire);
                    acquisition_factors.terminal = terminal_latch.is_set();
                    let acquisition_grant = ag::tool_registry::resolve_capability_grant_v2(
                        &runtime_profile,
                        &turn_id,
                        &explicit_edit_target_set.paths(),
                        acquisition_factors,
                        ag::tool_registry::CapabilityGrantPurposeV2::ToolCall {
                            tool_call_id: c.id.clone(),
                            computed_at_seq: 1,
                        },
                    );
                    let acquisition_allowed = !read_failure_final_pending
                        && arg_parse_error.is_none()
                        && tool_is_visible_in_manifest(&tools, &c.name)
                        && matches!(
                            ag::tool_registry::validate_requested_tool_for_grant(
                                &c.name,
                                &params,
                                &acquisition_grant,
                                &turn_id,
                                &c.id,
                                1,
                                ws_root.as_deref().map(std::path::Path::new),
                                &mut Default::default()
                            ),
                            ag::tool::ToolDecision::Allow
                        );
                    let context_refresh_error = if acquisition_allowed
                        && ag::tool_registry::lookup(&c.name)
                            .is_some_and(|tool| tool.requires_fresh_host_context)
                    {
                        if let crate::types::AgentContextFacts::V2(previous) = &agent_context_facts
                        {
                            match super::turn_runtime::refresh_agent_context(
                                on_context_request.as_ref(),
                                previous,
                                &read_ctx,
                                &c.name,
                                &params,
                                cancelled.clone(),
                            )
                            .await
                            {
                                Ok((facts, context)) => {
                                    event_emitter.record_internal("context_frozen", serde_json::json!({"contextVersion": facts.version(), "contextRevision": facts.revision(), "activeTarget": previous.active_target, "toolCallId": &c.id, "readyDocuments": facts.ready_documents()}))?;
                                    active_snapshot = active_doc_path.as_deref().and_then(|path| {
                                        context
                                            .open_snapshot(path)
                                            .and_then(|_| context.snapshot(path).ok())
                                    });
                                    agent_context_facts = facts;
                                    read_ctx = context;
                                    None
                                }
                                Err(error) => Some(error),
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    let context_failure_summary = context_refresh_error.as_ref().map(|error| format!("[{}] 专业上下文未就绪、已变化或采集失败；本次工具未执行，请就绪后重新提交", error.code));
                    // 每个 ToolCall 在真实 GatePassed 后重新取得一次性 CapabilityGrantV2。Prompt Manifest
                    // 只决定模型当时可见的工具，不能拿来执行；TaskPlan/IntentResult/ToolResult 内容均不授权。
                    let computed_at_seq = event_emitter
                        .record_internal(
                            "capability_grant_boundary",
                            serde_json::json!({
                                "toolCallId": &c.id,
                                "gateAttempt": model_call_ordinal,
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
                    action_factors.gate_attempt = model_call_ordinal as u32;
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
                        if read_failure_final_pending {
                            ag::tool::ToolDecision::Reject(
                                "重复只读失败后禁止继续工具调用；此前证据保留，请说明未完成范围。"
                                    .into(),
                            )
                        } else if matches!(validated, ag::tool::ToolDecision::Allow)
                            && !tool_was_visible
                        {
                            ag::tool::ToolDecision::Reject(format!(
                                "工具 {} 不在当前 Prompt Tool Manifest 中",
                                c.name
                            ))
                        } else {
                            validated
                        }
                    });
                    let parameter_contract_rejection = tool_decision
                        .as_ref()
                        .and_then(|decision| match decision {
                            ag::tool::ToolDecision::Reject(reason) => Some(reason.as_str()),
                            ag::tool::ToolDecision::Allow => None,
                        })
                        .is_some_and(|reason| {
                            reason.starts_with(&format!("工具 {} 参数", c.name))
                                || reason.starts_with("工具参数结构")
                        });
                    if parameter_contract_rejection && !correction_counted_this_round {
                        if final_only_after_rejection {
                            // Append only after the assistant tool-call/result group is paired.
                            final_only_correction_pending = true;
                        }
                        correction_counted_this_round = true;
                        protocol_correction_rounds += 1;
                        event_emitter.record_internal(
                            "tool_parameter_contract_rejected",
                            serde_json::json!({
                                "code":"AG_TOOL_ARG",
                                "rejectionStage":"tool_validation",
                                "toolName": &c.name,
                                "parameterKeys": safe_tool_params(&params)["parameterKeys"],
                                "correctionAttempt": protocol_correction_rounds,
                            }),
                        )?;
                        if protocol_correction_rounds > MAX_PROTOCOL_CORRECTION_ROUNDS {
                            let terminal_error = IpcError::new(
                                "AG_MODEL_CORRECTION_LIMIT",
                                "模型连续提交不符合工具参数契约的请求；已达到有限纠错上限，本次未执行。请检查参数或显式重试。",
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
                                &terminal_error,
                            );
                            return Ok(());
                        }
                    }
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
                                                IpcError::new(
                                                    "AG_TASK_DB_ERROR",
                                                    &format!(
                                                        "TaskRun 无法固化当前回合范围：{error}"
                                                    ),
                                                    true,
                                                    false,
                                                )
                                            }),
                                            Err(_) => Some(IpcError::new(
                                                "WS_DB_BUSY",
                                                "数据库连接繁忙，无法固化 TaskRun 回合范围",
                                                true,
                                                false,
                                            )),
                                        },
                                        Err(error) => Some(error),
                                    }
                                }
                                Err(error) => Some(error),
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
                                task_run_prepare_error = Some(error);
                            }
                        }
                    }
                    let mut execution_failure = control_error
                        .clone()
                        .or_else(|| {
                            parameter_contract_rejection.then(|| {
                                ag::tool_exec::validate_read_arguments(&c.name, &params)
                                    .err()
                                    .unwrap_or_else(|| {
                                        IpcError::new(
                                            "AG_TOOL_ARG",
                                            "工具参数不符合声明；本次未执行，请依据工具契约修正。",
                                            false,
                                            false,
                                        )
                                    })
                            })
                        })
                        .or_else(|| {
                            read_failure_final_pending.then(|| {
                                IpcError::new(
                                    "AG_TOOL_REPEATED_FAILURE",
                                    "重复只读失败后停止工具调用，请依据已有回执说明未完成范围。",
                                    false,
                                    false,
                                )
                            })
                        })
                        .or_else(|| context_refresh_error.clone())
                        .or_else(|| scope_candidates.as_ref().err().cloned())
                        .or_else(|| task_run_prepare_error.clone());
                    let execution_guard_error = if context_refresh_error.is_some() {
                        context_refresh_error
                            .map(|error| format!("[{}] {}", error.code, error.message))
                    } else if let Err(error) = &scope_candidates {
                        Some(error.message.clone())
                    } else if task_run_prepare_error.is_some() {
                        task_run_prepare_error.map(|error| error.message)
                    } else {
                        None
                    };
                    let read_execution = if arg_parse_error.is_none()
                        && execution_guard_error.is_none()
                        && matches!(tool_decision.as_ref(), Some(ag::tool::ToolDecision::Allow))
                        && ws_root.is_some()
                        && ag::tool_registry::lookup(&c.name).is_some_and(|tool| {
                            tool.permission == ag::tool_registry::ToolPermission::ReadOnly
                        }) {
                        advance_or_finish!(TlEv::ValidateOk);
                        Some(
                            ag::tool_exec::exec_read_only_with_deadline(
                                read_ctx.clone(),
                                c.name.clone(),
                                params.clone(),
                                cancelled.clone(),
                            )
                            .await,
                        )
                    } else {
                        None
                    };
                    let effect_lock_error: Option<String> = None;
                    let capability_identity = crate::types::ToolCallReceiptIdentityV1 {
                        workspace_id: workspace_id_for_turn.clone(),
                        session_id: session_id.clone(),
                        turn_id: turn_id.clone(),
                        tool_call_id: c.id.clone(),
                        tool_name: c.name.clone(),
                        capability_id: ag::tool_registry::lookup(&c.name)
                            .map(|tool| tool.capability_id)
                            .unwrap_or("unregistered")
                            .into(),
                        grant_id: execution_grant.grant_id.clone(),
                        attempt: 1,
                    };
                    let validation_allows_execution = arg_parse_error.is_none()
                        && execution_guard_error.is_none()
                        && effect_lock_error.is_none()
                        && matches!(tool_decision.as_ref(), Some(ag::tool::ToolDecision::Allow));
                    if validation_allows_execution {
                        // Only a validated call that crosses the execution boundary consumes the
                        // tool budget. Rejected protocol/argument/permission calls consume rounds,
                        // but not the execution allowance.
                        remaining_tool_calls = remaining_tool_calls.saturating_sub(1);
                    }
                    if validation_allows_execution && read_execution.is_none() {
                        advance_or_finish!(TlEv::ValidateOk);
                    } else if !validation_allows_execution {
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
                                        execution_failure = Some(error.clone());
                                        (format!("阻断报告无效：{}", error.message), false)
                                    }
                                }
                            }
                            // 任务级确认只创建 DecisionRequest，不修改文档或 Diff。成功后当前执行片段必须
                            // 停在 AwaitingDecision；由下方事件分派统一发送确认卡，但不得发送 Done。
                            ag::tool::ToolDecision::Allow
                                if matches!(
                                    c.name.as_str(),
                                    "propose_decision" | "plan_workspace_operation"
                                ) =>
                            {
                                let task = task_run_id
                                    .clone()
                                    .expect("control capability binds TaskRun");
                                let name = c.name.clone();
                                let parameters = params.clone();
                                let target_paths = scope_candidates
                                    .as_ref()
                                    .ok()
                                    .and_then(|paths| paths.clone());
                                let source_turn = turn_id.clone();
                                let executed = ag::capability_runtime::execute(
                                    state.clone(), capability_identity.clone(), cancelled.clone(), std::time::Duration::from_secs(30),
                                    move |state| {
                                        if name == "propose_decision" {
                                            if !parameters.get("summary").and_then(|value| value.as_str()).is_some_and(|summary| !summary.trim().is_empty())
                                                || parameters.get("detail").is_some_and(|value| !value.is_string())
                                                || parameters.get("options").is_some_and(|value| !value.as_array().is_some_and(|options| options.iter().all(|option| option.as_str().is_some_and(|option| !option.trim().is_empty())))) {
                                                return Err(IpcError::new("AG_TOOL_ARG", "确认参数无效", false, false));
                                            }
                                            let kind = parameters.get("kind").and_then(|value| value.as_str()).ok_or_else(|| IpcError::new("AG_TOOL_ARG", "缺少 kind", false, false))?;
                                            let payload = serde_json::json!({
                                                "summary": parameters.get("summary"), "detail": parameters.get("detail"),
                                                "options": parameters.get("options"), "targetPaths": target_paths,
                                            });
                                            let conn = state.db.lock().map_err(|_| IpcError::new("WS_DB_BUSY", "数据库连接繁忙", true, false))?;
                                            let decision = ag::task::propose_decision(&conn, &task, kind, payload)
                                                .map_err(|error| IpcError::new("AG_TASK_DB_ERROR", &error.to_string(), true, false))?;
                                            let result = serde_json::json!({ "request_id": decision.request_id, "status": "waiting_confirm",
                                                "summary": "已创建任务级确认请求；原 AgentTurn 正在等待用户确认或取消。" });
                                            Ok((decision, result))
                                        } else {
                                            let mut parameters = parameters;
                                            if let Some(object) = parameters.as_object_mut() {
                                                object.insert("taskRunId".into(), serde_json::Value::String(task));
                                                object.insert("sourceTurnId".into(), serde_json::Value::String(source_turn.clone()));
                                            }
                                            let value = route_plan_workspace_operation(&CommandRequest {
                                                command: "ws.plan_operation".into(), correlation_id: format!("ag-wop-{source_turn}"), params: parameters,
                                            }, state)?;
                                            let decision: crate::types::DecisionRequestV1 = value.get("decision_request").cloned()
                                                .and_then(|value| serde_json::from_value(value).ok())
                                                .ok_or_else(|| IpcError::new("AG_DECISION_RECEIPT_MISSING", "资源操作计划已创建，但确认回执缺失；请核对效果", false, false))?;
                                            let result = serde_json::json!({ "plan_id": value.get("plan").and_then(|plan| plan.get("plan_id")),
                                                "status": "waiting_confirm", "summary": "已生成工作区资源操作确认卡；确认前未修改磁盘，原 AgentTurn 正在等待裁决。" });
                                            Ok((decision, result))
                                        }
                                    },
                                    |(decision, _)| vec![crate::types::ToolCallReceiptReferenceV1 {
                                        kind: crate::types::ToolCallReceiptReferenceKind::DecisionRequest, id: decision.request_id.clone(),
                                    }],
                                ).await;
                                match executed {
                                    Ok((decision, result)) => {
                                        decision_event_after_tool = Some(decision);
                                        (result.to_string(), true)
                                    }
                                    Err(error) => {
                                        execution_failure = Some(error.clone());
                                        (
                                            format!(
                                                "能力执行失败：[{}] {}",
                                                error.code, error.message
                                            ),
                                            false,
                                        )
                                    }
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
                                    execution_failure = Some(IpcError::new(
                                        "AG_CURRENT_DOC_READ_REQUIRED",
                                        "当前文档目标内容尚未读取",
                                        true,
                                        false,
                                    ));
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
                                    let guarded_result = execute_proposal(
                                        state.clone(),
                                        app.clone(),
                                        params.clone(),
                                        read_ctx.clone(),
                                        active_doc_path.clone(),
                                        explicit_edit_target_set.clone(),
                                        capability_identity.clone(),
                                        cancelled.clone(),
                                    )
                                    .await;
                                    match guarded_result {
                                        Ok(mut edit_result) => {
                                            let checkpoint_diff_id = edit_result
                                                .projections
                                                .first()
                                                .filter(|projection| {
                                                    matches!(
                                                        projection.draft_mode.as_str(),
                                                        "current_editor" | "open_session"
                                                    )
                                                })
                                                .map(|projection| projection.diff_id.clone());
                                            let needs_preview_checkpoint =
                                                checkpoint_diff_id.is_some();
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
                                                                checkpoint_diff_id
                                                                    .as_deref()
                                                                    .unwrap_or(""),
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
                                                            for projection in
                                                                &mut edit_result.projections
                                                            {
                                                                projection
                                                                    .preview_checkpoint_required =
                                                                    false;
                                                            }
                                                            // Diff 已经持久化。此处不得把它变成前端不可见的孤儿
                                                            // 提案，也绝不能在没有 checkpoint 的情况下续用旧 read_ctx。
                                                            eprintln!(
                                                                "[ag-preview] checkpoint unavailable for diff {}: {error}",
                                                                checkpoint_diff_id.as_deref().unwrap_or("")
                                                            );
                                                        }
                                                    }
                                                } else {
                                                    // 简单局部编辑不创建 TaskRun，也不伪造依赖任务身份的
                                                    // 自动续片段。Diff 仍作为当前 AgentTurn 的待审提案可见。
                                                    for projection in &mut edit_result.projections {
                                                        projection.preview_checkpoint_required =
                                                            false;
                                                    }
                                                }
                                            }
                                            if preview_checkpoint_after_tool {
                                                for projection in
                                                    edit_result.projections.iter_mut().skip(1)
                                                {
                                                    projection.preview_checkpoint_required = false;
                                                }
                                            }
                                            proposed_diff = true;
                                            // 无 LogicalState 的 background Diff 同样不得让模型在同一份旧 read_ctx
                                            // 上继续续编；它没有 PreviewAck checkpoint，需经既有迁移或裁决后由
                                            // 人类下一次显式操作开启新片段。
                                            end_after_diff_proposal = true;
                                            let group_count =
                                                u32::try_from(edit_result.projections.len())
                                                    .unwrap_or(u32::MAX);
                                            diff_events_after_tool = edit_result
                                                .projections
                                                .iter()
                                                .enumerate()
                                                .map(|(offset, projection)| {
                                                    (
                                                        edit_result.proposal_batch_id.clone(),
                                                        u32::try_from(offset + 1)
                                                            .unwrap_or(u32::MAX),
                                                        group_count,
                                                        projection.diff_id.clone(),
                                                        edit_result.rel_path.clone(),
                                                    )
                                                })
                                                .collect();
                                            let diff_ids = edit_result
                                                .projections
                                                .iter()
                                                .map(|projection| projection.diff_id.clone())
                                                .collect::<Vec<_>>();
                                            let result = serde_json::json!({
                                                "summary": format!("已生成 {} 张可独立裁决的修改卡（proposal_batch_id={}）。本结果仅确认建议卡已生成；尚未收到编辑器预览、用户接受或文件保存的回执。回复请使用「已提出建议，待确认」，不得声称已应用或已保存。", group_count, edit_result.proposal_batch_id),
                                                "proposal_batch_id": edit_result.proposal_batch_id,
                                                "diff_ids": diff_ids,
                                                "rel_path": edit_result.rel_path,
                                                "projections": edit_result.projections,
                                                "proposal_status": "pending_user_review",
                                                "editor_preview_status": "not_reported",
                                                "disk_persistence_status": "not_saved",
                                                "preview_checkpoint_required": preview_checkpoint_after_tool,
                                            });
                                            (result.to_string(), true)
                                        }
                                        Err(e) => {
                                            execution_failure = Some(e.clone());
                                            (format!("生成修改建议失败：{}", e.message), false)
                                        }
                                    }
                                }
                            }
                            // 只读工具：经 WS/KB 执行（限 active workspace）。
                            ag::tool::ToolDecision::Allow => match ws_root.as_deref() {
                                None => ("无活动 workspace，无法执行工具".to_string(), false),
                                Some(_) => {
                                    let executed = if let Some(result) = read_execution {
                                        result
                                    } else if c.name == "import_image_asset" {
                                        let root = std::path::PathBuf::from(
                                            ws_root.as_deref().expect("workspace exists"),
                                        );
                                        let name = c.name.clone();
                                        let parameters = params.clone();
                                        ag::capability_runtime::execute(
                                            state.clone(),
                                            capability_identity.clone(),
                                            cancelled.clone(),
                                            std::time::Duration::from_secs(30),
                                            move |_| {
                                                ag::tool_exec::exec_structure_tool(
                                                    &root,
                                                    &name,
                                                    &parameters,
                                                )
                                            },
                                            |_| vec![],
                                        )
                                        .await
                                    } else {
                                        ag::tool_exec::exec_read_only(&read_ctx, &c.name, &params)
                                    };
                                    match executed {
                                        Ok(r) => (r, true),
                                        Err(e) => {
                                            execution_failure = Some(e.clone());
                                            (format!("工具执行失败：{}", e.message), false)
                                        }
                                    }
                                }
                            },
                        }
                    };
                    let repeated_read_failure = if validation_allows_execution && !success {
                        let is_read_only = ag::tool_registry::lookup(&c.name).is_some_and(|tool| {
                            tool.permission == ag::tool_registry::ToolPermission::ReadOnly
                        });
                        execution_failure.as_ref().is_some_and(|error| {
                            is_read_only
                                && repeated_read_failure(
                                    &mut failed_read_signatures,
                                    &c.name,
                                    &params,
                                    &error.code,
                                )
                        })
                    } else {
                        false
                    };
                    if let Some(error) = &execution_failure {
                        event_emitter.record_internal("tool_failure_observed", serde_json::json!({
                            "toolName": &c.name, "toolCallId": &c.id, "code": &error.code,
                            "rejectionStage": if validation_allows_execution { "tool_execution" } else { "tool_validation" },
                            "hasExpectedHash": params.get("expected_hash").is_some(),
                            "nonInitialPage": params.get("offset").and_then(|v|v.as_u64()).is_some_and(|offset|offset>0),
                            "requestFingerprint": ag::context::sha256_text(&params.to_string()),
                            "requestedReadOffset": params.get("offset").and_then(|value|value.as_u64()),
                            "requestedLimitChars": params.get("limit_chars").and_then(|value|value.as_u64()),
                            "requestedDirectoryLimit": params.get("limit").and_then(|value|value.as_u64()),
                            "requestedBlockCount": params.get("blocks").and_then(|value|value.as_array()).map(|blocks|blocks.len()),
                            "resultLimitChars": ag::prompt_runtime::TOOL_RESULT_MAX_SERIALIZED_CHARS,
                            "repeatedFailure": repeated_read_failure,
                        }))?;
                    }
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
                    if let Some(error) = execution_failure.as_ref().filter(|error| {
                        matches!(
                            error.code.as_str(),
                            "AG_CAPABILITY_TIMEOUT"
                                | "AG_CAPABILITY_WORKER_FAILED"
                                | "AG_EFFECT_FACT_UNAVAILABLE"
                                | "AG_EFFECT_PENDING"
                        )
                    }) {
                        advance_or_finish!(TlEv::LoopError);
                        settle_and_emit_ag_turn_error(
                            &state,
                            task_run_id.as_deref(),
                            &event_emitter,
                            &terminal_latch,
                            &app,
                            &turn_id,
                            error,
                        );
                        return Ok(());
                    }
                    if success && c.name == "read_doc" {
                        if let Ok(read) = serde_json::from_str::<serde_json::Value>(&result) {
                            event_emitter.record_internal("read_result_observed", serde_json::json!({
                                "toolCallId": c.id, "toolName": c.name,
                                "requestFingerprint": ag::context::sha256_text(&params.to_string()),
                                "readView": read.get("view").and_then(|value|value.as_str()).unwrap_or("structure"),
                                "requestedBlockCount": params.get("blocks").and_then(|value|value.as_array()).map(|blocks|blocks.len()),
                                "returnedBlockCount": read.get("blocks").and_then(|value|value.as_array()).map(|blocks|blocks.len()),
                                "readOffset": read.get("offset"), "readEndOffset": read.get("end_offset"),
                                "readTotalChars": read.get("total_chars"), "readHasMore": read.get("has_more"),
                                "readComplete": read.get("complete"), "readVersion": read.get("version_hash"),
                                "resultChars": serde_json::to_string(&result).expect("String serialization").chars().count(),
                                "resultLimitChars": ag::prompt_runtime::TOOL_RESULT_MAX_SERIALIZED_CHARS,
                            }))?;
                        }
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
                    let mut receipt_references = diff_events_after_tool
                        .iter()
                        .map(
                            |(_, _, _, diff_id, _)| crate::types::ToolCallReceiptReferenceV1 {
                                kind: crate::types::ToolCallReceiptReferenceKind::PendingDiff,
                                id: diff_id.clone(),
                            },
                        )
                        .collect::<Vec<_>>();
                    if let Some(decision) = &decision_event_after_tool {
                        receipt_references.push(crate::types::ToolCallReceiptReferenceV1 {
                            kind: crate::types::ToolCallReceiptReferenceKind::DecisionRequest,
                            id: decision.request_id.clone(),
                        });
                    }
                    let execution_receipt = ag::tool_exec::tool_call_receipt(
                        crate::types::ToolCallReceiptIdentityV1 {
                            workspace_id: workspace_id_for_turn.clone(),
                            session_id: session_id.clone(),
                            turn_id: turn_id.clone(),
                            tool_call_id: c.id.clone(),
                            tool_name: c.name.clone(),
                            capability_id: ag::tool_registry::lookup(&c.name)
                                .map(|tool| tool.capability_id)
                                .unwrap_or("unregistered")
                                .into(),
                            grant_id: execution_grant.grant_id.clone(),
                            attempt: 1,
                        },
                        validation_allows_execution,
                        success,
                        execution_failure.as_ref(),
                        receipt_references,
                        evidence_ids.clone(),
                    );
                    let tool_result_event = event_emitter.emit_recorded(
                        &terminal_latch,
                        AgTurnEventPayload::ToolResult {
                            tool_call_id: c.id.clone(),
                            tool_name: c.name.clone(),
                            result: context_failure_summary.unwrap_or_else(|| {
                                safe_tool_receipt_summary(
                                    &c.name,
                                    validation_allows_execution,
                                    success,
                                )
                            }),
                            success,
                            evidence_ids: evidence_ids.clone(),
                            execution_receipt: Some(execution_receipt.clone()),
                            diagnostic: (!success).then(|| {
                                crate::ipc::AgentFailureDiagnostic::observed(
                                    if validation_allows_execution {
                                        "tool_execution"
                                    } else {
                                        "tool_validation"
                                    },
                                    execution_failure
                                        .as_ref()
                                        .map(|error| error.code.as_str())
                                        .unwrap_or(""),
                                )
                            }),
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
                    if repeated_read_failure {
                        event_emitter.record_internal(
                            "repeated_read_failure_detected",
                            serde_json::json!({
                                "toolName": &c.name,
                                "failureCode": execution_failure.as_ref().map(|error| error.code.as_str()),
                                "attempt": 2,
                                "parametersLogged": false,
                            }),
                        )?;
                    }
                    for (proposal_batch_id, group_index, group_count, diff_id, rel_path) in
                        diff_events_after_tool
                    {
                        event_emitter.emit(
                            &terminal_latch,
                            AgTurnEventPayload::DiffProposed {
                                proposal_batch_id,
                                group_index,
                                group_count,
                                diff_id,
                                rel_path,
                                task_run_id: task_run_id.clone(),
                            },
                        )?;
                    }
                    let model_tool_result = serde_json::json!({
                        "toolResult": serde_json::from_str::<serde_json::Value>(&result)
                            .unwrap_or_else(|_| serde_json::Value::String(result.clone())),
                        "evidenceIds": evidence_ids,
                        "executionReceipt": execution_receipt,
                    })
                    .to_string();
                    let tool_message = serde_json::json!({
                        "role": "tool",
                        "tool_call_id": c.id,
                        "content": model_tool_result,
                    });
                    messages.push(tool_message.clone());
                    continuation_messages.push(tool_message);
                    if let Some(decision) = decision_event_after_tool {
                        if final_only_correction_pending {
                            continuation_messages.push(correction_instruction(true));
                        }
                        let request_id = decision.request_id.clone();
                        advance_or_finish!(TlEv::DecisionHandoff);
                        turn_state =
                            ag::turn::transition(turn_state, ag::turn::TurnEvent::DecisionProposed)
                                .ok_or_else(sm_error)?;
                        let current_task_run_id = task_run_id
                            .as_deref()
                            .expect("DecisionRequest must retain its TaskRun");
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
                    // Stop tool retries, not the user's entire conversation. The next response
                    // may only explain existing evidence or ask for clarification. It still
                    // passes the original Gate, prompt budget, cancellation and round limits.
                    read_failure_final_pending |= repeated_read_failure;
                    let has_more_calls = call_index + 1 < calls.len();
                    let loop_budget_reached = remaining_tool_calls == 0
                        || (_round + 1 == remaining_rounds && !has_more_calls);
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
                if _round + 1 == remaining_rounds || remaining_tool_calls == 0 {
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
            }
        }
        // ToolResult/Proposal 或响应拒绝已落事实后先回 SM-AG-01 Assembling；不能从 ResultRecording
        // 直接调用 provider，也不能复用首次 PromptPackage/CapabilityGrant。
        if final_only_correction_pending {
            continuation_messages.push(correction_instruction(true));
        }
        if read_failure_final_pending {
            continuation_messages.push(serde_json::json!({
                "role":"system", "layer":"read_failure_final_only",
                "content":"相同只读请求已重复失败，禁止再调用任何工具。请只输出合法 final 或 clarify：说明已有成功证据、失败的读取及未完成范围，不能将失败读取视为已读，不能宣称原业务任务成功；如无法继续，明确说明需要用户补充或缩小什么范围。不要重放任何已有操作。"
            }));
        }
        turn_state = ag::turn::transition(
            turn_state,
            if response_rejected {
                ag::turn::TurnEvent::ModelProtocolRejected
            } else {
                ag::turn::TurnEvent::ToolRoundCompleted
            },
        )
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
            budget_measurement,
            budget_sources,
            history_selection,
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
            (model_call_ordinal + 1) as u32,
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
        event_emitter.record_internal("history_assembled", history_selection)?;
        if let Err(error) = event_emitter.record_prompt_budget(
            if response_rejected {
                "protocol_correction"
            } else {
                "tool_result"
            },
            &budget_measurement,
            budget_sources,
        ) {
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
        if budget_measurement.estimated_tokens > super::prompt_runtime::PROMPT_INPUT_LIMIT {
            let error = IpcError::new(
                "AG_PROMPT_BUDGET_EXCEEDED",
                &super::prompt_runtime::budget_failure_message(
                    &budget_measurement,
                    if response_rejected {
                        "protocol_correction"
                    } else {
                        "tool_result"
                    },
                ),
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
                "nextRound": model_call_ordinal + 1,
                "intentRevision": intent_result.revision,
                "contextTraceId": prompt_package.context_trace_id,
                "toolManifestHash": prompt_package.tool_manifest_hash,
            }),
        )?;
        turn_state = ag::turn::transition(turn_state, ag::turn::TurnEvent::Assembled)
            .ok_or_else(sm_error)?;
        emit_ag01(&app, &turn_id, turn_state);
    }

    debug_assert!(false, "每个 provider 结果都应在循环内部返回或推进");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // covers: BR-AG-STATE-002
    #[test]
    fn checkpoint_continuation_preserves_total_execution_budget() {
        let messages = vec![
            serde_json::json!({"role":"assistant", "tool_calls":[{"id":"A"},{"id":"B"}]}),
            serde_json::json!({"role":"tool", "tool_call_id":"A"}),
            serde_json::json!({"role":"assistant", "tool_calls":[{"id":"C"}]}),
        ];
        assert_eq!(continuation_execution_budget(&messages, 6, 6), (3, 4));
        assert_eq!(continuation_execution_budget(&messages, 2, 1), (0, 0));
        assert_eq!(continuation_execution_budget(&[], 6, 6), (6, 6));
        assert!(!correction_final_only(&[
            serde_json::json!({"role":"assistant","tool_calls":[{"function":{"name":"workspace_glob"}}]})
        ]));
        assert!(correction_final_only(&[
            serde_json::json!({"role":"assistant","tool_calls":[{"function":{"name":"plan_workspace_operation"}}]})
        ]));
        assert!(correction_final_only(&[correction_instruction(true)]));
    }

    #[test]
    fn rejected_calls_do_not_consume_execution_budget_but_legacy_calls_do() {
        let messages = vec![
            serde_json::json!({"role":"assistant", "tool_calls":[
                {"id":"rejected"}, {"id":"executed"}, {"id":"legacy"}
            ]}),
            serde_json::json!({"role":"tool", "tool_call_id":"rejected", "content":
                "{\"executionReceipt\":{\"executionStatus\":\"rejected\"}}"}),
            serde_json::json!({"role":"tool", "tool_call_id":"executed", "content":
                "{\"executionReceipt\":{\"executionStatus\":\"succeeded\"}}"}),
        ];
        assert_eq!(continuation_execution_budget(&messages, 6, 6), (4, 5));
    }

    #[test]
    fn read_failure_convergence_requires_same_tool_arguments_and_error() {
        let mut seen = BTreeSet::new();
        let params = serde_json::json!({"path":"oversized.txt"});
        assert!(!repeated_read_failure(
            &mut seen,
            "read_doc",
            &params,
            "AG_TOOL_RESULT_TOO_LARGE"
        ));
        assert!(repeated_read_failure(
            &mut seen,
            "read_doc",
            &params,
            "AG_TOOL_RESULT_TOO_LARGE"
        ));
        assert!(!repeated_read_failure(
            &mut seen,
            "read_doc",
            &serde_json::json!({"path":"smaller.txt"}),
            "AG_TOOL_RESULT_TOO_LARGE"
        ));
        assert!(!repeated_read_failure(
            &mut seen,
            "read_doc",
            &params,
            "AG_READ_VERSION_STALE"
        ));
    }

    // covers: BR-AG-STATE-002, BR-AG-DATA-005
    #[test]
    fn clarification_can_release_only_read_failure_guards_not_non_read_protection() {
        let guards = vec![
            serde_json::json!({"role":"system","layer":"read_failure_final_only"}),
            correction_instruction(true),
        ];
        let mut read = guards.clone();
        read.push(
            serde_json::json!({"role":"assistant","tool_calls":[{"function":{"name":"read_doc"}}]}),
        );
        checkpoint_after_read_clarification(&mut read);
        assert!(!super::super::turn_prompt::final_only_continuation(&read));
        assert_eq!(read.len(), 1);
        let mut non_read = guards.clone();
        non_read.push(serde_json::json!({"role":"assistant","tool_calls":[{"function":{"name":"propose_decision"}}]}));
        checkpoint_after_read_clarification(&mut non_read);
        assert!(super::super::turn_prompt::final_only_continuation(
            &non_read
        ));
        assert_eq!(non_read.len(), 3);
        let mut only_protocol = vec![correction_instruction(true)];
        checkpoint_after_read_clarification(&mut only_protocol);
        assert_eq!(only_protocol.len(), 1);
    }

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
        assert_eq!(
            safe_tool_receipt_summary("read_doc", false, false),
            "read_doc 请求受阻，工具未执行"
        );
        assert_eq!(
            safe_tool_receipt_summary("read_doc", true, false),
            "read_doc 执行失败"
        );
    }

    // covers: BR-AG-DATA-004, BR-AG-OBS-001
    #[test]
    fn target_resolution_diagnostics_distinguish_history_overlap_without_logging_paths() {
        let path = "qa/old-target.txt";
        let history = vec![ag::context::HistTurn {
            workspace_id: "ws".into(),
            session_id: "session".into(),
            role: "user".into(),
            content: format!("此前提过 {path}"),
            rel_path: None,
            turn_id: Some("old-turn".into()),
        }];
        let mut intent = ag::context::IntentResultV2::unresolved();
        intent
            .target_candidates
            .push(ag::context::IntentTargetCandidateV2 {
                kind: "explicit_document".into(),
                rel_path: Some(path.into()),
            });
        let visible_history = vec![serde_json::json!({
            "role": "system", "layer": "history", "content": format!("此前提过 {path}"),
        })];
        let observed = target_candidate_observation(
            &intent,
            "把那个 TXT 文件改名",
            &history,
            &visible_history,
            None,
            &[],
            &[],
        );
        assert_eq!(
            observed["candidates"][0]["sourceOverlaps"]["initialUserInput"],
            false
        );
        assert_eq!(
            observed["candidates"][0]["sourceOverlaps"]["sameSessionHistorySubmitted"],
            true
        );
        assert_eq!(
            observed["candidates"][0]["sourceOverlaps"]["sameSessionHistoryVisible"],
            true
        );
        assert_eq!(
            observed["candidates"][0]["pathHash"],
            ag::context::sha256_text(path)
        );
        assert!(!observed.to_string().contains(path));

        let query = workspace_glob_query_observation(
            &serde_json::json!({"pattern": path}),
            "把那个 TXT 文件改名",
            &history,
            &visible_history,
            None,
            &[],
            &[],
        );
        assert_eq!(query["patternShape"], "exact");
        assert_eq!(query["patternHash"], ag::context::sha256_text(path));
        assert_eq!(query["sourceOverlaps"]["sameSessionHistorySubmitted"], true);
        assert!(!query.to_string().contains(path));

        let broad = workspace_glob_query_observation(
            &serde_json::json!({"pattern": "**/*.txt"}),
            "把那个 TXT 文件改名",
            &[],
            &[],
            None,
            &[],
            &[],
        );
        assert_eq!(broad["patternShape"], "glob");
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
            .find("ent::gateway::check_gate()")
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
