# Issue Trace — AG-1.0-IMP-TASKRUN-UI-01：TaskRun 前端 UI 呈现

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-15

## 1. 用户可见症状

受限 TaskRun（SM-AG-04）运行时/数据层已实现（AG-1.0-IMP-TASKRUN-01）：task_runs 表持久化、回合挂点、
DiffProposed 事件带 task_run_id、DiffView.taskRunId 数据层关联。**缺前端呈现**：用户无法看到当前任务
进度（步骤/确认点/状态）。P0 主链项 7"受限 TaskRun、真实进度"的真实进度需可见（BR-SYS-STATE-008：
真实工作状态由回合/步骤/工具事件驱动，UI 不得伪造）。

## 2. 入口点

- `src-tauri/src/ag/task.rs`（新增按会话列查询 list_task_runs_by_session；现有 load_task_run 解析复用）
- `src-tauri/src/lib.rs`（注册 ag.list_task_runs + 分发 arm + route_list_agent_task_runs）
- `src/services/agService.ts`（TaskRunDto/TaskStepDto + listSessionTaskRuns + 回合终态刷新）
- `src/store/agStore.ts`（AgSession.taskRuns + hydrateSessionTaskRuns）
- `src/components/ag/TaskRunCard.tsx`（新建：状态徽标/步骤/确认点/进度）
- `src/components/ag/MessageList.tsx`（顶部会话 TaskRun 摘要挂载点）
- `src/services/agRuntime.test.ts`（查询 + store 投影测试）

## 3. 相关规则

- `BR-SYS-STATE-008`（真实进度仅由真实事件驱动，UI 不伪造；TaskRun 受限编排）
- `TERM-AG-015/016`（TaskRun/TaskStep）
- `SM-AG-04`（Planned/Executing/WaitingConfirm/WaitingClarification/Completed/Failed/Cancelled/Interrupted）

## 4. 相关链路

AG-TURN（回合）、AG-UI（消息流呈现）。

## 5. 生产者块

- `ag/task.rs` 既有 @GOV（BR-SYS-STATE-008-RB-AG-AG-TASK-001）——新增 list 查询沿用。
- `types/mod.rs` TaskRunV1（serde snake_case 可直接 ws_json）。

## 6. 消费者块

- MessageList 顶部 TaskRun 摘要卡（TaskRunCard）；回合终态/历史恢复时刷新。

## 7. 路径上的空规则块

list 查询为只读，不新增 RULE；前端组件不跨层（X-SEQ-002）。

## 8. boundary 假设（人类确认方向，2026-08-15）

- **呈现位置**：MessageList 顶部"会话 TaskRun 摘要卡"（最不侵入消息流段结构；消息流内 DiffCard 结构不动）。
- **数据流**：回合后查询（Query-after-turn）——回合 done/error 后刷新 + 会话历史恢复时补查；**不做
  SM-AG-04 实时广播**（后端无广播，最小读侧改动）。
- **范围**：只读呈现（不新增 TaskRun 控制按钮/裁决交互；确认点复用 DiffCard 现有裁决）。
- 显示最近若干 TaskRun（按 updated_at 倒序），每卡：intent_summary、状态徽标（中文映射）、步骤列表
  （tool/propose、等待确认高亮）、进度（ok 步骤/总数）。

## 9. 运行时假设

- TaskRun 卡只展示后端真实持久化数据（task_runs 表）；无前端伪造进度。
- 无 TaskRun 数据（新会话）时不渲染卡/显示空态。

## 10. 疑似不变量破坏

- 消息流段结构不变（text/tool/diff）；MessageList 顶部新增摘要区不影响既有消息渲染。
- 现有事件/store API 不动（agStore 加字段 + 新 hydrate 方法）。

## 11. 验证计划

- 补测试：task.rs list 查询（空/多 run 倒序）、agRuntime listSessionTaskRuns + store 投影、回合终态刷新。
- `npm run test:plan -- --files [changed_files]` 审阅影响，再 `npm run test:related`；`npx tsc --noEmit`；
  `cargo test`；`npm run governance:audit`（全绿）。

## 12. 文件范围结论（Scope Lock 输入）

允许修改：
- `src-tauri/src/ag/task.rs`
- `src-tauri/src/lib.rs`
- `src/services/agService.ts`
- `src/store/agStore.ts`
- `src/components/ag/TaskRunCard.tsx`（新建）
- `src/components/ag/MessageList.tsx`
- `src/services/agRuntime.test.ts`
- `scripts/test_governance.test.mjs`
- `docs/01_modules/AG/A-AG-M-S-01_代理状态机设计.md`、`docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md`
- `.governance/issues/AG-1.0-IMP-TASKRUN-UI-01.md`、`.governance/active_scope.json`

禁止修改：其他组件（AgentPanel/Composer/DiffCard/ToolCallCard 不动）；deService/edService；src-tauri
其余模块；其他文档；治理控制文档。

## 13. 人类确认记录

2026-08-15：人类经 AskUserQuestion 选择"TaskRun 前端 UI（推荐）"——会话内 TaskRun 卡呈现任务进度/
步骤/确认点 + 后端查询 IPC。UI 位置（MessageList 顶部摘要）、数据流（回合后查询）按最小侵入默认，
无额外决策点。confirmation: YES。

## 14. Task Runtime Report

- task_type：`CODE_CHANGE`
- governance_phase：`IMPLEMENTATION`
- final_state：`READY`
- issue_trace：本文件
- changed_files：
  - `src-tauri/src/ag/task.rs`（新增 list_task_runs_by_session 按会话倒序含步骤解析，抽 row_to_run 复用；
    +1 测试：倒序/空会话/跨会话隔离/确认点状态）
  - `src-tauri/src/lib.rs`（注册 ag.list_task_runs + 分发 arm + route_list_agent_task_runs）
  - `src/services/agService.ts`（TaskRunDto/TaskStepDto + listSessionTaskRuns 映射 + refreshSessionTaskRuns
    best-effort；回合 done/error 后刷新 + ensureAgentSessionTurns 历史恢复补查）
  - `src/store/agStore.ts`（TaskStepView/TaskRunView 类型 + AgSession.taskRuns + hydrateSessionTaskRuns）
  - `src/components/ag/TaskRunCard.tsx`（新建：状态徽标中文映射/步骤/确认点高亮/进度条，只读不伪造）
  - `src/components/ag/MessageList.tsx`（顶部会话 TaskRun 摘要挂载，最近 2 个）
  - `src/styles.css`（taskcard 系样式）
  - `src/services/agRuntime.test.ts`（+2：查询映射/失败保持上次投影）
  - `scripts/test_governance.test.mjs`（AG 前端测试计数基线 94→96）
  - `docs/01_modules/AG/A-AG-M-S-01_代理状态机设计.md`（SM-AG-04 前端呈现变更记录 v2.15）
  - `docs/00_core/A-CORE-C-R-01_ADUS.md`（generate 更新：测试用例 915→918）
  - 本 Issue Trace、`.governance/active_scope.json`
- mapping_chain：SM-AG-04（D3 建机 → IMP-TASKRUN-01 运行时）→ IMP-TASKRUN-UI-01 查询 IPC + 前端投影
  呈现；只读数据源 task_runs 表，不伪造进度（BR-SYS-STATE-008）；**audit 维持全绿**。
- test_impact：
  - changed_files：task.rs、lib.rs、agService、agStore、TaskRunCard、MessageList、agRuntime.test、
    test_governance
  - direct_modules：AG、SYS
  - selection：test:plan 判定前端全量 + tsc + cargo 全量 + audit
- validation：
  - `npm run test:related`（npm test 前端全量） # passed：26 files / 576 tests（agRuntime +2）
  - `npx tsc --noEmit` # passed
  - `cargo test`（Rust 全量） # passed：333 tests（ag::task 6 过，含 list 查询）
  - `scripts/test_governance.test.mjs` # passed（基线 96）
  - `npm run governance:generate` # passed（@GOV 258；测试用例 915→918）
  - `npm run governance:audit` # **passed（exit 0）**
  - `git diff --check` # passed
- automated_tests：前端 576 + Rust 333 全过。
- stop_conditions：无。
- unresolved_items：SM-AG-04 实时广播（回合中进度实时推送）为后续迭代；TaskRun 控制按钮（重试
  Interrupted）复用既有交互，不新增；V-S7 端到端真实验证（确认点/中断重试）留待 VALIDATION 阶段。
