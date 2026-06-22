//! PI-KB-* · 入库/本地检索硬边界 · KB-RETRIEVE（无状态机豁免）
//!
//! G4 已落：PI-KB-01 store（主动入库可溯源/workspace 隔离/删除即生效，BR-KB-DATA-001）、PI-KB-02
//! retrieve（本地检索无上行/存续召回/空集不注入，BR-KB-DATA-002）。承接 L2 规则源见 A-KB-M-T-01 /
//! A-KB-X-L-01。KB v1 状态机豁免（A-KB-M-T-01 §5，请求-响应式无异步生命周期）。

/// PI-KB-01 · 条目存储、组织与主动入库（BR-KB-DATA-001）。
pub mod store;

/// PI-KB-02 · 检索召回供给（本地化硬边界，BR-KB-DATA-002）。
pub mod retrieve;
