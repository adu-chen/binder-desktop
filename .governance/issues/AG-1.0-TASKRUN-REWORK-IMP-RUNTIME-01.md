# Issue Trace — AG-1.0-TASKRUN-REWORK-IMP-RUNTIME-01：TaskRun 编排重构（运行时/数据层）

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-15
前置：AG-1.0-TASKRUN-REWORK-REQ-01 / DESIGN-01（阶段 0/1 冻结）

## 1. 用户可见问题

实现与冻结契约（A-AG-M-S-01 §2.2.2、A-AG-M-T-01 §7.2）对齐：迁移 v12 建 task_plans/task_plan_steps/
decision_requests；TaskRun 不再由 propose_edit 置 waiting_confirm；任意新消息不得绕过未决确认；DecisionRequest
生命周期 + 裁决 IPC；step_progress 事件。

## 2. 入口点

- `src-tauri/src/db/mod.rs`（迁移 v12：三张新表）
- `src-tauri/src/types/mod.rs`（DecisionRequestV1 / TaskPlanV1 / TaskPlanStepV1）
- `src-tauri/src/ag/task.rs`（begin_turn 不绕回 WaitingConfirm；propose_decision/resolve_decision/create_plan/
  pending_decision；settle 按未决 DecisionRequest 判定 WaitingConfirm；record_proposal_step 不驱动状态）
- `src-tauri/src/lib.rs`（回合入口 gate；resolve_decision IPC；step_progress 事件）
- `src-tauri/src/ipc/mod.rs`（AgTurnEvent 扩展：step_progress / decision_request_proposed / decision_request_resolved）

## 3. 相关规则

- REQ-AG-015、BR-SYS-STATE-008、TERM-AG-015~018、SM-AG-04（§2.2.2 重构）
- 三层分离：Diff 裁决不驱动 TaskRun

## 4. 人类确认记录

2026-08-15：人类批准按修复计划分阶段实现；阶段 2 仅后端运行时/数据层，前端 UI（阶段 3）、Prompt（阶段 4）不在本 Scope。
confirmation: YES。

## 5. Task Runtime Report（完成后填写）

- task_type：`CODE_CHANGE`
- governance_phase：`IMPLEMENTATION`
- final_state：`READY`
- changed_files：
  - `src-tauri/src/db/mod.rs`（迁移 v12：task_plans / task_plan_steps / decision_requests 三表）
  - `src-tauri/src/types/mod.rs`（DecisionRequestV1/DecisionRequestStatus/TaskPlanV1/TaskPlanStepV1）
  - `src-tauri/src/ag/task.rs`（begin_turn 不绕回 waiting_confirm；settle 按未决 DecisionRequest 判定；
    propose_decision/resolve_decision/has_pending_decision/pending_decision/latest_pending_decision_for_session/
    create_plan/list_plans_by_run；测试 8 过）
  - `src-tauri/src/lib.rs`（回合入口 gate AG_DECISION_PENDING；ag.propose_decision/ag.resolve_decision IPC；
    StepProgress 发射；settle_task_run 去 proposed_diff 参数）
  - `src-tauri/src/ipc/mod.rs`（AgTurnEvent 增 StepProgress/DecisionRequestProposed/DecisionRequestResolved）
  - `docs/01_modules/AG/A-AG-M-T-01`（v2.41）、`A-AG-M-S-01`（v2.18）、`A-CORE-C-R-01_ADUS.md`（generate）
  - 本 Issue Trace、`active_scope.json`
- validation：
  - `cargo test --lib` # **passed：335 tests（ag::task 8）**
  - `npm run test:related` # passed（26 files / 576 tests）
  - `npm run governance:generate` # passed（测试用例 918→920）
  - `npm run governance:audit` # **passed（exit 0）**
  - `git diff --check` # passed
- unresolved_items：阶段 3（IMP-2 UI）按 §7.2.3 实现——移除顶部卡、底部当前动作状态条、DecisionRequest 卡、
  前端裁决动作绑定（resolve_decision 调用）；阶段 4（Prompt）修正接受即确认/片段内多卡。
