//! AG-1.0-IMP-TASKRUN-01 · 受限任务实例编排机 SM-AG-04 · AG-TURN / BR-SYS-STATE-008
//!
//! 同一用户目标内组织步骤/回合的受限编排（TERM-AG-015/016）：非后台队列、不自动重试、不跨重启续跑、
//! 不越过确认点；真实工作状态只由回合/步骤/工具事件驱动。task_runs 表持久化（db 迁移 v11）；
//! 应用启动时将陈旧活动态置 Interrupted（展示"中断可重试"，retry 创建新回合，不续跑）。

use crate::types::{
    DecisionRequestStatus, DecisionRequestV1, TaskPlanStepV1, TaskPlanV1, TaskRunStatus, TaskRunV1,
    TaskStepStatus, TaskStepV1,
};
use rusqlite::{Connection, OptionalExtension};
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

fn is_terminal(status: TaskRunStatus) -> bool {
    matches!(
        status,
        TaskRunStatus::Completed
            | TaskRunStatus::Failed
            | TaskRunStatus::Cancelled
            | TaskRunStatus::Interrupted
    )
}

/// SM-AG-04 事件（A-AG-M-S-01 §2.2 主表）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskRunEvent {
    PlanApproved,
    StepStarted,
    StepOk,
    StepFail,
    WaitingConfirm,
    UserConfirm,
    UserCancel,
    ClarificationNeeded,
    Interrupt,
    Retry,
    Finish,
}

/**
 * @GOV
 * codes:    BR-SYS-STATE-008-RB-AG-AG-TASK-001
 * type:     RB
 * chain:    AG-TURN
 * rules:    BR-SYS-STATE-008, BR-AG-STATE-001
 * boundary: in=TaskRunEvent | out=TaskRunStatus(noneWhenForbiddenNoBackgroundQueue)
 */
/// SM-AG-04 转移引擎（BR-SYS-STATE-008）：返回 `None` = 无合法转移。forbidden：Executing→Completed
/// 未经 finish；WaitingConfirm/WaitingClarification 未经用户裁决/补充不得推进；Interrupted→执行无
/// 用户重试；任一态→后台队列不存在（本机无队列概念）。主表见 A-AG-M-S-01 §2.2。
pub fn transition(state: TaskRunStatus, event: TaskRunEvent) -> Option<TaskRunStatus> {
    use TaskRunEvent::{ClarificationNeeded, Finish, Interrupt, PlanApproved, Retry, StepFail, StepOk, StepStarted, UserCancel, UserConfirm, WaitingConfirm};
    use TaskRunStatus as S;
    let next = match (state, event) {
        (S::Planned, PlanApproved) => S::Executing,
        (S::Executing, StepStarted) => S::Executing,
        (S::Executing, StepOk) => S::Executing,
        (S::Executing, WaitingConfirm) => S::WaitingConfirm,
        (S::Executing, ClarificationNeeded) => S::WaitingClarification,
        (S::WaitingConfirm, UserConfirm) => S::Executing,
        (S::WaitingConfirm, UserCancel) => S::Cancelled,
        (S::Executing, StepFail) => S::Failed,
        (S::Executing, Interrupt) => S::Interrupted,
        (S::WaitingConfirm, Interrupt) | (S::WaitingClarification, Interrupt) => S::Interrupted,
        (S::Interrupted, Retry) => S::Planned,
        (S::Executing, Finish) => S::Completed,
        (S::WaitingConfirm, Finish) => S::Completed,
        _ => return None,
    };
    Some(next)
}

/// 回合终态（lib.rs 回合循环映射）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnOutcome {
    Completed,
    Failed,
    Cancelled,
}

/// 回合开始：返回当前回合的 task_run_id。
/// - 会话内最近存在**非终态** TaskRun → 沿用（同一目标接力；如上一回合 WaitingConfirm 待裁决后继续）；
/// - 否则新建（Planned→Executing）。
pub fn begin_turn(
    conn: &Connection,
    workspace_id: &str,
    session_id: &str,
    intent_summary: Option<&str>,
) -> rusqlite::Result<String> {
    if let Some(id) = latest_non_terminal_id(conn, workspace_id, session_id)? {
        let now = now_millis();
        conn.execute(
            "UPDATE task_runs SET status=?1, updated_at=?2 WHERE task_run_id=?3",
            rusqlite::params!["executing", now, id],
        )?;
        if let Some(summary) = intent_summary {
            conn.execute(
                "UPDATE task_runs SET intent_summary=?1 WHERE task_run_id=?2",
                rusqlite::params![summary, id],
            )?;
        }
        return Ok(id);
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
         VALUES (?1, ?2, ?3, 'executing', ?4, '[]', ?5, ?5)",
        rusqlite::params![id, workspace_id, session_id, intent_summary, now],
    )?;
    Ok(id)
}

fn latest_non_terminal_id(
    conn: &Connection,
    workspace_id: &str,
    session_id: &str,
) -> rusqlite::Result<Option<String>> {
    let mut stmt = conn.prepare(
        // AG-1.0-TASKRUN-REWORK-IMP-RUNTIME-01：waiting_confirm 不沿用（不绕回 Executing）——
        // 存在未决 DecisionRequest 时由回合入口 gate 拒绝新消息；此处在数据层双保险。
        "SELECT task_run_id, status FROM task_runs \
         WHERE workspace_id=?1 AND session_id=?2 AND status NOT IN \
         ('completed','failed','cancelled','interrupted','waiting_confirm') \
         ORDER BY updated_at DESC LIMIT 1",
    )?;
    stmt.query_row(
        rusqlite::params![workspace_id, session_id],
        |r| r.get(0),
    )
    .optional()
}

/// 记录工具步骤（真实工具事件驱动）：终态一次写入（ok/failed）。
pub fn record_tool_step(
    conn: &Connection,
    task_run_id: &str,
    step_id: &str,
    label: &str,
    ok: bool,
) -> rusqlite::Result<()> {
    let status = if ok { "ok" } else { "failed" };
    append_step(conn, task_run_id, "tool", label, status, step_id)
}

/// 记录提案步骤（＝W1/W2/W3 确认点）：进入 waiting_confirm。
pub fn record_proposal_step(
    conn: &Connection,
    task_run_id: &str,
    step_id: &str,
    label: &str,
) -> rusqlite::Result<()> {
    append_step(conn, task_run_id, "propose", label, "waiting_confirm", step_id)
}

fn append_step(
    conn: &Connection,
    task_run_id: &str,
    kind: &str,
    label: &str,
    status: &str,
    step_id: &str,
) -> rusqlite::Result<()> {
    let mut steps: Vec<TaskStepV1> = load_steps(conn, task_run_id)?;
    steps.push(TaskStepV1 {
        step_id: step_id.to_string(),
        kind: kind.to_string(),
        label: label.to_string(),
        status: parse_step_status(status).unwrap_or(TaskStepStatus::Ok),
    });
    let json = serde_json::to_string(&steps).unwrap_or_else(|_| "[]".to_string());
    conn.execute(
        "UPDATE task_runs SET steps_json=?1, updated_at=?2 WHERE task_run_id=?3",
        rusqlite::params![json, now_millis(), task_run_id],
    )?;
    Ok(())
}

fn load_steps(conn: &Connection, task_run_id: &str) -> rusqlite::Result<Vec<TaskStepV1>> {
    let raw: String = conn
        .query_row(
            "SELECT steps_json FROM task_runs WHERE task_run_id=?1",
            rusqlite::params![task_run_id],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| "[]".to_string());
    Ok(serde_json::from_str(&raw).unwrap_or_default())
}

fn parse_step_status(s: &str) -> Option<TaskStepStatus> {
    match s {
        "pending" => Some(TaskStepStatus::Pending),
        "running" => Some(TaskStepStatus::Running),
        "waiting_confirm" => Some(TaskStepStatus::WaitingConfirm),
        "ok" => Some(TaskStepStatus::Ok),
        "failed" => Some(TaskStepStatus::Failed),
        "cancelled" => Some(TaskStepStatus::Cancelled),
        _ => None,
    }
}

/// 回合终态映射（BR-SYS-STATE-008 · A-AG-M-S-01 §2.2.2 重构）：完成且有未决 DecisionRequest →
/// WaitingConfirm（保持非终态，等待任务级裁决后自动下一片段）；完成无未决裁决 → Completed；
/// 失败 → Failed；取消 → Cancelled。提案（Diff）不再驱动 waiting_confirm（三层分离）。
pub fn settle_turn(
    conn: &Connection,
    task_run_id: &str,
    outcome: TurnOutcome,
) -> rusqlite::Result<TaskRunStatus> {
    let status = match outcome {
        TurnOutcome::Completed if has_pending_decision(conn, task_run_id)? => {
            TaskRunStatus::WaitingConfirm
        }
        TurnOutcome::Completed => TaskRunStatus::Completed,
        TurnOutcome::Failed => TaskRunStatus::Failed,
        TurnOutcome::Cancelled => TaskRunStatus::Cancelled,
    };
    conn.execute(
        "UPDATE task_runs SET status=?1, updated_at=?2 WHERE task_run_id=?3",
        rusqlite::params![serde_json::to_string(&status).unwrap().trim_matches('"'), now_millis(), task_run_id],
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
    stmt.query_row(
        rusqlite::params![workspace_id, session_id],
        |r| r.get(0),
    )
    .optional()
}

/// 发起任务级裁决请求（TERM-AG-017）：INSERT + 置 task_run 为 waiting_confirm。
/// 仅此处可将 TaskRun 置 WaitingConfirm（confirm_point 唯一来源）。
pub fn propose_decision(
    conn: &Connection,
    task_run_id: &str,
    kind: &str,
    payload: serde_json::Value,
) -> rusqlite::Result<DecisionRequestV1> {
    let request_id = format!(
        "decision-{}-{}",
        task_run_id,
        now_millis() - (TASK_SEQ.fetch_add(1, Ordering::Relaxed) as i64 % 1000)
    );
    let now = now_millis();
    conn.execute(
        "INSERT INTO decision_requests (request_id, task_run_id, kind, payload_json, status, outcome, proposed_at, resolved_at) \
         VALUES (?1, ?2, ?3, ?4, 'proposed', NULL, ?5, NULL)",
        rusqlite::params![request_id, task_run_id, kind, payload.to_string(), now],
    )?;
    conn.execute(
        "UPDATE task_runs SET status='waiting_confirm', updated_at=?1 WHERE task_run_id=?2",
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

/// 裁决任务级请求（userConfirm/userCancel 真实入口）：confirm → Executing（可自动下一片段），
/// cancel → Cancelled。返回裁决后的 task_run 状态。
pub fn resolve_decision(
    conn: &Connection,
    request_id: &str,
    outcome: &str,
) -> rusqlite::Result<TaskRunStatus> {
    let task_run_id: String = conn.query_row(
        "SELECT task_run_id FROM decision_requests WHERE request_id=?1 AND status='proposed'",
        rusqlite::params![request_id],
        |r| r.get(0),
    )?;
    let now = now_millis();
    conn.execute(
        "UPDATE decision_requests SET status='resolved', outcome=?1, resolved_at=?2 WHERE request_id=?3",
        rusqlite::params![outcome, now, request_id],
    )?;
    let status = if outcome == "confirm" {
        TaskRunStatus::Executing
    } else {
        TaskRunStatus::Cancelled
    };
    conn.execute(
        "UPDATE task_runs SET status=?1, updated_at=?2 WHERE task_run_id=?3",
        rusqlite::params![
            serde_json::to_string(&status).unwrap().trim_matches('"'),
            now,
            task_run_id
        ],
    )?;
    Ok(status)
}

fn decision_row(
    r: &rusqlite::Row<'_>,
) -> rusqlite::Result<DecisionRequestV1> {
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

/// 应用启动清理（不跨重启续跑，BR-SYS-STATE-008）：把 workspace 内所有非终态 TaskRun 置
/// Interrupted（展示"中断可重试"）；新回合 begin_turn 将新建（retry 创建新回合）。
pub fn mark_stale_as_interrupted(conn: &Connection, workspace_id: &str) -> rusqlite::Result<usize> {
    let n = conn.execute(
        "UPDATE task_runs SET status='interrupted', updated_at=?1 \
         WHERE workspace_id=?2 AND status NOT IN \
         ('completed','failed','cancelled','interrupted')",
        rusqlite::params![now_millis(), workspace_id],
    )?;
    Ok(n)
}

/// 读取 TaskRun（含步骤）。
pub fn load_task_run(conn: &Connection, task_run_id: &str) -> rusqlite::Result<Option<TaskRunV1>> {
    row_to_run(
        conn
            .query_row(
                "SELECT task_run_id, workspace_id, session_id, status, intent_summary, steps_json, created_at, updated_at \
                 FROM task_runs WHERE task_run_id=?1",
                rusqlite::params![task_run_id],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, Option<String>>(4)?,
                        r.get::<_, String>(5)?,
                        r.get::<_, i64>(6)?,
                        r.get::<_, i64>(7)?,
                    ))
                },
            )
            .optional()?,
    )
}

/// AG-1.0-IMP-TASKRUN-UI-01：按会话列出 TaskRun（updated_at 倒序，最近在前），供前端会话摘要呈现。
pub fn list_task_runs_by_session(
    conn: &Connection,
    workspace_id: &str,
    session_id: &str,
) -> rusqlite::Result<Vec<TaskRunV1>> {
    let mut stmt = conn.prepare(
        "SELECT task_run_id, workspace_id, session_id, status, intent_summary, steps_json, created_at, updated_at \
         FROM task_runs WHERE workspace_id=?1 AND session_id=?2 \
         ORDER BY updated_at DESC",
    )?;
    let rows = stmt
        .query_map(rusqlite::params![workspace_id, session_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, i64>(6)?,
                r.get::<_, i64>(7)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows
        .into_iter()
        .map(|(id, ws, sess, status, summary, steps, created, updated)| TaskRunV1 {
            task_run_id: id,
            workspace_id: ws,
            session_id: sess,
            status: parse_run_status(&status).unwrap_or(TaskRunStatus::Planned),
            intent_summary: summary,
            steps: serde_json::from_str(&steps).unwrap_or_default(),
            created_at: created,
            updated_at: updated,
        })
        .collect())
}

fn row_to_run(
    row: Option<(
        String,
        String,
        String,
        String,
        Option<String>,
        String,
        i64,
        i64,
    )>,
) -> rusqlite::Result<Option<TaskRunV1>> {
    Ok(row.map(|(id, ws, sess, status, summary, steps, created, updated)| TaskRunV1 {
        task_run_id: id,
        workspace_id: ws,
        session_id: sess,
        status: parse_run_status(&status).unwrap_or(TaskRunStatus::Planned),
        intent_summary: summary,
        steps: serde_json::from_str(&steps).unwrap_or_default(),
        created_at: created,
        updated_at: updated,
    }))
}

/// AG-1.0-TASKRUN-REWORK-INTENT-01：判定是否需要对当前目标建立 TaskPlan（TERM-AG-018）。
/// 复杂 = 写意图（edit/create）+（多目标文档 或 单文档整篇改写信号）；简单单文档局部编辑直行不建。
pub fn is_complex_target(is_write_intent: bool, target_paths: &[String], prompt: &str) -> bool {
    if !is_write_intent || target_paths.is_empty() {
        return false;
    }
    if target_paths.len() >= 2 {
        return true;
    }
    ["整篇", "全文", "整个文档", "全部内容", "所有段落", "每一段", "整体"]
        .iter()
        .any(|kw| prompt.contains(kw))
}

/// AG-1.0-TASKRUN-REWORK-INTENT-01 · 保存 TaskPlan（TERM-AG-018，可展示操作计划，非思维链）：
/// task_plans 主行 + task_plan_steps 明细。
pub fn create_plan(conn: &Connection, plan: &TaskPlanV1) -> rusqlite::Result<()> {
    let now = now_millis();
    conn.execute(
        "INSERT INTO task_plans (plan_id, task_run_id, plan_json, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            plan.plan_id,
            plan.task_run_id,
            serde_json::to_string(&plan).unwrap_or_else(|_| "{}".to_string()),
            now,
            now
        ],
    )?;
    for step in &plan.steps {
        conn.execute(
            "INSERT OR REPLACE INTO task_plan_steps \
             (step_id, plan_id, seq, kind, label, deps_json, required_grant, confirm_point, status, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                step.step_id,
                plan.plan_id,
                step.seq,
                step.kind,
                step.label,
                serde_json::to_string(&step.deps).unwrap_or_else(|_| "[]".to_string()),
                step.required_grant,
                if step.confirm_point { 1 } else { 0 },
                serde_json::to_string(&step.status).unwrap_or_else(|_| "\"pending\"".to_string()),
                now
            ],
        )?;
    }
    Ok(())
}

/// 按 run 列出 TaskPlan（updated_at 倒序）。
pub fn list_plans_by_run(
    conn: &Connection,
    task_run_id: &str,
) -> rusqlite::Result<Vec<TaskPlanV1>> {
    let mut stmt = conn.prepare(
        "SELECT plan_id, task_run_id, plan_json FROM task_plans \
         WHERE task_run_id=?1 ORDER BY updated_at DESC",
    )?;
    let rows = stmt
        .query_map(rusqlite::params![task_run_id], |r| {
            let plan_json: String = r.get(2)?;
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                plan_json,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows
        .into_iter()
        .filter_map(|(plan_id, run_id, plan_json)| {
            let mut plan: TaskPlanV1 =
                serde_json::from_str(&plan_json).unwrap_or(TaskPlanV1 {
                    plan_id: plan_id.clone(),
                    task_run_id: run_id.clone(),
                    steps: Vec::new(),
                    cursor: 0,
                    created_at: 0,
                    updated_at: 0,
                });
            plan.plan_id = plan_id;
            plan.task_run_id = run_id;
            Some(plan)
        })
        .collect())
}

fn parse_run_status(s: &str) -> Option<TaskRunStatus> {
    match s {
        "planned" => Some(TaskRunStatus::Planned),
        "executing" => Some(TaskRunStatus::Executing),
        "waiting_confirm" => Some(TaskRunStatus::WaitingConfirm),
        "waiting_clarification" => Some(TaskRunStatus::WaitingClarification),
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
    use super::*;
    use crate::db;
    use crate::types::TaskRunStatus::*;

    fn fresh() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        db::migrate(&mut conn).unwrap();
        conn
    }

    // covers: BR-SYS-STATE-008
    // SM-AG-04 主表正路 + forbidden（无后台队列/未经裁决不推进/中断不自动续跑）。
    #[test]
    fn sm_ag04_transition_matrix() {
        use crate::types::TaskRunStatus as S;
        use TaskRunEvent as E;
        assert_eq!(transition(S::Planned, E::PlanApproved), Some(S::Executing));
        assert_eq!(transition(S::Executing, E::StepStarted), Some(S::Executing));
        assert_eq!(transition(S::Executing, E::StepOk), Some(S::Executing));
        assert_eq!(transition(S::Executing, E::WaitingConfirm), Some(S::WaitingConfirm));
        assert_eq!(transition(S::Executing, E::ClarificationNeeded), Some(S::WaitingClarification));
        assert_eq!(transition(S::WaitingConfirm, E::UserConfirm), Some(S::Executing));
        assert_eq!(transition(S::WaitingConfirm, E::UserCancel), Some(S::Cancelled));
        assert_eq!(transition(S::Executing, E::StepFail), Some(S::Failed));
        assert_eq!(transition(S::Executing, E::Interrupt), Some(S::Interrupted));
        assert_eq!(transition(S::Interrupted, E::Retry), Some(S::Planned));
        assert_eq!(transition(S::Executing, E::Finish), Some(S::Completed));
        assert_eq!(transition(S::WaitingConfirm, E::Finish), Some(S::Completed));
        // forbidden：Executing 未经 finish 不得 Completed；WaitingConfirm 未经裁决不得推进；Interrupted 无重试不得执行
        assert_eq!(transition(S::WaitingConfirm, E::StepStarted), None);
        assert_eq!(transition(S::WaitingConfirm, E::StepFail), None);
        assert_eq!(transition(S::Interrupted, E::StepStarted), None);
        assert_eq!(transition(S::Completed, E::StepStarted), None);
        assert_eq!(transition(S::Interrupted, E::Finish), None);
    }

    // covers: BR-SYS-STATE-008
    // 每回合建/沿用 TaskRun：首次新建 executing；WaitingConfirm 延续沿用；终态后新建。
    #[test]
    fn begin_turn_reuses_non_terminal_and_creates_new_after_terminal() {
        let conn = fresh();
        let id1 = begin_turn(&conn, "ws1", "s1", Some("分析文档")).unwrap();
        assert!(conn
            .query_row(
                "SELECT COUNT(*) FROM task_runs WHERE task_run_id=?1",
                rusqlite::params![id1],
                |r| r.get::<_, i64>(0),
            )
            .unwrap()
            > 0);
        // 沿用（非终态）
        let id2 = begin_turn(&conn, "ws1", "s1", Some("继续")).unwrap();
        assert_eq!(id1, id2);
        // 回合完成（无未决裁决）→ 终态 → 下次新建
        let st = settle_turn(&conn, &id1, TurnOutcome::Completed).unwrap();
        assert_eq!(st, Completed);
        let id3 = begin_turn(&conn, "ws1", "s1", Some("新目标")).unwrap();
        assert_ne!(id1, id3);
    }

    // covers: BR-SYS-STATE-008
    // 三层分离（A-AG-M-S-01 §2.2.2）：提案步骤只记步骤不驱动 run 状态；WaitingConfirm 唯一来源 =
    // DecisionRequest；任意新消息不绕回 waiting_confirm（begin_turn 新建）；裁决 confirm → Executing。
    #[test]
    fn decision_request_drives_waiting_confirm_and_resolution() {
        let conn = fresh();
        let id = begin_turn(&conn, "ws1", "s1", None).unwrap();
        record_proposal_step(&conn, &id, "step-p1", "修改 notes/a.md").unwrap();
        let run = load_task_run(&conn, &id).unwrap().unwrap();
        assert_eq!(run.steps.len(), 1);
        assert_eq!(run.steps[0].status, TaskStepStatus::WaitingConfirm);
        // 三层分离：提案步骤不改变 run 状态（仍 executing）
        assert_eq!(run.status, Executing);
        // 任务级裁决请求 → WaitingConfirm
        let dr = propose_decision(
            &conn,
            &id,
            "plan_choice",
            serde_json::json!({ "note": "确认方案" }),
        )
        .unwrap();
        assert_eq!(dr.status, DecisionRequestStatus::Proposed);
        assert_eq!(load_task_run(&conn, &id).unwrap().unwrap().status, WaitingConfirm);
        assert!(has_pending_decision(&conn, &id).unwrap());
        // settle 保持 WaitingConfirm（有未决裁决）
        let st = settle_turn(&conn, &id, TurnOutcome::Completed).unwrap();
        assert_eq!(st, WaitingConfirm);
        // 任意新消息不绕回：begin_turn 不沿用 waiting_confirm → 新建
        let id2 = begin_turn(&conn, "ws1", "s1", Some("继续")).unwrap();
        assert_ne!(id, id2);
        // 裁决 confirm → Executing（可自动下一片段）
        let st2 = resolve_decision(&conn, &dr.request_id, "confirm").unwrap();
        assert_eq!(st2, Executing);
        assert!(!has_pending_decision(&conn, &id).unwrap());
    }

    // covers: BR-SYS-STATE-008
    // 裁决取消（userCancel）→ Cancelled；重复裁决拒绝。
    #[test]
    fn decision_cancel_cancels_run_and_rejects_duplicate() {
        let conn = fresh();
        let id = begin_turn(&conn, "ws1", "s1", None).unwrap();
        let dr = propose_decision(&conn, &id, "permission", serde_json::json!({})).unwrap();
        let st = resolve_decision(&conn, &dr.request_id, "cancel").unwrap();
        assert_eq!(st, Cancelled);
        // 已 resolved 的 request 不可再裁决
        assert!(resolve_decision(&conn, &dr.request_id, "confirm").is_err());
    }

    // covers: BR-SYS-STATE-008
    // TaskPlan 持久化（TERM-AG-018）：保存/按 run 列出（可展示操作计划，非思维链）。
    #[test]
    fn task_plan_persists_and_lists() {
        let conn = fresh();
        let id = begin_turn(&conn, "ws1", "s1", Some("复合目标")).unwrap();
        let plan = TaskPlanV1 {
            plan_id: "plan-1".to_string(),
            task_run_id: id.clone(),
            steps: vec![
                TaskPlanStepV1 {
                    step_id: "s1".to_string(),
                    seq: 0,
                    kind: "read".to_string(),
                    label: "读取 a.md".to_string(),
                    deps: vec![],
                    required_grant: Some("read_doc".to_string()),
                    confirm_point: false,
                    status: TaskStepStatus::Ok,
                },
                TaskPlanStepV1 {
                    step_id: "s2".to_string(),
                    seq: 1,
                    kind: "propose".to_string(),
                    label: "生成修改建议".to_string(),
                    deps: vec!["s1".to_string()],
                    required_grant: Some("propose_edit".to_string()),
                    confirm_point: true,
                    status: TaskStepStatus::WaitingConfirm,
                },
            ],
            cursor: 2,
            created_at: 1,
            updated_at: 2,
        };
        create_plan(&conn, &plan).unwrap();
        let plans = list_plans_by_run(&conn, &id).unwrap();
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].plan_id, "plan-1");
        assert_eq!(plans[0].steps.len(), 2);
        assert!(plans[0].steps[1].confirm_point);
        assert_eq!(plans[0].steps[1].deps, vec!["s1".to_string()]);
        assert_eq!(plans[0].cursor, 2);
    }

    // covers: BR-SYS-STATE-008
    // AG-1.0-TASKRUN-REWORK-INTENT-01：复杂目标判定（多文档 / 单文档整篇信号建计划；简单局部编辑直行）。
    #[test]
    fn complex_target_detection() {
        let one = vec!["a.md".to_string()];
        let two = vec!["a.md".to_string(), "b.md".to_string()];
        // 写意图 + 多文档 → 建计划
        assert!(is_complex_target(true, &two, "把两个文档统一风格"));
        // 写意图 + 单文档整篇信号 → 建计划
        assert!(is_complex_target(true, &one, "把整篇文档改成正式风格"));
        // 写意图 + 单文档局部（无信号）→ 直行不建
        assert!(!is_complex_target(true, &one, "把第一段加粗"));
        // 只读意图 → 不建
        assert!(!is_complex_target(false, &two, "总结两个文档"));
        // 无目标 → 不建
        assert!(!is_complex_target(true, &[], "你好"));
    }

    // covers: BR-SYS-STATE-008
    // 重启不续跑：陈旧活动态置 Interrupted；新回合新建（retry 创建新回合）。
    #[test]
    fn stale_active_runs_become_interrupted_on_startup() {
        let conn = fresh();
        let id = begin_turn(&conn, "ws1", "s1", None).unwrap();
        assert_eq!(
            mark_stale_as_interrupted(&conn, "ws1").unwrap(),
            1,
            "活动态应置 Interrupted"
        );
        let run = load_task_run(&conn, &id).unwrap().unwrap();
        assert_eq!(run.status, Interrupted);
        // 中断后新回合不沿用（新建）
        let id2 = begin_turn(&conn, "ws1", "s1", None).unwrap();
        assert_ne!(id, id2);
        assert_eq!(mark_stale_as_interrupted(&conn, "ws1").unwrap(), 1);
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

    // covers: BR-SYS-STATE-008
    // AG-1.0-IMP-TASKRUN-UI-01：按会话列 TaskRun（倒序、含步骤）；空会话返回空列表；跨会话隔离。
    #[test]
    fn list_task_runs_by_session_orders_and_isolates() {
        let conn = fresh();
        // 空会话 → []
        assert!(list_task_runs_by_session(&conn, "ws1", "s-empty")
            .unwrap()
            .is_empty());
        // 会话 s1：回合1（完成）+ 回合2（进行中，含任务级裁决确认点）
        let id1 = begin_turn(&conn, "ws1", "s1", Some("目标一")).unwrap();
        let st = settle_turn(&conn, &id1, TurnOutcome::Completed).unwrap();
        assert_eq!(st, Completed);
        let id2 = begin_turn(&conn, "ws1", "s1", Some("目标二")).unwrap();
        record_tool_step(&conn, &id2, "t1", "read_doc", true).unwrap();
        record_proposal_step(&conn, &id2, "p1", "提案 notes/a.md").unwrap();
        // 任务级裁决请求 → 回合完成保持 WaitingConfirm（非终态，等裁决后自动下一片段）
        let _dr = propose_decision(&conn, &id2, "manual_confirm", serde_json::json!({})).unwrap();
        let st2 = settle_turn(&conn, &id2, TurnOutcome::Completed).unwrap();
        assert_eq!(st2, WaitingConfirm);
        // 其他会话 s2 隔离
        let id3 = begin_turn(&conn, "ws1", "s2", None).unwrap();
        assert_ne!(id2, id3);

        let runs = list_task_runs_by_session(&conn, "ws1", "s1").unwrap();
        assert_eq!(runs.len(), 2);
        // 倒序：id2（更新更晚）在前
        assert_eq!(runs[0].task_run_id, id2);
        assert_eq!(runs[1].task_run_id, id1);
        assert_eq!(runs[0].status, WaitingConfirm);
        assert_eq!(runs[0].steps.len(), 2);
        assert_eq!(runs[0].steps[0].status, TaskStepStatus::Ok);
        assert_eq!(runs[0].steps[1].status, TaskStepStatus::WaitingConfirm);
        // 跨会话：s2 只有自己的 run
        let runs2 = list_task_runs_by_session(&conn, "ws1", "s2").unwrap();
        assert_eq!(runs2.len(), 1);
        assert_eq!(runs2[0].task_run_id, id3);
    }
}
