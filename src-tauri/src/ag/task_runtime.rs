//! TaskRun、DecisionRequest 与 TaskPlan 的 command/runtime 编排。

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
 * boundary: in=AgentSessionIdentity | out=PersistedTaskRunProjections
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
 * boundary: in=TaskRunDecisionRequestProposalAndActiveWorkspace | out=PersistedDecisionRequestWithNormalizedFrozenScopeAndWaitingConfirmTaskRun | does_not=persistMissingOutsideWorkspaceOrAlreadyAuthorizedScopeTargets
 */
/// AG-1.0-TASKRUN-REWORK-IMP-RUNTIME-01：`ag.propose_decision` —— 发起任务级裁决请求
/// （TERM-AG-017；kind：permission/plan_choice/scope/manual_confirm；置 run waiting_confirm）。
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
 * boundary: in=DecisionRequestOutcomeBoundToFrozenScopeOrWorkspaceOperationPlan | out=TaskRunStatusWorkspaceOperationReceiptAndPersistedExplicitEditTargetSet | does_not=executeUnconfirmedPlanOrTreatModelTextAsUserConfirmation
 */
/// AG-1.0-TASKRUN-REWORK-IMP-RUNTIME-01：`ag.resolve_decision` —— 任务级裁决
/// （userConfirm/userCancel 真实入口；confirm → Executing，cancel → Cancelled）。
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
    if let Some(plan_id) = operation_plan_id {
        if outcome == "confirm" {
            match workspace::operation::execute_operation_plan(&conn, &ws, &plan_id, &request_id) {
                Ok(receipt) => {
                    if receipt.final_status != crate::types::WorkspaceOperationStatus::Completed {
                        status = ag::task::settle_turn(
                            &conn,
                            &task_run_id,
                            ag::task::TurnOutcome::Failed,
                        )
                        .map_err(|e| {
                            IpcError::new("AG_TASK_DB_ERROR", &e.to_string(), true, false)
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
                    let _ =
                        ag::task::settle_turn(&conn, &task_run_id, ag::task::TurnOutcome::Failed);
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
 * boundary: in=TaskRunIdentity | out=PersistedTaskPlanProjections
 */
/// AG-1.0-TASKRUN-REWORK-CONTINUE-01：`ag.list_plans` —— 按 task_run_id 列出任务计划
/// （TERM-AG-018，可展示操作计划：步骤/依赖/游标/授权/确认点）。
pub(crate) fn route_list_plans(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let task_run_id = str_param(request, "taskRunId", "AG_TASK_RUN_ID_REQUIRED")?;
    let conn = lock_db(state)?;
    let plans = ag::task::list_plans_by_run(&conn, &task_run_id)
        .map_err(|e| IpcError::new("AG_TASK_DB_ERROR", &e.to_string(), true, false))?;
    ws_json(&plans)
}

/**
 * @GOV
 * codes:    BR-SYS-STATE-008-EFFECT-AG-AG-TURN-028
 * type:     EFFECT
 * chain:    AG-TURN
 * rules:    BR-SYS-STATE-008
 * term_ref: TERM-AG-015, TERM-AG-018
 * boundary: in=TaskPlanV1Registration | out=PersistedTaskPlan
 */
/// AG-1.0-TASKRUN-REWORK-CONTINUE-01：`ag.create_plan` —— 登记任务计划
/// （供工具/后续 IntentResult 复杂目标接线调用；plan 为 TaskPlanV1 JSON）。
pub(crate) fn route_create_plan(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let plan: crate::types::TaskPlanV1 = request
        .params
        .get("plan")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .ok_or_else(|| {
            IpcError::new(
                "AG_PLAN_INVALID",
                "plan 参数须为 TaskPlanV1 JSON",
                false,
                false,
            )
        })?;
    let conn = lock_db(state)?;
    ag::task::create_plan(&conn, &plan)
        .map_err(|e| IpcError::new("AG_TASK_DB_ERROR", &e.to_string(), true, false))?;
    ws_json(&serde_json::json!({ "ok": true }))
}

#[cfg(test)]
mod ag_task_plan_runtime_tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

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
            crate::types::TaskRunStatus::Executing
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

    // covers: BR-SYS-STATE-008
    // TaskRun 创建后的统一失败出口必须把活动态收束为 Failed，不能遗留 executing。
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
            crate::types::TaskRunStatus::Failed,
        );
    }

    // covers: BR-AG-DATA-002, BR-SYS-STATE-008
    // RuntimeControl 只注入可审计的执行计划事实，明确当前就绪步骤，不注入模型推理或伪造完成态。
    #[test]
    fn task_plan_runtime_projection_exposes_only_current_ready_step() {
        let plan = build_task_plan("task-runtime", &["a.md".to_string(), "b.md".to_string()]);
        let projection = task_plan_runtime_projection(&plan);
        assert_eq!(projection["planId"], "plan-task-runtime-1");
        assert_eq!(projection["cursor"], 0);
        assert_eq!(projection["currentReadyStep"]["stepId"], "read-0");
        assert_eq!(projection["currentReadyStep"]["requiredTool"], "read_doc");
        assert_eq!(projection["steps"].as_array().unwrap().len(), 4);
        assert!(projection["executionRule"]
            .as_str()
            .unwrap()
            .contains("Tool success or failure"));
    }
}

pub(super) fn edit_intent_has_terminal_tool_result(
    proposed_diff: bool,
    edit_blocked_reported: bool,
) -> bool {
    proposed_diff || edit_blocked_reported
}

/// AG-1.0-IMP-TASKRUN-01：回合终态映射 TaskRun（SM-AG-04 settle，BR-SYS-STATE-008）。
/// best-effort（DB 失败不阻塞回合）；回合→TaskRun 关联写入 agent_turns.task_run_id（UNC-09）。
/// AG-1.0-TASKRUN-REWORK-IMP-RUNTIME-01：settle 不再接收"有提案"参数——WaitingConfirm 唯一来源
/// 是未决 DecisionRequest（三层分离，A-AG-M-S-01 §2.2.2）。
pub(super) fn settle_task_run(
    conn: &rusqlite::Connection,
    task_run_id: &str,
    turn_id: &str,
    outcome: ag::task::TurnOutcome,
) {
    let _ = ag::task::settle_turn(conn, task_run_id, outcome);
    let _ = ag::task::link_turn(conn, turn_id, task_run_id);
}

/// AG-1.0-TASKRUN-REWORK-INTENT-01：按目标文档构造可展示 TaskPlan（TERM-AG-018）。
/// 每文档一对步骤：读取（read_doc）→ 生成修改建议（propose_edit，确认点）；步骤依赖 read→propose。
/// 计划不预授权；执行顺序和真实进度由工具调用/ToolResult 驱动。
pub(super) fn build_task_plan(task_run_id: &str, paths: &[String]) -> crate::types::TaskPlanV1 {
    use crate::types::{TaskPlanStepV1, TaskPlanV1, TaskStepStatus};
    let now = now_millis();
    let mut steps = Vec::new();
    let mut seq = 0i32;
    for (i, p) in paths.iter().enumerate() {
        let read_id = format!("read-{i}");
        steps.push(TaskPlanStepV1 {
            step_id: read_id.clone(),
            seq,
            kind: "read".to_string(),
            label: format!("读取 {p}"),
            deps: Vec::new(),
            required_grant: Some("read_doc".to_string()),
            confirm_point: false,
            status: TaskStepStatus::Pending,
        });
        seq += 1;
        steps.push(TaskPlanStepV1 {
            step_id: format!("propose-{i}"),
            seq,
            kind: "propose".to_string(),
            label: format!("生成修改建议：{p}"),
            deps: vec![read_id],
            required_grant: Some("propose_edit".to_string()),
            confirm_point: true,
            status: TaskStepStatus::Pending,
        });
        seq += 1;
    }
    TaskPlanV1 {
        plan_id: format!("plan-{task_run_id}-1"),
        task_run_id: task_run_id.to_string(),
        steps,
        cursor: 0,
        created_at: now,
        updated_at: now,
    }
}

/// 将持久化 TaskPlan 投影到受信 RuntimeControl：只提供步骤、cursor 和当前可执行步骤，
/// 不包含模型思维链，也不产生 CapabilityGrant 或文档状态变化。
/**
 * @GOV
 * codes:    BR-AG-DATA-002-DATA-AG-AG-TURN-017, BR-SYS-STATE-008-DATA-AG-AG-TURN-017
 * type:     DATA
 * chain:    AG-TURN
 * rules:    BR-AG-DATA-002, BR-SYS-STATE-008
 * term_ref: TERM-AG-018
 * boundary: in=PersistedTaskPlan | out=RuntimeControlTaskPlanProjection(currentReadyStepOnly) | consumer=TaskPlanV1
 */
pub(super) fn task_plan_runtime_projection(plan: &crate::types::TaskPlanV1) -> serde_json::Value {
    let current_ready_step = ag::task::current_ready_plan_step(plan).map(|step| {
        serde_json::json!({
            "stepId": step.step_id,
            "seq": step.seq,
            "kind": step.kind,
            "label": step.label,
            "requiredTool": step.required_grant,
            "confirmPoint": step.confirm_point,
        })
    });
    serde_json::json!({
        "planId": plan.plan_id,
        "cursor": plan.cursor,
        "steps": plan.steps,
        "currentReadyStep": current_ready_step,
        "executionRule": "Only the currentReadyStep may advance the plan. Tool success or failure is the only source of step state; do not claim completion in text.",
    })
}
