//! PI-CBT-01 · 构建回合机 SM-CBT-01 与任务编排 · CBT-BUILD / BR-CBT-STATE-001 / BR-CBT-PERSIST-001
//!
//! 承接 A-CBT-M-S-01 SM-CBT-01：意图分解→计划→**计划确认**（PlanReview 须 confirm/adjust 方执行）→逐
//! 任务执行（嵌套 SM-AG-01/02）→裁决闭合；taskFail→TaskPaused **不静默跳步**（跳步须用户显式 resume）；
//! 任意非终态可 interrupt→Interrupted（**无半成品**，产物均在 DE diff 路径未裁决即不落）。控制态持久化于
//! build_runs，**重启无悬挂**（恢复非终态 run，BR-CBT-PERSIST-001）。步骤实际委托 AG（X-INTER），本机
//! 承接编排转移与 run 持久化；产物 diff 化归 PI-CBT-02、接管归 PI-CBT-03。

use rusqlite::Connection;
use std::time::{SystemTime, UNIX_EPOCH};

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// SM-CBT-01 状态（A-CBT-M-S-01 §2）。Completed/Interrupted 为终态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildState {
    Idle,
    Planning,
    PlanReview,
    TaskRunning,
    TaskPaused,
    Adjudicating,
    Completed,
    Interrupted,
}

/// SM-CBT-01 事件。takeoverIn 经 SM-AG-01 移交（PI-CBT-03）；taskOk 携带是否有后续任务。
/// fallbackOut 是**输出信号**（A-CBT-M-S-01 §3 X-INTER-001）：PlanFail→Idle / Interrupt→Interrupted
/// 路径发生后由编排层主动发出，驱动 AG 侧 SM-AG-01 的 FallbackIn——CBT 自身不因 fallbackOut 转移
/// （`transition` 对其返回 None），故不出现在转移表中。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildEvent {
    StartBuild,
    TakeoverIn,
    PlanReady,
    PlanFail,
    PlanConfirmed,
    PlanAdjusted,
    TaskOk, // 有后续任务
    AllTasksDone,
    TaskFail,
    Resume,
    AdjudicationDone,
    Interrupt,
    FallbackOut, // 输出信号：回落 chat（X-INTER-001），不驱动 CBT 自身转移
}

/// 终态判定。
pub fn is_terminal(state: BuildState) -> bool {
    matches!(state, BuildState::Completed | BuildState::Interrupted)
}

/**
 * @GOV
 * codes:    BR-CBT-STATE-001-RB-CBT-CBT-BUILD-001
 * type:     RB
 * chain:    CBT-BUILD
 * rules:    BR-CBT-STATE-001
 * boundary: in=BuildEvent | out=BuildState(planConfirmGatedNoSilentSkipInterruptNoHalfProduct)
 */
/// SM-CBT-01 转移引擎（BR-CBT-STATE-001）：PlanReview 须 planConfirmed/planAdjusted 方进 TaskRunning
/// （计划确认强制）；taskFail 只到 TaskPaused（**不静默跳步**，续行须显式 resume）；allTasksDone 经
/// Adjudicating 方 Completed（不绕过裁决）；任意非终态 interrupt→Interrupted（无半成品）。返回 `None`
/// = 无合法转移（含全部 forbidden）。
pub fn transition(state: BuildState, event: BuildEvent) -> Option<BuildState> {
    use BuildEvent::*;
    use BuildState::*;
    // 任意非终态用户中断 → Interrupted（无半成品）。
    if matches!(event, Interrupt) && !is_terminal(state) {
        return Some(Interrupted);
    }
    let next = match (state, event) {
        // 启动 / 接管进入（takeoverIn 守卫见 PI-CBT-03）。
        (Idle, StartBuild) | (Idle, TakeoverIn) => Planning,
        // 计划：就绪→评审；意图不明→回落 Idle（fallback 澄清）。
        (Planning, PlanReady) => PlanReview,
        (Planning, PlanFail) => Idle,
        // 计划确认强制：确认或调整后确认方执行。
        (PlanReview, PlanConfirmed) | (PlanReview, PlanAdjusted) => TaskRunning,
        // 任务执行：有后续续行；全完成→裁决；失败→暂停（不静默跳步）。
        (TaskRunning, TaskOk) => TaskRunning,
        (TaskRunning, AllTasksDone) => Adjudicating,
        (TaskRunning, TaskFail) => TaskPaused,
        (TaskPaused, Resume) => TaskRunning,
        // 裁决完毕 → 闭合。
        (Adjudicating, AdjudicationDone) => Completed,
        // 其余：无合法转移（终态冻结；不绕过计划确认；taskFail 不自动跳步；不绕过裁决直达 Completed）。
        _ => return None,
    };
    Some(next)
}

/**
 * @GOV
 * codes:    BR-CBT-PERSIST-001-EFFECT-CBT-CBT-BUILD-001
 * type:     EFFECT
 * chain:    CBT-BUILD
 * rules:    BR-CBT-PERSIST-001
 * boundary: in=buildRunState | out=persistedRun(restartRecoverableNoDangling)
 */
/// 持久化构建 run 控制态（BR-CBT-PERSIST-001「无悬挂」）：写/更新 build_runs（status + plan_ref +
/// interrupted_at），重启后可由 recover_runs 恢复到中断点。
pub fn save_run(
    conn: &Connection,
    run_id: &str,
    workspace_id: &str,
    status: &str,
    plan_ref: &str,
    interrupted_at: Option<i64>,
) -> rusqlite::Result<()> {
    let now = now_millis();
    conn.execute(
        "INSERT INTO build_runs \
         (run_id, workspace_id, status, plan_ref, interrupted_at, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6) \
         ON CONFLICT(run_id) DO UPDATE SET status=?3, plan_ref=?4, interrupted_at=?5, updated_at=?6",
        (run_id, workspace_id, status, plan_ref, interrupted_at, now),
    )?;
    Ok(())
}

/**
 * @GOV
 * codes:    BR-CBT-PERSIST-001-QUERY-CBT-CBT-BUILD-001
 * type:     QUERY
 * chain:    CBT-BUILD
 * rules:    BR-CBT-PERSIST-001
 * boundary: in=workspaceId | out=resumableBuildRuns(nonTerminalOnlyNoDangling)
 */
/// 恢复非终态构建 run（BR-CBT-PERSIST-001「无悬挂」）：返回当前 workspace 非终态（Completed/Interrupted
/// 以外）的 run，供重启后恢复；终态不返回（不悬挂/不重复执行）。
pub fn recover_runs(
    conn: &Connection,
    workspace_id: &str,
) -> rusqlite::Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT run_id, status FROM build_runs \
         WHERE workspace_id=?1 AND status NOT IN ('Completed','Interrupted') ORDER BY updated_at ASC",
    )?;
    let rows = stmt.query_map([workspace_id], |r| Ok((r.get(0)?, r.get(1)?)))?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::BuildEvent::*;
    use super::BuildState::*;
    use super::*;
    use crate::db;

    fn fresh() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        db::migrate(&mut conn).unwrap();
        conn
    }

    // covers: BR-CBT-STATE-001
    // 正路：启动→计划→评审→（确认）→执行→裁决→Completed；计划确认强制（不得绕过）。
    #[test]
    fn lifecycle_requires_plan_confirm_and_adjudication() {
        assert_eq!(transition(Idle, StartBuild), Some(Planning));
        assert_eq!(transition(Planning, PlanReady), Some(PlanReview));
        // 不得绕过计划确认直接执行
        assert_eq!(transition(Planning, PlanConfirmed), None);
        assert_eq!(transition(PlanReview, PlanConfirmed), Some(TaskRunning));
        assert_eq!(transition(PlanReview, PlanAdjusted), Some(TaskRunning));
        assert_eq!(transition(TaskRunning, TaskOk), Some(TaskRunning));
        assert_eq!(transition(TaskRunning, AllTasksDone), Some(Adjudicating));
        // 不得绕过裁决直达 Completed
        assert_eq!(transition(TaskRunning, AdjudicationDone), None);
        assert_eq!(transition(Adjudicating, AdjudicationDone), Some(Completed));
        // 意图不明回落
        assert_eq!(transition(Planning, PlanFail), Some(Idle));
    }

    // covers: BR-CBT-STATE-001
    // 失败不静默跳步；任意非终态可中断→Interrupted（无半成品）；终态冻结。
    #[test]
    fn failure_pauses_interrupt_anywhere_terminals_frozen() {
        // 失败 → 暂停（不自动前进），续行须显式 resume
        assert_eq!(transition(TaskRunning, TaskFail), Some(TaskPaused));
        assert_eq!(transition(TaskPaused, Resume), Some(TaskRunning));
        // 任意非终态中断 → Interrupted
        for s in [Idle, Planning, PlanReview, TaskRunning, TaskPaused, Adjudicating] {
            assert_eq!(transition(s, Interrupt), Some(Interrupted), "{s:?} 应可中断");
        }
        // 终态冻结（含 interrupt 不再生效）
        for ev in [StartBuild, PlanConfirmed, AdjudicationDone, Resume, Interrupt] {
            assert_eq!(transition(Completed, ev), None);
            assert_eq!(transition(Interrupted, ev), None);
        }
        assert!(is_terminal(Completed) && is_terminal(Interrupted) && !is_terminal(TaskRunning));
    }

    // covers: BR-CBT-STATE-001
    // fallbackOut 是输出信号（X-INTER-001）：CBT 自身不因其转移——任何状态对 FallbackOut 均返回 None。
    // 编排层在 PlanFail→Idle / Interrupt→Interrupted 路径后发出该信号驱动 AG 侧 FallbackIn（本机不消费）。
    #[test]
    fn fallback_out_is_output_signal_never_transitions() {
        // 回落语义所在的两条路径本身仍按既有转移工作（PlanFail→Idle，Interrupt→Interrupted）。
        assert_eq!(transition(Planning, PlanFail), Some(Idle));
        assert_eq!(transition(TaskRunning, Interrupt), Some(Interrupted));
        // FallbackOut 对任意状态都不驱动 CBT 转移（输出信号，非输入事件）。
        for s in [Idle, Planning, PlanReview, TaskRunning, TaskPaused, Adjudicating, Completed, Interrupted] {
            assert_eq!(transition(s, FallbackOut), None, "{s:?} 不应因 FallbackOut 转移");
        }
    }

    // covers: BR-CBT-PERSIST-001
    // 无悬挂：build run 持久化可恢复；终态/跨 workspace 不返回。
    #[test]
    fn run_persistence_recovers_non_terminal_only() {
        let conn = fresh();
        save_run(&conn, "b1", "ws1", "TaskRunning", "plan_1", None).unwrap();
        save_run(&conn, "b2", "ws1", "TaskPaused", "plan_2", None).unwrap();
        save_run(&conn, "b3", "ws1", "Completed", "plan_3", None).unwrap();
        save_run(&conn, "b4", "ws1", "Interrupted", "plan_4", Some(now_millis())).unwrap();
        save_run(&conn, "b5", "ws2", "TaskRunning", "plan_5", None).unwrap();

        let runs = recover_runs(&conn, "ws1").unwrap();
        let ids: Vec<&str> = runs.iter().map(|(id, _)| id.as_str()).collect();
        assert!(ids.contains(&"b1") && ids.contains(&"b2"));
        assert!(!ids.contains(&"b3") && !ids.contains(&"b4"), "终态不悬挂");
        assert!(!ids.contains(&"b5"), "跨 workspace 不返回");
        // 中断点更新可恢复
        save_run(&conn, "b1", "ws1", "Interrupted", "plan_1", Some(now_millis())).unwrap();
        assert!(!recover_runs(&conn, "ws1").unwrap().iter().any(|(id, _)| id == "b1"));
    }
}
