# Issue Trace — AG-1.0-TASKRUN-REWORK-CONTINUE-01：auto-continue + TaskPlan 呈现

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-15
前置：AG-1.0-TASKRUN-REWORK 阶段 0-4（三层分离/DecisionRequest/gate/实时状态已落地）

## 1. 用户可见问题

承接 unresolved：① 任务级确认（DecisionRequest confirm）后应**自动开始下一执行片段**（无需用户再输入聊天消息，
绝不后台续跑）——当前 resolve_decision 返回 Executing 但无触发方；② TaskPlan（可展示操作计划）数据层已建但
无 IPC/前端呈现。

## 2. 入口点

- `src-tauri/src/ag/task.rs`（list_plans_by_run 已有；无需新函数）
- `src-tauri/src/lib.rs`（注册 ag.list_plans / ag.create_plan IPC + route）
- `src/services/agService.ts`（resolveDecisionAndContinue：confirm → 自动 startTurn 续片段，含连续计数保护；
  listPlansByRun 封装）
- `src/store/agStore.ts`（AgSession.taskPlans + hydrateSessionPlans）
- `src/components/ag/DecisionRequestCard.tsx`（确认 → resolveDecisionAndContinue）
- `src/components/ag/PlanCard.tsx`（新建：计划清单卡——步骤/依赖/游标/确认点）
- `src/components/ag/MessageList.tsx`（渲染 PlanCard）
- `src/services/agRuntime.test.ts`（auto-continue + 上限测试）

## 3. 相关规则

- REQ-AG-015（确认后自动下一片段、不后台续跑）、TERM-AG-017/018、SM-AG-04 §2.2.2

## 4. 人类确认记录

2026-08-15：人类"接着做"——承接 unresolved 实施 auto-continue 与 TaskPlan 呈现。confirmation: YES。

## 5. Task Runtime Report（完成后填写）

- task_type：`CODE_CHANGE`
- governance_phase：`IMPLEMENTATION`
- final_state：`READY`
- changed_files：
  - `src-tauri/src/lib.rs`（注册 + route：ag.list_plans / ag.create_plan）
  - `src/services/agService.ts`（startTurn 增 autoContinue 选项 + 手动发送重置计数；resolveDecisionAndContinue
    （confirm→自动续片段、MAX=3 上限）；listPlansByRun）
  - `src/store/agStore.ts`（TaskPlanView/TaskPlanStepView + AgSession.taskPlans + hydrateSessionPlans）
  - `src/components/ag/DecisionRequestCard.tsx`（确认→resolveDecisionAndContinue"确认并继续"；取消→resolveDecision）
  - `src/components/ag/PlanCard.tsx`（新建：计划步骤/依赖/游标/确认点/授权）
  - `src/components/ag/MessageList.tsx`（渲染 PlanCard）
  - `src/styles.css`（plancard 样式）
  - `src/services/agRuntime.test.ts`（+3：确认续片段/上限与取消/计划投影）
  - `scripts/test_governance.test.mjs`（AG 前端测试计数 98→101）
  - `A-CORE-C-R-01_ADUS.md`（generate）、本 Issue Trace、`active_scope.json`
- validation：
  - `npx tsc --noEmit` # passed
  - `npm run test:related` # passed（26 files；agRuntime +3）
  - `cargo test --lib` # passed（335 tests）
  - `npm run governance:generate` # passed（测试用例 922→925）
  - `npm run governance:audit` # **passed（exit 0）**
  - `git diff --check` # passed
- unresolved_items：~~IntentResult 复杂目标 → 自动 create_plan 接线~~（**已落地**：AG-1.0-TASKRUN-REWORK-INTENT-01）；真实环境 V-S7 手动执行与登记（A-VAL-X-V-05 T1-T12）。
