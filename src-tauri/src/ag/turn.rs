//! 代理回合机 SM-AG-01 与门禁前置 · AG-TURN
//! 规则：BR-AG-STATE-001（回合生命周期）/ BR-AG-VERIFY-001（门禁前置无绕过）
//!
//! 承接 A-AG-M-S-01 SM-AG-01：回合经 组装→门禁→流式→产出处置 完整生命周期；**通往模型调用（Streaming）
//! 的唯一路径是 AwaitingGate + gatePassed**（门禁前置，结构上无绕过路径，BR-AG-VERIFY-001 / X-SEQ-001）；
//! gateDenied/cancel 为终态不可绕过且无半落盘副作用；含修改建议的产出经 HandingOff 交 DE（X-FORBID-006，
//! AG 无直写）。Agent 1.0 不包含 Build/CBT 接管或回落状态；回合持久化归 `ag::history`。

use crate::ipc::IpcError;
use std::sync::atomic::{AtomicU8, Ordering};

/// Binder AG 1.0 唯一开放的主回合运行配置。
pub const AGENT_EDIT_1_0_PROFILE: &str = "agent-edit-1.0";

/// 校验主回合运行配置。未知配置默认拒绝，避免未验证能力落入 1.0 主链。
pub fn ensure_runtime_profile(profile: &str) -> Result<(), IpcError> {
    if profile == AGENT_EDIT_1_0_PROFILE {
        Ok(())
    } else {
        Err(IpcError::new(
            "AG_RUNTIME_PROFILE_UNSUPPORTED",
            &format!("不支持的 Agent Runtime Profile: {profile}"),
            false,
            false,
        ))
    }
}

/// 回合 wire 终态。值固定用于原子锁存，不承载展示文案。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TerminalKind {
    Completed = 1,
    Cancelled = 2,
    Denied = 3,
    Error = 4,
}

/// 首个终态获胜的原子锁存器；后续终态及其副作用必须被拒绝。
#[derive(Debug, Default)]
pub struct TerminalLatch(AtomicU8);

impl TerminalLatch {
    pub fn try_set(&self, terminal: TerminalKind) -> bool {
        self.0
            .compare_exchange(0, terminal as u8, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    pub fn is_set(&self) -> bool {
        self.0.load(Ordering::Acquire) != 0
    }
}

/// SM-AG-01 状态（A-AG-M-S-01 §2）。TurnError/Completed/Cancelled/Denied 为终态；
/// AwaitingClarification 为等待用户输入的非终态（BR-SYS-STATE-007，澄清不终结回合）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnState {
    Idle,
    Assembling,
    AwaitingGate,
    Streaming,
    HandingOff,
    /// 澄清等待：IntentResult.ambiguity=blocking 时等待用户输入的非终态（BR-SYS-STATE-007）。
    AwaitingClarification,
    TurnError,
    Completed,
    Cancelled,
    Denied,
}

/// SM-AG-01 事件。重试会创建新的 AgentTurn，不是本状态机内事件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnEvent {
    Send,
    Assembled,
    ClarificationRequired,
    ClarificationProvided,
    AssembleFail,
    GatePassed,
    GateDenied,
    StreamEnd,
    StreamFail,
    Cancel,
    HandoffOk,
    HandoffFail,
}

/// 门禁裁决结果（AG 承接 SM-SYS-01 GatePassed/GateDenied；Offline=断网）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateState {
    Passed,
    Denied,
    Offline,
}

/// 终态判定。
pub fn is_terminal(state: TurnState) -> bool {
    matches!(
        state,
        TurnState::TurnError | TurnState::Completed | TurnState::Cancelled | TurnState::Denied
    )
}

/**
 * @GOV
 * codes:    BR-AG-STATE-001-RB-AG-AG-TURN-001, BR-SYS-STATE-007-RB-AG-AG-TURN-001
 * type:     RB
 * chain:    AG-TURN
 * rules:    BR-AG-STATE-001, BR-SYS-STATE-007
 * boundary: in=TurnEvent | out=TurnState(clarificationRequiredWaitsAndOnlyClarificationProvidedResumesSameAgentTurnGateGatedToStreaming)
 */
/// SM-AG-01 转移引擎（BR-AG-STATE-001 / BR-SYS-STATE-007）：**Streaming（模型调用）仅经
/// AwaitingGate+gatePassed 可达**（门禁前置无绕过）；gateDenied→Denied 终态不可绕过；cancel→Cancelled
/// 无半落盘；handoffOk→Completed（交 DE 完毕）；**澄清
/// （ClarificationRequired）→AwaitingClarification 非终态，不终结回合；仅 ClarificationProvided 可使
/// 同一 AgentTurn 回到 Assembling**。返回 `None` = 无合法转移（含全部 forbidden）。
pub fn transition(state: TurnState, event: TurnEvent) -> Option<TurnState> {
    use TurnEvent::*;
    use TurnState::*;
    let next = match (state, event) {
        (Idle, Send) => Assembling,
        // 澄清是等待用户输入的非终态（BR-SYS-STATE-007），不终结回合。
        (Assembling, ClarificationRequired) => AwaitingClarification,
        // 用户补充说明只恢复原 AgentTurn；不是 Retry，也不创建新回合身份。
        (AwaitingClarification, ClarificationProvided) => Assembling,
        (Assembling, Assembled) => AwaitingGate,
        (Assembling, AssembleFail) => TurnError,
        // 门禁前置：唯一通往 Streaming 的路径。
        (AwaitingGate, GatePassed) => Streaming,
        (AwaitingGate, GateDenied) => Denied,
        (Streaming, StreamEnd) => HandingOff,
        (Streaming, StreamFail) => TurnError,
        // 任一活动阶段取消都必须到达唯一 Cancelled 终态。
        (Assembling | AwaitingClarification | AwaitingGate | Streaming | HandingOff, Cancel) => {
            Cancelled
        }
        (HandingOff, HandoffOk) => Completed,
        (HandingOff, HandoffFail) => TurnError,
        // 其余无合法转移（终态冻结；无 gatePassed 不得 Streaming）。
        _ => return None,
    };
    Some(next)
}

/**
 * @GOV
 * codes:    BR-AG-VERIFY-001-GUARD-AG-AG-TURN-020
 * type:     GUARD
 * chain:    AG-TURN
 * rules:    BR-AG-VERIFY-001
 * boundary: in=GateState | out=invocationPermit(deniedOrOfflineRejectedUnifiedError)
 */
/// 门禁前置守卫（BR-AG-VERIFY-001 / X-SEQ-001）：每次模型调用（chat/工具续调/辅助）须先经 SM-SYS-01
/// 达 GatePassed。Passed→Ok（许可发起）；Denied/Offline→Err 统一错误形状（代理/工具回合明确拒绝；
/// 局部修改由 SM-AG-03 吞此 Err 静默不提供）。本守卫与 SM-AG-01 的结构性门禁路径双重把守，无绕过路径。
pub fn ensure_gate_passed(gate: GateState) -> Result<(), IpcError> {
    match gate {
        GateState::Passed => Ok(()),
        GateState::Denied => Err(IpcError::new(
            "AG_GATE_DENIED",
            "门禁拒绝：无有效订阅或权限不足，本次调用被拒",
            false,
            true,
        )),
        GateState::Offline => Err(IpcError::new(
            "AG_GATE_OFFLINE",
            "断网：无法经网关发起模型调用",
            true,
            false,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::TurnEvent::*;
    use super::TurnState::*;
    use super::*;

    // covers: BR-AG-STATE-001
    // 正路 + 门禁前置：Streaming 仅经 AwaitingGate+gatePassed 可达（无绕过）；gateDenied/cancel 终态。
    #[test]
    fn lifecycle_streaming_only_reachable_through_gate() {
        assert_eq!(transition(Idle, Send), Some(Assembling));
        assert_eq!(transition(Assembling, Assembled), Some(AwaitingGate));
        assert_eq!(
            transition(Assembling, ClarificationRequired),
            Some(AwaitingClarification)
        );
        assert_eq!(transition(AwaitingGate, GatePassed), Some(Streaming));
        assert_eq!(transition(Streaming, StreamEnd), Some(HandingOff));
        assert_eq!(transition(HandingOff, HandoffOk), Some(Completed));
        // 门禁拒绝 → Denied 终态不可绕过
        assert_eq!(transition(AwaitingGate, GateDenied), Some(Denied));
        // 结构性门禁前置：除 AwaitingGate+gatePassed 外无任何状态可达 Streaming
        for from in [
            Idle,
            Assembling,
            AwaitingClarification,
            Streaming,
            HandingOff,
            TurnError,
            Completed,
            Denied,
        ] {
            assert_eq!(
                transition(from, GatePassed),
                None,
                "{from:?} 不应可达 Streaming"
            );
        }
    }

    // covers: BR-AG-STATE-001
    // covers: BR-SYS-STATE-007
    // 澄清是等待用户输入的非终态：仅 ClarificationProvided 恢复同一回合；取消仍可到达终态。
    #[test]
    fn clarification_is_non_terminal_and_cancelable() {
        // 澄清 → AwaitingClarification（非终态）
        let s = transition(Assembling, ClarificationRequired).unwrap();
        assert_eq!(s, AwaitingClarification);
        assert!(!is_terminal(s), "AwaitingClarification 不得判定为终态");
        assert_eq!(
            transition(AwaitingClarification, ClarificationProvided),
            Some(Assembling),
            "补充说明必须恢复同一 AgentTurn"
        );
        // 澄清等待阶段取消 → Cancelled（任一活动阶段取消生效）
        assert_eq!(transition(AwaitingClarification, Cancel), Some(Cancelled));
        // 澄清不产生模型调用/产出：澄清态无通往 Streaming/HandingOff 的转移
        assert_eq!(transition(AwaitingClarification, GatePassed), None);
        assert_eq!(transition(AwaitingClarification, StreamEnd), None);
        assert_eq!(transition(AwaitingClarification, HandoffOk), None);
        assert_eq!(transition(TurnError, ClarificationProvided), None);
        assert_eq!(transition(Completed, ClarificationProvided), None);
    }

    // covers: BR-AG-STATE-001
    // 取消无半落盘终态；TurnError 和其余终态冻结，重试须创建新回合。
    #[test]
    fn cancel_error_and_terminals_are_frozen() {
        for active in [
            Assembling,
            AwaitingClarification,
            AwaitingGate,
            Streaming,
            HandingOff,
        ] {
            assert_eq!(transition(active, Cancel), Some(Cancelled));
        }
        assert_eq!(transition(Streaming, StreamFail), Some(TurnError));
        for ev in [Send, GatePassed, StreamEnd, HandoffOk, Cancel] {
            assert_eq!(transition(Completed, ev), None);
            assert_eq!(transition(Cancelled, ev), None);
            assert_eq!(transition(Denied, ev), None);
        }
        assert!(is_terminal(Denied) && is_terminal(TurnError) && !is_terminal(Streaming));
    }

    #[test]
    fn runtime_profile_and_terminal_latch_fail_closed() {
        assert!(ensure_runtime_profile(AGENT_EDIT_1_0_PROFILE).is_ok());
        assert_eq!(
            ensure_runtime_profile("build-1.0").unwrap_err().code,
            "AG_RUNTIME_PROFILE_UNSUPPORTED"
        );

        let latch = TerminalLatch::default();
        assert!(latch.try_set(TerminalKind::Error));
        assert!(latch.is_set());
        assert!(!latch.try_set(TerminalKind::Completed));
        assert!(!latch.try_set(TerminalKind::Cancelled));
    }

    // covers: BR-AG-VERIFY-001
    // 门禁前置守卫：Passed 许可；Denied/Offline 拒绝（统一错误形状，含 reauth/retriable 标记）。
    #[test]
    fn gate_guard_permits_only_when_passed() {
        assert!(ensure_gate_passed(GateState::Passed).is_ok());
        let denied = ensure_gate_passed(GateState::Denied).unwrap_err();
        assert_eq!(denied.code, "AG_GATE_DENIED");
        assert!(denied.reauth_required);
        let offline = ensure_gate_passed(GateState::Offline).unwrap_err();
        assert_eq!(offline.code, "AG_GATE_OFFLINE");
        assert!(offline.retriable);
    }
}
