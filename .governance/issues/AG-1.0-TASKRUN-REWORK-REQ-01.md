# Issue Trace — AG-1.0-TASKRUN-REWORK-REQ：需求澄清（三层分离 / TaskPlan / DecisionRequest / 确认 gate）

文档状态：A
任务类型：DOC
治理阶段：IMPLEMENTATION（需求/语义纠偏前置）
创建时间：2026-08-15
前置：AG-1.0-TASKRUN-REWORK-PLAN 阶段 0

## 1. 用户可见问题

TaskRun 落地收窄为"每回合事后事件记录"（AG-1.0-TASKRUN-REWORK-PLAN §0，P1-P7）：确认点被 Diff 内容确认窄化、
无任务级裁决入口、任意新消息绕过待确认、与 REQ-AG-015/R5B"IntentResult 编排入口、每步重新授权、确认点不伪装工作"
不符。本 Scope 只冻结需求语义，不实现。

## 2. 入口点

- `docs/01_modules/AG/A-AG-M-D-01_代理功能需求描述.md`（追加 REQ-AG-015 实现契约，不改 REQ 原文）
- `docs/01_modules/AG/A-AG-M-S-01_代理状态机设计.md`（SM-AG-04 §2.2 修订登记 v2.16）
- `docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md`（TERM-AG-017/018 + §7.1 修订登记 v2.39）
- `.governance/issues/AG-1.0-R5B-TASK-ORCHESTRATION-REQUIREMENTS-01.md`（契约更新登记）
- 本 Issue Trace、`.governance/active_scope.json`

## 3. 相关规则

- REQ-AG-015、BR-SYS-STATE-008、BR-ED-VERIFY-001（Diff 内容确认边界）
- 冻结基线 R5B：任务编排不实现为后台队列、每步最小授权、确认点不伪装工作中

## 4. 相关链路

AG-TURN、AG-TASK（TaskRun）、DE-DIFF、IPC-CALL。

## 5. 冻结语义（本 Scope 交付物）

### 5.1 三层分离（不混淆）
- **DiffCard**：只裁决内容及其 LogicalState/DiskState 生命周期；accept/reject **不驱动** TaskRun 状态。
- **DecisionRequest**：任务级通用裁决，类型 = 权限（permission）/ 方案（plan_choice）/ 范围（scope）/ 人工确认
  （manual_confirm）；生命周期 = proposed → resolved（confirm / cancel）；裁决结果 → TaskRun 事件
  （UserConfirm / UserCancel）。
- **TaskRun**：只响应明确任务裁决；不因 Diff 裁决或任意新消息隐式推进。

### 5.2 TaskPlan（可展示操作计划，非模型思维链）
- IntentResult 判定复杂目标后创建；保存：步骤列表、步骤依赖、当前游标、所需授权、确认点标记、阶段结果。
- 简单目标：单个 AgentTurn 直行，不建 TaskPlan。

### 5.3 执行语义
- 一个 TaskRun 可跨多个执行片段/AgentTurn；每个片段/步骤按
  RuntimeProfile + IntentResult + CapabilityGrant + 当前目标 + 当前上下文 + 既有确认点重新收窄（每步重新授权）。
- 任务级确认（DecisionRequest confirm）后**自动开始下一执行片段**：无需用户再输入聊天消息；**绝不后台续跑**。
- 确认点不伪装为工作中、不持有模型/工具执行（REQ-AG-015 acceptance）。

### 5.4 新消息 gate
- 存在未决 DecisionRequest 时，任意新聊天消息**不得绕过**待确认状态（新回合入口拒绝或仅接受带明确裁决语义的输入）；
  澄清（clarification）走既有路径不受此 gate 影响。

### 5.5 同文档连续 Diff
- 同一执行片段内按计划可连续生成多张 Proposal；Diff2 必须基于 Diff1 成功预览后的最新 LogicalState；
  Diff1 因基准推进失效，不可再接受/拒绝，不形成补丁栈。

## 6. 人类确认记录

2026-08-15：人类批准按修复计划（阶段 0-5）执行；阶段 0 冻结上述语义（5.1-5.5），本 Scope 仅文档不改代码。
confirmation: YES。

## 7. Task Runtime Report（阶段 0 完成后填写）

- task_type：`DOC`
- governance_phase：`IMPLEMENTATION`
- final_state：`READY`（文档冻结）
- changed_files：
  - `A-AG-M-D-01`（REQ-AG-015 实现契约：三层分离/TaskPlan/DecisionRequest/gate/同文档 Diff 失效）
  - `A-AG-M-S-01`（SM-AG-04 §2.2.1 语义纠偏，变更记录 v2.16）
  - `A-AG-M-T-01`（TERM-AG-017/018，变更记录 v2.39）
  - `AG-1.0-R5B-TASK-ORCHESTRATION-REQUIREMENTS-01.md`（§12 契约登记）
  - 本 Issue Trace、`active_scope.json`
- validation：
  - `npm run governance:generate` # passed（术语标注 62→64：TERM-AG-017/018）
  - `npm run governance:audit` # **passed（exit 0）**
  - `git diff --check` # passed
- unresolved_items：阶段 1（DESIGN）承接——SM-AG-04 重构 + 数据/事件/UI 契约。
