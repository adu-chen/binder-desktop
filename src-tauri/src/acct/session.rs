//! PI-ACCT-01 · 会话生命周期承接 · CLOUD-AUTH / BR-ACCT-STATE-001
//!
//! 承接 A-ACCT-M-T-01：ACCT 按 SM-SYS-01 会话段语义承接运行时——**复用 SYS 的 session::transition，不重定义
//! 状态/转移**（一致性以 A-SYS-I-P-03 §6 为准）；登录持令牌进认证态，登出/sessionExpire 一律清令牌回未认证
//! （无残留会话）；统一错误形状 `reauth_required=true` **可靠触发重认证**（不静默吞、不自动重试绕过）；
//! 未认证/断网不影响本地功能且**不反复弹认证打扰**。令牌持有归 PI-ACCT-02（token.rs）。

use crate::ipc::IpcError;
use crate::session::{transition as sys_transition, SessionEvent, SessionState};

/// reauth_required 错误的处置。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReauthAction {
    Trigger,   // 可靠触发重认证流程
    Propagate, // 非 reauth 错误：按原错误传播（不吞、不绕过）
}

/// 当前是否认证态（由 SM-SYS-01 状态派生；未认证态不影响本地功能）。
pub fn is_authenticated(state: SessionState) -> bool {
    !matches!(state, SessionState::Unauthenticated)
}

/**
 * @GOV
 * codes:    BR-ACCT-STATE-001-RB-ACCT-CLOUD-AUTH-001
 * type:     RB
 * chain:    CLOUD-AUTH
 * rules:    BR-ACCT-STATE-001
 * boundary: in=SessionEventOnSysMachine | out=SessionState(delegatedToSysNoRedefine)
 */
/// 会话段承接（BR-ACCT-STATE-001）：account 操作经 **SM-SYS-01（SYS session::transition）** 推进——
/// 本函数仅委托，**不重定义** SYS 状态/转移（一致性归 A-SYS-I-P-03 §6）。登录→认证态、登出/过期→未认证。
pub fn apply(state: SessionState, event: SessionEvent) -> Option<SessionState> {
    sys_transition(state, event) // 委托 SYS，不另立机
}

/**
 * @GOV
 * codes:    BR-ACCT-STATE-001-GUARD-ACCT-CLOUD-AUTH-001
 * type:     GUARD
 * chain:    CLOUD-AUTH
 * rules:    BR-ACCT-STATE-001
 * boundary: in=transitionToUnauthenticated | out=mustClearToken(noResidualSession)
 */
/// 清令牌判定（BR-ACCT-STATE-001）：凡转入 Unauthenticated（登出/sessionExpire/令牌失效）须清除令牌
/// （无残留会话）。返回 `true` = 调用方须 `TokenVault::clear`。
pub fn must_clear_token(prev: SessionState, next: SessionState) -> bool {
    next == SessionState::Unauthenticated && prev != SessionState::Unauthenticated
}

/**
 * @GOV
 * codes:    BR-ACCT-STATE-001-GUARD-ACCT-CLOUD-AUTH-002
 * type:     GUARD
 * chain:    CLOUD-AUTH
 * rules:    BR-ACCT-STATE-001
 * boundary: in=IpcError | out=ReauthAction(reauthReliablyTriggeredNotSwallowedNotAutoRetried)
 */
/// reauth 处置（BR-ACCT-STATE-001）：`reauth_required=true` **可靠触发重认证**（优先于 retriable，
/// 不静默吞、不自动重试绕过）；否则按原错误传播。
pub fn reauth_action(err: &IpcError) -> ReauthAction {
    if err.reauth_required {
        ReauthAction::Trigger // 即便 retriable 也先重认证，不静默重试绕过
    } else {
        ReauthAction::Propagate
    }
}

/**
 * @GOV
 * codes:    BR-ACCT-STATE-001-GUARD-ACCT-CLOUD-AUTH-003
 * type:     GUARD
 * chain:    CLOUD-AUTH
 * rules:    BR-ACCT-STATE-001
 * boundary: in=authOnlineAndPromptHistory | out=shouldPrompt(noRepeatNoOfflineDisturb)
 */
/// 认证提示判定（BR-ACCT-STATE-001）：仅在**未认证 且 在线 且 尚未提示过**时提示；已认证、断网、
/// 或已提示过一律不提示（断网不打扰、不反复弹）。
pub fn should_prompt_auth(authenticated: bool, online: bool, already_prompted: bool) -> bool {
    !authenticated && online && !already_prompted
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{SessionEvent, SessionState};

    // covers: BR-ACCT-STATE-001
    // 承接 SM-SYS-01：登录→认证态；登出/过期→未认证，且 must_clear_token 触发清令牌（无残留）。
    #[test]
    fn delegates_to_sys_and_clears_on_unauth() {
        // 委托 SYS：登录
        let after_login = apply(SessionState::Unauthenticated, SessionEvent::Login).unwrap();
        assert_eq!(after_login, SessionState::Authenticated);
        assert!(is_authenticated(after_login));
        // 登出 → 未认证 + 须清令牌
        let after_logout = apply(SessionState::Authenticated, SessionEvent::Logout).unwrap();
        assert_eq!(after_logout, SessionState::Unauthenticated);
        assert!(must_clear_token(SessionState::Authenticated, after_logout), "登出须清令牌");
        // sessionExpire → 未认证 + 须清令牌
        let exp = apply(SessionState::Authenticated, SessionEvent::SessionExpire).unwrap();
        assert!(must_clear_token(SessionState::Authenticated, exp));
        // 未认证→未认证（LoginFail）不重复清
        assert!(!must_clear_token(SessionState::Unauthenticated, SessionState::Unauthenticated));
    }

    // covers: BR-ACCT-STATE-001
    // reauth_required 可靠触发（优先于 retriable，不静默吞/不自动重试）；非 reauth 按原错误传播。
    #[test]
    fn reauth_required_triggers_reliably() {
        let reauth = IpcError::new("X", "需重认证", true, true); // retriable 且 reauth
        assert_eq!(reauth_action(&reauth), ReauthAction::Trigger, "reauth 优先，不静默重试");
        let plain = IpcError::new("Y", "普通错误", true, false);
        assert_eq!(reauth_action(&plain), ReauthAction::Propagate);
    }

    // covers: BR-ACCT-STATE-001
    // 认证提示：仅未认证+在线+未提示过才提示；已认证/断网/已提示一律不打扰。
    #[test]
    fn prompt_only_when_needed_no_repeat_no_offline_disturb() {
        assert!(should_prompt_auth(false, true, false)); // 未认证+在线+首次
        assert!(!should_prompt_auth(true, true, false)); // 已认证不提示
        assert!(!should_prompt_auth(false, false, false)); // 断网不打扰
        assert!(!should_prompt_auth(false, true, true)); // 已提示不反复弹
    }
}
