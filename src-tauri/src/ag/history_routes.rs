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
 * boundary: in=AgentSessionMetadataCommand | out=PersistedAgentSessionMetadata
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
 * boundary: in=AgentTurnPayloadAndLifecycleStatus | out=PersistedAgentTurnHistory
 */
pub(crate) fn route_save_agent_turn(
    request: &CommandRequest,
    state: &AppState,
) -> Result<serde_json::Value, IpcError> {
    let ws = active_ws(state)?;
    let session_id = str_param(request, "sessionId", "AG_SESSION_ID_REQUIRED")?;
    let turn_id = str_param(request, "turnId", "AG_TURN_ID_REQUIRED")?;
    let status = str_param(request, "status", "AG_TURN_STATUS_REQUIRED")?;
    if !ag::history::is_valid_status(&status) {
        return Err(IpcError::new(
            "AG_TURN_STATUS_INVALID",
            "未知代理回合历史状态",
            false,
            false,
        ));
    }
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
    ag::history::save_turn(
        &conn,
        &ws.workspace_id,
        &session_id,
        &turn_id,
        rel_path,
        &status,
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

/// AG-1.0-IMP-TASKRUN-UI-01：`ag.list_task_runs` —— 按会话列受限任务实例
/// （SM-AG-04 状态/步骤，updated_at 倒序；只读，不伪造进度，BR-SYS-STATE-008）。

/**
 * @GOV
 * codes:    BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-024
 * type:     EFFECT
 * chain:    AG-TURN
 * rules:    BR-AG-PERSIST-001
 * boundary: in=AgentSessionDeletionIdentity | out=DeletedAgentSessionAndOwnedTurnHistory
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
