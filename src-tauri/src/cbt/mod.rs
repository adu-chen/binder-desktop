//! PI-CBT-* · 构建回合机SM-CBT-01/产物diff化/接管 · CBT-BUILD
//!
//! G4 已落：PI-CBT-01 build（SM-CBT-01 计划确认/失败不跳步/中断无半成品 + 控制态无悬挂，
//! BR-CBT-STATE-001 / BR-CBT-PERSIST-001）、PI-CBT-02 apply（产物 diff 化无旁路写盘，BR-CBT-DATA-001 /
//! X-FORBID-006）、PI-CBT-03 takeover（构建移交/回落无双活双死，BR-CBT-STATE-002 / X-INTER-001）。
//! 承接 L2 规则源见 A-CBT-M-T-01 / A-CBT-M-S-01 / A-CBT-X-L-01。

/// PI-CBT-01 · 构建回合机 SM-CBT-01 与任务编排（BR-CBT-STATE-001 / BR-CBT-PERSIST-001）。
pub mod build;

/// PI-CBT-02 · 产物 diff 化落盘（BR-CBT-DATA-001 / X-FORBID-006）。
pub mod apply;

/// PI-CBT-03 · 构建移交/回落协议（BR-CBT-STATE-002 / X-INTER-001）。
pub mod takeover;
