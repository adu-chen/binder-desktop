//! PI-ACCT-02 · 会话令牌安全持有 · CLOUD-AUTH / BR-ACCT-DATA-001 / X-FORBID-001
//!
//! 承接 A-ACCT-M-T-01：SessionToken 经**系统级安全存储**持有（macOS 优先 Keychain）——不明文落盘（配置/
//! workspace.db/缓存）、不写日志/错误上报、不经 IPC 以外通道暴露；续期按云契约，**续期失败转未认证并清除
//! 旧令牌**；登出/过期后**本地无令牌残留**。令牌与 provider 密钥严格区分：前者可持有，后者受 X-FORBID-001
//! 客户端**永不持有**（本模块无任何 provider 密钥类型/API）。Keychain 后端为实现期接入（ACCT-02，A 类），
//! 本类型以内存持有建模安全存储语义，结构上不触 db/fs/log。

/// 会话令牌（newtype，与 provider 密钥类型层面严格区分；本模块不存在 provider 密钥类型）。
#[derive(Clone, PartialEq, Eq)]
pub struct SessionToken(String);

impl SessionToken {
    pub fn new(raw: impl Into<String>) -> Self {
        SessionToken(raw.into())
    }
    /// 仅供经 IPC 提交服务端裁决使用（不回显、不落日志）。
    pub fn expose(&self) -> &str {
        &self.0
    }
}

// 不回显明文：Debug 脱敏（不写日志/错误上报泄漏令牌）。
impl std::fmt::Debug for SessionToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SessionToken(***)")
    }
}

/// 安全存储持有（建模 Keychain 后端：实现期换 macOS Keychain，语义不变）。
#[derive(Debug, Default)]
pub struct TokenVault {
    held: Option<SessionToken>,
}

impl TokenVault {
    pub fn new() -> Self {
        TokenVault { held: None }
    }

    pub fn is_held(&self) -> bool {
        self.held.is_some()
    }

    /**
     * @GOV
     * codes:    BR-ACCT-DATA-001-EFFECT-ACCT-CLOUD-AUTH-001
     * type:     EFFECT
     * chain:    CLOUD-AUTH
     * rules:    BR-ACCT-DATA-001
     * boundary: in=SessionToken | out=securelyHeldToken(keychainNoPlaintextDiskNoLog)
     */
    /// 持有令牌（BR-ACCT-DATA-001）：存入系统级安全存储（Keychain）。结构上不写 db/fs/log（不明文落盘
    /// /不写日志），不经 IPC 外通道暴露。覆盖旧值（登录/续期）。
    pub fn store(&mut self, token: SessionToken) {
        self.held = Some(token);
    }

    /**
     * @GOV
     * codes:    BR-ACCT-DATA-001-EFFECT-ACCT-CLOUD-AUTH-002
     * type:     EFFECT
     * chain:    CLOUD-AUTH
     * rules:    BR-ACCT-DATA-001
     * boundary: in=logoutOrExpire | out=clearedVault(noTokenResidue)
     */
    /// 清除令牌（BR-ACCT-DATA-001）：登出/过期后**本地无令牌残留**——从安全存储移除并 drop 旧令牌。
    pub fn clear(&mut self) {
        self.held = None; // 旧令牌 drop，无残留
    }

    /**
     * @GOV
     * codes:    BR-ACCT-DATA-001-EFFECT-ACCT-CLOUD-AUTH-003
     * type:     EFFECT
     * chain:    CLOUD-AUTH
     * rules:    BR-ACCT-DATA-001
     * boundary: in=renewResult | out=renewedOrClearedToUnauth(failureClearsOldToken)
     */
    /// 续期（BR-ACCT-DATA-001）：成功→替换令牌；**失败→清除旧令牌并回报需转未认证**（不保留失效令牌）。
    /// 返回 `true` = 续期成功仍认证；`false` = 失败已清旧令牌，调用方须转未认证（承 STATE-001）。
    pub fn renew(&mut self, result: Result<SessionToken, ()>) -> bool {
        match result {
            Ok(fresh) => {
                self.held = Some(fresh);
                true
            }
            Err(()) => {
                self.clear(); // 续期失败清旧令牌（无残留）
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // covers: BR-ACCT-DATA-001
    // 持有/清除：store 后 is_held；clear 后无残留；Debug 脱敏不回显明文。
    #[test]
    fn store_and_clear_leaves_no_residue() {
        let mut v = TokenVault::new();
        assert!(!v.is_held());
        v.store(SessionToken::new("session-abc"));
        assert!(v.is_held());
        v.clear();
        assert!(!v.is_held(), "登出/过期后无令牌残留");
        // 不回显明文（脱敏 Debug）
        assert_eq!(
            format!("{:?}", SessionToken::new("secret")),
            "SessionToken(***)"
        );
    }

    // covers: BR-ACCT-DATA-001
    // 续期：成功替换仍认证；失败清旧令牌并回报转未认证（不保留失效令牌）。
    #[test]
    fn renew_success_replaces_failure_clears() {
        let mut v = TokenVault::new();
        v.store(SessionToken::new("old"));
        assert!(v.renew(Ok(SessionToken::new("new"))));
        assert!(v.is_held());
        // 续期失败：清旧令牌、转未认证
        assert!(!v.renew(Err(())));
        assert!(!v.is_held(), "续期失败须清旧令牌（无残留）");
    }
}
