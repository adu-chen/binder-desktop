//! AgentTurn 请求装配、模型轮次与受控工具循环。

use crate::ag::prompt_runtime::PROMPT_TOKEN_BUDGET;
use crate::ipc::{AgTurnEvent, IpcError};
use crate::types::ReferencePayload;
use crate::{ag, ent, workspace, AppState};
use std::sync::atomic::Ordering;

use super::edit_support::is_active_editor_snapshot_source;
use super::reference_runtime::BlockRef;
use super::task_runtime::{build_task_plan, settle_optional_task_run};
use super::turn_control::{
    bind_agent_turn_edit_target_set, bind_agent_turn_task_run, park_agent_turn_for_clarification,
    register_active_agent_turn,
};
use super::turn_events::{
    emit_ag01, emit_ag_turn_cancelled, emit_ag_turn_denied, emit_ag_turn_error,
    settle_and_emit_ag_turn_error, sm_error,
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
 * codes:    BR-AG-STATE-001-API-AG-AG-TURN-018, BR-SYS-STATE-007-API-AG-AG-TURN-018
 * type:     API
 * chain:    AG-TURN, AG-TOOL
 * rules:    BR-AG-STATE-001, BR-SYS-STATE-007
 * term_ref: TERM-AG-001, TERM-AG-012, TERM-AG-013, TERM-AG-014, TERM-AG-015, TERM-AG-016
 * boundary: in=ValidatedRuntimeProfileAndAgentTurnRequestOrSameTurnClarificationResumeAndAgentReadContext | out=AgentTurnEventStreamOrAwaitingClarificationIdentityAndOptionalTaskRunTransitions | effect=IPC-ag_start_turn | delegate=require_runtime_profile,resolve_intent_result_with_targets,prepare_turn_prompt,ent::gateway::check_gate,ag::turn::transition,run_turn_tool_loop,register_active_agent_turn | does_not=defaultMissingRuntimeProfileOrCreateTaskRunForSimpleAgentTurnOrCreateNewAgentTurnForClarificationOrBypassCurrentGateOutcomeOrWriteDocumentContentOutsidePendingDiff
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
    history: Vec<ag::context::HistTurn>,
    mentions: Vec<String>,
    block_refs: Vec<BlockRef>,
    references: Vec<ReferencePayload>,
    continuation_task_run_id: Option<String>,
    continuation_decision_request_id: Option<String>,
    continuation_preview_diff_id: Option<String>,
    clarification_resume: Option<bool>,
    on_event: tauri::ipc::Channel<AgTurnEvent>,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), IpcError> {
    use ag::turn::{ensure_gate_passed, transition, GateState, TurnEvent, TurnState};

    let runtime_profile = require_runtime_profile(runtime_profile)?;
    let clarification_resume = clarification_resume.unwrap_or(false);

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

    let (_active_turn_lease, cancelled, clarification_task_run_id, registered_edit_target_set) =
        register_active_agent_turn(
            &state,
            &workspace_id_for_turn,
            &session_id,
            &turn_id,
            clarification_resume,
        )?;
    let terminal_latch = ag::turn::TerminalLatch::default();

    {
        let conn = state
            .db
            .lock()
            .map_err(|_| IpcError::new("WS_DB_BUSY", "数据库连接繁忙", true, false))?;
        // 澄清恢复先由活动回合注册表完成 session/workspace/turn 身份认领；只有认领成功后才允许
        // 创建/更新持久化会话，错误身份不得留下空会话或草稿副作用。
        ag::history::ensure_session_exists(&conn, &workspace_id_for_turn, &session_id)
            .map_err(|e| IpcError::new("AG_HISTORY_DB_ERROR", &e.to_string(), true, false))?;
        let draft_payload = ag::history::AgentTurnPayload {
            version: 1,
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
    }

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
                (Some(decision_request_id), None) => ag::task::begin_confirmed_continuation(
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
                })?,
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
                        "存在待确认的任务级裁决项（{request_id}），请先完成确认；确认后自动继续下一执行片段。"
                    ),
                    false,
                    false,
                ));
            }
            clarification_task_run_id.clone()
        };
        Ok(task_run_id)
    })();
    let mut task_run_id = match task_run_result {
        Ok(task_run_id) => task_run_id,
        Err(error) => {
            // 已确认/已预览的续片段在身份核验阶段失败时，原 TaskRun 已可能处于 executing。
            // 必须使用请求携带的稳定身份收束，不能在建立局部 task_run_id 之前直接返回。
            if let Some(task_run_id) = continuation_task_run_id
                .as_deref()
                .or(clarification_task_run_id.as_deref())
            {
                settle_and_emit_ag_turn_error(
                    &state,
                    Some(task_run_id),
                    &on_event,
                    &terminal_latch,
                    &app,
                    &turn_id,
                    &error,
                );
                return Ok(());
            }
            return Err(error);
        }
    };

    // 任何后续装配失败都经同一出口收束 AgentTurn；若已按需创建 TaskRun，同时收束其持久态。
    macro_rules! started_turn_or_finish {
        ($result:expr) => {
            match $result {
                Ok(value) => value,
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
    } else if clarification_resume {
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

    // 结构性门禁前置：唯一通往 Streaming（模型调用）的路径是 AwaitingGate + gatePassed（X-SEQ-001）。
    // 每步转移广播 SM-AG-01（A-CORE-X-L-04 Task 2-E）：sending 等回合视图据此投影（X-FORBID-007）。
    let s = if clarification_resume {
        started_turn_or_finish!(transition(
            TurnState::AwaitingClarification,
            TurnEvent::ClarificationProvided,
        )
        .ok_or_else(sm_error))
    } else {
        started_turn_or_finish!(transition(TurnState::Idle, TurnEvent::Send).ok_or_else(sm_error))
    }; // → Assembling
    emit_ag01(&app, &turn_id, s);
    let has_selection = active_doc_path.as_deref().is_some_and(|active_path| {
        block_refs
            .iter()
            .any(|block_ref| block_ref.rel_path == active_path)
            || references.iter().any(|reference| {
                matches!(
                    reference,
                    ReferencePayload::Selection { rel_path, .. } if rel_path == active_path
                )
            })
    });
    let intent_target_paths = explicit_edit_target_set.paths();
    let intent_result = ag::context::resolve_intent_result_with_targets(
        &turn_id,
        &prompt,
        active_doc_path.as_deref(),
        has_selection,
        &intent_target_paths,
    );
    // AG-1.0-TASKRUN-REWORK-INTENT-01：IntentResult 复杂目标 → 自动创建 TaskPlan（TERM-AG-018）。
    // 计划为可展示的受控步骤（读取→生成建议）；实际 ToolResult 才推进计划状态和 cursor。
    // 简单单文档局部编辑直行不建；同一 run 已有计划时不重复建。
    {
        let target_paths = explicit_edit_target_set.paths();
        let is_write = matches!(
            intent_result.intent_type,
            ag::context::IntentTypeV1::Edit | ag::context::IntentTypeV1::CreateDocument
        );
        if ag::task::is_complex_target(is_write, &target_paths, &prompt) {
            if task_run_id.is_none() {
                let created_task_run_id = started_turn_or_finish!((|| {
                    let conn = state
                        .db
                        .lock()
                        .map_err(|_| IpcError::new("WS_DB_BUSY", "数据库连接繁忙", true, false))?;
                    ag::task::begin_for_complex_target(
                        &conn,
                        &workspace_id_for_turn,
                        &session_id,
                        is_write,
                        &target_paths,
                        &prompt,
                    )
                    .map_err(|error| {
                        IpcError::new("AG_TASK_DB_ERROR", &error.to_string(), true, false)
                    })
                    .and_then(|task_run_id| {
                        task_run_id.ok_or_else(|| {
                            IpcError::new(
                                "AG_TASK_REQUIREMENT_DRIFT",
                                "复杂目标判据在 TaskRun 创建边界发生漂移",
                                false,
                                false,
                            )
                        })
                    })
                })());
                task_run_id = Some(created_task_run_id.clone());
                started_turn_or_finish!(bind_agent_turn_task_run(
                    &state,
                    &session_id,
                    &turn_id,
                    &created_task_run_id,
                ));
                started_turn_or_finish!((|| {
                    let conn = state
                        .db
                        .lock()
                        .map_err(|_| IpcError::new("WS_DB_BUSY", "数据库连接繁忙", true, false))?;
                    ag::task::link_turn(&conn, &turn_id, &created_task_run_id).map_err(
                        |error| IpcError::new("AG_TASK_DB_ERROR", &error.to_string(), true, false),
                    )?;
                    ag::target_set::persist_for_task_run(
                        &conn,
                        &created_task_run_id,
                        &explicit_edit_target_set,
                    )
                    .map(|_| ())
                    .map_err(|error| {
                        IpcError::new("AG_TASK_DB_ERROR", &error.to_string(), true, false)
                    })
                })());
            }
            let current_task_run_id = task_run_id
                .as_deref()
                .expect("complex target must bind a task run before creating its plan");
            started_turn_or_finish!((|| {
                let conn = state
                    .db
                    .lock()
                    .map_err(|_| IpcError::new("WS_DB_BUSY", "数据库连接繁忙", true, false))?;
                let plan_missing = ag::task::list_plans_by_run(&conn, current_task_run_id)
                    .map_err(|error| {
                        IpcError::new("AG_TASK_DB_ERROR", &error.to_string(), true, false)
                    })?
                    .is_empty();
                if plan_missing {
                    let plan = build_task_plan(current_task_run_id, &target_paths);
                    ag::task::create_plan(&conn, &plan).map_err(|error| {
                        IpcError::new("AG_TASK_DB_ERROR", &error.to_string(), true, false)
                    })?;
                }
                Ok::<(), IpcError>(())
            })());
        }
    }
    if intent_result.is_blocking() {
        let reason = intent_result
            .ambiguity_reasons
            .first()
            .map(String::as_str)
            .unwrap_or("当前请求缺少安全执行所需的信息");
        // 澄清是等待用户输入的非终态（BR-SYS-STATE-007）：不锁存终态、不广播 Done。
        let s = started_turn_or_finish!(
            transition(s, TurnEvent::ClarificationRequired).ok_or_else(sm_error)
        ); // → AwaitingClarification
           // 先原子保留 session/workspace/turn/TaskRun 身份，再向前端公布可补充；command 返回后 lease
           // 仍保留等待态，下一次只允许同 turn_id 的 clarificationResume 认领一次。
        started_turn_or_finish!(park_agent_turn_for_clarification(
            &state,
            &session_id,
            &turn_id,
            task_run_id.as_deref(),
        ));
        emit_ag01(&app, &turn_id, s);
        let _ = on_event.send(AgTurnEvent::ClarificationRequired {
            turn_id: turn_id.clone(),
            text: format!("需要你先确认：{reason}。请打开或明确指定要修改的当前文档。"),
        });
        return Ok(());
    }

    // SM-AG-01 Assembling 必须在提交 Gate 前完成 PromptPackage、CapabilityGrant 与预算校验。
    // 这段装配不调用 provider、ToolCall 或 Proposal；任一失败都保持在 Assembling 的错误路径。
    let ws_root: Option<String> = Some(workspace_root_path.clone());
    let explicit_edit_targets = explicit_edit_target_set.paths();
    let required_terminal_tool = intent_result.required_terminal_tool().map(str::to_string);
    let requires_current_doc_tool_read = required_terminal_tool.as_deref() == Some("propose_edit");
    if intent_result.intent_type == ag::context::IntentTypeV1::Edit {
        if let Some(snapshot) = &active_snapshot {
            if !snapshot.content.trim().is_empty() && snapshot.blocks.is_empty() {
                let error = IpcError::new(
                    "AG_ACTIVE_BLOCKS_REQUIRED",
                    "当前文档块投影为空，已阻断编辑请求，请重新加载文档后重试。",
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
                return Ok(());
            }
        }
    }
    let PreparedTurnPrompt {
        current_doc_requires_tool_read,
        messages,
        tools,
        capability_grant,
        prompt_package,
    } = started_turn_or_finish!(prepare_turn_prompt(
        &state,
        &runtime_profile,
        &turn_id,
        &session_id,
        task_run_id.as_deref(),
        &intent_result,
        &explicit_edit_targets,
        &read_ctx,
        active_doc_path.as_deref(),
        active_snapshot.as_ref(),
        &history,
        &prompt,
        mention_block.as_deref(),
        requires_current_doc_tool_read,
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
        emit_ag_turn_error(&on_event, &terminal_latch, &app, &turn_id, &error);
        return Ok(());
    }
    eprintln!(
        "[ag-trace] turn_id={} session_id={} profile={} intent_id={} intent_type={:?} context_trace_id={} tool_manifest_hash={} envelope_count={} allowed_tool_count={} estimated_tokens={}",
        turn_id,
        session_id,
        runtime_profile,
        intent_result.intent_id,
        intent_result.intent_type,
        prompt_package.context_trace_id,
        prompt_package.tool_manifest_hash,
        prompt_package.envelopes.len(),
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
        emit_ag_turn_cancelled(&on_event, &terminal_latch, &app, &turn_id);
        return Ok(());
    }
    let gate = match ent::gateway::check_gate().await {
        Ok(ent::gateway::GateOutcome::Pass) => GateState::Passed,
        Ok(ent::gateway::GateOutcome::Deny) => GateState::Denied,
        Err(error) => {
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
    };
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
        emit_ag_turn_denied(&on_event, &terminal_latch, &app, &turn_id, &error);
        return Ok(());
    }
    started_turn_or_finish!(ensure_gate_passed(gate));
    if cancelled.load(Ordering::Acquire) {
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
    let s = started_turn_or_finish!(transition(s, TurnEvent::GatePassed).ok_or_else(sm_error)); // → Streaming
    emit_ag01(&app, &turn_id, s);
    debug_assert_eq!(s, TurnState::Streaming);

    let provider = provider.unwrap_or_else(|| "deepseek".to_string());
    let model = model.unwrap_or_else(|| ent::gateway::default_model(&provider));

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
        messages,
        tools,
        capability_grant,
        read_ctx,
        active_doc_path,
        explicit_edit_target_set,
        ws_root,
        required_terminal_tool,
        current_doc_requires_tool_read,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
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
