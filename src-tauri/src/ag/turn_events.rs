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

fn terminal_transition(
    source: ag::turn::TurnState,
    event: ag::turn::TurnEvent,
    expected: ag::turn::TurnState,
) -> Option<ag::turn::TurnState> {
    let next = ag::turn::transition(source, event)?;
    (next == expected && ag::turn::is_terminal(next)).then_some(next)
}

pub(super) fn emit_ag_turn_done(
    channel: &tauri::ipc::Channel<AgTurnEvent>,
    latch: &ag::turn::TerminalLatch,
    app: &tauri::AppHandle,
    turn_id: &str,
    stop_reason: &str,
) -> bool {
    use ag::turn::{transition, TurnEvent, TurnState};
    let Some(handing_off) = transition(TurnState::Streaming, TurnEvent::StreamEnd) else {
        return false;
    };
    let Some(completed) = transition(handing_off, TurnEvent::HandoffOk) else {
        return false;
    };
    if !latch.try_set(ag::turn::TerminalKind::Completed) {
        return false;
    }
    let _ = channel.send(AgTurnEvent::Done {
        turn_id: turn_id.to_string(),
        stop_reason: stop_reason.to_string(),
    });
    eprintln!("[ag-trace] turn_id={turn_id} terminal=completed stop_reason={stop_reason}");
    emit_ag01(app, turn_id, handing_off);
    emit_ag01(app, turn_id, completed);
    true
}

pub(super) fn emit_ag_turn_cancelled(
    channel: &tauri::ipc::Channel<AgTurnEvent>,
    latch: &ag::turn::TerminalLatch,
    app: &tauri::AppHandle,
    turn_id: &str,
) -> bool {
    emit_ag_turn_cancelled_from(channel, latch, app, turn_id, ag::turn::TurnState::Streaming)
}

pub(super) fn emit_ag_turn_cancelled_from(
    channel: &tauri::ipc::Channel<AgTurnEvent>,
    latch: &ag::turn::TerminalLatch,
    app: &tauri::AppHandle,
    turn_id: &str,
    source: ag::turn::TurnState,
) -> bool {
    let Some(cancelled) = terminal_transition(
        source,
        ag::turn::TurnEvent::Cancel,
        ag::turn::TurnState::Cancelled,
    ) else {
        return false;
    };
    if !latch.try_set(ag::turn::TerminalKind::Cancelled) {
        return false;
    }
    let _ = channel.send(AgTurnEvent::Done {
        turn_id: turn_id.to_string(),
        stop_reason: "cancelled".to_string(),
    });
    eprintln!("[ag-trace] turn_id={turn_id} terminal=cancelled");
    emit_ag01(app, turn_id, cancelled);
    true
}

pub(super) fn emit_ag_turn_denied(
    channel: &tauri::ipc::Channel<AgTurnEvent>,
    latch: &ag::turn::TerminalLatch,
    app: &tauri::AppHandle,
    turn_id: &str,
    error: &IpcError,
) -> bool {
    let Some(denied) = terminal_transition(
        ag::turn::TurnState::AwaitingGate,
        ag::turn::TurnEvent::GateDenied,
        ag::turn::TurnState::Denied,
    ) else {
        return false;
    };
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
    emit_ag01(app, turn_id, denied);
    true
}

pub(super) fn emit_ag_turn_error(
    channel: &tauri::ipc::Channel<AgTurnEvent>,
    latch: &ag::turn::TerminalLatch,
    app: &tauri::AppHandle,
    turn_id: &str,
    error: &IpcError,
) -> bool {
    emit_ag_turn_error_from(
        channel,
        latch,
        app,
        turn_id,
        error,
        ag::turn::TurnState::Streaming,
    )
}

pub(super) fn emit_ag_turn_error_from(
    channel: &tauri::ipc::Channel<AgTurnEvent>,
    latch: &ag::turn::TerminalLatch,
    app: &tauri::AppHandle,
    turn_id: &str,
    error: &IpcError,
    source: ag::turn::TurnState,
) -> bool {
    let event = match source {
        ag::turn::TurnState::Assembling => ag::turn::TurnEvent::AssembleFail,
        ag::turn::TurnState::Streaming => ag::turn::TurnEvent::StreamFail,
        ag::turn::TurnState::HandingOff => ag::turn::TurnEvent::HandoffFail,
        _ => return false,
    };
    let Some(turn_error) = terminal_transition(source, event, ag::turn::TurnState::TurnError)
    else {
        return false;
    };
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
    emit_ag01(app, turn_id, turn_error);
    true
}

/// AgentTurn 的统一失败出口：存在 TaskRun 时先尽力收束持久态，再发送唯一回合终态。
/// 调用方不得在 TaskRun 创建后直接用 `?` 绕过此出口；简单回合没有 TaskRun 时只收束回合。
pub(super) fn settle_and_emit_ag_turn_error(
    state: &AppState,
    task_run_id: Option<&str>,
    channel: &tauri::ipc::Channel<AgTurnEvent>,
    latch: &ag::turn::TerminalLatch,
    app: &tauri::AppHandle,
    turn_id: &str,
    error: &IpcError,
) {
    settle_and_emit_ag_turn_error_from(
        state,
        task_run_id,
        channel,
        latch,
        app,
        turn_id,
        error,
        ag::turn::TurnState::Streaming,
    );
}

pub(super) fn settle_and_emit_ag_turn_error_from(
    state: &AppState,
    task_run_id: Option<&str>,
    channel: &tauri::ipc::Channel<AgTurnEvent>,
    latch: &ag::turn::TerminalLatch,
    app: &tauri::AppHandle,
    turn_id: &str,
    error: &IpcError,
    source: ag::turn::TurnState,
) {
    if let (Some(task_run_id), Ok(conn)) = (task_run_id, state.db.lock()) {
        settle_task_run(&conn, task_run_id, turn_id, ag::task::TurnOutcome::Failed);
    }
    emit_ag_turn_error_from(channel, latch, app, turn_id, error, source);
}
