# Issue Trace — AG-1.0-DECISION-CARD-01：任务裁决卡重构（独立固定裁决区）+ 确认词降权 + 删除引导

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-15
前置：AG-1.0-TASKRUN-REWORK（阶段 0-5 + CONTINUE + INTENT）

## 1. 用户可见问题（T4 手动测试暴露）

1. 聊天输入"确认"不会裁决卡片——被当作新编辑请求，intent 落入 general 档（Edit+ProposeEdit）触发
   AG_EDIT_TOOL_REQUIRED 强制工具死循环；且模型无待裁决上下文（DiffCard pending 不进模型消息）。
2. 裁决卡（DecisionRequest）呈现与 DiffCard 几乎同构、不阻断任务、不强调"任务暂停等决策"。
3. 删除整个文档时模型借道 replace_document 空内容 → DE 投影失败（DE_DIFF_PROJECTION_MISSING），
   而正确的 delete_doc（AG-DELETE-01）通道未被采用。

## 2. 人类设计决策（2026-08-15 拍板）

- T3 diff 链式 supersede：暂不处理（登记后续）。
- 裁决卡形态：**独立固定裁决区**（Composer 上方；回合 WaitingConfirm 时模型停止生成；输入门控"等待你的决策"；
  类型化选项），非全屏模态。
- 删除业务链：**走裁决卡 + 真实删除**（AG-DELETE-01 已具备删除执行；本批强化 delete_doc 引导 + 门控）。

## 3. 实施范围

- `src-tauri/src/ag/context.rs`（纯确认/拒绝词降权为 Answer，不落入 Edit 强制工具档；+测试）
- `src-tauri/src/ag/prompts.rs`（删除引导强化：整个文档/文件删除一律 delete_doc，不得 replace_document 清空）
- `src/components/ag/DecisionBar.tsx`（新建：独立固定裁决区，类型化选项：permission 本次/总是/拒绝，
  plan_choice/manual_confirm/delete 确认并继续/取消；输入门控提示）
- `src/components/ag/AgentPanel.tsx`（渲染 DecisionBar：Composer 上方）
- `src/components/ag/MessageList.tsx`（移除 DecisionRequestCard 渲染，迁至 DecisionBar）
- `src/components/ag/DecisionRequestCard.tsx`（删除，逻辑并入 DecisionBar）
- `src/components/ag/Composer.tsx`（pendingDecision 时发送拦截 + 提示）
- `src/services/agService.ts`（startTurn 前端 gate：非 autoContinue 且存在 pendingDecision → 不发起回合）
- `src/styles.css`（decisionbar 样式）
- `src/services/agRuntime.test.ts`（+门控/降权测试）
- `scripts/test_governance.test.mjs`（基线调整）

## 4. 相关规则

- REQ-AG-015（确认点阻断、不伪装执行）、TERM-AG-017（DecisionRequest）、BR-SYS-STATE-008、A-AG-M-D-01 §5b

## 5. 人类确认记录

2026-08-15：人类拍板——独立固定裁决区 + 删除走裁决卡/真实删除；T3 链式暂缓。confirmation: YES。

## 6. Task Runtime Report（完成后填写）

- task_type：`CODE_CHANGE`
- governance_phase：`IMPLEMENTATION`
- final_state：`READY`
- changed_files：
  - `src-tauri/src/ag/context.rs`（`detect_pure_decision`：纯确认/拒绝短词 intent 降权 Answer 零工具，
    不落入 general 档 Edit 强制工具；+pure_decision_words_are_not_edit 测试）
  - `src-tauri/src/ag/prompts.rs`（删除引导强化：整个文档/文件删除一律 delete_doc/delete_folder，
    禁止 replace_document 清空代替删除）
  - `src/components/ag/DecisionBar.tsx`（新建：独立固定裁决区，类型化选项——permission 允许本次/拒绝，
    plan_choice/scope/manual_confirm 确认并继续/取消；任务暂停提示）
  - `src/components/ag/AgentPanel.tsx`（Composer 上方渲染 DecisionBar）
  - `src/components/ag/MessageList.tsx`（移除 DecisionRequestCard 渲染，保留实时当前动作状态条）
  - `src/components/ag/DecisionRequestCard.tsx`（删除，逻辑并入 DecisionBar）
  - `src/components/ag/Composer.tsx`（pendingDecision 时发送门控 + composer-gate 提示）
  - `src/services/agService.ts`（startTurn 前端 gate：非 autoContinue 且存在 pendingDecision → 不发起）
  - `src/styles.css`（decisionbar / composer-gate 样式；移除废弃 decisioncard）
  - `src/services/agRuntime.test.ts`（+2：手动发送门控/auto-continue 绕过）
  - `scripts/test_governance.test.mjs`（AG 前端测试计数 102→104）
  - `A-CORE-C-R-01_ADUS.md`（generate）、本 Issue Trace、`active_scope.json`
- validation：
  - `cargo test --lib` # **passed：337 tests（+1 intent 降权）**
  - `npm run test:related` # passed（26 files）
  - `npx tsc --noEmit` # passed
  - `npm run governance:generate` # passed；`npm run governance:audit` # **passed（exit 0）**
  - `git diff --check` # passed
- unresolved_items：T3 diff 链式 supersede（回合内连续 LogicalState 推进，用户拍板暂缓）；
  模型回合内自动发起决策请求链路（propose_decision 接线）；真实环境 V-S7 手动执行与登记。
