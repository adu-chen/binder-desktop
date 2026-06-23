//! PI-WS-04 · 文档级时间轴存储与还原 · WS-TIMELINE / BR-WS-DATA-002 / BR-WS-STATE-002 / SM-WS-03
//!
//! 承接 A-WS-M-T-01 §6 / A-WS-M-S-01 SM-WS-03：落盘与还原各记恰好一个节点，同一文档内 version_seq
//! 单调递增且不复用（timeline 表 UNIQUE(workspace_id, rel_path, version_seq) 强制）；还原前进式记新
//! 节点（不抹历史），还原失败保持现状无半还原。快照物理存储形态为实现期 A 类债务（WS-03）。

use crate::ipc::IpcError;
use crate::types::{DocRef, TimelineEntry};
use rusqlite::{Connection, OptionalExtension};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn gen_entry_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    format!("e_{}_{}", now_millis(), COUNTER.fetch_add(1, Ordering::Relaxed))
}

fn doc_type_of(rel_path: &str) -> String {
    match rel_path.rsplit_once('.') {
        Some((_, ext)) if !ext.is_empty() => ext.to_string(),
        _ => String::new(),
    }
}

/// 下一个 version_seq（同文档内 max+1，单调递增）。
fn next_version_seq(conn: &Connection, ws_id: &str, rel_path: &str) -> rusqlite::Result<u64> {
    let max: Option<i64> = conn.query_row(
        "SELECT MAX(version_seq) FROM timeline WHERE workspace_id = ?1 AND rel_path = ?2",
        (ws_id, rel_path),
        |r| r.get(0),
    )?;
    Ok(max.unwrap_or(0) as u64 + 1)
}

/**
 * @GOV
 * codes:    BR-WS-DATA-002-EFFECT-WS-WS-TIMELINE-001
 * type:     EFFECT
 * chain:    WS-TIMELINE
 * rules:    BR-WS-DATA-002
 * boundary: in=DocRefAndSnapshotRef | out=TimelineEntry(monotonicVersionSeqNoReuse)
 */
/// 记一个时间轴节点（BR-WS-DATA-002）：version_seq 取同文档 max+1（单调、不复用，UNIQUE 强制），
/// 关联可取回的版本快照引用。entry_kind ∈ {apply, restore, save}。
pub fn append_node(
    conn: &Connection,
    doc: &DocRef,
    entry_kind: &str,
    snapshot_ref: &str,
    label: Option<&str>,
) -> rusqlite::Result<TimelineEntry> {
    let seq = next_version_seq(conn, &doc.workspace_id, &doc.rel_path)?;
    let entry_id = gen_entry_id();
    let created = now_millis();
    conn.execute(
        "INSERT INTO timeline \
         (entry_id, workspace_id, rel_path, version_seq, entry_kind, label, snapshot_ref, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        (
            &entry_id,
            &doc.workspace_id,
            &doc.rel_path,
            seq as i64,
            entry_kind,
            label,
            snapshot_ref,
            created,
        ),
    )?;
    Ok(TimelineEntry {
        entry_id,
        target: doc.clone(),
        version_seq: seq,
        label: label.map(String::from),
        created_at: created,
    })
}

/**
 * @GOV
 * codes:    BR-WS-DATA-002-QUERY-WS-WS-TIMELINE-001
 * type:     QUERY
 * chain:    WS-TIMELINE
 * rules:    BR-WS-DATA-002
 * boundary: in=DocRef | out=timelineEntryList(consumerOrderedByVersionSeq)
 */
/// 列出文档时间轴节点，按 version_seq 升序（BR-WS-DATA-002 consumer：节点可浏览、保序）。
pub fn list_timeline(conn: &Connection, doc: &DocRef) -> rusqlite::Result<Vec<TimelineEntry>> {
    let mut stmt = conn.prepare(
        "SELECT entry_id, rel_path, version_seq, label, created_at FROM timeline \
         WHERE workspace_id = ?1 AND rel_path = ?2 ORDER BY version_seq",
    )?;
    let rows = stmt.query_map((&doc.workspace_id, &doc.rel_path), |r| {
        let rel_path: String = r.get(1)?;
        Ok(TimelineEntry {
            entry_id: r.get(0)?,
            target: DocRef {
                workspace_id: doc.workspace_id.clone(),
                doc_type: doc_type_of(&rel_path),
                rel_path,
            },
            version_seq: r.get::<_, i64>(2)? as u64,
            label: r.get(3)?,
            created_at: r.get(4)?,
        })
    })?;
    rows.collect()
}

/// 存储时间轴快照内容（解除 WS-03）：内容寻址（snapshot_ref=blob_{hash}）去重，多节点共享同一内容只存一份。
pub fn store_snapshot(conn: &Connection, snapshot_ref: &str, content: &[u8]) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO timeline_snapshots (snapshot_ref, content, created_at) VALUES (?1, ?2, ?3)",
        (snapshot_ref, content, now_millis()),
    )?;
    Ok(())
}

/// 取回快照内容（按 snapshot_ref）；不存在记为 None。
pub fn load_snapshot(conn: &Connection, snapshot_ref: &str) -> rusqlite::Result<Option<Vec<u8>>> {
    conn.query_row(
        "SELECT content FROM timeline_snapshots WHERE snapshot_ref = ?1",
        [snapshot_ref],
        |r| r.get::<_, Vec<u8>>(0),
    )
    .optional()
}

/// 节点定位信息（preview/restore 用）：rel_path + snapshot_ref。
pub fn node_locator(conn: &Connection, entry_id: &str) -> rusqlite::Result<Option<(String, String)>> {
    conn.query_row(
        "SELECT rel_path, snapshot_ref FROM timeline WHERE entry_id = ?1",
        [entry_id],
        |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
    )
    .optional()
}

/**
 * @GOV
 * codes:    BR-WS-STATE-002-QUERY-WS-WS-TIMELINE-002
 * type:     QUERY
 * chain:    WS-TIMELINE
 * rules:    BR-WS-STATE-002
 * boundary: in=timelineEntryId | out=nodeSnapshotContent(guardRejectedWhenSnapshotUnavailable)
 */
/// 预览节点版本内容（BR-WS-STATE-002 / SM-WS-03 Previewing）：经节点 snapshot_ref 取回快照；
/// 节点不存在或快照不可取回 → guard 错误（不进入 Previewing，不返回错位内容）。
pub fn preview_node(conn: &Connection, entry_id: &str) -> Result<Vec<u8>, IpcError> {
    let db_err = |e: rusqlite::Error| IpcError::new("WS_DB_ERROR", &e.to_string(), true, false);
    let (_, snapshot_ref) = node_locator(conn, entry_id)
        .map_err(db_err)?
        .ok_or_else(|| IpcError::new("WS_NODE_NOT_FOUND", "时间轴节点不存在", false, false))?;
    load_snapshot(conn, &snapshot_ref)
        .map_err(db_err)?
        .ok_or_else(|| IpcError::new("WS_SNAPSHOT_UNAVAILABLE", "快照不可取回", false, false))
}

/// SM-WS-03 状态（A-WS-M-S-01 §4 时间轴还原机）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreState {
    Browsing,
    Previewing,
    Restoring,
    Restored,
    RestoreError,
}

/// SM-WS-03 事件。`SelectNode { snapshot_available }`——快照缺失时 guard 拒绝进入 Previewing。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreEvent {
    SelectNode { snapshot_available: bool },
    ConfirmRestore,
    RestoreOk,
    RestoreFail,
    Back,
}

/**
 * @GOV
 * codes:    BR-WS-STATE-002-RB-WS-WS-TIMELINE-001
 * type:     RB
 * chain:    WS-TIMELINE
 * rules:    BR-WS-STATE-002
 * boundary: in=RestoreEvent | out=RestoreState(noneWhenForbidden,failurePreservesCurrent)
 */
/// SM-WS-03 转移引擎（BR-WS-STATE-002）：还原成功前进式（restoreOk→Restored 记新节点）；
/// 还原失败保持现状（restoreFail→RestoreError，不达 Restored）；快照缺失/未经预览确认不得还原。
/// 返回 `None` = 无合法转移（含全部 forbidden_transitions）。
pub fn transition(state: RestoreState, event: RestoreEvent) -> Option<RestoreState> {
    use RestoreEvent::*;
    use RestoreState::*;
    let next = match (state, event) {
        (Browsing, SelectNode { snapshot_available: true }) => Previewing,
        (Previewing, SelectNode { snapshot_available: true }) => Previewing,
        (Previewing, ConfirmRestore) => Restoring,
        (Restoring, RestoreOk) => Restored,
        (Restoring, RestoreFail) => RestoreError,
        (Previewing, Back) => Browsing,
        (RestoreError, Back) => Browsing,
        // 其余：无合法转移（快照缺失 SelectNode{false}、Browsing→Restoring 直达、
        // RestoreError→Restored 等 forbidden）。
        _ => return None,
    };
    Some(next)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn fresh_db() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        db::migrate(&mut conn).unwrap();
        conn
    }

    fn doc() -> DocRef {
        DocRef {
            workspace_id: "ws1".into(),
            rel_path: "a.md".into(),
            doc_type: "md".into(),
        }
    }

    // covers: BR-WS-STATE-002
    // 快照往返：store 后 preview 取回内容；内容寻址去重（同内容只一份）；节点/快照缺失 guard 拒。
    #[test]
    fn snapshot_store_preview_and_guards() {
        let conn = fresh_db();
        conn.execute(
            "INSERT INTO workspaces (workspace_id, root_path, is_active, opened_at) VALUES ('ws1','/w',1,0)",
            [],
        )
        .unwrap();
        let d = doc();
        store_snapshot(&conn, "blob_1", b"version one").unwrap();
        store_snapshot(&conn, "blob_1", b"version one").unwrap(); // 去重幂等
        let n = append_node(&conn, &d, "apply", "blob_1", None).unwrap();
        // preview 取回内容
        assert_eq!(preview_node(&conn, &n.entry_id).unwrap(), b"version one");
        // 去重：只存一份
        let cnt: i64 = conn
            .query_row("SELECT COUNT(*) FROM timeline_snapshots", [], |r| r.get(0))
            .unwrap();
        assert_eq!(cnt, 1);
        // 节点不存在 → guard
        assert_eq!(preview_node(&conn, "nope").unwrap_err().code, "WS_NODE_NOT_FOUND");
        // 节点存在但快照缺失 → guard
        let orphan = append_node(&conn, &d, "apply", "missing_blob", None).unwrap();
        assert_eq!(preview_node(&conn, &orphan.entry_id).unwrap_err().code, "WS_SNAPSHOT_UNAVAILABLE");
    }

    // covers: BR-WS-DATA-002
    // 节点 version_seq 单调递增、不复用；list 按 seq 有序；同 (ws,rel,seq) 重复被 UNIQUE 拒。
    #[test]
    fn version_seq_monotonic_no_reuse() {
        let conn = fresh_db();
        // 需先有 workspace（外键）
        conn.execute(
            "INSERT INTO workspaces (workspace_id, root_path, is_active, opened_at) VALUES ('ws1','/w',1,0)",
            [],
        )
        .unwrap();
        let d = doc();
        let n1 = append_node(&conn, &d, "apply", "snap1", None).unwrap();
        let n2 = append_node(&conn, &d, "restore", "snap2", Some("回退")).unwrap();
        assert_eq!(n1.version_seq, 1);
        assert_eq!(n2.version_seq, 2);
        let list = list_timeline(&conn, &d).unwrap();
        assert_eq!(list.iter().map(|e| e.version_seq).collect::<Vec<_>>(), vec![1, 2]);

        // 手动插入复用 version_seq=1 → UNIQUE 拒绝（不可伪造历史连续性）
        let dup = conn.execute(
            "INSERT INTO timeline (entry_id,workspace_id,rel_path,version_seq,entry_kind,snapshot_ref,created_at) \
             VALUES ('e_dup','ws1','a.md',1,'apply','snapX',0)",
            [],
        );
        assert!(dup.is_err(), "version_seq 复用应被 UNIQUE 拒绝");
    }

    // covers: BR-WS-STATE-002
    // SM-WS-03 强制矩阵：正路 + 还原失败保持现状 + forbidden（快照缺失/未预览直达/失败态不达 Restored）。
    #[test]
    fn sm_ws03_restore_matrix() {
        use RestoreEvent::*;
        use RestoreState::*;
        // 正路
        assert_eq!(
            transition(Browsing, SelectNode { snapshot_available: true }),
            Some(Previewing)
        );
        assert_eq!(transition(Previewing, ConfirmRestore), Some(Restoring));
        assert_eq!(transition(Restoring, RestoreOk), Some(Restored));
        // 还原失败保持现状（进 RestoreError，不达 Restored）
        assert_eq!(transition(Restoring, RestoreFail), Some(RestoreError));
        assert_eq!(transition(RestoreError, Back), Some(Browsing));
        // forbidden
        assert_eq!(
            transition(Browsing, SelectNode { snapshot_available: false }),
            None,
            "快照缺失不得进入预览"
        );
        assert_eq!(transition(Browsing, ConfirmRestore), None, "未经预览不得直接还原");
        assert_eq!(transition(RestoreError, RestoreOk), None, "失败态不得跳为已还原");
        assert_eq!(transition(Restoring, Back), None);
    }
}
