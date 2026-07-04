//! PI-AG-04 · 局部修改/辅助续写机 SM-AG-03 · AG-ASSIST / BR-AG-STATE-003
//!
//! 承接 A-AG-M-S-01 SM-AG-03（autocomplete 与 inline-assist 同机承载，context 区分）：按**节流策略**触发
//! （不得每次按键直发调用）；每次调用同样**经门禁与网关**（BR-AG-VERIFY-001，无例外路径）；候选经 ED
//! 装饰层呈现，**锚失效放弃**本次呈现；**采纳是用户编辑操作**——经 ED 受控接口进入编辑模型走 SM-ED-01→
//! SM-WS-01 正常脏态/保存路径，不构成 AI 直写文件；无订阅/断网/调用失败时**静默不提供**，不打断编辑、
//! 不弹错误。节流参数/成本策略延实现期标定（AG-03 数值部分），本机锁机制语义。

/// SM-AG-03 状态。Accepted/Dismissed 为终态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssistState {
    Idle,
    Throttling,
    Requesting,
    Presenting,
    Accepted,
    Dismissed,
}

/// SM-AG-03 事件。throttlePass 携带门禁是否可用（GatePassed 且有订阅/在线）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssistEvent {
    Trigger,
    ThrottleDrop,
    ThrottlePass { gate_available: bool },
    CandidateOk,
    RequestFail,
    Accept,
    Dismiss,
    AnchorLost,
}

/// 终态判定。
pub fn is_terminal(state: AssistState) -> bool {
    matches!(state, AssistState::Accepted | AssistState::Dismissed)
}

/**
 * @GOV
 * codes:    BR-AG-STATE-003-RB-AG-AG-ASSIST-001
 * type:     RB
 * chain:    AG-ASSIST
 * rules:    BR-AG-STATE-003
 * boundary: in=AssistEvent | out=AssistState(throttledGateGatedSilentOnFailAcceptIsEditAnchorLostDismiss)
 */
/// SM-AG-03 转移引擎（BR-AG-STATE-003）：trigger→节流；throttleDrop 静默回 Idle；throttlePass **仅在
/// 门禁可用**时→Requesting，否则静默回 Idle（无订阅/断网不提供、不打断）；requestFail 静默回 Idle（不弹
/// 错误）；candidateOk→Presenting；accept→Accepted（采纳=用户编辑，驱动 SM-ED-01）；dismiss/anchorLost→
/// Dismissed（装饰复位不残留）。返回 `None` = 无合法转移。
pub fn transition(state: AssistState, event: AssistEvent) -> Option<AssistState> {
    use AssistEvent::*;
    use AssistState::*;
    let next = match (state, event) {
        (Idle, Trigger) => Throttling,
        (Throttling, ThrottleDrop) => Idle, // 节流内/输入活跃，静默丢弃
        (Throttling, ThrottlePass { gate_available: true }) => Requesting,
        (Throttling, ThrottlePass { gate_available: false }) => Idle, // 无订阅/断网静默不提供
        (Requesting, CandidateOk) => Presenting,
        (Requesting, RequestFail) => Idle, // 静默（不弹错误、不打断编辑）
        (Presenting, Accept) => Accepted,  // 采纳=用户编辑操作
        (Presenting, Dismiss) => Dismissed,
        (Presenting, AnchorLost) => Dismissed, // 锚失效放弃呈现
        // 其余无合法转移（终态冻结；未过节流不得请求；未呈现不得采纳）。
        _ => return None,
    };
    Some(next)
}

#[cfg(test)]
mod tests {
    use super::AssistEvent::*;
    use super::AssistState::*;
    use super::*;

    // covers: BR-AG-STATE-003
    // 节流→门禁可用→请求→呈现→采纳；节流丢弃/门禁不可用/请求失败均静默回 Idle（不打断）。
    #[test]
    fn throttle_gate_request_present_accept_and_silent_paths() {
        assert_eq!(transition(Idle, Trigger), Some(Throttling));
        assert_eq!(transition(Throttling, ThrottlePass { gate_available: true }), Some(Requesting));
        assert_eq!(transition(Requesting, CandidateOk), Some(Presenting));
        assert_eq!(transition(Presenting, Accept), Some(Accepted));
        // 静默路径：节流丢弃 / 门禁不可用 / 请求失败 → 回 Idle（不打断、不弹错误）
        assert_eq!(transition(Throttling, ThrottleDrop), Some(Idle));
        assert_eq!(transition(Throttling, ThrottlePass { gate_available: false }), Some(Idle));
        assert_eq!(transition(Requesting, RequestFail), Some(Idle));
        // 门禁前置：未过节流不得直接请求
        assert_eq!(transition(Idle, CandidateOk), None);
    }

    // covers: BR-AG-STATE-003
    // 呈现期忽略/锚失效 → Dismissed（装饰复位）；终态冻结。
    #[test]
    fn dismiss_anchor_lost_and_terminals_frozen() {
        assert_eq!(transition(Presenting, Dismiss), Some(Dismissed));
        assert_eq!(transition(Presenting, AnchorLost), Some(Dismissed));
        for ev in [Trigger, Accept, Dismiss, CandidateOk] {
            assert_eq!(transition(Accepted, ev), None);
            assert_eq!(transition(Dismissed, ev), None);
        }
        assert!(is_terminal(Accepted) && is_terminal(Dismissed) && !is_terminal(Presenting));
    }
}
