//! PI-AST-* · 记忆写入机SM-AST-01/隔离/供给 · AST-MEMORY（workspace级隔离）
//!
//! G4 已落（全 3 规则）：PI-AST-01 store（workspace 级隔离查询，BR-AST-DATA-001）、PI-AST-02 capture
//! （自动写入机 SM-AST-01 + 撤销不可复活，BR-AST-STATE-001）、PI-AST-03 supply（记忆注入供给：active+
//! 分级/空集不注入/可溯源，BR-AST-DATA-002）。承接 L2 见 A-AST-M-T-01 / A-AST-M-S-01 / A-AST-X-L-01。

/// PI-AST-01 · 记忆条目存储与 workspace 级隔离（BR-AST-DATA-001）。
pub mod store;

/// PI-AST-02 · 自动写入机 SM-AST-01 与类型分级（BR-AST-STATE-001）。
pub mod capture;

/// PI-AST-03 · 记忆注入供给（BR-AST-DATA-002）。
pub mod supply;
