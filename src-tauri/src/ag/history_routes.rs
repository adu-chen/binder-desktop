//! Agent 会话与回合历史的 command 路由适配。

use crate::ipc::{CommandRequest, IpcError};
use crate::{active_ws, ag, lock_db, str_param, ws_json, AppState};

/**
 * @GOV
 * codes:    BR-AG-PERSIST-001-QUERY-AG-AG-TURN-020
 * type:     QUERY
 * chain:    AG-TURN
 * rules:    BR-AG-PERSIST-001
 * boundary: in=ActiveWorkspaceIdentity | out=PersistedAgentSessionSummaries
 */
pub(crate) fn route_list_agent_sessions(state: &AppState) -> Result<serde_json::Value, IpcError> {
    let ws = active_ws(state)?;
    let conn = lock_db(state)?;
    let sessions = ag::history::list_sessions(&conn, &ws.workspace_id)
        .map_err(|e| IpcError::new("AG_HISTORY_DB_ERROR", &e.to_string(), true, false))?;
    ws_json(&sessions)
}

/**
 * @GOV
 * codes:    BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-021
 * type:     EFFECT
 * chain:    AG-TURN
 * rules:    BR-AG-PERSIST-001
 * boundary: in=AgentSessionMetadataCommand | out=PersistedAgentSessionMetadata | does_not=validateBusinessPayloadOrExecuteTurnOrWriteWorkspace
 */
pub(crate) fn route_save_agent_session(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let ws = active_ws(state)?;
    let input = ag::history::AgentSessionInput {
        session_id: str_param(request, "sessionId", "AG_SESSION_ID_REQUIRED")?,
        title: request
            .params
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("新对话")
            .to_string(),
        mode: request
            .params
            .get("mode")
            .and_then(|v| v.as_str())
            .unwrap_or("edit")
            .to_string(),
        archived: request
            .params
            .get("archived")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        open_index: request
            .params
            .get("openIndex")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        is_active: request
            .params
            .get("isActive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    };
    let conn = lock_db(state)?;
    ag::history::upsert_session(&conn, &ws.workspace_id, &input)
        .map_err(|e| IpcError::new("AG_HISTORY_DB_ERROR", &e.to_string(), true, false))?;
    Ok(serde_json::json!({ "ok": true }))
}

/**
 * @GOV
 * codes:    BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-022
 * type:     EFFECT
 * chain:    AG-TURN
 * rules:    BR-AG-PERSIST-001
 * boundary: in=ExistingAgentTurnIdentityAndAgentTurnPayloadV2Projection | out=UpdatedAgentTurnDisplayCache | does_not=writeRuntimeStatusOrTurnEvent
 */
pub(crate) fn route_save_agent_turn(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let ws = active_ws(state)?;
    let session_id = str_param(request, "sessionId", "AG_SESSION_ID_REQUIRED")?;
    let turn_id = str_param(request, "turnId", "AG_TURN_ID_REQUIRED")?;
    let payload_value = request.params.get("payload").cloned().ok_or_else(|| {
        IpcError::new(
            "AG_TURN_PAYLOAD_REQUIRED",
            "缺少 payload 参数",
            false,
            false,
        )
    })?;
    let payload = serde_json::from_value::<ag::history::AgentTurnPayload>(payload_value)
        .map_err(|e| IpcError::new("AG_TURN_PAYLOAD_INVALID", &e.to_string(), false, false))?;
    let rel_path = request.params.get("relPath").and_then(|v| v.as_str());
    let conn = lock_db(state)?;
    ag::history::save_turn_projection(
        &conn,
        &ws.workspace_id,
        &session_id,
        &turn_id,
        rel_path,
        &payload,
    )
    .map_err(|e| IpcError::new("AG_HISTORY_DB_ERROR", &e.to_string(), true, false))?;
    Ok(serde_json::json!({ "ok": true }))
}

/**
 * @GOV
 * codes:    BR-AG-PERSIST-001-QUERY-AG-AG-TURN-023
 * type:     QUERY
 * chain:    AG-TURN
 * rules:    BR-AG-PERSIST-001
 * boundary: in=AgentSessionIdentity | out=PersistedAgentTurnHistory
 */
pub(crate) fn route_list_agent_turns(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let ws = active_ws(state)?;
    let session_id = str_param(request, "sessionId", "AG_SESSION_ID_REQUIRED")?;
    let conn = lock_db(state)?;
    let turns = ag::history::list_turns(&conn, &ws.workspace_id, &session_id)
        .map_err(|e| IpcError::new("AG_HISTORY_DB_ERROR", &e.to_string(), true, false))?;
    ws_json(&turns)
}

/**
 * @GOV
 * codes:    BR-AG-PERSIST-001-QUERY-AG-AG-TURN-047, BR-SYS-STATE-007-QUERY-AG-AG-TURN-047
 * type:     QUERY
 * chain:    AG-TURN
 * rules:    BR-AG-PERSIST-001, BR-SYS-STATE-007
 * term_ref: TERM-AG-001, TERM-AG-022
 * boundary: in=ActiveWorkspaceTurnIdAndOptionalAfterSeq | out=AscendingSafeAgTurnEventProjection | effect=IPC-ag.list_turn_events | does_not=returnInternalEventsOrCrossWorkspaceTurn
 */
pub(crate) fn route_list_agent_turn_events(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let ws = active_ws(state)?;
    let turn_id = str_param(request, "turnId", "AG_TURN_ID_REQUIRED")?;
    let after_seq = request
        .params
        .get("afterSeq")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let conn = lock_db(state)?;
    let events = ag::turn_events::list_safe_events(&conn, &ws.workspace_id, &turn_id, after_seq)
        .map_err(|error| {
            if matches!(error, rusqlite::Error::QueryReturnedNoRows) {
                IpcError::new(
                    "AG_TURN_NOT_FOUND",
                    "回合不存在或不属于当前 workspace",
                    false,
                    false,
                )
            } else {
                IpcError::new("AG_HISTORY_DB_ERROR", &error.to_string(), true, false)
            }
        })?;
    ws_json(&events)
}

/// AG-1.0-IMP-TASKRUN-UI-01：`ag.list_task_runs` —— 按会话列受限任务实例
/// （SM-AG-04 状态/步骤，updated_at 倒序；只读，不伪造进度，BR-SYS-STATE-008）。

/**
 * @GOV
 * codes:    BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-024
 * type:     EFFECT
 * chain:    AG-TURN
 * rules:    BR-AG-PERSIST-001
 * boundary: in=AgentSessionDeletionIdentity | out=DeletedAgentSessionAndOwnedTurnHistory | does_not=deleteDiffArtifactsOrFilesOrOtherSessions
 */
pub(crate) fn route_delete_agent_session(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let ws = active_ws(state)?;
    let session_id = str_param(request, "sessionId", "AG_SESSION_ID_REQUIRED")?;
    let mut conn = lock_db(state)?;
    let result = ag::history::delete_session(&mut conn, &ws.workspace_id, &session_id)
        .map_err(|e| IpcError::new("AG_HISTORY_DB_ERROR", &e.to_string(), true, false))?;
    ws_json(&result)
}
