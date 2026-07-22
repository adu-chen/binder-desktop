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
pub fn list_pending(
    conn: &Connection,
    workspace_id: &str,
) -> rusqlite::Result<Vec<PendingDiffRow>> {
    let mut stmt = conn.prepare(
        "SELECT diff_id, rel_path, created_at FROM pending_diffs \
         WHERE workspace_id=?1 AND status='pending' ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map([workspace_id], |r| {
        Ok(PendingDiffRow {
            diff_id: r.get(0)?,
            rel_path: r.get(1)?,
            created_at: r.get(2)?,
        })
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
        .filter_map(|p| {
            match recheck_overlap(applied_blocks, &p.blocks, p.before_hash, p.current_hash) {
                ExpiryDecision::Expire(_) => Some(p.diff_id.clone()),
                ExpiryDecision::KeepPending => None,
            }
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
/// **不写文件、不记时间轴节点**（落盘语义归 WS）。`terminal_reason` 可空（BR-SYS-DATA-005），供
/// 历史只读展示区分过期原因（如应用启动异常退出清算传 `Some("abnormal_exit")`）。返回成功落终态的条数。
pub fn expire_diffs(
    conn: &Connection,
    diff_ids: &[String],
    terminal_reason: Option<&str>,
) -> rusqlite::Result<usize> {
    let mut n = 0;
    for id in diff_ids {
        discard_diff(conn, id, "expired", terminal_reason)?;
        n += 1;
    }
    Ok(n)
}

/**
 * @GOV
 * codes:    BR-SYS-DATA-005-EFFECT-DE-DE-DIFF-003
 * type:     EFFECT
 * chain:    DE-DIFF
 * rules:    BR-SYS-DATA-005
 * boundary: in=workspaceIdAndRelPath | out=awaitingSaveDiffsFlippedToSaved(matchingRelPathOnly)
 */
/// 保存成功回调：把该 `rel_path` 下 `status='accepted' AND effect_status='awaiting_save'` 的行批量
/// 翻转为 `saved`（BR-SYS-DATA-005）。由 `ws.save_doc`/`doc.save_from_edit` 保存成功后调用，DE 不
/// 反向监听 ED/WS 内部状态机事件。返回翻转的行数（可能为 0，正常情况——该文档下没有等待落盘的 diff）。
pub fn flip_awaiting_save_to_saved(
    conn: &Connection,
    workspace_id: &str,
    rel_path: &str,
) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE pending_diffs SET effect_status='saved' \
         WHERE workspace_id=?1 AND rel_path=?2 AND status='accepted' AND effect_status='awaiting_save'",
        rusqlite::params![workspace_id, rel_path],
    )
}

/**
 * @GOV
 * codes:    BR-SYS-DATA-005-EFFECT-DE-DE-DIFF-004
 * type:     EFFECT
 * chain:    DE-DIFF
 * rules:    BR-SYS-DATA-005
 * boundary: in=workspaceIdAndTerminalReason | out=dirtySnapshotsDiscardedAndAwaitingSaveDiffsFlipped
 */
/// 丢弃未保存内容的共享步骤（BR-SYS-DATA-005）：把该 workspace **所有**
/// `status='accepted' AND effect_status='awaiting_save'` 行转 `discarded_on_abnormal_exit`。
/// `awaiting_save` 只表示当前进程尚未确认写盘；它不能跨会话存活，且不能以 dirty 快照是否已落库
/// 作为保留条件（编辑快照的防抖写入可能尚未执行）。该 effect_status 值表示"内容随未保存草稿一起丢失"，
/// 覆盖崩溃与用户在正常关闭流程内显式选择放弃两种触发源——`terminal_reason` 区分具体原因，效果态本身相同：
/// 内容确未落盘。随后删除全部 dirty 快照行。被
/// `sweep_abnormal_exit`（reason="abnormal_exit"，启动期）与显式的"放弃未保存修改"命令
/// （reason="user_discarded_on_close"，正常关闭流程内用户触发）共用。返回
/// (丢弃的快照行数, 效果翻转的 diff 行数)。
pub fn discard_dirty_sessions(
    conn: &Connection,
    workspace_id: &str,
    terminal_reason: &str,
) -> rusqlite::Result<(usize, usize)> {
    // 先终结 effect，再删快照：即使后续删除快照失败，也绝不留下可跨会话继承的 awaiting_save。
    let discarded_awaiting_save_diffs = conn.execute(
        "UPDATE pending_diffs SET effect_status='discarded_on_abnormal_exit', terminal_reason=?2 \
         WHERE workspace_id=?1 AND status='accepted' AND effect_status='awaiting_save'",
        rusqlite::params![workspace_id, terminal_reason],
    )?;
    let discarded_dirty_snapshots = conn.execute(
        "DELETE FROM ed_session_snapshots WHERE workspace_id=?1 AND dirty=1",
        [workspace_id],
    )?;

    Ok((discarded_dirty_snapshots, discarded_awaiting_save_diffs))
}

/// 单个 workspace 的异常退出清算结果（BR-SYS-STATE-005，A-SYS-M-T-02 §4）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AbnormalExitSweepReport {
    pub expired_pending: usize,
    pub discarded_dirty_snapshots: usize,
    pub discarded_awaiting_save_diffs: usize,
}

/**
 * @GOV
 * codes:    BR-SYS-STATE-005-EFFECT-SYS-DATA-MIGRATE-002
 * type:     EFFECT
 * chain:    DATA-MIGRATE
 * rules:    BR-SYS-STATE-005
 * boundary: in=workspaceId | out=AbnormalExitSweepReport(pendingExpiredDirtySnapshotsDiscarded)
 */
/// 单 workspace 异常退出清算（BR-SYS-STATE-005）：必须在应用启动、任何会话/历史恢复之前对每个已
/// 注册 workspace 调用一次。三步：① 该 workspace 全部 `status='pending'` diff 转 `expired`
/// （`terminal_reason='abnormal_exit'`，复用 `list_pending`/`expire_diffs`）；② 无条件把全部
/// `status='accepted' AND effect_status='awaiting_save'` 行转 `discarded_on_abnormal_exit`；③ 删除
/// `ed_session_snapshots.dirty=1` 行。`awaiting_save` 无论是否留下 dirty 快照均是未确认写盘的当前会话态，
/// 因而不得展示为已生效；dirty 快照也不作为可信 LogicalState
/// 恢复源，对应文档改由 ED 正常 `open` 从磁盘态重建）。无残留时三步均为空操作，函数本身不判断"是否
/// 异常退出"——调用方对每个 workspace 无条件调用，残留是否存在即结果。
pub fn sweep_abnormal_exit(
    conn: &Connection,
    workspace_id: &str,
) -> rusqlite::Result<AbnormalExitSweepReport> {
    let mut report = AbnormalExitSweepReport::default();

    // ① 残留 pending diff 全部过期。
    let pending = list_pending(conn, workspace_id)?;
    if !pending.is_empty() {
        let ids: Vec<String> = pending.iter().map(|r| r.diff_id.clone()).collect();
        report.expired_pending = expire_diffs(conn, &ids, Some("abnormal_exit"))?;
    }

    // ② + ③：未保存内容一律丢弃（异常退出场景，terminal_reason=abnormal_exit）。
    let (discarded_dirty_snapshots, discarded_awaiting_save_diffs) =
        discard_dirty_sessions(conn, workspace_id, "abnormal_exit")?;
    report.discarded_dirty_snapshots = discarded_dirty_snapshots;
    report.discarded_awaiting_save_diffs = discarded_awaiting_save_diffs;

    Ok(report)
}

/**
 * @GOV
 * codes:    BR-SYS-STATE-005-EFFECT-SYS-DATA-MIGRATE-003
 * type:     EFFECT
 * chain:    DATA-MIGRATE
 * rules:    BR-SYS-STATE-005
 * boundary: in=workspaceDbConnection | out=allWorkspacesSwept(precedesSessionAndHistoryRestore)
 */
/// 对全部已注册 workspace 执行异常退出清算（应用 `setup()` 内先于任何会话/历史恢复调用）：不区分
/// 当前活动 workspace，因为崩溃可能发生在任意一个 workspace 处于活动态时；任一 workspace 清算失败
/// 都必须让 setup 失败，避免该 workspace 的 pending diff 被恢复为可执行卡片。
pub fn sweep_all_workspaces_on_startup(
    conn: &Connection,
) -> rusqlite::Result<Vec<(String, AbnormalExitSweepReport)>> {
    let mut stmt = conn.prepare("SELECT workspace_id FROM workspaces")?;
    let workspace_ids: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    drop(stmt);
    let mut reports = Vec::with_capacity(workspace_ids.len());
    for workspace_id in workspace_ids {
        let report = sweep_abnormal_exit(conn, &workspace_id)?;
        reports.push((workspace_id, report));
    }
    Ok(reports)
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
            target: DocRef {
                workspace_id: "ws1".into(),
                rel_path: rel.into(),
                doc_type: "md".into(),
            },
            origin_module: "AG".into(),
            status: DiffStatus::Pending,
            created_at,
            payload_ref: "p".into(),
        };
        persist_diff(conn, &d).unwrap();
    }

    fn status_of(conn: &Connection, id: &str) -> String {
        conn.query_row(
            "SELECT status FROM pending_diffs WHERE diff_id=?1",
            [id],
            |r| r.get(0),
        )
        .unwrap()
    }

    // covers: BR-DE-STATE-002
    // 扫描过期：超时→过期；外部已变→过期；检测不可得且未超时→保持（不误杀）；只过期该过期的。
    #[test]
    fn sweep_expiry_timeout_external_and_conservative() {
        let rows = vec![
            PendingDiffRow {
                diff_id: "d_old".into(),
                rel_path: "a.md".into(),
                created_at: 0,
            },
            PendingDiffRow {
                diff_id: "d_new".into(),
                rel_path: "b.md".into(),
                created_at: 9_000,
            },
            PendingDiffRow {
                diff_id: "d_undet".into(),
                rel_path: "c.md".into(),
                created_at: 9_000,
            },
            PendingDiffRow {
                diff_id: "d_changed".into(),
                rel_path: "d.md".into(),
                created_at: 9_000,
            },
        ];
        // now=10_000, timeout=5_000：d_old 超时；其余未超时。外部：d_changed 已变；c.md 检测不可得。
        let expired = sweep_expiry(&rows, 10_000, 5_000, |rel| match rel {
            "d.md" => ExternalCheck::Changed,
            "c.md" => ExternalCheck::Undetectable,
            _ => ExternalCheck::Unchanged,
        });
        assert!(expired.contains(&"d_old".to_string()), "超时应过期");
        assert!(expired.contains(&"d_changed".to_string()), "外部已变应过期");
        assert!(
            !expired.contains(&"d_undet".to_string()),
            "检测不可得+未超时不误杀"
        );
        assert!(!expired.contains(&"d_new".to_string()), "未超时未变保持");
    }

    // covers: BR-DE-DATA-003
    // 冲突重检：重叠且内容已变→过期；不重叠或未变→保持。
    #[test]
    fn sweep_conflicts_overlap_stale_only() {
        let applied = vec!["b1".to_string(), "b2".to_string()];
        let pendings = vec![
            ConflictInput {
                diff_id: "stale".into(),
                blocks: vec!["b2".into()],
                before_hash: 1,
                current_hash: 2,
            },
            ConflictInput {
                diff_id: "unchanged".into(),
                blocks: vec!["b2".into()],
                before_hash: 1,
                current_hash: 1,
            },
            ConflictInput {
                diff_id: "disjoint".into(),
                blocks: vec!["b9".into()],
                before_hash: 1,
                current_hash: 2,
            },
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
        let n = expire_diffs(&conn, &expired, None).unwrap();
        assert_eq!(n, 1);
        assert_eq!(status_of(&conn, "d_old"), "expired"); // 过期落终态
        assert_eq!(status_of(&conn, "d_new"), "pending"); // 未过期保持
                                                          // 过期后不再出现在 pending 列举
        assert_eq!(list_pending(&conn, "ws1").unwrap().len(), 1);
    }

    fn effect_status_of(conn: &Connection, id: &str) -> String {
        conn.query_row(
            "SELECT effect_status FROM pending_diffs WHERE diff_id=?1",
            [id],
            |r| r.get(0),
        )
        .unwrap()
    }

    fn terminal_reason_of(conn: &Connection, id: &str) -> Option<String> {
        conn.query_row(
            "SELECT terminal_reason FROM pending_diffs WHERE diff_id=?1",
            [id],
            |r| r.get(0),
        )
        .unwrap()
    }

    fn seed_dirty_snapshot(conn: &Connection, rel_path: &str) {
        conn.execute(
            "INSERT INTO ed_session_snapshots \
             (workspace_id, rel_path, content, edit_format, dirty, session_version, content_hash, updated_at) \
             VALUES ('ws1', ?1, '未保存内容', 'text', 1, 1, 'h1', 0)",
            [rel_path],
        )
        .unwrap();
    }

    // covers: BR-SYS-STATE-005
    // 异常退出清算三步：① 残留 pending 全部过期（reason=abnormal_exit）；② 所有
    // accepted+awaiting_save diff 转 discarded_on_abnormal_exit；③ dirty 快照本身被删除。
    // saved 对照组保持不变；awaiting_save 不得因尚未来得及持久化 dirty 快照而跨会话保留。
    #[test]
    fn sweep_abnormal_exit_expires_pending_and_discards_unsaved_effect() {
        let conn = fresh();

        // 残留 pending diff。
        seed_diff(&conn, "d_pending", "a.md", 0);

        // accepted+awaiting_save 命中 dirty 快照 → 应转 discarded_on_abnormal_exit。
        seed_diff(&conn, "d_awaiting", "b.md", 0);
        conn.execute(
            "UPDATE pending_diffs SET status='accepted', effect_status='awaiting_save' WHERE diff_id='d_awaiting'",
            [],
        )
        .unwrap();
        seed_dirty_snapshot(&conn, "b.md");

        // 对照组 1：accepted+saved（background 路径已落盘）不受影响。
        seed_diff(&conn, "d_saved", "c.md", 0);
        conn.execute(
            "UPDATE pending_diffs SET status='accepted', effect_status='saved' WHERE diff_id='d_saved'",
            [],
        )
        .unwrap();

        // 无 dirty 快照的 awaiting_save 仍是未确认写盘的会话态，必须丢弃。
        seed_diff(&conn, "d_awaiting_clean", "d.md", 0);
        conn.execute(
            "UPDATE pending_diffs SET status='accepted', effect_status='awaiting_save' WHERE diff_id='d_awaiting_clean'",
            [],
        )
        .unwrap();

        let report = sweep_abnormal_exit(&conn, "ws1").unwrap();
        assert_eq!(report.expired_pending, 1);
        assert_eq!(report.discarded_dirty_snapshots, 1);
        assert_eq!(report.discarded_awaiting_save_diffs, 2);

        assert_eq!(status_of(&conn, "d_pending"), "expired");
        assert_eq!(
            terminal_reason_of(&conn, "d_pending"),
            Some("abnormal_exit".to_string())
        );

        assert_eq!(
            effect_status_of(&conn, "d_awaiting"),
            "discarded_on_abnormal_exit"
        );
        assert_eq!(
            terminal_reason_of(&conn, "d_awaiting"),
            Some("abnormal_exit".to_string())
        );

        assert_eq!(effect_status_of(&conn, "d_saved"), "saved"); // 不受影响
        assert_eq!(
            effect_status_of(&conn, "d_awaiting_clean"),
            "discarded_on_abnormal_exit"
        );

        // dirty 快照已被删除，不作为可信 LogicalState 恢复源。
        let remaining: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ed_session_snapshots WHERE workspace_id='ws1' AND rel_path='b.md'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(remaining, 0);
    }

    // covers: BR-SYS-STATE-005
    // 无残留时清算为空操作（幂等）：不产生任何转移，也不报错。
    #[test]
    fn sweep_abnormal_exit_is_noop_when_clean() {
        let conn = fresh();
        seed_diff(&conn, "d1", "a.md", 0);
        conn.execute(
            "UPDATE pending_diffs SET status='accepted', effect_status='saved' WHERE diff_id='d1'",
            [],
        )
        .unwrap();
        let report = sweep_abnormal_exit(&conn, "ws1").unwrap();
        assert_eq!(report, AbnormalExitSweepReport::default());
        assert_eq!(effect_status_of(&conn, "d1"), "saved");
    }

    // covers: BR-SYS-DATA-005
    // discard_dirty_sessions 与 sweep_abnormal_exit 共享同一丢弃步骤，但 terminal_reason 按调用方
    // 区分触发源（正常关闭内用户显式放弃 vs 启动期异常退出清算），不新增独立 effect_status 取值。
    #[test]
    fn discard_dirty_sessions_uses_caller_supplied_terminal_reason() {
        let conn = fresh();
        seed_diff(&conn, "d_awaiting", "b.md", 0);
        conn.execute(
            "UPDATE pending_diffs SET status='accepted', effect_status='awaiting_save' WHERE diff_id='d_awaiting'",
            [],
        )
        .unwrap();
        seed_dirty_snapshot(&conn, "b.md");

        let (discarded_snapshots, discarded_diffs) =
            discard_dirty_sessions(&conn, "ws1", "user_discarded_on_close").unwrap();
        assert_eq!(discarded_snapshots, 1);
        assert_eq!(discarded_diffs, 1);
        assert_eq!(
            effect_status_of(&conn, "d_awaiting"),
            "discarded_on_abnormal_exit"
        );
        assert_eq!(
            terminal_reason_of(&conn, "d_awaiting"),
            Some("user_discarded_on_close".to_string())
        );
    }

    // covers: BR-SYS-STATE-005
    // 多 workspace 场景：只清算各自的残留，不跨 workspace 串扰；崩溃可能发生在任一 workspace 活动时，
    // 不依赖"当前活动 workspace"判断，逐个已注册 workspace 独立清算。
    #[test]
    fn sweep_all_workspaces_on_startup_isolates_per_workspace() {
        let mut conn = Connection::open_in_memory().unwrap();
        db::migrate(&mut conn).unwrap();
        conn.execute(
            "INSERT INTO workspaces (workspace_id, root_path, is_active, opened_at) VALUES \
             ('ws1','/w1',1,0), ('ws2','/w2',0,0)",
            [],
        )
        .unwrap();
        let d1 = PendingDiff {
            diff_id: "d1".into(),
            target: DocRef {
                workspace_id: "ws1".into(),
                rel_path: "a.md".into(),
                doc_type: "md".into(),
            },
            origin_module: "AG".into(),
            status: DiffStatus::Pending,
            created_at: 0,
            payload_ref: "p1".into(),
        };
        let d2 = PendingDiff {
            diff_id: "d2".into(),
            target: DocRef {
                workspace_id: "ws2".into(),
                rel_path: "b.md".into(),
                doc_type: "md".into(),
            },
            origin_module: "AG".into(),
            status: DiffStatus::Pending,
            created_at: 0,
            payload_ref: "p2".into(),
        };
        persist_diff(&conn, &d1).unwrap();
        persist_diff(&conn, &d2).unwrap();

        let reports = sweep_all_workspaces_on_startup(&conn).unwrap();
        assert_eq!(reports.len(), 2);
        for (_, r) in &reports {
            assert_eq!(r.expired_pending, 1);
        }
        assert_eq!(status_of(&conn, "d1"), "expired");
        assert_eq!(status_of(&conn, "d2"), "expired");
    }
}
