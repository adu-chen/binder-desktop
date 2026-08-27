//! PI-SYS-04 · 会话与订阅门禁状态机 SM-SYS-01 · CLOUD-AUTH / CLOUD-GATE / CLOUD-INVOKE
//!
//! 承接 A-SYS-I-P-03 §6 SM-SYS-01：客户端「登录态维持 + 单次模型调用门控」生命周期。
//! 本模块实现纯客户端状态机骨架与转移守护；服务端裁决（gateAllow/gateDeny）为**结果事件**，
//! 客户端不自行裁决（X-FORBID-002），密钥不落地（X-FORBID-001），调用前必过门禁（X-SEQ-001）——
//! 三约束由转移表「无 gateAllow 不存在通往 Invoking 的路径」天然保证。
//! BR-SYS-OBS-001（云错误形状）/ BR-SYS-DATA-004（网关不留存）主责在网关实现（PI-ENT），不在此。

use serde::{Deserialize, Serialize};

/// SM-SYS-01 状态（A-SYS-I-P-03 §6）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionState {
    Unauthenticated,
    Authenticated,
    GateChecking,
    GatePassed,
    Invoking,
    InvokeCompleted,
    GateDenied,
    InvokeError,
}

/// SM-SYS-01 事件。`StartModelCall` 携带令牌有效性 guard；gateAllow/gateDeny 为服务端裁决结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionEvent {
    Login,
    LoginFail,
    StartModelCall { token_valid: bool },
    GateAllow,
    GateDeny,
    InvokeStart,
    InvokeStreamEnd,
    InvokeFail,
    SessionExpire,
    Logout,
}

/**
 * @GOV
 * codes:    BR-SYS-STATE-001-RB-SYS-CLOUD-AUTH-001
 * type:     RB
 * chain:    CLOUD-AUTH
 * rules:    BR-SYS-STATE-001
 * boundary: in=SessionEvent | out=SessionState(noneWhenForbidden)
 */
/// 状态转移引擎（BR-SYS-STATE-001）：会话令牌生命周期——登录持令牌、过期/登出回未认证、
/// 未认证态一切调用被拒。返回 `None` 表示该 (state,event) 无合法转移（含 forbidden_transitions：
/// 未认证/未过门禁/被拒态均无通往 Invoking 的路径）。
pub fn transition(state: SessionState, event: SessionEvent) -> Option<SessionState> {
    use SessionEvent::*;
    use SessionState::*;
    let next = match (state, event) {
        (Unauthenticated, Login) => Authenticated,
        (Unauthenticated, LoginFail) => Unauthenticated,
        (Authenticated, StartModelCall { token_valid: true }) => GateChecking,
        // 令牌已过期：不进门禁，要求重认证（回未认证态）。
        (Authenticated, StartModelCall { token_valid: false }) => Unauthenticated,
        (GateChecking, GateAllow) => GatePassed,
        (GateChecking, GateDeny) => GateDenied,
        (GatePassed, InvokeStart) => Invoking,
        (Invoking, InvokeStreamEnd) => InvokeCompleted,
        (Invoking, InvokeFail) => InvokeError,
        // sessionExpire 任意活动态回未认证（recovery 重认证）。
        (Authenticated, SessionExpire)
        | (GateChecking, SessionExpire)
        | (Invoking, SessionExpire) => Unauthenticated,
        (Authenticated, Logout) => Unauthenticated,
        // 其余 (state,event)：无合法转移（含全部 forbidden_transitions）。
        _ => return None,
    };
    Some(next)
}

/**
 * @GOV
 * codes:    BR-SYS-VERIFY-001-GUARD-SYS-CLOUD-GATE-001, BR-SYS-DATA-001-GUARD-SYS-CLOUD-GATE-001
 * type:     GUARD
 * chain:    CLOUD-GATE, CLOUD-INVOKE
 * rules:    BR-SYS-VERIFY-001, BR-SYS-DATA-001
 * boundary: in=SessionState | out=invokeAuthorization(onlyGatePassed)
 */
/// 调用授权守卫（BR-SYS-VERIFY-001 门禁硬门 + BR-SYS-DATA-001 网关调用前置）：
/// 唯有 `GatePassed` 态可发起模型调用——无服务端 gateAllow 裁决（GateChecking/GateDenied）
/// 或未认证/未过门禁态一律不可调用，无客户端绕过路径。
pub fn can_invoke(state: SessionState) -> bool {
    matches!(state, SessionState::GatePassed)
}

#[cfg(test)]
mod tests {
    use super::SessionEvent::*;
    use super::SessionState::*;
    use super::*;

    // covers: BR-SYS-STATE-001
    // 会话生命周期正路 + 登录/登出/过期转移；未认证态一切模型调用被拒（forbidden）。
    #[test]
    fn session_lifecycle_and_unauthenticated_rejects_invoke() {
        // 正路至完成
        let s = transition(Unauthenticated, Login).unwrap();
        assert_eq!(s, Authenticated);
        let s = transition(s, StartModelCall { token_valid: true }).unwrap();
        assert_eq!(s, GateChecking);
        let s = transition(s, GateAllow).unwrap();
        assert_eq!(s, GatePassed);
        let s = transition(s, InvokeStart).unwrap();
        assert_eq!(s, Invoking);
        let s = transition(s, InvokeStreamEnd).unwrap();
        assert_eq!(s, InvokeCompleted);

        // 登出/过期回未认证
        assert_eq!(transition(Authenticated, Logout), Some(Unauthenticated));
        assert_eq!(transition(Invoking, SessionExpire), Some(Unauthenticated));
        // 令牌过期不进门禁
        assert_eq!(
            transition(Authenticated, StartModelCall { token_valid: false }),
            Some(Unauthenticated)
        );
        // 未认证态发起调用：无合法转移
        assert_eq!(transition(Unauthenticated, InvokeStart), None);
        assert_eq!(transition(Unauthenticated, GateAllow), None);
    }

    // covers: BR-SYS-VERIFY-001
    // 门禁硬门强制矩阵：无 gateAllow 不存在通往 Invoking 的路径；GateDenied 终态不可绕过。
    #[test]
    fn gate_is_hard_gate_no_bypass_to_invoking() {
        // gateDeny → GateDenied 终态
        assert_eq!(transition(GateChecking, GateDeny), Some(GateDenied));
        // forbidden_transitions：均不可达 Invoking
        for from in [Unauthenticated, Authenticated, GateChecking, GateDenied] {
            assert_eq!(
                transition(from, InvokeStart),
                None,
                "{from:?} 不得绕过门禁直达 Invoking"
            );
        }
        // GateDenied 是终态，任何事件不复活为可调用态
        for ev in [InvokeStart, GateAllow, Login] {
            assert_eq!(transition(GateDenied, ev), None);
        }
    }

    // covers: BR-SYS-DATA-001
    // 网关调用前置：仅 GatePassed 态授权调用，其余一律拒绝。
    #[test]
    fn only_gate_passed_authorizes_invoke() {
        assert!(can_invoke(GatePassed));
        for s in [
            Unauthenticated,
            Authenticated,
            GateChecking,
            GateDenied,
            Invoking,
            InvokeCompleted,
            InvokeError,
        ] {
            assert!(!can_invoke(s), "{s:?} 不应授权模型调用");
        }
    }
}
