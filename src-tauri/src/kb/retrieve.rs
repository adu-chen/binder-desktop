//! PI-KB-02 · 检索召回供给（本地化硬边界） · KB-RETRIEVE / BR-KB-DATA-002
//!
//! 承接 A-KB-M-T-01 §3：检索召回**全程本地执行**——索引构建与查询不得将库内容经网络上行（含禁用云端
//! embedding；本地化是硬边界，违反即产品边界变更须 APC 授权）；仅召回当前 workspace 的**存续**条目
//! （已删除不召回，无悬空）；召回供给含 source_ref（消费方可溯源）；无相关召回**供给空集**不注入噪声。
//! v1 选型为本地关键词匹配（断网可用）；向量/混合选型为实现期细化（KB-03，A 类，仍受无上行约束）。

use crate::types::KnowledgeInjection;
use rusqlite::Connection;

/**
 * @GOV
 * codes:    BR-KB-DATA-002-QUERY-KB-KB-RETRIEVE-001
 * type:     QUERY
 * chain:    KB-RETRIEVE
 * rules:    BR-KB-DATA-002
 * boundary: in=workspaceLocalQuery | out=localKnowledgeInjections(noUplinkScopedLiveEmptyWhenNoMatch)
 */
/// 本地检索召回（BR-KB-DATA-002）：纯本地匹配（无网络上行）；仅当前 workspace 的存续条目（已删除
/// 物理不存在自然不召回）；空查询或无匹配 → **空集**（不注入噪声）；命中供给含 source_ref 可溯源。
pub fn retrieve(
    conn: &Connection,
    workspace_id: &str,
    query: &str,
) -> rusqlite::Result<Vec<KnowledgeInjection>> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Ok(Vec::new()); // 空查询不注入
    }
    // 本地读取当前 workspace 存续条目后在本地按关键词匹配（无内容上行）。
    let mut stmt = conn.prepare(
        "SELECT content, source_ref, organization FROM knowledge_entries \
         WHERE workspace_id=?1 ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map([workspace_id], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, Option<String>>(2)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (content, source_ref, organization) = row?;
        if content.to_lowercase().contains(&q) {
            out.push(KnowledgeInjection {
                content,
                source_ref,
                organization,
            });
        }
    }
    Ok(out) // 无匹配 → 空集（不注入噪声）
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::kb::store;

    fn fresh() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        db::migrate(&mut conn).unwrap();
        conn
    }

    // covers: BR-KB-DATA-002
    // 本地检索：命中含 source_ref 可溯源；仅当前 workspace（隔离）；空查询/无匹配 → 空集不注入。
    #[test]
    fn local_retrieve_scoped_traceable_and_empty_on_no_match() {
        let conn = fresh();
        store::ingest(
            &conn,
            "ws1",
            "TipTap 编辑器集成笔记",
            "doc:a.md",
            Some("编辑"),
        )
        .unwrap();
        store::ingest(&conn, "ws1", "Tauri IPC 设计", "doc:b.md", None).unwrap();
        store::ingest(&conn, "ws2", "TipTap 其它 workspace", "doc:c.md", None).unwrap();

        let hits = retrieve(&conn, "ws1", "tiptap").unwrap();
        assert_eq!(hits.len(), 1); // 仅 ws1 命中（不跨 workspace）
        assert_eq!(hits[0].source_ref, "doc:a.md"); // 可溯源

        // 无匹配 → 空集（不注入噪声）
        assert!(retrieve(&conn, "ws1", "无关词xyz").unwrap().is_empty());
        // 空查询 → 空集
        assert!(retrieve(&conn, "ws1", "   ").unwrap().is_empty());
    }

    // covers: BR-KB-DATA-002
    // 删除即不召回：入库可召回，删除后同一查询不再召回（无悬空）。
    #[test]
    fn deleted_entries_are_not_recalled() {
        let conn = fresh();
        let id = store::ingest(&conn, "ws1", "可被删除的知识条目", "doc:x.md", None).unwrap();
        assert_eq!(retrieve(&conn, "ws1", "可被删除").unwrap().len(), 1);
        // 删除即生效
        assert!(store::delete_entry(&conn, "ws1", &id).unwrap());
        // 删除即不召回（存续条目过滤，无悬空）
        assert!(retrieve(&conn, "ws1", "可被删除").unwrap().is_empty());
    }
}
