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
#[cfg(test)]
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
    let mut conn = lock_db(state)?;
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
        drop(conn);
        let candidates = ag::target_set::validated_scope_candidates(
            std::path::Path::new(&ws.root_path),
            &raw_paths,
            &current,
        )?;
        conn = lock_db(state)?;
        let latest = ag::task::load_task_run(&conn, &task_run_id)
            .map_err(|error| IpcError::new("AG_TASK_DB_ERROR", &error.to_string(), true, false))?;
        let active = workspace::lifecycle::active_workspace(&conn)
            .map_err(|error| IpcError::new("AG_TASK_DB_ERROR", &error.to_string(), true, false))?;
        let targets = ag::target_set::load_for_task_run(&conn, &task_run_id)
            .map_err(|error| IpcError::new("AG_TASK_DB_ERROR", &error.to_string(), true, false))?;
        if latest.as_ref().is_none_or(|latest| {
            latest.workspace_id != run.workspace_id
                || latest.session_id != run.session_id
                || latest.status != run.status
        }) || active.as_ref().is_none_or(|active| {
            active.workspace_id != ws.workspace_id || active.root_path != ws.root_path
        }) || targets != current
        {
            return Err(IpcError::new(
                "AG_DECISION_CONTEXT_CHANGED",
                "路径校验期间任务或工作区已变化，未创建裁决请求",
                false,
                false,
            ));
        }
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
    resolve_decision_with_executor(
        request,
        state,
        workspace::operation::execute_operation_plan_shared,
    )
}

fn resolve_decision_with_executor(
    request: &CommandRequest,
    state: &AppState,
    executor: impl FnOnce(
        &std::sync::Mutex<rusqlite::Connection>,
        &crate::types::WorkspaceRef,
        &str,
        &str,
    ) -> Result<crate::types::WorkspaceOperationReceipt, IpcError>,
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
    let mut conn = lock_db(state)?;
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
        if decision_status == "resolved" && outcome == "confirm" {
            let receipt = workspace::operation::load_outcome(&conn, plan_id)?.ok_or_else(|| {
                IpcError::new(
                    "AG_EFFECT_PENDING",
                    "裁决已提交，资源效果尚待结算；请查询原回合，不得再次执行",
                    false,
                    false,
                )
            })?;
            let run = ag::task::load_task_run(&conn, &decision_task_run_id)
                .map_err(|error| {
                    IpcError::new("AG_TASK_DB_ERROR", &error.to_string(), true, false)
                })?
                .ok_or_else(|| {
                    IpcError::new("AG_TASK_NOT_FOUND", "TaskRun 不存在", false, false)
                })?;
            return ws_json(&serde_json::json!({
                "task_run_id": decision_task_run_id, "status": run.status,
                "workspace_operation_receipt": receipt,
                "workspace_operation_items": plan.operation_items,
                "create_document_bundle": null, "already_resolved": true,
            }));
        }
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
    let confirmed_scope_targets = if decision_status != "resolved"
        && decision_kind == "scope"
        && outcome == "confirm"
    {
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
        let current = ag::target_set::load_for_task_run(&conn, &decision_task_run_id)
            .map_err(|error| IpcError::new("AG_TASK_DB_ERROR", &error.to_string(), true, false))?;
        let frozen_task: (String, String, String) = conn
            .query_row(
                "SELECT workspace_id,session_id,status FROM task_runs WHERE task_run_id=?1",
                [&decision_task_run_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|error| IpcError::new("AG_TASK_DB_ERROR", &error.to_string(), true, false))?;
        drop(conn);
        let candidates = ag::target_set::validated_scope_candidates(
            std::path::Path::new(&ws.root_path),
            &raw_paths,
            &current,
        )?;
        conn = lock_db(state)?;
        let latest: (String,String,String,Option<String>) = conn.query_row("SELECT task_run_id,payload_json,status,outcome FROM decision_requests WHERE request_id=?1", [&request_id], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?)))
                .map_err(|error| IpcError::new("AG_TASK_DB_ERROR", &error.to_string(), true, false))?;
        let latest_task: (String, String, String) = conn
            .query_row(
                "SELECT workspace_id,session_id,status FROM task_runs WHERE task_run_id=?1",
                [&decision_task_run_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|error| IpcError::new("AG_TASK_DB_ERROR", &error.to_string(), true, false))?;
        let active = workspace::lifecycle::active_workspace(&conn)
            .map_err(|error| IpcError::new("AG_TASK_DB_ERROR", &error.to_string(), true, false))?;
        let targets = ag::target_set::load_for_task_run(&conn, &decision_task_run_id)
            .map_err(|error| IpcError::new("AG_TASK_DB_ERROR", &error.to_string(), true, false))?;
        if latest
            != (
                decision_task_run_id.clone(),
                decision_payload_json.clone(),
                decision_status.clone(),
                resolved_outcome.clone(),
            )
            || latest_task != frozen_task
            || targets != current
            || active.as_ref().is_none_or(|active| {
                active.workspace_id != ws.workspace_id || active.root_path != ws.root_path
            })
        {
            return Err(IpcError::new(
                "AG_DECISION_CONTEXT_CHANGED",
                "路径校验期间裁决或任务已变化，未授权目标",
                false,
                false,
            ));
        }
        Some(candidates)
    } else {
        None
    };
    // 先取得原 AgentTurn 的效果租约；已开始的资源执行独立结算，不阻塞取消终态。
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
    if let Some((plan, session_id)) = operation_plan {
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
            drop(conn);
            let execute = || executor(&state.db, &ws, &plan_id, &request_id);
            let executed = if already_resolved {
                execute()
            } else {
                super::capability_runtime::record_owned_execution(
                    state,
                    crate::types::ToolCallReceiptIdentityV1 {
                        workspace_id: ws.workspace_id.clone(),
                        session_id,
                        turn_id: source_turn_id.into(),
                        tool_call_id: format!("decision:{request_id}"),
                        tool_name: "execute_workspace_operation".into(),
                        capability_id: "workspace_operation".into(),
                        grant_id: format!("decision:{request_id}:{computed_at_seq}"),
                        attempt: 1,
                    },
                    execute,
                )
            };
            let mut conn = lock_db(state)?;
            match executed {
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
                            drop(conn);
                            let target = ag::target_set::normalize_existing_target(
                                std::path::Path::new(&ws.root_path),
                                &bundle.target.rel_path,
                            )?;
                            conn = lock_db(state)?;
                            // 取消或切换任务之后仍保留操作证据，但不新增编辑授权。
                            let run =
                                ag::task::load_task_run(&conn, &task_run_id).map_err(|error| {
                                    IpcError::new(
                                        "AG_TASK_DB_ERROR",
                                        &error.to_string(),
                                        true,
                                        false,
                                    )
                                })?;
                            let active =
                                workspace::lifecycle::active_workspace(&conn).map_err(|error| {
                                    IpcError::new(
                                        "AG_TASK_DB_ERROR",
                                        &error.to_string(),
                                        true,
                                        false,
                                    )
                                })?;
                            if active.as_ref().is_some_and(|active| {
                                active.workspace_id == ws.workspace_id
                                    && active.root_path == ws.root_path
                            }) && run.as_ref().is_some_and(|run| {
                                run.workspace_id == ws.workspace_id
                                    && matches!(
                                        run.status,
                                        ag::task::TaskRunStatus::Active
                                            | ag::task::TaskRunStatus::Waiting
                                    )
                            }) {
                                ag::target_set::append_task_run_targets(
                                    &conn,
                                    &task_run_id,
                                    &[target],
                                    ag::target_set::SOURCE_CREATE_DOCUMENT_BUNDLE,
                                    Some(&plan_id),
                                )
                                .map_err(|error| {
                                    IpcError::new(
                                        "AG_TASK_DB_ERROR",
                                        &error.to_string(),
                                        true,
                                        false,
                                    )
                                })?;
                                if !already_resolved {
                                    create_document_bundle = Some(bundle);
                                }
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
                db: std::sync::Arc::new(Mutex::new(conn)),
                registry: crate::build_registry(),
                assist_terminals: std::sync::Arc::new(Mutex::new(HashMap::new())),
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
        {
            let conn = state.db.lock().unwrap();
            ag::history::ensure_session(
                &conn,
                "ws-runtime",
                "session-runtime-effect",
                "runtime",
                "edit",
                0,
                true,
            )
            .unwrap();
            let payload = serde_json::from_value(serde_json::json!({
                "version": 1, "lastEventSeq": 0,
                "userMessage": {"id": "request", "role": "user", "content": "resource operation"},
                "toolSummaries": [], "diffRefs": []
            }))
            .unwrap();
            ag::history::save_turn(
                &conn,
                "ws-runtime",
                "session-runtime-effect",
                turn_id,
                None,
                "draft",
                &payload,
            )
            .unwrap();
        }
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
            db: std::sync::Arc::new(Mutex::new(conn)),
            registry: crate::build_registry(),
            assist_terminals: std::sync::Arc::new(Mutex::new(HashMap::new())),
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
        for cancel_during_execution in [false, true] {
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
                workspace::operation::bind_decision_request(
                    &conn,
                    &plan.plan_id,
                    &decision.request_id,
                )
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
            let request = CommandRequest {
                command: "ag.resolve_decision".to_string(),
                correlation_id: "runtime-effect-confirm".to_string(),
                params: serde_json::json!({
                    "requestId": decision_request_id,
                    "outcome": "confirm"
                }),
            };
            let response =
                resolve_decision_with_executor(&request, &state, |db, ws, plan, decision| {
                    assert!(
                        db.try_lock().is_ok(),
                        "executor must not inherit the shared database lock"
                    );
                    assert_eq!(
                        route_resolve_decision(&request, &state).unwrap_err().code,
                        "AG_EFFECT_PENDING"
                    );
                    if cancel_during_execution {
                        let app = tauri::test::mock_builder()
                            .build(tauri::test::mock_context(tauri::test::noop_assets()))
                            .unwrap();
                        ag::turn_control::cancel_active_agent_turn(
                            &state,
                            "session-runtime-effect",
                            "turn-runtime-effect",
                        )?;
                        ag::turn_events::persist_parked_ag_turn_cancelled(
                            &state,
                            app.handle(),
                            "turn-runtime-effect",
                            ag::turn::TurnState::AwaitingDecision,
                        )?;
                        let conn = db.lock().unwrap();
                        settle_task_run(
                            &conn,
                            &task_run_id,
                            "turn-runtime-effect",
                            ag::task::TurnOutcome::Cancelled,
                        )
                        .unwrap();
                    }
                    workspace::operation::execute_operation_plan_shared(db, ws, plan, decision)
                })
                .unwrap();
            assert!(!root.join("delete-me.md").exists());
            assert!(!response["workspace_operation_receipt"].is_null());
            let repeated = route_resolve_decision(&request, &state).unwrap();
            assert_eq!(repeated["already_resolved"], true);
            assert_eq!(
                repeated["workspace_operation_receipt"],
                response["workspace_operation_receipt"]
            );
            {
                let conn = state.db.lock().unwrap();
                let facts = ag::capability_runtime::facts(
                    &conn,
                    "ws-runtime",
                    Some(("session-runtime-effect", "turn-runtime-effect")),
                )
                .unwrap();
                assert_eq!(facts.len(), 2);
                let receipt = facts.iter().find_map(|fact| fact.receipt.as_ref()).unwrap();
                assert_eq!(
                    receipt.effect_status,
                    crate::types::ToolCallEffectStatus::ResourceExecuted
                );
                if cancel_during_execution {
                    assert_eq!(
                        conn.query_row::<String, _, _>(
                            "SELECT status FROM agent_turns WHERE turn_id='turn-runtime-effect'",
                            [],
                            |row| row.get(0)
                        )
                        .unwrap(),
                        "cancelled"
                    );
                    assert_eq!(
                        ag::task::load_task_run(&conn, &task_run_id)
                            .unwrap()
                            .unwrap()
                            .status,
                        ag::task::TaskRunStatus::Cancelled
                    );
                }
            }
            std::fs::remove_dir_all(root).unwrap();
        }
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
