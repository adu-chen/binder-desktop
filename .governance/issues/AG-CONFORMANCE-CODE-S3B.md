# Issue Trace — AG-CONFORMANCE-CODE-S3B：TaskRun 按任务级产物创建

文档状态：A
任务类型：CODE_CHANGE
治理阶段：VALIDATION
任务状态：READY
创建时间：2026-08-28
上游基线：AG-CONFORMANCE-GOVERNANCE-S2；人类已确认 HD-S2-03。

## 1. 用户可见症状

当前每个 AgentTurn 都在意图识别前创建或沿用 TaskRun，导致简单回答、分析、检索和单文档局部修改被错误投影为任务编排。TaskRun 进而成为提示词、工具循环、澄清和编辑器预览续片段的隐式必填身份。

## 2. 入口点

- ADUS `AG-TURN` 链路登记的 `ag_start_turn` 主入口。
- ADUS `BR-SYS-STATE-008` 登记的 TaskRun 状态、查询、DecisionRequest 与工具循环生产块。
- 主入口直接调用但 S2 判定未注册的 TaskRun 创建/复用 owner。

## 3. 相关规则

- `BR-SYS-STATE-008`：TaskRun 只组织同一用户目标内多个受限步骤/AgentTurn，不是后台队列；状态只能由真实回合、步骤或工具事件驱动。
- `BR-SYS-STATE-007`：AgentTurn 澄清为非终态，AgentTurn 可关联而非必须关联 TaskRun。
- `BR-AG-STATE-001/002`：AgentTurn 与 ToolCall 的受控状态及终态边界。
- `BR-AG-DATA-001/002`：LogicalState、PromptPackage 与编辑提案边界。

## 4. 相关链路

主链：`AG-TURN → AG-TOOL`。只有创建 WorkspaceOperationPlan 时交叉承接 `WS-OPERATION`；本 Scope 不修改其既有计划与执行实现。

## 5. 生产者块

- `turn_runtime.rs::ag_start_turn`：当前在 IntentResult 前无条件调用 `task::begin_turn`。
- `task.rs::begin_turn`：当前真实创建/复用 TaskRun，但没有独立 @GOV owner。
- `turn_prompt.rs::prepare_turn_prompt`：仅为读取 TaskPlan 而要求必填 task_run_id。
- `turn_tool_loop.rs::run_turn_tool_loop`：所有结算、步骤记录、DecisionRequest、WorkspaceOperationPlan、Diff 预览检查点均假设 task_run_id 必填。
- `turn_control.rs`：澄清恢复与 PreviewAck 把 TaskRun 当作必填回合身份。

## 6. 消费者块

- TaskPlan、DecisionRequest、WorkspaceOperationPlan：必须消费真实 TaskRun。
- 普通模型回复、只读工具、简单 pending Diff：不需要消费 TaskRun。
- Preview continuation：只有已处于任务编排的连续提案才消费 TaskRun；简单 Diff 没有任务续片段。
- 前端 DiffCard：`task_run_id` 已是可选字段，但 PreviewAck 当前错误要求其存在。

## 7. 路径上的空规则块

ADUS 没有 `rules: []` 的相关生产块；但 `task::begin_turn` 是已注册主入口的直接持久态依赖而未单独注册。S3B 将在真实创建 owner 上登记 `BR-SYS-STATE-008`，不以外层注释替代实际状态写入者。

## 8. boundary 假设

- `AgentTurn → TaskRun` 是可选关联，不能由每次发送自动推导。
- “任务级产物”限定为 TaskPlan、DecisionRequest、WorkspaceOperationPlan 或显式 TaskRun continuation。
- pending Diff 本身是 AgentTurn/Proposal 产物，不自动等同 TaskRun；只有多步骤连续提案需要 TaskRun preview checkpoint。

## 9. 运行时假设

- IntentResult 与目标范围可在没有 TaskRun 时安全构建。
- 复杂目标可在 TaskPlan 持久化前同步创建 TaskRun。
- 简单回合若动态调用 `propose_decision` 或 `plan_workspace_operation`，可在该产物持久化前同步创建并绑定 TaskRun。
- 不存在 TaskRun 时，结算与工具步骤记录为空操作，但 AgentTurn 的唯一终态仍正常发送。

## 10. 疑似不变量破坏

- 无条件创建会把“一次交互”伪装成“任务编排”。
- 直接把 task_run_id 改为可空但不处理步骤、结算和预览消费者，会造成空身份错误或孤立检查点。
- 为保持自动预览续片段而给简单 Diff 创建 TaskRun，会继续违反 HD-S2-03。
- 动态创建 TaskRun 后未绑定当前 AgentTurn，会破坏回合/任务/裁决追溯。

## 11. 验证计划

1. 单测覆盖简单/复杂目标的 TaskRun 判据与创建 owner。
2. 单测覆盖无 TaskRun 的澄清恢复身份、DiffCard 可选关联和 PreviewAck 不自动继续。
3. 保持复杂计划、DecisionRequest、WorkspaceOperationPlan 与预览 continuation 的既有测试通过。
4. 运行 changed_files `test:plan`、`test:related`、`test:governance`、`governance:audit` 与差异检查。
5. 刷新 ADUS，确认 `BR-SYS-STATE-008` 可定位到真实创建判据与 owner。

## 人类裁决

- `HD-S2-03`（CONFIRMED）：简单 AgentTurn 完全不创建 TaskRun；需要任务级产物时按需创建，并允许调整当前必填关联。
- `HD-S3B-01`（CONFIRMED）：只有显式 continuation 才能复用非终态 TaskRun；无 continuation 时失败关闭，不静默合并目标或自行取消旧任务。
- `HD-S3B-02`（CONFIRMED）：同步 AG 前端测试治理基线 108→109，登记无 TaskRun 简单 Diff 不自动续片段证据。

## Task Runtime Report

- scope：见 `.governance/active_scope.json`。
- changed_files：`.governance/active_scope.json`、本 Issue Trace、`src-tauri/src/ag/{task,task_runtime,turn_control,turn_events,turn_prompt,turn_runtime,turn_tool_loop}.rs`、`src/services/agRuntime.test.ts`、`scripts/test_governance.test.mjs`、`docs/40_validation/A-VAL-X-V-04_Agent1.0验证卡.md`、机器生成的 `docs/00_core/A-CORE-C-R-01_ADUS.md`。
- implementation：简单 AgentTurn 不再创建 TaskRun；复杂目标在 TaskPlan 前按需创建；获准的 DecisionRequest / WorkspaceOperationPlan 工具在产物落库前按需创建；非终态 TaskRun 仅允许显式 continuation 复用；无 TaskRun 的澄清与 pending Diff 保持 AgentTurn 身份且不伪造预览续片段。
- validation：`test:plan` 选择前端全量、Rust 全量和治理审计；沙箱外 `test:related` 退出码 0，前端 26 文件 / 592 用例、Rust lib 363 用例、`core_chain` 5 用例、`sm_broadcast` 4 用例及 doc-tests 全部通过，治理审计通过；`test:governance` 19/19 通过，发现 104 个测试资产、948 个用例；`cargo check --lib`、相关定向单测与 `git diff --check` 通过。沙箱内首次 Rust 全量仅因 LibreOffice AppKit 环境限制中止，沙箱外同一真实转换用例已通过。
- requirements_validation：规则链和自动化证据已闭合；真实 Tauri/WKWebView、真实 provider 及验证卡 V-S7 的生产交互尚未执行，不能据此声明系统级验证通过。
- unresolved_items：CapabilityGrant 最小化留待 S3-C；跨文档目标集规则留待后续独立 Scope；真实 UI/provider 链路待验证卡执行。

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-08-28 | v1.0 | 建立 S3B CODE_CHANGE Scope；按 HD-S2-03 锁定 TaskRun 按任务级产物创建的边界。 |
| 2026-08-28 | v1.1 | 人类确认 HD-S3B-01：非终态 TaskRun 只允许显式 continuation 复用；无身份的新任务级请求失败关闭。 |
| 2026-08-28 | v1.2 | 人类确认 HD-S3B-02：AG 前端测试治理基线随新增安全证据由 108 调整为 109。 |
| 2026-08-28 | v1.3 | 实现 TaskRun 按需创建与可选关联，完成影响测试、治理审计和运行时证据，任务进入 READY。 |
