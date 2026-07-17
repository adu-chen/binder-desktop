//! PI-WS-03 · pending_diffs 持久化与裁决后落盘 · WS-DIFF-APPLY / BR-WS-STATE-001 / SM-WS-02
//!
//! 承接 A-WS-M-T-01 §5 / A-WS-M-S-01 SM-WS-02：WS 持久化 pending_diff（重启可恢复），仅在 DE 终态机
//! `accepted` 信号触发时落盘——写入目标文档并记恰好一个时间轴节点；`rejected`/`expired` 不写文件；
//! 写盘失败不记节点、控制态可重试且原文件不被破坏。diff 终态转移语义归 DE（本模块只执行 WS 侧落盘）。
//! accepted 信号来自 DE（X-INTER-001），covers 中以事件模拟。

use super::{save, timeline};
use crate::ipc::IpcError;
use crate::types::{DocRef, PendingDiff};
use rusqlite::{Connection, OptionalExtension};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// SM-WS-02 状态（A-WS-M-S-01 §3 diff 落盘机）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffApplyState {
    AwaitingDecision,
    Applying,
    Applied,
    Discarded,
    ApplyError,
}

/// SM-WS-02 事件。accepted/rejected/expired 为 DE 终态机裁决信号；applyOk/applyFail 为落盘结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffApplyEvent {
    Accepted,
    Rejected,
    Expired,
    ApplyOk,
    ApplyFail,
    Retry,
}

/**
 * @GOV
 * codes:    BR-WS-STATE-001-RB-WS-WS-DIFF-APPLY-001
 * type:     RB
 * chain:    WS-DIFF-APPLY
 * rules:    BR-WS-STATE-001
 * boundary: in=DiffApplyEvent | out=DiffApplyState(noneWhenForbidden,acceptedOnlyToApplying)
 */
/// SM-WS-02 转移引擎（BR-WS-STATE-001）：仅 accepted→Applying→applyOk→Applied；rejected/expired→
/// Discarded（不写文件）；applyFail→ApplyError（可 retry）。返回 `None` = 无合法转移（含 forbidden：
/// 未经 Applying 不得 Applied；rejected/expired 后不得绕过裁决再落盘）。
pub fn transition(state: DiffApplyState, event: DiffApplyEvent) -> Option<DiffApplyState> {
    use DiffApplyEvent::*;
    use DiffApplyState::*;
    let next = match (state, event) {
        (AwaitingDecision, Accepted) => Applying,
        (AwaitingDecision, Rejected) => Discarded,
        (AwaitingDecision, Expired) => Discarded,
        (Applying, ApplyOk) => Applied,
        (Applying, ApplyFail) => ApplyError,
        (ApplyError, Retry) => Applying,
        _ => return None,
    };
    Some(next)
}

/// SM-WS-02 终态判定：Applied/Discarded 为终态；ApplyError 可 Retry（非终态）。
pub fn is_terminal(state: DiffApplyState) -> bool {
    matches!(state, DiffApplyState::Applied | DiffApplyState::Discarded)
}

/**
 * @GOV
 * codes:    BR-WS-STATE-001-EFFECT-WS-WS-DIFF-APPLY-001
 * type:     EFFECT
 * chain:    WS-DIFF-APPLY
 * rules:    BR-WS-STATE-001
 * boundary: in=PendingDiff | out=persistedPendingDiff(restartRecoverable)
 */
/// 持久化 pending_diff（BR-WS-STATE-001：未裁决 diff 入库，重启可恢复）。
pub fn persist_diff(conn: &Connection, diff: &PendingDiff) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO pending_diffs \
         (diff_id, workspace_id, rel_path, origin_module, status, payload_ref, created_at) \
         VALUES (?1, ?2, ?3, ?4, 'pending', ?5, ?6)",
        (
            &diff.diff_id,
            &diff.target.workspace_id,
            &diff.target.rel_path,
            &diff.origin_module,
            &diff.payload_ref,
            diff.created_at,
        ),
    )?;
    Ok(())
}

/**
 * @GOV
 * codes:    BR-WS-STATE-001-EFFECT-WS-WS-DIFF-APPLY-002
 * type:     EFFECT
 * chain:    WS-DIFF-APPLY
 * rules:    BR-WS-STATE-001
 * boundary: in=acceptedDiffContent | out=oneTimelineNode(writtenAtomicNoHalfNodeOnFailure)
 */
/// accepted 落盘（BR-WS-STATE-001）：原子写入目标文档后记**恰好一个**时间轴节点并置 status=accepted。
/// 写盘失败先于记节点返回错误——不留半节点（节点仅在 save 成功后写）。仅应在 SM-WS-02 Applying 态调用。
pub fn apply_diff(
    conn: &Connection,
    diff_id: &str,
    doc: &DocRef,
    abs_path: &Path,
    after_content: &[u8],
    loaded_baseline: Option<u64>,
) -> Result<crate::types::TimelineEntry, IpcError> {
    if let Some(expected) = loaded_baseline {
        let current = std::fs::read(abs_path).map_err(|e| {
            IpcError::new(
                "WS_READ_FAILED",
                &format!("apply 前读取当前磁盘内容失败: {e}"),
                true,
                false,
            )
        })?;
        if save::content_hash(&current) != expected {
            return Err(IpcError::new(
                "WS_SAVE_CONFLICT",
                "磁盘内容已在 diff 生成后变化，请重新校验后再应用",
                false,
                false,
            ));
        }
    }
    // 1) 原子落盘（失败则不记节点、原文件保持，BR-WS-PERSIST-001）。
    let saved = save::save_atomic(abs_path, after_content)?;
    // 2) save 成功后存全量快照（内容寻址去重，解除 WS-03）+ 记恰好一个节点指向之。
    let snapshot_ref = format!("blob_{}", saved.baseline);
    timeline::store_snapshot(conn, &snapshot_ref, after_content)
        .map_err(|e| IpcError::new("WS_DB_ERROR", &format!("存快照失败: {e}"), true, false))?;
    let entry = timeline::append_node(conn, doc, "apply", &snapshot_ref, None).map_err(|e| {
        IpcError::new(
            "WS_DB_ERROR",
            &format!("记时间轴节点失败: {e}"),
            true,
            false,
        )
    })?;
    // 3) 置 diff 终态 accepted。
    conn.execute(
        "UPDATE pending_diffs SET status = 'accepted', decided_at = ?2 WHERE diff_id = ?1",
        (diff_id, now_millis()),
    )
    .map_err(|e| {
        IpcError::new(
            "WS_DB_ERROR",
            &format!("更新 diff 状态失败: {e}"),
            true,
            false,
        )
    })?;
    Ok(entry)
}

/**
 * @GOV
 * codes:    BR-WS-STATE-001-EFFECT-WS-WS-DIFF-APPLY-003
 * type:     EFFECT
 * chain:    WS-DIFF-APPLY
 * rules:    BR-WS-STATE-001
 * boundary: in=diffIdAndTerminal | out=discardedDiff(noFileWriteNoNode)
 */
/// rejected/expired 丢弃（BR-WS-STATE-001）：仅置 diff 终态，不写文件、不记时间轴节点。
pub fn discard_diff(conn: &Connection, diff_id: &str, terminal: &str) -> rusqlite::Result<()> {
    debug_assert!(terminal == "rejected" || terminal == "expired");
    conn.execute(
        "UPDATE pending_diffs SET status = ?2, decided_at = ?3 WHERE diff_id = ?1",
        (diff_id, terminal, now_millis()),
    )?;
    Ok(())
}

/// DB status 文本 → DiffStatus 枚举（与 A-SYS-I-P-01 单一来源对齐；未知按 pending 保守）。
pub(crate) fn parse_status(s: &str) -> crate::types::DiffStatus {
    use crate::types::DiffStatus::*;
    match s {
        "accepted" => Accepted,
        "rejected" => Rejected,
        "expired" => Expired,
        _ => Pending,
    }
}

/// 读取 diff 当前持久态（供 SM-DE-01 转移守护取当前态，A-CORE-X-L-04 §5.4 Task 2-B）；
/// diff 不存在回统一错误形状（不静默放行）。
pub fn current_status(
    conn: &Connection,
    diff_id: &str,
) -> Result<crate::types::DiffStatus, IpcError> {
    let s: Option<String> = conn
        .query_row(
            "SELECT status FROM pending_diffs WHERE diff_id=?1",
            [diff_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| {
            IpcError::new(
                "WS_DB_ERROR",
                &format!("diff 状态查询失败: {e}"),
                true,
                false,
            )
        })?;
    let s = s.ok_or_else(|| {
        IpcError::new(
            "DE_DIFF_NOT_FOUND",
            &format!("diff 不存在: {diff_id}"),
            false,
            false,
        )
    })?;
    Ok(parse_status(&s))
}

/// 由 rel_path 扩展名推断 doc_type（DocRef 需要；与 listing 一致小写无点）。
fn doc_type_of(rel_path: &str) -> String {
    rel_path
        .rsplit('.')
        .next()
        .filter(|ext| *ext != rel_path && !ext.is_empty())
        .map(|ext| ext.to_ascii_lowercase())
        .unwrap_or_else(|| "bin".to_string())
}

/**
 * @GOV
 * codes:    BR-WS-STATE-001-QUERY-WS-WS-DIFF-APPLY-001
 * type:     QUERY
 * chain:    WS-DIFF-APPLY
 * rules:    BR-WS-STATE-001
 * boundary: in=workspaceIdAndStatusFilter | out=pendingDiffList(restartRecoverable)
 */
/// 列举 workspace 的 pending_diff（BR-WS-STATE-001 重启恢复入口）：按 workspace_id 过滤，可选 status
/// 过滤（None=全部）；按 created_at 升序，使重启后裁决顺序稳定。
pub fn list_pending_diffs(
    conn: &Connection,
    workspace_id: &str,
    status_filter: Option<&str>,
) -> rusqlite::Result<Vec<PendingDiff>> {
    let mut stmt = conn.prepare(
        "SELECT diff_id, workspace_id, rel_path, origin_module, status, payload_ref, created_at \
         FROM pending_diffs WHERE workspace_id = ?1 AND (?2 IS NULL OR status = ?2) \
         ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map((workspace_id, status_filter), |r| {
        let rel_path: String = r.get(2)?;
        let status: String = r.get(4)?;
        Ok(PendingDiff {
            diff_id: r.get(0)?,
            target: DocRef {
                workspace_id: r.get(1)?,
                doc_type: doc_type_of(&rel_path),
                rel_path,
            },
            origin_module: r.get(3)?,
            status: parse_status(&status),
            payload_ref: r.get(5)?,
            created_at: r.get(6)?,
        })
    })?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
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

    fn sample_diff(id: &str) -> PendingDiff {
        PendingDiff {
            diff_id: id.into(),
            target: DocRef {
                workspace_id: "ws1".into(),
                rel_path: "a.md".into(),
                doc_type: "md".into(),
            },
            origin_module: "DE".into(),
            status: DiffStatus::Pending,
            created_at: 0,
            payload_ref: "p1".into(),
        }
    }

    fn status_of(conn: &Connection, id: &str) -> String {
        conn.query_row(
            "SELECT status FROM pending_diffs WHERE diff_id=?1",
            [id],
            |r| r.get(0),
        )
        .unwrap()
    }

    // covers: BR-WS-STATE-001
    // SM-WS-02 强制矩阵：accepted→Applying→applyOk→Applied；rejected/expired→Discarded；
    // applyFail→ApplyError→retry；forbidden（未经 Applying 不得 Applied、丢弃后不得再落盘）。
    #[test]
    fn sm_ws02_matrix() {
        use DiffApplyEvent::*;
        use DiffApplyState::*;
        assert_eq!(transition(AwaitingDecision, Accepted), Some(Applying));
        assert_eq!(transition(Applying, ApplyOk), Some(Applied));
        assert_eq!(transition(AwaitingDecision, Rejected), Some(Discarded));
        assert_eq!(transition(AwaitingDecision, Expired), Some(Discarded));
        assert_eq!(transition(Applying, ApplyFail), Some(ApplyError));
        assert_eq!(transition(ApplyError, Retry), Some(Applying));
        // forbidden
        assert_eq!(
            transition(AwaitingDecision, ApplyOk),
            None,
            "未经 Applying 不得 Applied"
        );
        assert_eq!(
            transition(Discarded, Accepted),
            None,
            "丢弃后不得绕过裁决再落盘"
        );
        assert_eq!(transition(Applied, ApplyOk), None);
    }

    // covers: BR-WS-STATE-001
    // current_status（A-CORE-X-L-04 Task 2-B 集成入口）：读回当前持久态供 SM-DE-01 守护；
    // 终态被裁决后正确反映（accepted/rejected）；diff 缺失回 DE_DIFF_NOT_FOUND（不静默放行）。
    #[test]
    fn current_status_reads_persisted_state() {
        let conn = fresh();
        persist_diff(&conn, &sample_diff("d1")).unwrap();
        assert_eq!(current_status(&conn, "d1").unwrap(), DiffStatus::Pending);
        discard_diff(&conn, "d1", "rejected").unwrap();
        assert_eq!(current_status(&conn, "d1").unwrap(), DiffStatus::Rejected);
        // 缺失 diff：错误形状（处理器据此拒绝，不静默落盘）
        let err = current_status(&conn, "missing").unwrap_err();
        assert_eq!(err.code, "DE_DIFF_NOT_FOUND");
    }

    // covers: BR-WS-STATE-001
    // list_pending_diffs：按 workspace 过滤、created_at 升序、可选 status 过滤；重启恢复入口。
    #[test]
    fn list_pending_diffs_filters_and_orders() {
        let conn = fresh();
        let mut d1 = sample_diff("d1");
        d1.created_at = 200;
        let mut d2 = sample_diff("d2");
        d2.created_at = 100;
        persist_diff(&conn, &d1).unwrap();
        persist_diff(&conn, &d2).unwrap();
        discard_diff(&conn, "d1", "rejected").unwrap();

        // 全部：按 created_at 升序（d2 先于 d1）。
        let all = list_pending_diffs(&conn, "ws1", None).unwrap();
        assert_eq!(
            all.iter().map(|d| d.diff_id.as_str()).collect::<Vec<_>>(),
            vec!["d2", "d1"]
        );
        assert_eq!(
            all[1].status,
            DiffStatus::Rejected,
            "状态从 DB 文本正确解析"
        );
        assert_eq!(all[0].target.doc_type, "md", "doc_type 由 rel_path 推断");

        // status 过滤：仅 pending → d2。
        let pending = list_pending_diffs(&conn, "ws1", Some("pending")).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].diff_id, "d2");

        // 其他 workspace 无结果（按 workspace 隔离）。
        assert!(list_pending_diffs(&conn, "wsX", None).unwrap().is_empty());
    }

    // covers: BR-WS-STATE-001
    // accepted 落盘：写文件 + 记恰好一个时间轴节点 + status=accepted。
    #[test]
    fn accepted_applies_writes_file_and_one_node() {
        let conn = fresh();
        let diff = sample_diff("d1");
        persist_diff(&conn, &diff).unwrap();
        assert_eq!(status_of(&conn, "d1"), "pending");

        let mut path = std::env::temp_dir();
        path.push(format!("binder_apply_{}.md", now_millis()));
        let _ = std::fs::remove_file(&path);

        let entry = apply_diff(&conn, "d1", &diff.target, &path, b"new content", None).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"new content");
        assert_eq!(entry.version_seq, 1); // 恰好一个节点
        assert_eq!(
            timeline::list_timeline(&conn, &diff.target).unwrap().len(),
            1
        );
        assert_eq!(status_of(&conn, "d1"), "accepted");
        let _ = std::fs::remove_file(&path);
    }

    // covers: BR-WS-STATE-001
    // rejected/expired：不写文件、不记节点、仅置终态。
    #[test]
    fn rejected_and_expired_do_not_write_file_or_node() {
        let conn = fresh();
        for (id, terminal) in [("d2", "rejected"), ("d3", "expired")] {
            let diff = sample_diff(id);
            persist_diff(&conn, &diff).unwrap();
            discard_diff(&conn, id, terminal).unwrap();
            assert_eq!(status_of(&conn, id), terminal);
        }
        // 无任何时间轴节点（rejected/expired 不记节点）
        assert_eq!(
            timeline::list_timeline(&conn, &sample_diff("d2").target)
                .unwrap()
                .len(),
            0
        );
    }
}
