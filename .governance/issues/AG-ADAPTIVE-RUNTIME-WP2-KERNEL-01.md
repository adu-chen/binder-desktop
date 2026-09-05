# Issue Trace — AG-ADAPTIVE-RUNTIME-WP2-KERNEL-01：Adaptive Turn Kernel 收敛

文档状态：A
任务类型：CODE_CHANGE
治理阶段：VALIDATION
创建时间：2026-09-04

## 1. 用户可见症状

Agent 对自然语言的反应受关键词分类和预设工具路线限制；工具返回新事实后仍复用首次 PromptPackage 与
CapabilityGrant；DecisionRequest 会把当前回合伪装为完成并由前端创建新回合自动续跑。结果表现为交互呆板、
泛化不足、路线不能随事实调整，且等待态、完成态与真实执行状态不一致。

## 2. 入口点

- `src-tauri/src/ag/turn_runtime.rs`：`ag_start_turn` 请求装配与首次 Gate。
- `src-tauri/src/ag/turn_tool_loop.rs`：provider/ToolCall/ToolResult 循环。
- `src/services/agService.ts`：澄清、裁决等待和恢复的前端 Channel 编排。

## 3. 相关规则

- BR-AG-STATE-001、BR-AG-STATE-002、BR-AG-DATA-002、BR-AG-DATA-004、BR-AG-DATA-005、
  BR-AG-VERIFY-001、BR-SYS-STATE-007、BR-SYS-STATE-008。
- X-SEQ-001、X-FORBID-006。

## 4. 相关链路

- AG-TURN：AgentTurn 组装、逐次 Gate、模型输出、等待和唯一终态。
- AG-TOOL：ToolCall 校验、执行、ToolResult 回注和回到 Assembling。

## 5. 生产者块

- `ag_start_turn` 创建/恢复 AgentTurn、冻结 AgentContextFacts。
- provider 原生文本/ToolCall 输出产生模型语义与动作。
- `propose_edit`、`propose_decision`、`plan_workspace_operation` 产生 Proposal/Decision 事实。

## 6. 消费者块

- `TurnEventEmitter` 持久化并投影 IntentUpdated、RouteUpdated、ToolResult、DiffProposed、
  DecisionRequestProposed、ClarificationRequired 与唯一终态。
- `agService`/`agStore` 按同一 turn_id 呈现文本、工具、等待和恢复。
- DE/WS 继续承载修改建议与资源操作的既有受控边界。

## 7. 路径上的空规则块

现有主入口、状态机、Prompt 装配、工具循环和前端投影均已有规则映射；本工作包不新增规则。新增的模型输出
解析 owner 必须映射 BR-AG-DATA-004、BR-AG-STATE-001；所有修改后的生产边界保持原链路映射。

## 8. boundary 假设

- 模型语义使用内部版本化结构输出，不展示隐藏推理，也不成为授权对象。
- 初始 IntentResult 为 unresolved；没有模型语义时不以关键词授予工具。
- 每次 ToolResult 后回到 Assembling，重建 PromptPackage，再取得一次真实 GatePassed 才能继续 provider。
- Proposal 只证明待审建议已生成；DecisionRequest 是非终态等待；二者都不得伪装 Completed。
- 本工作包允许保留 IntentResultV1/CapabilityGrantV1 的短期兼容适配，但关键词 resolver 不再进入生产主链；
  WP3 将移除 IntentResult 对授权的剩余影响。

## 9. 运行时假设

- provider 支持现有文本流和原生 ToolCall；内部控制封套可从文本流首部确定性解析并在投影前剥离。
- 首轮 unresolved 可直接 final/clarify；需要工具时先产生 intent/route revision，再重新装配获得当前工具清单。
- 等待期间原 Channel 可结束，但活动回合身份保留；合法补充/裁决通过新 command 继续原 turn_id。
- 无自动重试；循环预算、取消、Gate/provider/解析错误均进入真实 TurnError/Cancelled/Denied。

## 10. 疑似不变量破坏

- 当前 `turn.rs` 仍有 HandingOff/StreamEnd/HandoffOk，违反 Active SM-AG-01 的 Finalizing 路径。
- 当前关键词 resolver 在 provider 前决定最终语义和整轮工具集，违反 BR-AG-DATA-004。
- 当前续轮只重过 Gate，未重建 PromptPackage，违反 SM-AG-01/02 回归边界。
- 当前 TaskPlan 会阻止模型 final，且 Proposal/Decision 会发送 Done，违反 TaskRun 仅投影与等待非终态。
- 当前 DecisionRequest 确认后创建新 turn_id，违反 `decisionResolved → Assembling`。

## 11. 验证计划

- Rust 单元测试：SM-AG-01 完整矩阵、模型控制封套解析/隐藏、unresolved→修订、ToolResult 后改线、
  逐次 Gate 调度、Decision/Clarification 同回合恢复、取消竞态、循环上限与错误传播。
- 前端测试：DecisionRequest 等待不终结消息；confirm/cancel 均以原 session_id/turn_id 恢复，且不产生新用户回合。
- 运行 `npm run test:plan -- --files <changed_files>` 与同输入 `npm run test:related`，以及
  `npm run governance:generate`、`npm run governance:audit`、`git diff --check`。
- 对 governance audit 的 semantic candidates 逐项人工复核并写入本报告。

## 12. 文件范围结论（Scope Lock 输入）

允许修改：

- `.governance/active_scope.json`、本 Issue Trace、生成的 ADUS。
- `src-tauri/src/ag/{mod.rs,context.rs,history.rs,model_output.rs,turn.rs,tool.rs,tool_registry.rs,turn_runtime.rs,turn_prompt.rs,turn_tool_loop.rs,turn_control.rs,turn_events.rs,prompt_runtime.rs,prompts.rs,task.rs}`。
- `src/components/ag/DecisionBar.tsx`、`src/services/agService.ts`、`src/store/agStore.ts`、
  `src/services/agRuntime.test.ts`、`scripts/test_governance.test.mjs`（仅同步本工作包新增 AG 回归用例后的
  动态 inventory 断言）。

禁止修改：需求、设计、规则与协议文档；ADU/ADP/AGENTS/APC；cloud/dev-gateway；WP3 动态授权的最终实现；
WP4 上下文账本；WP5 Evidence/ProjectReport；WP6 TaskRun 全量投影重构；WP7 ProposalBatch/DiffCard 聚合。

## 13. 人类确认记录

2026-09-04：人类已确认按推荐方案完整执行，并在 WP1/文档残差完成后再次明确“确认，继续”“继续”。
本工作包是已确认计划中的 WP-AG10-02，`confirmation: YES`。

2026-09-04 范围自检：初版文件清单笔误漏列 `tool_registry.rs` 与 `DecisionBar.tsx`，但目标、入口、
boundary 假设与验收项已分别明确要求重算 CapabilityGrant、confirm/cancel 恢复原回合；本次只校正清单，
不扩展工作包目标或功能范围。

2026-09-04 验证范围自检：完整前端回归发现 AG 动态测试 inventory 已由本工作包新增回归用例从 110 增至
111，`scripts/test_governance.test.mjs` 仍断言旧现状。本次仅同步该测试资产事实，不调整测试文件 baseline、
不弱化 audit、不改变选择器或治理逻辑；已确认的完整执行授权覆盖为 WP2 新行为补充回归测试。

2026-09-04 持久化边界自检：同一 AgentTurn 恢复仍复用新回合 `save_turn`，会覆盖 WP1 已建立的事件投影
缓存，与本工作包“澄清/裁决恢复原回合”目标直接冲突。补列 `history.rs` 仅用于增加非写入的恢复身份门禁，
禁止恢复路径重建 draft 或覆盖既有 payload；不扩展持久化需求。

## 14. Task Runtime Report

### 14.1 changed_files

- 治理留痕/派生索引：`.governance/active_scope.json`、本 Issue Trace、
  `docs/00_core/A-CORE-C-R-01_ADUS.md`、`scripts/test_governance.test.mjs`。
- Rust Agent Kernel：`src-tauri/src/ag/context.rs`、`history.rs`、`mod.rs`、`model_output.rs`、
  `prompt_runtime.rs`、`task.rs`、`tool.rs`、`tool_registry.rs`、`turn.rs`、`turn_control.rs`、
  `turn_events.rs`、`turn_prompt.rs`、`turn_runtime.rs`、`turn_tool_loop.rs`。
- 前端同回合等待/恢复投影：`src/components/ag/DecisionBar.tsx`、`src/services/agRuntime.test.ts`、
  `src/services/agService.ts`、`src/store/agStore.ts`。

合计 22 个文件；全部位于 Scope Lock 允许清单。未修改需求、设计、规则、协议、cloud/dev-gateway 或
WP3-WP7 文件。

### 14.2 mapping_chain

- `AG-TURN`：BR-AG-STATE-001、BR-AG-DATA-002、BR-AG-DATA-004、BR-AG-DATA-005、
  BR-AG-VERIFY-001、BR-SYS-STATE-007、BR-SYS-STATE-008。
- `AG-TOOL`：BR-AG-STATE-002、BR-AG-VERIFY-001、X-SEQ-001、X-FORBID-006。
- 新边界 `ModelTurnControl`/`ModelOutputDecoder`、自适应兼容 grant、原回合恢复身份门禁均复用既有规则；
  未新增 RULE/CHAIN/CONSTRAINT/TERM，也未扩大授权语义。

### 14.3 test_impact

以实际 22 个 changed_files 运行 `test:plan`，选择器认定直接模块为 AG、CORE、ED、PCG、REF、SYS、WS，
受影响模块为 ACCT、AG、AST、CBT、CORE、DE、DOC、ED、ENT、KB、PCG、PRV、REF、SYS、TPL、WS；选中：

- 前端全量测试；
- TypeScript 类型检查；
- Rust 全量测试与集成测试；
- governance audit。

AG 新增回归使动态 inventory 从 110 增至 111，只同步事实断言；没有删除、跳过或弱化测试。

### 14.4 validation

- `cargo fmt --check`：通过。
- Rust Agent 定向回归：180/180 通过。
- `npm run test:plan -- --files <22 changed_files>`：通过，选择结果与上述 test_impact 一致。
- `npm run test:related -- --files <22 changed_files>`：在受限环境外完整通过；前端 26 files、596/596，
  TypeScript 通过，Rust lib 393/393，`core_chain` 5/5，`sm_broadcast` 4/4，doc-tests 通过，
  governance audit 通过。
- 首次受限环境运行仅有 LibreOffice 端到端用例因 macOS `_RegisterApplication`/SIGABRT 失败；同一用例在
  受限环境外精确复测 1/1 通过，随后同一 changed-files 的完整 `test:related` 通过，已证实为验证环境假阴性，
  不是 Agent 回归。
- `npm run governance:generate`：通过；ADUS 生成统计为 modules=16、requirements=142、
  mappings=195、rules=100、chains=27、state_machines=16、tests=982。
- `npm run governance:audit`：通过；测试治理 findings=0、warnings=0，未映射可执行源码=0，
  unowned tests=0，focused/skipped=0。
- `git diff --check`：通过。

### 14.5 semantic review

- 初始意图保持 unresolved；生产主链不再调用关键词 resolver，模型结构输出才产生 IntentUpdated/RouteUpdated。
- ToolResult 后严格回到 SM-AG-01 Assembling，重建 PromptPackage/工具投影，再取得当前次真实 GatePassed；
  旧 PromptPackage、旧 grant、旧 GateOutcome 均不复用。
- SM-AG-01 状态集合已与 Active 真源一致，删除 HandingOff/StreamEnd/HandoffOk/HandoffFail；Final 必经
  Finalizing，terminal latch 保证首终态唯一。
- Clarification/Decision 均是原 AgentTurn 的非终态等待；恢复校验同一 session_id/turn_id 及精确
  request_id，错误或重复裁决不会消费合法恢复权。
- Proposal/Decision/WorkspaceOperationPlan 不再触发伪 Done；取消竞态由 effect lock 与终态冻结守住，
  Decision cancel 不会被后到 AgentTurn 结算覆写。
- 恢复路径只验证 draft 身份，不再调用 `save_turn` 覆盖 WP1 append-only 事件投影。
- semantic candidates 逐项复核后未发现缺失规则、缺失链路、禁用术语或越出 Scope 的新业务边界。
  `IntentResultV1`/`CapabilityGrantV1` 的兼容定义仍在，但已退出生产语义决策路径，按已确认计划由 WP3 删除。

### 14.6 stop_conditions / unresolved_items

- stop_conditions：无。
- unresolved_items：无。本工作包验收项全部满足，治理状态进入 `VALIDATION / READY`。
