# Issue Trace — AG-1.0-TASKRUN-REWORK-DESIGN-01：TaskRun 编排重构设计

文档状态：A
任务类型：DESIGN
治理阶段：IMPLEMENTATION（设计前置）
创建时间：2026-08-15
前置：AG-1.0-TASKRUN-REWORK-REQ-01（阶段 0 冻结语义）

## 1. 用户可见问题

阶段 0 冻结的三层分离/确认 gate/片段执行语义尚无设计载体：SM-AG-04 现有转移表与冻结语义冲突
（确认点=提案卡、无 DecisionRequest、新消息绕回）；无数据/事件/UI 契约承接。本 Scope 产出设计，不实现。

## 2. 入口点

- `docs/01_modules/AG/A-AG-M-S-01_代理状态机设计.md`（§2.2.2 SM-AG-04 重构设计，变更记录 v2.17）
- `docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md`（§7.2 数据/事件/UI 契约，变更记录 v2.40）
- 本 Issue Trace、`.governance/active_scope.json`

## 3. 相关规则/链路/术语

- 规则：REQ-AG-015、BR-SYS-STATE-008、BR-ED-VERIFY-001（内容层边界）
- 链路：AG-TURN、AG-TASK、DE-DIFF、IPC-CALL
- 术语：TERM-AG-015~018（TaskRun/TaskStep/DecisionRequest/TaskPlan）

## 4. 设计产出

### 4.1 SM-AG-04 重构（A-AG-M-S-01 §2.2.2）

**状态**（保留 8 态，语义修正）：
- Planned（可携带 TaskPlan）；Executing（执行片段内推进）；WaitingConfirm（**唯一来源=DecisionRequest**）；
  WaitingClarification；Completed / Failed / Cancelled / Interrupted。

**事件**（增补）：
- planApproved（TaskPlan 确认，进入首片段）、segmentStarted（片段开始，每片段重新授权）、
  decisionProposed（DecisionRequest proposed → WaitingConfirm）、userConfirm / userCancel（DecisionRequest 裁决，
  真实入口）、interrupt / retry / finish；stepStarted / stepOk / stepFail 保留。

**转移**：
- Planned --planApproved--> Executing（首片段）
- Executing --decisionProposed--> WaitingConfirm（仅 DecisionRequest）
- WaitingConfirm --userConfirm--> Executing（裁决后自动下一片段，重新授权）
- WaitingConfirm --userCancel--> Cancelled
- Executing --finish(片段完成/无更多步骤)--> Completed（阶段收敛=终态，有阶段结果）
- Executing --stepFail--> Failed；Executing --interrupt--> Interrupted；Interrupted --retry--> Planned(新回合)

**guard / forbidden**：
- 新消息 gate：存在未决 DecisionRequest 时新回合不得绕过（拒绝或仅带明确裁决语义）；澄清不受影响
- 每片段/步骤重新授权（profile+intent+capability+目标+上下文+既有确认点）
- 不后台续跑、不自动重试、不跨重启续跑；Diff 接受/拒绝不驱动 TaskRun

### 4.2 数据契约（A-AG-M-T-01 §7.2.1）

- `task_plans(plan_id, task_run_id, plan_json, created_at, updated_at)`；plan_json 含步骤/依赖/游标/授权/确认点/阶段结果；
- `task_plan_steps(step_id, plan_id, seq, kind, label, deps_json, required_grant, confirm_point, status, updated_at)`；
- `decision_requests(request_id, task_run_id, kind, payload_json, status(proposed/resolved), outcome, proposed_at, resolved_at)`；
- `task_runs` 保留：status 语义修正（waiting_confirm 仅由 DecisionRequest 产生）；迁移 v12。

### 4.3 事件契约（A-AG-M-T-01 §7.2.2）

- `step_progress`：{ turn_id, action: reading|tool|generating|proposing|deciding, label, detail? }（实时当前动作）
- `decision_request_proposed`：{ turn_id, request_id, kind, payload }
- `decision_request_resolved`：{ turn_id, request_id, outcome }（confirm/cancel）
- `diff_proposed` 保留（内容层，不驱动 TaskRun）；回合事件扩展。

### 4.4 UI 契约（A-AG-M-T-01 §7.2.3）

- 移除顶部 TaskRun 卡（TaskRunCard 废弃）；
- 信息流保留真实步骤事件（ToolCallCard/DiffCard）；
- 信息流底部实时"当前动作"状态条（step_progress 驱动，如"正在读取 a.md"）；
- DecisionRequest 卡（权限/方案/范围/人工确认，裁决动作绑定）；
- 状态区分（等待确认/等待补充/完成/失败/中断）来源=运行事件。

## 5. 人类确认记录

2026-08-15：人类批准按修复计划分阶段执行；阶段 1 按 4.1-4.4 产出设计，不实现。confirmation: YES。

## 6. Task Runtime Report（阶段 1 完成后填写）

- task_type：`DESIGN`
- final_state：`READY`
- changed_files：A-AG-M-S-01（§2.2.2 重构设计 + v2.17）、A-AG-M-T-01（§7.2 契约 + v2.40）、本 Issue Trace、active_scope
- validation：
  - `npm run governance:audit` # **passed（exit 0）**
  - `git diff --check` # passed
- unresolved_items：阶段 2（IMP-1）按 §7.2.1/7.2.2 实现运行时/数据层（迁移 v12 + task.rs 重构 + 回合事件）；
  阶段 3（IMP-2）按 §7.2.3 实现 UI。
