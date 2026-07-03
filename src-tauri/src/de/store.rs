//! PI-BE-02 · DiffPayload 持久化 · DE-DIFF / BR-DE-DATA-002
//!
//! 承接 A-DE-M-T-01 §1（DE 拥有 payload 形状、WS db 承载持久化）与 A-DE-I-P-01 §7（本 PI 解除存储
//! 形态延期项）：DiffPayload（TERM-DE-004）落 workspace.db `diff_payloads`，经 PendingDiff.payload_ref
//! 关联；accepted 落盘（de.accept_diff→ws.apply_diff）经此取回 after_content。

use super::generate::DiffPayload;
use crate::ipc::IpcError;
use rusqlite::{Connection, OptionalExtension};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// 取回的落盘载荷（accept 落盘所需：目标相对路径 + 提案后内容 + 样式操作序列可空）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredPayload {
    pub rel_path: String,
    pub after_content: Vec<u8>,
    pub is_new_doc: bool,
    /// 样式操作序列（StyleOp JSON，可空；PI-ED-11——审阅呈现与未来样式 apply 语义载体）。
    pub style_ops: Option<String>,
}

/**
 * @GOV
 * codes:    BR-DE-DATA-002-IO-DE-DE-DIFF-002
 * type:     IO
 * chain:    DE-DIFF
 * rules:    BR-DE-DATA-002
 * term_ref: TERM-DE-004
 * boundary: in=DiffPayload | out=persistedDiffPayload(byPayloadRefRestartRecoverable)
 */
/// 持久化 DiffPayload（BR-DE-DATA-002）：按 payload_ref 入 diff_payloads，重启可恢复；
/// 载荷形状统一（target/after_content/before_snapshot/is_new_doc/style_ops 可空——PI-ED-11
/// 样式操作序列，A-DE-M-T-01 v1.1），供 accepted 落盘取回内容与样式审阅呈现。
pub fn persist_payload(
    conn: &Connection,
    payload_ref: &str,
    payload: &DiffPayload,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO diff_payloads \
         (payload_ref, workspace_id, rel_path, after_content, before_snapshot, is_new_doc, style_ops, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            payload_ref,
            payload.target.workspace_id,
            payload.target.rel_path,
            payload.after_content,
            payload.before_snapshot,
            payload.is_new_doc as i64,
            payload.style_ops,
            now_millis(),
        ],
    )?;
    Ok(())
}

/**
 * @GOV
 * codes:    BR-DE-DATA-002-QUERY-DE-DE-DIFF-001
 * type:     QUERY
 * chain:    DE-DIFF
 * rules:    BR-DE-DATA-002
 * term_ref: TERM-DE-004
 * boundary: in=diffId | out=StoredPayload(joinedViaPayloadRefForApply)
 */
/// 经 diff_id 取回落盘载荷（BR-DE-DATA-002）：pending_diffs.payload_ref 关联 diff_payloads，
/// 回 rel_path + after_content（accept 落盘消费）；diff 或载荷缺失回统一错误形状（不静默成功）。
pub fn load_payload_for_diff(conn: &Connection, diff_id: &str) -> Result<StoredPayload, IpcError> {
    let row = conn
        .query_row(
            "SELECT p.rel_path, p.after_content, p.is_new_doc, p.style_ops \
             FROM pending_diffs d JOIN diff_payloads p ON d.payload_ref = p.payload_ref \
             WHERE d.diff_id = ?1",
            [diff_id],
            |r| {
                Ok(StoredPayload {
                    rel_path: r.get(0)?,
                    after_content: r.get(1)?,
                    is_new_doc: r.get::<_, i64>(2)? != 0,
                    style_ops: r.get(3)?,
                })
            },
        )
        .optional()
        .map_err(|e| IpcError::new("DE_DB_ERROR", &format!("载荷查询失败: {e}"), true, false))?;
    row.ok_or_else(|| {
        IpcError::new("DE_PAYLOAD_MISSING", &format!("diff 载荷不存在: {diff_id}"), false, false)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::de::generate::{build_payload, ChangeProposal};
    use crate::types::DocRef;
    use crate::workspace::diff_apply::persist_diff;
    use crate::types::{DiffStatus, PendingDiff};

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

    // covers: BR-DE-DATA-002
    // 载荷往返：持久化 DiffPayload 后经 diff_id（关联 payload_ref）取回 rel_path + after_content；
    // 缺失 diff 回 DE_PAYLOAD_MISSING（不静默）。
    #[test]
    fn payload_persist_and_load_roundtrip() {
        let conn = fresh();
        let proposal = ChangeProposal {
            origin_task: "turn1".into(),
            target: DocRef { workspace_id: "ws1".into(), rel_path: "a.md".into(), doc_type: "md".into() },
            after_content: b"new body".to_vec(),
            before_snapshot: Some("h0".into()),
            style_ops: Some(r#"[{"op":"set_mark","blockId":"b1","start":0,"end":3,"mark":{"type":"bold"}}]"#.into()),
        };
        let payload = build_payload(&proposal);
        persist_payload(&conn, "pl1", &payload).unwrap();
        persist_diff(
            &conn,
            &PendingDiff {
                diff_id: "d1".into(),
                target: proposal.target.clone(),
                origin_module: "AG".into(),
                status: DiffStatus::Pending,
                created_at: 0,
                payload_ref: "pl1".into(),
            },
        )
        .unwrap();

        let got = load_payload_for_diff(&conn, "d1").unwrap();
        assert_eq!(got.rel_path, "a.md");
        assert_eq!(got.after_content, b"new body");
        assert!(!got.is_new_doc);
        // PI-ED-11：样式操作序列随载荷持久化并可取回（迁移 v5 style_ops 列，可空字段往返）。
        assert!(got.style_ops.as_deref().unwrap_or("").contains("set_mark"));

        let err = load_payload_for_diff(&conn, "missing").unwrap_err();
        assert_eq!(err.code, "DE_PAYLOAD_MISSING");
    }
}
