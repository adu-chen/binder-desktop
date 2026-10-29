//! SM-AG-04 TaskRun/TaskPlan 事件投影与 DecisionRequest 控制事实。
//!
//! `task_runs` 只保留稳定身份和 Decision/续片段所需控制态；用户可见 TaskRun/TaskPlan/TaskStep
//! 一律从 append-only TurnEvent 重建，不含执行游标、计划授权或调度入口。

use crate::types::{
    DecisionRequestStatus, DecisionRequestV1, TaskPlanProjectionV2, TaskRunProjectionStatusV2,
    TaskRunProjectionV2, TaskStepProjectionStatusV2, TaskStepProjectionV2,
};
use rusqlite::{Connection, OptionalExtension};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// task_run_id 生成序号：保证同一毫秒内多次新建不冲突（task_runs.task_run_id 唯一约束）。
static TASK_SEQ: AtomicU64 = AtomicU64::new(0);

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TaskRunStatus {
    Active,
    Waiting,
    Completed,
    Failed,
    Cancelled,
    Interrupted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaskRunControl {
    pub task_run_id: String,
    pub workspace_id: String,
    pub session_id: String,
    pub status: TaskRunStatus,
    pub intent_summary: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 回合终态（lib.rs 回合循环映射）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnOutcome {
    Completed,
    Failed,
    Cancelled,
}

/// 已确认的编辑器预览基线。它只作为 AgentTurn 续片段的控制态，绝不携带正文、Diff payload
/// 或用户裁决结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewCheckpoint {
    pub diff_id: String,
    pub task_run_id: String,
    pub workspace_id: String,
    pub session_id: String,
    pub turn_id: String,
    pub rel_path: String,
    pub session_version: u64,
    pub content_hash: String,
}

/**
 * @GOV
 * codes:    BR-SYS-STATE-008-EFFECT-AG-AG-TASK-002
 * type:     EFFECT
 * chain:    AG-TURN, AG-TOOL
 * rules:    BR-SYS-STATE-008
 * term_ref: TERM-AG-015, TERM-AG-018
 * boundary: in=TurnEventBackedComplexRouteOrTaskLevelWaitingPointAndAgentSessionIdentity | out=StableTaskRunProjectionIdentity(rejectsUnfinishedRunWithoutExplicitContinuation) | does_not=createTaskRunForSimpleAgentTurnScheduleCandidateWorkOrSilentlyMergeUserGoals
 */
/// 为已有复杂路线或任务级等待事实取得稳定 task_run_id。该行只服务身份关联和裁决控制，
/// 用户可见状态与步骤必须从 TurnEvent 重建。
/// - 会话内存在**非终态** TaskRun → 拒绝；复用只能经携带稳定 task_run_id 的显式 continuation；
/// - 否则新建 active 身份；不创建 TaskPlan 或候选执行项。
/// 调用方不得把普通 AgentTurn 开始本身当成创建依据，也不得凭 session 猜测两个请求属于同一目标。
pub fn begin_turn(
    conn: &Connection,
    workspace_id: &str,
    session_id: &str,
    intent_summary: Option<&str>,
) -> rusqlite::Result<String> {
    if let Some(id) = latest_non_terminal_id(conn, workspace_id, session_id)? {
        return Err(rusqlite::Error::InvalidParameterName(format!(
            "unfinished task run requires explicit continuation: {id}"
        )));
    }
    let id = format!(
        "task-{}-{}-{}",
        workspace_id,
        now_millis(),
        TASK_SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let now = now_millis();
    conn.execute(
        "INSERT INTO task_runs (task_run_id, workspace_id, session_id, status, intent_summary, steps_json, created_at, updated_at) \
         VALUES (?1, ?2, ?3, 'active', ?4, '[]', ?5, ?5)",
        rusqlite::params![id, workspace_id, session_id, intent_summary, now],
    )?;
    Ok(id)
}

/**
 * @GOV
 * codes:    BR-SYS-STATE-008-EFFECT-AG-AG-TURN-032, BR-AG-DATA-001-EFFECT-AG-AG-TURN-032
 * type:     EFFECT
 * chain:    AG-TURN, DE-DIFF, ED-EDIT
 * rules:    BR-SYS-STATE-008, BR-AG-DATA-001
 * term_ref: TERM-AG-015, TERM-ED-008
 * boundary: in=LogicalStateDiffProposalIdentity | out=AwaitingEditorPreviewCheckpoint(noModelContinuationOrDiskWrite)
 */
/// 为一个会改变已有 LogicalState 的 pending Diff 建立续片段检查点。创建卡片不等于预览成功：
/// checkpoint 只能处于 awaiting_ack，直到编辑器回报实际 sessionVersion/contentHash。
pub fn create_preview_checkpoint(
    conn: &Connection,
    task_run_id: &str,
    workspace_id: &str,
    session_id: &str,
    turn_id: &str,
    diff_id: &str,
    rel_path: &str,
) -> rusqlite::Result<()> {
    let run_matches: Option<String> = conn
        .query_row(
            "SELECT task_run_id FROM task_runs WHERE task_run_id=?1 AND workspace_id=?2 AND session_id=?3 AND status IN ('active','executing')",
            rusqlite::params![task_run_id, workspace_id, session_id],
            |row| row.get(0),
        )
        .optional()?;
    if run_matches.is_none()
        && !confirmed_bundle_checkpoint_owner(
            conn,
            task_run_id,
            workspace_id,
            session_id,
            turn_id,
            rel_path,
            None,
        )?
    {
        return Err(rusqlite::Error::InvalidParameterName(
            "preview checkpoint requires the active task run".to_string(),
        ));
    }
    conn.execute(
        "INSERT INTO task_preview_checkpoints \
         (diff_id, task_run_id, workspace_id, session_id, turn_id, rel_path, status, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'awaiting_ack', ?7)",
        rusqlite::params![
            diff_id,
            task_run_id,
            workspace_id,
            session_id,
            turn_id,
            rel_path,
            now_millis()
        ],
    )?;
    Ok(())
}

/**
 * @GOV
 * codes:    BR-SYS-STATE-008-GUARD-AG-AG-TURN-005, BR-AG-DATA-001-GUARD-AG-AG-TURN-005
 * type:     GUARD
 * chain:    AG-TURN, DE-DIFF, ED-EDIT
 * rules:    BR-SYS-STATE-008, BR-AG-DATA-001
 * term_ref: TERM-AG-015, TERM-ED-008
 * boundary: in=EditorAppliedLogicalStateVersionHash | out=AcknowledgedPreviewCheckpoint(rejectsWrongSessionTurnDiffPathOrTerminalDiff)
 */
/// 接收真实 ED LogicalState 预览回执。只接受对应 pending Diff 的第一份回执；模型文本、卡片显示态
/// 和旧快照均不能伪造此转移。
pub fn acknowledge_preview_checkpoint(
    conn: &Connection,
    checkpoint: &PreviewCheckpoint,
) -> rusqlite::Result<()> {
    let pending_diff: Option<String> = conn
        .query_row(
            "SELECT d.diff_id FROM pending_diffs d \
             JOIN task_runs r ON r.workspace_id=d.workspace_id \
             WHERE d.diff_id=?1 AND d.status='pending' AND r.task_run_id=?2 \
             AND d.workspace_id=?3 AND d.rel_path=?4 AND r.session_id=?5 AND r.status IN ('active','executing')",
            rusqlite::params![checkpoint.diff_id, checkpoint.task_run_id, checkpoint.workspace_id, checkpoint.rel_path, checkpoint.session_id],
            |row| row.get(0),
        )
        .optional()?;
    if pending_diff.is_none()
        && !confirmed_bundle_checkpoint_owner(
            conn,
            &checkpoint.task_run_id,
            &checkpoint.workspace_id,
            &checkpoint.session_id,
            &checkpoint.turn_id,
            &checkpoint.rel_path,
            Some(&checkpoint.diff_id),
        )?
    {
        return Err(rusqlite::Error::InvalidParameterName(
            "preview acknowledgement requires a pending diff".to_string(),
        ));
    }
    let changed = conn.execute(
        "UPDATE task_preview_checkpoints SET status='acknowledged', session_version=?1, content_hash=?2, acknowledged_at=?3 \
         WHERE diff_id=?4 AND task_run_id=?5 AND workspace_id=?6 AND session_id=?7 AND turn_id=?8 AND rel_path=?9 \
         AND status='awaiting_ack'",
        rusqlite::params![
            checkpoint.session_version as i64,
            checkpoint.content_hash,
            now_millis(),
            checkpoint.diff_id,
            checkpoint.task_run_id,
            checkpoint.workspace_id,
            checkpoint.session_id,
            checkpoint.turn_id,
            checkpoint.rel_path,
        ],
    )?;
    if changed != 1 {
        return Err(rusqlite::Error::InvalidParameterName(
            "preview acknowledgement does not match an awaiting checkpoint".to_string(),
        ));
    }
    Ok(())
}

// A confirmed resource operation can be active while its TaskRun projection is still waiting.
// Only the exact live turn/decision/plan/bundle may bridge that projection boundary.
fn confirmed_bundle_checkpoint_owner(
    conn: &Connection,
    run: &str,
    workspace: &str,
    session: &str,
    turn: &str,
    path: &str,
    diff: Option<&str>,
) -> rusqlite::Result<bool> {
    conn.query_row("SELECT EXISTS(SELECT 1 FROM task_runs r \
        JOIN agent_turns t ON t.task_run_id=r.task_run_id \
        JOIN workspace_operation_plans p ON p.task_run_id=r.task_run_id AND p.source_turn_id=t.turn_id \
        JOIN create_document_bundles b ON b.plan_id=p.plan_id AND b.operation_id=p.operation_id \
        JOIN decision_requests d ON d.request_id=p.decision_request_id AND d.task_run_id=r.task_run_id \
        WHERE r.task_run_id=?1 AND r.workspace_id=?2 AND r.session_id=?3 AND r.status IN ('waiting','active','executing') \
        AND t.turn_id=?4 AND t.session_id=?3 AND t.workspace_id=?2 AND t.status='draft' \
        AND p.workspace_id=?2 AND p.status IN ('completed','awaiting_content') \
        AND b.workspace_id=?2 AND b.target_rel_path=?5 AND b.status IN ('projecting_initial_diff','initial_diff_pending') \
        AND d.status='resolved' AND d.outcome='confirm' AND (?6 IS NULL OR (b.initial_diff_id=?6 \
        AND EXISTS(SELECT 1 FROM pending_diffs x WHERE x.diff_id=?6 AND x.workspace_id=?2 AND x.rel_path=?5 AND x.status='pending'))))",
        rusqlite::params![run,workspace,session,turn,path,diff], |row| row.get(0))
}

/// 读取已确认但尚未消费的 checkpoint，供续片段入口先比对新采集的 OpenDocSnapshot。
pub fn acknowledged_preview_checkpoint(
    conn: &Connection,
    task_run_id: &str,
    workspace_id: &str,
    session_id: &str,
    diff_id: &str,
) -> rusqlite::Result<Option<PreviewCheckpoint>> {
    conn.query_row(
        "SELECT diff_id, task_run_id, workspace_id, session_id, turn_id, rel_path, session_version, content_hash \
         FROM task_preview_checkpoints WHERE diff_id=?1 AND task_run_id=?2 AND workspace_id=?3 AND session_id=?4 \
         AND status='acknowledged'",
        rusqlite::params![diff_id, task_run_id, workspace_id, session_id],
        |row| {
            Ok(PreviewCheckpoint {
                diff_id: row.get(0)?,
                task_run_id: row.get(1)?,
                workspace_id: row.get(2)?,
                session_id: row.get(3)?,
                turn_id: row.get(4)?,
                rel_path: row.get(5)?,
                session_version: row.get::<_, i64>(6)? as u64,
                content_hash: row.get(7)?,
            })
        },
    )
    .optional()
}

/**
 * @GOV
 * codes:    BR-SYS-STATE-008-GUARD-AG-AG-TURN-006, BR-AG-DATA-001-GUARD-AG-AG-TURN-006
 * type:     GUARD
 * chain:    AG-TURN, ED-EDIT
 * rules:    BR-SYS-STATE-008, BR-AG-DATA-001
 * term_ref: TERM-AG-015, TERM-ED-008
 * boundary: in=FreshOpenDocSnapshotMatchingAcknowledgedPreviewCheckpoint | out=SameTaskRunContinuationIdentity(consumedExactlyOnceNoBackgroundQueue)
 */
/// 消费一次已确认预览检查点，授权同一 TaskRun 的下一 AgentTurn。必须再次传入和 Ack 完全一致的
/// fresh LogicalState 基线；这切断旧 AgentReadContext 与新 Diff 的联系。
pub fn begin_preview_continuation(
    conn: &Connection,
    checkpoint: &PreviewCheckpoint,
) -> rusqlite::Result<Option<String>> {
    let changed = conn.execute(
        "UPDATE task_preview_checkpoints SET status='consumed', consumed_at=?1 \
         WHERE diff_id=?2 AND task_run_id=?3 AND workspace_id=?4 AND session_id=?5 AND turn_id=?6 AND rel_path=?7 \
         AND session_version=?8 AND content_hash=?9 AND status='acknowledged' \
         AND EXISTS (SELECT 1 FROM task_runs r WHERE r.task_run_id=?3 AND r.workspace_id=?4 AND r.session_id=?5 AND r.status IN ('active','executing')) \
         AND EXISTS (SELECT 1 FROM pending_diffs d WHERE d.diff_id=?2 AND d.workspace_id=?4 AND d.rel_path=?7 AND d.status='pending')",
        rusqlite::params![
            now_millis(),
            checkpoint.diff_id,
            checkpoint.task_run_id,
            checkpoint.workspace_id,
            checkpoint.session_id,
            checkpoint.turn_id,
            checkpoint.rel_path,
            checkpoint.session_version as i64,
            checkpoint.content_hash,
        ],
    )?;
    if changed == 1 {
        conn.execute(
            "UPDATE task_runs SET updated_at=?1 WHERE task_run_id=?2",
            rusqlite::params![now_millis(), checkpoint.task_run_id],
        )?;
        Ok(Some(checkpoint.task_run_id.clone()))
    } else {
        Ok(None)
    }
}

/// 仅供正常回合收束使用：存在未消耗的预览检查点时，TaskRun 仍在等待本地事实，不能被标为 Completed。
pub fn has_pending_preview_checkpoint(
    conn: &Connection,
    task_run_id: &str,
) -> rusqlite::Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM task_preview_checkpoints WHERE task_run_id=?1 AND status IN ('awaiting_ack','acknowledged')",
        rusqlite::params![task_run_id],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

fn latest_non_terminal_id(
    conn: &Connection,
    workspace_id: &str,
    session_id: &str,
) -> rusqlite::Result<Option<String>> {
    let mut stmt = conn.prepare(
        // 未完成任务只能经稳定 task_run_id 的显式 continuation 恢复；按 session 猜测复用会混合用户目标。
        "SELECT task_run_id, status FROM task_runs \
         WHERE workspace_id=?1 AND session_id=?2 AND status NOT IN \
         ('completed','failed','cancelled','interrupted') \
         ORDER BY updated_at DESC LIMIT 1",
    )?;
    stmt.query_row(rusqlite::params![workspace_id, session_id], |r| r.get(0))
        .optional()
}

/// 回合终态映射：完成且有未决 DecisionRequest → Waiting（裁决后由原 AgentTurn 恢复）；
/// 完成无未决裁决 → Completed；
/// 失败 → Failed；取消 → Cancelled。TaskRun 终态/Interrupted 不得被后到的 AgentTurn 结算覆写；
/// 提案（Diff）不再驱动 waiting_confirm（三层分离）。
pub(crate) fn settle_turn(
    conn: &Connection,
    task_run_id: &str,
    outcome: TurnOutcome,
) -> rusqlite::Result<TaskRunStatus> {
    let current_status = conn
        .query_row(
            "SELECT status FROM task_runs WHERE task_run_id=?1",
            rusqlite::params![task_run_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .and_then(|status| parse_run_status(&status));
    if let Some(
        status @ (TaskRunStatus::Completed
        | TaskRunStatus::Failed
        | TaskRunStatus::Cancelled
        | TaskRunStatus::Interrupted),
    ) = current_status
    {
        return Ok(status);
    }
    if outcome == TurnOutcome::Cancelled {
        conn.execute(
            "UPDATE decision_requests SET status='resolved', outcome='cancel', resolved_at=?1 \
             WHERE task_run_id=?2 AND status='proposed'",
            rusqlite::params![now_millis(), task_run_id],
        )?;
    }
    let status = match outcome {
        TurnOutcome::Completed if has_pending_preview_checkpoint(conn, task_run_id)? => {
            TaskRunStatus::Active
        }
        TurnOutcome::Completed if has_pending_decision(conn, task_run_id)? => {
            TaskRunStatus::Waiting
        }
        TurnOutcome::Completed => TaskRunStatus::Completed,
        TurnOutcome::Failed => TaskRunStatus::Failed,
        TurnOutcome::Cancelled => TaskRunStatus::Cancelled,
    };
    conn.execute(
        "UPDATE task_runs SET status=?1, updated_at=?2 WHERE task_run_id=?3",
        rusqlite::params![
            serde_json::to_string(&status).unwrap().trim_matches('"'),
            now_millis(),
            task_run_id
        ],
    )?;
    Ok(status)
}

/// 是否存在未决任务级裁决请求（decision_requests.status='proposed'）。
pub fn has_pending_decision(conn: &Connection, task_run_id: &str) -> rusqlite::Result<bool> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM decision_requests WHERE task_run_id=?1 AND status='proposed'",
        rusqlite::params![task_run_id],
        |r| r.get(0),
    )?;
    Ok(n > 0)
}

/// 最新未决任务级裁决请求（按 proposed_at 倒序）。
pub fn pending_decision(
    conn: &Connection,
    task_run_id: &str,
) -> rusqlite::Result<Option<DecisionRequestV1>> {
    let mut stmt = conn.prepare(
        "SELECT request_id, task_run_id, kind, payload_json, status, outcome, proposed_at, resolved_at \
         FROM decision_requests WHERE task_run_id=?1 AND status='proposed' \
         ORDER BY proposed_at DESC LIMIT 1",
    )?;
    let row = stmt
        .query_row(rusqlite::params![task_run_id], decision_row)
        .optional()?;
    Ok(row)
}

/// AG-1.0-TASKRUN-REWORK-IMP-RUNTIME-01 · 新消息 gate：返回本会话最新 run 上未决 DecisionRequest 的
/// request_id（若有）。存在时新回合入口必须拒绝（不得绕过待确认，A-AG-M-S-01 §2.2.2）。
pub fn latest_pending_decision_for_session(
    conn: &Connection,
    workspace_id: &str,
    session_id: &str,
) -> rusqlite::Result<Option<String>> {
    let mut stmt = conn.prepare(
        "SELECT d.request_id FROM decision_requests d \
         JOIN task_runs r ON r.task_run_id = d.task_run_id \
         WHERE r.workspace_id=?1 AND r.session_id=?2 AND d.status='proposed' \
         ORDER BY d.proposed_at DESC LIMIT 1",
    )?;
    stmt.query_row(rusqlite::params![workspace_id, session_id], |r| r.get(0))
        .optional()
}

/// 发起任务级裁决请求（TERM-AG-017）：INSERT + 置 task_run 为 waiting。
/// DecisionRequest 是任务等待的真实来源；TaskRun 只同步缓存 Waiting 投影。
pub const DECISION_KINDS: &[&str] = &["permission", "plan_choice", "scope", "manual_confirm"];

pub fn is_supported_decision_kind(kind: &str) -> bool {
    DECISION_KINDS.contains(&kind)
}

fn frozen_scope_targets(
    kind: &str,
    payload: &serde_json::Value,
) -> rusqlite::Result<Option<Vec<String>>> {
    if kind != "scope" {
        if payload
            .get("targetPaths")
            .is_some_and(|value| !value.is_null())
        {
            return Err(rusqlite::Error::InvalidParameterName(
                "targetPaths requires kind=scope".to_string(),
            ));
        }
        return Ok(None);
    }
    let paths = payload
        .get("targetPaths")
        .and_then(|value| value.as_array())
        .filter(|paths| !paths.is_empty())
        .ok_or_else(|| {
            rusqlite::Error::InvalidParameterName(
                "scope decision requires frozen targetPaths".to_string(),
            )
        })?;
    let mut normalized = std::collections::BTreeSet::new();
    for path in paths {
        let path = path
            .as_str()
            .filter(|path| !path.trim().is_empty())
            .ok_or_else(|| {
                rusqlite::Error::InvalidParameterName(
                    "scope decision targetPaths must be non-empty strings".to_string(),
                )
            })?;
        normalized.insert(path.to_string());
    }
    Ok(Some(normalized.into_iter().collect()))
}

/**
 * @GOV
 * codes:    BR-SYS-STATE-008-EFFECT-AG-AG-DECISION-001, BR-AG-DATA-003-EFFECT-AG-AG-DECISION-001
 * type:     EFFECT
 * chain:    AG-TURN, AG-TOOL
 * rules:    BR-SYS-STATE-008, BR-AG-DATA-003
 * term_ref: TERM-AG-015, TERM-AG-017, TERM-AG-019
 * boundary: in=ActiveOrActivityObservedTaskRunProjectionIdentityAndValidatedDecisionPayload | out=PersistedProposedDecisionRequestAndWaitingTaskProjectionCache | effect=workspaceDb | does_not=treatDecisionResolutionAloneAsActivityAppendScopeTargetsBeforeConfirmScheduleWorkOrReplaceTurnEventProjection
 */
pub fn propose_decision(
    conn: &Connection,
    task_run_id: &str,
    kind: &str,
    payload: serde_json::Value,
) -> rusqlite::Result<DecisionRequestV1> {
    if !is_supported_decision_kind(kind) {
        return Err(rusqlite::Error::InvalidParameterName(format!(
            "unsupported decision kind: {kind}"
        )));
    }
    frozen_scope_targets(kind, &payload)?;
    let run = load_task_run(conn, task_run_id)?.ok_or_else(|| {
        rusqlite::Error::InvalidParameterName(format!("unknown task run: {task_run_id}"))
    })?;
    // confirm 只解除等待，控制缓存保持 Waiting；恢复回合的真实 provider/tool 事件
    // 已使 TurnEvent 投影回到 Active。第二次裁决应读取这份事实，而不是要求缓存
    // 在 waitResolved 时提前伪造 Active。
    let activity_observed = if run.status == TaskRunStatus::Waiting {
        rebuild_task_projection(conn, &run.workspace_id, &run.session_id, task_run_id)?
            .is_some_and(|projection| projection.status == TaskRunProjectionStatusV2::Active)
    } else {
        false
    };
    if run.status != TaskRunStatus::Active && !activity_observed {
        return Err(rusqlite::Error::InvalidParameterName(format!(
            "task run is not active: {task_run_id}"
        )));
    }
    if has_pending_decision(conn, task_run_id)? {
        return Err(rusqlite::Error::InvalidParameterName(format!(
            "task run already has a pending decision: {task_run_id}"
        )));
    }
    let request_id = format!("decision-{}-{}", task_run_id, uuid::Uuid::new_v4());
    let now = now_millis();
    conn.execute(
        "INSERT INTO decision_requests (request_id, task_run_id, kind, payload_json, status, outcome, proposed_at, resolved_at) \
         VALUES (?1, ?2, ?3, ?4, 'proposed', NULL, ?5, NULL)",
        rusqlite::params![request_id, task_run_id, kind, payload.to_string(), now],
    )?;
    conn.execute(
        "UPDATE task_runs SET status='waiting', updated_at=?1 WHERE task_run_id=?2",
        rusqlite::params![now, task_run_id],
    )?;
    Ok(DecisionRequestV1 {
        request_id,
        task_run_id: task_run_id.to_string(),
        kind: kind.to_string(),
        payload,
        status: DecisionRequestStatus::Proposed,
        outcome: None,
        proposed_at: now,
        resolved_at: None,
    })
}

#[cfg(test)]
pub(crate) fn resolve_decision(
    conn: &Connection,
    request_id: &str,
    outcome: &str,
) -> rusqlite::Result<(String, TaskRunStatus, bool)> {
    resolve_decision_with_scope_targets(conn, request_id, outcome, None)
}

/**
 * @GOV
 * codes:    BR-SYS-STATE-008-EFFECT-AG-AG-DECISION-002, BR-AG-DATA-003-EFFECT-AG-AG-DECISION-002
 * type:     EFFECT
 * chain:    AG-TURN, AG-TOOL
 * rules:    BR-SYS-STATE-008, BR-AG-DATA-003
 * term_ref: TERM-AG-015, TERM-AG-017, TERM-AG-019
 * boundary: in=DecisionRequestOutcomeAndRevalidatedFrozenScopeTargets | out=ResolvedDecisionTaskRunStatusAndAtomicExplicitEditTargetSetAppend | effect=workspaceDb | does_not=appendOnCancelMismatchOrReplayWithDifferentOutcome
 */
/// 裁决任务级请求：confirm 只解除等待条件并恢复原 AgentTurn，cancel 终结该路线。
/// scope confirm 与 ExplicitEditTargetSet 追加在同一事务内完成；后续活动仍须由真实回合事件证明。
pub(crate) fn resolve_decision_with_scope_targets(
    conn: &Connection,
    request_id: &str,
    outcome: &str,
    confirmed_scope_targets: Option<&[String]>,
) -> rusqlite::Result<(String, TaskRunStatus, bool)> {
    if outcome != "confirm" && outcome != "cancel" {
        return Err(rusqlite::Error::InvalidParameterName(format!(
            "unsupported decision outcome: {outcome}"
        )));
    }
    let tx = conn.unchecked_transaction()?;
    let (task_run_id, kind, payload_json, decision_status, resolved_outcome): (
        String,
        String,
        String,
        String,
        Option<String>,
    ) = tx.query_row(
        "SELECT task_run_id, kind, payload_json, status, outcome FROM decision_requests WHERE request_id=?1",
        rusqlite::params![request_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
    )?;
    if decision_status == "resolved" {
        if resolved_outcome.as_deref() != Some(outcome) {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let status: String = tx.query_row(
            "SELECT status FROM task_runs WHERE task_run_id=?1",
            rusqlite::params![&task_run_id],
            |r| r.get(0),
        )?;
        let result = parse_run_status(&status)
            .map(|status| (task_run_id, status, true))
            .ok_or(rusqlite::Error::InvalidQuery)?;
        tx.commit()?;
        return Ok(result);
    }
    if decision_status != "proposed" {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let payload: serde_json::Value =
        serde_json::from_str(&payload_json).map_err(|_| rusqlite::Error::InvalidQuery)?;
    let frozen_scope_targets = frozen_scope_targets(&kind, &payload)?;
    if kind == "scope" && outcome == "confirm" {
        let confirmed = confirmed_scope_targets.ok_or(rusqlite::Error::InvalidQuery)?;
        if frozen_scope_targets.as_deref() != Some(confirmed) {
            return Err(rusqlite::Error::InvalidQuery);
        }
    } else if confirmed_scope_targets.is_some() {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let now = now_millis();
    tx.execute(
        "UPDATE decision_requests SET status='resolved', outcome=?1, resolved_at=?2 WHERE request_id=?3",
        rusqlite::params![outcome, now, request_id],
    )?;
    let status = if outcome == "confirm" {
        // waitResolved 本身不能伪造 activityObserved；恢复原回合后的 provider/tool 事件才使投影 Active。
        TaskRunStatus::Waiting
    } else {
        TaskRunStatus::Cancelled
    };
    if outcome == "confirm" {
        if let Some(paths) = frozen_scope_targets.as_deref() {
            super::target_set::append_task_run_targets(
                &tx,
                &task_run_id,
                paths,
                super::target_set::SOURCE_SCOPE_DECISION,
                Some(request_id),
            )?;
        }
    }
    tx.execute(
        "UPDATE task_runs SET status=?1, updated_at=?2 WHERE task_run_id=?3",
        rusqlite::params![
            serde_json::to_string(&status).unwrap().trim_matches('"'),
            now,
            task_run_id
        ],
    )?;
    tx.commit()?;
    Ok((task_run_id, status, false))
}

/// 受控续片段入口：只允许使用刚刚确认的 DecisionRequest 继续其所属 TaskRun。
/// 不依赖“最近活动任务”排序，避免同会话存在其他任务时接错身份。
pub fn begin_confirmed_continuation(
    conn: &Connection,
    workspace_id: &str,
    session_id: &str,
    task_run_id: &str,
    decision_request_id: &str,
) -> rusqlite::Result<Option<String>> {
    let matched: Option<String> = conn
        .query_row(
            "SELECT r.task_run_id FROM task_runs r \
             JOIN decision_requests d ON d.task_run_id=r.task_run_id \
             WHERE r.task_run_id=?1 AND r.workspace_id=?2 AND r.session_id=?3 \
             AND r.status IN ('waiting','active','executing') AND d.request_id=?4 \
             AND d.status='resolved' AND d.outcome='confirm'",
            rusqlite::params![task_run_id, workspace_id, session_id, decision_request_id],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(ref id) = matched {
        conn.execute(
            "UPDATE task_runs SET updated_at=?1 WHERE task_run_id=?2",
            rusqlite::params![now_millis(), id],
        )?;
    }
    Ok(matched)
}

/// 同一 AgentTurn 的裁决恢复入口：confirm/cancel 均必须匹配已解决的 DecisionRequest 和所属任务身份。
/// cancel 只终止该 TaskRun 投影，不把原 AgentTurn 伪装成取消终态；模型仍可基于结果形成最终说明。
pub fn begin_resolved_decision_continuation(
    conn: &Connection,
    workspace_id: &str,
    session_id: &str,
    task_run_id: &str,
    decision_request_id: &str,
) -> rusqlite::Result<Option<String>> {
    let matched: Option<String> = conn
        .query_row(
            "SELECT r.task_run_id FROM task_runs r \
             JOIN decision_requests d ON d.task_run_id=r.task_run_id \
             WHERE r.task_run_id=?1 AND r.workspace_id=?2 AND r.session_id=?3 \
             AND d.request_id=?4 AND d.status='resolved' \
             AND ((d.outcome='confirm' AND r.status IN ('waiting','active','executing')) \
               OR (d.outcome='cancel' AND r.status='cancelled'))",
            rusqlite::params![task_run_id, workspace_id, session_id, decision_request_id],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(ref id) = matched {
        conn.execute(
            "UPDATE task_runs SET updated_at=?1 WHERE task_run_id=?2",
            rusqlite::params![now_millis(), id],
        )?;
    }
    Ok(matched)
}

pub fn resolved_decision_outcome(
    conn: &Connection,
    task_run_id: &str,
    decision_request_id: &str,
) -> rusqlite::Result<Option<String>> {
    conn.query_row(
        "SELECT outcome FROM decision_requests \
         WHERE task_run_id=?1 AND request_id=?2 AND status='resolved'",
        rusqlite::params![task_run_id, decision_request_id],
        |row| row.get(0),
    )
    .optional()
}

/**
 * @GOV
 * codes: BR-AG-DATA-004-QUERY-AG-AG-TURN-052, BR-SYS-STATE-008-QUERY-AG-AG-TURN-052
 * type: QUERY
 * chain: AG-TURN, WS-OPERATION
 * rules: BR-AG-DATA-004, BR-SYS-STATE-008
 * term_ref: TERM-AG-017, TERM-AG-015
 * boundary: in=OriginalWorkspaceSessionTurnTaskAndResolvedDecisionIdentity | out=IdentityBoundDecisionAndWorkspaceOperationEffectProjection | does_not=authorizeExecutionReadDocumentBodyInferSaveFromConfirmationOrRewriteTaskOutcome
 */
pub(super) fn resolved_decision_receipt(
    conn: &Connection,
    workspace: &str,
    session: &str,
    turn: &str,
    run: &str,
    request: &str,
) -> Result<Option<serde_json::Value>, crate::ipc::IpcError> {
    use crate::ipc::IpcError;
    let db_error =
        |e: rusqlite::Error| IpcError::new("AG_TASK_DB_ERROR", &e.to_string(), true, false);
    let invalid = || {
        IpcError::new(
            "AG_TASK_CONTINUATION_INVALID",
            "裁决效果与原回合身份或执行版本不匹配，未恢复执行",
            false,
            false,
        )
    };
    let decision: Option<(String,Option<String>)> = conn.query_row(
        "SELECT d.outcome,json_extract(d.payload_json,'$.workspaceOperationPlanId') FROM decision_requests d \
         JOIN task_runs r ON r.task_run_id=d.task_run_id JOIN agent_turns t ON t.task_run_id=r.task_run_id \
         WHERE r.workspace_id=?1 AND r.session_id=?2 AND t.turn_id=?3 AND t.workspace_id=?1 AND t.session_id=?2 \
         AND r.task_run_id=?4 AND d.request_id=?5 AND d.status='resolved'",
        rusqlite::params![workspace,session,turn,run,request], |row|Ok((row.get(0)?,row.get(1)?)),
    ).optional().map_err(db_error)?;
    let Some((outcome, plan_id)) = decision else {
        return Ok(None);
    };
    let mut fact = serde_json::json!({"version":"decision-result-v2","workspaceId":workspace,"sessionId":session,
        "turnId":turn,"taskRunId":run,"requestId":request,"outcome":outcome,
        "workspaceOperation":null,"readBackVerified":false});
    if let Some(plan_id) = plan_id {
        let plan = crate::workspace::operation::load_plan(conn, &plan_id)
            .map_err(db_error)?
            .ok_or_else(invalid)?;
        if plan.workspace_id != workspace
            || plan.task_run_id.as_deref() != Some(run)
            || plan.source_turn_id.as_deref() != Some(turn)
            || plan.decision_request_id.as_deref() != Some(request)
        {
            return Err(invalid());
        }
        let receipt = crate::workspace::operation::load_outcome(conn, &plan_id)?;
        if receipt.as_ref().is_some_and(|r| {
            r.plan_id != plan_id || Some(r.operation_id.as_str()) != plan.operation_id.as_deref()
        }) {
            return Err(invalid());
        }
        let mut content = serde_json::Value::Null;
        if let Some(bundle) =
            crate::workspace::operation::load_create_document_bundle(conn, &plan_id)
                .map_err(db_error)?
        {
            if bundle.target.workspace_id != workspace || bundle.operation_id != plan.operation_id {
                return Err(invalid());
            }
            let diff = if let Some(id) = bundle.initial_diff_id.as_deref() {
                conn.query_row("SELECT status,effect_status FROM pending_diffs WHERE diff_id=?1 AND workspace_id=?2 AND rel_path=?3",
                    rusqlite::params![id,workspace,bundle.target.rel_path],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?))).optional().map_err(db_error)?
            } else {
                None
            };
            let saved = bundle.status == "saved"
                && diff
                    .as_ref()
                    .is_some_and(|(s, e)| s == "accepted" && e == "saved");
            content = serde_json::json!({"bundleStatus":bundle.status,"targetPath":bundle.target.rel_path,
                "diffId":bundle.initial_diff_id,"diffStatus":diff.as_ref().map(|d|&d.0),
                "effectStatus":diff.as_ref().map(|d|&d.1),"saveVerified":saved});
        }
        let items:Vec<_>=plan.operation_items.iter().map(|item|serde_json::json!({
            "itemId":item.item_id,"kind":item.kind,"sourcePath":item.source_rel_path,"targetPath":item.target_rel_path,
        })).collect();
        fact["workspaceOperation"] = serde_json::json!({"planId":plan_id,"planStatus":plan.status,
            "operationId":plan.operation_id,"receipt":receipt,"items":items,"content":content});
    }
    Ok(Some(fact))
}

fn decision_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<DecisionRequestV1> {
    let payload_json: String = r.get(3)?;
    let status: String = r.get(4)?;
    let outcome: Option<String> = r.get(5)?;
    Ok(DecisionRequestV1 {
        request_id: r.get(0)?,
        task_run_id: r.get(1)?,
        kind: r.get(2)?,
        payload: serde_json::from_str(&payload_json).unwrap_or(serde_json::Value::Null),
        status: if status == "resolved" {
            DecisionRequestStatus::Resolved
        } else {
            DecisionRequestStatus::Proposed
        },
        outcome,
        proposed_at: r.get(6)?,
        resolved_at: r.get(7)?,
    })
}

/**
 * @GOV
 * codes:    BR-SYS-STATE-007-EFFECT-AG-AG-TURN-047, BR-AG-STATE-001-EFFECT-AG-AG-TURN-047, BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-047
 * type:     EFFECT
 * chain:    AG-TURN
 * rules:    BR-SYS-STATE-007, BR-AG-STATE-001, BR-AG-PERSIST-001
 * term_ref: TERM-AG-015
 * boundary: in=DerivedTaskProjectionFactBoundToExistingAgentTurn | out=AppendOnlyInternalTurnEvent | effect=workspaceDb | does_not=appendLateBusinessSideEffectsOrMutateSourceFacts
 */
pub fn record_projection_fact(
    conn: &Connection,
    turn_id: &str,
    kind: &str,
    payload: serde_json::Value,
) -> rusqlite::Result<bool> {
    let tx = conn.unchecked_transaction()?;
    let identity = tx
        .query_row(
            "SELECT workspace_id, session_id, status FROM agent_turns WHERE turn_id=?1",
            [turn_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()?;
    let Some((workspace_id, session_id, turn_status)) = identity else {
        return Ok(false);
    };
    let is_recovery_observation = kind == "task_status_observed"
        && payload.get("status").and_then(serde_json::Value::as_str) == Some("interrupted");
    if !is_recovery_observation
        && matches!(
            turn_status.as_str(),
            "completed" | "error" | "cancelled" | "gate_rejected"
        )
    {
        return Ok(false);
    }
    let next_seq = tx.query_row(
        "SELECT COALESCE(MAX(seq), 0) + 1 FROM agent_turn_events WHERE turn_id=?1",
        [turn_id],
        |row| row.get::<_, u64>(0),
    )?;
    let occurred_at = now_millis();
    tx.execute(
        "INSERT INTO agent_turn_events \
         (event_id, workspace_id, session_id, turn_id, seq, kind, payload_version, payload_json, occurred_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?8)",
        rusqlite::params![
            format!("turn-event:{turn_id}:{next_seq}"),
            workspace_id,
            session_id,
            turn_id,
            next_seq,
            kind,
            payload.to_string(),
            occurred_at,
        ],
    )?;
    tx.commit()?;
    Ok(true)
}

/// 应用启动清理（不跨重启续跑，BR-SYS-STATE-008）：把所有 workspace 的非终态 TaskRun 置
/// Interrupted（展示"中断可重试"）；新回合 begin_turn 将新建（retry 创建新回合）。
/// 此函数只能由应用启动恢复路径调用，不能在每个 AgentTurn 入口调用，否则会中断同一进程内的续片段。
/**
 * @GOV
 * codes:    BR-SYS-STATE-008-EFFECT-AG-AG-TURN-033, BR-AG-STATE-001-EFFECT-AG-AG-TURN-033
 * type:     EFFECT
 * chain:    AG-TURN
 * rules:    BR-SYS-STATE-008, BR-AG-STATE-001
 * term_ref: TERM-AG-015
 * boundary: in=applicationStartupPersistedNonTerminalTaskRuns | out=InterruptedTaskRuns(noCrossRestartContinuation)
 */
pub fn mark_all_stale_as_interrupted(conn: &Connection) -> rusqlite::Result<usize> {
    let mut statement = conn.prepare(
        "SELECT r.task_run_id, \
         (SELECT t.turn_id FROM agent_turns t WHERE t.task_run_id=r.task_run_id \
          ORDER BY t.updated_at DESC, t.created_at DESC, t.turn_id DESC LIMIT 1) \
         FROM task_runs r \
         WHERE r.status NOT IN ('completed','failed','cancelled','interrupted') \
         AND EXISTS (SELECT 1 FROM agent_turns t WHERE t.task_run_id=r.task_run_id)",
    )?;
    let linked = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    drop(statement);
    let n = conn.execute(
        "UPDATE task_runs SET status='interrupted', updated_at=?1 \
         WHERE status NOT IN \
         ('completed','failed','cancelled','interrupted')",
        rusqlite::params![now_millis()],
    )?;
    for (task_run_id, turn_id) in linked {
        let _ = record_projection_fact(
            conn,
            &turn_id,
            "task_status_observed",
            serde_json::json!({"taskRunId": task_run_id, "status": "interrupted"}),
        )?;
    }
    Ok(n)
}

/// 读取 TaskRun 身份/裁决控制缓存。展示状态不得直接消费此结构。
pub(crate) fn load_task_run(
    conn: &Connection,
    task_run_id: &str,
) -> rusqlite::Result<Option<TaskRunControl>> {
    conn.query_row(
        "SELECT task_run_id, workspace_id, session_id, status, intent_summary, created_at, updated_at \
         FROM task_runs WHERE task_run_id=?1",
        rusqlite::params![task_run_id],
        |row| {
            Ok(TaskRunControl {
                task_run_id: row.get(0)?,
                workspace_id: row.get(1)?,
                session_id: row.get(2)?,
                status: parse_run_status(&row.get::<_, String>(3)?)
                    .unwrap_or(TaskRunStatus::Active),
                intent_summary: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        },
    )
    .optional()
}

/**
 * @GOV
 * codes:    BR-SYS-STATE-008-GUARD-AG-AG-TASK-004
 * type:     GUARD
 * chain:    AG-TURN, AG-TOOL
 * rules:    BR-SYS-STATE-008
 * term_ref: TERM-AG-015, TERM-AG-018
 * boundary: in=ModelVisibleRouteRevisionTargetCandidatesAndConfirmationPoints | out=boolean（true 当 route证明复杂目标或跨可见片段）
 */
pub fn route_requires_task_projection(
    route_steps: &[String],
    confirmation_points: &[String],
    target_paths: &[String],
) -> bool {
    let distinct_targets = target_paths.iter().collect::<BTreeSet<_>>().len();
    !confirmation_points.is_empty() || route_steps.len() >= 3 || distinct_targets >= 2
}

/// 只有会产生任务级持久产物的动态工具才要求在执行前取得 TaskRun。
/// 内容提案（propose_edit）仍属于 AgentTurn 产物；简单局部编辑不能因此升级为任务编排。
pub fn tool_requires_task_run(tool_name: &str) -> bool {
    matches!(tool_name, "propose_decision" | "plan_workspace_operation")
}

/**
 * @GOV
 * codes:    BR-SYS-STATE-008-GUARD-AG-AG-TASK-003
 * type:     GUARD
 * chain:    AG-TURN, AG-TOOL
 * rules:    BR-SYS-STATE-008
 * term_ref: TERM-AG-015, TERM-AG-018
 * boundary: in=ModelVisibleComplexRouteOrTaskLevelWaitingTool | out=OptionalStableTaskRunProjectionIdentity | does_not=classifyPromptKeywordsCreateSimpleTurnProjectionOrExecuteCandidateRoute
 */
pub fn begin_for_model_route(
    conn: &Connection,
    workspace_id: &str,
    session_id: &str,
    goal: &str,
    route_steps: &[String],
    confirmation_points: &[String],
    target_paths: &[String],
) -> rusqlite::Result<Option<String>> {
    if !route_requires_task_projection(route_steps, confirmation_points, target_paths) {
        return Ok(None);
    }
    begin_turn(conn, workspace_id, session_id, Some(goal)).map(Some)
}

pub fn begin_for_tool_product(
    conn: &Connection,
    workspace_id: &str,
    session_id: &str,
    tool_name: &str,
) -> rusqlite::Result<Option<String>> {
    if !tool_requires_task_run(tool_name) {
        return Ok(None);
    }
    begin_turn(
        conn,
        workspace_id,
        session_id,
        Some(&format!("任务级工具 {tool_name}")),
    )
    .map(Some)
}

#[derive(Debug)]
struct ProjectionEvent {
    event_id: String,
    turn_id: String,
    seq: u64,
    kind: String,
    payload: serde_json::Value,
    occurred_at: i64,
}

fn projection_events(
    conn: &Connection,
    workspace_id: &str,
    session_id: &str,
    task_run_id: &str,
) -> rusqlite::Result<Vec<ProjectionEvent>> {
    let mut statement = conn.prepare(
        "SELECT e.event_id, e.turn_id, e.seq, e.kind, e.payload_json, e.occurred_at \
         FROM agent_turn_events e JOIN agent_turns t ON t.turn_id=e.turn_id \
         WHERE e.workspace_id=?1 AND e.session_id=?2 AND t.task_run_id=?3 \
         ORDER BY e.occurred_at ASC, e.turn_id ASC, e.seq ASC",
    )?;
    let rows = statement.query_map(
        rusqlite::params![workspace_id, session_id, task_run_id],
        |row| {
            let payload_json: String = row.get(4)?;
            Ok(ProjectionEvent {
                event_id: row.get(0)?,
                turn_id: row.get(1)?,
                seq: row.get(2)?,
                kind: row.get(3)?,
                payload: serde_json::from_str(&payload_json).unwrap_or(serde_json::Value::Null),
                occurred_at: row.get(5)?,
            })
        },
    )?;
    rows.collect()
}

fn candidate_steps(task_run_id: &str, route_event: &ProjectionEvent) -> Vec<TaskStepProjectionV2> {
    route_event
        .payload
        .get("steps")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
        .filter_map(|(index, value)| value.as_str().map(|label| (index, label)))
        .map(|(index, label)| TaskStepProjectionV2 {
            step_id: format!("candidate:{task_run_id}:{}:{index}", route_event.event_id),
            kind: "candidate".to_string(),
            label: label.to_string(),
            status: TaskStepProjectionStatusV2::Candidate,
            depends_on: if index == 0 {
                Vec::new()
            } else {
                vec![format!(
                    "candidate:{task_run_id}:{}:{}",
                    route_event.event_id,
                    index - 1
                )]
            },
            source_event_id: route_event.event_id.clone(),
            source_event_seq: route_event.seq,
            result: None,
        })
        .collect()
}

fn event_string<'a>(payload: &'a serde_json::Value, field: &str, legacy: &str) -> Option<&'a str> {
    payload
        .get(field)
        .or_else(|| payload.get(legacy))
        .and_then(serde_json::Value::as_str)
}

/**
 * @GOV
 * codes:    BR-SYS-STATE-008-QUERY-AG-AG-TASK-004, BR-AG-PERSIST-001-QUERY-AG-AG-TASK-004
 * type:     QUERY
 * chain:    AG-TURN
 * rules:    BR-SYS-STATE-008, BR-AG-PERSIST-001
 * term_ref: TERM-AG-015, TERM-AG-016, TERM-AG-018, TERM-AG-022
 * boundary: in=AppendOnlyTurnEventsLinkedToTaskRunIdentity | out=DeterministicTaskRunProjectionV2WithObservedWaitingAndCurrentRouteFacts | does_not=readLegacyPlanCursorScheduleCandidateWorkOrMutateDecisionProposalToolAndWorkspaceFacts
 */
pub fn rebuild_task_projection(
    conn: &Connection,
    workspace_id: &str,
    session_id: &str,
    task_run_id: &str,
) -> rusqlite::Result<Option<TaskRunProjectionV2>> {
    let control = load_task_run(conn, task_run_id)?;
    let events = projection_events(conn, workspace_id, session_id, task_run_id)?;
    if control.is_none() && events.is_empty() {
        return Ok(None);
    }
    let mut source_turn_ids = Vec::new();
    let mut through_seq = 0;
    let mut goal = control
        .as_ref()
        .and_then(|row| row.intent_summary.clone())
        .unwrap_or_default();
    let mut status = TaskRunProjectionStatusV2::Projected;
    let mut observed_steps = Vec::new();
    let mut tool_steps = BTreeMap::<String, usize>::new();
    let mut decision_steps = BTreeMap::<String, usize>::new();
    let mut waiting_on = BTreeSet::new();
    let mut latest_route: Option<&ProjectionEvent> = None;
    let mut route_revision = None;
    let mut created_at = control.as_ref().map(|row| row.created_at).unwrap_or(0);
    let mut updated_at = control.as_ref().map(|row| row.updated_at).unwrap_or(0);

    for event in &events {
        if !source_turn_ids.contains(&event.turn_id) {
            source_turn_ids.push(event.turn_id.clone());
        }
        through_seq = through_seq.max(event.seq);
        if created_at == 0 || event.occurred_at < created_at {
            created_at = event.occurred_at;
        }
        updated_at = updated_at.max(event.occurred_at);
        match event.kind.as_str() {
            "task_projected" => {
                if let Some(value) = event.payload.get("goal").and_then(|value| value.as_str()) {
                    goal = value.to_string();
                }
            }
            "intent_updated" => {
                if let Some(value) = event
                    .payload
                    .get("projection")
                    .and_then(|value| value.get("goal"))
                    .and_then(|value| value.as_str())
                {
                    goal = value.to_string();
                }
            }
            "route_updated" => {
                let revision = event
                    .payload
                    .get("revision")
                    .and_then(|value| value.as_u64())
                    .unwrap_or(0);
                if route_revision.is_none_or(|current| revision >= current) {
                    route_revision = Some(revision);
                    latest_route = Some(event);
                }
            }
            "provider_started" | "step_progress" => {
                status = TaskRunProjectionStatusV2::Active;
                waiting_on.clear();
            }
            "tool_request" => {
                status = TaskRunProjectionStatusV2::Active;
                waiting_on.clear();
                let call_id = event_string(&event.payload, "toolCallId", "tool_call_id")
                    .unwrap_or(&event.event_id)
                    .to_string();
                let label = event_string(&event.payload, "toolName", "tool_name")
                    .unwrap_or("tool")
                    .to_string();
                tool_steps.insert(call_id, observed_steps.len());
                observed_steps.push(TaskStepProjectionV2 {
                    step_id: format!("observed:{}", event.event_id),
                    kind: "tool".to_string(),
                    label,
                    status: TaskStepProjectionStatusV2::Active,
                    depends_on: Vec::new(),
                    source_event_id: event.event_id.clone(),
                    source_event_seq: event.seq,
                    result: None,
                });
            }
            "tool_result" => {
                let call_id =
                    event_string(&event.payload, "toolCallId", "tool_call_id").unwrap_or_default();
                if let Some(index) = tool_steps.get(call_id).copied() {
                    let success = event
                        .payload
                        .get("success")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false);
                    observed_steps[index].status = if success {
                        TaskStepProjectionStatusV2::Completed
                    } else {
                        TaskStepProjectionStatusV2::Failed
                    };
                    observed_steps[index].result =
                        Some(if success { "completed" } else { "failed" }.to_string());
                }
            }
            "diff_proposed" => observed_steps.push(TaskStepProjectionV2 {
                step_id: format!("observed:{}", event.event_id),
                kind: "proposal".to_string(),
                label: event
                    .payload
                    .get("relPath")
                    .or_else(|| event.payload.get("rel_path"))
                    .and_then(|value| value.as_str())
                    .map(|path| format!("Proposal: {path}"))
                    .unwrap_or_else(|| "Proposal".to_string()),
                status: TaskStepProjectionStatusV2::Completed,
                depends_on: Vec::new(),
                source_event_id: event.event_id.clone(),
                source_event_seq: event.seq,
                result: Some("proposed".to_string()),
            }),
            "decision_request_proposed" => {
                status = TaskRunProjectionStatusV2::Waiting;
                let request_id = event_string(&event.payload, "requestId", "request_id")
                    .unwrap_or(&event.event_id)
                    .to_string();
                waiting_on.insert(format!("decision:{request_id}"));
                decision_steps.insert(request_id, observed_steps.len());
                observed_steps.push(TaskStepProjectionV2 {
                    step_id: format!("observed:{}", event.event_id),
                    kind: "decision".to_string(),
                    label: "Waiting for decision".to_string(),
                    status: TaskStepProjectionStatusV2::Waiting,
                    depends_on: Vec::new(),
                    source_event_id: event.event_id.clone(),
                    source_event_seq: event.seq,
                    result: None,
                });
            }
            "decision_request_resolved" => {
                let request_id =
                    event_string(&event.payload, "requestId", "request_id").unwrap_or_default();
                let outcome = event
                    .payload
                    .get("outcome")
                    .and_then(|value| value.as_str())
                    .unwrap_or("cancel");
                waiting_on.remove(&format!("decision:{request_id}"));
                if let Some(index) = decision_steps.get(request_id).copied() {
                    observed_steps[index].status = if outcome == "confirm" {
                        TaskStepProjectionStatusV2::Completed
                    } else {
                        TaskStepProjectionStatusV2::Cancelled
                    };
                    observed_steps[index].result = Some(outcome.to_string());
                }
                status = if outcome == "cancel" {
                    TaskRunProjectionStatusV2::Cancelled
                } else {
                    TaskRunProjectionStatusV2::Waiting
                };
            }
            "clarification_required" => {
                status = TaskRunProjectionStatusV2::Waiting;
                waiting_on.insert("clarification".to_string());
            }
            "task_status_observed" => {
                status = match event
                    .payload
                    .get("status")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
                {
                    "active" => TaskRunProjectionStatusV2::Active,
                    "waiting" => TaskRunProjectionStatusV2::Waiting,
                    "completed" => TaskRunProjectionStatusV2::Completed,
                    "failed" => TaskRunProjectionStatusV2::Failed,
                    "cancelled" => TaskRunProjectionStatusV2::Cancelled,
                    "interrupted" => TaskRunProjectionStatusV2::Interrupted,
                    _ => status,
                };
            }
            "error" => status = TaskRunProjectionStatusV2::Failed,
            _ => {}
        }
    }
    let candidate_steps = latest_route
        .map(|event| candidate_steps(task_run_id, event))
        .unwrap_or_default();
    Ok(Some(TaskRunProjectionV2 {
        task_run_id: task_run_id.to_string(),
        workspace_id: workspace_id.to_string(),
        session_id: session_id.to_string(),
        source_turn_ids,
        through_seq,
        goal,
        status,
        observed_steps,
        waiting_on: waiting_on.into_iter().collect(),
        current_route_revision: route_revision,
        candidate_steps,
        created_at,
        updated_at,
    }))
}

pub fn list_task_runs_by_session(
    conn: &Connection,
    workspace_id: &str,
    session_id: &str,
) -> rusqlite::Result<Vec<TaskRunProjectionV2>> {
    let mut statement = conn.prepare(
        "SELECT DISTINCT task_run_id FROM agent_turns \
         WHERE workspace_id=?1 AND session_id=?2 AND task_run_id IS NOT NULL",
    )?;
    let ids = statement
        .query_map(rusqlite::params![workspace_id, session_id], |row| {
            row.get::<_, String>(0)
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut projections = Vec::new();
    for task_run_id in ids {
        if let Some(projection) =
            rebuild_task_projection(conn, workspace_id, session_id, &task_run_id)?
        {
            projections.push(projection);
        }
    }
    projections.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    Ok(projections)
}

pub fn list_plans_by_run(
    conn: &Connection,
    workspace_id: &str,
    session_id: &str,
    task_run_id: &str,
) -> rusqlite::Result<Vec<TaskPlanProjectionV2>> {
    let Some(run) = rebuild_task_projection(conn, workspace_id, session_id, task_run_id)? else {
        return Ok(Vec::new());
    };
    let Some(revision) = run.current_route_revision else {
        return Ok(Vec::new());
    };
    let summary = projection_events(conn, workspace_id, session_id, task_run_id)?
        .into_iter()
        .filter(|event| event.kind == "route_updated")
        .filter_map(|event| {
            let event_revision = event.payload.get("revision")?.as_u64()?;
            let summary = event.payload.get("summary")?.as_str()?.to_string();
            (event_revision == revision).then_some(summary)
        })
        .last()
        .unwrap_or_default();
    Ok(vec![TaskPlanProjectionV2 {
        plan_id: format!("plan:{task_run_id}:{revision}"),
        task_run_id: task_run_id.to_string(),
        through_seq: run.through_seq,
        route_revision: revision,
        summary,
        candidate_steps: run.candidate_steps,
    }])
}

fn parse_run_status(s: &str) -> Option<TaskRunStatus> {
    match s {
        "planned" | "active" | "executing" => Some(TaskRunStatus::Active),
        "waiting" | "waiting_confirm" | "waiting_clarification" => Some(TaskRunStatus::Waiting),
        "completed" => Some(TaskRunStatus::Completed),
        "failed" => Some(TaskRunStatus::Failed),
        "cancelled" => Some(TaskRunStatus::Cancelled),
        "interrupted" => Some(TaskRunStatus::Interrupted),
        _ => None,
    }
}

/// 回合→TaskRun 关联（agent_turns.task_run_id，DiffCard 可经此关联，UNC-09）。
pub fn link_turn(conn: &Connection, turn_id: &str, task_run_id: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE agent_turns SET task_run_id=?1 WHERE turn_id=?2",
        rusqlite::params![task_run_id, turn_id],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::TaskRunStatus::*;
    use super::*;
    use crate::db;

    fn fresh() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        db::migrate(&mut conn).unwrap();
        conn
    }

    fn insert_turn(conn: &Connection, turn_id: &str, session_id: &str, task_run_id: &str) {
        conn.execute(
            "INSERT INTO agent_turns (turn_id, workspace_id, session_id, rel_path, status, payload_ref, created_at, updated_at, task_run_id) \
             VALUES (?1, 'ws1', ?2, 'notes/a.md', 'draft', ?1, 1, 1, ?3)",
            rusqlite::params![turn_id, session_id, task_run_id],
        )
        .unwrap();
    }

    fn insert_event(
        conn: &Connection,
        turn_id: &str,
        session_id: &str,
        seq: u64,
        kind: &str,
        payload: serde_json::Value,
    ) {
        conn.execute(
            "INSERT INTO agent_turn_events (event_id, workspace_id, session_id, turn_id, seq, kind, payload_version, payload_json, occurred_at) \
             VALUES (?1, 'ws1', ?2, ?3, ?4, ?5, 1, ?6, ?4)",
            rusqlite::params![
                format!("turn-event:{turn_id}:{seq}"),
                session_id,
                turn_id,
                seq,
                kind,
                payload.to_string()
            ],
        )
        .unwrap();
    }

    // covers: BR-SYS-STATE-008
    // 只有任务级产物入口才调用 begin_turn：首次新建 active；非终态目标必须显式续接；终态后新建。
    #[test]
    fn task_level_product_rejects_implicit_reuse_and_creates_after_terminal() {
        let conn = fresh();
        let id1 = begin_turn(&conn, "ws1", "s1", Some("分析文档")).unwrap();
        assert!(
            conn.query_row(
                "SELECT COUNT(*) FROM task_runs WHERE task_run_id=?1",
                rusqlite::params![id1],
                |r| r.get::<_, i64>(0),
            )
            .unwrap()
                > 0
        );
        // 不携带 continuation 身份时不得按 session 猜测沿用。
        let error = begin_turn(&conn, "ws1", "s1", Some("继续")).unwrap_err();
        assert!(error.to_string().contains("explicit continuation"));
        // 回合完成（无未决裁决）→ 终态 → 下次新建
        let st = settle_turn(&conn, &id1, TurnOutcome::Completed).unwrap();
        assert_eq!(st, Completed);
        let id3 = begin_turn(&conn, "ws1", "s1", Some("新目标")).unwrap();
        assert_ne!(id1, id3);
    }

    // covers: BR-SYS-STATE-008
    // DecisionRequest 是 Waiting 的唯一任务级来源；任意新消息不绕过，confirm 只解除等待条件。
    #[test]
    fn decision_request_drives_waiting_confirm_and_resolution() {
        let conn = fresh();
        let id = begin_turn(&conn, "ws1", "s1", None).unwrap();
        let run = load_task_run(&conn, &id).unwrap().unwrap();
        assert_eq!(run.status, Active);
        // 任务级裁决请求 → Waiting
        let dr = propose_decision(
            &conn,
            &id,
            "plan_choice",
            serde_json::json!({ "note": "确认方案" }),
        )
        .unwrap();
        assert_eq!(dr.status, DecisionRequestStatus::Proposed);
        assert_eq!(load_task_run(&conn, &id).unwrap().unwrap().status, Waiting);
        assert!(has_pending_decision(&conn, &id).unwrap());
        // settle 保持 Waiting（有未决裁决）
        let st = settle_turn(&conn, &id, TurnOutcome::Completed).unwrap();
        assert_eq!(st, Waiting);
        // 任意新消息不绕回：没有显式 continuation 时拒绝，不创建第二个 TaskRun。
        let error = begin_turn(&conn, "ws1", "s1", Some("继续")).unwrap_err();
        assert!(error.to_string().contains("explicit continuation"));
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM task_runs", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
        // 裁决 confirm 只解除等待条件；没有后续真实 activityObserved 时投影仍是 Waiting。
        let (resolved_run_id, st2, already_resolved) =
            resolve_decision(&conn, &dr.request_id, "confirm").unwrap();
        assert_eq!(st2, Waiting);
        assert_eq!(resolved_run_id, id);
        assert!(!already_resolved);
        assert!(!has_pending_decision(&conn, &id).unwrap());
        let continuation_id =
            begin_confirmed_continuation(&conn, "ws1", "s1", &resolved_run_id, &dr.request_id)
                .unwrap();
        assert_eq!(continuation_id.as_deref(), Some(id.as_str()));
    }

    // covers: BR-SYS-STATE-008
    // 裁决取消（userCancel）→ Cancelled；相同结果重放幂等，冲突结果拒绝。
    #[test]
    fn decision_cancel_is_idempotent_and_rejects_conflicting_outcome() {
        let conn = fresh();
        let id = begin_turn(&conn, "ws1", "s1", None).unwrap();
        let dr = propose_decision(&conn, &id, "permission", serde_json::json!({})).unwrap();
        let (resolved_id, st, already_resolved) =
            resolve_decision(&conn, &dr.request_id, "cancel").unwrap();
        assert_eq!(resolved_id, id);
        assert_eq!(st, Cancelled);
        assert!(!already_resolved);
        let (replayed_id, replayed_status, replayed) =
            resolve_decision(&conn, &dr.request_id, "cancel").unwrap();
        assert_eq!(replayed_id, id);
        assert_eq!(replayed_status, Cancelled);
        assert!(replayed);
        assert!(resolve_decision(&conn, &dr.request_id, "confirm").is_err());
        // cancel 只恢复原 AgentTurn 生成可见结论；后续回合完成不得覆写 TaskRun 的取消终态。
        assert_eq!(
            settle_turn(&conn, &id, TurnOutcome::Completed).unwrap(),
            Cancelled
        );
    }

    // covers: BR-SYS-STATE-008
    // AgentTurn 从任意取消入口结束时，未决 DecisionRequest 也必须收敛为 cancel，不能遗留幽灵门禁。
    #[test]
    fn turn_cancellation_resolves_pending_decision_request() {
        let conn = fresh();
        let id = begin_turn(&conn, "ws1", "cancel-pending-decision", None).unwrap();
        let dr = propose_decision(&conn, &id, "permission", serde_json::json!({})).unwrap();
        assert!(has_pending_decision(&conn, &id).unwrap());

        assert_eq!(
            settle_turn(&conn, &id, TurnOutcome::Cancelled).unwrap(),
            Cancelled
        );
        assert!(!has_pending_decision(&conn, &id).unwrap());
        assert_eq!(
            resolved_decision_outcome(&conn, &id, &dr.request_id).unwrap(),
            Some("cancel".to_string())
        );
        assert_eq!(
            latest_pending_decision_for_session(&conn, "ws1", "cancel-pending-decision").unwrap(),
            None
        );
    }

    // covers: BR-AG-DATA-003
    // covers: BR-SYS-STATE-008
    #[test]
    fn scope_decision_appends_frozen_targets_only_on_confirm() {
        let conn = fresh();
        let confirmed_id = begin_turn(&conn, "ws1", "scope-confirm", None).unwrap();
        let confirmed = propose_decision(
            &conn,
            &confirmed_id,
            "scope",
            serde_json::json!({ "targetPaths": ["b.md", "b.md"] }),
        )
        .unwrap();
        assert!(resolve_decision(&conn, &confirmed.request_id, "confirm").is_err());
        let (_, status, already_resolved) = resolve_decision_with_scope_targets(
            &conn,
            &confirmed.request_id,
            "confirm",
            Some(&["b.md".to_string()]),
        )
        .unwrap();
        assert_eq!(status, Waiting);
        assert!(!already_resolved);
        assert_eq!(
            crate::ag::target_set::load_for_task_run(&conn, &confirmed_id)
                .unwrap()
                .paths(),
            vec!["b.md"]
        );

        let cancelled_id = begin_turn(&conn, "ws1", "scope-cancel", None).unwrap();
        let cancelled = propose_decision(
            &conn,
            &cancelled_id,
            "scope",
            serde_json::json!({ "targetPaths": ["c.md"] }),
        )
        .unwrap();
        resolve_decision(&conn, &cancelled.request_id, "cancel").unwrap();
        assert!(
            crate::ag::target_set::load_for_task_run(&conn, &cancelled_id)
                .unwrap()
                .is_empty()
        );
    }

    // covers: BR-SYS-STATE-008, BR-AG-DATA-001
    // 连续 Diff 的下一片段只可由真实 LogicalState 回执授权：卡片生成不等于预览，身份/基线不符或
    // 重复消费均失败关闭；等待 Ack 时 TaskRun 不得被 settle 为 completed。
    #[test]
    fn preview_checkpoint_requires_pending_diff_ack_and_consumes_once() {
        let conn = fresh();
        conn.execute(
            "INSERT INTO workspaces (workspace_id, root_path, is_active, opened_at) VALUES ('ws1','/w',1,0)",
            [],
        )
        .unwrap();
        let task_run_id = begin_turn(&conn, "ws1", "s-preview", Some("连续编辑")).unwrap();
        conn.execute(
            "INSERT INTO pending_diffs (diff_id, workspace_id, rel_path, origin_module, status, payload_ref, created_at) \
             VALUES ('diff-preview','ws1','notes/a.md','AG','pending','payload-preview',1)",
            [],
        )
        .unwrap();
        create_preview_checkpoint(
            &conn,
            &task_run_id,
            "ws1",
            "s-preview",
            "turn-preview",
            "diff-preview",
            "notes/a.md",
        )
        .unwrap();
        assert!(has_pending_preview_checkpoint(&conn, &task_run_id).unwrap());
        assert_eq!(
            settle_turn(&conn, &task_run_id, TurnOutcome::Completed).unwrap(),
            Active,
            "等待真实预览时不得把 TaskRun 伪装为 completed"
        );

        let valid = PreviewCheckpoint {
            diff_id: "diff-preview".to_string(),
            task_run_id: task_run_id.clone(),
            workspace_id: "ws1".to_string(),
            session_id: "s-preview".to_string(),
            turn_id: "turn-preview".to_string(),
            rel_path: "notes/a.md".to_string(),
            session_version: 9,
            content_hash: "hash-after-preview".to_string(),
        };
        let mut wrong_session = valid.clone();
        wrong_session.session_id = "s-other".to_string();
        assert!(acknowledge_preview_checkpoint(&conn, &wrong_session).is_err());
        acknowledge_preview_checkpoint(&conn, &valid).unwrap();
        assert!(acknowledge_preview_checkpoint(&conn, &valid).is_err());
        assert!(has_pending_preview_checkpoint(&conn, &task_run_id).unwrap());

        for terminal in ["accepted", "rejected", "expired"] {
            conn.execute(
                "UPDATE pending_diffs SET status=?1 WHERE diff_id='diff-preview'",
                [terminal],
            )
            .unwrap();
            assert_eq!(begin_preview_continuation(&conn, &valid).unwrap(), None);
            assert!(acknowledged_preview_checkpoint(
                &conn,
                &task_run_id,
                "ws1",
                "s-preview",
                "diff-preview"
            )
            .unwrap()
            .is_some());
        }
        conn.execute(
            "UPDATE pending_diffs SET status='pending' WHERE diff_id='diff-preview'",
            [],
        )
        .unwrap();

        let mut stale = valid.clone();
        stale.content_hash = "old-hash".to_string();
        assert_eq!(begin_preview_continuation(&conn, &stale).unwrap(), None);
        assert_eq!(
            begin_preview_continuation(&conn, &valid).unwrap(),
            Some(task_run_id.clone())
        );
        assert_eq!(begin_preview_continuation(&conn, &valid).unwrap(), None);
        assert!(!has_pending_preview_checkpoint(&conn, &task_run_id).unwrap());
    }

    // covers: BR-SYS-STATE-008, BR-AG-STATE-002
    // 模型/IPC 都必须经过同一个裁决守卫：仅受支持 kind、仅 active、每个任务当前只可有一个 pending。
    #[test]
    fn decision_request_rejects_invalid_kind_and_duplicate_without_state_change() {
        let conn = fresh();
        let id = begin_turn(&conn, "ws1", "s1", None).unwrap();

        assert!(propose_decision(&conn, &id, "unknown", serde_json::json!({})).is_err());
        assert_eq!(load_task_run(&conn, &id).unwrap().unwrap().status, Active);

        let decision = propose_decision(
            &conn,
            &id,
            "scope",
            serde_json::json!({ "targetPaths": ["notes/a.md"] }),
        )
        .unwrap();
        assert_eq!(decision.kind, "scope");
        assert_eq!(load_task_run(&conn, &id).unwrap().unwrap().status, Waiting);
        assert!(propose_decision(&conn, &id, "permission", serde_json::json!({})).is_err());
        assert_eq!(
            pending_decision(&conn, &id).unwrap().unwrap().request_id,
            decision.request_id
        );
    }

    // covers: BR-SYS-STATE-008, BR-AG-STATE-002
    // waitResolved 不足以提第二张卡；同 TaskRun 的真实续片段 ToolRequest 才可重新裁决。
    #[test]
    fn decision_request_requires_resumed_activity_before_next_decision() {
        let conn = fresh();
        let task_run_id = begin_turn(&conn, "ws1", "s1", None).unwrap();
        insert_turn(&conn, "turn-decision-chain", "s1", &task_run_id);
        let first = propose_decision(
            &conn,
            &task_run_id,
            "manual_confirm",
            serde_json::json!({ "summary": "A" }),
        )
        .unwrap();
        insert_event(
            &conn,
            "turn-decision-chain",
            "s1",
            1,
            "decision_request_proposed",
            serde_json::json!({"requestId": first.request_id}),
        );
        resolve_decision(&conn, &first.request_id, "confirm").unwrap();
        insert_event(
            &conn,
            "turn-decision-chain",
            "s1",
            2,
            "decision_request_resolved",
            serde_json::json!({"requestId": first.request_id, "outcome": "confirm"}),
        );
        assert_eq!(
            load_task_run(&conn, &task_run_id).unwrap().unwrap().status,
            Waiting
        );
        assert!(propose_decision(
            &conn,
            &task_run_id,
            "manual_confirm",
            serde_json::json!({ "summary": "B" }),
        )
        .is_err());
        assert!(!has_pending_decision(&conn, &task_run_id).unwrap());

        insert_event(
            &conn,
            "turn-decision-chain",
            "s1",
            3,
            "tool_request",
            serde_json::json!({"toolCallId": "call-B", "toolName": "propose_decision"}),
        );
        let second = propose_decision(
            &conn,
            &task_run_id,
            "manual_confirm",
            serde_json::json!({ "summary": "B" }),
        )
        .unwrap();
        assert_ne!(second.request_id, first.request_id);
        assert_eq!(second.task_run_id, task_run_id);
        assert!(has_pending_decision(&conn, &task_run_id).unwrap());
        assert_eq!(
            load_task_run(&conn, &task_run_id).unwrap().unwrap().status,
            Waiting
        );
    }

    // covers: BR-SYS-STATE-008
    // 复杂度只来自模型可见路线/目标/等待点，不读取自然语言关键词。
    #[test]
    fn route_projection_boundary_keeps_simple_turns_empty() {
        let one = vec!["a.md".to_string()];
        let two = vec!["a.md".to_string(), "b.md".to_string()];
        assert!(!route_requires_task_projection(
            &["answer".into()],
            &[],
            &one
        ));
        assert!(!route_requires_task_projection(
            &["read".into(), "propose".into()],
            &[],
            &one
        ));
        assert!(route_requires_task_projection(
            &["inspect".into(), "compare".into(), "report".into()],
            &[],
            &one
        ));
        assert!(route_requires_task_projection(
            &["compare".into()],
            &[],
            &two
        ));
        assert!(route_requires_task_projection(
            &["prepare".into()],
            &["scope".into()],
            &one
        ));
        assert!(tool_requires_task_run("propose_decision"));
        assert!(tool_requires_task_run("plan_workspace_operation"));
        assert!(!tool_requires_task_run("read_doc"));
        assert!(!tool_requires_task_run("propose_edit"));
    }

    // covers: BR-SYS-STATE-008
    #[test]
    fn simple_turn_inputs_do_not_persist_task_run() {
        let conn = fresh();
        let one = vec!["a.md".to_string()];
        assert_eq!(
            begin_for_model_route(
                &conn,
                "ws1",
                "s1",
                "总结文档",
                &["answer".into()],
                &[],
                &one,
            )
            .unwrap(),
            None
        );
        assert_eq!(
            begin_for_model_route(
                &conn,
                "ws1",
                "s1",
                "把第一段加粗",
                &["read".into(), "propose".into()],
                &[],
                &one,
            )
            .unwrap(),
            None
        );
        assert_eq!(
            begin_for_tool_product(&conn, "ws1", "s1", "read_doc").unwrap(),
            None
        );
        assert_eq!(
            begin_for_tool_product(&conn, "ws1", "s1", "propose_edit").unwrap(),
            None
        );
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM task_runs", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    // covers: BR-SYS-STATE-008
    // 重启不续跑：应用启动时陈旧活动态置 Interrupted；新回合新建（retry 创建新回合）。
    #[test]
    fn stale_active_runs_become_interrupted_on_startup() {
        let conn = fresh();
        let id = begin_turn(&conn, "ws1", "s1", None).unwrap();
        insert_turn(&conn, "turn-recovery", "s1", &id);
        conn.execute(
            "UPDATE agent_turns SET status='completed' WHERE turn_id='turn-recovery'",
            [],
        )
        .unwrap();
        let other_workspace_id = begin_turn(&conn, "ws2", "s2", None).unwrap();
        assert_eq!(
            mark_all_stale_as_interrupted(&conn).unwrap(),
            2,
            "启动恢复必须清理所有 workspace 的活动态"
        );
        let run = load_task_run(&conn, &id).unwrap().unwrap();
        assert_eq!(run.status, Interrupted);
        assert_eq!(
            rebuild_task_projection(&conn, "ws1", "s1", &id)
                .unwrap()
                .unwrap()
                .status,
            TaskRunProjectionStatusV2::Interrupted,
            "来源回合已终态时仍必须可追加 Interrupted 恢复观察"
        );
        let other_workspace_run = load_task_run(&conn, &other_workspace_id).unwrap().unwrap();
        assert_eq!(other_workspace_run.status, Interrupted);
        // 中断后新回合不沿用（新建）
        let id2 = begin_turn(&conn, "ws1", "s1", None).unwrap();
        assert_ne!(id, id2);
        assert_eq!(mark_all_stale_as_interrupted(&conn).unwrap(), 1);
    }

    // covers: BR-SYS-STATE-008
    // 回合→TaskRun 关联（agent_turns.task_run_id，UNC-09）。
    #[test]
    fn turn_links_to_task_run() {
        let conn = fresh();
        conn.execute(
            "INSERT INTO agent_turns (turn_id, workspace_id, rel_path, payload_ref, created_at) \
             VALUES ('turn-1', 'ws1', 'notes/a.md', 'payload-1', 1)",
            [],
        )
        .unwrap();
        let task_id = begin_turn(&conn, "ws1", "s1", None).unwrap();
        link_turn(&conn, "turn-1", &task_id).unwrap();
        let got: String = conn
            .query_row(
                "SELECT task_run_id FROM agent_turns WHERE turn_id='turn-1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(got, task_id);
    }

    // covers: BR-SYS-STATE-008, BR-AG-PERSIST-001
    // 相同 TurnEvent 序列重建同一投影；route revision 替换候选路线，Decision 解除后无 activity 仍等待。
    #[test]
    fn task_projection_rebuilds_route_activity_and_waiting_from_events() {
        let conn = fresh();
        assert!(list_task_runs_by_session(&conn, "ws1", "s-empty")
            .unwrap()
            .is_empty());
        let task_run_id = begin_turn(&conn, "ws1", "s1", Some("检查两份材料")).unwrap();
        insert_turn(&conn, "turn-1", "s1", &task_run_id);
        insert_event(
            &conn,
            "turn-1",
            "s1",
            1,
            "task_projected",
            serde_json::json!({"taskRunId": task_run_id, "goal": "检查两份材料"}),
        );
        insert_event(
            &conn,
            "turn-1",
            "s1",
            2,
            "route_updated",
            serde_json::json!({"revision": 1, "summary": "旧路线", "steps": ["列出", "读取", "比较"]}),
        );
        insert_event(
            &conn,
            "turn-1",
            "s1",
            3,
            "tool_request",
            serde_json::json!({"toolCallId": "call-1", "toolName": "list_docs"}),
        );
        insert_event(
            &conn,
            "turn-1",
            "s1",
            4,
            "tool_result",
            serde_json::json!({"toolCallId": "call-1", "toolName": "list_docs", "success": true}),
        );
        insert_event(
            &conn,
            "turn-1",
            "s1",
            5,
            "route_updated",
            serde_json::json!({"revision": 2, "summary": "修订路线", "steps": ["搜索", "比较", "说明"]}),
        );
        insert_event(
            &conn,
            "turn-1",
            "s1",
            6,
            "decision_request_proposed",
            serde_json::json!({"requestId": "decision-1", "kind": "scope", "payload": {}}),
        );
        insert_event(
            &conn,
            "turn-1",
            "s1",
            7,
            "decision_request_resolved",
            serde_json::json!({"requestId": "decision-1", "outcome": "confirm"}),
        );

        let first = rebuild_task_projection(&conn, "ws1", "s1", &task_run_id)
            .unwrap()
            .unwrap();
        assert_eq!(first.status, TaskRunProjectionStatusV2::Waiting);
        assert_eq!(first.current_route_revision, Some(2));
        assert_eq!(first.candidate_steps.len(), 3);
        assert_eq!(first.candidate_steps[0].label, "搜索");
        assert_eq!(first.observed_steps.len(), 2);
        assert_eq!(
            first.observed_steps[0].status,
            TaskStepProjectionStatusV2::Completed
        );
        assert!(first.waiting_on.is_empty());

        let rebuilt = rebuild_task_projection(&conn, "ws1", "s1", &task_run_id)
            .unwrap()
            .unwrap();
        assert_eq!(rebuilt, first);
        assert!(list_task_runs_by_session(&conn, "ws1", "s2")
            .unwrap()
            .is_empty());
    }
}
