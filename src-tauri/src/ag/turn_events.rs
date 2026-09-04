//! AgentTurn append-only TurnEvent、Channel 安全投影与唯一终态输出。

use super::task_runtime::settle_task_run;
use crate::ipc::{AgTurnEvent, AgTurnEventPayload, IpcError};
use crate::{ag, sm_events, AppState};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

const TURN_EVENT_PAYLOAD_VERSION: u32 = 1;

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

fn is_terminal_status(status: &str) -> bool {
    matches!(
        status,
        "completed" | "error" | "cancelled" | "gate_rejected"
    )
}

fn event_db_error(error: impl std::fmt::Display) -> IpcError {
    IpcError::new(
        "AG_TURN_EVENT_DB_ERROR",
        &format!("回合事件持久化失败：{error}"),
        true,
        false,
    )
}

#[derive(Debug)]
struct AppendedEvent {
    event_id: String,
    seq: u64,
    occurred_at: i64,
}

fn append_event_record(
    conn: &mut Connection,
    turn_id: &str,
    kind: &str,
    payload_json: &str,
    terminal_status: Option<&str>,
) -> rusqlite::Result<Option<AppendedEvent>> {
    let tx = conn.transaction()?;
    let (workspace_id, session_id, current_status) = tx
        .query_row(
            "SELECT workspace_id, session_id, status FROM agent_turns WHERE turn_id = ?1",
            [turn_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()?
        .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
    if is_terminal_status(&current_status) {
        return Ok(None);
    }
    let next_seq = tx.query_row(
        "SELECT COALESCE(MAX(seq), 0) + 1 FROM agent_turn_events WHERE turn_id = ?1",
        [turn_id],
        |row| row.get::<_, u64>(0),
    )?;
    let occurred_at = now_millis();
    let event_id = format!("turn-event:{turn_id}:{next_seq}");
    tx.execute(
        "INSERT INTO agent_turn_events \
         (event_id, workspace_id, session_id, turn_id, seq, kind, payload_version, payload_json, occurred_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            event_id,
            workspace_id,
            session_id,
            turn_id,
            next_seq,
            kind,
            TURN_EVENT_PAYLOAD_VERSION,
            payload_json,
            occurred_at
        ],
    )?;
    if let Some(status) = terminal_status {
        tx.execute(
            "UPDATE agent_turns SET status = ?4, updated_at = ?5 \
             WHERE turn_id = ?1 AND workspace_id = ?2 AND session_id = ?3",
            params![turn_id, workspace_id, session_id, status, occurred_at],
        )?;
    }
    tx.commit()?;
    Ok(Some(AppendedEvent {
        event_id,
        seq: next_seq,
        occurred_at,
    }))
}

/**
 * @GOV
 * codes:    BR-AG-STATE-001-EFFECT-AG-AG-TURN-044, BR-AG-STATE-002-EFFECT-AG-AG-TURN-044, BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-044, BR-SYS-STATE-007-EFFECT-AG-AG-TURN-044
 * type:     EFFECT
 * chain:    AG-TURN, AG-TOOL
 * rules:    BR-AG-STATE-001, BR-AG-STATE-002, BR-AG-PERSIST-001, BR-SYS-STATE-007
 * term_ref: TERM-AG-001, TERM-AG-002, TERM-AG-022
 * boundary: in=CurrentAgentTurnAndSafeOrInternalTurnEvent | out=AppendOnlyMonotonicTurnEventAndSameIdentityChannelOrAfterSeqProjection | effect=workspaceDb,IPC-ag_turn_stream | does_not=overwriteCommittedEventExposeInternalPayloadOrAppendAfterFirstTerminal
 */
pub(super) struct TurnEventEmitter<'a> {
    state: &'a AppState,
    channel: Option<&'a tauri::ipc::Channel<AgTurnEvent>>,
    turn_id: &'a str,
}

impl<'a> TurnEventEmitter<'a> {
    pub(super) fn new(
        state: &'a AppState,
        channel: &'a tauri::ipc::Channel<AgTurnEvent>,
        turn_id: &'a str,
    ) -> Self {
        Self {
            state,
            channel: Some(channel),
            turn_id,
        }
    }

    /// AwaitingClarification 的原 Channel 已关闭；取消仍写同一事件事实，随后由 afterSeq 补流。
    pub(super) fn without_channel(state: &'a AppState, turn_id: &'a str) -> Self {
        Self {
            state,
            channel: None,
            turn_id,
        }
    }

    /// 追加后端内部事件。payload 只允许安全元数据，不会投影到 Channel。
    pub(super) fn record_internal(
        &self,
        kind: &str,
        payload: Value,
    ) -> Result<Option<u64>, IpcError> {
        let payload_json = serde_json::to_string(&payload).map_err(event_db_error)?;
        let mut conn = self.state.db.lock().map_err(|_| {
            IpcError::new(
                "WS_DB_BUSY",
                "数据库连接繁忙，无法记录回合事件",
                true,
                false,
            )
        })?;
        append_event_record(&mut conn, self.turn_id, kind, &payload_json, None)
            .map(|appended| appended.map(|event| event.seq))
            .map_err(event_db_error)
    }

    /// 安全事件必须先提交持久化，再使用完全相同的身份与 seq 投影到 Channel。
    pub(super) fn emit(
        &self,
        latch: &ag::turn::TerminalLatch,
        payload: AgTurnEventPayload,
    ) -> Result<bool, IpcError> {
        if latch.is_set() || payload.terminal_status().is_some() {
            return Ok(false);
        }
        let payload_json = serde_json::to_string(&payload).map_err(event_db_error)?;
        let appended = {
            let mut conn = self.state.db.lock().map_err(|_| {
                IpcError::new(
                    "WS_DB_BUSY",
                    "数据库连接繁忙，无法记录回合事件",
                    true,
                    false,
                )
            })?;
            append_event_record(&mut conn, self.turn_id, payload.kind(), &payload_json, None)
                .map_err(event_db_error)?
        };
        let Some(appended) = appended else {
            return Ok(false);
        };
        let Some(channel) = self.channel else {
            return Err(IpcError::new(
                "AG_TURN_CHANNEL_REQUIRED",
                "非终态回合事件缺少 Channel 投影目标",
                false,
                false,
            ));
        };
        if let Err(error) = channel.send(AgTurnEvent {
            event_id: appended.event_id,
            turn_id: self.turn_id.to_string(),
            seq: appended.seq,
            occurred_at: appended.occurred_at,
            payload_version: TURN_EVENT_PAYLOAD_VERSION,
            event: payload,
        }) {
            // Channel 不是事实源。断线后继续产生真实终态，客户端按 afterSeq 补流。
            eprintln!(
                "[ag-trace] turn_id={} seq={} channel_projection_failed={error}",
                self.turn_id, appended.seq
            );
        }
        Ok(true)
    }

    fn emit_terminal(
        &self,
        latch: &ag::turn::TerminalLatch,
        terminal_kind: ag::turn::TerminalKind,
        payload: AgTurnEventPayload,
    ) -> Result<bool, IpcError> {
        if latch.is_set() {
            return Ok(false);
        }
        let Some(status) = payload.terminal_status() else {
            return Ok(false);
        };
        let payload_json = serde_json::to_string(&payload).map_err(event_db_error)?;
        let appended = {
            let mut conn = self.state.db.lock().map_err(|_| {
                IpcError::new(
                    "WS_DB_BUSY",
                    "数据库连接繁忙，无法记录回合终态",
                    true,
                    false,
                )
            })?;
            append_event_record(
                &mut conn,
                self.turn_id,
                payload.kind(),
                &payload_json,
                Some(status),
            )
            .map_err(event_db_error)?
        };
        let Some(appended) = appended else {
            return Ok(false);
        };
        if !latch.try_set(terminal_kind) {
            return Ok(false);
        }
        // Channel 可能已断开；事件与 status 已原子提交，可由 afterSeq 补流，因此不回滚事实。
        if let Some(channel) = self.channel {
            let _ = channel.send(AgTurnEvent {
                event_id: appended.event_id,
                turn_id: self.turn_id.to_string(),
                seq: appended.seq,
                occurred_at: appended.occurred_at,
                payload_version: TURN_EVENT_PAYLOAD_VERSION,
                event: payload,
            });
        }
        Ok(true)
    }
}

/**
 * @GOV
 * codes:    BR-AG-PERSIST-001-QUERY-AG-AG-TURN-045, BR-SYS-STATE-007-QUERY-AG-AG-TURN-045
 * type:     QUERY
 * chain:    AG-TURN
 * rules:    BR-AG-PERSIST-001, BR-SYS-STATE-007
 * term_ref: TERM-AG-001, TERM-AG-022
 * boundary: in=WorkspaceScopedAgentTurnIdentityAndAfterSeq | out=AscendingSafeAgTurnEventsWithOriginalSeqGaps | does_not=exposeInternalContextGateProviderEventsOrCrossWorkspaceTurn
 */
pub fn list_safe_events(
    conn: &Connection,
    workspace_id: &str,
    turn_id: &str,
    after_seq: u64,
) -> rusqlite::Result<Vec<AgTurnEvent>> {
    let owns_turn = conn
        .query_row(
            "SELECT 1 FROM agent_turns WHERE workspace_id = ?1 AND turn_id = ?2",
            params![workspace_id, turn_id],
            |_| Ok(()),
        )
        .optional()?;
    if owns_turn.is_none() {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    let mut statement = conn.prepare(
        "SELECT event_id, turn_id, seq, occurred_at, payload_version, payload_json \
         FROM agent_turn_events \
         WHERE workspace_id = ?1 AND turn_id = ?2 AND seq > ?3 AND kind IN ( \
           'chunk','intent_updated','route_updated','tool_request','tool_result','diff_proposed', \
           'step_progress','decision_request_proposed','decision_request_resolved', \
           'clarification_required','done','error' \
         ) ORDER BY seq ASC",
    )?;
    let rows = statement.query_map(params![workspace_id, turn_id, after_seq], |row| {
        let payload_json: String = row.get(5)?;
        let event = serde_json::from_str::<AgTurnEventPayload>(&payload_json).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                5,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?;
        Ok(AgTurnEvent {
            event_id: row.get(0)?,
            turn_id: row.get(1)?,
            seq: row.get(2)?,
            occurred_at: row.get(3)?,
            payload_version: row.get(4)?,
            event,
        })
    })?;
    rows.collect()
}

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
    emitter: &TurnEventEmitter<'_>,
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
    let emitted = emitter
        .emit_terminal(
            latch,
            ag::turn::TerminalKind::Completed,
            AgTurnEventPayload::Done {
                stop_reason: stop_reason.to_string(),
            },
        )
        .unwrap_or_else(|error| {
            eprintln!(
                "[ag-trace] turn_id={turn_id} terminal_persist_failed={}",
                error.message
            );
            false
        });
    if !emitted {
        return false;
    }
    eprintln!("[ag-trace] turn_id={turn_id} terminal=completed stop_reason={stop_reason}");
    emit_ag01(app, turn_id, handing_off);
    emit_ag01(app, turn_id, completed);
    true
}

pub(super) fn emit_ag_turn_cancelled(
    emitter: &TurnEventEmitter<'_>,
    latch: &ag::turn::TerminalLatch,
    app: &tauri::AppHandle,
    turn_id: &str,
) -> bool {
    emit_ag_turn_cancelled_from(emitter, latch, app, turn_id, ag::turn::TurnState::Streaming)
}

pub(super) fn emit_ag_turn_cancelled_from(
    emitter: &TurnEventEmitter<'_>,
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
    let emitted = emitter
        .emit_terminal(
            latch,
            ag::turn::TerminalKind::Cancelled,
            AgTurnEventPayload::Done {
                stop_reason: "cancelled".to_string(),
            },
        )
        .unwrap_or_else(|error| {
            eprintln!(
                "[ag-trace] turn_id={turn_id} terminal_persist_failed={}",
                error.message
            );
            false
        });
    if !emitted {
        return false;
    }
    eprintln!("[ag-trace] turn_id={turn_id} terminal=cancelled");
    emit_ag01(app, turn_id, cancelled);
    true
}

/// AwaitingClarification 已无活动 Channel；取消在数据库中原子锁存，再由安全补流返回同一事件。
pub(super) fn persist_parked_ag_turn_cancelled(
    state: &AppState,
    app: &tauri::AppHandle,
    turn_id: &str,
) -> Result<bool, IpcError> {
    let Some(cancelled) = terminal_transition(
        ag::turn::TurnState::AwaitingClarification,
        ag::turn::TurnEvent::Cancel,
        ag::turn::TurnState::Cancelled,
    ) else {
        return Err(sm_error());
    };
    let emitter = TurnEventEmitter::without_channel(state, turn_id);
    let latch = ag::turn::TerminalLatch::default();
    let emitted = emitter.emit_terminal(
        &latch,
        ag::turn::TerminalKind::Cancelled,
        AgTurnEventPayload::Done {
            stop_reason: "cancelled".to_string(),
        },
    )?;
    if emitted {
        emit_ag01(app, turn_id, cancelled);
    }
    Ok(emitted)
}

pub(super) fn emit_ag_turn_denied(
    emitter: &TurnEventEmitter<'_>,
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
    let emitted = emitter
        .emit_terminal(
            latch,
            ag::turn::TerminalKind::Denied,
            AgTurnEventPayload::Error {
                code: error.code.clone(),
                message: error.message.clone(),
                retriable: error.retriable,
            },
        )
        .unwrap_or_else(|persist_error| {
            eprintln!(
                "[ag-trace] turn_id={turn_id} terminal_persist_failed={}",
                persist_error.message
            );
            false
        });
    if !emitted {
        return false;
    }
    eprintln!(
        "[ag-trace] turn_id={} terminal=denied code={} retriable={}",
        turn_id, error.code, error.retriable
    );
    emit_ag01(app, turn_id, denied);
    true
}

pub(super) fn emit_ag_turn_error(
    emitter: &TurnEventEmitter<'_>,
    latch: &ag::turn::TerminalLatch,
    app: &tauri::AppHandle,
    turn_id: &str,
    error: &IpcError,
) -> bool {
    emit_ag_turn_error_from(
        emitter,
        latch,
        app,
        turn_id,
        error,
        ag::turn::TurnState::Streaming,
    )
}

pub(super) fn emit_ag_turn_error_from(
    emitter: &TurnEventEmitter<'_>,
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
    let emitted = emitter
        .emit_terminal(
            latch,
            ag::turn::TerminalKind::Error,
            AgTurnEventPayload::Error {
                code: error.code.clone(),
                message: error.message.clone(),
                retriable: error.retriable,
            },
        )
        .unwrap_or_else(|persist_error| {
            eprintln!(
                "[ag-trace] turn_id={turn_id} terminal_persist_failed={}",
                persist_error.message
            );
            false
        });
    if !emitted {
        return false;
    }
    eprintln!(
        "[ag-trace] turn_id={} terminal=error code={} retriable={}",
        turn_id, error.code, error.retriable
    );
    emit_ag01(app, turn_id, turn_error);
    true
}

/// AgentTurn 的统一失败出口：存在 TaskRun 时先尽力收束持久态，再发送唯一回合终态。
pub(super) fn settle_and_emit_ag_turn_error(
    state: &AppState,
    task_run_id: Option<&str>,
    emitter: &TurnEventEmitter<'_>,
    latch: &ag::turn::TerminalLatch,
    app: &tauri::AppHandle,
    turn_id: &str,
    error: &IpcError,
) {
    settle_and_emit_ag_turn_error_from(
        state,
        task_run_id,
        emitter,
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
    emitter: &TurnEventEmitter<'_>,
    latch: &ag::turn::TerminalLatch,
    app: &tauri::AppHandle,
    turn_id: &str,
    error: &IpcError,
    source: ag::turn::TurnState,
) {
    if let (Some(task_run_id), Ok(conn)) = (task_run_id, state.db.lock()) {
        settle_task_run(&conn, task_run_id, turn_id, ag::task::TurnOutcome::Failed);
    }
    emit_ag_turn_error_from(emitter, latch, app, turn_id, error, source);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_registry, db};
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    fn fresh_turn() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        db::migrate(&mut conn).unwrap();
        conn.execute(
            "INSERT INTO workspaces (workspace_id, root_path, is_active, opened_at) \
             VALUES ('ws1','/w',1,0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO agent_sessions \
             (session_id,workspace_id,title,mode,archived,open_index,is_active,created_at,updated_at) \
             VALUES ('s1','ws1','s','edit',0,0,1,0,0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO agent_turn_payloads \
             (payload_ref,workspace_id,payload_json,payload_size,created_at) \
             VALUES ('p1','ws1','{}',2,0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO agent_turns \
             (turn_id,workspace_id,payload_ref,created_at,session_id,status,updated_at) \
             VALUES ('t1','ws1','p1',0,'s1','draft',0)",
            [],
        )
        .unwrap();
        conn
    }

    // covers: BR-AG-STATE-001, BR-SYS-STATE-007, BR-AG-PERSIST-001
    #[test]
    fn parked_turn_can_persist_cancelled_terminal_without_live_channel() {
        let state = AppState {
            db: Mutex::new(fresh_turn()),
            registry: build_registry(),
            assist_terminals: Mutex::new(HashMap::new()),
            active_agent_turns: Arc::new(Mutex::new(HashMap::new())),
        };
        let latch = ag::turn::TerminalLatch::default();
        let emitter = TurnEventEmitter::without_channel(&state, "t1");
        assert!(emitter
            .emit_terminal(
                &latch,
                ag::turn::TerminalKind::Cancelled,
                AgTurnEventPayload::Done {
                    stop_reason: "cancelled".into(),
                },
            )
            .unwrap());

        let conn = state.db.lock().unwrap();
        let events = list_safe_events(&conn, "ws1", "t1", 0).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event.terminal_status(), Some("cancelled"));
        let status: String = conn
            .query_row(
                "SELECT status FROM agent_turns WHERE turn_id='t1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(status, "cancelled");
    }

    // covers: BR-AG-PERSIST-001
    // covers: BR-SYS-STATE-007
    #[test]
    fn append_only_sequence_filters_internal_events_and_freezes_after_terminal() {
        let mut conn = fresh_turn();
        let internal = append_event_record(
            &mut conn,
            "t1",
            "context_frozen",
            &serde_json::json!({"openDocCount": 1}).to_string(),
            None,
        )
        .unwrap()
        .unwrap();
        assert_eq!(internal.seq, 1);

        let chunk = AgTurnEventPayload::Chunk { text: "A".into() };
        let chunk_record = append_event_record(
            &mut conn,
            "t1",
            chunk.kind(),
            &serde_json::to_string(&chunk).unwrap(),
            None,
        )
        .unwrap()
        .unwrap();
        assert_eq!(chunk_record.seq, 2);

        let done = AgTurnEventPayload::Done {
            stop_reason: "stop".into(),
        };
        let done_record = append_event_record(
            &mut conn,
            "t1",
            done.kind(),
            &serde_json::to_string(&done).unwrap(),
            done.terminal_status(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(done_record.seq, 3);

        assert!(append_event_record(
            &mut conn,
            "t1",
            chunk.kind(),
            &serde_json::to_string(&chunk).unwrap(),
            None,
        )
        .unwrap()
        .is_none());
        let status: String = conn
            .query_row(
                "SELECT status FROM agent_turns WHERE turn_id='t1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(status, "completed");

        let safe = list_safe_events(&conn, "ws1", "t1", 0).unwrap();
        assert_eq!(
            safe.iter().map(|event| event.seq).collect::<Vec<_>>(),
            vec![2, 3]
        );
        assert!(matches!(safe[0].event, AgTurnEventPayload::Chunk { .. }));
        let after = list_safe_events(&conn, "ws1", "t1", 2).unwrap();
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].seq, 3);
    }

    // covers: BR-AG-PERSIST-001
    #[test]
    fn safe_event_query_is_workspace_scoped_and_committed_rows_cannot_update() {
        let mut conn = fresh_turn();
        let chunk = AgTurnEventPayload::Chunk { text: "A".into() };
        append_event_record(
            &mut conn,
            "t1",
            chunk.kind(),
            &serde_json::to_string(&chunk).unwrap(),
            None,
        )
        .unwrap();
        assert!(matches!(
            list_safe_events(&conn, "other", "t1", 0),
            Err(rusqlite::Error::QueryReturnedNoRows)
        ));
        let update = conn.execute(
            "UPDATE agent_turn_events SET payload_json='{}' WHERE turn_id='t1'",
            [],
        );
        assert!(update.is_err(), "已提交 TurnEvent 不得原位修改");
    }
}
