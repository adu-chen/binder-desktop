# Issue Trace — AG-1.0-IMP-TASKRUN-01：受限 TaskRun 运行时/数据层实现（SM-AG-04）

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-14

## 1. 用户可见症状

P0 主链项 7「受限 TaskRun、真实进度、连续 Proposal 接力」——连续接力已实现（BR-SYS-STATE-008），
TaskRun 本体（SM-AG-04）仅有设计登记（A-AG-M-S-01 §2.2），无代码/测试/IPC。冻结基线 R5-B
（REQ-AG-015 / BR-SYS-STATE-008）要求：TaskRun 只组织当前用户目标内的受限步骤；非后台队列；不自动
重试、不跨重启续跑、不越确认点；真实工作状态由回合/步骤/工具事件驱动；DiffCard 可关联 task_run_id
（UNC-09，跨回合关联字段缺失）。

## 2. 入口点

- `src-tauri/src/ag/turn.rs`（SM-AG-01；新增 SM-AG-04 转移引擎）
- `src-tauri/src/ag/history.rs`（agent_turns 持久化；task_run_id 关联）
- `src-tauri/src/types/mod.rs`（TaskRunV1/TaskStepV1 共享结构，TERM-AG-015/016）
- `src-tauri/src/db/`（task_runs 表迁移）
- `src-tauri/src/lib.rs`（ag_start_turn 回合创建/终态处挂钩 TaskRun 生命周期；DiffProposed 事件带 task_run_id）
- `src/services/agService.ts`（事件透传 task_run_id 存 diff 记录，UNC-09 数据层）

## 3. 相关规则

- `BR-SYS-STATE-008`（TaskRun 受限组织、连续接力、非补丁栈；D1 注册）
- `BR-AG-STATE-001`（唯一终态/真实取消）、`BR-AG-STATE-002`（每步最小授权）
- `TERM-AG-015/016`（TaskRun/TaskStep）
- `X-SHARED-001`（共享结构经 SYS——TaskRunV1 放 types/mod.rs）
- `SM-AG-04`（D3 建机：Planned/Executing/WaitingConfirm/WaitingClarification/Completed/Failed/Cancelled/Interrupted；
  steps pending/running/waiting_confirm/ok/failed/cancelled；W1/W2/W3 确认点）

## 4. 相关链路

AG-TURN（回合）、DE-DIFF（提案卡裁决＝确认点）、WS-SAVE（保存）。

## 5. 生产者块

- `ag/turn.rs` 既有 @GOV（SM-AG-01）——新增 SM-AG-04 转移引擎 @GOV。
- `types/mod.rs` TaskRunV1（SYS 共享）。

## 6. 消费者块

- 回合事件（DiffProposed 带 task_run_id）；DiffCard 关联（前端数据层透传，无 UI）。
- V-S7 验证卡（编排/确认点/中断重试）自动化证据。

## 7. 路径上的空规则块

本 Scope 改动文件均有 @GOV 基础；新增 task.rs 补 @GOV（BR-SYS-STATE-008）。

## 8. boundary 假设（人类确认，2026-08-14）

- **创建时机**：每回合建立一个 TaskRun（Planned→Executing），回合内工具调用/提案为步骤；同会话连续回合
  沿用同一 task_run_id（同一目标接力），目标完成/失败/取消/会话切换后新回合新建。
- **确认点**：waitingConfirm 由 propose_edit 提案卡触发（卡片裁决即 W1/W2/W3 确认点），不引入独立确认点
  机制/UI。
- **持久化**：task_run_id 随回合持久化到 DB（task_runs 表 + agent_turns 关联列），重启可恢复 Interrupted
  显示（不跨重启续跑，retry 创建新回合）；DiffCard 数据层可关联 task_run_id（UNC-09）。
- **范围**：运行时/数据层（SM-AG-04 状态机 + 数据结构 + 生命周期 + 事件透传 + 持久化），**无前端 UI**。

## 9. 运行时假设

- TaskRun 状态变化由真实回合/工具/提案事件驱动（BR-SYS-STATE-008），无静态伪进度。
- 每步重新取得当前最小授权（复用既有 CapabilityGrant 校验，不新增授权机制）。
- 取消不回滚已成功预览内容；中断不自动续跑。

## 10. 疑似不变量破坏

- SM-AG-01 回合状态机不变（TaskRun 为外层编排，回合照旧）。
- 既有回合事件/持久化格式向后兼容（新增字段缺省可解析）。

## 11. 验证计划

- 补测试：SM-AG-04 转移矩阵、TaskRun 生命周期（创建/推进/确认点/终态）、task_run_id 跨回合沿用与
  持久化恢复、DiffProposed 事件带 task_run_id。
- `npm run test:plan -- --files [changed_files]` 审阅影响，再 `npm run test:related`；`npx tsc --noEmit`；
  `cargo test`；`npm run governance:audit`（全绿）。
- 同步 `scripts/test_governance.test.mjs`（测试计数基线，如需）。

## 12. 文件范围结论（Scope Lock 输入）

允许修改：
- `src-tauri/src/ag/turn.rs`
- `src-tauri/src/ag/history.rs`
- `src-tauri/src/ag/task.rs`（新建）
- `src-tauri/src/ag/mod.rs`
- `src-tauri/src/types/mod.rs`
- `src-tauri/src/db/mod.rs`
- `src-tauri/src/lib.rs`
- `src/services/agService.ts`
- `src/store/deStore.ts`
- `src/services/agRuntime.test.ts`、`src/services/deRuntime.test.ts`
- `scripts/test_governance.test.mjs`
- `docs/01_modules/AG/A-AG-M-S-01_代理状态机设计.md`、`docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md`
- `.governance/issues/AG-1.0-IMP-TASKRUN-01.md`、`.governance/active_scope.json`

禁止修改：工作区其他未提交改动；前端 UI（AgentPanel/Composer/面板）；src-tauri 其余模块；其他文档；
治理控制文档；governance.config.json。

## 13. 人类确认记录

2026-08-14：人类经 AskUserQuestion 收紧决策（四项全选推荐）——①每回合建一个 TaskRun；②确认点复用
提案卡裁决；③task_run_id 会话级持久化（同会话连续回合沿用，重启恢复 Interrupted）；④本轮运行时/数据层
无前端 UI。confirmation: YES。

## 14. Task Runtime Report

- task_type：`CODE_CHANGE`
- governance_phase：`IMPLEMENTATION`
- final_state：`READY`
- issue_trace：本文件
- changed_files：
  - `src-tauri/src/ag/task.rs`（新建：SM-AG-04 转移引擎 + TaskRun 生命周期 begin_turn/settle/mark_stale +
    task_runs 持久化 + agent_turns 关联；@GOV BR-SYS-STATE-008-RB-AG-AG-TASK-001；5 个测试）
  - `src-tauri/src/ag/mod.rs`（声明 task 模块）
  - `src-tauri/src/types/mod.rs`（TaskRunStatus/TaskStepStatus/TaskStepV1/TaskRunV1，SYS 共享，TERM-AG-015/016）
  - `src-tauri/src/db/mod.rs`（迁移 v11：task_runs 表 + agent_turns.task_run_id 列）
  - `src-tauri/src/lib.rs`（ag_start_turn 挂 TaskRun：begin_turn/mark_stale 回合入口 + 工具/提案步骤记录 +
    DiffProposed 事件带 task_run_id + 全部终态出口 settle_task_run + 模块级 settle 辅助）
  - `src-tauri/src/ipc/mod.rs`（AgTurnEvent::DiffProposed 增 task_run_id，serde 缺省兼容）
  - `src/services/agService.ts`（事件类型 + addDiff 传 taskRunId）
  - `src/store/deStore.ts`（DiffView.taskRunId 数据层关联，UNC-09）
  - `docs/01_modules/AG/A-AG-M-S-01_代理状态机设计.md`（SM-AG-04 落地变更记录 v2.14）
  - `docs/00_core/A-CORE-C-R-01_ADUS.md`（generate 更新：@GOV 257→258、测试资产 93、测试用例 910→915）
  - 本 Issue Trace、`.governance/active_scope.json`
- mapping_chain：REQ-AG-015/BR-SYS-STATE-008 → SM-AG-04（D3 建机）→ ag/task.rs @GOV + covers（5 测试）
  + 回合/事件/持久化全链；**audit 维持全绿**。
- test_impact：
  - changed_files：task.rs、mod.rs、types、db、lib.rs、ipc、agService、deStore
  - direct_modules：AG、SYS
  - selection：test:plan 判定前端全量 + tsc + cargo 全量 + audit
- validation：
  - `npm run test:related`（npm test 前端全量） # passed：26 files / 574 tests
  - `npx tsc --noEmit` # passed
  - `cargo test`（Rust 全量） # passed：332 tests（ag::task 5 过：转移矩阵/沿用新建/确认点/中断/关联）
  - `npm run governance:generate` # passed（@GOV 257→258；测试用例 910→915）
  - `npm run governance:audit` # **passed（exit 0）**
  - `git diff --check` # passed
- automated_tests：前端 574 + Rust 332 全过。
- stop_conditions：无。
- unresolved_items：TaskRun 前端呈现（进度/步骤/确认点面板）为后续 UI Scope；批量裁决编排态（UNC-39）
  另行决策；节流/循环上限量化数值延 G5 标定（UNC-43）；V-S7 端到端真实验证（确认点/中断重试）留待
  VALIDATION 阶段。
