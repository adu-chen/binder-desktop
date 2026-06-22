//! PI-ACCT-* · 会话段承接/令牌Keychain安全持有 · BR-ACCT-* / X-FORBID-001
//!
//! G4 已落（全 2 规则）：PI-ACCT-01 session（承接 SM-SYS-01 会话段不重定义 + reauth 可靠触发 + 离线
//! 不打扰，BR-ACCT-STATE-001）、PI-ACCT-02 token（SessionToken 安全持有零残留/续期失败清旧令牌/与
//! provider 密钥严格区分，BR-ACCT-DATA-001 / X-FORBID-001）。承接 L2 见 A-ACCT-M-T-01 / A-ACCT-X-L-01。

/// PI-ACCT-01 · 会话生命周期承接（BR-ACCT-STATE-001）。
pub mod session;

/// PI-ACCT-02 · 会话令牌安全持有（BR-ACCT-DATA-001）。
pub mod token;
