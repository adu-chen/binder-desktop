//! PI-DE-02 · diff 终态机 SM-DE-01 · DE-DIFF / BR-DE-STATE-001
//!
//! 承接 A-DE-M-S-01 SM-DE-01：单条待裁决修改自生成到终态闭合。三终态（Accepted/Rejected/Expired）
//! 互斥不可逆；accepted 是 WS 落盘的唯一触发（驱动 SM-WS-02，X-INTER-001）；ApplyError 不回退终态。
//! 过期的保守判定（超时/外部变更检测，BR-DE-STATE-002）与冲突重检（BR-DE-DATA-003）归 PI-DE-03；
//! 本机 transition 仅承接 timeout/externalChange 的终态转移（判定是否触发由 PI-DE-03 决定）。

/// PI-DE-01 · diff 生成与载荷（BR-DE-DATA-001 / BR-DE-DATA-002）；ChangeProposal 单一拥有方。
pub mod generate;

/// PI-DE-03 · 过期保守判定与多 diff 冲突重检（BR-DE-STATE-002 / BR-DE-DATA-003）。
pub mod expiry;

/// PI-BE-02 · DiffPayload 持久化（diff_payloads，BR-DE-DATA-002）；accepted 落盘取回内容。
pub mod store;

/// SM-DE-01 状态（A-DE-M-S-01 §2）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffState {
    Generating,
    Pending,
    Accepted,
    Rejected,
    Expired,
    GenFailed,
}

/// SM-DE-01 事件。userAccept 是通往落盘的唯一触发；appliedOk/applyFail 为 WS 落盘回执。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffEvent {
    GenOk,
    GenFail,
    UserAccept,
    UserReject,
    Timeout,
    ExternalChange,
    AppliedOk,
    ApplyFail,
}

/// 终态判定（三终态 + GenFailed 均为 SM-DE-01 terminal_states）。
pub fn is_terminal(state: DiffState) -> bool {
    matches!(
        state,
        DiffState::Accepted | DiffState::Rejected | DiffState::Expired | DiffState::GenFailed
    )
}

/**
 * @GOV
 * codes:    BR-DE-STATE-001-RB-DE-DE-DIFF-001
 * type:     RB
 * chain:    DE-DIFF
 * rules:    BR-DE-STATE-001
 * boundary: in=DiffEvent | out=DiffState(threeExclusiveIrreversibleTerminals,acceptOnlyToApply)
 */
/// SM-DE-01 转移引擎（BR-DE-STATE-001）：三终态互斥不可逆——Accepted/Rejected/Expired 间不互转、
/// 不复活；userAccept 是唯一通往落盘（Accepted）的路径；ApplyError 回执不回退终态（Accepted 保持）。
/// 返回 `None` = 无合法转移（含全部 forbidden_transitions）。
pub fn transition(state: DiffState, event: DiffEvent) -> Option<DiffState> {
    use DiffEvent::*;
    use DiffState::*;
    let next = match (state, event) {
        (Generating, GenOk) => Pending,
        (Generating, GenFail) => GenFailed,
        (Pending, UserAccept) => Accepted, // 唯一触发 SM-WS-02 落盘
        (Pending, UserReject) => Rejected,
        (Pending, Timeout) => Expired,
        (Pending, ExternalChange) => Expired,
        // 落盘回执：Accepted 终态保持（applyFail 不回退，落盘重试归 WS）。
        (Accepted, AppliedOk) => Accepted,
        (Accepted, ApplyFail) => Accepted,
        // 其余：无合法转移（三终态互斥不可逆、未呈现不得裁决、无 accept 不通往落盘）。
        _ => return None,
    };
    Some(next)
}

#[cfg(test)]
mod tests {
    use super::DiffEvent::*;
    use super::DiffState::*;
    use super::*;

    // covers: BR-DE-STATE-001
    // 正路（生成→裁决→落盘回执）+ 各裁决终态 + accepted 唯一通往落盘。
    #[test]
    fn lifecycle_and_accept_is_only_path_to_apply() {
        assert_eq!(transition(Generating, GenOk), Some(Pending));
        assert_eq!(transition(Generating, GenFail), Some(GenFailed));
        assert_eq!(transition(Pending, UserAccept), Some(Accepted));
        assert_eq!(transition(Pending, UserReject), Some(Rejected));
        assert_eq!(transition(Pending, Timeout), Some(Expired));
        assert_eq!(transition(Pending, ExternalChange), Some(Expired));
        // 落盘回执不改终态；applyFail 不回退（落盘重试归 WS SM-WS-02）
        assert_eq!(transition(Accepted, AppliedOk), Some(Accepted));
        assert_eq!(transition(Accepted, ApplyFail), Some(Accepted));
        // 唯一通往 Accepted（落盘）的路径是 Pending+UserAccept
        for from in [Generating, Rejected, Expired, GenFailed] {
            assert_eq!(transition(from, UserAccept), None, "{from:?} 不应可达 Accepted");
        }
    }

    // covers: BR-DE-STATE-001
    // 三终态互斥不可逆：终态间不互转、不复活；未呈现态（Generating）不得裁决；Accepted 不回退 Pending。
    #[test]
    fn terminals_are_exclusive_and_irreversible() {
        // 三终态 + GenFailed 对任何事件不再转移（自闭，除 Accepted 落盘回执保持自身）
        for ev in [UserAccept, UserReject, Timeout, ExternalChange, GenOk] {
            assert_eq!(transition(Rejected, ev), None);
            assert_eq!(transition(Expired, ev), None);
            assert_eq!(transition(GenFailed, ev), None);
        }
        // Accepted 不可转其他终态、不回退 Pending
        for ev in [UserReject, Timeout, ExternalChange, GenOk] {
            assert_eq!(transition(Accepted, ev), None, "Accepted 不可逆/不可转其他终态");
        }
        // 未呈现态不得裁决
        assert_eq!(transition(Generating, UserAccept), None);
        assert_eq!(transition(Generating, UserReject), None);
        // 终态判定
        assert!(is_terminal(Accepted) && is_terminal(Rejected) && is_terminal(Expired) && is_terminal(GenFailed));
        assert!(!is_terminal(Generating) && !is_terminal(Pending));
    }
}
