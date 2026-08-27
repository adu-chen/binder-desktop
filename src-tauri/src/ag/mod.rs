//! PI-AG-* · 回合机/门禁前置/工具/辅助/记忆触发/接管 · AG-TURN/TOOL/ASSIST / X-FORBID-006
//!
//! G4 已落（全 9 规则）：PI-AG-01/03/06 turn（SM-AG-01 + 门禁前置 ensure_gate_passed + 回合持久化，
//! BR-AG-STATE-001/VERIFY-001/PERSIST-001）、PI-AG-01 context（上下文裁剪/提示词骨架/单文档边界，
//! BR-AG-DATA-001/002/003）、PI-AG-02 tool（SM-AG-02 + 工具校验写盘禁令，BR-AG-STATE-002）、PI-AG-04
//! assist（SM-AG-03 节流/门禁/静默/采纳即编辑，BR-AG-STATE-003）、PI-AG-05 memory_link（终态异步触发
//! ast.capture 失败隔离，BR-AG-STATE-004）。承接 L2 见 A-AG-M-T-01 / A-AG-M-S-01 / A-AG-X-L-01。

/// PI-AG-01/03/06 · 代理回合机 SM-AG-01 + 门禁前置 + 回合持久化。
pub mod turn;

/// PI-AG-12 · 代理编辑历史持久化（会话标签 + 回合内容）。
pub mod history;

/// PI-AG-01（B-1/B-4）· 上下文组装 / 提示词骨架 / 单文档边界。
pub mod context;

/// PI-AG-02 · 工具调用循环机 SM-AG-02。
pub mod tool;

/// PI-AG-04 · 局部修改/辅助续写机 SM-AG-03。
pub mod assist;

/// PI-AG-05 · 记忆写入对接（终态异步触发 AST）。
pub mod memory_link;

/// AG-1.0-IMP-TASKRUN-01 · 受限任务实例编排机 SM-AG-04（TERM-AG-015/016 · BR-SYS-STATE-008）。
pub mod task;

/// AG Prompt Runtime · 动态消息装配、当前文档投影、provider 投影与最终请求审计。
pub mod prompt_runtime;

/// PI-AG-07 · 系统提示词正文（三类回合，文本属实现期运营配置）。
pub mod prompts;

/// PI-SYS-06 · BDocxCapabilityManifest 加载、投影与 Diff 前校验。
pub mod bdocx_capabilities;

/// PI-AG-08A · 工具注册表（4 工具 + 权限分级，喂 SM-AG-02 validate_tool）。
pub mod tool_registry;

/// PI-AG-08B · 工具执行（只读：list_docs/read_doc/search_text，经 WS 限 workspace）。
pub mod tool_exec;

/// PI-AG-08E/08F · 文档分块（text 段落 + docx/html 块），块列表注入 + 块级 propose_edit 重建。
pub mod blocks;

/// 富文档结构化 BlockOperation 校验与 Diff 规范化。
pub mod block_ops;

/// AgentTurn command、流式回合与状态事件编排。
pub mod turn_runtime;

/// AgentTurn 单活注册、取消控制与预览回执。
pub mod turn_control;

/// AgentTurn 和 ToolLoop 的状态事件输出。
pub mod turn_events;

/// AgentTurn 显式目标集合与引用上下文装配。
pub mod turn_reference;

/// AgentTurn PromptPackage、消息和 CapabilityGrant 装配。
pub mod turn_prompt;

/// AgentTurn 模型续轮、工具执行与终态结算。
pub mod turn_tool_loop;

/// Agent 内容修改操作解析、结构校验与 PendingDiff 投影。
pub mod edit_runtime;

/// Agent 编辑操作的纯逻辑解析与结构保护。
pub mod edit_support;

/// ImageOperation 运行时校验与物化。
pub mod edit_image_runtime;

/// TableOperation 运行时校验。
pub mod edit_table_runtime;

/// Agent 编辑运行时内部类型。
pub mod edit_types;

/// 已校验编辑操作的内容与 DiffCard 展示投影构造。
pub mod edit_projection;

#[cfg(test)]
mod edit_runtime_tests;

/// TaskRun、DecisionRequest 与 TaskPlan 运行时编排。
pub mod task_runtime;

/// Agent 会话和回合历史 command 路由。
pub mod history_routes;

/// Agent block 查询与显式引用上下文适配。
pub mod reference_runtime;

/// AG-ASSIST command 与终态编排。
pub mod assist_runtime;

/// AG-ASSIST 轻量请求消息构造。
pub mod assist_prompt;

/// Agent runtime 兼容门面；实现由上述 owner 模块承载。
pub mod runtime;
