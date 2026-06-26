//! PI-AG-* · 回合机/门禁前置/工具/辅助/记忆触发/接管 · AG-TURN/TOOL/ASSIST / X-FORBID-006
//!
//! G4 已落（全 9 规则）：PI-AG-01/03/06 turn（SM-AG-01 + 门禁前置 ensure_gate_passed + 回合持久化，
//! BR-AG-STATE-001/VERIFY-001/PERSIST-001）、PI-AG-01 context（上下文裁剪/提示词骨架/单文档边界，
//! BR-AG-DATA-001/002/003）、PI-AG-02 tool（SM-AG-02 + 工具校验写盘禁令，BR-AG-STATE-002）、PI-AG-04
//! assist（SM-AG-03 节流/门禁/静默/采纳即编辑，BR-AG-STATE-003）、PI-AG-05 memory_link（终态异步触发
//! ast.capture 失败隔离，BR-AG-STATE-004）。承接 L2 见 A-AG-M-T-01 / A-AG-M-S-01 / A-AG-X-L-01。

/// PI-AG-01/03/06 · 代理回合机 SM-AG-01 + 门禁前置 + 回合持久化。
pub mod turn;

/// PI-AG-01（B-1/B-4）· 上下文组装 / 提示词骨架 / 单文档边界。
pub mod context;

/// PI-AG-02 · 工具调用循环机 SM-AG-02。
pub mod tool;

/// PI-AG-04 · 内联辅助/自动补全机 SM-AG-03。
pub mod assist;

/// PI-AG-05 · 记忆写入对接（终态异步触发 AST）。
pub mod memory_link;

/// PI-AG-07 · 系统提示词正文（三类回合，文本属实现期运营配置）。
pub mod prompts;
