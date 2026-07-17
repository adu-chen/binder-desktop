//! PI-AG-05 · 记忆写入对接（强制承接 B-2）· AG-TURN / BR-AG-STATE-004
//!
//! 承接 A-AG-M-T-01 BR-AG-STATE-004：回合产出处置完成后（SM-AG-01 HandingOff→Completed / HandedOver
//! 终态），AG **异步触发记忆写入**——将回合浮现的可沉淀事实候选交 AST（ast.capture，传 origin_turn），
//! 由 AST 自动落库（BR-AST-STATE-001）。记忆写入**异步、不阻塞回合终态、失败不影响回合**（记忆为增益非
//! 关键路径，不进 X-FORBID-006 落盘路径）。可沉淀事实判定机制延实现期；本机锁「终态后触发、传 origin_turn、
//! 不阻塞、失败隔离」对接语义，落库语义归 AST。

use crate::ag::turn::TurnState;
use crate::ast::capture;
use rusqlite::Connection;

/// 交 AST 的记忆候选（origin_turn 来源回合标识，可溯源）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureRequest {
    pub origin_turn: String,
    pub memory_kind: String,
    pub content: String,
}

/**
 * @GOV
 * codes:    BR-AG-STATE-004-GUARD-AG-AG-TURN-001
 * type:     GUARD
 * chain:    AG-TURN
 * rules:    BR-AG-STATE-004
 * boundary: in=terminalTurnStateAndCandidate | out=CaptureRequestOrNone(onlyCompletedHandedOverWithCandidate)
 */
/// 终态记忆触发判定（BR-AG-STATE-004）：仅 SM-AG-01 **Completed / HandedOver** 终态且存在可沉淀候选时
/// 产出 CaptureRequest（带 origin_turn）；Cancelled/Denied/TurnError 或无候选 → None（不触发）。本函数
/// 只决定是否触发，不阻塞、不落库（落库经 try_capture 异步进行）。
pub fn on_turn_terminal(
    state: TurnState,
    turn_id: &str,
    candidate: Option<(&str, &str)>, // (memory_kind, content)
) -> Option<CaptureRequest> {
    let eligible = matches!(state, TurnState::Completed | TurnState::HandedOver);
    match (eligible, candidate) {
        (true, Some((kind, content))) => Some(CaptureRequest {
            origin_turn: turn_id.to_string(),
            memory_kind: kind.to_string(),
            content: content.to_string(),
        }),
        _ => None, // 非终态/非合格终态/无候选 → 不触发
    }
}

/**
 * @GOV
 * codes:    BR-AG-STATE-004-EFFECT-AG-AG-TURN-002
 * type:     EFFECT
 * chain:    AG-TURN
 * rules:    BR-AG-STATE-004
 * boundary: in=CaptureRequestAndWorkspace | out=memoryWriteAttempt(failureIsolatedNeverAffectsTurn)
 */
/// 异步写入尝试（BR-AG-STATE-004）：调 ast.capture 落库（传 origin_turn）。**失败被隔离**——返回 false
/// 而非传播错误（记忆为增益，失败不影响回合终态）。调用方在回合终态后异步触发，不阻塞回合。
pub fn try_capture(conn: &Connection, workspace_id: &str, req: &CaptureRequest) -> bool {
    // 失败隔离：ast.capture 的 Err（如无类型/无来源、落库失败）不向回合传播。
    capture::capture_memory(
        conn,
        workspace_id,
        &req.memory_kind,
        &req.content,
        &req.origin_turn,
    )
    .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ag::turn::TurnState;
    use crate::db;

    fn fresh() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        db::migrate(&mut conn).unwrap();
        conn
    }

    // covers: BR-AG-STATE-004
    // 仅合格终态（Completed/HandedOver）+ 有候选触发，带 origin_turn；其余不触发。
    #[test]
    fn triggers_only_on_eligible_terminal_with_candidate() {
        let req = on_turn_terminal(
            TurnState::Completed,
            "turn_9",
            Some(("fact", "用户用 Rust")),
        )
        .unwrap();
        assert_eq!(req.origin_turn, "turn_9"); // 传 origin_turn 可溯源
        assert!(on_turn_terminal(TurnState::HandedOver, "t", Some(("fact", "x"))).is_some());
        // 取消/拒绝终态不触发
        assert!(on_turn_terminal(TurnState::Cancelled, "t", Some(("fact", "x"))).is_none());
        assert!(on_turn_terminal(TurnState::Denied, "t", Some(("fact", "x"))).is_none());
        // 非终态不触发
        assert!(on_turn_terminal(TurnState::Streaming, "t", Some(("fact", "x"))).is_none());
        // 合格终态但无候选不触发
        assert!(on_turn_terminal(TurnState::Completed, "t", None).is_none());
    }

    // covers: BR-AG-STATE-004
    // 写入失败被隔离：合规候选落库成功返回 true；无类型候选 ast 拒绝，try_capture 返回 false 不报错。
    #[test]
    fn capture_failure_is_isolated_from_turn() {
        let conn = fresh();
        let ok = CaptureRequest {
            origin_turn: "t1".into(),
            memory_kind: "fact".into(),
            content: "事实".into(),
        };
        assert!(try_capture(&conn, "ws1", &ok)); // 成功落库

        // AST 会拒绝无类型候选；try_capture 隔离为 false（不 panic、不传播 Err）
        let bad = CaptureRequest {
            origin_turn: "t2".into(),
            memory_kind: "".into(),
            content: "x".into(),
        };
        assert!(!try_capture(&conn, "ws1", &bad), "失败应被隔离为 false");
        // 回合不受影响：库中仅有成功的一条
        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM memory_entries", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);
    }
}
