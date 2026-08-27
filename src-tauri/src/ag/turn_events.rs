//! AgentTurn 与 ToolLoop 状态事件及唯一终态输出。

use super::task_runtime::settle_task_run;
use crate::ipc::{AgTurnEvent, IpcError};
use crate::{ag, sm_events, AppState};
pub(super) fn sm_error() -> IpcError {
    IpcError::new("AG_SM_INVALID", "SM-AG-01 非法转移", false, false)
}

/// 广播 SM-AG-01 回合机转移（turn_id 为实例，A-CORE-X-L-04 Task 2-E）。
pub(super) fn emit_ag01(app: &tauri::AppHandle, turn_id: &str, st: ag::turn::TurnState) {
    sm_events::emit_sm_state(
        app,
        sm_events::SmStatePayload::new(
            "SM-AG-01",
            turn_id,
            &format!("{st:?}"),
            ag::turn::is_terminal(st),
        ),
    );
}

/// 广播 SM-AG-02 工具循环机转移（turn_id 为实例，A-CORE-X-L-04 Task 2-AG2）。
pub(super) fn emit_ag02(app: &tauri::AppHandle, turn_id: &str, st: ag::tool::ToolLoopState) {
    sm_events::emit_sm_state(
        app,
        sm_events::SmStatePayload::new(
            "SM-AG-02",
            turn_id,
            &format!("{st:?}"),
            ag::tool::is_terminal(st),
        ),
    );
}

/// 广播 SM-AG-03 局部修改/辅助续写机转移（assist_id 为实例，A-CORE-X-L-04 Task 2-AG3）。

pub(super) fn emit_ag01_complete(app: &tauri::AppHandle, turn_id: &str) {
    use ag::turn::{transition, TurnEvent, TurnState};
    let st = transition(TurnState::Streaming, TurnEvent::StreamEnd)
        .and_then(|s| transition(s, TurnEvent::HandoffOk))
        .unwrap_or(TurnState::Completed);
    emit_ag01(app, turn_id, st);
}

pub(super) fn emit_ag_turn_done(
    channel: &tauri::ipc::Channel<AgTurnEvent>,
    latch: &ag::turn::TerminalLatch,
    app: &tauri::AppHandle,
    turn_id: &str,
    stop_reason: &str,
) -> bool {
    if !latch.try_set(ag::turn::TerminalKind::Completed) {
        return false;
    }
    let _ = channel.send(AgTurnEvent::Done {
        turn_id: turn_id.to_string(),
        stop_reason: stop_reason.to_string(),
    });
    eprintln!("[ag-trace] turn_id={turn_id} terminal=completed stop_reason={stop_reason}");
    emit_ag01_complete(app, turn_id);
    true
}

pub(super) fn emit_ag_turn_cancelled(
    channel: &tauri::ipc::Channel<AgTurnEvent>,
    latch: &ag::turn::TerminalLatch,
    app: &tauri::AppHandle,
    turn_id: &str,
) -> bool {
    if !latch.try_set(ag::turn::TerminalKind::Cancelled) {
        return false;
    }
    let _ = channel.send(AgTurnEvent::Done {
        turn_id: turn_id.to_string(),
        stop_reason: "cancelled".to_string(),
    });
    eprintln!("[ag-trace] turn_id={turn_id} terminal=cancelled");
    emit_ag01(app, turn_id, ag::turn::TurnState::Cancelled);
    true
}

pub(super) fn emit_ag_turn_denied(
    channel: &tauri::ipc::Channel<AgTurnEvent>,
    latch: &ag::turn::TerminalLatch,
    app: &tauri::AppHandle,
    turn_id: &str,
    error: &IpcError,
) -> bool {
    if !latch.try_set(ag::turn::TerminalKind::Denied) {
        return false;
    }
    let _ = channel.send(AgTurnEvent::Error {
        turn_id: turn_id.to_string(),
        code: error.code.clone(),
        message: error.message.clone(),
        retriable: error.retriable,
    });
    eprintln!(
        "[ag-trace] turn_id={} terminal=denied code={} retriable={}",
        turn_id, error.code, error.retriable
    );
    emit_ag01(app, turn_id, ag::turn::TurnState::Denied);
    true
}

pub(super) fn emit_ag_turn_error(
    channel: &tauri::ipc::Channel<AgTurnEvent>,
    latch: &ag::turn::TerminalLatch,
    app: &tauri::AppHandle,
    turn_id: &str,
    error: &IpcError,
) -> bool {
    if !latch.try_set(ag::turn::TerminalKind::Error) {
        return false;
    }
    let _ = channel.send(AgTurnEvent::Error {
        turn_id: turn_id.to_string(),
        code: error.code.clone(),
        message: error.message.clone(),
        retriable: error.retriable,
    });
    eprintln!(
        "[ag-trace] turn_id={} terminal=error code={} retriable={}",
        turn_id, error.code, error.retriable
    );
    emit_ag01(app, turn_id, ag::turn::TurnState::TurnError);
    true
}

/// TaskRun 已进入活动态后的统一失败出口：先尽力收束持久态，再发送唯一回合终态。
/// 调用方不得在 TaskRun 创建后直接用 `?` 绕过此出口。
pub(super) fn settle_and_emit_ag_turn_error(
    state: &AppState,
    task_run_id: &str,
    channel: &tauri::ipc::Channel<AgTurnEvent>,
    latch: &ag::turn::TerminalLatch,
    app: &tauri::AppHandle,
    turn_id: &str,
    error: &IpcError,
) {
    if let Ok(conn) = state.db.lock() {
        settle_task_run(&conn, task_run_id, turn_id, ag::task::TurnOutcome::Failed);
    }
    emit_ag_turn_error(channel, latch, app, turn_id, error);
}
