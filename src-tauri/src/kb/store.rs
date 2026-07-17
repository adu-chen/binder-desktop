//! PI-KB-01 · 知识条目存储、组织与主动入库 · KB-RETRIEVE / BR-KB-DATA-001
//!
//! 承接 A-KB-M-T-01 §2：知识条目 workspace 级隔离（knowledge_entries，PI-SYS-03；跨 workspace 不共享）；
//! 入库为**用户主动操作——AI 不静默建库**（每次入库须显式来源 source_ref，否则拒绝）；每条可溯源到
//! 入库来源；条目可见可管理，**删除即生效**（物理删除，配合 PI-KB-02「删除即不召回」）；库内容不上行
//! 云端（无网络出口）。组织结构（organization）的层级模型为实现期细化（KB-01，A 类）。

use crate::ipc::IpcError;
use rusqlite::Connection;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

static SEQ: AtomicU64 = AtomicU64::new(0);

fn gen_entry_id() -> String {
    format!(
        "kb_{}_{}",
        now_millis(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    )
}

/**
 * @GOV
 * codes:    BR-KB-DATA-001-EFFECT-KB-KB-RETRIEVE-001
 * type:     EFFECT
 * chain:    KB-RETRIEVE
 * rules:    BR-KB-DATA-001
 * boundary: in=KnowledgeIngestRequest | out=traceableEntry(rejectedWhenNoSourceRef)
 */
/// 主动入库（BR-KB-DATA-001）：落 knowledge_entries（status 隐含存续）。**AI 不静默建库**——
/// source_ref（入库来源）为空一律拒绝（每条须可溯源）；content 为空亦拒绝。落库于 workspace 控制态
/// （隔离），不上行云端。返回 entry_id。
pub fn ingest(
    conn: &Connection,
    workspace_id: &str,
    content: &str,
    source_ref: &str,
    organization: Option<&str>,
) -> Result<String, IpcError> {
    if source_ref.trim().is_empty() {
        return Err(IpcError::new(
            "KB_INGEST_REJECTED",
            "入库被拒：缺来源 source_ref（AI 不静默建库，每条须可溯源）",
            false,
            false,
        ));
    }
    if content.trim().is_empty() {
        return Err(IpcError::new(
            "KB_INGEST_REJECTED",
            "入库被拒：内容为空",
            false,
            false,
        ));
    }
    let entry_id = gen_entry_id();
    let now = now_millis();
    conn.execute(
        "INSERT INTO knowledge_entries \
         (entry_id, workspace_id, content, organization, source_ref, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        (
            &entry_id,
            workspace_id,
            content,
            organization,
            source_ref,
            now,
        ),
    )
    .map_err(|e| IpcError::new("KB_DB_ERROR", &format!("知识入库失败: {e}"), true, false))?;
    Ok(entry_id)
}

/**
 * @GOV
 * codes:    BR-KB-DATA-001-QUERY-KB-KB-RETRIEVE-001
 * type:     QUERY
 * chain:    KB-RETRIEVE
 * rules:    BR-KB-DATA-001
 * boundary: in=workspaceId | out=manageableEntries(scopedNeverCrossWorkspace)
 */
/// 管理面列举（BR-KB-DATA-001）：当前 workspace 全部条目（可见可管理），查询**恒以 workspace_id
/// 作用域**，跨 workspace 不可见。返回 (entry_id, source_ref, organization)。
pub fn list_for_management(
    conn: &Connection,
    workspace_id: &str,
) -> rusqlite::Result<Vec<(String, String, Option<String>)>> {
    let mut stmt = conn.prepare(
        "SELECT entry_id, source_ref, organization FROM knowledge_entries \
         WHERE workspace_id=?1 ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map([workspace_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
    rows.collect()
}

/**
 * @GOV
 * codes:    BR-KB-DATA-001-EFFECT-KB-KB-RETRIEVE-002
 * type:     EFFECT
 * chain:    KB-RETRIEVE
 * rules:    BR-KB-DATA-001
 * boundary: in=entryIdScopedToWorkspace | out=removedEntry(deleteEffectiveImmediately)
 */
/// 删除（BR-KB-DATA-001）：物理删除条目，**删除即生效**（配合 PI-KB-02 检索即不再召回）。以
/// (entry_id, workspace_id) 双键作用域，不跨 workspace 误删。返回是否命中（不存在 → false，幂等）。
pub fn delete_entry(
    conn: &Connection,
    workspace_id: &str,
    entry_id: &str,
) -> rusqlite::Result<bool> {
    let n = conn.execute(
        "DELETE FROM knowledge_entries WHERE entry_id=?1 AND workspace_id=?2",
        (entry_id, workspace_id),
    )?;
    Ok(n == 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn fresh() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        db::migrate(&mut conn).unwrap();
        conn
    }

    // covers: BR-KB-DATA-001
    // 主动入库可溯源：有 source_ref 落库；无 source_ref/空内容拒绝（AI 不静默建库），不残留。
    #[test]
    fn ingest_requires_source_ref_no_silent_build() {
        let conn = fresh();
        let id = ingest(&conn, "ws1", "知识A", "doc:notes.md#1", Some("章节1")).unwrap();
        let src: String = conn
            .query_row(
                "SELECT source_ref FROM knowledge_entries WHERE entry_id=?1",
                [&id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(src, "doc:notes.md#1"); // 可溯源

        // 无来源 / 空内容 → 拒绝
        assert_eq!(
            ingest(&conn, "ws1", "x", "  ", None).unwrap_err().code,
            "KB_INGEST_REJECTED"
        );
        assert_eq!(
            ingest(&conn, "ws1", "  ", "src", None).unwrap_err().code,
            "KB_INGEST_REJECTED"
        );
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM knowledge_entries", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1, "拒绝项不得残留（不静默建库）");
    }

    // covers: BR-KB-DATA-001
    // workspace 隔离：管理面列举恒以 workspace_id 作用域，ws1 看不到 ws2。
    #[test]
    fn management_listing_is_workspace_scoped() {
        let conn = fresh();
        ingest(&conn, "ws1", "ws1 知识", "s1", None).unwrap();
        ingest(&conn, "ws2", "ws2 知识", "s2", None).unwrap();
        assert_eq!(list_for_management(&conn, "ws1").unwrap().len(), 1);
        assert_eq!(list_for_management(&conn, "ws2").unwrap().len(), 1);
        assert!(list_for_management(&conn, "wsX").unwrap().is_empty());
    }

    // covers: BR-KB-DATA-001
    // 删除即生效：物理删除后管理面不再可见；跨 workspace 不误删；不存在幂等 false。
    #[test]
    fn delete_is_immediate_and_workspace_scoped() {
        let conn = fresh();
        let id = ingest(&conn, "ws1", "将删除", "s1", None).unwrap();
        // 跨 workspace 不误删
        assert!(!delete_entry(&conn, "ws2", &id).unwrap());
        assert_eq!(list_for_management(&conn, "ws1").unwrap().len(), 1);
        // 正确 workspace 删除即生效
        assert!(delete_entry(&conn, "ws1", &id).unwrap());
        assert!(list_for_management(&conn, "ws1").unwrap().is_empty());
        // 不存在幂等
        assert!(!delete_entry(&conn, "ws1", &id).unwrap());
    }
}
