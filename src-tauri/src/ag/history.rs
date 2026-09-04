//! PI-AG-12 · 代理编辑历史持久化 · AG-TURN / BR-AG-PERSIST-001
//!
//! 会话标签元数据全恢复；回合内容按 session 懒加载；工具卡只存只读摘要；DiffCard 只存引用。

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

pub const HISTORY_SOFT_LIMIT_BYTES: i64 = 200 * 1024 * 1024;

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionRecord {
    pub session_id: String,
    pub workspace_id: String,
    pub title: String,
    pub mode: String,
    pub archived: bool,
    pub open_index: i64,
    pub is_active: bool,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_active_at: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentSessionInput {
    pub session_id: String,
    pub title: String,
    pub mode: String,
    pub archived: bool,
    pub open_index: i64,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedChatMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parts: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub streaming: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<bool>,
    /// 回合内消息片段（TERM-AG-011 · BR-AG-UI-001）：additive，旧记录缺省 → None（前端降级沉底）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub segments: Option<Vec<TurnSegment>>,
}

/// 回合内消息片段（TERM-AG-011）：text=流式累积正文 / tool=引用 ToolCall 摘要 / diff=引用 DiffRef。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TurnSegment {
    Text {
        text: String,
    },
    Tool {
        #[serde(rename = "toolCallId")]
        tool_call_id: String,
    },
    Diff {
        #[serde(rename = "diffId")]
        diff_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolSummary {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_summary: Option<String>,
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor_message_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffRef {
    pub diff_id: String,
    pub rel_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor_message_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnFailure {
    pub code: String,
    pub message: String,
    pub retriable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTurnPayload {
    pub version: i64,
    /// 展示缓存已归约到的最后一个安全 TurnEvent seq；旧 V1 记录缺省为 0。
    #[serde(default)]
    pub last_event_seq: u64,
    pub user_message: PersistedChatMessage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assistant_message: Option<PersistedChatMessage>,
    pub tool_summaries: Vec<ToolSummary>,
    pub diff_refs: Vec<DiffRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure: Option<TurnFailure>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub incomplete_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentTurnRecord {
    pub turn_id: String,
    pub session_id: String,
    pub workspace_id: String,
    pub rel_path: Option<String>,
    pub status: String,
    pub payload: AgentTurnPayload,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentSessionsResponse {
    pub sessions: Vec<AgentSessionRecord>,
    pub active_session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentSessionDeleteResult {
    pub deleted_sessions: usize,
    pub deleted_turns: usize,
    pub deleted_payloads: usize,
    pub deleted_events: usize,
}

pub fn is_valid_status(status: &str) -> bool {
    matches!(
        status,
        "completed" | "error" | "cancelled" | "gate_rejected" | "draft"
    )
}

fn normalize_mode(mode: &str) -> &str {
    if mode == "build" {
        "build"
    } else {
        "edit"
    }
}

/**
 * @GOV
 * codes:    BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-030
 * type:     EFFECT
 * chain:    AG-TURN
 * rules:    BR-AG-PERSIST-001
 * term_ref: TERM-AG-009
 * boundary: in=AgentSessionMetadata | out=persistedAgentSession(localWorkspaceScoped)
 */
/// 持久化代理编辑会话标签元数据：标题、模式、归档、打开顺序与激活态可重启恢复。
pub fn upsert_session(
    conn: &Connection,
    workspace_id: &str,
    input: &AgentSessionInput,
) -> rusqlite::Result<()> {
    let now = now_millis();
    if input.is_active {
        conn.execute(
            "UPDATE agent_sessions SET is_active = 0 WHERE workspace_id = ?1",
            [workspace_id],
        )?;
    }
    conn.execute(
        "INSERT INTO agent_sessions \
         (session_id, workspace_id, title, mode, archived, open_index, is_active, created_at, updated_at, last_active_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, ?9) \
         ON CONFLICT(session_id) DO UPDATE SET \
           title=excluded.title, mode=excluded.mode, archived=excluded.archived, open_index=excluded.open_index, \
           is_active=excluded.is_active, updated_at=excluded.updated_at, last_active_at=excluded.last_active_at",
        params![
            &input.session_id,
            workspace_id,
            &input.title,
            normalize_mode(&input.mode),
            input.archived as i64,
            input.open_index,
            input.is_active as i64,
            now,
            if input.is_active { Some(now) } else { None },
        ],
    )?;
    Ok(())
}

pub fn ensure_session(
    conn: &Connection,
    workspace_id: &str,
    session_id: &str,
    title: &str,
    mode: &str,
    open_index: i64,
    is_active: bool,
) -> rusqlite::Result<()> {
    upsert_session(
        conn,
        workspace_id,
        &AgentSessionInput {
            session_id: session_id.to_string(),
            title: title.to_string(),
            mode: mode.to_string(),
            archived: false,
            open_index,
            is_active,
        },
    )
}

pub fn ensure_session_exists(
    conn: &Connection,
    workspace_id: &str,
    session_id: &str,
) -> rusqlite::Result<()> {
    let now = now_millis();
    conn.execute(
        "INSERT OR IGNORE INTO agent_sessions \
         (session_id, workspace_id, title, mode, archived, open_index, is_active, created_at, updated_at, last_active_at) \
         VALUES (?1, ?2, '新对话', 'edit', 0, 0, 1, ?3, ?3, ?3)",
        params![session_id, workspace_id, now],
    )?;
    Ok(())
}

/**
 * @GOV
 * codes:    BR-AG-PERSIST-001-QUERY-AG-AG-TURN-002
 * type:     QUERY
 * chain:    AG-TURN
 * rules:    BR-AG-PERSIST-001
 * term_ref: TERM-AG-009
 * boundary: in=workspaceId | out=agentSessionsMetadata(allTagsRecoverable)
 */
/// 列出当前 workspace 的全部代理编辑会话标签元数据，不加载回合 payload。
pub fn list_sessions(
    conn: &Connection,
    workspace_id: &str,
) -> rusqlite::Result<AgentSessionsResponse> {
    let mut stmt = conn.prepare(
        "SELECT session_id, workspace_id, title, mode, archived, open_index, is_active, created_at, updated_at, last_active_at \
         FROM agent_sessions WHERE workspace_id = ?1 ORDER BY open_index ASC, created_at ASC",
    )?;
    let rows = stmt.query_map([workspace_id], |r| {
        Ok(AgentSessionRecord {
            session_id: r.get(0)?,
            workspace_id: r.get(1)?,
            title: r.get(2)?,
            mode: r.get(3)?,
            archived: r.get::<_, i64>(4)? != 0,
            open_index: r.get(5)?,
            is_active: r.get::<_, i64>(6)? != 0,
            created_at: r.get(7)?,
            updated_at: r.get(8)?,
            last_active_at: r.get(9)?,
        })
    })?;
    let sessions: Vec<AgentSessionRecord> = rows.collect::<rusqlite::Result<_>>()?;
    let active_session_id = sessions
        .iter()
        .find(|s| s.is_active)
        .map(|s| s.session_id.clone())
        .or_else(|| {
            sessions
                .iter()
                .find(|s| !s.archived)
                .map(|s| s.session_id.clone())
        })
        .or_else(|| sessions.first().map(|s| s.session_id.clone()));
    Ok(AgentSessionsResponse {
        sessions,
        active_session_id,
    })
}

/**
 * @GOV
 * codes:    BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-003
 * type:     EFFECT
 * chain:    AG-TURN
 * rules:    BR-AG-PERSIST-001
 * term_ref: TERM-AG-001, TERM-AG-009
 * boundary: in=AgentTurnPayloadAndStatus | out=persistedAgentTurn(fullStatusRecoverable)
 */
/// 写入或更新代理回合历史。payload 只保存消息、工具摘要与 Diff 引用，不保存 diff 快照。
pub fn save_turn(
    conn: &Connection,
    workspace_id: &str,
    session_id: &str,
    turn_id: &str,
    rel_path: Option<&str>,
    status: &str,
    payload: &AgentTurnPayload,
) -> rusqlite::Result<()> {
    if !is_valid_status(status) {
        return Err(rusqlite::Error::InvalidParameterName(format!(
            "invalid agent turn status: {status}"
        )));
    }
    let now = now_millis();
    let payload_ref = format!("agp-{turn_id}");
    let payload_json = serde_json::to_string(payload)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    let payload_size = payload_json.len() as i64;
    conn.execute(
        "INSERT INTO agent_turn_payloads (payload_ref, workspace_id, payload_json, payload_size, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5) \
         ON CONFLICT(payload_ref) DO UPDATE SET payload_json=excluded.payload_json, payload_size=excluded.payload_size",
        params![payload_ref, workspace_id, payload_json, payload_size, now],
    )?;
    let existing_created_at = conn
        .query_row(
            "SELECT created_at FROM agent_turns WHERE turn_id = ?1",
            [turn_id],
            |r| r.get::<_, i64>(0),
        )
        .optional()?;
    conn.execute(
        "INSERT INTO agent_turns (turn_id, workspace_id, rel_path, payload_ref, created_at, session_id, status, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) \
         ON CONFLICT(turn_id) DO UPDATE SET rel_path=excluded.rel_path, payload_ref=excluded.payload_ref, \
           session_id=excluded.session_id, status=excluded.status, updated_at=excluded.updated_at",
        params![
            turn_id,
            workspace_id,
            rel_path,
            payload_ref,
            existing_created_at.unwrap_or(now),
            session_id,
            status,
            now,
        ],
    )?;
    soft_archive_over_limit(conn, workspace_id, HISTORY_SOFT_LIMIT_BYTES)?;
    Ok(())
}

/**
 * @GOV
 * codes:    BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-046
 * type:     EFFECT
 * chain:    AG-TURN
 * rules:    BR-AG-PERSIST-001
 * term_ref: TERM-AG-001, TERM-AG-009, TERM-AG-022
 * boundary: in=ExistingAgentTurnIdentityAndAgentTurnPayloadV2Projection | out=UpdatedRecoverableDisplayCacheAtCommittedEventSeq | does_not=createAgentTurnOverwriteRuntimeStatusEventOrderToolResultDecisionOrDiffFact
 */
pub fn save_turn_projection(
    conn: &Connection,
    workspace_id: &str,
    session_id: &str,
    turn_id: &str,
    rel_path: Option<&str>,
    payload: &AgentTurnPayload,
) -> rusqlite::Result<()> {
    if payload.version != 2 {
        return Err(rusqlite::Error::InvalidParameterName(
            "AgentTurnPayload projection writes require version=2".to_string(),
        ));
    }
    let payload_ref = conn
        .query_row(
            "SELECT payload_ref FROM agent_turns \
             WHERE turn_id = ?1 AND workspace_id = ?2 AND session_id = ?3",
            params![turn_id, workspace_id, session_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
    let committed_event_seq = conn.query_row(
        "SELECT COALESCE(MAX(seq), 0) FROM agent_turn_events WHERE turn_id = ?1",
        [turn_id],
        |row| row.get::<_, u64>(0),
    )?;
    if payload.last_event_seq > committed_event_seq {
        return Err(rusqlite::Error::InvalidParameterName(format!(
            "lastEventSeq {} exceeds committed seq {committed_event_seq}",
            payload.last_event_seq
        )));
    }
    let now = now_millis();
    let payload_json = serde_json::to_string(payload)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    conn.execute(
        "UPDATE agent_turn_payloads \
         SET payload_json = ?2, payload_size = ?3 \
         WHERE payload_ref = ?1 AND workspace_id = ?4",
        params![
            payload_ref,
            payload_json,
            payload_json.len() as i64,
            workspace_id
        ],
    )?;
    conn.execute(
        "UPDATE agent_turns SET rel_path = ?2, updated_at = ?3 \
         WHERE turn_id = ?1 AND workspace_id = ?4 AND session_id = ?5",
        params![turn_id, rel_path, now, workspace_id, session_id],
    )?;
    Ok(())
}

/**
 * @GOV
 * codes:    BR-AG-PERSIST-001-QUERY-AG-AG-TURN-003
 * type:     QUERY
 * chain:    AG-TURN
 * rules:    BR-AG-PERSIST-001
 * term_ref: TERM-AG-001, TERM-AG-009
 * boundary: in=sessionId | out=agentTurns(lazyHydratedInOrder)
 */
/// 懒加载指定代理编辑会话的回合历史；只返回当前 workspace + session_id 匹配的记录。
pub fn list_turns(
    conn: &Connection,
    workspace_id: &str,
    session_id: &str,
) -> rusqlite::Result<Vec<AgentTurnRecord>> {
    let mut stmt = conn.prepare(
        "SELECT t.turn_id, t.session_id, t.workspace_id, t.rel_path, t.status, p.payload_json, t.created_at, t.updated_at \
         FROM agent_turns t JOIN agent_turn_payloads p ON p.payload_ref = t.payload_ref \
         WHERE t.workspace_id = ?1 AND t.session_id = ?2 ORDER BY t.created_at ASC",
    )?;
    let rows = stmt.query_map(params![workspace_id, session_id], |r| {
        let payload_json: String = r.get(5)?;
        let payload = serde_json::from_str::<AgentTurnPayload>(&payload_json).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, Box::new(e))
        })?;
        Ok(AgentTurnRecord {
            turn_id: r.get(0)?,
            session_id: r.get(1)?,
            workspace_id: r.get(2)?,
            rel_path: r.get(3)?,
            status: r.get(4)?,
            payload,
            created_at: r.get(6)?,
            updated_at: r.get(7)?,
        })
    })?;
    rows.collect()
}

/**
 * @GOV
 * codes:    BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-031
 * type:     EFFECT
 * chain:    AG-TURN
 * rules:    BR-AG-PERSIST-001
 * term_ref: TERM-AG-001, TERM-AG-009
 * boundary: in=agentSessionDeleteRequest | out=agentSessionHistoryDeleted(deDiffsPreserved)
 */
/// 删除当前 workspace 内指定代理编辑会话及其回合历史；不删除 DE pending_diffs / diff_payloads。
pub fn delete_session(
    conn: &mut Connection,
    workspace_id: &str,
    session_id: &str,
) -> rusqlite::Result<AgentSessionDeleteResult> {
    let tx = conn.transaction()?;
    let deleted_events = tx.execute(
        "DELETE FROM agent_turn_events WHERE turn_id IN ( \
           SELECT turn_id FROM agent_turns WHERE workspace_id = ?1 AND session_id = ?2 \
         )",
        params![workspace_id, session_id],
    )?;
    let deleted_payloads = tx.execute(
        "DELETE FROM agent_turn_payloads WHERE workspace_id = ?1 AND payload_ref IN ( \
           SELECT payload_ref FROM agent_turns WHERE workspace_id = ?1 AND session_id = ?2 \
         )",
        params![workspace_id, session_id],
    )?;
    let deleted_turns = tx.execute(
        "DELETE FROM agent_turns WHERE workspace_id = ?1 AND session_id = ?2",
        params![workspace_id, session_id],
    )?;
    let deleted_sessions = tx.execute(
        "DELETE FROM agent_sessions WHERE workspace_id = ?1 AND session_id = ?2",
        params![workspace_id, session_id],
    )?;
    tx.commit()?;
    Ok(AgentSessionDeleteResult {
        deleted_sessions,
        deleted_turns,
        deleted_payloads,
        deleted_events,
    })
}

pub fn soft_archive_over_limit(
    conn: &Connection,
    workspace_id: &str,
    limit_bytes: i64,
) -> rusqlite::Result<()> {
    let total: i64 = conn.query_row(
        "SELECT COALESCE(SUM(payload_size), 0) FROM agent_turn_payloads WHERE workspace_id = ?1",
        [workspace_id],
        |r| r.get(0),
    )?;
    if total <= limit_bytes {
        return Ok(());
    }
    conn.execute(
        "UPDATE agent_sessions SET archived = 1, updated_at = ?2 \
         WHERE workspace_id = ?1 AND is_active = 0 AND archived = 0 AND session_id IN ( \
           SELECT session_id FROM agent_sessions WHERE workspace_id = ?1 AND is_active = 0 AND archived = 0 \
           ORDER BY COALESCE(last_active_at, created_at) ASC LIMIT 1 \
         )",
        params![workspace_id, now_millis()],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn fresh() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        db::migrate(&mut conn).unwrap();
        conn.execute(
            "INSERT INTO workspaces (workspace_id, root_path, is_active, opened_at) VALUES ('ws1','/w',1,0)",
            [],
        )
        .unwrap();
        conn
    }

    fn payload(text: &str) -> AgentTurnPayload {
        AgentTurnPayload {
            version: 1,
            last_event_seq: 0,
            user_message: PersistedChatMessage {
                id: "u1".into(),
                role: "user".into(),
                content: text.into(),
                parts: None,
                streaming: None,
                error: None,
                segments: None,
            },
            assistant_message: Some(PersistedChatMessage {
                id: "a1".into(),
                role: "assistant".into(),
                content: "ok".into(),
                parts: None,
                streaming: Some(false),
                error: None,
                segments: None,
            }),
            tool_summaries: vec![ToolSummary {
                id: "tc1".into(),
                name: "read_doc".into(),
                input_summary: Some("a.md".into()),
                result_summary: Some("done".into()),
                state: "done".into(),
                anchor_message_id: Some("a1".into()),
            }],
            diff_refs: vec![DiffRef {
                diff_id: "d1".into(),
                rel_path: "a.md".into(),
                anchor_message_id: Some("a1".into()),
            }],
            failure: None,
            incomplete_reason: None,
        }
    }

    // covers: BR-AG-PERSIST-001, BR-AG-UI-001
    #[test]
    fn legacy_payload_without_segments_deserializes_to_none_and_new_segments_roundtrip() {
        // 新 payload：assistantMessage 携带 segments（text/tool/diff 时序交错）。
        let json = r#"{"version":1,"userMessage":{"id":"u1","role":"user","content":"hi"},"assistantMessage":{"id":"a1","role":"assistant","content":"先看文档","segments":[{"kind":"text","text":"先看文档"},{"kind":"tool","toolCallId":"tc1"},{"kind":"diff","diffId":"d1"}]},"toolSummaries":[{"id":"tc1","name":"read_doc","state":"done","anchorMessageId":"a1"}],"diffRefs":[{"diffId":"d1","relPath":"a.md","anchorMessageId":"a1"}]}"#;
        let p: AgentTurnPayload = serde_json::from_str(json).unwrap();
        let segments = p.assistant_message.unwrap().segments.unwrap();
        assert_eq!(segments.len(), 3);
        match &segments[1] {
            TurnSegment::Tool { tool_call_id } => assert_eq!(tool_call_id, "tc1"),
            other => panic!("expected tool segment, got {other:?}"),
        }
        match &segments[2] {
            TurnSegment::Diff { diff_id } => assert_eq!(diff_id, "d1"),
            other => panic!("expected diff segment, got {other:?}"),
        }

        // 旧记录（无 segments 字段）→ None，前端降级沉底、不丢数据（BR-AG-PERSIST-001）。
        let legacy = r#"{"version":1,"userMessage":{"id":"u1","role":"user","content":"hi"},"assistantMessage":{"id":"a1","role":"assistant","content":"ok"},"toolSummaries":[],"diffRefs":[]}"#;
        let p2: AgentTurnPayload = serde_json::from_str(legacy).unwrap();
        assert_eq!(p2.last_event_seq, 0);
        assert!(p2.assistant_message.unwrap().segments.is_none());
    }

    // covers: BR-AG-PERSIST-001, BR-AG-STATE-001
    #[test]
    fn projection_write_requires_v2_committed_cursor_and_preserves_runtime_status() {
        let conn = fresh();
        ensure_session(&conn, "ws1", "s1", "标题", "edit", 0, true).unwrap();
        save_turn(
            &conn,
            "ws1",
            "s1",
            "t1",
            Some("before.md"),
            "completed",
            &payload("hello"),
        )
        .unwrap();
        conn.execute(
            "INSERT INTO agent_turn_events \
             (event_id, workspace_id, session_id, turn_id, seq, kind, payload_version, payload_json, occurred_at) \
             VALUES ('e1', 'ws1', 's1', 't1', 1, 'done', 1, '{}', 1)",
            [],
        )
        .unwrap();

        let mut projection = payload("updated");
        projection.version = 2;
        projection.last_event_seq = 1;
        save_turn_projection(&conn, "ws1", "s1", "t1", Some("after.md"), &projection).unwrap();

        let turn = list_turns(&conn, "ws1", "s1").unwrap().remove(0);
        assert_eq!(turn.status, "completed");
        assert_eq!(turn.rel_path.as_deref(), Some("after.md"));
        assert_eq!(turn.payload.version, 2);
        assert_eq!(turn.payload.last_event_seq, 1);

        projection.last_event_seq = 2;
        assert!(
            save_turn_projection(&conn, "ws1", "s1", "t1", Some("future.md"), &projection,)
                .is_err()
        );
        assert_eq!(
            list_turns(&conn, "ws1", "s1").unwrap()[0]
                .rel_path
                .as_deref(),
            Some("after.md")
        );
    }

    // covers: BR-AG-PERSIST-001
    #[test]
    fn sessions_and_turns_roundtrip_by_workspace_and_session() {
        let conn = fresh();
        ensure_session(&conn, "ws1", "s1", "标题", "edit", 0, true).unwrap();
        save_turn(
            &conn,
            "ws1",
            "s1",
            "t1",
            Some("a.md"),
            "completed",
            &payload("hello"),
        )
        .unwrap();
        save_turn(
            &conn,
            "ws1",
            "s1",
            "t2",
            None,
            "gate_rejected",
            &payload("danger"),
        )
        .unwrap();

        let sessions = list_sessions(&conn, "ws1").unwrap();
        assert_eq!(sessions.active_session_id.as_deref(), Some("s1"));
        assert_eq!(sessions.sessions[0].title, "标题");

        let turns = list_turns(&conn, "ws1", "s1").unwrap();
        assert_eq!(
            turns.iter().map(|t| t.status.as_str()).collect::<Vec<_>>(),
            ["completed", "gate_rejected"]
        );
        assert_eq!(turns[0].payload.tool_summaries[0].name, "read_doc");
        assert_eq!(turns[0].payload.diff_refs[0].diff_id, "d1");
        assert!(list_turns(&conn, "other", "s1").unwrap().is_empty());
    }

    // covers: BR-AG-PERSIST-001
    #[test]
    fn soft_archive_marks_oldest_inactive_session_when_payloads_exceed_limit() {
        let conn = fresh();
        ensure_session(&conn, "ws1", "old", "旧", "edit", 0, false).unwrap();
        ensure_session(&conn, "ws1", "active", "当前", "edit", 1, true).unwrap();
        save_turn(
            &conn,
            "ws1",
            "old",
            "t-old",
            None,
            "draft",
            &payload(&"x".repeat(100)),
        )
        .unwrap();
        soft_archive_over_limit(&conn, "ws1", 1).unwrap();
        let sessions = list_sessions(&conn, "ws1").unwrap();
        let old = sessions
            .sessions
            .iter()
            .find(|s| s.session_id == "old")
            .unwrap();
        let active = sessions
            .sessions
            .iter()
            .find(|s| s.session_id == "active")
            .unwrap();
        assert!(old.archived);
        assert!(!active.archived);
    }

    // covers: BR-AG-PERSIST-001
    #[test]
    fn delete_session_removes_only_selected_agent_session_history() {
        let mut conn = fresh();
        conn.execute(
            "INSERT INTO workspaces (workspace_id, root_path, is_active, opened_at) VALUES ('ws2','/w2',0,0)",
            [],
        )
        .unwrap();
        ensure_session(&conn, "ws1", "s1", "当前", "edit", 0, true).unwrap();
        ensure_session(&conn, "ws1", "s1-keep", "同工作区保留", "edit", 1, false).unwrap();
        ensure_session(&conn, "ws2", "s2", "其他", "edit", 0, true).unwrap();
        save_turn(
            &conn,
            "ws1",
            "s1",
            "t1",
            Some("a.md"),
            "completed",
            &payload("hello"),
        )
        .unwrap();
        save_turn(
            &conn,
            "ws1",
            "s1-keep",
            "t1-keep",
            Some("keep.md"),
            "completed",
            &payload("keep"),
        )
        .unwrap();
        save_turn(
            &conn,
            "ws2",
            "s2",
            "t2",
            Some("b.md"),
            "completed",
            &payload("other"),
        )
        .unwrap();
        conn.execute(
            "INSERT INTO agent_turn_events \
             (event_id, workspace_id, session_id, turn_id, seq, kind, payload_version, payload_json, occurred_at) \
             VALUES ('event-delete', 'ws1', 's1', 't1', 1, 'done', 1, '{}', 1), \
                    ('event-keep', 'ws1', 's1-keep', 't1-keep', 1, 'done', 1, '{}', 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO pending_diffs (diff_id, workspace_id, rel_path, origin_module, status, payload_ref, created_at) \
             VALUES ('d1', 'ws1', 'a.md', 'AG', 'pending', 'dp1', 1)",
            [],
        )
        .unwrap();

        let result = delete_session(&mut conn, "ws1", "s1").unwrap();

        assert_eq!(result.deleted_sessions, 1);
        assert_eq!(result.deleted_turns, 1);
        assert_eq!(result.deleted_payloads, 1);
        assert_eq!(result.deleted_events, 1);
        let ws1_sessions = list_sessions(&conn, "ws1").unwrap();
        assert_eq!(ws1_sessions.sessions.len(), 1);
        assert_eq!(ws1_sessions.sessions[0].session_id, "s1-keep");
        assert!(list_turns(&conn, "ws1", "s1").unwrap().is_empty());
        assert_eq!(list_turns(&conn, "ws1", "s1-keep").unwrap().len(), 1);
        let kept_event_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM agent_turn_events WHERE turn_id = 't1-keep'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(kept_event_count, 1);
        assert_eq!(list_sessions(&conn, "ws2").unwrap().sessions.len(), 1);
        assert_eq!(list_turns(&conn, "ws2", "s2").unwrap().len(), 1);
        let pending_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pending_diffs WHERE workspace_id = 'ws1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(pending_count, 1);
    }
}
