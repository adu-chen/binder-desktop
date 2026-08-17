# Issue Trace — AG-1.0-TASKRUN-REWORK-INTENT-01：IntentResult 复杂目标 → 自动 TaskPlan

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-15
前置：AG-1.0-TASKRUN-REWORK-CONTINUE-01（计划呈现/IPC 已就绪）

## 1. 用户可见问题

承接 unresolved：IntentResult 复杂目标自动创建 TaskPlan（TERM-AG-018）的接线缺失。简单目标单回合直行；
复杂目标（多文档 / 单文档整篇改写）应在回合中自动建立可展示计划（读取→生成建议/确认点），供前端 PlanCard 呈现。

## 2. 入口点

- `src-tauri/src/ag/task.rs`（`is_complex_target` 纯函数判定 + 测试）
- `src-tauri/src/lib.rs`（intent_result 生成后接线：复杂目标 → build_task_plan → create_plan；同 run 已有计划不重复建）
- `src/services/agService.ts`（`listPlansByRun` snake→camel 映射；`refreshSessionPlans`；回合 done/error 后刷新）
- `src/services/agRuntime.test.ts`（+1：刷新会话任务计划）

## 3. 相关规则

- REQ-AG-015（简单目标单 AgentTurn、复杂目标 TaskRun 编排入口）、TERM-AG-018、A-AG-M-D-01 §5b

## 4. 复杂目标判定（确定性，不误建）

- 写意图（edit/create）+ 目标文档集 ≥ 2 → 复杂；
- 写意图 + 单文档 + prompt 含整篇信号（整篇/全文/整个文档/全部内容/所有段落/每一段/整体）→ 复杂；
- 其余单文档局部编辑 / 只读 / 澄清 → 直行不建。

计划为**可展示操作计划**（每文档：读取→生成修改建议/确认点，read→propose 依赖），不预授权、不驱动执行。

## 5. 人类确认记录

2026-08-15：人类"继续做 IntentResult"——实施复杂目标自动建计划接线。confirmation: YES。

## 6. Task Runtime Report（完成后填写）

- task_type：`CODE_CHANGE`
- final_state：`READY`
- changed_files：`src-tauri/src/ag/task.rs`（is_complex_target + 测试）、`src-tauri/src/lib.rs`（接线 + build_task_plan）、
  `src/services/agService.ts`（listPlansByRun 映射 + refreshSessionPlans + done/error 刷新）、
  `src/services/agRuntime.test.ts`（+1）、`scripts/test_governance.test.mjs`（AG 前端 101→102）、
  `A-CORE-C-R-01_ADUS.md`（generate）、本 Issue Trace、`active_scope.json`
- validation：
  - `cargo test --lib` # **passed：336 tests（+1 complex_target_detection）**
  - `npm run test:related` # passed（26 files）
  - `npx tsc --noEmit` # passed
  - `npm run governance:generate` # passed；`npm run governance:audit` # **passed（exit 0）**
  - `git diff --check` # passed
- unresolved_items：真实环境 V-S7 手动执行与登记（A-VAL-X-V-05 T1-T12）；计划执行进度与 TaskRun 步骤状态联动为后续迭代。