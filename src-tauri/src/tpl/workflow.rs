//! PI-TPL-02 · 工作流执行机 SM-TPL-01 · TPL-FLOW / BR-TPL-STATE-001
//!
//! 承接 A-TPL-M-S-01 SM-TPL-01：多步工作流编排——**确认点强制暂停**（有确认点的步须经用户 confirm
//! 方执行）；**失败不静默跳步**（stepFail→Paused，跳步须用户 skipConfirmed 显式留痕）；**中止无半成品**
//! （abort→Aborted，产物均在 DE diff 路径未裁决即不落）；**重启无悬挂**（run 状态持久化于 workflow_runs，
//! 可恢复到中断点）。步骤实际委托 SM-AG-01/SM-CBT-01（X-INTER），本机只承接编排转移与 run 持久化。

use rusqlite::Connection;
use std::time::{SystemTime, UNIX_EPOCH};

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// SM-TPL-01 状态（A-TPL-M-S-01 §2）。Completed/Aborted 为终态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowState {
    Idle,
    AwaitingConfirm,
    StepRunning,
    Paused,
    Completed,
    Aborted,
}

/// SM-TPL-01 事件。确认点/后续步是否需确认/是否有后续由步骤定义携带。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowEvent {
    Start { first_has_confirm: bool },
    ConfirmStep,
    StepOk { next_has_confirm: bool }, // 有后续步
    AllDone,                           // 末步完成
    StepFail,
    Pause,
    Resume,
    SkipConfirmed { has_more: bool }, // 用户显式跳过失败步（留痕）
    Abort,
}

/// 终态判定。
pub fn is_terminal(state: WorkflowState) -> bool {
    matches!(state, WorkflowState::Completed | WorkflowState::Aborted)
}

/**
 * @GOV
 * codes:    BR-TPL-STATE-001-RB-TPL-TPL-FLOW-001
 * type:     RB
 * chain:    TPL-FLOW
 * rules:    BR-TPL-STATE-001
 * boundary: in=WorkflowEvent | out=WorkflowState(confirmGatedNoSilentSkipNoHalfProduct)
 */
/// SM-TPL-01 转移引擎（BR-TPL-STATE-001）：有确认点的步须经 AwaitingConfirm+confirmStep 方执行（确认点
/// 强制）；stepFail 只到 Paused（**不静默跳步**），跳步须 Paused+skipConfirmed 显式；abort 通往 Aborted
/// （无半成品）；终态不再转移。返回 `None` = 无合法转移（含全部 forbidden）。
pub fn transition(state: WorkflowState, event: WorkflowEvent) -> Option<WorkflowState> {
    use WorkflowEvent::*;
    use WorkflowState::*;
    let next = match (state, event) {
        // 启动：首步有确认点→等待确认；否则直接执行。
        (
            Idle,
            Start {
                first_has_confirm: true,
            },
        ) => AwaitingConfirm,
        (
            Idle,
            Start {
                first_has_confirm: false,
            },
        ) => StepRunning,
        // 确认点：用户确认后执行。
        (AwaitingConfirm, ConfirmStep) => StepRunning,
        // 步成功且有后续：后续有确认点→等待确认；否则续行。
        (
            StepRunning,
            StepOk {
                next_has_confirm: true,
            },
        ) => AwaitingConfirm,
        (
            StepRunning,
            StepOk {
                next_has_confirm: false,
            },
        ) => StepRunning,
        // 末步完成 → 闭合。
        (StepRunning, AllDone) => Completed,
        // 失败 → 暂停（不静默跳步）；用户暂停 → 暂停。
        (StepRunning, StepFail) => Paused,
        (StepRunning, Pause) => Paused,
        // 暂停后处置：重试/续行；显式跳过（有后续续行，无后续闭合）。
        (Paused, Resume) => StepRunning,
        (Paused, SkipConfirmed { has_more: true }) => StepRunning,
        (Paused, SkipConfirmed { has_more: false }) => Completed,
        // 中止（任一活动态）→ Aborted（无半成品）。
        (AwaitingConfirm, Abort) | (StepRunning, Abort) | (Paused, Abort) => Aborted,
        // 其余：无合法转移（终态不再转移；StepRunning 不得绕过 Paused 跳步；不得绕过确认点）。
        _ => return None,
    };
    Some(next)
}

/**
 * @GOV
 * codes:    BR-TPL-STATE-001-EFFECT-TPL-TPL-FLOW-001
 * type:     EFFECT
 * chain:    TPL-FLOW
 * rules:    BR-TPL-STATE-001
 * boundary: in=workflowRunState | out=persistedRun(restartRecoverable)
 */
/// 持久化 run 状态（BR-TPL-STATE-001「无悬挂」）：写/更新 workflow_runs（current_step + status），
/// 重启后可由 recover_runs 恢复到中断点。status 用状态名（如 "Paused"/"AwaitingConfirm"）。
pub fn save_run(
    conn: &Connection,
    run_id: &str,
    workspace_id: &str,
    workflow_id: &str,
    current_step: i64,
    status: &str,
) -> rusqlite::Result<()> {
    let now = now_millis();
    conn.execute(
        "INSERT INTO workflow_runs \
         (run_id, workspace_id, workflow_id, current_step, status, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6) \
         ON CONFLICT(run_id) DO UPDATE SET current_step=?4, status=?5, updated_at=?6",
        (run_id, workspace_id, workflow_id, current_step, status, now),
    )?;
    Ok(())
}

/**
 * @GOV
 * codes:    BR-TPL-STATE-001-QUERY-TPL-TPL-FLOW-001
 * type:     QUERY
 * chain:    TPL-FLOW
 * rules:    BR-TPL-STATE-001
 * boundary: in=workspaceId | out=resumableRuns(noDanglingNonTerminalOnly)
 */
/// 恢复非终态 run（BR-TPL-STATE-001「无悬挂」）：返回当前 workspace 处于非终态（Completed/Aborted
/// 以外）的 run，供重启后恢复到中断点；已 Completed/Aborted 不返回（不悬挂、不重复执行）。
pub fn recover_runs(
    conn: &Connection,
    workspace_id: &str,
) -> rusqlite::Result<Vec<(String, i64, String)>> {
    let mut stmt = conn.prepare(
        "SELECT run_id, current_step, status FROM workflow_runs \
         WHERE workspace_id=?1 AND status NOT IN ('Completed','Aborted') ORDER BY updated_at ASC",
    )?;
    let rows = stmt.query_map([workspace_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::WorkflowEvent::*;
    use super::WorkflowState::*;
    use super::*;
    use crate::db;
    use rusqlite::Connection;

    fn fresh() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        db::migrate(&mut conn).unwrap();
        conn
    }

    // covers: BR-TPL-STATE-001
    // SM-TPL-01：确认点强制（有确认点须 confirm 方执行）；正路逐步直至 Completed；中止→Aborted。
    #[test]
    fn confirm_gated_lifecycle_and_abort() {
        // 首步有确认点：Idle→AwaitingConfirm，须 confirm 才执行
        assert_eq!(
            transition(
                Idle,
                Start {
                    first_has_confirm: true
                }
            ),
            Some(AwaitingConfirm)
        );
        assert_eq!(transition(AwaitingConfirm, ConfirmStep), Some(StepRunning));
        // 不得绕过确认点直接在 StepRunning confirm
        assert_eq!(transition(StepRunning, ConfirmStep), None, "不得绕过确认点");
        // 后续步有确认点 → 回到 AwaitingConfirm（强制）
        assert_eq!(
            transition(
                StepRunning,
                StepOk {
                    next_has_confirm: true
                }
            ),
            Some(AwaitingConfirm)
        );
        // 无确认点续行 / 末步完成
        assert_eq!(
            transition(
                StepRunning,
                StepOk {
                    next_has_confirm: false
                }
            ),
            Some(StepRunning)
        );
        assert_eq!(transition(StepRunning, AllDone), Some(Completed));
        // 首步无确认点直接执行
        assert_eq!(
            transition(
                Idle,
                Start {
                    first_has_confirm: false
                }
            ),
            Some(StepRunning)
        );
        // 中止 → Aborted（无半成品）
        assert_eq!(transition(StepRunning, Abort), Some(Aborted));
        assert_eq!(transition(AwaitingConfirm, Abort), Some(Aborted));
    }

    // covers: BR-TPL-STATE-001
    // 失败不静默跳步：stepFail→Paused；跳步须显式 skipConfirmed；终态不再转移。
    #[test]
    fn failure_does_not_silently_skip_and_terminals_frozen() {
        // 失败只到 Paused，不自动前进
        assert_eq!(transition(StepRunning, StepFail), Some(Paused));
        // StepRunning 不得绕过 Paused 直接跳步
        assert_eq!(
            transition(StepRunning, SkipConfirmed { has_more: true }),
            None
        );
        // 暂停后：重试 / 显式跳过（留痕）
        assert_eq!(transition(Paused, Resume), Some(StepRunning));
        assert_eq!(
            transition(Paused, SkipConfirmed { has_more: true }),
            Some(StepRunning)
        );
        assert_eq!(
            transition(Paused, SkipConfirmed { has_more: false }),
            Some(Completed)
        );
        // 终态不再转移
        for ev in [ConfirmStep, AllDone, StepFail, Resume, Abort] {
            assert_eq!(transition(Completed, ev), None);
            assert_eq!(transition(Aborted, ev), None);
        }
        assert!(is_terminal(Completed) && is_terminal(Aborted));
        assert!(!is_terminal(Paused));
    }

    // covers: BR-TPL-STATE-001
    // 无悬挂：run 状态持久化可恢复；终态 run 不再返回（不悬挂/不重复执行）。
    #[test]
    fn run_persistence_recovers_non_terminal_only() {
        let conn = fresh();
        save_run(&conn, "r1", "ws1", "wf1", 2, "Paused").unwrap();
        save_run(&conn, "r2", "ws1", "wf2", 0, "AwaitingConfirm").unwrap();
        save_run(&conn, "r3", "ws1", "wf3", 5, "Completed").unwrap();
        save_run(&conn, "r4", "ws2", "wf4", 1, "Paused").unwrap(); // 其它 workspace

        let runs = recover_runs(&conn, "ws1").unwrap();
        let ids: Vec<&str> = runs.iter().map(|(id, _, _)| id.as_str()).collect();
        assert!(ids.contains(&"r1") && ids.contains(&"r2"));
        assert!(!ids.contains(&"r3"), "Completed 不返回（不悬挂）");
        assert!(!ids.contains(&"r4"), "跨 workspace 不返回");
        // 更新中断点（恢复后续行）
        save_run(&conn, "r1", "ws1", "wf1", 3, "StepRunning").unwrap();
        let r1 = recover_runs(&conn, "ws1")
            .unwrap()
            .into_iter()
            .find(|(id, _, _)| id == "r1")
            .unwrap();
        assert_eq!(r1.1, 3); // current_step 已更新
    }
}
