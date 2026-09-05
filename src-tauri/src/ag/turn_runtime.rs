//! AgentTurn 请求装配、模型轮次与受控工具循环。

use crate::ag::prompt_runtime::PROMPT_TOKEN_BUDGET;
use crate::ipc::{AgTurnEvent, AgTurnEventPayload, IpcError};
use crate::types::{AgentContextFactsV1, ReferencePayload, AGENT_CONTEXT_FACTS_VERSION_V1};
use crate::{ag, ent, workspace, AppState};
use std::collections::BTreeSet;
use std::sync::atomic::Ordering;

use super::edit_support::is_active_editor_snapshot_source;
use super::reference_runtime::BlockRef;
use super::task_runtime::settle_optional_task_run;
use super::turn_control::{
    bind_agent_turn_edit_target_set, register_active_agent_turn,
    validate_agent_turn_decision_resume, AgentTurnResumeKind,
};
use super::turn_events::{
    emit_ag01, emit_ag_turn_cancelled_from, emit_ag_turn_denied, emit_ag_turn_error_from,
    settle_and_emit_ag_turn_error_from, sm_error, TurnEventEmitter,
};
use super::turn_prompt::{prepare_turn_prompt, PreparedTurnPrompt};
use super::turn_reference::{build_turn_reference_runtime, TurnReferenceRuntime};
use super::turn_tool_loop::run_turn_tool_loop;

/**
 * @GOV
 * codes:    BR-SYS-STATE-007-GUARD-AG-AG-TURN-037
 * type:     GUARD
 * chain:    AG-TURN, AG-TOOL
 * rules:    BR-SYS-STATE-007
 * term_ref: TERM-AG-013
 * boundary: in=OptionalRuntimeProfile | out=ValidatedAgentEditRuntimeProfileOrStablePreGateRejection | does_not=applyCompatibilityDefaultOrEmitExecutableEvent
 */
fn require_runtime_profile(runtime_profile: Option<String>) -> Result<String, IpcError> {
    let runtime_profile = runtime_profile.ok_or_else(|| {
        IpcError::new(
            "AG_RUNTIME_PROFILE_REQUIRED",
            "缺少 Agent RuntimeProfile",
            false,
            false,
        )
    })?;
    ag::turn::ensure_runtime_profile(&runtime_profile)?;
    Ok(runtime_profile)
}

/**
 * @GOV
 * codes:    BR-AG-DATA-008-GUARD-AG-AG-TURN-042, BR-ED-DATA-008-GUARD-AG-AG-TURN-042, BR-SYS-DATA-008-GUARD-AG-AG-TURN-042
 * type:     GUARD
 * chain:    AG-TURN
 * rules:    BR-AG-DATA-008, BR-ED-DATA-008, BR-SYS-DATA-008
 * term_ref: TERM-AG-020
 * boundary: in=AgentContextFactsV1WorkspaceTurnOpenDocSnapshotsAndExplicitReferences | out=VerifiedSameTurnLogicalStateFactsOrStablePreGateRejection | does_not=acceptUnknownVersionCrossWorkspaceFactsOrGrantReadWriteCapability
 */
fn validate_agent_context_facts(
    facts: Option<&AgentContextFactsV1>,
    workspace_id: &str,
    workspace_root: &std::path::Path,
    turn_id: &str,
    session_id: &str,
    active_doc_path: Option<&str>,
    open_doc_snapshots: &[ag::blocks::OpenDocSnapshot],
    references: &[ReferencePayload],
    block_refs: &[BlockRef],
) -> Result<(), IpcError> {
    let facts = facts.ok_or_else(|| {
        IpcError::new(
            "AG_CONTEXT_FACTS_REQUIRED",
            "缺少版本化 AgentContextFacts，已在 Gate 前拒绝回合",
            false,
            false,
        )
    })?;
    if facts.version != AGENT_CONTEXT_FACTS_VERSION_V1 {
        return Err(IpcError::new(
            "AG_CONTEXT_FACTS_VERSION_UNSUPPORTED",
            "AgentContextFacts 版本未知，已在 Gate 前拒绝回合",
            false,
            false,
        ));
    }
    if facts.workspace_id != workspace_id || facts.turn_id != turn_id {
        return Err(IpcError::new(
            "AG_CONTEXT_FACTS_IDENTITY_MISMATCH",
            "AgentContextFacts 不属于当前 workspace/AgentTurn",
            false,
            false,
        ));
    }
    if facts.open_docs.len() != open_doc_snapshots.len() {
        return Err(IpcError::new(
            "AG_CONTEXT_FACTS_OPEN_DOCS_MISMATCH",
            "AgentContextFacts.openDocs 与 OpenDocSnapshot 集合不一致",
            true,
            false,
        ));
    }
    let mut seen = BTreeSet::new();
    for fact in &facts.open_docs {
        crate::workspace::resolve_in_workspace(workspace_root, &fact.rel_path)?;
        if !seen.insert(fact.rel_path.as_str()) {
            return Err(IpcError::new(
                "AG_CONTEXT_FACTS_DUPLICATE_DOC",
                "AgentContextFacts.openDocs 含重复路径",
                false,
                false,
            ));
        }
        let snapshot = open_doc_snapshots
            .iter()
            .find(|snapshot| snapshot.rel_path == fact.rel_path)
            .ok_or_else(|| {
                IpcError::new(
                    "AG_CONTEXT_FACTS_OPEN_DOCS_MISMATCH",
                    "AgentContextFacts 声明了不存在的打开文档事实",
                    true,
                    false,
                )
            })?;
        let baseline = snapshot.baseline.map(|value| value.to_string());
        if fact.source != snapshot.source
            || fact.edit_format != snapshot.edit_format
            || fact.content_format != snapshot.content_format
            || fact.dirty != snapshot.dirty
            || fact.baseline != baseline
            || fact.session_version != snapshot.session_version
            || fact.content_hash != snapshot.content_hash
        {
            return Err(IpcError::new(
                "AG_CONTEXT_FACTS_LOGICAL_STATE_MISMATCH",
                "AgentContextFacts 与同轮 OpenDocSnapshot LogicalState 身份不一致",
                true,
                false,
            ));
        }
    }
    match (active_doc_path, facts.active_doc.as_ref()) {
        (None, None) => {}
        (Some(active_path), Some(active_fact))
            if active_fact.rel_path == active_path
                && facts.open_docs.iter().any(|fact| fact == active_fact)
                && is_active_editor_snapshot_source(&active_fact.source) => {}
        _ => {
            return Err(IpcError::new(
                "AG_CONTEXT_FACTS_ACTIVE_DOC_MISMATCH",
                "AgentContextFacts.activeDoc 与当前活动编辑器不一致",
                true,
                false,
            ));
        }
    }
    let mut expected_selections = references
        .iter()
        .filter_map(|reference| match reference {
            ReferencePayload::Selection {
                rel_path,
                block_id,
                start,
                end,
                ..
            } => Some((rel_path.as_str(), block_id.as_str(), *start, *end)),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    expected_selections.extend(block_refs.iter().map(|reference| {
        (
            reference.rel_path.as_str(),
            reference.block_id.as_str(),
            reference.start,
            reference.end,
        )
    }));
    let actual_selections = facts
        .selections
        .iter()
        .map(|selection| {
            crate::workspace::resolve_in_workspace(workspace_root, &selection.rel_path)?;
            Ok((
                selection.rel_path.as_str(),
                selection.block_id.as_str(),
                selection.start,
                selection.end,
            ))
        })
        .collect::<Result<BTreeSet<_>, IpcError>>()?;
    if actual_selections != expected_selections {
        return Err(IpcError::new(
            "AG_CONTEXT_FACTS_SELECTION_MISMATCH",
            "AgentContextFacts.selections 只能来自本轮显式 SelectionRef",
            false,
            false,
        ));
    }
    for proposal in &facts.proposal_facts {
        crate::workspace::resolve_in_workspace(workspace_root, &proposal.rel_path)?;
        if proposal.diff_id.trim().is_empty()
            || proposal.session_id != session_id
            || !matches!(
                proposal.state.as_str(),
                "pending"
                    | "validating"
                    | "applying"
                    | "accepted"
                    | "rejected"
                    | "expired"
                    | "conflict"
                    | "error"
            )
            || !matches!(
                proposal.effect_status.as_deref(),
                None | Some("not_applicable")
                    | Some("awaiting_save")
                    | Some("saved")
                    | Some("discarded_on_abnormal_exit")
            )
            || (proposal.saved && proposal.effect_status.as_deref() == Some("awaiting_save"))
        {
            return Err(IpcError::new(
                "AG_CONTEXT_FACTS_PROPOSAL_INVALID",
                "AgentContextFacts.proposalFacts 含无效身份或状态",
                false,
                false,
            ));
        }
    }
    Ok(())
}

/**
 * @GOV
 * codes:    BR-AG-STATE-001-API-AG-AG-TURN-018, BR-AG-STATE-004-API-AG-AG-TURN-018, BR-AG-DATA-008-API-AG-AG-TURN-018, BR-ED-DATA-008-API-AG-AG-TURN-018, BR-SYS-STATE-007-API-AG-AG-TURN-018, BR-SYS-DATA-008-API-AG-AG-TURN-018
 * type:     API
 * chain:    AG-TURN, AG-TOOL
 * rules:    BR-AG-STATE-001, BR-AG-STATE-004, BR-AG-DATA-008, BR-ED-DATA-008, BR-SYS-STATE-007, BR-SYS-DATA-008
 * term_ref: TERM-AG-001, TERM-AG-012, TERM-AG-013, TERM-AG-014, TERM-AG-015, TERM-AG-016, TERM-AG-020
 * boundary: in=ValidatedRuntimeProfileAndAgentTurnRequestOrSameTurnResumeAndAgentReadContext | out=AdaptiveModelToolEventStreamOrNonTerminalWaitAndOptionalTaskRunProjection | effect=IPC-ag_start_turn | delegate=require_runtime_profile,prepare_turn_prompt,ent::gateway::check_gate,ag::turn::transition,run_turn_tool_loop,register_active_agent_turn | does_not=classifyIntentByKeywordsDefaultMissingRuntimeProfileCreateNewTurnForResumeBypassCurrentGateOutcomeOrWriteDocumentContentOutsidePendingDiff
 */
/// PI-AG-WIRE-01 · `ag_start_turn`（独立 async tauri command，不走 dispatch 信封）。
///
/// 驱动 SM-AG-01 门禁前置，经 ENT 网关流式调用并执行受控工具循环。
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub(crate) async fn ag_start_turn(
    turn_id: String,
    session_id: String,
    runtime_profile: Option<String>,
    prompt: String,
    provider: Option<String>,
    model: Option<String>,
    active_doc_path: Option<String>,
    active_doc_content: Option<String>,
    active_doc_format: Option<String>,
    open_doc_snapshots: Vec<ag::blocks::OpenDocSnapshot>,
    agent_context_facts: Option<AgentContextFactsV1>,
    history: Vec<ag::context::HistTurn>,
    mentions: Vec<String>,
    block_refs: Vec<BlockRef>,
    references: Vec<ReferencePayload>,
    continuation_task_run_id: Option<String>,
    continuation_decision_request_id: Option<String>,
    continuation_preview_diff_id: Option<String>,
    clarification_resume: Option<bool>,
    decision_resume: Option<bool>,
    on_event: tauri::ipc::Channel<AgTurnEvent>,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), IpcError> {
    use ag::turn::{ensure_gate_passed, transition, GateState, TurnEvent, TurnState};

    let runtime_profile = require_runtime_profile(runtime_profile)?;
    let clarification_resume = clarification_resume.unwrap_or(false);
    let decision_resume = decision_resume.unwrap_or(false);
    if clarification_resume && decision_resume {
        return Err(IpcError::new(
            "AG_TURN_RESUME_CONFLICT",
            "同一次调用不能同时恢复澄清等待和裁决等待",
            false,
            false,
        ));
    }
    let resume_kind = if clarification_resume {
        AgentTurnResumeKind::Clarification
    } else if decision_resume {
        AgentTurnResumeKind::Decision
    } else {
        AgentTurnResumeKind::New
    };

    if prompt.trim().is_empty() && references.is_empty() {
        return Err(IpcError::new(
            "AG_PROMPT_REQUIRED",
            "缺少 prompt",
            false,
            false,
        ));
    }

    let (workspace_id_for_turn, workspace_root_path) = {
        let conn = state
            .db
            .lock()
            .map_err(|_| IpcError::new("WS_DB_BUSY", "数据库连接繁忙", true, false))?;
        let ws = workspace::lifecycle::active_workspace(&conn)
            .map_err(|e| IpcError::new("WS_ACTIVE_FAILED", &e.to_string(), true, false))?
            .ok_or_else(|| IpcError::new("WS_NO_ACTIVE", "无活动 workspace", false, false))?;
        (ws.workspace_id.clone(), ws.root_path.clone())
    };

    validate_agent_context_facts(
        agent_context_facts.as_ref(),
        &workspace_id_for_turn,
        std::path::Path::new(&workspace_root_path),
        &turn_id,
        &session_id,
        active_doc_path.as_deref(),
        &open_doc_snapshots,
        &references,
        &block_refs,
    )?;
    let agent_context_facts = agent_context_facts
        .expect("validated AgentContextFacts must be present after fail-closed guard");

    if clarification_resume
        && (continuation_task_run_id.is_some()
            || continuation_decision_request_id.is_some()
            || continuation_preview_diff_id.is_some())
    {
        return Err(IpcError::new(
            "AG_CLARIFICATION_CONTINUATION_CONFLICT",
            "澄清恢复不能与任务裁决或预览续片段混用",
            false,
            false,
        ));
    }
    if decision_resume
        && (continuation_task_run_id.is_none()
            || continuation_decision_request_id.is_none()
            || continuation_preview_diff_id.is_some())
    {
        return Err(IpcError::new(
            "AG_DECISION_CONTINUATION_INVALID",
            "裁决恢复必须且只能携带原 TaskRun 与已解决的 DecisionRequest 身份",
            false,
            false,
        ));
    }
    if !decision_resume && continuation_decision_request_id.is_some() {
        return Err(IpcError::new(
            "AG_DECISION_RESUME_REQUIRED",
            "已解决裁决只能用于恢复原 AgentTurn，不能创建替代回合",
            false,
            false,
        ));
    }
    if decision_resume {
        validate_agent_turn_decision_resume(
            &state,
            &session_id,
            &turn_id,
            continuation_decision_request_id
                .as_deref()
                .expect("decision resume identity was validated as present"),
        )?;
    }

    let (
        _active_turn_lease,
        cancelled,
        clarification_task_run_id,
        registered_edit_target_set,
        resumed_runtime_checkpoint,
    ) = register_active_agent_turn(
        &state,
        &workspace_id_for_turn,
        &session_id,
        &turn_id,
        resume_kind,
    )?;
    let terminal_latch = ag::turn::TerminalLatch::default();

    // 注册成功即执行 send / clarificationProvided；后续持久化与上下文构建都是
    // Assembling 的动作，失败只能由 assembleFail 收束为 TurnError。
    let s = match resume_kind {
        AgentTurnResumeKind::Clarification => transition(
            TurnState::AwaitingClarification,
            TurnEvent::ClarificationProvided,
        ),
        AgentTurnResumeKind::Decision => {
            transition(TurnState::AwaitingDecision, TurnEvent::DecisionResolved)
        }
        AgentTurnResumeKind::New => transition(TurnState::Idle, TurnEvent::Send),
    };
    let s = s.ok_or_else(sm_error)?;
    emit_ag01(&app, &turn_id, s);

    let history_result = (|| -> Result<(), IpcError> {
        let conn = state
            .db
            .lock()
            .map_err(|_| IpcError::new("WS_DB_BUSY", "数据库连接繁忙", true, false))?;
        // 澄清恢复先由活动回合注册表完成 session/workspace/turn 身份认领；只有认领成功后才允许
        // 创建/更新持久化会话，错误身份不得留下空会话或草稿副作用。
        ag::history::ensure_session_exists(&conn, &workspace_id_for_turn, &session_id)
            .map_err(|e| IpcError::new("AG_HISTORY_DB_ERROR", &e.to_string(), true, false))?;
        if resume_kind != AgentTurnResumeKind::New {
            return ag::history::ensure_turn_resumable(
                &conn,
                &workspace_id_for_turn,
                &session_id,
                &turn_id,
            )
            .map_err(|e| {
                IpcError::new(
                    "AG_TURN_RESUME_HISTORY_INVALID",
                    &format!("原 AgentTurn 的持久化身份不可恢复：{e}"),
                    false,
                    false,
                )
            });
        }
        let draft_payload = ag::history::AgentTurnPayload {
            version: 2,
            last_event_seq: 0,
            user_message: ag::history::PersistedChatMessage {
                id: format!("u-{turn_id}"),
                role: "user".to_string(),
                content: prompt.trim().to_string(),
                parts: None,
                streaming: None,
                error: None,
                segments: None,
            },
            assistant_message: None,
            tool_summaries: Vec::new(),
            diff_refs: Vec::new(),
            failure: None,
            incomplete_reason: Some("stream_interrupted".to_string()),
        };
        ag::history::save_turn(
            &conn,
            &workspace_id_for_turn,
            &session_id,
            &turn_id,
            active_doc_path.as_deref(),
            "draft",
            &draft_payload,
        )
        .map_err(|e| IpcError::new("AG_HISTORY_DB_ERROR", &e.to_string(), true, false))?;
        Ok(())
    })();
    if let Err(error) = history_result {
        // draft 尚未建立时不存在可追加的 TurnEvent；由 command error 返回，不能制造未持久化事件。
        return Err(error);
    }
    let event_emitter = TurnEventEmitter::new(&state, &on_event, &turn_id);
    event_emitter.record_internal(
        "turn_started",
        serde_json::json!({
            "runtimeProfile": &runtime_profile,
            "clarificationResume": clarification_resume,
            "decisionResume": decision_resume,
        }),
    )?;
    event_emitter.record_internal(
        "context_frozen",
        serde_json::json!({
            "contextVersion": &agent_context_facts.version,
            "openDocCount": agent_context_facts.open_docs.len(),
            "selectionCount": agent_context_facts.selections.len(),
            "proposalFactCount": agent_context_facts.proposal_facts.len(),
        }),
    )?;

    // AG-1.0-IMP-TASKRUN-01（SM-AG-04 / BR-SYS-STATE-008）：先校验显式 TaskRun 续片段；
    // 普通 AgentTurn 此时不创建 TaskRun。只有后续确定需要 TaskPlan，或工具真实产生任务级产物时，
    // 才按需创建并绑定。应用启动时已清理陈旧活动态（重启不续跑）。
    // AG-1.0-TASKRUN-REWORK-IMP-RUNTIME-01：新消息 gate——存在未决 DecisionRequest 时拒绝新回合
    // （不得绕过待确认，A-AG-M-S-01 §2.2.2；三层分离）。
    let task_run_result = (|| -> Result<Option<String>, IpcError> {
        let conn = state
            .db
            .lock()
            .map_err(|_| IpcError::new("WS_DB_BUSY", "数据库连接繁忙", true, false))?;
        let ws = workspace::lifecycle::active_workspace(&conn)
            .map_err(|e| IpcError::new("WS_ACTIVE_FAILED", &e.to_string(), true, false))?
            .ok_or_else(|| IpcError::new("WS_NO_ACTIVE", "无活动 workspace", false, false))?;
        let task_run_id = if let Some(task_run_id) = continuation_task_run_id.as_deref() {
            match (
                continuation_decision_request_id.as_deref(),
                continuation_preview_diff_id.as_deref(),
            ) {
                (Some(decision_request_id), None) => {
                    ag::task::begin_resolved_decision_continuation(
                        &conn,
                        &ws.workspace_id,
                        &session_id,
                        task_run_id,
                        decision_request_id,
                    )
                    .map_err(|e| IpcError::new("AG_TASK_DB_ERROR", &e.to_string(), true, false))?
                    .ok_or_else(|| {
                        IpcError::new(
                            "AG_TASK_CONTINUATION_INVALID",
                            "裁决确认已失效，无法继续原任务；请重新发起任务。",
                            false,
                            false,
                        )
                    })?
                }
                (None, Some(diff_id)) => {
                    let checkpoint = ag::task::acknowledged_preview_checkpoint(
                        &conn,
                        task_run_id,
                        &ws.workspace_id,
                        &session_id,
                        diff_id,
                    )
                    .map_err(|e| IpcError::new("AG_TASK_DB_ERROR", &e.to_string(), true, false))?
                    .ok_or_else(|| {
                        IpcError::new(
                            "AG_PREVIEW_CONTINUATION_INVALID",
                            "编辑器预览回执已失效或已被消费，无法继续原任务。",
                            false,
                            false,
                        )
                    })?;
                    let fresh_snapshot = open_doc_snapshots.iter().any(|snapshot| {
                        snapshot.rel_path == checkpoint.rel_path
                            && snapshot.session_version == checkpoint.session_version
                            && snapshot.content_hash == checkpoint.content_hash
                    });
                    if !fresh_snapshot {
                        return Err(IpcError::new(
                            "AG_PREVIEW_CONTINUATION_STALE",
                            "续片段未携带编辑器预览后的最新 LogicalState，已拒绝继续。",
                            true,
                            false,
                        ));
                    }
                    ag::task::begin_preview_continuation(&conn, &checkpoint)
                        .map_err(|e| {
                            IpcError::new("AG_TASK_DB_ERROR", &e.to_string(), true, false)
                        })?
                        .ok_or_else(|| {
                            IpcError::new(
                                "AG_PREVIEW_CONTINUATION_INVALID",
                                "编辑器预览回执身份或基线不匹配，无法继续原任务。",
                                false,
                                false,
                            )
                        })?
                }
                _ => {
                    return Err(IpcError::new(
                        "AG_TASK_CONTINUATION_INVALID",
                        "续片段必须且只能提供已确认裁决或已确认预览回执之一。",
                        false,
                        false,
                    ));
                }
            }
            .into()
        } else {
            if continuation_decision_request_id.is_some() || continuation_preview_diff_id.is_some()
            {
                return Err(IpcError::new(
                    "AG_TASK_CONTINUATION_INVALID",
                    "续片段标识不能脱离 TaskRun 使用",
                    false,
                    false,
                ));
            }
            if let Some(request_id) =
                ag::task::latest_pending_decision_for_session(&conn, &ws.workspace_id, &session_id)
                    .map_err(|e| IpcError::new("AG_TASK_DB_ERROR", &e.to_string(), true, false))?
            {
                return Err(IpcError::new(
                    "AG_DECISION_PENDING",
                    &format!(
                        "存在待确认的任务级裁决项（{request_id}），请先完成确认；裁决结果将恢复原 AgentTurn。"
                    ),
                    false,
                    false,
                ));
            }
            clarification_task_run_id.clone()
        };
        Ok(task_run_id)
    })();
    let task_run_id = match task_run_result {
        Ok(task_run_id) => task_run_id,
        Err(error) => {
            // 已确认/已预览的续片段在身份核验阶段失败时，原 TaskRun 已可能处于 executing。
            // 必须使用请求携带的稳定身份收束，不能在建立局部 task_run_id 之前直接返回。
            let task_run_id = continuation_task_run_id
                .as_deref()
                .or(clarification_task_run_id.as_deref());
            settle_and_emit_ag_turn_error_from(
                &state,
                task_run_id,
                &event_emitter,
                &terminal_latch,
                &app,
                &turn_id,
                &error,
                TurnState::Assembling,
            );
            return Ok(());
        }
    };

    // 任何后续装配失败都经同一出口收束 AgentTurn；若已按需创建 TaskRun，同时收束其持久态。
    macro_rules! started_turn_or_finish {
        ($result:expr) => {
            match $result {
                Ok(value) => value,
                Err(error) => {
                    settle_and_emit_ag_turn_error_from(
                        &state,
                        task_run_id.as_deref(),
                        &event_emitter,
                        &terminal_latch,
                        &app,
                        &turn_id,
                        &error,
                        TurnState::Assembling,
                    );
                    return Ok(());
                }
            }
        };
    }

    if let Some(existing_task_run_id) = task_run_id.clone() {
        started_turn_or_finish!((|| {
            let conn = state
                .db
                .lock()
                .map_err(|_| IpcError::new("WS_DB_BUSY", "数据库连接繁忙", true, false))?;
            ag::task::link_turn(&conn, &turn_id, &existing_task_run_id)
                .map_err(|error| IpcError::new("AG_TASK_DB_ERROR", &error.to_string(), true, false))
        })());
    }

    let read_ctx = started_turn_or_finish!(ag::blocks::AgentReadContext::new(
        std::path::Path::new(&workspace_root_path),
        open_doc_snapshots,
    ));
    let active_snapshot = started_turn_or_finish!((|| -> Result<_, IpcError> {
        Ok(match active_doc_path.as_deref() {
            Some(path) => {
                let open = read_ctx.open_snapshot(path).ok_or_else(|| {
                    IpcError::new(
                        "AG_OPEN_DOC_SNAPSHOT_REQUIRED",
                        "当前打开文档缺少 LogicalState 快照，已阻断代理回合",
                        true,
                        false,
                    )
                })?;
                if !is_active_editor_snapshot_source(&open.source) {
                    return Err(IpcError::new(
                        "AG_ACTIVE_SNAPSHOT_REQUIRED",
                        "当前打开文档必须来自 active editor LogicalState 快照",
                        true,
                        false,
                    ));
                }
                if let Some(format) = active_doc_format.as_deref() {
                    if !format.is_empty() && format != open.edit_format {
                        return Err(IpcError::new(
                        "AG_ACTIVE_FORMAT_MISMATCH",
                        "active_doc_format 与 OpenDocSnapshot.editFormat 不一致，已阻断代理回合",
                        true,
                        false,
                    ));
                    }
                }
                if let Some(legacy_content) = active_doc_content.as_deref() {
                    if legacy_content != open.content {
                        return Err(IpcError::new(
                            "AG_ACTIVE_SNAPSHOT_MISMATCH",
                            "active_doc_content 与 OpenDocSnapshot 不一致，已阻断代理回合",
                            true,
                            false,
                        ));
                    }
                }
                Some(read_ctx.snapshot(path)?)
            }
            None => None,
        })
    })());

    let TurnReferenceRuntime { mention_block } =
        build_turn_reference_runtime(&mentions, &block_refs, &references, &read_ctx);
    let workspace_root = std::path::Path::new(&workspace_root_path);
    let mut explicit_edit_target_set = if let Some(task_run_id) = task_run_id.as_deref() {
        started_turn_or_finish!((|| {
            let conn = state
                .db
                .lock()
                .map_err(|_| IpcError::new("WS_DB_BUSY", "数据库连接繁忙", true, false))?;
            ag::target_set::load_for_task_run(&conn, task_run_id)
                .map_err(|error| IpcError::new("AG_TASK_DB_ERROR", &error.to_string(), true, false))
        })())
    } else if resume_kind != AgentTurnResumeKind::New {
        registered_edit_target_set
    } else {
        started_turn_or_finish!(ag::target_set::initial_for_turn(
            workspace_root,
            &turn_id,
            active_doc_path.as_deref(),
            &mentions,
            &block_refs,
            &references,
        ))
    };
    if clarification_resume {
        explicit_edit_target_set.merge(started_turn_or_finish!(
            ag::target_set::explicit_references_for_turn(
                workspace_root,
                &turn_id,
                &mentions,
                &block_refs,
                &references,
            )
        ));
    }
    started_turn_or_finish!(bind_agent_turn_edit_target_set(
        &state,
        &session_id,
        &turn_id,
        &explicit_edit_target_set,
    ));
    if let Some(task_run_id) = task_run_id.as_deref() {
        started_turn_or_finish!((|| {
            let conn = state
                .db
                .lock()
                .map_err(|_| IpcError::new("WS_DB_BUSY", "数据库连接繁忙", true, false))?;
            ag::target_set::persist_for_task_run(&conn, task_run_id, &explicit_edit_target_set)
                .map(|_| ())
                .map_err(|error| IpcError::new("AG_TASK_DB_ERROR", &error.to_string(), true, false))
        })());
    }

    // 初始语义只声明 unresolved。关键词 resolver 已退出生产主链；首个模型文本或 ToolCall 的结构化
    // 控制封套形成 revision=1，并作为后续 PromptPackage 的可见语义投影。
    let (intent_result, initial_prompt, mut continuation_messages) =
        match resumed_runtime_checkpoint {
            Some(checkpoint) => (
                checkpoint.intent,
                checkpoint.initial_prompt,
                checkpoint.continuation_messages,
            ),
            None => (
                ag::context::IntentResultV2::unresolved(),
                prompt.clone(),
                Vec::<serde_json::Value>::new(),
            ),
        };
    if clarification_resume {
        continuation_messages.push(serde_json::json!({
            "role": "user",
            "content": prompt,
        }));
    }
    let decision_receipt_ids = continuation_decision_request_id
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    event_emitter.record_internal(
        if intent_result.revision == 0 {
            "intent_initialized"
        } else {
            "intent_resumed"
        },
        serde_json::json!({
            "revision": intent_result.revision,
            "intentType": intent_result.intent_type,
        }),
    )?;
    if decision_resume {
        let decision_request_id = continuation_decision_request_id
            .as_deref()
            .expect("decision resume was validated with a request id");
        let current_task_run_id = task_run_id
            .as_deref()
            .expect("decision resume was validated with a task run");
        let outcome = started_turn_or_finish!((|| {
            let conn = state
                .db
                .lock()
                .map_err(|_| IpcError::new("WS_DB_BUSY", "数据库连接繁忙", true, false))?;
            ag::task::resolved_decision_outcome(&conn, current_task_run_id, decision_request_id)
                .map_err(|error| {
                    IpcError::new("AG_TASK_DB_ERROR", &error.to_string(), true, false)
                })?
                .ok_or_else(|| {
                    IpcError::new(
                        "AG_TASK_CONTINUATION_INVALID",
                        "裁决结果不存在，无法恢复原 AgentTurn",
                        false,
                        false,
                    )
                })
        })());
        event_emitter.emit(
            &terminal_latch,
            AgTurnEventPayload::DecisionRequestResolved {
                request_id: decision_request_id.to_string(),
                outcome: outcome.clone(),
            },
        )?;
        continuation_messages.push(serde_json::json!({
            "role": "system",
            "content": format!(
                "[DECISION RESULT — DATA ONLY] request_id={decision_request_id}; outcome={outcome}. Continue the same AgentTurn from this verified user decision."
            ),
        }));
    }

    // SM-AG-01 Assembling 必须在提交 Gate 前完成 PromptPackage、CapabilityGrant 与预算校验。
    // 这段装配不调用 provider、ToolCall 或 Proposal；任一失败都保持在 Assembling 的错误路径。
    let ws_root: Option<String> = Some(workspace_root_path.clone());
    let explicit_edit_targets = explicit_edit_target_set.paths();
    let requires_current_doc_tool_read = false;
    let PreparedTurnPrompt {
        current_doc_requires_tool_read,
        current_doc_prompt_coverage,
        messages,
        tools,
        capability_grant,
        capability_factors,
        prompt_package,
    } = started_turn_or_finish!(prepare_turn_prompt(
        &state,
        &runtime_profile,
        &turn_id,
        &session_id,
        task_run_id.as_deref(),
        &intent_result,
        &agent_context_facts,
        &explicit_edit_targets,
        &read_ctx,
        active_doc_path.as_deref(),
        active_snapshot.as_ref(),
        &history,
        &initial_prompt,
        mention_block.as_deref(),
        requires_current_doc_tool_read,
        false,
        &continuation_messages,
        &decision_receipt_ids,
        1,
        6,
    ));

    // 预算与实际 provider messages/tools 使用同一序列化口径，并在 Gate 前失败关闭。
    if ag::context::estimate_serialized_input_tokens(&messages, &tools) > PROMPT_TOKEN_BUDGET {
        let error = IpcError::new(
            "AG_PROMPT_BUDGET_EXCEEDED",
            "本轮可信上下文超出 Prompt 预算，请减少引用或缩小文档范围后重试。",
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
            TurnState::Assembling,
        );
        return Ok(());
    }
    eprintln!(
        "[ag-trace] turn_id={} session_id={} profile={} intent_revision={} intent_type={:?} context_trace_id={} tool_manifest_hash={} allowed_tool_count={} estimated_tokens={}",
        turn_id,
        session_id,
        runtime_profile,
        intent_result.revision,
        intent_result.intent_type,
        prompt_package.context_trace_id,
        prompt_package.tool_manifest_hash,
        capability_grant.allowed_tools.len(),
        prompt_package.estimated_tokens,
    );

    let s = started_turn_or_finish!(transition(s, TurnEvent::Assembled).ok_or_else(sm_error)); // → AwaitingGate
    emit_ag01(&app, &turn_id, s);
    if cancelled.load(Ordering::Acquire) {
        if let Ok(conn) = state.db.lock() {
            settle_optional_task_run(
                &conn,
                task_run_id.as_deref(),
                &turn_id,
                ag::task::TurnOutcome::Cancelled,
            );
        }
        emit_ag_turn_cancelled_from(
            &event_emitter,
            &terminal_latch,
            &app,
            &turn_id,
            TurnState::AwaitingGate,
        );
        return Ok(());
    }
    event_emitter.record_internal("gate_requested", serde_json::json!({"attempt": 1}))?;
    let gate = match ent::gateway::check_gate().await {
        Ok(ent::gateway::GateOutcome::Pass) => GateState::Passed,
        Ok(ent::gateway::GateOutcome::Deny) => GateState::Denied,
        Err(error) => {
            event_emitter.record_internal(
                "gate_evaluated",
                serde_json::json!({"attempt": 1, "outcome": "error", "code": &error.code}),
            )?;
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
    };
    event_emitter.record_internal(
        "gate_evaluated",
        serde_json::json!({
            "attempt": 1,
            "outcome": if gate == GateState::Passed { "passed" } else { "denied" },
        }),
    )?;
    if gate != GateState::Passed {
        let error = ensure_gate_passed(gate).expect_err("非 Passed 门禁必须被拒绝");
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
    if cancelled.load(Ordering::Acquire) {
        if let Ok(conn) = state.db.lock() {
            settle_optional_task_run(
                &conn,
                task_run_id.as_deref(),
                &turn_id,
                ag::task::TurnOutcome::Cancelled,
            );
        }
        emit_ag_turn_cancelled_from(
            &event_emitter,
            &terminal_latch,
            &app,
            &turn_id,
            TurnState::AwaitingGate,
        );
        return Ok(());
    }
    let s = transition(s, TurnEvent::GatePassed)
        .expect("AwaitingGate + GatePassed must be a valid SM-AG-01 transition"); // → Streaming
    emit_ag01(&app, &turn_id, s);
    debug_assert_eq!(s, TurnState::Streaming);

    let provider = provider.unwrap_or_else(|| "deepseek".to_string());
    let model = model.unwrap_or_else(|| ent::gateway::default_model(&provider));

    drop(event_emitter);
    run_turn_tool_loop(
        state,
        app,
        on_event,
        cancelled,
        terminal_latch,
        turn_id,
        session_id,
        task_run_id,
        workspace_id_for_turn,
        provider,
        model,
        runtime_profile,
        prompt_package,
        agent_context_facts,
        current_doc_prompt_coverage,
        messages,
        tools,
        capability_factors,
        read_ctx,
        active_doc_path,
        explicit_edit_target_set,
        ws_root,
        current_doc_requires_tool_read,
        intent_result,
        continuation_messages,
        history,
        initial_prompt,
        mention_block,
        active_snapshot,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // covers: BR-AG-STATE-004
    fn runtime_profile_is_required_and_unknown_values_fail_closed() {
        let missing = require_runtime_profile(None).expect_err("missing profile must fail closed");
        assert_eq!(missing.code, "AG_RUNTIME_PROFILE_REQUIRED");

        let unknown = require_runtime_profile(Some("future-profile".to_string()))
            .expect_err("unknown profile must fail closed");
        assert_eq!(unknown.code, "AG_RUNTIME_PROFILE_UNSUPPORTED");

        assert_eq!(
            require_runtime_profile(Some(ag::turn::AGENT_EDIT_1_0_PROFILE.to_string())).unwrap(),
            ag::turn::AGENT_EDIT_1_0_PROFILE
        );
    }

    // covers: BR-AG-DATA-008
    // covers: BR-ED-DATA-008
    // covers: BR-SYS-DATA-008
    #[test]
    fn agent_context_facts_fail_closed_on_logical_state_drift() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("ag-context-facts-{unique}"));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("a.md"), "disk old").unwrap();
        let snapshot = ag::blocks::OpenDocSnapshot {
            rel_path: "a.md".to_string(),
            content: "logical new".to_string(),
            baseline: Some(7),
            edit_format: "markdown".to_string(),
            content_format: "text".to_string(),
            source: "active_editor".to_string(),
            dirty: true,
            session_version: 3,
            content_hash: "h3".to_string(),
            captured_at: 1,
            block_snapshots: Vec::new(),
            table_snapshots: Vec::new(),
            image_snapshots: Vec::new(),
        };
        let doc_fact = crate::types::AgentContextDocumentFactV1 {
            rel_path: "a.md".to_string(),
            source: "active_editor".to_string(),
            edit_format: "markdown".to_string(),
            content_format: "text".to_string(),
            dirty: true,
            baseline: Some("7".to_string()),
            session_version: 3,
            content_hash: "h3".to_string(),
        };
        let facts = AgentContextFactsV1 {
            version: AGENT_CONTEXT_FACTS_VERSION_V1.to_string(),
            workspace_id: "ws1".to_string(),
            turn_id: "t1".to_string(),
            active_doc: Some(doc_fact.clone()),
            open_docs: vec![doc_fact],
            selections: vec![crate::types::AgentContextSelectionFactV1 {
                rel_path: "a.md".to_string(),
                block_id: "legacy".to_string(),
                start: None,
                end: None,
            }],
            proposal_facts: vec![crate::types::AgentContextProposalFactV1 {
                diff_id: "diff-1".to_string(),
                session_id: "s1".to_string(),
                rel_path: "a.md".to_string(),
                state: "accepted".to_string(),
                effect_status: Some("saved".to_string()),
                saved: true,
            }],
        };
        let legacy_ref = BlockRef {
            rel_path: "a.md".to_string(),
            block_id: "legacy".to_string(),
            start: None,
            end: None,
            text: "selected".to_string(),
        };
        validate_agent_context_facts(
            Some(&facts),
            "ws1",
            &root,
            "t1",
            "s1",
            Some("a.md"),
            std::slice::from_ref(&snapshot),
            &[],
            std::slice::from_ref(&legacy_ref),
        )
        .unwrap();

        let mut cross_session = facts.clone();
        cross_session.proposal_facts[0].session_id = "s2".to_string();
        let error = validate_agent_context_facts(
            Some(&cross_session),
            "ws1",
            &root,
            "t1",
            "s1",
            Some("a.md"),
            std::slice::from_ref(&snapshot),
            &[],
            std::slice::from_ref(&legacy_ref),
        )
        .unwrap_err();
        assert_eq!(error.code, "AG_CONTEXT_FACTS_PROPOSAL_INVALID");

        let mut stale = facts;
        stale.open_docs[0].content_hash = "old".to_string();
        let error = validate_agent_context_facts(
            Some(&stale),
            "ws1",
            &root,
            "t1",
            "s1",
            Some("a.md"),
            &[snapshot],
            &[],
            &[legacy_ref],
        )
        .unwrap_err();
        assert_eq!(error.code, "AG_CONTEXT_FACTS_LOGICAL_STATE_MISMATCH");
        let _ = std::fs::remove_dir_all(root);
    }

    // covers: BR-AG-STATE-001, BR-SYS-STATE-007
    // 生产源码顺序回归：PromptPackage/预算必须先于 assembled、Gate 和 Streaming。
    #[test]
    fn prompt_package_and_budget_precede_gate_and_streaming_in_production_path() {
        let source = include_str!("turn_runtime.rs");
        let prepare_marker = ["started_turn_or_finish!(prepare_turn_prompt", "("].concat();
        let budget_marker = ["estimate_serialized_input_tokens", "("].concat();
        let assembled_marker = ["TurnEvent::", "Assembled"].concat();
        let gate_marker = ["let gate = match ent::gateway::check_gate", "()"].concat();
        let streaming_marker = ["TurnEvent::", "GatePassed"].concat();

        let prepare = source
            .find(&prepare_marker)
            .expect("production prompt assembly");
        let budget = source
            .find(&budget_marker)
            .expect("production budget check");
        let assembled = source
            .find(&assembled_marker)
            .expect("production assembled transition");
        let gate = source.find(&gate_marker).expect("production Gate call");
        let streaming = source
            .find(&streaming_marker)
            .expect("production streaming transition");

        assert!(prepare < budget);
        assert!(budget < assembled);
        assert!(assembled < gate);
        assert!(gate < streaming);
    }
}
