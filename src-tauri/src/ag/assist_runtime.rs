//! AG-ASSIST command、终态去重与状态事件编排。

use super::assist_prompt::{build_assist_messages, AssistPromptInput};
use crate::ipc::IpcError;
use crate::{ag, ent, sm_events, AppState};
use tauri::State;

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

/// 辅助产出（A-AG-I-P-01 `ag.request_assist`）：候选或空。
#[derive(serde::Serialize)]
pub(crate) struct AssistResult {
    /// 前端 requestId；用于 latest-wins，旧响应由前端丢弃。
    request_id: String,
    /// 候选文本；门禁不可用 / 失败 / 空产出 → None（前端静默不提供，不打断编辑）。
    candidate: Option<String>,
    /// 辅助续写候选列表；autocomplete 期望最多 3 个。
    candidates: Vec<String>,
    /// 内部诊断原因；前端不直接暴露给普通用户。
    failure_reason: Option<String>,
}

/**
 * @GOV
 * codes:    BR-AG-STATE-003-EFFECT-AG-AG-ASSIST-010
 * type:     EFFECT
 * chain:    AG-ASSIST
 * rules:    BR-AG-STATE-003
 * boundary: in=AssistTerminalEvent(acceptOrDismissOrAnchorLost) | out=DeduplicatedAssistTerminalState | does_not=callProviderOrToolOrWriteEditorOrCreateSecondTerminal | consumer=BR-AG-STATE-003
 */
#[tauri::command]
pub(crate) fn ag_finish_assist(
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
 * codes:    BR-AG-STATE-003-EFFECT-AG-AG-ASSIST-001, BR-AG-VERIFY-001-EFFECT-AG-AG-ASSIST-001
 * type:     EFFECT
 * chain:    AG-ASSIST
 * rules:    BR-AG-STATE-003, BR-AG-VERIFY-001
 * boundary: in=AssistRequest | out=AssistCandidate(gateGatedSilentOnDenyOrFailNeverInterruptsEditing) | delegate=ent::gateway::check_gate | does_not=mutateDocumentLogicalState | consumer=BR-AG-STATE-003,BR-AG-VERIFY-001
 */
/// 命令 `ag_request_assist`（A-AG-I-P-01 request_assist · SM-AG-03 · 独立 async，不走 dispatch 信封）：
/// 前端经 SM-AG-03 节流（Idle→Throttling）后调本命令承接 throttlePass 之后。**门禁前置**（BR-AG-VERIFY-001）：
/// 门禁不可用 → ThrottlePass{false}→Idle，静默返回空候选（不报错、不打断编辑）；可用 → Requesting → 经网关
/// 单次产出 → CandidateOk→Presenting（回候选）/ RequestFail→Idle（静默）。采纳=用户编辑（前端经 ED，非本命令）。
/// 节流参数/成本策略延实现期标定（AG-03，A-AG-M-S-01 §4）。
#[tauri::command]
pub(crate) async fn ag_request_assist(
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
    let model = model.unwrap_or_else(|| ent::gateway::default_model(&provider));
    let messages = build_assist_messages(AssistPromptInput {
        kind: &kind,
        doc_path: doc_path.as_deref(),
        load_seq,
        block_id: block_id.as_deref(),
        start,
        end,
        instruction: instruction.as_deref(),
        selection: selection.as_deref(),
        prefix: prefix.as_deref(),
        before: before.as_deref(),
        after: after.as_deref(),
        base_text: base_text.as_deref(),
    });

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
