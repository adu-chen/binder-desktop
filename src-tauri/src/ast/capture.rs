//! PI-AST-02 · 自动写入机 SM-AST-01 与类型分级 · AST-MEMORY / BR-AST-STATE-001
//!
//! 承接 A-AST-M-S-01 SM-AST-01：可沉淀事实自动写入（无需确认，按类型分级——人类已决策），写入条目
//! 可见、可溯源（origin_turn）、可撤销；类型不可判定或来源不可溯 → 不写入（无残留）；撤销后即刻停止
//! 供给且**不可复活**（终态 Revoked）。落库于当前 workspace 控制态（隔离由 PI-AST-01 store 保证）。
//! 类型分级标准与去重/合并策略为实现期细化（AST-02，A 类）。

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
        "mem_{}_{}",
        now_millis(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    )
}

/// SM-AST-01 状态（A-AST-M-S-01 §2）。Written/Revoked/Discarded 为终态；WriteError 可重试。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryState {
    Capturing,
    Writing,
    Written,
    WriteError,
    Revoked,
    Discarded,
}

/// SM-AST-01 事件。CaptureHit 携带「类型可判定 / 来源可溯」两要素——二者皆备方可写入。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryEvent {
    CaptureHit {
        type_resolvable: bool,
        origin_traceable: bool,
    },
    CaptureMiss,
    WriteOk,
    WriteFail,
    Retry,
    Revoke,
}

/**
 * @GOV
 * codes:    BR-AST-STATE-001-RB-AST-AST-MEMORY-001
 * type:     RB
 * chain:    AST-MEMORY
 * rules:    BR-AST-STATE-001
 * boundary: in=MemoryEvent | out=MemoryState(noTypeNoOriginRejected,revokedIrreversible)
 */
/// SM-AST-01 转移引擎（BR-AST-STATE-001）：captureHit 须**类型可判定且来源可溯**方可进入 Writing
/// （任一缺失 → 无合法转移，等同 captureMiss 的不写语义）；非可沉淀 → Discarded（无残留）；写入
/// 成功 → Written，失败 → WriteError（可 retry）；Written 经 revoke → Revoked **终态不可复活**。
/// 返回 `None` = 无合法转移（含全部 forbidden：未经 Writing 不得 Written、终态不再转移）。
pub fn transition(state: MemoryState, event: MemoryEvent) -> Option<MemoryState> {
    use MemoryEvent::*;
    use MemoryState::*;
    let next = match (state, event) {
        (
            Capturing,
            CaptureHit {
                type_resolvable: true,
                origin_traceable: true,
            },
        ) => Writing,
        // captureHit 缺类型/缺来源：不写入（无合法转移，与 captureMiss 同效——不残留）。
        (Capturing, CaptureMiss) => Discarded,
        (Writing, WriteOk) => Written,
        (Writing, WriteFail) => WriteError,
        (WriteError, Retry) => Writing,
        (Written, Revoke) => Revoked,
        // 其余：无合法转移（终态 Written/Revoked/Discarded 不再转移；撤销不可复活；未经 Writing 不得 Written）。
        _ => return None,
    };
    Some(next)
}

/**
 * @GOV
 * codes:    BR-AST-STATE-001-EFFECT-AST-AST-MEMORY-001
 * type:     EFFECT
 * chain:    AST-MEMORY
 * rules:    BR-AST-STATE-001
 * boundary: in=MemoryCandidate | out=visibleTraceableEntry(rejectedWhenNoKindNoOrigin)
 */
/// 自动写入（BR-AST-STATE-001）：类型分级条目落 memory_entries（status='active'，可见、可溯源）。
/// **无类型（memory_kind 空）或无来源（origin_turn 空）一律拒绝**（与 SM-AST-01 captureHit 守卫一致，
/// 不写残留）。无需确认（人类已决策按类型分级自动写入）。返回 entry_id。
pub fn capture_memory(
    conn: &Connection,
    workspace_id: &str,
    memory_kind: &str,
    content: &str,
    origin_turn: &str,
) -> Result<String, IpcError> {
    if memory_kind.trim().is_empty() || origin_turn.trim().is_empty() {
        return Err(IpcError::new(
            "AST_CAPTURE_REJECTED",
            "记忆写入被拒：类型不可判定或来源不可溯（无类型/无来源不写入）",
            false,
            false,
        ));
    }
    let entry_id = gen_entry_id();
    conn.execute(
        "INSERT INTO memory_entries \
         (entry_id, workspace_id, memory_kind, content, origin_turn, status, created_at, revoked_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, 'active', ?6, NULL)",
        (
            &entry_id,
            workspace_id,
            memory_kind,
            content,
            origin_turn,
            now_millis(),
        ),
    )
    .map_err(|e| IpcError::new("AST_DB_ERROR", &format!("记忆落库失败: {e}"), true, false))?;
    Ok(entry_id)
}

/**
 * @GOV
 * codes:    BR-AST-STATE-001-EFFECT-AST-AST-MEMORY-002
 * type:     EFFECT
 * chain:    AST-MEMORY
 * rules:    BR-AST-STATE-001
 * boundary: in=entryId | out=revokedEntry(supplyStopsIrreversible)
 */
/// 撤销（BR-AST-STATE-001）：置 status='revoked' 并记 revoked_at——即刻停止供给（供给侧只取 active）。
/// **不可复活**：无任何路径将 revoked 改回 active（仅可新建条目），与 SM-AST-01 Revoked 终态一致。
/// 返回是否命中一条 active 条目（已撤销/不存在 → false，幂等不报错）。
pub fn revoke_memory(conn: &Connection, entry_id: &str) -> rusqlite::Result<bool> {
    let n = conn.execute(
        "UPDATE memory_entries SET status='revoked', revoked_at=?2 \
         WHERE entry_id=?1 AND status='active'",
        (entry_id, now_millis()),
    )?;
    Ok(n == 1)
}

#[cfg(test)]
mod tests {
    use super::MemoryEvent::*;
    use super::MemoryState::*;
    use super::*;
    use crate::db;

    fn fresh() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        db::migrate(&mut conn).unwrap();
        conn
    }

    fn hit(t: bool, o: bool) -> super::MemoryEvent {
        CaptureHit {
            type_resolvable: t,
            origin_traceable: o,
        }
    }

    fn status_of(conn: &Connection, id: &str) -> String {
        conn.query_row(
            "SELECT status FROM memory_entries WHERE entry_id=?1",
            [id],
            |r| r.get(0),
        )
        .unwrap()
    }

    // covers: BR-AST-STATE-001
    // SM-AST-01 矩阵：可沉淀（类型+来源齐）→ Writing→Written；非可沉淀/缺类型/缺来源 → 不写入；
    // 写失败→WriteError→retry；Written→revoke→Revoked 终态不可复活；终态/未经 Writing 不转移。
    #[test]
    fn sm_ast01_matrix_and_revoke_irreversible() {
        // 正路：类型+来源齐 → Writing → Written
        assert_eq!(transition(Capturing, hit(true, true)), Some(Writing));
        assert_eq!(transition(Writing, WriteOk), Some(Written));
        // 非可沉淀 → Discarded（无残留）
        assert_eq!(transition(Capturing, CaptureMiss), Some(Discarded));
        // 缺类型 / 缺来源 → 无合法转移（不写入，等同不沉淀）
        assert_eq!(
            transition(Capturing, hit(false, true)),
            None,
            "缺类型不得写入"
        );
        assert_eq!(
            transition(Capturing, hit(true, false)),
            None,
            "缺来源不得写入"
        );
        // 写失败可重试
        assert_eq!(transition(Writing, WriteFail), Some(WriteError));
        assert_eq!(transition(WriteError, Retry), Some(Writing));
        // 撤销 → Revoked，不可复活：Revoked 对任何事件不再转移
        assert_eq!(transition(Written, Revoke), Some(Revoked));
        for ev in [hit(true, true), CaptureMiss, WriteOk, Retry, Revoke] {
            assert_eq!(transition(Revoked, ev), None, "Revoked 终态不可复活/不转移");
        }
        // 终态 Discarded/Written 不再转移；未经 Writing 不得 Written
        assert_eq!(transition(Discarded, hit(true, true)), None);
        assert_eq!(
            transition(Capturing, WriteOk),
            None,
            "未经 Writing 不得 Written"
        );
        assert_eq!(transition(Written, WriteOk), None);
    }

    // covers: BR-AST-STATE-001
    // 自动写入：类型+来源齐 → active 条目可见、可溯源（origin_turn）；无类型/无来源拒绝（不残留）。
    #[test]
    fn capture_writes_visible_traceable_entry_and_rejects_incomplete() {
        let conn = fresh();
        let id = capture_memory(&conn, "ws1", "preference", "用户偏好简洁", "turn_7").unwrap();
        let (kind, origin, status): (String, String, String) = conn
            .query_row(
                "SELECT memory_kind, origin_turn, status FROM memory_entries WHERE entry_id=?1",
                [&id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(kind, "preference"); // 类型分级可见
        assert_eq!(origin, "turn_7"); // 可溯源
        assert_eq!(status, "active"); // 可见（供给取 active）

        // 无类型 / 无来源 → 拒绝，不落库
        assert_eq!(
            capture_memory(&conn, "ws1", "  ", "x", "turn_8")
                .unwrap_err()
                .code,
            "AST_CAPTURE_REJECTED"
        );
        assert_eq!(
            capture_memory(&conn, "ws1", "fact", "x", "")
                .unwrap_err()
                .code,
            "AST_CAPTURE_REJECTED"
        );
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM memory_entries", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1, "拒绝项不得残留");
    }

    // covers: BR-AST-STATE-001
    // 撤销即刻停止供给且不可复活：active→revoked；重复撤销/不存在幂等返回 false；无路径改回 active。
    #[test]
    fn revoke_stops_supply_and_is_irreversible() {
        let conn = fresh();
        let id = capture_memory(&conn, "ws1", "fact", "事实A", "turn_1").unwrap();
        assert!(revoke_memory(&conn, &id).unwrap());
        assert_eq!(status_of(&conn, &id), "revoked");
        // 已撤销再撤销：幂等 false（只命中 active）
        assert!(!revoke_memory(&conn, &id).unwrap());
        // 不存在条目：幂等 false
        assert!(!revoke_memory(&conn, "nope").unwrap());
        // 撤销后仍为 revoked（无复活路径）
        assert_eq!(status_of(&conn, &id), "revoked");
    }
}
