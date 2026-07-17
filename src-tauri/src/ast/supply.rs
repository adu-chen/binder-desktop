//! PI-AST-03 · 记忆注入供给 · AST-MEMORY / BR-AST-DATA-002
//!
//! 承接 A-AST-M-T-01：记忆供给**仅当前 workspace 的 active 条目**（隔离 + revoked 不供给，复用 PI-AST-01
//! store::list_active）；按类型分级优先级筛选（preference/style 优先，session_context 最低）；载荷含
//! memory_kind + origin_turn（消费方可溯源）；**无相关记忆供给空集，不注入噪声**；载荷用 MemoryInjection
//! （不复用 RefInjection——其 target 语义不符）。相关性筛选机制延实现期（AST-04，A 类）；预算与裁剪归 AG
//! （BR-AG-DATA-001，AST 不裁剪）。

use crate::ast::store;
use crate::types::MemoryInjection;
use rusqlite::Connection;

/// 类型分级优先级（数值越小越优先）：preference/style 优先，session_context 最低。
fn kind_priority(kind: &str) -> u8 {
    match kind {
        "preference" | "style" => 0,
        "session_context" => 2,
        _ => 1,
    }
}

/**
 * @GOV
 * codes:    BR-AST-DATA-002-QUERY-AST-AST-MEMORY-001
 * type:     QUERY
 * chain:    AST-MEMORY
 * rules:    BR-AST-DATA-002
 * boundary: in=workspaceId | out=MemoryInjections(activeScopedTypeRankedTraceableEmptyWhenNone)
 */
/// 记忆供给（BR-AST-DATA-002）：仅当前 workspace 的 active 条目（隔离、revoked 不供给）；按类型分级
/// 优先级稳定排序（preference/style > 其它 > session_context）；载荷含 memory_kind + origin_turn 可溯源；
/// **无记忆 → 空集**（不注入噪声）。不做预算裁剪（归 AG）。
pub fn supply(conn: &Connection, workspace_id: &str) -> rusqlite::Result<Vec<MemoryInjection>> {
    let mut items = store::list_active(conn, workspace_id)?; // active + workspace 隔离
                                                             // 稳定排序：同优先级保持入库序（确定性）。
    items.sort_by_key(|m| kind_priority(&m.memory_kind));
    Ok(items) // 空集即不注入
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

    // covers: BR-AST-DATA-002
    // 仅 active + workspace 隔离；revoked 不供给；分级优先级（preference 先于 session_context）；可溯源。
    #[test]
    fn supplies_active_scoped_ranked_and_traceable() {
        let conn = fresh();
        capture::capture_memory(&conn, "ws1", "session_context", "上下文", "t1").unwrap();
        capture::capture_memory(&conn, "ws1", "preference", "偏好简洁", "t2").unwrap();
        let revoke_id = capture::capture_memory(&conn, "ws1", "fact", "将撤销", "t3").unwrap();
        capture::capture_memory(&conn, "ws2", "preference", "他 workspace", "t4").unwrap();
        capture::revoke_memory(&conn, &revoke_id).unwrap();

        let out = supply(&conn, "ws1").unwrap();
        // ws1 active：preference + session_context（revoked 的 fact 不供给；ws2 不含）
        assert_eq!(out.len(), 2);
        // 分级优先：preference 在 session_context 之前
        assert_eq!(out[0].memory_kind, "preference");
        assert_eq!(out[1].memory_kind, "session_context");
        // 可溯源
        assert_eq!(out[0].origin_turn, "t2");
    }

    // covers: BR-AST-DATA-002
    // 无记忆 → 空集（不注入噪声）。
    #[test]
    fn empty_when_no_memory() {
        let conn = fresh();
        assert!(supply(&conn, "ws_empty").unwrap().is_empty());
    }
}
