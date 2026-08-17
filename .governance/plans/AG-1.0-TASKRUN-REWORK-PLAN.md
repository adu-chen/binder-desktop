# 修复计划 — AG-1.0-TASKRUN-REWORK-PLAN：TaskRun 编排落地纠偏

文档状态：A
类型：PLAN（需求/设计纠偏前置计划）
创建时间：2026-08-15
关联：AG-1.0-R5B-TASK-ORCHESTRATION-REQUIREMENTS-01（冻结需求）、REQ-AG-015、
A-AG-M-S-01 §2.2（SM-AG-04）、A-AG-M-T-01 §7.1（运行时契约）、A-AG-M-D-01（需求描述）

## 0. 问题基线（已核实证据）

| # | 缺陷 | 证据 |
|---|---|---|
| P1 | TaskRun 在意图识别前以固定"agent 回合"直接建成 executing，无计划/步骤依赖/游标 | lib.rs:8383 begin_turn 先于 intent_result |
| P2 | UserConfirm/UserCancel 仅存在于状态机与单元测试，无真实裁决入口；实际只有 propose_edit 置 waiting_confirm；任意新消息把非终态静默绕回 executing | ag/task.rs begin_turn 沿用逻辑 |
| P3 | Diff 接受/拒绝只走内容三态链路，不驱动 TaskRun | deService.ts 裁决无 TaskRun 事件 |
| P4 | 工具授权卡是演示壳，按钮无行为绑定，不能承担任务级裁决 | ToolCallCard.tsx awaiting_confirmation 无 onClick |
| P5 | UI 只在回合 done/error 后查询刷新，无实时"当前动作"；顶部卡重复信息流步骤 | agService refreshSessionTaskRuns、TaskRunCard 顶部挂载 |
| P6 | 提示词把"接受/拒绝即人类确认"总括化 + 批次跨轮诱导，倾向单卡等待 | prompts.rs:51、:62-63 |
| P7 | 与冻结需求不符：REQ-AG-015 要求 IntentResult 后形成 TaskRun、组织多 AgentTurn 与依赖步骤、可见当前步骤/等待点、每步重新授权 | A-AG-M-D-01:290-296、R5B |

结论：落地收窄成"每回合事后事件记录"，P0 编排主链未闭环。

## 1. 修复目标与原则

1. **三层分离（不混淆）**：
   - `DiffCard`：只裁决内容及其 LogicalState/DiskState 生命周期；accept/reject **不驱动** TaskRun；
   - `DecisionRequest`：任务级通用裁决，类型含权限、方案、范围、人工确认；裁决结果 → TaskRun 事件；
   - `TaskRun`：只响应明确任务裁决，不因 Diff 裁决或任意新消息隐式推进。
2. **IntentResult 是编排入口**：复杂目标才创建 TaskPlan；简单目标单 AgentTurn 直行。
3. **TaskPlan 是可展示操作计划，不是模型思维链**：保存步骤、依赖、当前游标、所需授权、确认点、阶段结果。
4. **TaskRun 跨多执行片段/AgentTurn**，但每步/每片段重新授权（RuntimeProfile+IntentResult+CapabilityGrant+当前目标+上下文+既有确认点收窄）。
5. **任务级确认后自动开始下一执行片段**：无需用户再输入聊天消息；**绝不后台续跑**（无确认不推进）。
6. **任意新聊天消息不能绕过待确认状态**。
7. **不做"Diff 接受后自动续聊"**——那仍是把内容裁决当任务裁决。

## 2. 阶段划分与每阶段范围

### 阶段 0 · 需求澄清 Scope（DOC）
- 新 Issue Trace：`AG-1.0-TASKRUN-REWORK-REQ`（需求/语义纠偏，不实现）。
- 产出：
  1. 更新 R5B/REQ-AG-015 实现契约：冻结三层边界（DiffCard/DecisionRequest/TaskRun）；
  2. 定义 DecisionRequest 概念（类型：permission/plan/scope/manual_confirm）、生命周期（proposed→resolved(confirm/cancel)）；
  3. 定义 TaskPlan 概念（步骤/依赖/游标/授权/确认点/阶段结果）与"任务级确认→自动下一片段"语义；
  4. 明确"新消息不绕过确认" gate 规则。
- 允许修改：A-AG-M-D-01、A-AG-M-S-01 §2.2、A-AG-M-T-01 §7.1、R5B Issue。
- 验收：三层边界冻结、术语登记（DecisionRequest/TaskPlan，TERM-AG 系列）。

### 阶段 1 · TaskRun 重设计 Scope（DESIGN）
- 新 Issue Trace：`AG-1.0-TASKRUN-REWORK-DESIGN`。
- 产出：
  1. SM-AG-04 重构状态机：Planned(带 TaskPlan) → 执行片段循环（每片段：授权→步骤推进→确认点/完成）→ 终态；
     新增 DecisionRequest 生命周期（与 TaskRun 状态解耦但可联动）；
  2. 数据设计：task_plans / task_plan_steps（依赖、游标、所需授权、确认点标记）、decision_requests（类型、载荷、
     proposed_at、resolved_at、outcome）；task_runs 保留为执行实例（可含多个 plan 片段）；
  3. 事件契约：`step_progress`（实时当前动作）、`decision_request_proposed/resolved`、回合事件扩展；
  4. gate 规则：存在未决 DecisionRequest 时，新回合入口（begin_turn/ag_start_turn）拒绝或仅允许带明确裁决语义；
  5. UI 契约：移除顶部 TaskRun 卡、信息流底部实时"当前动作"条、DecisionRequest 卡、状态区分（等待确认/等待补充/
     完成/失败/中断）来源=运行事件。
- 允许修改：A-AG-M-S-01、A-AG-M-T-01、A-AG-M-D-01（若需）、types 共享契约草案。
- 验收：状态机/数据/事件/UI 契约冻结，评审通过。

### 阶段 2 · 运行时/数据层实现 Scope（IMP-1）
- 新 Issue Trace：`AG-1.0-TASKRUN-REWORK-IMP-RUNTIME`。
- 改动：
  1. `ag/task.rs`：TaskPlan 持久化与游标推进；DecisionRequest 生命周期（propose/resolve）；begin_turn 改为
     IntentResult 驱动（复杂目标建 plan，简单目标直行）；**移除"任意新消息绕回 executing"路径**；
     新增待确认 gate（新回合不绕过未决 DecisionRequest）；
  2. `lib.rs`：回合入口按意图建 plan；step_progress 事件；DecisionRequest 事件；ToolCallCard 授权壳移除/绑定；
  3. `types` / `db`：新表与迁移（task_plans、task_plan_steps、decision_requests），保留 task_runs 兼容；
  4. 测试：状态机矩阵扩展（计划→片段→确认→片段）、确认不绕行、每片段重新授权、DecisionRequest 裁决事件。
- 验收：cargo test 全绿、audit 全绿；运行时支持任务级确认后自动下一片段。

### 阶段 3 · 事件与 UI 实现 Scope（IMP-2）
- 新 Issue Trace：`AG-1.0-TASKRUN-REWORK-IMP-UI`。
- 改动：
  1. **移除**顶部 TaskRun 卡（TaskRunCard 废弃/删除）；
  2. 信息流保留真实步骤事件（ToolCallCard/DiffCard 不动）；
  3. 信息流底部实时"当前动作"状态条（step_progress 驱动，如"正在读取 a.md"）；
  4. DecisionRequest 卡（权限/方案/人工确认）UI + 裁决动作绑定；
  5. 状态区分（等待确认/等待补充/完成/失败/中断）明确且来自运行事件。
- 验收：tsc/test:related 全绿；UI 不再重复步骤、状态实时且不伪造。

### 阶段 4 · Prompt 修正 Scope（IMP-3）
- 新 Issue Trace：`AG-1.0-TASKRUN-REWORK-PROMPT`。
- 改动：
  1. 删除"接受/拒绝即人类确认"总括表述（prompts.rs:51）；
  2. 明确同一执行片段内按计划可连续生成多张 Proposal（回收批次机制到片段内，去除"接受某批后…继续提出剩余批次"
     的跨轮诱导，prompts.rs:62-63）；
  3. 同文档 Diff2 必须基于 Diff1 成功预览后的 LogicalState；Diff1 因基准推进失效，不形成补丁栈；
  4. 任务级确认点（权限/方案）走 DecisionRequest，内容确认走 DiffCard。
- 验收：提示词审计通过；模型在片段内连续多卡。

### 阶段 5 · 验证 Scope（VALIDATION）
- 更新 V-S7 与手动测试卡（A-VAL-X-V-05 修订）。
- 验证覆盖（对齐 REQ-AG-015/R5B acceptance）：
  1. 任务计划 → 读取 → 多 Proposal（单片段连续多卡）；
  2. 独立权限/方案确认后自动续片段（无需聊天消息、不后台续跑）；
  3. Diff 接受/拒绝**不改变** TaskRun；
  4. 任意新消息**不绕过**待确认；
  5. 实时状态流（"正在读取 a.md"）；
  6. 同文档连续 Diff 失效规则（Diff1 因基准推进失效）。
- 真实环境手动执行 + 事件/审计登记。

## 3. 依赖与顺序

- 阶段 0 → 1（先冻结语义与设计，后改实现）；
- 阶段 2 → 3（运行时契约先定，UI 随后）；
- 阶段 4 依赖 2/3（运行时先支持连续多卡，再改提示词）；
- 阶段 5 收口；每阶段独立 Scope Lock + gate（test:related / tsc / cargo test / audit 全绿）。

## 4. 治理衔接

- 当前 `AG-1.0-DEBT-CLEANUP-01` 已完成（READY），**不混入**本次行为语义调整；
- 每个阶段独立 Issue Trace + active_scope；不允许跨阶段越界；
- 阶段 0/1 属需求/设计纠偏（DOC/DESIGN），阶段 2-4 属 IMPLEMENTATION，阶段 5 属 VALIDATION。

## 5. 风险与回退

| 风险 | 缓解 |
|---|---|
| 行为变更大（多片段连续执行、确认点 gate 新消息）回归风险 | 阶段化交付；每阶段独立验证 gate；先单会话灰度 |
| 运行时契约与 Prompt 联动顺序错（模型多卡但运行时拒收） | 严格阶段 2/3 → 4 顺序；契约先冻结 |
| 新消息被 gate 阻断影响既有交互 | 明确区分"待确认阻断"与"正常续聊"；澄清仍走既有路径 |
| 数据迁移（新表 + 兼容 task_runs） | 迁移 v12 独立、向后兼容、保留回滚点 |
| 回退 | 每阶段 READY 后 Task Runtime Report 留档；保留行为开关（如 gate 可配置关闭）仅限灰度期 |

## 6. 成功标准

全部满足即"P0 编排主链闭环"：
- IntentResult 后复杂目标形成可见 TaskPlan；单片段内可连续多 Proposal；
- 任务级确认（权限/方案/人工）后自动续片段；内容确认（Diff）不影响任务状态；
- 任意新消息不绕过待确认；确认点不伪装工作中、不后台续跑；
- 信息流底部实时"当前动作"；顶部重复卡移除；
- 同文档连续 Diff 失效规则生效（Diff2 基于 Diff1 成功预览后 LogicalState）；
- 验证卡 V-S7 全过并登记真实证据。

## 变更记录

| 日期 | 说明 |
|---|---|
| 2026-08-15 | 创建：依据技术顾问审计报告 + 代码/需求核实（REQ-AG-015/R5B）制定 TaskRun 编排纠偏修复计划（阶段 0-5）。 |
