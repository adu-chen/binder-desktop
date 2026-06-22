//! PI-TPL-* · 模板实例化/工作流执行机SM-TPL-01 · TPL-FLOW
//!
//! G4 已落：PI-TPL-01 template（workspace 隔离 + 原子实例化失败无半成品，BR-TPL-DATA-001）、
//! PI-TPL-02 workflow（SM-TPL-01 执行机：确认点强制/失败不静默跳步/中止无半成品/重启无悬挂，
//! BR-TPL-STATE-001）。承接 L2 规则源见 A-TPL-M-T-01 / A-TPL-M-S-01 / A-TPL-X-L-01。

/// PI-TPL-01 · 模板管理与原子实例化（BR-TPL-DATA-001）。
pub mod template;

/// PI-TPL-02 · 工作流执行机 SM-TPL-01（BR-TPL-STATE-001）。
pub mod workflow;
