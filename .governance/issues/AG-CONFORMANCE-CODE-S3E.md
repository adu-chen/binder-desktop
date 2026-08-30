# Issue Trace — AG-CONFORMANCE-CODE-S3E：AgentTurn 装配顺序与 TaskRun 状态对齐

文档状态：A
任务类型：CODE_CHANGE
治理阶段：VALIDATION
任务状态：READY
创建时间：2026-08-29
上游基线：REQ-AG-007/015；BR-SYS-STATE-007/008；BR-AG-DATA-004；BR-AG-STATE-002；SM-AG-01/04；WP-AG10-04/05；AG-CONFORMANCE-CODE-S3C；人类指令“继续完成修复，设计实现已完成但仅测试验证滞后的统一沉淀到待验证列表”。

## 1. 用户可见症状

AgentTurn 在 PromptPackage 与预算尚未完成时已经进入 Gate/Streaming；TaskRun 状态转移表允许 WaitingConfirm 被 Finish 直接完成；A 状态计划仍保留已废止的 CreateDocument `base=empty proposal` 描述。三者造成生产状态、设计声明和治理索引不一致。

## 2. 入口点

- `src-tauri/src/ag/turn_runtime.rs::ag_start_turn`
- `src-tauri/src/ag/task.rs::transition`
- `docs/01_modules/AG/A-AG-X-L-04_代理1.0开发与验证计划.md::WP-AG10-05`

## 3. 相关规则

- BR-SYS-STATE-007
- BR-SYS-STATE-008
- BR-AG-DATA-004
- BR-AG-STATE-001
- BR-AG-STATE-002
- BR-AG-DATA-002

## 4. 相关链路

`AG-TURN → AG-TOOL`

## 5. 生产者块

- `BR-AG-STATE-001-API-AG-AG-TURN-018, BR-SYS-STATE-007-API-AG-AG-TURN-018` 位于 `src-tauri/src/ag/turn_runtime.rs`
- `BR-SYS-STATE-008-RB-AG-AG-TASK-001, BR-AG-STATE-001-RB-AG-AG-TASK-001` 位于 `src-tauri/src/ag/task.rs`
- `BR-AG-DATA-001-DATA-AG-AG-TURN-011, BR-AG-DATA-002-DATA-AG-AG-TURN-011, BR-AG-DATA-004-DATA-AG-AG-TURN-011, BR-AG-DATA-005-DATA-AG-AG-TURN-011` 位于 `src-tauri/src/ag/prompt_runtime.rs`

## 6. 消费者块

- `BR-AG-STATE-002-RB-AG-AG-TOOL-018, BR-SYS-STATE-008-RB-AG-AG-TOOL-018` 位于 `src-tauri/src/ag/turn_tool_loop.rs`
- `BR-SYS-STATE-008-EFFECT-AG-AG-TURN-026, BR-AG-DATA-003-EFFECT-AG-AG-TURN-026` 位于 `src-tauri/src/ag/task_runtime.rs`

## 7. 路径上的空规则块

未发现需要在本 Scope 修改的 `rules: []` 生产块。

## 8. boundary 假设

- `ag_start_turn` 的 PromptPackage、CapabilityGrant 和预算必须在 Assembling 完成后，才允许 `assembled → AwaitingGate`。
- `task::transition` 只承载 SM-AG-04 已登记转移，不得为 WaitingConfirm 增加未声明完成路径。
- 本次不改变 document_snapshot 或 LogicalState/DiskState 语义。

## 9. 运行时假设

- Gate 仍由 `ent::gateway::check_gate` 返回真实 GateOutcome；本次只调整调用前装配顺序，不改变 Gate 来源。
- PromptPackage 组装不调用 provider，也不产生 ToolCall 或 Proposal 副作用。
- TaskRun WaitingConfirm 的离开仍以对应 DecisionRequest 的确认或取消为唯一来源。

## 10. 疑似不变量破坏

- 移动 PromptPackage 组装时必须保持澄清零 Gate/零模型、取消生效和 TaskRun 失败结算。
- 删除 WaitingConfirm + Finish 时必须保持 `settle_turn` 对未决 DecisionRequest 的 WaitingConfirm 投影。
- 计划更新只能对齐已冻结 CreateDocumentBundle，不得改变资源创建与内容 Diff 的两段裁决语义。

## 11. 验证计划

- 补充/修正 SM-AG-04 forbidden 转移断言。
- 为 PromptPackage/预算先于 Gate 的生产顺序建立可执行断言；若需要抽取纯函数，只在既有文件内完成且不新增业务概念。
- 运行 `test:plan`、同一 changed_files 的 `test:related`、`test:governance` 与 `governance:audit`。
- 核验 AG-TURN 状态广播顺序和 SM-AG-04 转移矩阵；不把真实 Tauri/provider G5 场景伪装为自动化完成。

## Scope Lock

允许修改：`.governance/active_scope.json`、本 Issue Trace、S3C Issue Trace、`src-tauri/src/ag/context.rs`、`prompt_runtime.rs`、`tool.rs`、`tool_registry.rs`、`turn_prompt.rs`、`turn_runtime.rs`、`turn_tool_loop.rs`、`task.rs`、SM-AG-02 状态机设计、AG 1.0 开发与验证计划、Agent 1.0 验证卡、机器生成 ADUS。

禁止修改：ADU/ADP/AGENTS/CLAUDE/APC、需求与技术设计规则正文、WS/DE/ED/DOC 生产代码及所有未列入 Scope 的代码和测试。

## 人类决策

- `HD-S3C-02`（CONFIRMED）：实现已经对齐但仅环境/系统验证滞后的事项进入待验证列表，不反向修改实现。
- `HD-S3E-01`（CONFIRMED）：继续修复审计确认的实现与设计声明偏差；遇到新增设计语义或取舍项停止确认。
- `HD-S3E-02`（CONFIRMED）：SM-AG-02 新增 `loopError` 与 `handoff` 事件；多 ToolCall 按单调用顺序驱动状态；`ag::tool::transition` 成为生产状态唯一驱动器。

## 已解除 Stop Condition

- `SC-03 DESIGN_CODE_CONFLICT`：SM-AG-02 现有事件集没有唯一表达三类真实生产路径：provider/Gate/TaskPlan 等循环级错误进入 LoopAborted；Proposal/DecisionRequest 已形成后提前移交进入 LoopCompleted；一个模型轮次返回多个 ToolCall 时按批还是按单调用转移。若不先冻结事件语义，直接把 `ag::tool::transition` 接入生产会制造新的伪状态或改变混合 ToolCall 执行行为。
- 解除：人类确认 HD-S3E-02；允许同步状态机权威块、转移引擎和生产循环，并保持未获允许或校验失败的 ToolCall 不进入 ToolExecuting。

## Task Runtime Report

- task_type：`CODE_CHANGE`。
- governance_phase：`VALIDATION`。
- final_state：`READY`（实现对齐完成；不等同 Agent 1.0 G5/production_ready）。
- issue_trace：`AG-CONFORMANCE-CODE-S3E`。
- changed_files：`.governance/active_scope.json`、S3C/S3E Issue Trace、`src-tauri/src/ag/context.rs`、`prompt_runtime.rs`、`task.rs`、`tool.rs`、`tool_registry.rs`、`turn_prompt.rs`、`turn_runtime.rs`、`turn_tool_loop.rs`、SM-AG-02 状态机设计、AG 1.0 开发与验证计划、Agent 1.0 验证卡及机器生成 ADUS；均在 Scope Lock 内。
- implementation：PromptPackage、CapabilityGrant 与序列化预算检查已前移至 `Assembling`，完成后才允许 `assembled → AwaitingGate → GatePassed → Streaming`；缺失/未知 RuntimeProfile 失败关闭；`ag::tool::transition` 经唯一生产广播入口 `advance_tool_loop` 逐 ToolCall 驱动，新增 `loopError` 与 `handoff`，校验失败不进入 ToolExecuting，LoopAborted 不再结算 TaskRun Completed 或 AgentTurn Done；SM-AG-04 删除 `WaitingConfirm + Finish → Completed`；WP-AG10-05 已与 CreateDocumentBundle/WorkspaceOperationPlan 当前规则一致。
- validation：`cargo fmt --check` 与 `cargo check --lib` 通过；AG 模块 Rust 回归 164/164 通过；SM-AG-02 handoff/loopError、SM-AG-04 WaitingConfirm forbidden、RuntimeProfile、资源终态区分，以及生产装配顺序/唯一状态广播 owner 的直接测试均通过。最终 changed_files 的 `test:plan` 选择前端全量、Rust 全量及治理硬门；`test:related` 中前端 592/592、Rust 376/377，唯一失败仍为 Scope 外 `doc::libreoffice_runtime::tests::docx_to_pdf_end_to_end_when_libreoffice_available` 的 LibreOffice Worker `signal=6`。独立 `test:governance` 19/19 通过（107 个测试资产、962 个测试用例、0 未映射源码、0 skip/only）；独立 `governance:audit` 通过（311 个唯一代码块、0 governance finding）。`git diff --check` 通过。
- semantic_review：按完整规则块复核 `BR-AG-STATE-001`、`BR-AG-STATE-002`、`BR-SYS-STATE-007`、`BR-SYS-STATE-008` 与 `BR-AG-DATA-004`。`turn_runtime.rs` 的 RuntimeProfile GUARD/AgentTurn API、`tool.rs` 的转移引擎、`turn_tool_loop.rs` 的状态广播 EFFECT/生产 RB、`task.rs` 的 TaskRun 转移 RB 均与规则核心实体、事件真值、失败关闭和禁止边一致；ADUS 已注册 `BR-AG-STATE-002-EFFECT-AG-AG-TOOL-038`，未发现 `SEMANTIC_MISMATCH` 或 `NEEDS_HUMAN_DECISION`。
- stop_conditions：无；原 SC-03 已由 HD-S3E-02 解除。范围外 LibreOffice 环境失败按 HD-S3C-02 作为已完成 AG 实现的待验证事项，不扩 Scope、不修改 DOC 实现。
- unresolved_items：真实 Tauri/WKWebView + provider 的 PromptPackage→Gate→Streaming→多 ToolCall→Proposal/DecisionRequest 事件顺序仍待 G5；LibreOffice Worker `signal=6` 待 DOC 独立环境复核。两项均已进入 Agent 1.0 验证卡 §4.17 待验证列表。

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-08-29 | v1.0 | 建立 S3E Issue Trace，承接 PromptPackage/Gate、SM-AG-04 与 WP-AG10-05 的实现/声明对齐。 |
| 2026-08-29 | v1.1 | HD-S3E-02 冻结 SM-AG-02 loopError、handoff 与逐 ToolCall 顺序；生产循环改由转移引擎唯一驱动，完成自动化、影响测试、语义复核与 ADUS 注册。范围外环境失败和真实运行链统一保留待验证。 |
