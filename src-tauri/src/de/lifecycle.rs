//! PI-DE-03（实现期续接）· diff 生命周期编排 · DE-DIFF / BR-DE-STATE-002 / BR-DE-DATA-003
//!
//! 在真实 pending_diffs 控制态上编排过期与冲突重检——把 expiry.rs 的纯判定（check_expiry/recheck_overlap）
//! 接到实际数据：`list_pending` 取待裁决 diff，`sweep_expiry` 据「now-created_at 超时 + 外部变更检查」逐条
//! 判定过期（保守：检测不可得不误杀，BR-DE-STATE-002），`sweep_conflicts` 据先序落盘块重叠 + 目标哈希变化
//! 判定冲突过期（不覆盖新内容，BR-DE-DATA-003），`expire_diffs` 经 WS `discard_diff(expired)` 落终态（不写
//! 文件、不记节点）。外部变更检查由调用方经 `ws.check_external_change` 提供；超时时长与触发时机为调度（A 类）。

use crate::de::expiry::{check_expiry, recheck_overlap, ExpiryDecision, ExternalCheck};
use crate::workspace::diff_apply::discard_diff;
use rusqlite::Connection;

/// 待裁决 diff 行（生命周期编排所需字段）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingDiffRow {
    pub diff_id: String,
    pub rel_path: String,
    pub created_at: i64,
}

/// 冲突重检输入（先序落盘后，与已落盘块重叠的待裁决 diff）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictInput {
    pub diff_id: String,
    pub blocks: Vec<String>,
    pub before_hash: u64,
    pub current_hash: u64,
}

/**
 * @GOV
 * codes:    BR-DE-STATE-002-QUERY-DE-DE-DIFF-001
 * type:     QUERY
 * chain:    DE-DIFF
 * rules:    BR-DE-STATE-002
 * boundary: in=workspaceId | out=pendingDiffRows(statusPendingScoped)
 */
/// 列举当前 workspace 待裁决（status='pending'）diff（BR-DE-STATE-002）：供生命周期扫描；按创建时序。
pub fn list_pending(conn: &Connection, workspace_id: &str) -> rusqlite::Result<Vec<PendingDiffRow>> {
    let mut stmt = conn.prepare(
        "SELECT diff_id, rel_path, created_at FROM pending_diffs \
         WHERE workspace_id=?1 AND status='pending' ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map([workspace_id], |r| {
        Ok(PendingDiffRow { diff_id: r.get(0)?, rel_path: r.get(1)?, created_at: r.get(2)? })
    })?;
    rows.collect()
}

/**
 * @GOV
 * codes:    BR-DE-STATE-002-GUARD-DE-DE-DIFF-002
 * type:     GUARD
 * chain:    DE-DIFF
 * rules:    BR-DE-STATE-002
 * boundary: in=pendingRowsAndExternalChecks | out=expiredDiffIds(timeoutOrExternalConservative)
 */
/// 过期扫描（BR-DE-STATE-002）：逐条以 (now-created_at) 超时 + 目标文档外部变更检查判过期；**检测不可得
/// 不误杀**（保守保持 Pending），但超时独立生效。external_of 按 rel_path 提供外部变更结果。返回应过期的
/// diff_id 集（顺序与输入一致）。
pub fn sweep_expiry(
    rows: &[PendingDiffRow],
    now_ms: i64,
    timeout_ms: i64,
    external_of: impl Fn(&str) -> ExternalCheck,
) -> Vec<String> {
    rows.iter()
        .filter_map(|r| {
            let elapsed = now_ms.saturating_sub(r.created_at);
            match check_expiry(elapsed, timeout_ms, external_of(&r.rel_path)) {
                ExpiryDecision::Expire(_) => Some(r.diff_id.clone()),
                ExpiryDecision::KeepPending => None,
            }
        })
        .collect()
}

/**
 * @GOV
 * codes:    BR-DE-DATA-003-GUARD-DE-DE-DIFF-002
 * type:     GUARD
 * chain:    DE-DIFF
 * rules:    BR-DE-DATA-003
 * boundary: in=appliedBlocksAndPendings | out=conflictedDiffIds(overlapStaleNoStaleOverwrite)
 */
/// 冲突重检扫描（BR-DE-DATA-003）：先序 diff 落盘后，与其目标块重叠且 before_snapshot 哈希不再匹配当前
/// （内容已变）的待裁决 diff 判过期——不以陈旧 after_content 覆盖新内容；不重叠/未变保持。返回应过期 diff_id。
pub fn sweep_conflicts(applied_blocks: &[String], pendings: &[ConflictInput]) -> Vec<String> {
    pendings
        .iter()
        .filter_map(|p| match recheck_overlap(applied_blocks, &p.blocks, p.before_hash, p.current_hash) {
            ExpiryDecision::Expire(_) => Some(p.diff_id.clone()),
            ExpiryDecision::KeepPending => None,
        })
        .collect()
}

/**
 * @GOV
 * codes:    BR-DE-STATE-002-EFFECT-DE-DE-DIFF-001
 * type:     EFFECT
 * chain:    DE-DIFF
 * rules:    BR-DE-STATE-002
 * boundary: in=expiredDiffIds | out=discardedAsExpired(noFileWriteNoNode)
 */
/// 落过期终态（BR-DE-STATE-002）：对扫描判定过期的 diff，经 WS `discard_diff(expired)` 置终态——
/// **不写文件、不记时间轴节点**（落盘语义归 WS）。返回成功落终态的条数。
pub fn expire_diffs(conn: &Connection, diff_ids: &[String]) -> rusqlite::Result<usize> {
    let mut n = 0;
    for id in diff_ids {
        discard_diff(conn, id, "expired")?;
        n += 1;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::types::{DiffStatus, DocRef, PendingDiff};
    use crate::workspace::diff_apply::persist_diff;

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

    fn seed_diff(conn: &Connection, id: &str, rel: &str, created_at: i64) {
        let d = PendingDiff {
            diff_id: id.into(),
            target: DocRef { workspace_id: "ws1".into(), rel_path: rel.into(), doc_type: "md".into() },
            origin_module: "AG".into(),
            status: DiffStatus::Pending,
            created_at,
            payload_ref: "p".into(),
        };
        persist_diff(conn, &d).unwrap();
    }

    fn status_of(conn: &Connection, id: &str) -> String {
        conn.query_row("SELECT status FROM pending_diffs WHERE diff_id=?1", [id], |r| r.get(0)).unwrap()
    }

    // covers: BR-DE-STATE-002
    // 扫描过期：超时→过期；外部已变→过期；检测不可得且未超时→保持（不误杀）；只过期该过期的。
    #[test]
    fn sweep_expiry_timeout_external_and_conservative() {
        let rows = vec![
            PendingDiffRow { diff_id: "d_old".into(), rel_path: "a.md".into(), created_at: 0 },
            PendingDiffRow { diff_id: "d_new".into(), rel_path: "b.md".into(), created_at: 9_000 },
            PendingDiffRow { diff_id: "d_undet".into(), rel_path: "c.md".into(), created_at: 9_000 },
            PendingDiffRow { diff_id: "d_changed".into(), rel_path: "d.md".into(), created_at: 9_000 },
        ];
        // now=10_000, timeout=5_000：d_old 超时；其余未超时。外部：d_changed 已变；c.md 检测不可得。
        let expired = sweep_expiry(&rows, 10_000, 5_000, |rel| match rel {
            "d.md" => ExternalCheck::Changed,
            "c.md" => ExternalCheck::Undetectable,
            _ => ExternalCheck::Unchanged,
        });
        assert!(expired.contains(&"d_old".to_string()), "超时应过期");
        assert!(expired.contains(&"d_changed".to_string()), "外部已变应过期");
        assert!(!expired.contains(&"d_undet".to_string()), "检测不可得+未超时不误杀");
        assert!(!expired.contains(&"d_new".to_string()), "未超时未变保持");
    }

    // covers: BR-DE-DATA-003
    // 冲突重检：重叠且内容已变→过期；不重叠或未变→保持。
    #[test]
    fn sweep_conflicts_overlap_stale_only() {
        let applied = vec!["b1".to_string(), "b2".to_string()];
        let pendings = vec![
            ConflictInput { diff_id: "stale".into(), blocks: vec!["b2".into()], before_hash: 1, current_hash: 2 },
            ConflictInput { diff_id: "unchanged".into(), blocks: vec!["b2".into()], before_hash: 1, current_hash: 1 },
            ConflictInput { diff_id: "disjoint".into(), blocks: vec!["b9".into()], before_hash: 1, current_hash: 2 },
        ];
        let expired = sweep_conflicts(&applied, &pendings);
        assert_eq!(expired, vec!["stale".to_string()]); // 仅重叠且内容变者
    }

    // covers: BR-DE-STATE-002
    // 端到端：list_pending → sweep → expire_diffs 落终态；过期者 status=expired，其余仍 pending、文件不动。
    #[test]
    fn list_sweep_expire_roundtrip_on_db() {
        let conn = fresh();
        seed_diff(&conn, "d_old", "a.md", 0);
        seed_diff(&conn, "d_new", "b.md", 9_000);
        let rows = list_pending(&conn, "ws1").unwrap();
        assert_eq!(rows.len(), 2);
        let expired = sweep_expiry(&rows, 10_000, 5_000, |_| ExternalCheck::Unchanged);
        let n = expire_diffs(&conn, &expired).unwrap();
        assert_eq!(n, 1);
        assert_eq!(status_of(&conn, "d_old"), "expired"); // 过期落终态
        assert_eq!(status_of(&conn, "d_new"), "pending"); // 未过期保持
        // 过期后不再出现在 pending 列举
        assert_eq!(list_pending(&conn, "ws1").unwrap().len(), 1);
    }
}
