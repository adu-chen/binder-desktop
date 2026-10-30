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
    emit_ag01, emit_ag02, emit_ag_turn_denied, emit_ag_turn_done, emit_ag_turn_error,
    emit_ag_turn_error_from, settle_and_emit_ag_turn_error, settle_and_emit_ag_turn_error_from,
    sm_error, TurnEventEmitter,
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

// A protocol repair must never turn an already issued non-read operation into a new call.
// A matching runtime rejection proves the call never crossed the execution boundary.
// Missing/conflicting receipts and any actual/unknown non-read effect stay conservative.
fn correction_final_only(messages: &[serde_json::Value], scope: (&str, &str, &str)) -> bool {
    super::turn_prompt::final_only_continuation(messages) || has_non_read_calls(messages, scope)
}

fn has_non_read_calls(messages: &[serde_json::Value], scope: (&str, &str, &str)) -> bool {
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
                    }) && !rejected_before_execution(messages, call, name, scope)
                })
            })
    })
}

fn rejected_before_execution(
    messages: &[serde_json::Value],
    call: &serde_json::Value,
    name: &str,
    scope: (&str, &str, &str),
) -> bool {
    let Some(id) = call["id"].as_str().filter(|id| !id.is_empty()) else {
        return false;
    };
    let mut replies = messages
        .iter()
        .filter(|m| m["role"] == "tool" && m["tool_call_id"] == id);
    let Some(reply) = replies.next() else {
        return false;
    };
    if replies.next().is_some() {
        return false;
    }
    let receipt = reply["content"]
        .as_str()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
        .and_then(|mut v| v.get_mut("executionReceipt").map(serde_json::Value::take))
        .and_then(|v| serde_json::from_value::<crate::types::ToolCallReceiptV1>(v).ok());
    receipt.is_some_and(|r| {
        r.identity.tool_call_id == id
            && r.identity.tool_name == name
            && r.identity.workspace_id == scope.0
            && r.identity.session_id == scope.1
            && r.identity.turn_id == scope.2
            && r.execution_status == crate::types::ToolCallExecutionStatus::Rejected
            && r.completion_status == crate::types::ToolCallCompletionStatus::Blocked
            && r.effect_status == crate::types::ToolCallEffectStatus::NotStarted
            && r.next_action == crate::types::ToolCallNextAction::RespondBlocked
            && r.failure.is_some()
            && r.references.is_empty()
            && r.evidence_ids.is_empty()
    })
}

fn checkpoint_after_read_clarification(
    messages: &mut Vec<serde_json::Value>,
    scope: (&str, &str, &str),
) {
    // Parking cannot execute anything. A validated user clarification may choose a new
    // read scope on resume, under the same cumulative budgets. Never release a guard
    // protecting any prior non-read operation (including uncertain/rejected receipts).
    let read_guard = messages.iter().any(|message| {
        message["role"] == "system" && message["layer"] == "read_failure_final_only"
    });
    if read_guard && !has_non_read_calls(messages, scope) {
        messages.retain(|message| {
            !(message["role"] == "system"
                && matches!(
                    message["layer"].as_str(),
                    Some("read_failure_final_only" | "protocol_correction_final_only")
                ))
        });
    }
}

#[cfg(test)]
fn correction_instruction(final_only: bool) -> serde_json::Value {
    correction_instruction_for_protocol(final_only, ag::model_output::BATCH_TOOL_PROTOCOL)
}

/**
 * @GOV
 * codes:    BR-AG-DATA-004-DATA-AG-AG-TURN-062
 * type:     DATA
 * chain:    AG-TURN
 * rules:    BR-AG-DATA-004
 * boundary: in=RejectionKindAndFrozenTransportProtocol | out=ProtocolCorrectionSystemMessage | does_not=grantToolsReplayOperationsOrChangeBudget
 */
fn correction_instruction_for_protocol(final_only: bool, protocol: &str) -> serde_json::Value {
    serde_json::json!({
        "role":"system",
        "layer": if final_only { "protocol_correction_final_only" } else { "protocol_correction" },
        "content": if final_only {
            "上一响应的控制封套或工具参数被拒绝。受拒调用未执行，其他调用的事实以已有回执为准。工具权限已冻结，禁止重新提交工具或重放操作。只重新输出合法 final 或 clarify 控制封套及回答；只能陈述既有回执支持的事实。"
        } else if protocol == ag::model_output::BATCH_TOOL_PROTOCOL {
            "上一响应的控制封套或工具参数被拒绝。受拒调用未执行，其他调用的事实以已有回执为准。直接回答/澄清（包括工具失败、受阻或解释错误）都须先输出合法final/clarify控制封套，再输出正文；不得以Markdown标题开头。修正未执行参数可在原剩余额度内用新调用ID提交一次binder_tool_batch，参数含唯一control及calls[{toolName,params}]，业务params不重复控制；不能增加范围或重放已执行/效果未知操作。不要输出代码围栏或前置说明。"
        } else {
            "上一响应的控制封套或工具参数被拒绝。受拒调用未执行，其他调用的事实以已有回执为准。直接回答/澄清须先输出合法final/clarify控制封套，再输出正文；不得以Markdown标题开头。可在原剩余额度内用新调用ID直接调用本轮授权的业务工具：每次调用的arguments顶层都必须带完整_binderTurn，并完整保留原有业务参数；不要使用calls/control传输封套，也不要把_binderTurn嵌套进params。若上次被拒原因是缺少_binderTurn，重试时必须补上完整控制，不能沿用被拒参数。多项仅限独立只读，计划、提案和裁决须单项提交。不能增加范围或重放已执行/效果未知操作。不要输出代码围栏或前置说明。"
        }
    })
}

#[cfg(test)]
fn response_correction_instruction(final_only: bool, rejection_kind: &str) -> serde_json::Value {
    response_correction_instruction_for_protocol(
        final_only,
        rejection_kind,
        ag::model_output::BATCH_TOOL_PROTOCOL,
    )
}

fn response_correction_instruction_for_protocol(
    final_only: bool,
    rejection_kind: &str,
    protocol: &str,
) -> serde_json::Value {
    let mut instruction = correction_instruction_for_protocol(final_only, protocol);
    let detail = match rejection_kind {
        "text_tool_action" if final_only => "本次实际错误：没有原生工具调用，模型却把nextAction=tool写进了文本；该文本未触发工具。当前工具权限已冻结，只能以合法final/clarify封套说明未执行和已有回执，不能重放操作。",
        "text_tool_action" if protocol == ag::model_output::BATCH_TOOL_PROTOCOL => "本次实际错误：没有原生工具调用，模型却把nextAction=tool写进了文本；该文本未触发工具。若仍需工具，下一响应必须调用API原生函数binder_tool_batch，把control.nextAction=tool和calls放在函数arguments内；不要在content中打印工具JSON、控制标签或供应商内部工具标记。若无需工具则以合法final/clarify文本封套如实说明未执行。",
        "text_tool_action" => "本次实际错误：没有原生工具调用，模型却把nextAction=tool写进了文本；该文本未触发工具。若仍需工具，下一响应须直接调用本轮授权的原生业务工具，在每个arguments中携带nextAction=tool的_binderTurn；不要在content中打印工具JSON或控制标签。无需工具则以合法final/clarify封套说明未执行。",
        "control_incomplete" => "本次实际错误：文本控制封套未以精确的</binder-turn-control>结束，正文未展示。请完整输出一个final或clarify控制封套，再输出正文；不要输出伪工具标记。",
        "control_missing" => "本次实际错误：文本回答没有以<binder-turn-control>开始，正文未展示。直接回答或澄清时须先输出合法final/clarify控制封套。",
        _ => "本次实际错误：文本控制封套结构无效，正文未展示。请按本轮协议重新输出完整合法控制封套，不要重复无效结构。",
    };
    let base = instruction["content"].as_str().unwrap_or_default();
    instruction["content"] = serde_json::json!(format!("{detail}\n{base}"));
    instruction
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

pub(super) fn tool_is_visible_in_manifest(tools: &serde_json::Value, tool_name: &str) -> bool {
    tools.as_array().is_some_and(|definitions| {
        definitions.iter().any(|definition| {
            definition["function"]["name"].as_str() == Some(tool_name)
                || (definition["function"]["name"].as_str()
                    == Some(ag::model_output::TOOL_BATCH_NAME)
                    && definition
                        .pointer("/function/parameters/properties/calls/items/oneOf")
                        .and_then(|v| v.as_array())
                        .is_some_and(|variants| {
                            variants.iter().any(|variant| {
                                variant
                                    .pointer("/properties/toolName/enum")
                                    .and_then(|v| v.as_array())
                                    .is_some_and(|names| {
                                        names.iter().any(|name| name.as_str() == Some(tool_name))
                                    })
                            })
                        }))
        })
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
 * codes:    BR-AG-DATA-011-DATA-AG-AG-TOOL-062
 * type:     DATA
 * chain:    AG-TOOL
 * rules:    BR-AG-DATA-011
 * boundary: in=ValidatedToolCallBoundGrantCreatedPendingDiffIdsAndDecisionRequestId | out=ToolCallReceiptIdentityReceiptReferencesAndModelToolMessage | does_not=fabricateExecutionOutcomeEvidenceOrReferenceUncreatedProducts
 */
fn tool_call_receipt_identity(
    workspace_id: &str,
    session_id: &str,
    turn_id: &str,
    tool_call_id: &str,
    tool_name: &str,
    grant_id: &str,
) -> crate::types::ToolCallReceiptIdentityV1 {
    crate::types::ToolCallReceiptIdentityV1 {
        workspace_id: workspace_id.to_string(),
        session_id: session_id.to_string(),
        turn_id: turn_id.to_string(),
        tool_call_id: tool_call_id.to_string(),
        tool_name: tool_name.to_string(),
        capability_id: ag::tool_registry::lookup(tool_name)
            .map(|tool| tool.capability_id)
            .unwrap_or("unregistered")
            .into(),
        grant_id: grant_id.to_string(),
        attempt: 1,
    }
}

fn tool_result_references(
    pending_diff_ids: impl IntoIterator<Item = String>,
    decision_request_id: Option<&str>,
) -> Vec<crate::types::ToolCallReceiptReferenceV1> {
    let mut references = pending_diff_ids
        .into_iter()
        .map(|diff_id| crate::types::ToolCallReceiptReferenceV1 {
            kind: crate::types::ToolCallReceiptReferenceKind::PendingDiff,
            id: diff_id,
        })
        .collect::<Vec<_>>();
    if let Some(request_id) = decision_request_id {
        references.push(crate::types::ToolCallReceiptReferenceV1 {
            kind: crate::types::ToolCallReceiptReferenceKind::DecisionRequest,
            id: request_id.to_string(),
        });
    }
    references
}

fn model_tool_result_message(
    tool_call_id: &str,
    result: &str,
    evidence_ids: &[String],
    receipt: &crate::types::ToolCallReceiptV1,
) -> serde_json::Value {
    let content = serde_json::json!({
        "toolResult": serde_json::from_str::<serde_json::Value>(result)
            .unwrap_or_else(|_| serde_json::Value::String(result.to_string())),
        "evidenceIds": evidence_ids,
        "executionReceipt": receipt,
    })
    .to_string();
    serde_json::json!({
        "role": "tool",
        "tool_call_id": tool_call_id,
        "content": content,
    })
}

/**
 * @GOV
 * codes:    BR-AG-DATA-012-DATA-AG-AG-TOOL-063
 * type:     DATA
 * chain:    AG-TOOL
 * rules:    BR-AG-DATA-012
 * boundary: in=ProposalBatchIdRelPathDiffProjectionsAndPreviewCheckpointOutcome | out=PendingReviewToolResultProjection | does_not=claimEditorPreviewAcceptanceOrDiskPersistence
 */
fn proposal_result_projection(
    proposal_batch_id: &str,
    rel_path: &str,
    projections: &[crate::types::DiffProjection],
    preview_checkpoint_required: bool,
) -> serde_json::Value {
    let group_count = u32::try_from(projections.len()).unwrap_or(u32::MAX);
    let diff_ids = projections
        .iter()
        .map(|projection| projection.diff_id.clone())
        .collect::<Vec<_>>();
    serde_json::json!({
        "summary": format!("已生成 {group_count} 张可独立裁决的修改卡（proposal_batch_id={proposal_batch_id}）。本结果仅确认建议卡已生成；尚未收到编辑器预览、用户接受或文件保存的回执。回复请使用「已提出建议，待确认」，不得声称已应用或已保存。"),
        "proposal_batch_id": proposal_batch_id,
        "diff_ids": diff_ids,
        "rel_path": rel_path,
        "projections": projections,
        "proposal_status": "pending_user_review",
        "editor_preview_status": "not_reported",
        "disk_persistence_status": "not_saved",
        "preview_checkpoint_required": preview_checkpoint_required,
    })
}

/**
 * @GOV
 * codes:    BR-AG-STATE-005-EFFECT-AG-AG-TURN-071
 * type:     EFFECT
 * chain:    AG-TURN
 * rules:    BR-AG-STATE-005
 * boundary: in=FrozenExecutionBudgetCompletionStatusRemainingCallCountsAndStructureWarning | out=CompletionStageEventUserClosingChunkTaskRunOutcomeAndDoneTerminal | does_not=resetExecutionBudgetReopenToolsOrMarkPartialCompletionAsCompleted
 */
fn completed_turn_stop_reason<'a>(
    budget_closing: bool,
    completion: Option<&'a ag::model_output::ModelCompletion>,
    structure_warning: Option<&str>,
    stop_reason: &'a str,
) -> &'a str {
    if budget_closing {
        "budget_exhausted"
    } else if completion.is_none() && structure_warning.is_some() {
        "answer_incomplete"
    } else {
        completion
            .map(|completion| completion.status.as_str())
            .unwrap_or(stop_reason)
    }
}

fn completion_settles_as_failed(
    budget_closing: bool,
    completion: Option<&ag::model_output::ModelCompletion>,
    side_effect_intent: &ag::context::SideEffectIntentV2,
) -> bool {
    budget_closing
        || completion.is_some_and(|completion| {
            completion.status == "partial"
                || completion.status == "blocked"
                || (completion.status == "report_only"
                    && side_effect_intent != &ag::context::SideEffectIntentV2::None)
        })
}

#[allow(clippy::too_many_arguments)]
fn settle_completed_turn<R: tauri::Runtime>(
    state: &AppState,
    app: &tauri::AppHandle<R>,
    emitter: &TurnEventEmitter<'_>,
    latch: &ag::turn::TerminalLatch,
    turn_id: &str,
    task_run_id: Option<&str>,
    budget_closing: bool,
    completion: Option<&ag::model_output::ModelCompletion>,
    side_effect_intent: &ag::context::SideEffectIntentV2,
    structure_warning: Option<&str>,
    stop_reason: &str,
    remaining_tool_calls: u32,
    remaining_rounds: usize,
) -> Result<(), IpcError> {
    let final_stop_reason =
        completed_turn_stop_reason(budget_closing, completion, structure_warning, stop_reason);
    if budget_closing || completion.is_some() {
        // 记录收尾时的剩余额度：回合以 partial 结束却仍有余量，说明是模型提前收工，
        // 而不是门禁拦停。把两个计数一起落事件，事后可直接区分"真耗尽"和"误报"。
        emitter.record_internal("completion_stage", serde_json::json!({"completionStatus":final_stop_reason,"budgetClosing":budget_closing,"remainingToolCalls":remaining_tool_calls,"remainingRounds":remaining_rounds}))?;
        emitter.emit(latch, AgTurnEventPayload::Chunk { text: if budget_closing { "\n\n本次执行额度已到，尚未完成的范围未被标记为完成。此前结果和效果记录保留；需要继续时请明确提出，未知效果须先核对。" } else { "\n\n以上为当前阶段说明，不是资源已修改或已进入正式确认等待的凭证；实际效果以执行回执为准。" }.into() })?;
    }
    if let Ok(conn) = state.db.lock() {
        settle_optional_task_run(
            &conn,
            task_run_id,
            turn_id,
            if completion_settles_as_failed(budget_closing, completion, side_effect_intent) {
                ag::task::TurnOutcome::Failed
            } else {
                ag::task::TurnOutcome::Completed
            },
        );
    }
    emit_ag_turn_done(
        emitter,
        latch,
        app,
        turn_id,
        final_stop_reason,
        ag::turn::TurnState::Finalizing,
    );
    Ok(())
}

/**
 * @GOV
 * codes:    BR-AG-DATA-013-RB-AG-AG-TOOL-065
 * type:     RB
 * chain:    AG-TOOL, AG-TURN
 * rules:    BR-AG-DATA-013
 * boundary: in=GatewayRoundOutcomeStreamedModelControlBudgetClosingAndContinuationGuards | out=ValidatedModelTurnControlOrIpcError | does_not=projectIntentCreateTaskRunOrExecuteToolBeforeValidation
 */
fn validate_round_response(
    round: &ent::gateway::RoundOutcome,
    decoder: &ag::model_output::ModelOutputDecoder,
    budget_closing: bool,
    workspace_plan_completed: bool,
    visible_output_chars: usize,
    any_tool_called: bool,
    continuation_messages: &[serde_json::Value],
) -> Result<Option<ag::model_output::ModelTurnControl>, IpcError> {
    match round {
        ent::gateway::RoundOutcome::Done { .. } => decoder.clone().finish().and_then(|control| {
            if budget_closing && control.next_action != ag::model_output::ModelNextAction::Final {
                return Err(IpcError::new(
                    "AG_MODEL_CONTROL_INVALID",
                    "额度收尾仅允许final，不得创建工具或无法恢复的等待",
                    true,
                    false,
                ));
            }
            if !budget_closing
                && control.next_action == ag::model_output::ModelNextAction::Final
                && control.intent.side_effect_intent
                    == ag::context::SideEffectIntentV2::StructurePlan
                && control.completion.is_none()
                && !workspace_plan_completed
            {
                return Err(required_terminal_tool_error(
                    Some("plan_workspace_operation"),
                    any_tool_called,
                ));
            }
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
        }),
        ent::gateway::RoundOutcome::ToolCalls(_) => {
            if budget_closing || super::turn_prompt::final_only_continuation(continuation_messages)
            {
                Err(IpcError::new(
                    "AG_MODEL_CONTROL_INVALID",
                    "结果纠错禁止重新提交工具；请依据已保留回执回答或澄清",
                    true,
                    false,
                ))
            } else {
                decoder.clone().finish_optional()
            }
        }
        ent::gateway::RoundOutcome::Failed(_) => Ok(None),
    }
}

/**
 * @GOV
 * codes:    BR-SYS-STATE-008-RB-AG-AG-TOOL-064
 * type:     RB
 * chain:    AG-TOOL, AG-TURN
 * rules:    BR-SYS-STATE-008
 * term_ref: TERM-AG-012, TERM-AG-015
 * boundary: in=ValidatedModelControlObservationFactsAndExplicitEditTargetSet | out=IntentUpdatedRouteUpdatedTargetCandidateObservationAndOptionalTaskRunProjection | consumer=BR-SYS-STATE-008 | does_not=treatModelControlAsAuthorizationOrCreateTaskRunForSimpleTurn
 */
#[allow(clippy::too_many_arguments)]
fn apply_control_and_observe(
    state: &AppState,
    emitter: &TurnEventEmitter<'_>,
    latch: &ag::turn::TerminalLatch,
    session_id: &str,
    workspace_id: &str,
    turn_id: &str,
    explicit_edit_target_set: &ag::target_set::ExplicitEditTargetSet,
    task_run_id: &mut Option<String>,
    intent_result: &mut ag::context::IntentResultV2,
    control: &ag::model_output::ModelTurnControl,
    raw_prompt: &str,
    history: &[ag::context::HistTurn],
    messages: &[serde_json::Value],
    active_doc_path: Option<&str>,
    references: &[ag::context::PromptDataEnvelopeV1],
    continuation_messages: &[serde_json::Value],
) -> Result<(), IpcError> {
    *intent_result = apply_model_control(emitter, latch, intent_result, control)?;
    emitter.record_internal(
        "target_candidate_observed",
        target_candidate_observation(
            intent_result,
            raw_prompt,
            history,
            messages,
            active_doc_path,
            references,
            continuation_messages,
        ),
    )?;
    ensure_route_task_projection(
        state,
        emitter,
        session_id,
        workspace_id,
        turn_id,
        explicit_edit_target_set,
        task_run_id,
        control,
    )
}

/**
 * @GOV
 * codes:    BR-AG-OBS-001-EFFECT-AG-AG-TURN-072
 * type:     EFFECT
 * chain:    AG-TURN
 * rules:    BR-AG-OBS-001
 * term_ref: TERM-AG-021, TERM-AG-023
 * boundary: in=IntentResultVisibleAnswerEvidenceLedgerAndModelFindings | out=ProjectReportTurnEventOrSkipForToolFreeAnswer | effect=DB-rebuild_evidence_ledger | does_not=acceptModelClaimedCoverageOrTreatProjectReportAsCompletionGate
 */
#[allow(clippy::too_many_arguments)]
fn emit_project_report(
    state: &AppState,
    emitter: &TurnEventEmitter<'_>,
    latch: &ag::turn::TerminalLatch,
    workspace_id: &str,
    turn_id: &str,
    intent: &ag::context::IntentResultV2,
    active_doc_path: Option<&str>,
    findings: &[ag::model_output::ModelFinalFindingV2],
    any_tool_called: bool,
) -> Result<bool, IpcError> {
    // 直接回答且没有工具或证据结论时，不把打开资源的 session_metadata
    // 强行包装成项目调查报告；专业事实仍保留在 EvidenceLedger 中。
    if intent.intent_type == ag::context::IntentTypeV2::Answer
        && !any_tool_called
        && findings.is_empty()
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
        super::turn_events::rebuild_evidence_ledger(&conn, workspace_id, turn_id)?
    };
    let project_report = ag::tool_exec::derive_project_report(
        turn_id,
        requested_report_scope(intent, active_doc_path),
        &evidence_ledger,
        findings,
    );
    emitter.emit(
        latch,
        AgTurnEventPayload::ProjectReport {
            report: project_report,
        },
    )
}

/// 记录一次 ToolCall 的结果、证据与回执，并把同一回执回注模型。
/// 规则由 BR-AG-DATA-011（回执/回注）、BR-AG-OBS-001（证据账本）与
/// BR-AG-STATE-002（ToolResult 事实）的既有块承接；本函数不新增授权、终态或错误分类判断。
struct ToolResultFacts<'a> {
    workspace_id: &'a str,
    turn_id: &'a str,
    tool_call_id: &'a str,
    tool_name: &'a str,
    params: &'a serde_json::Value,
    read_ctx: &'a ag::blocks::AgentReadContext,
    result: &'a str,
    success: bool,
    validation_allows_execution: bool,
    execution_failure: Option<&'a IpcError>,
    context_failure_summary: Option<String>,
    identity: &'a crate::types::ToolCallReceiptIdentityV1,
    diff_events: &'a [(String, u32, u32, String, String)],
    decision: Option<&'a crate::types::DecisionRequestV1>,
    repeated_read_failure: bool,
    control_error: bool,
    task_run_id: Option<&'a str>,
}

/**
 * @GOV
 * codes:    BR-AG-DATA-011-EFFECT-AG-AG-TOOL-070
 * type:     EFFECT
 * chain:    AG-TOOL
 * rules:    BR-AG-DATA-011
 * boundary: in=ExecutedToolCallResultEvidenceAndReceipt | out=ToolResultEventEvidenceLedgerItemsDiffProposedEventsAndModelToolMessage | delegate=tool_call_receipt_identity,tool_result_references,model_tool_result_message | consumer=BR-AG-DATA-011 | does_not=fabricateEvidenceOrEmitSecondTerminal
 */
fn record_tool_result(
    emitter: &TurnEventEmitter<'_>,
    latch: &ag::turn::TerminalLatch,
    facts: ToolResultFacts<'_>,
    messages: &mut Vec<serde_json::Value>,
    continuation_messages: &mut Vec<serde_json::Value>,
) -> Result<bool, IpcError> {
    let pending_evidence = if facts.success {
        ag::tool_exec::successful_tool_evidence(
            facts.read_ctx,
            facts.tool_name,
            facts.params,
            facts.result,
        )
    } else {
        Vec::new()
    };
    let evidence_ids = ag::tool_exec::evidence_ids(
        facts.workspace_id,
        facts.turn_id,
        facts.tool_call_id,
        &pending_evidence,
    );
    let execution_receipt = ag::tool_exec::tool_call_receipt(
        facts.identity.clone(),
        facts.validation_allows_execution,
        facts.success,
        facts.execution_failure,
        tool_result_references(
            facts
                .diff_events
                .iter()
                .map(|(_, _, _, diff_id, _)| diff_id.clone()),
            facts.decision.map(|decision| decision.request_id.as_str()),
        ),
        evidence_ids.clone(),
    );
    let tool_result_event = emitter.emit_recorded(
        latch,
        AgTurnEventPayload::ToolResult {
            tool_call_id: facts.tool_call_id.to_string(),
            tool_name: facts.tool_name.to_string(),
            result: facts.context_failure_summary.unwrap_or_else(|| {
                safe_tool_receipt_summary(
                    facts.tool_name,
                    facts.validation_allows_execution,
                    facts.success,
                )
            }),
            success: facts.success,
            evidence_ids: evidence_ids.clone(),
            execution_receipt: Some(execution_receipt.clone()),
            diagnostic: (!facts.success).then(|| {
                crate::ipc::AgentFailureDiagnostic::observed(
                    if facts.validation_allows_execution {
                        "tool_execution"
                    } else {
                        "tool_validation"
                    },
                    facts
                        .execution_failure
                        .map(|error| error.code.as_str())
                        .unwrap_or(""),
                )
            }),
        },
    )?;
    let Some(source_event) = tool_result_event else {
        return Ok(false);
    };
    let evidence_items = ag::tool_exec::materialize_evidence(
        facts.workspace_id,
        facts.turn_id,
        &source_event.event_id,
        source_event.seq,
        facts.tool_call_id,
        pending_evidence,
    );
    if !evidence_items.is_empty() {
        emitter.record_internal(
            "evidence_recorded",
            serde_json::json!({ "evidenceItems": evidence_items }),
        )?;
    }
    if facts.repeated_read_failure {
        emitter.record_internal(
            "repeated_read_failure_detected",
            serde_json::json!({
                "toolName": facts.tool_name,
                "failureCode": facts.execution_failure.map(|error| error.code.as_str()),
                "attempt": 2,
                "parametersLogged": false,
            }),
        )?;
    }
    for (proposal_batch_id, group_index, group_count, diff_id, rel_path) in facts.diff_events {
        emitter.emit(
            latch,
            AgTurnEventPayload::DiffProposed {
                proposal_batch_id: proposal_batch_id.clone(),
                group_index: *group_index,
                group_count: *group_count,
                diff_id: diff_id.clone(),
                rel_path: rel_path.clone(),
                task_run_id: facts.task_run_id.map(str::to_string),
            },
        )?;
    }
    let tool_message = model_tool_result_message(
        facts.tool_call_id,
        facts.result,
        &evidence_ids,
        &execution_receipt,
    );
    messages.push(tool_message.clone());
    if !facts.control_error {
        continuation_messages.push(tool_message);
    }
    Ok(true)
}

/**
 * @GOV
 * codes:    BR-AG-STATE-001-EFFECT-AG-AG-TURN-074
 * type:     EFFECT
 * chain:    AG-TURN
 * rules:    BR-AG-STATE-001
 * boundary: in=ActiveAgentTurnCancellationSourceStateAndOptionalTaskRun | out=SettledCancelledTaskRunAndSingleCancelledTerminalEvent | effect=DB-settle_optional_task_run | consumer=BR-AG-STATE-001 | does_not=emitSecondTerminalOrMarkCompleted
 */
/// 结算取消终态：TaskRun 取消 + SM-AG-01 Cancelled 广播。
/// 规则由 BR-AG-STATE-001 / BR-SYS-STATE-007 的既有块承接；调用方负责先推进 SM-AG-02 取消转移。
fn finish_cancelled_turn<R: tauri::Runtime>(
    state: &AppState,
    app: &tauri::AppHandle<R>,
    emitter: &TurnEventEmitter<'_>,
    latch: &ag::turn::TerminalLatch,
    turn_id: &str,
    task_run_id: Option<&str>,
    turn_state: ag::turn::TurnState,
) {
    if let Ok(conn) = state.db.lock() {
        settle_optional_task_run(
            &conn,
            task_run_id,
            turn_id,
            ag::task::TurnOutcome::Cancelled,
        );
    }
    super::turn_events::emit_ag_turn_cancelled_from(emitter, latch, app, turn_id, turn_state);
}

/// 重建下一轮 PromptPackage 并记录预算证据；装配与预算规则由既有 Prompt 规则块承接。
/// 本函数不新增授权、终态或错误分类判断；`history_assembled` 的写事件失败仍按原路径向上传播。
struct RebuiltRoundPrompt {
    current_doc_prompt_coverage: Option<&'static str>,
    messages: Vec<serde_json::Value>,
    tools: serde_json::Value,
    capability_factors: ag::tool_registry::CapabilityFactorsV2,
    prompt_package: ag::context::PromptPackageV2,
}

enum PromptRebuildFailure {
    Propagate(IpcError),
    Settle(IpcError),
}

struct PromptRebuildFacts<'a> {
    state: &'a AppState,
    session_id: &'a str,
    turn_id: &'a str,
    runtime_profile: &'a str,
    task_run_id: Option<&'a str>,
    intent_result: &'a ag::context::IntentResultV2,
    agent_context_facts: &'a crate::types::AgentContextFacts,
    explicit_edit_target_set: &'a ag::target_set::ExplicitEditTargetSet,
    read_ctx: &'a ag::blocks::AgentReadContext,
    active_doc_path: Option<&'a str>,
    active_snapshot: Option<&'a ag::blocks::AgentEditableSnapshot>,
    history: &'a [ag::context::HistTurn],
    raw_prompt: &'a str,
    mention_block: Option<&'a str>,
    reference_envelopes: &'a [ag::context::PromptDataEnvelopeV1],
    proposed_diff: bool,
    continuation_messages: &'a [serde_json::Value],
    decision_receipt_ids: &'a [String],
    next_model_call: u32,
    remaining_tool_calls: u32,
    model_capacity: ag::prompt_runtime::ModelCapacity,
    response_rejected: bool,
}

/**
 * @GOV
 * codes:    BR-AG-DATA-001-EFFECT-AG-AG-TURN-078
 * type:     EFFECT
 * chain:    AG-TURN
 * rules:    BR-AG-DATA-001
 * boundary: in=RoundIntentContextFactsReadContextHistoryAndBudgetFacts | out=RebuiltRoundPromptOrSettlePropagateFailure | effect=DB-record_prompt_budget | consumer=BR-AG-DATA-001 | does_not=widenContextSourcesOrRaiseBudget
 */
fn rebuild_round_prompt(
    emitter: &TurnEventEmitter<'_>,
    facts: PromptRebuildFacts<'_>,
) -> Result<RebuiltRoundPrompt, PromptRebuildFailure> {
    let PreparedTurnPrompt {
        current_doc_requires_tool_read: _,
        current_doc_prompt_coverage,
        messages,
        tools,
        capability_grant: _rebuilt_grant,
        capability_factors,
        prompt_package,
        budget_measurement,
        budget_sources,
        history_selection,
    } = prepare_turn_prompt(
        facts.state,
        facts.runtime_profile,
        facts.turn_id,
        facts.session_id,
        facts.task_run_id,
        facts.intent_result,
        facts.agent_context_facts,
        &facts.explicit_edit_target_set.paths(),
        facts.read_ctx,
        facts.active_doc_path,
        facts.active_snapshot,
        facts.history,
        facts.raw_prompt,
        facts.mention_block,
        facts.reference_envelopes,
        facts.proposed_diff,
        facts.continuation_messages,
        facts.decision_receipt_ids,
        facts.next_model_call,
        facts.remaining_tool_calls,
        facts.model_capacity,
    )
    .map_err(PromptRebuildFailure::Settle)?;
    emitter
        .record_internal("history_assembled", history_selection)
        .map_err(PromptRebuildFailure::Propagate)?;
    let stage = if facts.response_rejected {
        "protocol_correction"
    } else {
        "tool_result"
    };
    emitter
        .record_prompt_budget(
            stage,
            &budget_measurement,
            budget_sources,
            facts.model_capacity,
        )
        .map_err(PromptRebuildFailure::Settle)?;
    if budget_measurement.estimated_tokens > facts.model_capacity.input_tokens() {
        return Err(PromptRebuildFailure::Settle(IpcError::new(
            "AG_PROMPT_BUDGET_EXCEEDED",
            &super::prompt_runtime::budget_failure_message(
                &budget_measurement,
                stage,
                facts.model_capacity,
            ),
            true,
            false,
        )));
    }
    Ok(RebuiltRoundPrompt {
        current_doc_prompt_coverage,
        messages,
        tools,
        capability_factors,
        prompt_package,
    })
}

/// 逐 ToolCall 执行分派：edit_blocked / 任务级确认 / propose_edit / 只读工具。
/// 不推进 SM-AG-02，也不重新计算授权；授权与状态转移仍由调用方完成。
struct ToolExecutionFacts<'a> {
    state: &'a AppState,
    cancelled: &'a Arc<AtomicBool>,
    call: &'a crate::ent::gateway::GwToolCall,
    params: &'a serde_json::Value,
    arg_parse_error: Option<&'a String>,
    execution_guard_error: Option<&'a String>,
    tool_decision: Option<&'a ag::tool::ToolDecision>,
    execution_failure: Option<IpcError>,
    read_execution: Option<Result<String, IpcError>>,
    ws_root: Option<&'a str>,
    read_ctx: &'a ag::blocks::AgentReadContext,
    active_doc_path: Option<&'a str>,
    explicit_edit_target_set: &'a ag::target_set::ExplicitEditTargetSet,
    capability_identity: &'a crate::types::ToolCallReceiptIdentityV1,
    task_run_id: Option<&'a str>,
    scope_candidates: &'a Result<Option<Vec<String>>, IpcError>,
    current_doc_requires_tool_read: bool,
    current_doc_read_confirmed: bool,
    workspace_id: &'a str,
    session_id: &'a str,
    turn_id: &'a str,
    proposed_diff: bool,
    edit_blocked_reported: bool,
}

struct ToolExecutionOutcome {
    result: String,
    success: bool,
    diff_events: Vec<(String, u32, u32, String, String)>,
    decision: Option<crate::types::DecisionRequestV1>,
    end_after_diff: bool,
    proposed_diff: bool,
    edit_blocked_reported: bool,
    execution_failure: Option<IpcError>,
}

/**
 * @GOV
 * codes:    BR-AG-STATE-002-RB-AG-AG-TOOL-068
 * type:     RB
 * chain:    AG-TOOL
 * rules:    BR-AG-STATE-002
 * boundary: in=ValidatedToolDecisionGrantParamsReadContextAndTaskRun | out=ToolExecutionOutcomePendingDiffDecisionRequestPreviewCheckpointOrFailure | delegate=exec_edit_blocked,execute_proposal,exec_read_only,exec_structure_tool,create_preview_checkpoint | consumer=BR-AG-STATE-002 | does_not=computeGrantAdvanceToolLoopStateOrSettleAgentTurn
 */
async fn execute_tool_call<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    facts: ToolExecutionFacts<'_>,
) -> ToolExecutionOutcome {
    let mut diff_events = Vec::new();
    let mut decision = None;
    let mut preview_checkpoint = false;
    let mut end_after_diff = false;
    let mut proposed_diff = facts.proposed_diff;
    let mut edit_blocked_reported = facts.edit_blocked_reported;
    let mut execution_failure = facts.execution_failure;
    let (result, success) = if let Some(error_msg) = facts.arg_parse_error {
        (error_msg.to_string(), false)
    } else if let Some(error_msg) = facts.execution_guard_error {
        (error_msg.to_string(), false)
    } else {
        let tool_decision: ag::tool::ToolDecision = facts
            .tool_decision
            .expect("parsed tool arguments must have a validation decision")
            .clone();
        match tool_decision {
            ag::tool::ToolDecision::Reject(reason) => (reason, false),
            ag::tool::ToolDecision::InvalidArguments {
                field,
                constraint,
                message,
            } => (
                serde_json::json!({"code":"AG_TOOL_ARG","fieldPath":field,"constraint":constraint,
                    "message":message,"notExecuted":true,"requiresNewToolCallId":true})
                .to_string(),
                false,
            ),
            ag::tool::ToolDecision::Allow if facts.call.name == "edit_blocked" => {
                match ag::tool_exec::exec_edit_blocked(facts.params) {
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
            // 停在 AwaitingDecision；由调用方事件分派统一发送确认卡，但不得发送 Done。
            ag::tool::ToolDecision::Allow
                if matches!(
                    facts.call.name.as_str(),
                    "propose_decision" | "plan_workspace_operation"
                ) =>
            {
                let task = facts
                    .task_run_id
                    .expect("control capability binds TaskRun")
                    .to_string();
                let name = facts.call.name.clone();
                let parameters = (*facts.params).clone();
                let target_paths = facts
                    .scope_candidates
                    .as_ref()
                    .ok()
                    .and_then(|paths| paths.clone());
                let source_turn = facts.turn_id.to_string();
                let executed = ag::capability_runtime::execute(
                    (*facts.state).clone(),
                    (*facts.capability_identity).clone(),
                    Arc::clone(facts.cancelled),
                    std::time::Duration::from_secs(30),
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
                    Ok((decision_request, result)) => {
                        decision = Some(decision_request);
                        (result.to_string(), true)
                    }
                    Err(error) => {
                        execution_failure = Some(error.clone());
                        (
                            format!("能力执行失败：[{}] {}", error.code, error.message),
                            false,
                        )
                    }
                }
            }
            // 写工具 propose_edit：经 DE 生成 pending diff（X-FORBID-006，不直写）→ emit DiffProposed。
            ag::tool::ToolDecision::Allow if facts.call.name == "propose_edit" => {
                if current_doc_edit_requires_prior_read(
                    facts.params,
                    facts.active_doc_path,
                    facts.current_doc_requires_tool_read,
                    facts.current_doc_read_confirmed,
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
                        (*facts.state).clone(),
                        (*app).clone(),
                        (*facts.params).clone(),
                        (*facts.read_ctx).clone(),
                        facts.active_doc_path.map(str::to_string),
                        (*facts.explicit_edit_target_set).clone(),
                        (*facts.capability_identity).clone(),
                        Arc::clone(facts.cancelled),
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
                            let needs_preview_checkpoint = checkpoint_diff_id.is_some();
                            if needs_preview_checkpoint {
                                if let Some(current_task_run_id) = facts.task_run_id {
                                    let checkpoint_result = match facts.state.db.lock() {
                                        Ok(conn) => ag::task::create_preview_checkpoint(
                                            &conn,
                                            current_task_run_id,
                                            facts.workspace_id,
                                            facts.session_id,
                                            facts.turn_id,
                                            checkpoint_diff_id.as_deref().unwrap_or(""),
                                            &edit_result.rel_path,
                                        ),
                                        Err(_) => Err(rusqlite::Error::InvalidQuery),
                                    };
                                    match checkpoint_result {
                                        Ok(()) => {
                                            preview_checkpoint = true;
                                        }
                                        Err(error) => {
                                            for projection in &mut edit_result.projections {
                                                projection.preview_checkpoint_required = false;
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
                                        projection.preview_checkpoint_required = false;
                                    }
                                }
                            }
                            if preview_checkpoint {
                                for projection in edit_result.projections.iter_mut().skip(1) {
                                    projection.preview_checkpoint_required = false;
                                }
                            }
                            proposed_diff = true;
                            // 无 LogicalState 的 background Diff 同样不得让模型在同一份旧 read_ctx
                            // 上继续续编；它没有 PreviewAck checkpoint，需经既有迁移或裁决后由
                            // 人类下一次显式操作开启新片段。
                            end_after_diff = true;
                            let group_count =
                                u32::try_from(edit_result.projections.len()).unwrap_or(u32::MAX);
                            diff_events = edit_result
                                .projections
                                .iter()
                                .enumerate()
                                .map(|(offset, projection)| {
                                    (
                                        edit_result.proposal_batch_id.clone(),
                                        u32::try_from(offset + 1).unwrap_or(u32::MAX),
                                        group_count,
                                        projection.diff_id.clone(),
                                        edit_result.rel_path.clone(),
                                    )
                                })
                                .collect();
                            let result = proposal_result_projection(
                                &edit_result.proposal_batch_id,
                                &edit_result.rel_path,
                                &edit_result.projections,
                                preview_checkpoint,
                            );
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
            ag::tool::ToolDecision::Allow => match facts.ws_root {
                None => ("无活动 workspace，无法执行工具".to_string(), false),
                Some(_) => {
                    let executed = if let Some(result) = facts.read_execution {
                        result
                    } else if facts.call.name == "import_image_asset" {
                        let root =
                            std::path::PathBuf::from(facts.ws_root.expect("workspace exists"));
                        let name = facts.call.name.clone();
                        let parameters = (*facts.params).clone();
                        ag::capability_runtime::execute(
                            (*facts.state).clone(),
                            (*facts.capability_identity).clone(),
                            Arc::clone(facts.cancelled),
                            std::time::Duration::from_secs(30),
                            move |_| ag::tool_exec::exec_structure_tool(&root, &name, &parameters),
                            |_| vec![],
                        )
                        .await
                    } else {
                        ag::tool_exec::exec_read_only(
                            facts.read_ctx,
                            &facts.call.name,
                            facts.params,
                        )
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
    ToolExecutionOutcome {
        result,
        success,
        diff_events,
        decision,
        end_after_diff,
        proposed_diff,
        edit_blocked_reported,
        execution_failure,
    }
}

/**
 * @GOV
 * codes:    BR-AG-DATA-008-EFFECT-AG-AG-TURN-073
 * type:     EFFECT
 * chain:    AG-TURN
 * rules:    BR-AG-DATA-008
 * boundary: in=FreshHostContextRequestPreviousContextFactsAndAgentReadContext | out=RefreshedContextFactsReadContextActiveSnapshotOrToolContextFailure | consumer=BR-AG-DATA-008 | does_not=executeToolOrTreatContextRefreshAsAuthorization
 */
/// 为需要新鲜宿主上下文的工具刷新 AgentContextFacts / AgentReadContext。
/// Ok(None)=无需刷新或刷新成功；Ok(Some(error))=工具级上下文失败（非终态）；
/// Err(error)=context_frozen 写事件失败，按原路径向上传播，不在这里结算终态。
#[allow(clippy::too_many_arguments)]
async fn refresh_tool_context(
    emitter: &TurnEventEmitter<'_>,
    on_context_request: Option<&tauri::ipc::Channel<super::turn_runtime::AgentContextRequest>>,
    agent_context_facts: &mut crate::types::AgentContextFacts,
    read_ctx: &mut ag::blocks::AgentReadContext,
    active_snapshot: &mut Option<ag::blocks::AgentEditableSnapshot>,
    active_doc_path: Option<&str>,
    tool_call_id: &str,
    tool_name: &str,
    params: &serde_json::Value,
    cancelled: Arc<AtomicBool>,
) -> Result<Option<IpcError>, IpcError> {
    let crate::types::AgentContextFacts::V2(previous) = &*agent_context_facts else {
        return Ok(None);
    };
    match super::turn_runtime::refresh_agent_context(
        on_context_request,
        previous,
        read_ctx,
        tool_name,
        params,
        cancelled,
    )
    .await
    {
        Ok((facts, context)) => {
            emitter.record_internal(
                "context_frozen",
                serde_json::json!({
                    "contextVersion": facts.version(),
                    "contextRevision": facts.revision(),
                    "activeTarget": previous.active_target,
                    "toolCallId": tool_call_id,
                    "readyDocuments": facts.ready_documents(),
                }),
            )?;
            *active_snapshot = active_doc_path.and_then(|path| {
                context
                    .open_snapshot(path)
                    .and_then(|_| context.snapshot(path).ok())
            });
            *agent_context_facts = facts;
            *read_ctx = context;
            Ok(None)
        }
        Err(error) => Ok(Some(error)),
    }
}

/// 计算 propose_decision(kind=scope) 可追加的显式目标候选；非 scope 调用返回 Ok(None)。
/// 不在本函数内追加目标，也不把读取/历史发现升级为目标。
fn compute_scope_candidates(
    tool_decision: Option<&ag::tool::ToolDecision>,
    tool_name: &str,
    params: &serde_json::Value,
    ws_root: Option<&str>,
    explicit_edit_target_set: &ag::target_set::ExplicitEditTargetSet,
) -> Result<Option<Vec<String>>, IpcError> {
    if !matches!(tool_decision, Some(ag::tool::ToolDecision::Allow))
        || tool_name != "propose_decision"
        || params.get("kind").and_then(|value| value.as_str()) != Some("scope")
    {
        return Ok(None);
    }
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
    match ws_root {
        Some(root) => ag::target_set::validated_scope_candidates(
            std::path::Path::new(root),
            &raw_paths,
            explicit_edit_target_set,
        )
        .map(Some),
        None => Err(IpcError::new(
            "WS_NO_ACTIVE",
            "缺少 active workspace，不能冻结范围追加目标",
            false,
            false,
        )),
    }
}

/**
 * @GOV
 * codes:    BR-SYS-STATE-008-EFFECT-AG-AG-TOOL-071
 * type:     EFFECT
 * chain:    AG-TOOL
 * rules:    BR-SYS-STATE-008
 * boundary: in=AllowedToolProductExplicitEditTargetSetAndGoal | out=PersistedAndBoundTaskRunOrPreparationFailure | effect=DB-begin_for_tool_product | consumer=BR-SYS-STATE-008 | does_not=executeToolOrCreateTaskRunForSimpleTurn
 */
/// 仅当已获准的任务级工具真的到达执行边界且尚无 TaskRun 时，创建/绑定/持久化 TaskRun 投影。
/// 返回 Some(error) 表示本调用的 TaskRun 投影失败；不在这里执行工具或推进 SM-AG-02。
#[allow(clippy::too_many_arguments)]
fn prepare_task_run_for_tool(
    state: &AppState,
    emitter: &TurnEventEmitter<'_>,
    session_id: &str,
    workspace_id: &str,
    turn_id: &str,
    tool_name: &str,
    goal: &str,
    explicit_edit_target_set: &ag::target_set::ExplicitEditTargetSet,
    scope_candidates_ok: bool,
    tool_decision_allows: bool,
    task_run_id: &mut Option<String>,
) -> Option<IpcError> {
    if !tool_decision_allows
        || !scope_candidates_ok
        || !ag::task::tool_requires_task_run(tool_name)
        || task_run_id.is_some()
    {
        return None;
    }
    let created = match state.db.lock() {
        Ok(conn) => ag::task::begin_for_tool_product(&conn, workspace_id, session_id, tool_name)
            .map_err(|error| IpcError::new("AG_TASK_DB_ERROR", &error.to_string(), true, false))
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
    let created_task_run_id = match created {
        Ok(created_task_run_id) => created_task_run_id,
        Err(error) => return Some(error),
    };
    *task_run_id = Some(created_task_run_id.clone());
    match bind_agent_turn_task_run(state, session_id, turn_id, &created_task_run_id) {
        Ok(()) => match state.db.lock() {
            Ok(conn) => {
                if let Err(error) = ag::task::link_turn(&conn, turn_id, &created_task_run_id)
                    .and_then(|_| {
                        ag::target_set::persist_for_task_run(
                            &conn,
                            &created_task_run_id,
                            explicit_edit_target_set,
                        )
                    })
                {
                    return Some(IpcError::new(
                        "AG_TASK_DB_ERROR",
                        &format!("TaskRun 无法固化当前回合范围：{error}"),
                        true,
                        false,
                    ));
                }
            }
            Err(_) => {
                return Some(IpcError::new(
                    "WS_DB_BUSY",
                    "数据库连接繁忙，无法固化 TaskRun 回合范围",
                    true,
                    false,
                ))
            }
        },
        Err(error) => return Some(error),
    }
    if let Err(error) = emitter.record_internal(
        "task_projected",
        serde_json::json!({
            "taskRunId": created_task_run_id,
            "goal": goal,
            "reason": "task_level_waiting_point",
        }),
    ) {
        return Some(error);
    }
    None
}

/// 计算本次 ToolCall 的一次性 CapabilityGrantV2，并完成注册/权限/参数校验与参数纠错计数。
/// Propagate = 事件写失败或授权边界发现回合已终结；Settle = 参数纠错达到上限，由调用方结算终态。
struct ToolGrantFacts<'a> {
    latch: &'a ag::turn::TerminalLatch,
    cancelled: &'a Arc<AtomicBool>,
    runtime_profile: &'a str,
    turn_id: &'a str,
    has_task_run: bool,
    capability_factors: &'a ag::tool_registry::CapabilityFactorsV2,
    model_call_ordinal: usize,
    remaining_tool_calls: u32,
    explicit_edit_target_set: &'a ag::target_set::ExplicitEditTargetSet,
    tools: &'a serde_json::Value,
    ws_root: Option<&'a str>,
    read_failure_final_pending: bool,
    arg_parse_error_none: bool,
    final_only_after_rejection: bool,
    max_corrections: usize,
    call: &'a crate::ent::gateway::GwToolCall,
    params: &'a serde_json::Value,
    capability_use_tracker: &'a mut ag::tool_registry::CapabilityGrantUseTrackerV2,
    parameter_correction_rounds: &'a mut usize,
    correction_counted_this_round: &'a mut bool,
    parameter_correction_pending: &'a mut bool,
    final_only_correction_pending: &'a mut bool,
}

enum ToolGrantFailure {
    Propagate(IpcError),
    Settle(IpcError),
}

struct ToolGrantOutcome {
    execution_grant: ag::tool_registry::CapabilityGrantV2,
    tool_decision: Option<ag::tool::ToolDecision>,
    parameter_contract_rejection: bool,
}

/**
 * @GOV
 * codes:    BR-AG-STATE-002-RB-AG-AG-TOOL-067
 * type:     RB
 * chain:    AG-TOOL
 * rules:    BR-AG-STATE-002
 * boundary: in=ToolCallBoundGrantFactsAndCorrectionCounters | out=ToolGrantOutcomeOrToolGrantFailure | delegate=resolve_capability_grant_v2,validate_requested_tool_for_grant | consumer=BR-AG-STATE-002 | does_not=executeToolAdvanceToolLoopStateOrSettleAgentTurn
 */
fn resolve_execution_grant_and_decision(
    emitter: &TurnEventEmitter<'_>,
    facts: ToolGrantFacts<'_>,
) -> Result<ToolGrantOutcome, ToolGrantFailure> {
    let computed_at_seq = emitter
        .record_internal(
            "capability_grant_boundary",
            serde_json::json!({
                "toolCallId": &facts.call.id,
                "gateAttempt": facts.model_call_ordinal,
                "remainingToolCalls": facts.remaining_tool_calls,
            }),
        )
        .map_err(ToolGrantFailure::Propagate)?
        .ok_or_else(|| {
            ToolGrantFailure::Propagate(IpcError::new(
                "AG_CAPABILITY_GRANT_TERMINAL",
                "AgentTurn 已终结，不能再计算工具执行授权",
                false,
                false,
            ))
        })?;
    let mut action_factors = facts.capability_factors.clone();
    action_factors.has_task_run = facts.has_task_run;
    action_factors.gate_attempt = facts.model_call_ordinal as u32;
    action_factors.gate_passed = true;
    action_factors.remaining_tool_calls = facts.remaining_tool_calls;
    action_factors.cancelled = facts.cancelled.load(Ordering::Acquire);
    action_factors.terminal = facts.latch.is_set();
    let execution_grant = ag::tool_registry::resolve_capability_grant_v2(
        facts.runtime_profile,
        facts.turn_id,
        &facts.explicit_edit_target_set.paths(),
        action_factors,
        ag::tool_registry::CapabilityGrantPurposeV2::ToolCall {
            tool_call_id: facts.call.id.clone(),
            computed_at_seq,
        },
    );
    emitter
        .record_internal(
            "capability_grant_computed",
            serde_json::json!({
                "grantId": &execution_grant.grant_id,
                "toolCallId": &facts.call.id,
                "computedAtSeq": computed_at_seq,
                "authorizationFactsHash": &execution_grant.authorization_facts_hash,
                "allowedToolCount": execution_grant.allowed_tools.len(),
            }),
        )
        .map_err(ToolGrantFailure::Propagate)?;
    let tool_was_visible = tool_is_visible_in_manifest(facts.tools, &facts.call.name);
    let tool_decision = facts.arg_parse_error_none.then(|| {
        let validated = ag::tool_registry::validate_requested_tool_for_grant(
            &facts.call.name,
            facts.params,
            &execution_grant,
            facts.turn_id,
            &facts.call.id,
            computed_at_seq,
            facts.ws_root.map(std::path::Path::new),
            facts.capability_use_tracker,
        );
        if facts.read_failure_final_pending {
            ag::tool::ToolDecision::Reject(
                "重复只读失败后禁止继续工具调用；此前证据保留，请说明未完成范围。".into(),
            )
        } else if matches!(validated, ag::tool::ToolDecision::Allow) && !tool_was_visible {
            ag::tool::ToolDecision::Reject(format!(
                "工具 {} 不在当前 Prompt Tool Manifest 中",
                facts.call.name
            ))
        } else {
            validated
        }
    });
    let parameter_contract_rejection = tool_decision
        .as_ref()
        .and_then(|decision| match decision {
            ag::tool::ToolDecision::Reject(reason) => Some(reason.as_str()),
            ag::tool::ToolDecision::Allow | ag::tool::ToolDecision::InvalidArguments { .. } => None,
        })
        .is_some_and(|reason| {
            reason.starts_with(&format!("工具 {} 参数", facts.call.name))
                || reason.starts_with("工具参数结构")
        })
        || matches!(
            tool_decision.as_ref(),
            Some(ag::tool::ToolDecision::InvalidArguments { .. })
        );
    let parameter_diagnostic = match tool_decision.as_ref() {
        Some(ag::tool::ToolDecision::InvalidArguments {
            field, constraint, ..
        }) => Some(serde_json::json!({"fieldPath":field,"constraint":constraint})),
        _ => None,
    };
    if parameter_contract_rejection && !*facts.correction_counted_this_round {
        *facts.parameter_correction_pending = true;
        if facts.final_only_after_rejection {
            // Append only after the assistant tool-call/result group is paired.
            *facts.final_only_correction_pending = true;
        }
        *facts.correction_counted_this_round = true;
        *facts.parameter_correction_rounds += 1;
        emitter
            .record_internal(
                "tool_parameter_contract_rejected",
                serde_json::json!({
                    "code":"AG_TOOL_ARG",
                    "rejectionStage":"tool_validation",
                    "toolName": &facts.call.name,
                    "parameterKeys": safe_tool_params(facts.params)["parameterKeys"],
                    "parameterDiagnostic": parameter_diagnostic,
                    "correctionAttempt": *facts.parameter_correction_rounds,
                }),
            )
            .map_err(ToolGrantFailure::Propagate)?;
        if *facts.parameter_correction_rounds > facts.max_corrections {
            return Err(ToolGrantFailure::Settle(IpcError::new(
                "AG_MODEL_CORRECTION_LIMIT",
                "本回合参数拒绝已超过独立纠错上限；本次未执行，此前结果保留。请检查参数或显式重试。",
                true,
                false,
            )));
        }
    }
    Ok(ToolGrantOutcome {
        execution_grant,
        tool_decision,
        parameter_contract_rejection,
    })
}

/**
 * @GOV
 * codes:    BR-AG-STATE-002-GUARD-AG-AG-TOOL-066
 * type:     GUARD
 * chain:    AG-TOOL
 * rules:    BR-AG-STATE-002
 * boundary: in=ToolCallCapabilityFactorsManifestAndPreflightGrant | out=boolean（true 当 预检校验允许进入上下文刷新） | consumer=BR-AG-STATE-002 | does_not=executeToolOrReplaceExecutionGrant
 */
/// 执行前的 acquisition 预检：同一硬边界下计算临时 grant 并校验，仅决定是否需要刷新上下文。
/// 该临时 grant 不执行工具，也不替代后续真实的一次性执行授权。
#[allow(clippy::too_many_arguments)]
fn preflight_tool_acquisition_allowed(
    capability_factors: &ag::tool_registry::CapabilityFactorsV2,
    cancelled: &Arc<AtomicBool>,
    latch: &ag::turn::TerminalLatch,
    runtime_profile: &str,
    turn_id: &str,
    explicit_edit_target_set: &ag::target_set::ExplicitEditTargetSet,
    model_call_ordinal: usize,
    remaining_tool_calls: u32,
    tools: &serde_json::Value,
    ws_root: Option<&str>,
    read_failure_final_pending: bool,
    arg_parse_error_none: bool,
    call: &crate::ent::gateway::GwToolCall,
    params: &serde_json::Value,
) -> bool {
    let mut acquisition_factors = capability_factors.clone();
    acquisition_factors.gate_passed = true;
    acquisition_factors.gate_attempt = model_call_ordinal as u32;
    acquisition_factors.remaining_tool_calls = remaining_tool_calls;
    acquisition_factors.cancelled = cancelled.load(Ordering::Acquire);
    acquisition_factors.terminal = latch.is_set();
    let acquisition_grant = ag::tool_registry::resolve_capability_grant_v2(
        runtime_profile,
        turn_id,
        &explicit_edit_target_set.paths(),
        acquisition_factors,
        ag::tool_registry::CapabilityGrantPurposeV2::ToolCall {
            tool_call_id: call.id.clone(),
            computed_at_seq: 1,
        },
    );
    !read_failure_final_pending
        && arg_parse_error_none
        && tool_is_visible_in_manifest(tools, &call.name)
        && matches!(
            ag::tool_registry::validate_requested_tool_for_grant(
                &call.name,
                params,
                &acquisition_grant,
                turn_id,
                &call.id,
                1,
                ws_root.map(std::path::Path::new),
                &mut Default::default()
            ),
            ag::tool::ToolDecision::Allow
        )
}

/// 组合本次 ToolCall 的初始执行失败与 guard 文案；只做错误优先级归并，不执行工具或推进状态。
fn compute_execution_failure(
    control_error: Option<&IpcError>,
    parameter_contract_rejection: bool,
    tool_decision: Option<&ag::tool::ToolDecision>,
    tool_name: &str,
    params: &serde_json::Value,
    read_failure_final_pending: bool,
    context_refresh_error: Option<&IpcError>,
    scope_candidates: &Result<Option<Vec<String>>, IpcError>,
    task_run_prepare_error: Option<&IpcError>,
) -> (Option<IpcError>, Option<String>) {
    let execution_failure = control_error
        .cloned()
        .or_else(|| {
            parameter_contract_rejection.then(|| {
                ag::tool_exec::validate_read_arguments(tool_name, params)
                    .err()
                    .unwrap_or_else(|| {
                        let message = match tool_decision {
                            Some(ag::tool::ToolDecision::InvalidArguments { message, .. }) => {
                                message.as_str()
                            }
                            _ => "工具参数不符合声明；本次未执行，请依据工具契约修正。",
                        };
                        IpcError::new("AG_TOOL_ARG", message, true, false)
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
        .or_else(|| context_refresh_error.cloned())
        .or_else(|| scope_candidates.as_ref().err().cloned())
        .or_else(|| task_run_prepare_error.cloned());
    let execution_guard_error = if context_refresh_error.is_some() {
        context_refresh_error.map(|error| format!("[{}] {}", error.code, error.message))
    } else if let Err(error) = scope_candidates {
        Some(error.message.clone())
    } else if task_run_prepare_error.is_some() {
        task_run_prepare_error.map(|error| error.message.clone())
    } else {
        None
    };
    (execution_failure, execution_guard_error)
}

/// 归一化并校验一个模型工具批次：transport group/expand、空批、重复 ID、batch parse、控制应用。
/// Propagate = 事件写失败；Settle = 空批/重复 ID/纠错达上限/控制应用失败，由调用方 LoopError + 结算。
struct ToolBatchFacts<'a> {
    state: &'a AppState,
    latch: &'a ag::turn::TerminalLatch,
    session_id: &'a str,
    workspace_id: &'a str,
    turn_id: &'a str,
    explicit_edit_target_set: &'a ag::target_set::ExplicitEditTargetSet,
    calls: Vec<crate::ent::gateway::GwToolCall>,
    transport_protocol: &'a str,
    output_control: Option<ag::model_output::ModelTurnControl>,
    seen_tool_call_ids: &'a mut BTreeSet<String>,
    protocol_correction_rounds: &'a mut usize,
    protocol_correction_diagnostic: &'a mut Option<serde_json::Value>,
    max_corrections: usize,
    task_run_id: &'a mut Option<String>,
    intent_result: &'a mut ag::context::IntentResultV2,
    raw_prompt: &'a str,
    history: &'a [ag::context::HistTurn],
    messages: &'a [serde_json::Value],
    active_doc_path: Option<&'a str>,
    reference_envelopes: &'a [ag::context::PromptDataEnvelopeV1],
    continuation_messages: &'a [serde_json::Value],
}

struct PreparedToolBatch {
    calls: Vec<crate::ent::gateway::GwToolCall>,
    parsed_params: Vec<serde_json::Value>,
    control_error: Option<IpcError>,
    native_transport: Option<crate::ent::gateway::GwToolCall>,
}

enum ToolBatchFailure {
    Propagate(IpcError),
    Settle(IpcError),
}

/**
 * @GOV
 * codes:    BR-AG-STATE-002-RB-AG-AG-TOOL-069
 * type:     RB
 * chain:    AG-TOOL
 * rules:    BR-AG-STATE-002
 * boundary: in=RawToolCallBatchTransportProtocolContinuationAndSeenIds | out=PreparedToolBatchOrToolBatchFailure | delegate=validate_transport_call_group,expand_native_batch,parse_tool_batch_detailed_with_policy,apply_control_and_observe | consumer=BR-AG-STATE-002 | does_not=executeToolOrSettleAgentTurn
 */
fn prepare_tool_batch(
    emitter: &TurnEventEmitter<'_>,
    facts: ToolBatchFacts<'_>,
) -> Result<PreparedToolBatch, ToolBatchFailure> {
    let group_rejection =
        ag::model_output::validate_transport_call_group(facts.transport_protocol, &facts.calls)
            .err();
    let native_transport = (facts.calls.len() == 1
        && facts.calls[0].name == ag::model_output::TOOL_BATCH_NAME)
        .then(|| facts.calls[0].clone());
    let (calls, transport_rejection) = match group_rejection
        .map_or_else(|| ag::model_output::expand_native_batch(&facts.calls), Err)
    {
        Ok(Some(expanded)) => (expanded, None),
        Ok(None) => (facts.calls, None),
        Err(error) => (facts.calls, Some(error)),
    };
    if calls.is_empty() {
        return Err(ToolBatchFailure::Settle(IpcError::new(
            "AG_TOOL_CALLS_EMPTY",
            "模型返回了空 ToolCall 集合，当前工具循环已中止。",
            true,
            false,
        )));
    }
    // Validate the whole batch before creating any task/tool product. Continuation
    // messages belong to this AgentTurn and survive decision parking/resumption.
    if calls
        .iter()
        .any(|call| call.id.trim().is_empty() || !facts.seen_tool_call_ids.insert(call.id.clone()))
    {
        return Err(ToolBatchFailure::Settle(IpcError::new(
            "AG_MODEL_TOOL_ID_INVALID",
            "模型工具调用标识为空或重复；本批未执行。此前执行证据保留，请核对结果后显式重试。",
            true,
            false,
        )));
    }
    let arguments = calls
        .iter()
        .map(|call| call.arguments.as_str())
        .collect::<Vec<_>>();
    let independent_read_only_direct_group = facts.transport_protocol
        == ag::model_output::DIRECT_TOOL_PROTOCOL
        && calls.iter().all(|call| {
            ag::tool_registry::lookup(&call.name)
                .is_some_and(|tool| tool.permission == ag::tool_registry::ToolPermission::ReadOnly)
        });
    let (control, parsed_params, control_error) = match transport_rejection.map_or_else(
        || {
            ag::model_output::parse_tool_batch_detailed_with_policy(
                &arguments,
                facts.output_control,
                independent_read_only_direct_group,
            )
        },
        Err,
    ) {
        Ok(batch) => {
            if !batch.observations.is_empty() {
                emitter
                        .record_internal(
                            "model_control_description_reconciled",
                            serde_json::json!({"toolCallCount":calls.len(),"observations":batch.observations}),
                        )
                        .map_err(ToolBatchFailure::Propagate)?;
            }
            (Some(batch.control), batch.params, None)
        }
        Err(rejection) => {
            let protocol_diagnostic = rejection.diagnostic();
            *facts.protocol_correction_diagnostic = Some(protocol_diagnostic.clone());
            let error = rejection.error;
            *facts.protocol_correction_rounds += 1;
            emitter
                .record_internal(
                    "model_control_rejected",
                    serde_json::json!({
                        "code": &error.code,
                        "toolCallCount": calls.len(),
                        "correctionAttempt": *facts.protocol_correction_rounds,
                        "rejectionStage":"tool_batch",
                        "protocolDiagnostic": protocol_diagnostic,
                        "conflictFieldsOnly": (error.code == "AG_MODEL_CONTROL_CONFLICT")
                            .then_some(error.message.as_str()),
                    }),
                )
                .map_err(ToolBatchFailure::Propagate)?;
            if *facts.protocol_correction_rounds > facts.max_corrections {
                return Err(ToolBatchFailure::Settle(IpcError::new(
                        "AG_MODEL_CORRECTION_LIMIT",
                        "本回合控制封套拒绝已超过独立纠错上限；本批未执行，此前结果保留。请检查协议或显式重试。",
                        true,
                        false,
                    )));
            }
            // 一整批均未执行；逐个回注失败，复用 ValidateFail→Recorded→Assembling→Gate。
            // 不从工具名猜测意图，不复用旧语义替代本次模型结论。
            (None, vec![serde_json::json!({}); calls.len()], Some(error))
        }
    };
    if let Some(control) = control {
        if let Err(error) = apply_control_and_observe(
            facts.state,
            emitter,
            facts.latch,
            facts.session_id,
            facts.workspace_id,
            facts.turn_id,
            facts.explicit_edit_target_set,
            facts.task_run_id,
            facts.intent_result,
            &control,
            facts.raw_prompt,
            facts.history,
            facts.messages,
            facts.active_doc_path,
            facts.reference_envelopes,
            facts.continuation_messages,
        ) {
            return Err(ToolBatchFailure::Settle(error));
        }
    }
    Ok(PreparedToolBatch {
        calls,
        parsed_params,
        control_error,
        native_transport,
    })
}

/// 续调轮次的门禁等待与结果归类；只记录 gate 事件并返回结果，状态转移/终态结算仍由调用方完成。
/// Err = gate 事件写失败，按原路径向上传播。
enum RoundGate {
    Proceed,
    Cancelled,
    Denied(IpcError),
    Failed(IpcError),
}

/**
 * @GOV
 * codes:    BR-AG-VERIFY-001-RB-AG-AG-TURN-076
 * type:     RB
 * chain:    AG-TURN
 * rules:    BR-AG-VERIFY-001
 * boundary: in=ContinuationRoundSessionTurnAndGateProbe | out=RoundGateProceedCancelledDeniedOrFailed | delegate=await_turn_operation,check_gate | consumer=BR-AG-VERIFY-001 | does_not=transitionTurnStateOrSettleTerminal
 */
async fn await_round_gate(
    state: &AppState,
    emitter: &TurnEventEmitter<'_>,
    session_id: &str,
    turn_id: &str,
    model_call_ordinal: usize,
) -> Result<RoundGate, IpcError> {
    emitter.record_internal(
        "gate_requested",
        serde_json::json!({"attempt": model_call_ordinal}),
    )?;
    match super::turn_control::await_turn_operation(
        state,
        session_id,
        turn_id,
        ent::gateway::check_gate(),
    )
    .await
    {
        Ok(ent::gateway::GateOutcome::Pass) => {
            emitter.record_internal(
                "gate_evaluated",
                serde_json::json!({"attempt": model_call_ordinal, "outcome": "passed"}),
            )?;
            Ok(RoundGate::Proceed)
        }
        Ok(ent::gateway::GateOutcome::Deny) => {
            emitter.record_internal(
                "gate_evaluated",
                serde_json::json!({"attempt": model_call_ordinal, "outcome": "denied"}),
            )?;
            Ok(RoundGate::Denied(IpcError::new(
                "AG_GATE_DENIED",
                "续调门禁被服务端拒绝，本轮已停止。",
                false,
                true,
            )))
        }
        Err(error) if error.code == "AG_TURN_CANCELLED" => {
            emitter.record_internal(
                "gate_evaluated",
                serde_json::json!({"attempt": model_call_ordinal, "outcome": "cancelled"}),
            )?;
            Ok(RoundGate::Cancelled)
        }
        Err(error) => {
            emitter.record_internal(
                "gate_evaluated",
                serde_json::json!({
                    "attempt": model_call_ordinal,
                    "outcome": "error",
                    "code": &error.code,
                }),
            )?;
            Ok(RoundGate::Failed(error))
        }
    }
}

/// 执行一次 provider 流式调用并返回结果/取消/失败归类。
/// 只记录 provider_started/provider_finished；abort 安装失败、传输错误的终态转移与结算仍由调用方完成。
struct ProviderRoundReady {
    round: ent::gateway::RoundOutcome,
    decoder: ag::model_output::ModelOutputDecoder,
    visible_output: String,
    visible_output_chars: usize,
}

enum ProviderRoundResult {
    Ready(ProviderRoundReady),
    Cancelled,
    Failed(IpcError),
}

#[allow(clippy::too_many_arguments)]
/**
 * @GOV
 * codes:    BR-AG-VERIFY-001-EFFECT-AG-AG-TURN-077
 * type:     EFFECT
 * chain:    AG-TURN
 * rules:    BR-AG-VERIFY-001
 * boundary: in=PreparedProviderMessagesToolsAuditContextAndAbortState | out=ProviderRoundReadyCancelledOrFailedWithVisibleOutput | delegate=stream_round,install_agent_turn_abort_handle | effect=IPC-stream_round | consumer=BR-AG-VERIFY-001 | does_not=transitionTurnStateOrSettleTerminal
 */
async fn run_provider_round(
    state: &AppState,
    emitter: &TurnEventEmitter<'_>,
    latch: &ag::turn::TerminalLatch,
    session_id: &str,
    turn_id: &str,
    provider: &str,
    model: &str,
    runtime_profile: &str,
    messages: &[serde_json::Value],
    tools: &serde_json::Value,
    prompt_package: &ag::context::PromptPackageV2,
    final_prompt_audit: &mut Option<FinalPromptAuditLog>,
    model_call_ordinal: usize,
    budget_closing: bool,
    workspace_plan_completed: bool,
) -> Result<ProviderRoundResult, IpcError> {
    let provider_messages = provider_messages_for_model(messages);
    if let Some(audit_log) = final_prompt_audit.as_mut() {
        let audit = build_prompt_audit_record(
            turn_id,
            session_id,
            model_call_ordinal,
            runtime_profile,
            provider,
            model,
            prompt_package,
            tools,
            &provider_messages,
        );
        audit_log.capture(audit);
    }
    let msgs = serde_json::Value::Array(provider_messages);
    emitter.record_internal(
        "provider_started",
        serde_json::json!({
            "roundIndex": model_call_ordinal,
            "provider": provider,
            "model": model,
            "protocolVersion": "binder-response-v2",
        }),
    )?;
    let (abort_handle, abort_registration) = futures_util::future::AbortHandle::new_pair();
    if let Err(error) = install_agent_turn_abort_handle(state, session_id, turn_id, abort_handle) {
        return Ok(ProviderRoundResult::Failed(error));
    }
    let mut decoder = ag::model_output::ModelOutputDecoder::default();
    let mut visible_output_chars = 0usize;
    let mut visible_output = String::new();
    let round = match futures_util::future::Abortable::new(
        ent::gateway::stream_round(
            turn_id,
            model_call_ordinal,
            provider,
            model,
            &msgs,
            tools,
            |text| {
                for visible in decoder.push(&text)? {
                    // Reject a resource-final handoff before exposing its finished-looking
                    // prose. The response can then be corrected without a duplicate answer.
                    if decoder.parsed_control().is_some_and(|control| {
                        control.next_action == ag::model_output::ModelNextAction::Final
                            && control.intent.side_effect_intent
                                == ag::context::SideEffectIntentV2::StructurePlan
                            && control.completion.is_none()
                            && !budget_closing
                            && !workspace_plan_completed
                            || budget_closing
                                && control.next_action != ag::model_output::ModelNextAction::Final
                    }) {
                        continue;
                    }
                    visible_output_chars += visible.chars().count();
                    visible_output.push_str(&visible);
                    emitter.emit(latch, AgTurnEventPayload::Chunk { text: visible })?;
                }
                Ok(())
            },
            |kind, details| emitter.record_internal(kind, details).map(|_| ()),
        ),
        abort_registration,
    )
    .await
    {
        Err(_) => {
            emitter.record_internal(
                "provider_finished",
                serde_json::json!({"roundIndex": model_call_ordinal, "outcome": "cancelled"}),
            )?;
            return Ok(ProviderRoundResult::Cancelled);
        }
        Ok(Err(error)) => {
            emitter.record_internal(
                "provider_finished",
                serde_json::json!({
                    "roundIndex": model_call_ordinal,
                    "outcome": if error.code.starts_with("AG_MODEL_CONTROL_") {
                        "model_protocol_rejected"
                    } else { "transport_error" },
                    "code": &error.code,
                    "prefixKind": decoder.prefix_kind(),
                }),
            )?;
            return Ok(ProviderRoundResult::Failed(error));
        }
        Ok(Ok(outcome)) => {
            emitter.record_internal(
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
    Ok(ProviderRoundResult::Ready(ProviderRoundReady {
        round,
        decoder,
        visible_output,
        visible_output_chars,
    }))
}

/// 判定 final 响应是否缺少本轮必需的终态工具产物；返回 Some(error) 表示应终止本轮。
/// 预算收尾或已提供 completion 时明确不要求终态工具。
fn terminal_tool_requirement_error(
    budget_closing: bool,
    has_completion: bool,
    workspace_plan_completed: bool,
    intent: &ag::context::IntentResultV2,
    proposed_diff: bool,
    edit_blocked_reported: bool,
    any_tool_called: bool,
) -> Option<IpcError> {
    if budget_closing || has_completion {
        return None;
    }
    let required_terminal_tool = match intent.side_effect_intent {
        ag::context::SideEffectIntentV2::ContentProposal => Some("propose_edit"),
        ag::context::SideEffectIntentV2::StructurePlan => Some("plan_workspace_operation"),
        ag::context::SideEffectIntentV2::None => None,
    };
    if required_terminal_tool == Some("plan_workspace_operation") && workspace_plan_completed {
        return None;
    }
    terminal_tool_is_missing(required_terminal_tool, proposed_diff, edit_blocked_reported)
        .then(|| required_terminal_tool_error(required_terminal_tool, any_tool_called))
}

/**
 * @GOV
 * codes:    BR-AG-STATE-001-EFFECT-AG-AG-TURN-075
 * type:     EFFECT
 * chain:    AG-TURN
 * rules:    BR-AG-STATE-001
 * boundary: in=ClarificationQuestionIntentInitialPromptAndContinuationMessages | out=ParkedAwaitingClarificationCheckpointAndClarificationRequiredEvent | effect=DB-park_agent_turn_for_clarification | consumer=BR-AG-STATE-001 | does_not=createNewAgentTurnOrAutoResume
 */
/// 进入 AwaitingClarification：整理 checkpoint、转移 SM-AG-01、停放并广播澄清请求。
/// 写事件/停放失败按原路径向上传播；不新建 AgentTurn。
#[allow(clippy::too_many_arguments)]
fn park_clarification<R: tauri::Runtime>(
    state: &AppState,
    app: &tauri::AppHandle<R>,
    emitter: &TurnEventEmitter<'_>,
    latch: &ag::turn::TerminalLatch,
    session_id: &str,
    turn_id: &str,
    task_run_id: Option<&str>,
    turn_state: &mut ag::turn::TurnState,
    intent: ag::context::IntentResultV2,
    initial_prompt: String,
    question: String,
    continuation_messages: Vec<serde_json::Value>,
    workspace_id: &str,
) -> Result<(), IpcError> {
    let mut checkpoint_messages = continuation_messages;
    checkpoint_after_read_clarification(
        &mut checkpoint_messages,
        (workspace_id, session_id, turn_id),
    );
    checkpoint_messages.push(serde_json::json!({
        "role": "assistant",
        "content": question,
    }));
    *turn_state = ag::turn::transition(*turn_state, ag::turn::TurnEvent::ModelClarification)
        .ok_or_else(sm_error)?;
    park_agent_turn_for_clarification(
        state,
        session_id,
        turn_id,
        task_run_id,
        AgentTurnRuntimeCheckpoint {
            intent,
            initial_prompt,
            continuation_messages: checkpoint_messages,
        },
    )?;
    emit_ag01(app, turn_id, *turn_state);
    emitter.emit(
        latch,
        AgTurnEventPayload::ClarificationRequired { text: question },
    )?;
    Ok(())
}

/// final 回答的收尾：派生/emit ProjectReport，并在成功时执行额度收尾与终态结算。
/// ProjectReport 失败仍按原路径结算 TaskRun Failed 并广播 Finalizing 错误；不推进 SM-AG-01。
struct FinalAnswerFacts<'a> {
    state: &'a AppState,
    latch: &'a ag::turn::TerminalLatch,
    workspace_id: &'a str,
    turn_id: &'a str,
    task_run_id: Option<&'a str>,
    intent: &'a ag::context::IntentResultV2,
    active_doc_path: Option<&'a str>,
    findings: &'a [ag::model_output::ModelFinalFindingV2],
    any_tool_called: bool,
    budget_closing: bool,
    completion: Option<&'a ag::model_output::ModelCompletion>,
    side_effect_intent: &'a ag::context::SideEffectIntentV2,
    structure_warning: Option<&'a str>,
    stop_reason: &'a str,
    remaining_tool_calls: u32,
    remaining_rounds: usize,
}

/**
 * @GOV
 * codes:    BR-AG-STATE-005-EFFECT-AG-AG-TURN-079, BR-AG-OBS-001-EFFECT-AG-AG-TURN-079
 * type:     EFFECT
 * chain:    AG-TURN
 * rules:    BR-AG-STATE-005, BR-AG-OBS-001
 * boundary: in=FinalAnswerIntentFindingsCompletionAndExecutionBudget | out=ProjectReportEventCompletedSettlementOrReportFailureSettlement | delegate=emit_project_report,settle_completed_turn | consumer=BR-AG-STATE-005,BR-AG-OBS-001 | does_not=advanceToolLoopTransitionOrCreateNewEvidence
 */
fn finish_final_answer<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    emitter: &TurnEventEmitter<'_>,
    facts: FinalAnswerFacts<'_>,
) -> Result<(), IpcError> {
    match emit_project_report(
        facts.state,
        emitter,
        facts.latch,
        facts.workspace_id,
        facts.turn_id,
        facts.intent,
        facts.active_doc_path,
        facts.findings,
        facts.any_tool_called,
    ) {
        Ok(true) => {}
        Ok(false) => return Ok(()),
        Err(error) => {
            if let Ok(conn) = facts.state.db.lock() {
                settle_optional_task_run(
                    &conn,
                    facts.task_run_id,
                    facts.turn_id,
                    ag::task::TurnOutcome::Failed,
                );
            }
            emit_ag_turn_error_from(
                emitter,
                facts.latch,
                app,
                facts.turn_id,
                &error,
                ag::turn::TurnState::Finalizing,
            );
            return Ok(());
        }
    }
    settle_completed_turn(
        facts.state,
        app,
        emitter,
        facts.latch,
        facts.turn_id,
        facts.task_run_id,
        facts.budget_closing,
        facts.completion,
        facts.side_effect_intent,
        facts.structure_warning,
        facts.stop_reason,
        facts.remaining_tool_calls,
        facts.remaining_rounds,
    )
}

struct RoundContext<'a, R: tauri::Runtime> {
    state: &'a AppState,
    app: tauri::AppHandle<R>,
    cancelled: Arc<AtomicBool>,
    terminal_latch: ag::turn::TerminalLatch,
    turn_id: String,
    session_id: String,
    workspace_id_for_turn: String,
    provider: String,
    model: String,
    runtime_profile: String,
    active_doc_path: Option<String>,
    explicit_edit_target_set: ag::target_set::ExplicitEditTargetSet,
    ws_root: Option<String>,
    history: Vec<ag::context::HistTurn>,
    raw_prompt: String,
    mention_block: Option<String>,
    reference_envelopes: Vec<ag::context::PromptDataEnvelopeV1>,
    on_context_request: Option<tauri::ipc::Channel<super::turn_runtime::AgentContextRequest>>,
    transport_protocol: String,
    model_capacity: ag::prompt_runtime::ModelCapacity,
    execution_policy: ag::tool_registry::ExecutionBudgetPolicy,
    workspace_plan_completed: bool,
    previous_model_calls: usize,
    remaining_rounds: usize,
}

struct RoundState {
    messages: Vec<serde_json::Value>,
    tools: serde_json::Value,
    capability_factors: ag::tool_registry::CapabilityFactorsV2,
    read_ctx: ag::blocks::AgentReadContext,
    active_snapshot: Option<ag::blocks::AgentEditableSnapshot>,
    intent_result: ag::context::IntentResultV2,
    continuation_messages: Vec<serde_json::Value>,
    task_run_id: Option<String>,
    remaining_tool_calls: u32,
    protocol_correction_rounds: usize,
    parameter_correction_rounds: usize,
    failed_read_signatures: BTreeSet<String>,
    seen_tool_call_ids: BTreeSet<String>,
    tool_loop_state: ag::tool::ToolLoopState,
    turn_state: ag::turn::TurnState,
    current_doc_prompt_coverage: Option<&'static str>,
    current_doc_requires_tool_read: bool,
    current_doc_read_confirmed: bool,
    any_tool_called: bool,
    proposed_diff: bool,
    edit_blocked_reported: bool,
    final_prompt_audit: Option<FinalPromptAuditLog>,
    capability_use_tracker: ag::tool_registry::CapabilityGrantUseTrackerV2,
    prompt_package: ag::context::PromptPackageV2,
    agent_context_facts: crate::types::AgentContextFacts,
}

/// 处理本轮 Done 结果：应用控制、澄清停放、终态工具检查与 final 收尾。
/// 该分支必然结束本轮；状态转移/结算仍在本函数内完成，错误按原路径传播。
struct DoneRoundFacts<'a> {
    state: &'a AppState,
    latch: &'a ag::turn::TerminalLatch,
    session_id: &'a str,
    workspace_id: &'a str,
    turn_id: &'a str,
    explicit_edit_target_set: &'a ag::target_set::ExplicitEditTargetSet,
    task_run_id: Option<String>,
    intent_result: ag::context::IntentResultV2,
    tool_loop_state: ag::tool::ToolLoopState,
    turn_state: ag::turn::TurnState,
    control: ag::model_output::ModelTurnControl,
    raw_prompt: String,
    history: &'a [ag::context::HistTurn],
    messages: &'a [serde_json::Value],
    active_doc_path: Option<&'a str>,
    reference_envelopes: &'a [ag::context::PromptDataEnvelopeV1],
    continuation_messages: Vec<serde_json::Value>,
    budget_closing: bool,
    workspace_plan_completed: bool,
    proposed_diff: bool,
    edit_blocked_reported: bool,
    any_tool_called: bool,
    visible_output: &'a str,
    stop_reason: &'a str,
    model_call_ordinal: usize,
    remaining_tool_calls: u32,
    remaining_rounds: usize,
}

#[allow(clippy::too_many_arguments)]
fn advance_or_settle<R: tauri::Runtime>(
    state: &AppState,
    app: &tauri::AppHandle<R>,
    emitter: &TurnEventEmitter<'_>,
    latch: &ag::turn::TerminalLatch,
    turn_id: &str,
    task_run_id: Option<&str>,
    tool_loop_state: &mut ag::tool::ToolLoopState,
    event: ag::tool::ToolLoopEvent,
) -> bool {
    match advance_tool_loop(app, turn_id, tool_loop_state, event) {
        Ok(_) => true,
        Err(error) => {
            settle_and_emit_ag_turn_error(state, task_run_id, emitter, latch, app, turn_id, &error);
            false
        }
    }
}

/**
 * @GOV
 * codes:    BR-AG-STATE-001-RB-AG-AG-TOOL-073, BR-AG-STATE-002-RB-AG-AG-TOOL-073
 * type:     RB
 * chain:    AG-TURN, AG-TOOL
 * rules:    BR-AG-STATE-001, BR-AG-STATE-002
 * boundary: in=DoneRoundControlVisibleAnswerAndRoundState | out=SingleAgentTurnTerminalOrClarificationParked | delegate=apply_control_and_observe,terminal_tool_requirement_error,park_clarification,finish_final_answer | consumer=BR-AG-STATE-001,BR-AG-STATE-002 | does_not=continueProviderLoopOrEmitSecondTerminal
 */
fn handle_done_round<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    emitter: &TurnEventEmitter<'_>,
    facts: DoneRoundFacts<'_>,
) -> Result<(), IpcError> {
    let DoneRoundFacts {
        state,
        latch,
        session_id,
        workspace_id,
        turn_id,
        explicit_edit_target_set,
        mut task_run_id,
        mut intent_result,
        mut tool_loop_state,
        mut turn_state,
        control,
        raw_prompt,
        history,
        messages,
        active_doc_path,
        reference_envelopes,
        continuation_messages,
        budget_closing,
        workspace_plan_completed,
        proposed_diff,
        edit_blocked_reported,
        any_tool_called,
        visible_output,
        stop_reason,
        model_call_ordinal,
        remaining_tool_calls,
        remaining_rounds,
    } = facts;
    let structure_warning = ag::model_output::answer_structure_warning(visible_output);
    emitter.record_internal(
        "answer_delivery_observed",
        serde_json::json!({
            "roundIndex": model_call_ordinal,
            "stopReason": stop_reason,
            "textUtf8Bytes": visible_output.len(),
            "textSha256": ag::context::sha256_text(visible_output),
            "contentBasis": "visible_body_excluding_control",
            "answerStructureWarning": structure_warning,
        }),
    )?;
    if let Err(error) = apply_control_and_observe(
        state,
        emitter,
        latch,
        session_id,
        workspace_id,
        turn_id,
        explicit_edit_target_set,
        &mut task_run_id,
        &mut intent_result,
        &control,
        raw_prompt.as_str(),
        history,
        messages,
        active_doc_path,
        reference_envelopes,
        &continuation_messages,
    ) {
        if !advance_or_settle(
            state,
            app,
            emitter,
            latch,
            turn_id,
            task_run_id.as_deref(),
            &mut tool_loop_state,
            ag::tool::ToolLoopEvent::LoopError,
        ) {
            return Ok(());
        }
        settle_and_emit_ag_turn_error(
            state,
            task_run_id.as_deref(),
            emitter,
            latch,
            app,
            turn_id,
            &error,
        );
        return Ok(());
    }
    match control.next_action {
        ag::model_output::ModelNextAction::Clarify => {
            let clarification_question = control
                .clarification_question
                .expect("validated clarification must include a question");
            park_clarification(
                state,
                app,
                emitter,
                latch,
                session_id,
                turn_id,
                task_run_id.as_deref(),
                &mut turn_state,
                intent_result,
                raw_prompt,
                clarification_question,
                continuation_messages,
                workspace_id,
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
            if !advance_or_settle(
                state,
                app,
                emitter,
                latch,
                turn_id,
                task_run_id.as_deref(),
                &mut tool_loop_state,
                ag::tool::ToolLoopEvent::LoopError,
            ) {
                return Ok(());
            }
            settle_and_emit_ag_turn_error(
                state,
                task_run_id.as_deref(),
                emitter,
                latch,
                app,
                turn_id,
                &error,
            );
            return Ok(());
        }
        ag::model_output::ModelNextAction::Final => {}
    }
    if let Some(error) = terminal_tool_requirement_error(
        budget_closing,
        control.completion.is_some(),
        workspace_plan_completed,
        &intent_result,
        proposed_diff,
        edit_blocked_reported,
        any_tool_called,
    ) {
        if !advance_or_settle(
            state,
            app,
            emitter,
            latch,
            turn_id,
            task_run_id.as_deref(),
            &mut tool_loop_state,
            ag::tool::ToolLoopEvent::LoopError,
        ) {
            return Ok(());
        }
        if let Ok(conn) = state.db.lock() {
            settle_optional_task_run(
                &conn,
                task_run_id.as_deref(),
                turn_id,
                ag::task::TurnOutcome::Failed,
            );
        }
        emit_ag_turn_error(emitter, latch, app, turn_id, &error);
        return Ok(());
    }
    if !advance_or_settle(
        state,
        app,
        emitter,
        latch,
        turn_id,
        task_run_id.as_deref(),
        &mut tool_loop_state,
        ag::tool::ToolLoopEvent::FinalAnswer,
    ) {
        return Ok(());
    }
    turn_state =
        ag::turn::transition(turn_state, ag::turn::TurnEvent::ModelFinal).ok_or_else(sm_error)?;
    emit_ag01(app, turn_id, turn_state);
    finish_final_answer(
        app,
        emitter,
        FinalAnswerFacts {
            state,
            latch,
            workspace_id,
            turn_id,
            task_run_id: task_run_id.as_deref(),
            intent: &intent_result,
            active_doc_path,
            findings: &control.findings,
            any_tool_called,
            budget_closing,
            completion: control.completion.as_ref(),
            side_effect_intent: &control.intent.side_effect_intent,
            structure_warning,
            stop_reason,
            remaining_tool_calls,
            remaining_rounds,
        },
    )
}

/// 回合收尾：纠正指令、只读失败守卫、SM-AG-01 转移、Prompt 重建与 Assembled。
/// Prompt 重建的 Settle 失败在这里结算并返回 Ended；Propagate 失败向上传播。
struct CloseRoundFacts<'a> {
    state: &'a AppState,
    latch: &'a ag::turn::TerminalLatch,
    session_id: &'a str,
    turn_id: &'a str,
    runtime_profile: &'a str,
    task_run_id: &'a mut Option<String>,
    intent_result: &'a mut ag::context::IntentResultV2,
    agent_context_facts: &'a mut crate::types::AgentContextFacts,
    explicit_edit_target_set: &'a ag::target_set::ExplicitEditTargetSet,
    read_ctx: &'a mut ag::blocks::AgentReadContext,
    active_doc_path: Option<&'a str>,
    active_snapshot: &'a mut Option<ag::blocks::AgentEditableSnapshot>,
    history: &'a [ag::context::HistTurn],
    raw_prompt: &'a str,
    mention_block: Option<&'a str>,
    reference_envelopes: &'a [ag::context::PromptDataEnvelopeV1],
    proposed_diff: bool,
    continuation_messages: &'a mut Vec<serde_json::Value>,
    model_call_ordinal: usize,
    remaining_tool_calls: u32,
    model_capacity: ag::prompt_runtime::ModelCapacity,
    response_rejected: bool,
    transport_protocol: &'a str,
    final_only_correction_pending: bool,
    parameter_correction_pending: bool,
    protocol_correction_diagnostic: &'a Option<serde_json::Value>,
    read_failure_final_pending: bool,
    turn_state: &'a mut ag::turn::TurnState,
    messages: &'a mut Vec<serde_json::Value>,
    tools: &'a mut serde_json::Value,
    capability_factors: &'a mut ag::tool_registry::CapabilityFactorsV2,
    prompt_package: &'a mut ag::context::PromptPackageV2,
    current_doc_prompt_coverage: &'a mut Option<&'static str>,
}

enum RoundClose {
    Continue,
    Ended,
}

/**
 * @GOV
 * codes:    BR-AG-STATE-001-RB-AG-AG-TOOL-074, BR-AG-DATA-001-RB-AG-AG-TOOL-074
 * type:     RB
 * chain:    AG-TURN, AG-TOOL
 * rules:    BR-AG-STATE-001, BR-AG-DATA-001
 * boundary: in=RoundCorrectionFlagsContinuationMessagesAndPromptFacts | out=RoundCloseContinueOrEndedWithRebuiltPrompt | delegate=rebuild_round_prompt | consumer=BR-AG-STATE-001,BR-AG-DATA-001 | does_not=executeToolOrCallProvider
 */
fn close_round<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    emitter: &TurnEventEmitter<'_>,
    facts: CloseRoundFacts<'_>,
) -> Result<RoundClose, IpcError> {
    let CloseRoundFacts {
        state,
        latch,
        session_id,
        turn_id,
        runtime_profile,
        task_run_id,
        intent_result,
        agent_context_facts,
        explicit_edit_target_set,
        read_ctx,
        active_doc_path,
        active_snapshot,
        history,
        raw_prompt,
        mention_block,
        reference_envelopes,
        proposed_diff,
        continuation_messages,
        model_call_ordinal,
        remaining_tool_calls,
        model_capacity,
        response_rejected,
        transport_protocol,
        final_only_correction_pending,
        parameter_correction_pending,
        protocol_correction_diagnostic,
        read_failure_final_pending,
        turn_state,
        messages,
        tools,
        capability_factors,
        prompt_package,
        current_doc_prompt_coverage,
    } = facts;
    if final_only_correction_pending {
        continuation_messages.push(correction_instruction_for_protocol(
            true,
            transport_protocol,
        ));
    } else if parameter_correction_pending {
        let mut instruction = correction_instruction_for_protocol(false, transport_protocol);
        if let Some(diagnostic) = protocol_correction_diagnostic.as_ref() {
            // Both values are generated from fixed schema vocabulary, never from the
            // rejected provider arguments. Give the model a precise repair target
            // without reintroducing the malformed payload as an example.
            let reason = diagnostic["reasonCode"]
                .as_str()
                .unwrap_or("schema_invalid");
            let field = diagnostic["fieldPath"].as_str().unwrap_or("$batch");
            let base = instruction["content"].as_str().unwrap_or_default();
            instruction["content"] = serde_json::json!(format!(
                "本次拒绝位置：{field}；原因类别：{reason}。请从当前 schema 重新生成完整参数，不复制上一份被拒参数。\n{base}"
            ));
        }
        continuation_messages.push(instruction);
    }
    if read_failure_final_pending {
        continuation_messages.push(serde_json::json!({
            "role":"system", "layer":"read_failure_final_only",
            "content":"相同只读请求已重复失败，禁止再调用任何工具。请只输出合法 final 或 clarify：说明已有成功证据、失败的读取及未完成范围，不能将失败读取视为已读，不能宣称原业务任务成功；如无法继续，明确说明需要用户补充或缩小什么范围。不要重放任何已有操作。"
        }));
    }
    *turn_state = ag::turn::transition(
        *turn_state,
        if response_rejected {
            ag::turn::TurnEvent::ModelProtocolRejected
        } else {
            ag::turn::TurnEvent::ToolRoundCompleted
        },
    )
    .ok_or_else(sm_error)?;
    emit_ag01(app, turn_id, *turn_state);
    let rebuilt = match rebuild_round_prompt(
        emitter,
        PromptRebuildFacts {
            state,
            session_id,
            turn_id,
            runtime_profile,
            task_run_id: task_run_id.as_deref(),
            intent_result,
            agent_context_facts,
            explicit_edit_target_set,
            read_ctx,
            active_doc_path,
            active_snapshot: active_snapshot.as_ref(),
            history,
            raw_prompt,
            mention_block,
            reference_envelopes,
            proposed_diff,
            continuation_messages,
            decision_receipt_ids: &capability_factors.decision_receipt_ids,
            next_model_call: (model_call_ordinal + 1) as u32,
            remaining_tool_calls,
            model_capacity,
            response_rejected,
        },
    ) {
        Ok(rebuilt) => rebuilt,
        Err(PromptRebuildFailure::Propagate(error)) => return Err(error),
        Err(PromptRebuildFailure::Settle(error)) => {
            settle_and_emit_ag_turn_error_from(
                state,
                task_run_id.as_deref(),
                emitter,
                latch,
                app,
                turn_id,
                &error,
                ag::turn::TurnState::Assembling,
            );
            return Ok(RoundClose::Ended);
        }
    };
    *messages = rebuilt.messages;
    *tools = rebuilt.tools;
    *capability_factors = rebuilt.capability_factors;
    *prompt_package = rebuilt.prompt_package;
    *current_doc_prompt_coverage = rebuilt.current_doc_prompt_coverage;
    emitter.record_internal(
        "prompt_rebuilt",
        serde_json::json!({
            "nextRound": model_call_ordinal + 1,
            "intentRevision": intent_result.revision,
            "contextTraceId": prompt_package.context_trace_id,
            "toolManifestHash": prompt_package.tool_manifest_hash,
        }),
    )?;
    *turn_state =
        ag::turn::transition(*turn_state, ag::turn::TurnEvent::Assembled).ok_or_else(sm_error)?;
    emit_ag01(app, turn_id, *turn_state);
    Ok(RoundClose::Continue)
}

/**
 * @GOV
 * codes:    BR-AG-STATE-002-RB-AG-AG-TOOL-072
 * type:     RB
 * chain:    AG-TURN, AG-TOOL
 * rules:    BR-AG-STATE-002
 * term_ref: TERM-AG-014, TERM-AG-015, TERM-AG-016, TERM-AG-020
 * boundary: in=RoundContextAndRoundState | out=SingleAgentTurnTerminalOrContinuedRoundFacts | delegate=await_round_gate,run_provider_round,prepare_tool_batch,execute_tool_call,record_tool_result,handle_done_round,close_round,finish_final_answer,rebuild_round_prompt | consumer=BR-AG-STATE-002 | does_not=executeToolOutsideFreshGrantOrTreatPromptIntentAsAuthorization
 */
/// 执行回合的 provider/tool 循环；不可变上下文与可变回合状态显式分离。
/// 终态路径仍在本函数内结算并返回 Ok(())，错误按原路径向上传播。
async fn run_round_loop<R: tauri::Runtime>(
    emitter: &TurnEventEmitter<'_>,
    ctx: RoundContext<'_, R>,
    round: RoundState,
) -> Result<(), IpcError> {
    let RoundContext {
        state,
        app,
        cancelled,
        terminal_latch,
        turn_id,
        session_id,
        workspace_id_for_turn,
        provider,
        model,
        runtime_profile,
        active_doc_path,
        explicit_edit_target_set,
        ws_root,
        history,
        raw_prompt,
        mention_block,
        reference_envelopes,
        on_context_request,
        transport_protocol,
        model_capacity,
        execution_policy,
        workspace_plan_completed,
        previous_model_calls,
        remaining_rounds,
    } = ctx;
    let event_emitter = emitter;
    let RoundState {
        mut messages,
        mut tools,
        mut capability_factors,
        mut read_ctx,
        mut active_snapshot,
        mut intent_result,
        mut continuation_messages,
        mut task_run_id,
        mut remaining_tool_calls,
        mut protocol_correction_rounds,
        mut parameter_correction_rounds,
        mut failed_read_signatures,
        mut seen_tool_call_ids,
        mut tool_loop_state,
        mut turn_state,
        mut current_doc_prompt_coverage,
        mut current_doc_requires_tool_read,
        mut current_doc_read_confirmed,
        mut any_tool_called,
        mut proposed_diff,
        mut edit_blocked_reported,
        mut final_prompt_audit,
        mut capability_use_tracker,
        mut prompt_package,
        mut agent_context_facts,
    } = round;
    use ag::tool::{ToolLoopEvent as TlEv, ToolLoopState as TlSt};
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
    if (remaining_tool_calls == 0 && execution_policy.final_reserve == 0) || remaining_rounds == 0 {
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
        let budget_closing = messages
            .iter()
            .any(|m| m["role"] == "system" && m["layer"] == "execution_budget_final_only");
        let model_call_ordinal = previous_model_calls + _round + 1;
        event_emitter.record_internal("execution_budget_checked", serde_json::json!({
            "roundIndex": model_call_ordinal, "maxModelCalls":execution_policy.max_model_calls,
            "maxToolCalls":execution_policy.max_tool_calls, "maxCorrectionsPerCategory":execution_policy.max_corrections,
            "controlCorrections":protocol_correction_rounds, "parameterCorrections":parameter_correction_rounds,
            "finalReserve":execution_policy.final_reserve, "remainingModelCalls":remaining_rounds - _round,
            "remainingToolCalls":remaining_tool_calls, "budgetClosing":budget_closing
        }))?;
        if cancelled.load(Ordering::Acquire) {
            // Between rounds SM-AG-02 is already frozen; only the outer AgentTurn is active.
            if !ag::tool::is_terminal(tool_loop_state) {
                advance_or_finish!(TlEv::Cancel);
            }
            finish_cancelled_turn(
                &state,
                &app,
                &event_emitter,
                &terminal_latch,
                &turn_id,
                task_run_id.as_deref(),
                turn_state,
            );
            return Ok(());
        }
        if _round > 0 {
            debug_assert_eq!(turn_state, ag::turn::TurnState::AwaitingGate);
            match await_round_gate(
                &state,
                &event_emitter,
                &session_id,
                &turn_id,
                model_call_ordinal,
            )
            .await?
            {
                RoundGate::Proceed => {
                    turn_state = ag::turn::transition(turn_state, ag::turn::TurnEvent::GatePassed)
                        .ok_or_else(sm_error)?;
                    emit_ag01(&app, &turn_id, turn_state);
                    // SM-AG-01 已完成 Assembling→AwaitingGate→GatePassed；这里是新的嵌套激活起点，
                    // 从初始态 ModelStreaming 开始，不是同一 SM-AG-02 激活内的状态转移。
                    tool_loop_state = TlSt::ModelStreaming;
                }
                RoundGate::Cancelled => {
                    // The preceding nested loop completed before this Gate wait.
                    finish_cancelled_turn(
                        &state,
                        &app,
                        &event_emitter,
                        &terminal_latch,
                        &turn_id,
                        task_run_id.as_deref(),
                        turn_state,
                    );
                    return Ok(());
                }
                RoundGate::Denied(error) | RoundGate::Failed(error) => {
                    if let Ok(conn) = state.db.lock() {
                        settle_optional_task_run(
                            &conn,
                            task_run_id.as_deref(),
                            &turn_id,
                            ag::task::TurnOutcome::Failed,
                        );
                    }
                    if error.code == "AG_GATE_DENIED" {
                        emit_ag_turn_denied(
                            &event_emitter,
                            &terminal_latch,
                            &app,
                            &turn_id,
                            &error,
                        );
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
            }
        }
        debug_assert_eq!(tool_loop_state, TlSt::ModelStreaming);
        // layer 是审计/装配内部元数据，发送 provider 前剥离（不进入模型输入）。
        let provider_result = run_provider_round(
            state,
            &event_emitter,
            &terminal_latch,
            &session_id,
            &turn_id,
            &provider,
            &model,
            &runtime_profile,
            &messages,
            &tools,
            &prompt_package,
            &mut final_prompt_audit,
            model_call_ordinal,
            budget_closing,
            workspace_plan_completed,
        )
        .await?;
        let (round, output_decoder, visible_output, visible_output_chars) = match provider_result {
            ProviderRoundResult::Ready(ready) => (
                ready.round,
                ready.decoder,
                ready.visible_output,
                ready.visible_output_chars,
            ),
            ProviderRoundResult::Cancelled => {
                advance_or_finish!(TlEv::Cancel);
                finish_cancelled_turn(
                    &state,
                    &app,
                    &event_emitter,
                    &terminal_latch,
                    &turn_id,
                    task_run_id.as_deref(),
                    turn_state,
                );
                return Ok(());
            }
            ProviderRoundResult::Failed(error) => {
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
        // A provider completion and an accepted Binder response are separate facts. Text
        // protocol errors are collected until the native response kind is known, before any
        // intent/task projection or tool execution. Network failures never enter correction.
        let final_only_after_rejection = correction_final_only(
            &continuation_messages,
            (&workspace_id_for_turn, &session_id, &turn_id),
        );
        let response_control = validate_round_response(
            &round,
            &output_decoder,
            budget_closing,
            workspace_plan_completed,
            visible_output_chars,
            any_tool_called,
            &continuation_messages,
        );
        let response_rejected = response_control.is_err();
        let response_rejection_kind = matches!(round, ent::gateway::RoundOutcome::Done { .. })
            .then(|| output_decoder.response_rejection_kind());
        let mut final_only_correction_pending = false;
        let mut parameter_correction_pending = false;
        let mut protocol_correction_diagnostic = None;
        let mut read_failure_final_pending = false;
        if let Err(error) = &response_control {
            protocol_correction_rounds += 1;
            let final_only = final_only_after_rejection;
            event_emitter.record_internal("model_control_rejected", serde_json::json!({
                "code": error.code, "prefixKind": output_decoder.prefix_kind(),
                "rejectionStage":"response_completion", "correctionAttempt":protocol_correction_rounds,
                "providerCompletionSeen":true, "finalOnly":final_only,
                "responseRejectionKind":response_rejection_kind,
            }))?;
            // Once visible body has streamed, do not append a second answer to the same body.
            // Oversized quarantine failures also stop immediately in the callback above.
            if protocol_correction_rounds > execution_policy.max_corrections
                || _round + 1 == remaining_rounds
                || visible_output_chars > 0
            {
                if budget_closing && visible_output_chars == 0 {
                    event_emitter.record_internal(
                        "budget_close_fallback",
                        serde_json::json!({"code":error.code,"budgetClosing":true}),
                    )?;
                    event_emitter.emit(&terminal_latch, AgTurnEventPayload::Chunk { text: "本次执行额度已到；模型未提供可验证的收尾回答。此前工具回执和效果记录保留，未完成部分不代表成功；请核对执行效果后明确提出后续请求。系统没有自动重放操作。".into() })?;
                    if let Ok(conn) = state.db.lock() {
                        settle_optional_task_run(
                            &conn,
                            task_run_id.as_deref(),
                            &turn_id,
                            ag::task::TurnOutcome::Failed,
                        );
                    }
                    advance_or_finish!(TlEv::ProtocolRejected);
                    emit_ag_turn_done(
                        &event_emitter,
                        &terminal_latch,
                        &app,
                        &turn_id,
                        "budget_exhausted",
                        turn_state,
                    );
                    return Ok(());
                }
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
            let instruction = response_rejection_kind.map_or_else(
                || correction_instruction_for_protocol(final_only, &transport_protocol),
                |kind| {
                    response_correction_instruction_for_protocol(
                        final_only,
                        kind,
                        &transport_protocol,
                    )
                },
            );
            continuation_messages.push(instruction);
            if error.code == "AG_WORKSPACE_PLAN_TOOL_REQUIRED" {
                continuation_messages.push(serde_json::json!({"role":"system", "layer":"action_handoff_correction", "content":"上一final声明需要结构计划但没有计划，本段正文未展示。若本轮已要求恢复资源，请在核对当前对象后调用plan_workspace_operation生成真实裁决请求；缺信息则clarify；若确实仅报告、阻断或无需操作，final必须提供completion.status和具体reason，不得假称已执行或已进入等待。不得重放已有操作。"}));
            }
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
                return handle_done_round(
                    &app,
                    &event_emitter,
                    DoneRoundFacts {
                        state,
                        latch: &terminal_latch,
                        session_id: &session_id,
                        workspace_id: &workspace_id_for_turn,
                        turn_id: &turn_id,
                        explicit_edit_target_set: &explicit_edit_target_set,
                        task_run_id,
                        intent_result,
                        tool_loop_state,
                        turn_state,
                        control,
                        raw_prompt,
                        history: &history,
                        messages: &messages,
                        active_doc_path: active_doc_path.as_deref(),
                        reference_envelopes: &reference_envelopes,
                        continuation_messages,
                        budget_closing,
                        workspace_plan_completed,
                        proposed_diff,
                        edit_blocked_reported,
                        any_tool_called,
                        visible_output: &visible_output,
                        stop_reason: &stop_reason,
                        model_call_ordinal,
                        remaining_tool_calls,
                        remaining_rounds,
                    },
                );
            }
            ent::gateway::RoundOutcome::ToolCalls(calls) => {
                let output_control = response_control.expect("validated optional control");
                let prepared = match prepare_tool_batch(
                    &event_emitter,
                    ToolBatchFacts {
                        state,
                        latch: &terminal_latch,
                        session_id: &session_id,
                        workspace_id: &workspace_id_for_turn,
                        turn_id: &turn_id,
                        explicit_edit_target_set: &explicit_edit_target_set,
                        calls,
                        transport_protocol: &transport_protocol,
                        output_control,
                        seen_tool_call_ids: &mut seen_tool_call_ids,
                        protocol_correction_rounds: &mut protocol_correction_rounds,
                        protocol_correction_diagnostic: &mut protocol_correction_diagnostic,
                        max_corrections: execution_policy.max_corrections,
                        task_run_id: &mut task_run_id,
                        intent_result: &mut intent_result,
                        raw_prompt: &raw_prompt,
                        history: &history,
                        messages: &messages,
                        active_doc_path: active_doc_path.as_deref(),
                        reference_envelopes: &reference_envelopes,
                        continuation_messages: &continuation_messages,
                    },
                ) {
                    Ok(prepared) => prepared,
                    Err(ToolBatchFailure::Propagate(error)) => return Err(error),
                    Err(ToolBatchFailure::Settle(error)) => {
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
                let calls = prepared.calls;
                let parsed_params = prepared.parsed_params;
                let control_error = prepared.control_error;
                let native_transport = prepared.native_transport;
                let mut correction_counted_this_round = control_error.is_some();
                if control_error.is_some() {
                    // The response-level rejection happened before the per-tool parameter
                    // branch below. It still needs an explicit correction on the next model
                    // request; otherwise the model sees neither the rejected arguments nor
                    // the reason to rebuild them.
                    if final_only_after_rejection {
                        final_only_correction_pending = true;
                    } else {
                        parameter_correction_pending = true;
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
                let mut assistant_tool_calls = serde_json::json!({ "role": "assistant", "content": null, "tool_calls": tc_json });
                if control_error.is_none() {
                    if let Some(native) = native_transport.as_ref() {
                        event_emitter.record_internal("tool_transport_observed", serde_json::json!({"protocolVersion":"binder-tool-batch-v1","businessCallCount":calls.len()}))?;
                        assistant_tool_calls["nativeBatch"] = serde_json::json!({"id":native.id,"type":"function","function":{"name":native.name,"arguments":native.arguments}});
                    } else {
                        event_emitter.record_internal("tool_transport_observed", serde_json::json!({"protocolVersion":&transport_protocol,"businessCallCount":calls.len()}))?;
                    }
                }
                messages.push(assistant_tool_calls.clone());
                // A batch rejected before execution must remain in the internal event and
                // receipt trail, but its malformed arguments are not a useful model example.
                // Keep only the safe correction instruction in the next provider request.
                if control_error.is_none() {
                    continuation_messages.push(assistant_tool_calls);
                }

                let mut stop_after_current_tool = false;
                for (call_index, c) in calls.iter().enumerate() {
                    if cancelled.load(Ordering::Acquire) {
                        advance_or_finish!(TlEv::Cancel);
                        finish_cancelled_turn(
                            &state,
                            &app,
                            &event_emitter,
                            &terminal_latch,
                            &turn_id,
                            task_run_id.as_deref(),
                            turn_state,
                        );
                        return Ok(());
                    }
                    advance_or_finish!(TlEv::ToolRequest);
                    let params = parsed_params[call_index].clone();
                    let arg_parse_error = control_error.as_ref().map(|error| {
                        serde_json::json!({
                            "code": &error.code,
                            "message": format!("{}；本批没有执行任何工具。{}", error.message,
                                if transport_protocol == ag::model_output::BATCH_TOOL_PROTOCOL {
                                    "请按当前schema修正并仅提交一次binder_tool_batch，参数为唯一control和calls[{toolName,params}]，业务params不得含_binderTurn；不要重复已有操作。"
                                } else {
                                    "请按本原回合已冻结的工具schema修正响应控制，不得更换协议或重复已有操作。"
                                }),
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
                    let mut end_after_diff_proposal = false;
                    // Preflight acquisition under the same hard boundaries; this temporary grant never executes a tool.
                    // The real one-use execution grant is computed below after acquisition.
                    let acquisition_allowed = preflight_tool_acquisition_allowed(
                        &capability_factors,
                        &cancelled,
                        &terminal_latch,
                        &runtime_profile,
                        &turn_id,
                        &explicit_edit_target_set,
                        model_call_ordinal,
                        remaining_tool_calls,
                        &tools,
                        ws_root.as_deref(),
                        read_failure_final_pending,
                        arg_parse_error.is_none(),
                        c,
                        &params,
                    );
                    let context_refresh_error = if acquisition_allowed
                        && ag::tool_registry::lookup(&c.name)
                            .is_some_and(|tool| tool.requires_fresh_host_context)
                    {
                        refresh_tool_context(
                            &event_emitter,
                            on_context_request.as_ref(),
                            &mut agent_context_facts,
                            &mut read_ctx,
                            &mut active_snapshot,
                            active_doc_path.as_deref(),
                            &c.id,
                            &c.name,
                            &params,
                            cancelled.clone(),
                        )
                        .await?
                    } else {
                        None
                    };
                    let context_failure_summary = context_refresh_error.as_ref().map(|error| format!("[{}] 专业上下文未就绪、已变化或采集失败；本次工具未执行，请就绪后重新提交", error.code));
                    // 每个 ToolCall 在真实 GatePassed 后重新取得一次性 CapabilityGrantV2。Prompt Manifest
                    // 只决定模型当时可见的工具，不能拿来执行；TaskPlan/IntentResult/ToolResult 内容均不授权。
                    let grant = match resolve_execution_grant_and_decision(
                        &event_emitter,
                        ToolGrantFacts {
                            latch: &terminal_latch,
                            cancelled: &cancelled,
                            runtime_profile: &runtime_profile,
                            turn_id: &turn_id,
                            has_task_run: task_run_id.is_some(),
                            capability_factors: &capability_factors,
                            model_call_ordinal,
                            remaining_tool_calls,
                            explicit_edit_target_set: &explicit_edit_target_set,
                            tools: &tools,
                            ws_root: ws_root.as_deref(),
                            read_failure_final_pending,
                            arg_parse_error_none: arg_parse_error.is_none(),
                            final_only_after_rejection,
                            max_corrections: execution_policy.max_corrections,
                            call: c,
                            params: &params,
                            capability_use_tracker: &mut capability_use_tracker,
                            parameter_correction_rounds: &mut parameter_correction_rounds,
                            correction_counted_this_round: &mut correction_counted_this_round,
                            parameter_correction_pending: &mut parameter_correction_pending,
                            final_only_correction_pending: &mut final_only_correction_pending,
                        },
                    ) {
                        Ok(grant) => grant,
                        Err(ToolGrantFailure::Propagate(error)) => return Err(error),
                        Err(ToolGrantFailure::Settle(error)) => {
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
                    let execution_grant = grant.execution_grant;
                    let tool_decision = grant.tool_decision;
                    let parameter_contract_rejection = grant.parameter_contract_rejection;
                    let scope_candidates = compute_scope_candidates(
                        tool_decision.as_ref(),
                        &c.name,
                        &params,
                        ws_root.as_deref(),
                        &explicit_edit_target_set,
                    );
                    // 简单回合保持无 TaskRun。只有已获准的任务级工具真的到达执行边界时，
                    // 才在创建 DecisionRequest / WorkspaceOperationPlan 之前取得并绑定 TaskRun。
                    let task_run_prepare_error = prepare_task_run_for_tool(
                        &state,
                        &event_emitter,
                        &session_id,
                        &workspace_id_for_turn,
                        &turn_id,
                        &c.name,
                        &intent_result.goal,
                        &explicit_edit_target_set,
                        scope_candidates.is_ok(),
                        matches!(tool_decision.as_ref(), Some(ag::tool::ToolDecision::Allow)),
                        &mut task_run_id,
                    );
                    let (execution_failure, execution_guard_error) = compute_execution_failure(
                        control_error.as_ref(),
                        parameter_contract_rejection,
                        tool_decision.as_ref(),
                        &c.name,
                        &params,
                        read_failure_final_pending,
                        context_refresh_error.as_ref(),
                        &scope_candidates,
                        task_run_prepare_error.as_ref(),
                    );
                    let mut execution_failure = execution_failure;
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
                    let capability_identity = tool_call_receipt_identity(
                        &workspace_id_for_turn,
                        &session_id,
                        &turn_id,
                        &c.id,
                        &c.name,
                        &execution_grant.grant_id,
                    );
                    let validation_allows_execution = arg_parse_error.is_none()
                        && execution_guard_error.is_none()
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
                    let execution = execute_tool_call(
                        &app,
                        ToolExecutionFacts {
                            state,
                            cancelled: &cancelled,
                            call: c,
                            params: &params,
                            arg_parse_error: arg_parse_error.as_ref(),
                            execution_guard_error: execution_guard_error.as_ref(),
                            tool_decision: tool_decision.as_ref(),
                            execution_failure,
                            read_execution,
                            ws_root: ws_root.as_deref(),
                            read_ctx: &read_ctx,
                            active_doc_path: active_doc_path.as_deref(),
                            explicit_edit_target_set: &explicit_edit_target_set,
                            capability_identity: &capability_identity,
                            task_run_id: task_run_id.as_deref(),
                            scope_candidates: &scope_candidates,
                            current_doc_requires_tool_read,
                            current_doc_read_confirmed,
                            workspace_id: &workspace_id_for_turn,
                            session_id: &session_id,
                            turn_id: &turn_id,
                            proposed_diff,
                            edit_blocked_reported,
                        },
                    )
                    .await;
                    let result = execution.result;
                    let success = execution.success;
                    diff_events_after_tool = execution.diff_events;
                    decision_event_after_tool = execution.decision;
                    end_after_diff_proposal = execution.end_after_diff;
                    proposed_diff = execution.proposed_diff;
                    edit_blocked_reported = execution.edit_blocked_reported;
                    execution_failure = execution.execution_failure;
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
                            "hasCursor": params.get("cursor").is_some(),
                            "nonInitialPage": params.get("cursor").is_some() || params.get("offset").and_then(|v|v.as_u64()).is_some_and(|offset|offset>0),
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
                        finish_cancelled_turn(
                            &state,
                            &app,
                            &event_emitter,
                            &terminal_latch,
                            &turn_id,
                            task_run_id.as_deref(),
                            turn_state,
                        );
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
                    if !record_tool_result(
                        &event_emitter,
                        &terminal_latch,
                        ToolResultFacts {
                            workspace_id: &workspace_id_for_turn,
                            turn_id: &turn_id,
                            tool_call_id: &c.id,
                            tool_name: &c.name,
                            params: &params,
                            read_ctx: &read_ctx,
                            result: &result,
                            success,
                            validation_allows_execution,
                            execution_failure: execution_failure.as_ref(),
                            context_failure_summary,
                            identity: &capability_identity,
                            diff_events: &diff_events_after_tool,
                            decision: decision_event_after_tool.as_ref(),
                            repeated_read_failure,
                            control_error: control_error.is_some(),
                            task_run_id: task_run_id.as_deref(),
                        },
                        &mut messages,
                        &mut continuation_messages,
                    )? {
                        return Ok(());
                    }
                    if let Some(decision) = decision_event_after_tool {
                        if final_only_correction_pending {
                            continuation_messages.push(correction_instruction_for_protocol(
                                true,
                                &transport_protocol,
                            ));
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
                                finish_cancelled_turn(
                                    &state,
                                    &app,
                                    &event_emitter,
                                    &terminal_latch,
                                    &turn_id,
                                    task_run_id.as_deref(),
                                    turn_state,
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
                    let loop_budget_reached = execution_policy.final_reserve == 0
                        && (remaining_tool_calls == 0
                            || (_round + 1 == remaining_rounds && !has_more_calls));
                    if loop_budget_reached {
                        advance_or_finish!(TlEv::LoopLimit);
                        stop_after_current_tool = true;
                    } else if end_after_diff_proposal && success {
                        advance_or_finish!(TlEv::ProposalRecorded);
                        stop_after_current_tool = true;
                    } else if has_more_calls {
                        // 同一响应还有独立 ToolCall：经转移引擎保持当前激活，
                        // 不直接改写状态，也不回归 SM-AG-01。
                        advance_or_finish!(TlEv::NextToolCall);
                    } else {
                        advance_or_finish!(TlEv::Recorded);
                    }
                    if stop_after_current_tool {
                        break;
                    }
                }
                if _round + 1 == remaining_rounds
                    || (remaining_tool_calls == 0 && execution_policy.final_reserve == 0)
                {
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
        match close_round(
            &app,
            &event_emitter,
            CloseRoundFacts {
                state,
                latch: &terminal_latch,
                session_id: &session_id,
                turn_id: &turn_id,
                runtime_profile: &runtime_profile,
                task_run_id: &mut task_run_id,
                intent_result: &mut intent_result,
                agent_context_facts: &mut agent_context_facts,
                explicit_edit_target_set: &explicit_edit_target_set,
                read_ctx: &mut read_ctx,
                active_doc_path: active_doc_path.as_deref(),
                active_snapshot: &mut active_snapshot,
                history: &history,
                raw_prompt: &raw_prompt,
                mention_block: mention_block.as_deref(),
                reference_envelopes: &reference_envelopes,
                proposed_diff,
                continuation_messages: &mut continuation_messages,
                model_call_ordinal,
                remaining_tool_calls,
                model_capacity,
                response_rejected,
                transport_protocol: &transport_protocol,
                final_only_correction_pending,
                parameter_correction_pending,
                protocol_correction_diagnostic: &protocol_correction_diagnostic,
                read_failure_final_pending,
                turn_state: &mut turn_state,
                messages: &mut messages,
                tools: &mut tools,
                capability_factors: &mut capability_factors,
                prompt_package: &mut prompt_package,
                current_doc_prompt_coverage: &mut current_doc_prompt_coverage,
            },
        )? {
            RoundClose::Continue => {}
            RoundClose::Ended => return Ok(()),
        }
    }

    debug_assert!(false, "每个 provider 结果都应在循环内部返回或推进");
    Ok(())
}

/**
 * @GOV
 * codes:    BR-AG-STATE-002-RB-AG-AG-TOOL-018
 * type:     RB
 * chain:    AG-TURN, AG-TOOL
 * rules:    BR-AG-STATE-002
 * term_ref: TERM-AG-014, TERM-AG-015, TERM-AG-016, TERM-AG-020
 * boundary: in=PreparedPromptAgentContextFactsPromptManifestCapabilityFactorsAndActiveAgentTurnWithOptionalTaskRun | out=PerToolCallCapabilityGrantValidatedToolResultsEvidenceLedgerDerivedProjectReportPendingDiffOrWorkspaceOperationDecisionRequestAndSingleAgentTurnTerminal | delegate=run_round_loop,resolve_capability_grant_v2,validate_requested_tool_for_grant,advance_tool_loop,validate_round_response,apply_control_and_observe,emit_project_report,record_tool_result,finish_cancelled_turn,rebuild_round_prompt,execute_tool_call,refresh_tool_context,compute_scope_candidates,prepare_task_run_for_tool,preflight_tool_acquisition_allowed,resolve_execution_grant_and_decision,compute_execution_failure,prepare_tool_batch,await_round_gate,run_provider_round,terminal_tool_requirement_error,park_clarification,finish_final_answer,tool_call_receipt_identity,tool_result_references,model_tool_result_message,proposal_result_projection,settle_completed_turn | consumer=OptionalTaskRunStatus | does_not=executeToolOutsideFreshGrantOrTreatPromptIntentProjectReportOrWorkspaceOperationPlanAsAuthorization
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
    task_run_id: Option<String>,
    workspace_id_for_turn: String,
    provider: String,
    model: String,
    runtime_profile: String,
    prompt_package: ag::context::PromptPackageV2,
    agent_context_facts: crate::types::AgentContextFacts,
    current_doc_prompt_coverage: Option<&'static str>,
    messages: Vec<serde_json::Value>,
    tools: serde_json::Value,
    capability_factors: ag::tool_registry::CapabilityFactorsV2,
    read_ctx: ag::blocks::AgentReadContext,
    active_doc_path: Option<String>,
    explicit_edit_target_set: ag::target_set::ExplicitEditTargetSet,
    ws_root: Option<String>,
    current_doc_requires_tool_read: bool,
    intent_result: ag::context::IntentResultV2,
    continuation_messages: Vec<serde_json::Value>,
    history: Vec<ag::context::HistTurn>,
    raw_prompt: String,
    mention_block: Option<String>,
    reference_envelopes: Vec<ag::context::PromptDataEnvelopeV1>,
    active_snapshot: Option<ag::blocks::AgentEditableSnapshot>,
    on_context_request: Option<tauri::ipc::Channel<super::turn_runtime::AgentContextRequest>>,
) -> Result<(), IpcError> {
    let event_emitter = TurnEventEmitter::new(&state, &on_event, &turn_id);
    let transport_protocol = event_emitter.frozen_transport_protocol()?.ok_or_else(|| {
        IpcError::new(
            "AG_MODEL_PROTOCOL_VERSION_UNSUPPORTED",
            "AgentTurn 尚未冻结模型工具传输协议",
            false,
            false,
        )
    })?;
    let model_capacity = ag::prompt_runtime::model_capacity(&provider, &model);
    let execution_policy = event_emitter.execution_policy()?;
    let workspace_plan_completed = event_emitter.has_workspace_plan_receipt()?;
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
    let proposed_diff = false;
    let edit_blocked_reported = false;

    // SM-AG-02 工具循环：注册/权限校验与本回合冻结的执行策略。
    // 循环经状态机驱动 + 广播 sm:SM-AG-02（ModelStreaming→ToolValidating→ToolExecuting→ResultRecording→
    // Recorded 后回归 SM-AG-01 并重新装配；FinalAnswer→LoopCompleted；达上限→LoopAborted。
    // 工具卡数据仍由 Channel 投影（互不替代）。
    use ag::tool::ToolLoopState as TlSt;
    let current_doc_read_confirmed = !current_doc_requires_tool_read;
    // AG-CTX-01：本轮是否调用过任何工具（用于区分"已读未提案"与"零工具调用"的失败文案）。
    let any_tool_called = false;
    let (previous_model_calls, _) = event_emitter.protocol_attempt_counts()?;
    let (protocol_correction_rounds, parameter_correction_rounds) =
        event_emitter.correction_attempt_breakdown()?;
    let failed_read_signatures = BTreeSet::new();
    // Rejected tool IDs are still used by this AgentTurn. Do not make malformed
    // arguments model-visible merely to retain duplicate-ID protection.
    let seen_tool_call_ids = continuation_messages
        .iter()
        .filter_map(|message| {
            message
                .get("tool_calls")
                .and_then(serde_json::Value::as_array)
        })
        .flatten()
        .filter_map(|call| call.get("id").and_then(serde_json::Value::as_str))
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    // 完整 Prompt 审计只保留本次 AgentTurn 最后一次实际发给模型的请求；回合退出时由 Drop
    // 统一输出一次，避免工具续轮把前序 messages/tools 反复打印。
    let final_prompt_audit = prompt_audit_enabled().then(FinalPromptAuditLog::default);
    let tool_loop_state = TlSt::ModelStreaming;
    let capability_use_tracker = ag::tool_registry::CapabilityGrantUseTrackerV2::default();
    let (remaining_tool_calls, continuation_rounds) = continuation_execution_budget(
        &continuation_messages,
        execution_policy.max_tool_calls,
        execution_policy.max_model_calls,
    );
    let remaining_rounds = continuation_rounds.min(
        execution_policy
            .max_model_calls
            .saturating_sub(previous_model_calls),
    );
    let turn_state = ag::turn::TurnState::Streaming;
    let round_context = RoundContext {
        state,
        app,
        cancelled,
        terminal_latch,
        turn_id: turn_id.clone(),
        session_id,
        workspace_id_for_turn,
        provider,
        model,
        runtime_profile,
        active_doc_path,
        explicit_edit_target_set,
        ws_root,
        history,
        raw_prompt,
        mention_block,
        reference_envelopes,
        on_context_request,
        transport_protocol,
        model_capacity,
        execution_policy,
        workspace_plan_completed,
        previous_model_calls,
        remaining_rounds,
    };
    let round_state = RoundState {
        messages,
        tools,
        capability_factors,
        read_ctx,
        active_snapshot,
        intent_result,
        continuation_messages,
        task_run_id,
        remaining_tool_calls,
        protocol_correction_rounds,
        parameter_correction_rounds,
        failed_read_signatures,
        seen_tool_call_ids,
        tool_loop_state,
        turn_state,
        current_doc_prompt_coverage,
        current_doc_requires_tool_read,
        current_doc_read_confirmed,
        any_tool_called,
        proposed_diff,
        edit_blocked_reported,
        final_prompt_audit,
        capability_use_tracker,
        prompt_package,
        agent_context_facts,
    };
    run_round_loop(&event_emitter, round_context, round_state).await
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
        assert!(!correction_final_only(
            &[
                serde_json::json!({"role":"assistant","tool_calls":[{"function":{"name":"workspace_glob"}}]})
            ],
            ("w", "s", "t")
        ));
        assert!(correction_final_only(
            &[
                serde_json::json!({"role":"assistant","tool_calls":[{"function":{"name":"plan_workspace_operation"}}]})
            ],
            ("w", "s", "t")
        ));
        assert!(correction_final_only(
            &[correction_instruction(true)],
            ("w", "s", "t")
        ));
    }

    // covers: BR-AG-STATE-001, BR-AG-STATE-002
    #[test]
    fn text_tool_action_correction_is_specific_and_never_grants_execution() {
        let instruction = response_correction_instruction(false, "text_tool_action");
        let content = instruction["content"].as_str().unwrap();
        assert_eq!(instruction["role"], "system");
        assert_eq!(instruction["layer"], "protocol_correction");
        assert!(content.contains("没有原生工具调用"));
        assert!(content.contains("binder_tool_batch"));
        assert!(content.contains("该文本未触发工具"));
        assert!(!content.contains("</｜｜DSML｜｜"));
        let frozen = response_correction_instruction(true, "text_tool_action");
        assert_eq!(frozen["layer"], "protocol_correction_final_only");
        assert!(frozen["content"]
            .as_str()
            .unwrap()
            .contains("只能以合法final/clarify"));
        assert!(!frozen["content"]
            .as_str()
            .unwrap()
            .contains("下一响应必须调用"));
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

    // covers: BR-AG-STATE-002, BR-SYS-STATE-007
    #[test]
    fn correction_uses_owned_pre_execution_receipt_not_transport_name() {
        let call = serde_json::json!({"id":"batch-1","function":{"name":ag::model_output::TOOL_BATCH_NAME}});
        let assistant = serde_json::json!({"role":"assistant","tool_calls":[call]});
        let receipt = serde_json::json!({"version":"tool-call-receipt-v1","workspaceId":"w","sessionId":"s","turnId":"t",
            "toolCallId":"batch-1","toolName":ag::model_output::TOOL_BATCH_NAME,"capabilityId":"unregistered","grantId":"g","attempt":1,
            "executionStatus":"rejected","completionStatus":"blocked","effectStatus":"not_started","references":[],"evidenceIds":[],
            "failure":{"code":"AG_MODEL_CONTROL_INVALID","retriable":true,"reauthRequired":false},"nextAction":"respond_blocked"});
        let reply = |receipt: &serde_json::Value| serde_json::json!({"role":"tool","tool_call_id":"batch-1","content":serde_json::json!({"executionReceipt":receipt}).to_string()});
        assert!(correction_final_only(&[assistant.clone()], ("w", "s", "t")));
        assert!(!correction_final_only(
            &[assistant.clone(), reply(&receipt)],
            ("w", "s", "t")
        ));
        for (field, value) in [
            ("workspaceId", serde_json::json!("other")),
            ("sessionId", serde_json::json!("other")),
            ("turnId", serde_json::json!("other")),
            ("toolCallId", serde_json::json!("other")),
            ("toolName", serde_json::json!("read_doc")),
            ("effectStatus", serde_json::json!("unknown")),
            ("effectStatus", serde_json::json!("resource_executed")),
            ("executionStatus", serde_json::json!("succeeded")),
            ("completionStatus", serde_json::json!("waiting")),
            (
                "references",
                serde_json::json!([{"kind":"pending_diff","id":"d"}]),
            ),
            ("evidenceIds", serde_json::json!(["e"])),
            ("failure", serde_json::Value::Null),
        ] {
            let mut changed = receipt.clone();
            changed[field] = value;
            assert!(
                correction_final_only(&[assistant.clone(), reply(&changed)], ("w", "s", "t")),
                "unsafe receipt field {field}"
            );
        }
        assert!(correction_final_only(
            &[assistant.clone(), reply(&receipt), reply(&receipt)],
            ("w", "s", "t")
        ));
        let non_read = serde_json::json!({"role":"assistant","tool_calls":[{"id":"issued","function":{"name":"propose_edit"}}]});
        assert!(correction_final_only(
            &[non_read, assistant, reply(&receipt)],
            ("w", "s", "t")
        ));
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
        checkpoint_after_read_clarification(&mut read, ("w", "s", "t"));
        assert!(!super::super::turn_prompt::final_only_continuation(&read));
        assert_eq!(read.len(), 1);
        let mut non_read = guards.clone();
        non_read.push(serde_json::json!({"role":"assistant","tool_calls":[{"function":{"name":"propose_decision"}}]}));
        checkpoint_after_read_clarification(&mut non_read, ("w", "s", "t"));
        assert!(super::super::turn_prompt::final_only_continuation(
            &non_read
        ));
        assert_eq!(non_read.len(), 3);
        let mut only_protocol = vec![correction_instruction(true)];
        checkpoint_after_read_clarification(&mut only_protocol, ("w", "s", "t"));
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

    // covers: BR-AG-DATA-011
    #[test]
    fn tool_receipt_projection_binds_grant_identity_and_actual_products() {
        let identity = tool_call_receipt_identity(
            "ws",
            "session",
            "turn",
            "call-1",
            "propose_edit",
            "grant-1",
        );
        assert_eq!(identity.workspace_id, "ws");
        assert_eq!(identity.session_id, "session");
        assert_eq!(identity.turn_id, "turn");
        assert_eq!(identity.tool_call_id, "call-1");
        assert_eq!(identity.tool_name, "propose_edit");
        assert_eq!(identity.grant_id, "grant-1");
        assert_eq!(identity.attempt, 1);

        let references = tool_result_references(
            vec!["diff-1".to_string(), "diff-2".to_string()],
            Some("decision-1"),
        );
        assert_eq!(references.len(), 3);
        assert_eq!(
            references[0].kind,
            crate::types::ToolCallReceiptReferenceKind::PendingDiff
        );
        assert_eq!(references[0].id, "diff-1");
        assert_eq!(
            references[2].kind,
            crate::types::ToolCallReceiptReferenceKind::DecisionRequest
        );
        assert_eq!(references[2].id, "decision-1");

        let receipt = ag::tool_exec::tool_call_receipt(
            identity,
            true,
            true,
            None,
            references,
            vec!["ev-1".to_string()],
        );
        let message =
            model_tool_result_message("call-1", "{\"ok\":true}", &["ev-1".to_string()], &receipt);
        assert_eq!(message["role"], "tool");
        assert_eq!(message["tool_call_id"], "call-1");
        let content = serde_json::from_str::<serde_json::Value>(
            message["content"]
                .as_str()
                .expect("tool content is a string"),
        )
        .expect("tool content is JSON");
        assert_eq!(content["toolResult"]["ok"], true);
        assert_eq!(content["evidenceIds"][0], "ev-1");
        assert_eq!(content["executionReceipt"]["toolCallId"], "call-1");
        assert_eq!(content["executionReceipt"]["grantId"], "grant-1");
        assert_eq!(content["executionReceipt"]["evidenceIds"][0], "ev-1");
    }

    fn test_projection(diff_id: &str) -> crate::types::DiffProjection {
        crate::types::DiffProjection {
            diff_id: diff_id.into(),
            rel_path: "notes/a.md".into(),
            patch_kind: "block_ops".into(),
            block_id: None,
            block_index: None,
            before_text: String::new(),
            after_text: String::new(),
            location_label: "…".into(),
            draft_mode: "current_editor".into(),
            snapshot_source: "active_editor".into(),
            base_session_version: Some(1),
            base_content_hash: Some("hash".into()),
            style_ops: None,
            block_ops: None,
            table_ops: None,
            image_ops: None,
            proposal_batch_id: "batch-1".into(),
            group_index: 1,
            group_count: 2,
            preview_checkpoint_required: true,
            cross_doc: None,
        }
    }

    // covers: BR-AG-DATA-012
    #[test]
    fn proposal_result_projection_derives_card_count_and_keeps_pending_semantics() {
        let projections = vec![test_projection("diff-1"), test_projection("diff-2")];
        let result = proposal_result_projection("batch-1", "notes/a.md", &projections, false);
        let summary = result["summary"].as_str().expect("summary string");
        assert!(summary.contains("2 张"), "{summary}");
        assert!(summary.contains("待确认"), "{summary}");
        assert_eq!(result["proposal_batch_id"], "batch-1");
        assert_eq!(result["diff_ids"], serde_json::json!(["diff-1", "diff-2"]));
        assert_eq!(result["rel_path"], "notes/a.md");
        assert_eq!(
            result["projections"].as_array().map(Vec::len),
            Some(projections.len())
        );
        assert_eq!(result["proposal_status"], "pending_user_review");
        assert_eq!(result["editor_preview_status"], "not_reported");
        assert_eq!(result["disk_persistence_status"], "not_saved");
        assert_eq!(result["preview_checkpoint_required"], false);
    }

    // covers: BR-AG-STATE-005
    #[test]
    fn completion_settlement_marks_partial_blocked_and_budget_closing_as_failed() {
        let none = ag::context::SideEffectIntentV2::None;
        let proposal = ag::context::SideEffectIntentV2::ContentProposal;
        let completion = |status: &str| ag::model_output::ModelCompletion {
            status: status.to_string(),
            reason: "test".to_string(),
        };
        assert!(completion_settles_as_failed(true, None, &none));
        assert!(completion_settles_as_failed(
            false,
            Some(&completion("partial")),
            &none
        ));
        assert!(completion_settles_as_failed(
            false,
            Some(&completion("blocked")),
            &none
        ));
        assert!(completion_settles_as_failed(
            false,
            Some(&completion("report_only")),
            &proposal
        ));
        assert!(!completion_settles_as_failed(
            false,
            Some(&completion("report_only")),
            &none
        ));
        assert!(!completion_settles_as_failed(
            false,
            Some(&completion("completed")),
            &proposal
        ));
        assert!(!completion_settles_as_failed(false, None, &proposal));

        assert_eq!(
            completed_turn_stop_reason(true, None, None, "stop"),
            "budget_exhausted"
        );
        assert_eq!(
            completed_turn_stop_reason(false, None, Some("warning"), "stop"),
            "answer_incomplete"
        );
        let partial = completion("partial");
        assert_eq!(
            completed_turn_stop_reason(false, Some(&partial), None, "stop"),
            "partial"
        );
        assert_eq!(
            completed_turn_stop_reason(false, None, None, "stop"),
            "stop"
        );
    }

    // covers: BR-AG-DATA-013
    #[test]
    fn round_response_validation_rejects_budget_closing_tools_and_defers_failed_rounds() {
        let decoder = ag::model_output::ModelOutputDecoder::default();
        let failed = ent::gateway::RoundOutcome::Failed(IpcError::new(
            "AG_GATEWAY_TEST",
            "test transport failure",
            false,
            false,
        ));
        assert!(
            validate_round_response(&failed, &decoder, false, false, 0, false, &[])
                .expect("failed round has no control")
                .is_none()
        );

        let tools = ent::gateway::RoundOutcome::ToolCalls(Vec::new());
        let error = validate_round_response(&tools, &decoder, true, false, 0, false, &[])
            .expect_err("budget closing must reject tool calls");
        assert_eq!(error.code, "AG_MODEL_CONTROL_INVALID");

        let final_only = vec![serde_json::json!({
            "role": "system",
            "layer": "read_failure_final_only",
        })];
        let error = validate_round_response(&tools, &decoder, false, false, 0, false, &final_only)
            .expect_err("final-only continuation must reject tool calls");
        assert_eq!(error.code, "AG_MODEL_CONTROL_INVALID");
    }

    // covers: BR-AG-STATE-002
    // 同批多 ToolCall 必须经 nextToolCall 转移；直接赋值状态只允许出现在每次激活起点。
    #[test]
    fn multi_tool_call_batch_uses_transition_instead_of_direct_state_reset() {
        let source = include_str!("turn_tool_loop.rs");
        let production = &source[..source
            .rfind("#[cfg(test)]")
            .expect("test module starts after production code")];
        let next = production
            .find("TlEv::NextToolCall")
            .expect("同批下一 ToolCall 必须有显式转移");
        let branch = production[..next]
            .rfind("let has_more_calls")
            .expect("NextToolCall 必须位于同批剩余调用判断之后");
        assert!(branch < next);
        assert_eq!(
            production
                .matches("tool_loop_state = TlSt::ModelStreaming")
                .count(),
            2,
            "唯一允许的直接赋值是初始状态与每次 provider 激活起点"
        );
    }

    #[test]
    fn execution_failure_priority_and_guard_projection_are_stable() {
        let control = IpcError::new("AG_CONTROL", "control failure", true, false);
        let refresh = IpcError::new("AG_CONTEXT", "context failure", true, false);
        let scope: Result<Option<Vec<String>>, IpcError> =
            Err(IpcError::new("WS_SCOPE", "scope failure", false, false));
        let task = IpcError::new("AG_TASK", "task failure", true, false);

        let (failure, guard) = compute_execution_failure(
            Some(&control),
            true,
            Some(&ag::tool::ToolDecision::InvalidArguments {
                field: "operations",
                constraint: "required",
                message: "missing".into(),
            }),
            "read_doc",
            &serde_json::json!({}),
            true,
            Some(&refresh),
            &scope,
            Some(&task),
        );
        assert_eq!(failure.expect("control error wins").code, "AG_CONTROL");
        assert_eq!(guard.as_deref(), Some("[AG_CONTEXT] context failure"));

        let (failure, guard) = compute_execution_failure(
            None,
            false,
            None,
            "read_doc",
            &serde_json::json!({}),
            false,
            None,
            &scope,
            Some(&task),
        );
        assert_eq!(
            failure.expect("scope error wins over task").code,
            "WS_SCOPE"
        );
        assert_eq!(guard.as_deref(), Some("scope failure"));
    }

    #[test]
    fn terminal_tool_requirement_error_matches_existing_terminal_semantics() {
        let mut intent = ag::context::IntentResultV2::unresolved();
        intent.side_effect_intent = ag::context::SideEffectIntentV2::ContentProposal;
        assert!(
            terminal_tool_requirement_error(true, false, false, &intent, false, false, false)
                .is_none()
        );
        assert!(
            terminal_tool_requirement_error(false, true, false, &intent, false, false, false)
                .is_none()
        );
        assert_eq!(
            terminal_tool_requirement_error(false, false, false, &intent, false, false, false)
                .expect("missing propose_edit")
                .code,
            "AG_EDIT_TOOL_REQUIRED"
        );
        assert!(
            terminal_tool_requirement_error(false, false, false, &intent, true, false, false)
                .is_none()
        );
        intent.side_effect_intent = ag::context::SideEffectIntentV2::StructurePlan;
        assert!(
            terminal_tool_requirement_error(false, false, true, &intent, false, false, false)
                .is_none()
        );
        assert_eq!(
            terminal_tool_requirement_error(false, false, false, &intent, false, false, false)
                .expect("missing plan")
                .code,
            "AG_WORKSPACE_PLAN_TOOL_REQUIRED"
        );
        intent.side_effect_intent = ag::context::SideEffectIntentV2::None;
        assert!(
            terminal_tool_requirement_error(false, false, false, &intent, false, false, false)
                .is_none()
        );
    }
}
