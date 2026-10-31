use super::task_runtime::settle_optional_task_run;
use super::turn_events::{emit_cancelled_terminal, TurnEventEmitter};
use crate::ag;
use crate::ipc::IpcError;
use crate::AppState;

/**
 * @GOV
 * codes:    BR-AG-STATE-001-EFFECT-AG-AG-TURN-086, BR-SYS-STATE-007-EFFECT-AG-AG-TURN-086
 * type:     EFFECT
 * chain:    AG-TURN
 * rules:    BR-AG-STATE-001, BR-SYS-STATE-007
 * boundary: in=CancelledTurnSourceAndOptionalTaskRun | out=SettledTaskRunAndSingleCancelledTerminal | consumer=BR-AG-STATE-001,BR-SYS-STATE-007 | does_not=emitSecondTerminalOrMarkCompletedOrBypassTerminalLatch
 */
/// 取消的唯一结算入口：先收束可选 TaskRun，再走 SM-AG-01 Cancelled 终态发射。
/// 调用方负责提供本次回合真实的 emitter/latch；停放态使用无 Channel emitter 与默认 latch。
pub(super) fn settle_cancelled_turn<R: tauri::Runtime>(
    state: &AppState,
    app: &tauri::AppHandle<R>,
    emitter: &TurnEventEmitter<'_>,
    latch: &ag::turn::TerminalLatch,
    turn_id: &str,
    task_run_id: Option<&str>,
    source: ag::turn::TurnState,
) -> Result<Option<bool>, IpcError> {
    if let Ok(conn) = state.db.lock() {
        settle_optional_task_run(
            &conn,
            task_run_id,
            turn_id,
            ag::task::TurnOutcome::Cancelled,
        );
    }
    emit_cancelled_terminal(emitter, latch, app, turn_id, source)
}
