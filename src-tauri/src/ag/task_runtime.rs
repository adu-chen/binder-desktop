//! DecisionRequest 运行时与 TaskRun/TaskPlan 只读事件投影查询。

use crate::ipc::{CommandRequest, IpcError};
use crate::{active_ws, ag, lock_db, str_param, workspace, ws_json, AppState};
use rusqlite::OptionalExtension;

fn now_millis() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

/**
 * @GOV
 * codes:    BR-AG-DATA-003-EFFECT-AG-AG-TURN-037
 * type:     EFFECT
 * chain:    AG-TURN, WS-OPERATION
 * rules:    BR-AG-DATA-003
 * term_ref: TERM-AG-015, TERM-AG-019, TERM-WS-003
 * boundary: in=CompletedCreateDocumentBundleWithExistingWorkspaceResource | out=PersistedTaskRunExplicitEditTarget | effect=workspaceDb | does_not=authorizePlannedOrMissingResource
 */
fn persist_completed_create_document_target(
    conn: &rusqlite::Connection,
    workspace_root: &std::path::Path,
    task_run_id: &str,
    plan_id: &str,
    bundle: &crate::types::CreateDocumentBundle,
) -> Result<(), IpcError> {
    let target =
        ag::target_set::normalize_existing_target(workspace_root, &bundle.target.rel_path)?;
    ag::target_set::append_task_run_targets(
        conn,
        task_run_id,
        &[target],
        ag::target_set::SOURCE_CREATE_DOCUMENT_BUNDLE,
        Some(plan_id),
    )
    .map(|_| ())
    .map_err(|error| IpcError::new("AG_TASK_DB_ERROR", &error.to_string(), true, false))
}

/**
 * @GOV
 * codes:    BR-SYS-STATE-008-QUERY-AG-AG-TURN-025
 * type:     QUERY
 * chain:    AG-TURN
 * rules:    BR-SYS-STATE-008
 * term_ref: TERM-AG-015
 * boundary: in=ActiveWorkspaceAgentSessionIdentity | out=TurnEventDerivedTaskRunProjectionV2List
 */
pub(crate) fn route_list_agent_task_runs(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let ws = active_ws(state)?;
    let session_id = str_param(request, "sessionId", "AG_SESSION_ID_REQUIRED")?;
    let conn = lock_db(state)?;
    let runs = ag::task::list_task_runs_by_session(&conn, &ws.workspace_id, &session_id)
        .map_err(|e| IpcError::new("AG_TASK_DB_ERROR", &e.to_string(), true, false))?;
    ws_json(&runs)
}

/// 简单 AgentTurn 没有 TaskRun；任务级路径存在时才收束对应持久态。
pub(super) fn settle_optional_task_run(
    conn: &rusqlite::Connection,
    task_run_id: Option<&str>,
    turn_id: &str,
    outcome: ag::task::TurnOutcome,
) {
    if let Some(task_run_id) = task_run_id {
        settle_task_run(conn, task_run_id, turn_id, outcome);
    }
}

/**
 * @GOV
 * codes:    BR-SYS-STATE-008-EFFECT-AG-AG-TURN-026, BR-AG-DATA-003-EFFECT-AG-AG-TURN-026
 * type:     EFFECT
 * chain:    AG-TURN
 * rules:    BR-SYS-STATE-008, BR-AG-DATA-003
 * term_ref: TERM-AG-015, TERM-AG-017, TERM-AG-019
 * boundary: in=TaskRunDecisionRequestProposalAndActiveWorkspace | out=PersistedDecisionRequestWithNormalizedFrozenScopeAndWaitingTaskProjection | does_not=persistMissingOutsideWorkspaceOrAlreadyAuthorizedScopeTargetsScheduleTaskPlanOrStartProvider
 */
/// AG-1.0-TASKRUN-REWORK-IMP-RUNTIME-01：`ag.propose_decision` —— 发起任务级裁决请求
/// （TERM-AG-017；kind：permission/plan_choice/scope/manual_confirm；投影进入 Waiting）。
pub(crate) fn route_propose_decision(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let task_run_id = str_param(request, "taskRunId", "AG_TASK_RUN_ID_REQUIRED")?;
    let kind = str_param(request, "kind", "AG_DECISION_KIND_REQUIRED")?;
    let mut payload = request
        .params
        .get("payload")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let scope_workspace = if kind == "scope" {
        Some(active_ws(state)?)
    } else {
        None
    };
    let conn = lock_db(state)?;
    if let Some(ws) = scope_workspace.as_ref() {
        let run = ag::task::load_task_run(&conn, &task_run_id)
            .map_err(|error| IpcError::new("AG_TASK_DB_ERROR", &error.to_string(), true, false))?
            .ok_or_else(|| IpcError::new("AG_TASK_NOT_FOUND", "TaskRun 不存在", false, false))?;
        if run.workspace_id != ws.workspace_id {
            return Err(IpcError::new(
                "AG_TASK_WORKSPACE_MISMATCH",
                "scope DecisionRequest 的 TaskRun 不属于 active workspace",
                false,
                false,
            ));
        }
        let raw_paths = payload
            .get("targetPaths")
            .and_then(|value| value.as_array())
            .ok_or_else(|| {
                IpcError::new(
                    "AG_EDIT_TARGET_SCOPE_INVALID",
                    "scope DecisionRequest 缺少 targetPaths",
                    false,
                    false,
                )
            })?
            .iter()
            .map(|value| {
                value.as_str().map(str::to_owned).ok_or_else(|| {
                    IpcError::new(
                        "AG_EDIT_TARGET_SCOPE_INVALID",
                        "scope DecisionRequest 的 targetPaths 必须是字符串路径",
                        false,
                        false,
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let current = ag::target_set::load_for_task_run(&conn, &task_run_id)
            .map_err(|error| IpcError::new("AG_TASK_DB_ERROR", &error.to_string(), true, false))?;
        let candidates = ag::target_set::validated_scope_candidates(
            std::path::Path::new(&ws.root_path),
            &raw_paths,
            &current,
        )?;
        let object = payload.as_object_mut().ok_or_else(|| {
            IpcError::new(
                "AG_EDIT_TARGET_SCOPE_INVALID",
                "scope DecisionRequest payload 必须是对象",
                false,
                false,
            )
        })?;
        object.insert(
            "targetPaths".to_string(),
            serde_json::to_value(candidates).map_err(|error| {
                IpcError::new(
                    "AG_EDIT_TARGET_SCOPE_INVALID",
                    &error.to_string(),
                    false,
                    false,
                )
            })?,
        );
    }
    let decision = ag::task::propose_decision(&conn, &task_run_id, &kind, payload)
        .map_err(|e| IpcError::new("AG_TASK_DB_ERROR", &e.to_string(), true, false))?;
    ws_json(&decision)
}

/**
 * @GOV
 * codes:    BR-AG-STATE-002-API-AG-WS-OPERATION-005, BR-AG-DATA-003-API-AG-WS-OPERATION-005, BR-SYS-STATE-008-API-AG-WS-OPERATION-005, BR-WS-STATE-003-API-AG-WS-OPERATION-005, BR-WS-PERSIST-003-API-AG-WS-OPERATION-005, BR-WS-OBS-002-API-AG-WS-OPERATION-005
 * type:     API
 * chain:    AG-TURN, AG-TOOL, WS-OPERATION
 * rules:    BR-AG-STATE-002, BR-AG-DATA-003, BR-SYS-STATE-008, BR-WS-STATE-003, BR-WS-PERSIST-003, BR-WS-OBS-002
 * term_ref: TERM-AG-015, TERM-AG-017, TERM-AG-019, TERM-WS-003
 * boundary: in=DecisionRequestOutcomeBoundToFrozenScopeOrWorkspaceOperationPlan | out=DecisionReceiptWorkspaceOperationReceiptAndPersistedExplicitEditTargetSet | does_not=executeUnconfirmedPlanTreatModelTextAsUserConfirmationOrAdvanceTaskPlan
 */
/// AG-1.0-TASKRUN-REWORK-IMP-RUNTIME-01：`ag.resolve_decision` —— 任务级裁决
/// confirm/cancel 只裁决对应请求；同回合恢复和后续 activity 由 AgentTurn 入口证明。
pub(crate) fn route_resolve_decision(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let request_id = str_param(request, "requestId", "AG_DECISION_REQUEST_ID_REQUIRED")?;
    let outcome = str_param(request, "outcome", "AG_DECISION_OUTCOME_REQUIRED")?;
    if outcome != "confirm" && outcome != "cancel" {
        return Err(IpcError::new(
            "AG_DECISION_OUTCOME_INVALID",
            "outcome 须为 confirm 或 cancel",
            false,
            false,
        ));
    }
    let ws = active_ws(state)?;
    let conn = lock_db(state)?;
    let decision_row: Option<(String, String, String, String, Option<String>)> = conn
        .query_row(
            "SELECT task_run_id, kind, payload_json, status, outcome FROM decision_requests WHERE request_id=?1",
            [&request_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .optional()
        .map_err(|error| IpcError::new("AG_TASK_DB_ERROR", &error.to_string(), true, false))?;
    let (
        decision_task_run_id,
        decision_kind,
        decision_payload_json,
        decision_status,
        resolved_outcome,
    ) = decision_row.ok_or_else(|| {
        IpcError::new(
            "AG_DECISION_NOT_FOUND",
            "DecisionRequest 不存在",
            false,
            false,
        )
    })?;
    if decision_status == "resolved" && resolved_outcome.as_deref() != Some(outcome.as_str()) {
        return Err(IpcError::new(
            "AG_DECISION_ALREADY_RESOLVED",
            &format!(
                "DecisionRequest 已裁决为 {}，不能改为 {}",
                resolved_outcome.as_deref().unwrap_or("unknown"),
                outcome
            ),
            false,
            false,
        ));
    }
    let decision_payload: serde_json::Value =
        serde_json::from_str(&decision_payload_json).unwrap_or(serde_json::Value::Null);
    let operation_plan_id = decision_payload
        .get("workspaceOperationPlanId")
        .and_then(|value| value.as_str())
        .map(str::to_owned);
    let operation_plan = if let Some(plan_id) = operation_plan_id.as_deref() {
        let plan = workspace::operation::load_plan(&conn, plan_id)
            .map_err(|error| {
                IpcError::new("WS_OPERATION_DB_ERROR", &error.to_string(), true, false)
            })?
            .ok_or_else(|| {
                IpcError::new(
                    "WS_OPERATION_PLAN_NOT_FOUND",
                    "WorkspaceOperationPlan 不存在",
                    false,
                    false,
                )
            })?;
        if plan.workspace_id != ws.workspace_id
            || plan.task_run_id.as_deref() != Some(decision_task_run_id.as_str())
            || plan.decision_request_id.as_deref() != Some(request_id.as_str())
        {
            return Err(IpcError::new(
                "AG_CAPABILITY_GRANT_DENIED",
                "WorkspaceOperationPlan 与当前 workspace、TaskRun 或 DecisionRequest 不匹配",
                false,
                false,
            ));
        }
        let session_id: String = conn
            .query_row(
                "SELECT session_id FROM task_runs WHERE task_run_id=?1 AND workspace_id=?2",
                rusqlite::params![&decision_task_run_id, &ws.workspace_id],
                |row| row.get(0),
            )
            .map_err(|error| IpcError::new("AG_TASK_DB_ERROR", &error.to_string(), true, false))?;
        if outcome == "confirm" {
            let source_turn_id = plan.source_turn_id.as_deref().ok_or_else(|| {
                IpcError::new(
                    "AG_CAPABILITY_GRANT_DENIED",
                    "WorkspaceOperationPlan 缺少 source AgentTurn，不能生成运行时授权",
                    false,
                    false,
                )
            })?;
            super::turn_control::validate_agent_turn_decision_resume(
                state,
                &session_id,
                source_turn_id,
                &request_id,
            )?;
        }
        Some((plan, session_id))
    } else {
        None
    };
    let confirmed_scope_targets =
        if decision_status != "resolved" && decision_kind == "scope" && outcome == "confirm" {
            let raw_paths = decision_payload
                .get("targetPaths")
                .and_then(|value| value.as_array())
                .ok_or_else(|| {
                    IpcError::new(
                        "AG_EDIT_TARGET_SCOPE_INVALID",
                        "scope DecisionRequest 缺少冻结的 targetPaths",
                        false,
                        false,
                    )
                })?
                .iter()
                .map(|value| {
                    value.as_str().map(str::to_owned).ok_or_else(|| {
                        IpcError::new(
                            "AG_EDIT_TARGET_SCOPE_INVALID",
                            "scope DecisionRequest 的 targetPaths 必须是字符串路径",
                            false,
                            false,
                        )
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            let current = ag::target_set::load_for_task_run(&conn, &decision_task_run_id).map_err(
                |error| IpcError::new("AG_TASK_DB_ERROR", &error.to_string(), true, false),
            )?;
            Some(ag::target_set::validated_scope_candidates(
                std::path::Path::new(&ws.root_path),
                &raw_paths,
                &current,
            )?)
        } else {
            None
        };
    // WorkspaceOperation 的 confirm 必须在 DecisionRequest 落库前取得原 AgentTurn 的效果锁。
    // 这样取消若先发生，本次裁决保持 Proposed 且零副作用；若 confirm 先取得锁，取消等待
    // 已开始的原子裁决与 WS 执行完成，避免“已确认但未执行”的半状态。
    let _runtime_effect_guard = if outcome == "confirm" {
        operation_plan
            .as_ref()
            .map(|(plan, session_id)| {
                let source_turn_id = plan.source_turn_id.as_deref().ok_or_else(|| {
                    IpcError::new(
                        "AG_CAPABILITY_GRANT_DENIED",
                        "WorkspaceOperationPlan 缺少 source AgentTurn，不能生成运行时授权",
                        false,
                        false,
                    )
                })?;
                super::turn_control::lock_agent_turn_effect(state, session_id, source_turn_id)
            })
            .transpose()?
    } else {
        None
    };
    let (task_run_id, mut status, already_resolved) =
        ag::task::resolve_decision_with_scope_targets(
            &conn,
            &request_id,
            &outcome,
            confirmed_scope_targets.as_deref(),
        )
        .map_err(|e| IpcError::new("AG_TASK_DB_ERROR", &e.to_string(), true, false))?;
    if let Some(plan_id) = operation_plan_id.as_deref() {
        eprintln!(
            "[workspace-operation] {}",
            serde_json::json!({
                "event": "decision_resolved",
                "plan_id": plan_id,
                "request_id": &request_id,
                "outcome": &outcome,
            })
        );
    }
    let mut operation_receipt = None;
    let mut workspace_operation_items = None;
    let mut create_document_bundle = None;
    if let Some((plan, _session_id)) = operation_plan {
        let plan_id = plan.plan_id.clone();
        if outcome == "confirm" {
            let source_turn_id = plan.source_turn_id.as_deref().ok_or_else(|| {
                IpcError::new(
                    "AG_CAPABILITY_GRANT_DENIED",
                    "WorkspaceOperationPlan 缺少 source AgentTurn，不能生成运行时授权",
                    false,
                    false,
                )
            })?;
            let computed_at_seq = now_millis().max(1) as u64;
            let runtime_grant = ag::tool_registry::resolve_capability_grant_v2(
                ag::turn::AGENT_EDIT_1_0_PROFILE,
                source_turn_id,
                &[],
                ag::tool_registry::CapabilityFactorsV2 {
                    has_task_run: true,
                    decision_receipt_ids: vec![request_id.clone()],
                    ..Default::default()
                },
                ag::tool_registry::CapabilityGrantPurposeV2::RuntimeWorkspaceEffect {
                    plan_id: plan_id.clone(),
                    decision_request_id: request_id.clone(),
                    computed_at_seq,
                },
            );
            if let ag::tool::ToolDecision::Reject(reason) =
                ag::tool_registry::validate_workspace_operation_effect_for_grant(
                    &runtime_grant,
                    source_turn_id,
                    &plan_id,
                    &request_id,
                    computed_at_seq,
                )
            {
                return Err(IpcError::new(
                    "AG_CAPABILITY_GRANT_DENIED",
                    &reason,
                    false,
                    false,
                ));
            }
            match workspace::operation::execute_operation_plan(&conn, &ws, &plan_id, &request_id) {
                Ok(receipt) => {
                    if receipt.final_status != crate::types::WorkspaceOperationStatus::Completed {
                        status = settle_task_run(
                            &conn,
                            &task_run_id,
                            source_turn_id,
                            ag::task::TurnOutcome::Failed,
                        )
                        .ok_or_else(|| {
                            IpcError::new(
                                "AG_TASK_DB_ERROR",
                                "TaskRun 失败状态未能落库",
                                true,
                                false,
                            )
                        })?;
                    }
                    workspace_operation_items = workspace::operation::load_plan(&conn, &plan_id)
                        .map_err(|error| {
                            IpcError::new("WS_OPERATION_DB_ERROR", &error.to_string(), true, false)
                        })?
                        .map(|plan| plan.operation_items);
                    if receipt.final_status == crate::types::WorkspaceOperationStatus::Completed {
                        if let Some(bundle) =
                            workspace::operation::load_create_document_bundle(&conn, &plan_id)
                                .map_err(|error| {
                                    IpcError::new(
                                        "WS_OPERATION_DB_ERROR",
                                        &error.to_string(),
                                        true,
                                        false,
                                    )
                                })?
                        {
                            persist_completed_create_document_target(
                                &conn,
                                std::path::Path::new(&ws.root_path),
                                &task_run_id,
                                &plan_id,
                                &bundle,
                            )?;
                            if !already_resolved {
                                create_document_bundle = Some(bundle);
                            }
                        }
                    }
                    operation_receipt = Some(ws_json(&receipt)?);
                }
                Err(error) => {
                    let _ = settle_task_run(
                        &conn,
                        &task_run_id,
                        source_turn_id,
                        ag::task::TurnOutcome::Failed,
                    );
                    return Err(error);
                }
            }
        } else {
            workspace::operation::cancel_operation_plan(
                &conn,
                &ws.workspace_id,
                &plan_id,
                &request_id,
            )?;
        }
    }
    ws_json(&serde_json::json!({
        "task_run_id": task_run_id,
        "status": status,
        "workspace_operation_receipt": operation_receipt,
        "workspace_operation_items": workspace_operation_items,
        "create_document_bundle": create_document_bundle,
        "already_resolved": already_resolved,
    }))
}

/**
 * @GOV
 * codes:    BR-SYS-STATE-008-QUERY-AG-AG-TURN-027
 * type:     QUERY
 * chain:    AG-TURN
 * rules:    BR-SYS-STATE-008
 * term_ref: TERM-AG-015, TERM-AG-018
 * boundary: in=ActiveWorkspaceTaskRunIdentity | out=CurrentRouteDerivedTaskPlanProjectionV2
 */
/// `ag.list_plans` 是兼容查询名；结果只来自当前 route revision，不读取旧 task_plans 执行字段。
pub(crate) fn route_list_plans(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let ws = active_ws(state)?;
    let task_run_id = str_param(request, "taskRunId", "AG_TASK_RUN_ID_REQUIRED")?;
    let conn = lock_db(state)?;
    let run = ag::task::load_task_run(&conn, &task_run_id)
        .map_err(|e| IpcError::new("AG_TASK_DB_ERROR", &e.to_string(), true, false))?
        .ok_or_else(|| IpcError::new("AG_TASK_NOT_FOUND", "TaskRun 不存在", false, false))?;
    if run.workspace_id != ws.workspace_id {
        return Err(IpcError::new(
            "AG_TASK_NOT_FOUND",
            "TaskRun 不属于 active workspace",
            false,
            false,
        ));
    }
    let plans = ag::task::list_plans_by_run(&conn, &ws.workspace_id, &run.session_id, &task_run_id)
        .map_err(|e| IpcError::new("AG_TASK_DB_ERROR", &e.to_string(), true, false))?;
    ws_json(&plans)
}

#[cfg(test)]
mod ag_task_plan_runtime_tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    fn runtime_state_with_workspace(root: &std::path::Path) -> (AppState, String) {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::migrate(&mut conn).unwrap();
        conn.execute(
            "INSERT INTO workspaces (workspace_id, root_path, is_active, opened_at) VALUES (?1,?2,1,0)",
            rusqlite::params!["ws-runtime", root.to_string_lossy()],
        )
        .unwrap();
        let task_run_id =
            ag::task::begin_turn(&conn, "ws-runtime", "session-runtime-effect", None).unwrap();
        (
            AppState {
                db: Mutex::new(conn),
                registry: crate::build_registry(),
                assist_terminals: Mutex::new(HashMap::new()),
                active_agent_turns: Arc::new(Mutex::new(HashMap::new())),
            },
            task_run_id,
        )
    }

    fn park_runtime_decision(
        state: &AppState,
        task_run_id: &str,
        turn_id: &str,
        decision_request_id: &str,
    ) {
        let (lease, _, _, _, _) = ag::turn_control::register_active_agent_turn(
            state,
            "ws-runtime",
            "session-runtime-effect",
            turn_id,
            ag::turn_control::AgentTurnResumeKind::New,
        )
        .unwrap();
        ag::turn_control::park_agent_turn_for_decision(
            state,
            "session-runtime-effect",
            turn_id,
            task_run_id,
            decision_request_id,
            ag::turn_control::AgentTurnRuntimeCheckpoint {
                intent: ag::context::IntentResultV2::unresolved(),
                initial_prompt: "runtime effect test".to_string(),
                continuation_messages: Vec::new(),
            },
        )
        .unwrap();
        drop(lease);
    }

    // covers: BR-AG-DATA-003
    // covers: BR-SYS-STATE-008
    #[test]
    fn scope_decision_route_freezes_normalized_existing_workspace_targets() {
        let root = std::env::temp_dir().join(format!(
            "binder-scope-decision-route-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("b.md"), "b").unwrap();
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::migrate(&mut conn).unwrap();
        conn.execute(
            "INSERT INTO workspaces (workspace_id, root_path, is_active, opened_at) VALUES (?1,?2,1,0)",
            rusqlite::params!["ws-runtime", root.to_string_lossy()],
        )
        .unwrap();
        let task_run_id = ag::task::begin_turn(&conn, "ws-runtime", "session-scope", None).unwrap();
        let invalid_task_run_id =
            ag::task::begin_turn(&conn, "ws-runtime", "session-invalid-scope", None).unwrap();
        let state = AppState {
            db: Mutex::new(conn),
            registry: crate::build_registry(),
            assist_terminals: Mutex::new(HashMap::new()),
            active_agent_turns: Arc::new(Mutex::new(HashMap::new())),
        };

        let request = CommandRequest {
            command: "ag.propose_decision".to_string(),
            correlation_id: "scope-route".to_string(),
            params: serde_json::json!({
                "taskRunId": task_run_id,
                "kind": "scope",
                "payload": { "targetPaths": ["./b.md", "b.md"] }
            }),
        };
        route_propose_decision(&request, &state).unwrap();
        let conn = state.db.lock().unwrap();
        let pending = ag::task::pending_decision(&conn, &task_run_id)
            .unwrap()
            .unwrap();
        assert_eq!(pending.payload["targetPaths"], serde_json::json!(["b.md"]));
        drop(conn);

        let invalid = CommandRequest {
            command: "ag.propose_decision".to_string(),
            correlation_id: "scope-route-invalid".to_string(),
            params: serde_json::json!({
                "taskRunId": invalid_task_run_id,
                "kind": "scope",
                "payload": { "targetPaths": ["missing.md"] }
            }),
        };
        assert_eq!(
            route_propose_decision(&invalid, &state).unwrap_err().code,
            "AG_EDIT_TARGET_NOT_FOUND"
        );
        let conn = state.db.lock().unwrap();
        assert_eq!(
            ag::task::load_task_run(&conn, &invalid_task_run_id)
                .unwrap()
                .unwrap()
                .status,
            ag::task::TaskRunStatus::Active
        );
        drop(conn);
        std::fs::remove_dir_all(root).unwrap();
    }

    // covers: BR-AG-DATA-003
    #[test]
    fn completed_create_document_bundle_authorizes_only_after_resource_exists() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::migrate(&mut conn).unwrap();
        let task_run_id =
            ag::task::begin_turn(&conn, "ws-runtime", "session-bundle", None).unwrap();
        let root = std::env::temp_dir().join(format!(
            "binder-completed-bundle-target-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let bundle = crate::types::CreateDocumentBundle {
            plan_id: "plan-bundle".to_string(),
            workspace_id: "ws-runtime".to_string(),
            target: crate::types::DocRef {
                workspace_id: "ws-runtime".to_string(),
                rel_path: "new/report.md".to_string(),
                doc_type: "md".to_string(),
            },
            initial_content_ref: "content-ref".to_string(),
            operation_id: Some("operation-bundle".to_string()),
            initial_diff_id: Some("diff-bundle".to_string()),
            rejection_guard_ref: Some("guard-bundle".to_string()),
            status: "completed".to_string(),
        };

        assert!(persist_completed_create_document_target(
            &conn,
            &root,
            &task_run_id,
            "plan-bundle",
            &bundle,
        )
        .is_err());
        assert!(ag::target_set::load_for_task_run(&conn, &task_run_id)
            .unwrap()
            .is_empty());

        std::fs::create_dir_all(root.join("new")).unwrap();
        std::fs::write(root.join("new/report.md"), "# Report").unwrap();
        persist_completed_create_document_target(
            &conn,
            &root,
            &task_run_id,
            "plan-bundle",
            &bundle,
        )
        .unwrap();
        assert_eq!(
            ag::target_set::load_for_task_run(&conn, &task_run_id)
                .unwrap()
                .paths(),
            vec!["new/report.md"]
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    // covers: BR-AG-STATE-002, BR-SYS-STATE-008, BR-WS-STATE-003
    #[test]
    fn confirmed_workspace_operation_recomputes_runtime_grant_before_ws_revalidation() {
        let root = std::env::temp_dir().join(format!(
            "binder-runtime-effect-grant-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("delete-me.md"), "body").unwrap();
        let (state, task_run_id) = runtime_state_with_workspace(&root);
        let decision_request_id = {
            let conn = state.db.lock().unwrap();
            let workspace = crate::types::WorkspaceRef {
                workspace_id: "ws-runtime".to_string(),
                root_path: root.to_string_lossy().to_string(),
                is_active: true,
            };
            let plan = workspace::operation::plan_operation(
                &conn,
                &workspace,
                &[workspace::operation::WorkspaceOperationIntent {
                    kind: crate::types::WorkspaceOperationKind::DeleteResource,
                    source_path: Some("delete-me.md".to_string()),
                    target_path: None,
                    resource_kind: Some(crate::types::WorkspaceResourceKind::File),
                    recursive: false,
                    restore_payload_ref: None,
                }],
                Some(&task_run_id),
                Some("turn-runtime-effect"),
                None,
            )
            .unwrap();
            let decision = ag::task::propose_decision(
                &conn,
                &task_run_id,
                "permission",
                serde_json::json!({"workspaceOperationPlanId": plan.plan_id}),
            )
            .unwrap();
            workspace::operation::bind_decision_request(&conn, &plan.plan_id, &decision.request_id)
                .unwrap();
            decision.request_id
        };
        park_runtime_decision(
            &state,
            &task_run_id,
            "turn-runtime-effect",
            &decision_request_id,
        );

        assert!(root.join("delete-me.md").exists());
        let response = route_resolve_decision(
            &CommandRequest {
                command: "ag.resolve_decision".to_string(),
                correlation_id: "runtime-effect-confirm".to_string(),
                params: serde_json::json!({
                    "requestId": decision_request_id,
                    "outcome": "confirm"
                }),
            },
            &state,
        )
        .unwrap();
        assert!(!root.join("delete-me.md").exists());
        assert!(!response["workspace_operation_receipt"].is_null());
        std::fs::remove_dir_all(root).unwrap();
    }

    // covers: BR-AG-STATE-002, BR-SYS-STATE-007, BR-SYS-STATE-008
    #[test]
    fn cancelled_awaiting_turn_rejects_workspace_effect_before_decision_resolution() {
        let root = std::env::temp_dir().join(format!(
            "binder-runtime-effect-cancelled-turn-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("keep.md"), "body").unwrap();
        let (state, task_run_id) = runtime_state_with_workspace(&root);
        let decision_request_id = {
            let conn = state.db.lock().unwrap();
            let workspace = crate::types::WorkspaceRef {
                workspace_id: "ws-runtime".to_string(),
                root_path: root.to_string_lossy().to_string(),
                is_active: true,
            };
            let plan = workspace::operation::plan_operation(
                &conn,
                &workspace,
                &[workspace::operation::WorkspaceOperationIntent {
                    kind: crate::types::WorkspaceOperationKind::DeleteResource,
                    source_path: Some("keep.md".to_string()),
                    target_path: None,
                    resource_kind: Some(crate::types::WorkspaceResourceKind::File),
                    recursive: false,
                    restore_payload_ref: None,
                }],
                Some(&task_run_id),
                Some("turn-cancelled-effect"),
                None,
            )
            .unwrap();
            let decision = ag::task::propose_decision(
                &conn,
                &task_run_id,
                "permission",
                serde_json::json!({"workspaceOperationPlanId": plan.plan_id}),
            )
            .unwrap();
            workspace::operation::bind_decision_request(&conn, &plan.plan_id, &decision.request_id)
                .unwrap();
            decision.request_id
        };
        park_runtime_decision(
            &state,
            &task_run_id,
            "turn-cancelled-effect",
            &decision_request_id,
        );
        ag::turn_control::cancel_active_agent_turn(
            &state,
            "session-runtime-effect",
            "turn-cancelled-effect",
        )
        .unwrap();

        let error = route_resolve_decision(
            &CommandRequest {
                command: "ag.resolve_decision".to_string(),
                correlation_id: "runtime-effect-cancelled".to_string(),
                params: serde_json::json!({
                    "requestId": decision_request_id,
                    "outcome": "confirm"
                }),
            },
            &state,
        )
        .unwrap_err();
        assert_eq!(error.code, "AG_TURN_CANCELLED");
        assert!(root.join("keep.md").exists());
        let conn = state.db.lock().unwrap();
        let decision = ag::task::pending_decision(&conn, &task_run_id)
            .unwrap()
            .unwrap();
        assert_eq!(decision.request_id, decision_request_id);
        assert_eq!(
            decision.status,
            crate::types::DecisionRequestStatus::Proposed
        );
        drop(conn);
        std::fs::remove_dir_all(root).unwrap();
    }

    // covers: BR-AG-STATE-002, BR-SYS-STATE-008
    #[test]
    fn workspace_operation_without_live_awaiting_turn_fails_before_decision_resolution() {
        let root = std::env::temp_dir().join(format!(
            "binder-runtime-effect-no-turn-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("keep.md"), "body").unwrap();
        let (state, task_run_id) = runtime_state_with_workspace(&root);
        let decision_request_id = {
            let conn = state.db.lock().unwrap();
            let workspace = crate::types::WorkspaceRef {
                workspace_id: "ws-runtime".to_string(),
                root_path: root.to_string_lossy().to_string(),
                is_active: true,
            };
            let plan = workspace::operation::plan_operation(
                &conn,
                &workspace,
                &[workspace::operation::WorkspaceOperationIntent {
                    kind: crate::types::WorkspaceOperationKind::DeleteResource,
                    source_path: Some("keep.md".to_string()),
                    target_path: None,
                    resource_kind: Some(crate::types::WorkspaceResourceKind::File),
                    recursive: false,
                    restore_payload_ref: None,
                }],
                Some(&task_run_id),
                Some("turn-no-longer-active"),
                None,
            )
            .unwrap();
            let decision = ag::task::propose_decision(
                &conn,
                &task_run_id,
                "permission",
                serde_json::json!({"workspaceOperationPlanId": plan.plan_id}),
            )
            .unwrap();
            workspace::operation::bind_decision_request(&conn, &plan.plan_id, &decision.request_id)
                .unwrap();
            decision.request_id
        };
        let error = route_resolve_decision(
            &CommandRequest {
                command: "ag.resolve_decision".to_string(),
                correlation_id: "runtime-effect-denied".to_string(),
                params: serde_json::json!({
                    "requestId": decision_request_id,
                    "outcome": "confirm"
                }),
            },
            &state,
        )
        .unwrap_err();
        assert_eq!(error.code, "AG_TURN_NOT_AWAITING");
        assert!(root.join("keep.md").exists());
        let conn = state.db.lock().unwrap();
        let decision = ag::task::pending_decision(&conn, &task_run_id)
            .unwrap()
            .unwrap();
        assert_eq!(decision.request_id, decision_request_id);
        assert_eq!(
            decision.status,
            crate::types::DecisionRequestStatus::Proposed
        );
        drop(conn);
        std::fs::remove_dir_all(root).unwrap();
    }

    // covers: BR-SYS-STATE-008
    // TaskRun 创建后的统一失败出口必须记录 Failed 投影事实。
    #[test]
    fn started_turn_failure_settles_task_run() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::migrate(&mut conn).unwrap();
        let task_run_id =
            ag::task::begin_turn(&conn, "ws-runtime", "session-runtime", None).unwrap();

        settle_task_run(
            &conn,
            &task_run_id,
            "turn-runtime",
            ag::task::TurnOutcome::Failed,
        );

        assert_eq!(
            ag::task::load_task_run(&conn, &task_run_id)
                .unwrap()
                .unwrap()
                .status,
            ag::task::TaskRunStatus::Failed,
        );
    }
}

pub(super) fn edit_intent_has_terminal_tool_result(
    proposed_diff: bool,
    edit_blocked_reported: bool,
) -> bool {
    proposed_diff || edit_blocked_reported
}

/// 回合终态先更新 Decision/续片段控制缓存，再追加 task_status_observed；展示层只消费后者。
pub(super) fn settle_task_run(
    conn: &rusqlite::Connection,
    task_run_id: &str,
    turn_id: &str,
    outcome: ag::task::TurnOutcome,
) -> Option<ag::task::TaskRunStatus> {
    let _ = ag::task::link_turn(conn, turn_id, task_run_id);
    if let Ok(status) = ag::task::settle_turn(conn, task_run_id, outcome) {
        let status_name = match status {
            ag::task::TaskRunStatus::Active => "active",
            ag::task::TaskRunStatus::Waiting => "waiting",
            ag::task::TaskRunStatus::Completed => "completed",
            ag::task::TaskRunStatus::Failed => "failed",
            ag::task::TaskRunStatus::Cancelled => "cancelled",
            ag::task::TaskRunStatus::Interrupted => "interrupted",
        };
        let _ = ag::task::record_projection_fact(
            conn,
            turn_id,
            "task_status_observed",
            serde_json::json!({"taskRunId": task_run_id, "status": status_name}),
        );
        return Some(status);
    }
    None
}
