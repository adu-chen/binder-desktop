//! PI-AST-01 · 记忆条目存储与 workspace 级隔离 · AST-MEMORY / BR-AST-DATA-001
//!
//! 承接 A-AST-M-T-01：记忆条目 workspace 级隔离——任一查询恒以 workspace_id 作用域，跨 workspace
//! 不可见；条目纯本地（memory_entries，PI-SYS-03），**不上云**（无网络出口，仅本地 sqlite）；提供管理
//! 面可感知的列举（active 供供给、含 revoked 供管理）。供给侧相关性排序归 PI-AST-03；写入/撤销归
//! PI-AST-02。索引与大规模条目分页为实现期细化（AST-01，A 类）。

use crate::types::MemoryInjection;
use rusqlite::Connection;

/// 管理面可见的记忆条目（含 content，供用户显式选择浏览；BR-SYS-DATA-007 / TERM-AST-002）。
#[derive(Debug, Clone)]
pub struct MemoryManageEntry {
    pub entry_id: String,
    pub memory_kind: String,
    pub content: String,
    pub status: String,
    pub origin_turn: String,
}

/**
 * @GOV
 * codes:    BR-AST-DATA-001-QUERY-AST-AST-MEMORY-004
 * type:     QUERY
 * chain:    AST-MEMORY
 * rules:    BR-AST-DATA-001
 * boundary: in=workspaceId | out=activeMemoryEntries(scopedNeverCrossWorkspaceNeverCloud)
 */
/// 列举当前 workspace 的 active 记忆条目（BR-AST-DATA-001）：查询**恒以 workspace_id 作用域**，
/// 跨 workspace 条目不可见（隔离）；仅取 active（revoked 不供给）；纯本地读取无网络出口（不上云）。
pub fn list_active(
    conn: &Connection,
    workspace_id: &str,
) -> rusqlite::Result<Vec<MemoryInjection>> {
    let mut stmt = conn.prepare(
        "SELECT entry_id, memory_kind, content, origin_turn FROM memory_entries \
         WHERE workspace_id=?1 AND status='active' ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map([workspace_id], |r| {
        Ok(MemoryInjection {
            memory_kind: r.get(1)?,
            content: r.get(2)?,
            origin_turn: r.get(3)?,
        })
    })?;
    rows.collect()
}

/**
 * @GOV
 * codes:    BR-AST-DATA-001-QUERY-AST-AST-MEMORY-002
 * type:     QUERY
 * chain:    AST-MEMORY
 * rules:    BR-AST-DATA-001
 * boundary: in=workspaceId | out=allWorkspaceEntriesWithStatus(manageableScoped)
 */
/// 管理面列举（BR-AST-DATA-001）：当前 workspace 全部条目含状态（active/revoked），供管理面可感知/
/// 可撤销；同样以 workspace_id 作用域隔离，不跨 workspace、不上云。返回 (entry_id, kind, status)。
pub fn list_for_management(
    conn: &Connection,
    workspace_id: &str,
) -> rusqlite::Result<Vec<(String, String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT entry_id, memory_kind, status FROM memory_entries \
         WHERE workspace_id=?1 ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map([workspace_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
    rows.collect()
}

/**
 * @GOV
 * codes:    BR-SYS-DATA-007-QUERY-AST-AST-MEMORY-003, BR-AST-DATA-001-QUERY-AST-AST-MEMORY-003
 * type:     QUERY
 * chain:    AST-MEMORY, REF-INJECT
 * rules:    BR-SYS-DATA-007, BR-AST-DATA-001
 * term_ref: TERM-AST-002
 * boundary: in=workspaceId | out=activeManageableMemoryEntriesWithContent(scopedExplicitSelectionDataOnlyNeverAutoInject)
 */
/// 显式记忆选择供给（BR-SYS-DATA-007 / TERM-AST-002）：当前 workspace 的 **active** 记忆条目含 content
/// 供浏览/选择（构造 MemorySelectionRef 快照进当前回合）。workspace 作用域隔离、纯本地不上云
/// （BR-AST-DATA-001）。仅显式选择进入回合，本查询不自动供给、不自动注入（1.0 关闭自动链路）。
pub fn list_manageable(
    conn: &Connection,
    workspace_id: &str,
) -> rusqlite::Result<Vec<MemoryManageEntry>> {
    let mut stmt = conn.prepare(
        "SELECT entry_id, memory_kind, content, status, origin_turn FROM memory_entries \
         WHERE workspace_id=?1 AND status='active' ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map([workspace_id], |r| {
        Ok(MemoryManageEntry {
            entry_id: r.get(0)?,
            memory_kind: r.get(1)?,
            content: r.get(2)?,
            status: r.get(3)?,
            origin_turn: r.get(4)?,
        })
    })?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::capture;
    use crate::db;

    fn fresh() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        db::migrate(&mut conn).unwrap();
        conn
    }

    // covers: BR-AST-DATA-001
    // workspace 级隔离：查询恒以 workspace_id 作用域，ws1 看不到 ws2 的条目（跨 workspace 不可见）。
    #[test]
    fn queries_are_workspace_scoped() {
        let conn = fresh();
        capture::capture_memory(&conn, "ws1", "fact", "ws1 的记忆", "t1").unwrap();
        capture::capture_memory(&conn, "ws2", "fact", "ws2 的记忆", "t1").unwrap();

        let a = list_active(&conn, "ws1").unwrap();
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].content, "ws1 的记忆"); // 不含 ws2
        assert_eq!(list_active(&conn, "ws2").unwrap().len(), 1);
        // 不存在的 workspace → 空集（不泄漏其它 workspace）
        assert!(list_active(&conn, "wsX").unwrap().is_empty());
    }

    // covers: BR-AST-DATA-001
    // active 供给只见 active；管理面可感知全部（含 revoked）；均限当前 workspace 作用域。
    #[test]
    fn active_excludes_revoked_management_sees_all() {
        let conn = fresh();
        let id1 = capture::capture_memory(&conn, "ws1", "fact", "保留", "t1").unwrap();
        let id2 = capture::capture_memory(&conn, "ws1", "fact", "将撤销", "t2").unwrap();
        capture::revoke_memory(&conn, &id2).unwrap();

        let _ = id1;
        // 供给侧只见 active（撤销项不供给）
        let active = list_active(&conn, "ws1").unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].content, "保留");
        // 管理面可感知全部（active + revoked）
        let mgmt = list_for_management(&conn, "ws1").unwrap();
        assert_eq!(mgmt.len(), 2);
        let revoked: Vec<_> = mgmt.iter().filter(|(_, _, s)| s == "revoked").collect();
        assert_eq!(revoked.len(), 1);
        assert_eq!(revoked[0].0, id2);
    }

    // covers: BR-SYS-DATA-007
    // 显式记忆选择供给（TERM-AST-002）：active 条目含 content 供选择；revoked/跨 workspace 不可见。
    #[test]
    fn manageable_listing_serves_explicit_selection() {
        let conn = fresh();
        let id1 = capture::capture_memory(&conn, "ws1", "fact", "选择A", "t1").unwrap();
        let id2 = capture::capture_memory(&conn, "ws1", "preference", "选择B", "t2").unwrap();
        capture::capture_memory(&conn, "ws2", "fact", "他ws", "t1").unwrap();
        capture::revoke_memory(&conn, &id2).unwrap();

        let list = list_manageable(&conn, "ws1").unwrap();
        // revoked（id2）与跨 workspace（ws2）不可见；active 含 entry_id + content 供构造 MemorySelectionRef。
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].entry_id, id1);
        assert_eq!(list[0].memory_kind, "fact");
        assert_eq!(list[0].content, "选择A");
        assert_eq!(list[0].status, "active");
        // 不存在 workspace → 空集（作用域隔离）
        assert!(list_manageable(&conn, "wsX").unwrap().is_empty());
    }
}
