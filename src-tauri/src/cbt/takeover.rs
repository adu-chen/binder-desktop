//! PI-CBT-03 · Chat↔Build 接管协议 · CBT-BUILD / BR-CBT-STATE-002 / X-INTER-001
//!
//! 承接 A-CBT-M-T-01 §4：chat 回合（SM-AG-01）识别多文件构建意图后**经用户显式确认**方接管进入构建
//! （SM-CBT-01）；接管/回落均为事件+回执双向确认（takeover/takeoverAck、fallback/fallbackAck），两侧状态
//! 一致——**不得双活**（chat 与构建同时持有回合）或**双死**（移交中丢失回合）。本机以单值状态枚举结构性
//! 保证恒有唯一所有者：移交中（HandingTo*）所有权仍归发起方直到 ack 才原子转移，故任一时刻非双活非双死。
//! AG 侧 takeover/fallback 对接 PI-AG-06（B-3），本机承接 CBT 侧协议转移；ack 信号来自对侧（X-INTER-001）。

/// 接管协议状态。HandingToBuild/HandingToChat 为移交中（仍单一所有者）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandoffState {
    ChatActive,
    HandingToBuild,
    BuildActive,
    HandingToChat,
}

/// 回合逻辑所有者（恒为单一方——结构性排除双活/双死）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnOwner {
    Chat,
    Build,
}

/// 接管协议事件。TakeoverInit 携带用户是否显式确认。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandoffEvent {
    TakeoverInit { user_confirmed: bool },
    TakeoverAck,
    FallbackInit,
    FallbackAck,
}

/**
 * @GOV
 * codes:    BR-CBT-STATE-002-GUARD-CBT-CBT-BUILD-001
 * type:     GUARD
 * chain:    CBT-BUILD
 * rules:    BR-CBT-STATE-002
 * boundary: in=HandoffState | out=singleTurnOwner(neverDualActiveNeverDualDead)
 */
/// 回合所有者（BR-CBT-STATE-002）：每个状态映射**恰好一个**所有者——移交中所有权归发起方直到 ack，
/// 故任一时刻非双活（不会两侧皆持有）非双死（不会无人持有）。此全函数即「无双活双死」的结构性证据。
pub fn active_owner(state: HandoffState) -> TurnOwner {
    match state {
        // chat 持有；移交给 build 期间仍由 chat 持有直到 ack（不双死）。
        HandoffState::ChatActive | HandoffState::HandingToBuild => TurnOwner::Chat,
        // build 持有；回落 chat 期间仍由 build 持有直到 ack（不双死）。
        HandoffState::BuildActive | HandoffState::HandingToChat => TurnOwner::Build,
    }
}

/**
 * @GOV
 * codes:    BR-CBT-STATE-002-RB-CBT-CBT-BUILD-001
 * type:     RB
 * chain:    CBT-BUILD
 * rules:    BR-CBT-STATE-002
 * boundary: in=HandoffEvent | out=HandoffState(confirmGatedAckHandoffNoDualActiveNoDualDead)
 */
/// 接管/回落转移引擎（BR-CBT-STATE-002）：takeover 须**用户显式确认**方发起；所有权仅在收到对侧
/// **ack** 时原子转移（双向回执）——故移交全程恒单一所有者，不双活不双死。返回 `None` = 无合法转移
/// （含 forbidden：未确认不得接管、无待移交时收 ack、重复接管/回落）。
pub fn transition(state: HandoffState, event: HandoffEvent) -> Option<HandoffState> {
    use HandoffEvent::*;
    use HandoffState::*;
    let next = match (state, event) {
        // chat→build：须用户确认方发起移交（仍 chat 持有）。
        (ChatActive, TakeoverInit { user_confirmed: true }) => HandingToBuild,
        // 收到 build 侧 ack：原子转移所有权给 build。
        (HandingToBuild, TakeoverAck) => BuildActive,
        // build→chat：回落发起（仍 build 持有）。
        (BuildActive, FallbackInit) => HandingToChat,
        // 收到 chat 侧 ack：原子转移回 chat。
        (HandingToChat, FallbackAck) => ChatActive,
        // 其余：无合法转移（未确认不得接管；无待移交时 ack 无效；移交中不得重复发起）。
        _ => return None,
    };
    Some(next)
}

#[cfg(test)]
mod tests {
    use super::HandoffEvent::*;
    use super::HandoffState::*;
    use super::*;

    // covers: BR-CBT-STATE-002
    // 接管须用户显式确认；所有权仅在 ack 时原子转移（双向回执）；回落对称。
    #[test]
    fn handoff_requires_confirm_and_ack() {
        // 未确认不得接管
        assert_eq!(transition(ChatActive, TakeoverInit { user_confirmed: false }), None);
        // 确认后发起移交（仍 chat 持有），ack 后转 build
        assert_eq!(transition(ChatActive, TakeoverInit { user_confirmed: true }), Some(HandingToBuild));
        assert_eq!(transition(HandingToBuild, TakeoverAck), Some(BuildActive));
        // 回落：发起（仍 build 持有），ack 后回 chat
        assert_eq!(transition(BuildActive, FallbackInit), Some(HandingToChat));
        assert_eq!(transition(HandingToChat, FallbackAck), Some(ChatActive));
        // forbidden：无待移交时收 ack、移交中重复发起
        assert_eq!(transition(ChatActive, TakeoverAck), None, "无待移交不得 ack");
        assert_eq!(transition(HandingToBuild, TakeoverInit { user_confirmed: true }), None);
        assert_eq!(transition(BuildActive, TakeoverAck), None);
    }

    // covers: BR-CBT-STATE-002
    // 无双活双死：每个状态恒有唯一所有者；移交中所有权归发起方（既非两侧皆持有，亦非无人持有）。
    #[test]
    fn never_dual_active_nor_dual_dead() {
        // active_owner 为全函数：四状态各映射恰好一个所有者
        assert_eq!(active_owner(ChatActive), TurnOwner::Chat);
        assert_eq!(active_owner(HandingToBuild), TurnOwner::Chat); // 移交中仍 chat（不双死）
        assert_eq!(active_owner(BuildActive), TurnOwner::Build);
        assert_eq!(active_owner(HandingToChat), TurnOwner::Build); // 回落中仍 build（不双死）
        // 沿合法路径走一圈，每一步都恰有一个所有者（不双活）
        let mut s = ChatActive;
        for ev in [
            TakeoverInit { user_confirmed: true },
            TakeoverAck,
            FallbackInit,
            FallbackAck,
        ] {
            let _owner = active_owner(s); // 总有定义（编译期穷尽）
            s = transition(s, ev).expect("合法路径应可转移");
        }
        assert_eq!(s, ChatActive); // 回到起点，闭环一致
    }
}
