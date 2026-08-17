# Issue Trace — AG-1.0-TASKRUN-REWORK-IMP-UI-01：TaskRun 编排重构（事件与 UI）

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-15
前置：AG-1.0-TASKRUN-REWORK-IMP-RUNTIME-01（阶段 2 已落地）

## 1. 用户可见问题

按 A-AG-M-T-01 §7.2.3 实现 UI：移除顶部 TaskRun 卡；信息流保留真实步骤事件；信息流底部实时"当前动作"
状态条（step_progress 驱动）；DecisionRequest 卡（裁决动作绑定 confirm/cancel → ag.resolve_decision）；
回合终态状态（等待确认/等待补充/完成/失败/中断）来自运行事件。

## 2. 入口点

- `src/services/agService.ts`（AgTurnEvent 事件类型扩展 + startTurn 事件处理：step_progress → agStore 当前动作、
  decision_request_proposed/resolved；resolveDecision 封装）
- `src/store/agStore.ts`（回合当前动作 currentAction + 待决裁决 pendingDecision + setter）
- `src/components/ag/MessageList.tsx`（移除顶部 taskrun-summary；底部渲染当前动作状态条 + DecisionRequest 卡）
- `src/components/ag/DecisionRequestCard.tsx`（新建：kind 区分 + 裁决按钮 confirm/cancel）
- `src/store/agStore.ts`（AgSession 状态字段）
- `src/styles.css`（状态条/裁决卡样式；移除 taskcard 样式）
- `src/services/agRuntime.test.ts`（事件处理测试）

## 3. 相关规则

- REQ-AG-015、BR-SYS-STATE-008（真实进度不伪造）、TERM-AG-017/018、SM-AG-04 §2.2.2

## 4. 人类确认记录

2026-08-15：人类批准分阶段实现；阶段 3 仅前端事件与 UI（含移除顶部卡），不改后端语义。confirmation: YES。

## 5. Task Runtime Report（完成后填写）

- task_type：`CODE_CHANGE`
- governance_phase：`IMPLEMENTATION`
- final_state：`READY`
- changed_files：
  - `src/services/agService.ts`（AgTurnEvent 扩展 step_progress/decision_request_proposed/resolved；startTurn 事件
    处理：step_progress→currentAction、decision_*→pendingDecision；resolveDecision 封装；回合 done/error 清 currentAction）
  - `src/store/agStore.ts`（TurnCurrentAction/PendingDecisionView 类型 + AgState.currentAction/pendingDecision +
    setCurrentAction/setPendingDecision；reset/hydrate/initial 补齐）
  - `src/components/ag/MessageList.tsx`（**移除顶部 taskrun-summary/TaskRunCard**；底部渲染当前动作状态条 +
    DecisionRequest 卡；currentActionLabel 中文映射）
  - `src/components/ag/DecisionRequestCard.tsx`（新建：kind 区分权限/方案/范围/人工确认；confirm/cancel →
    ag.resolve_decision）
  - `src/components/ag/TaskRunCard.tsx`（**删除**）
  - `src/styles.css`（turn-statusbar / decisioncard 样式；移除 taskcard 样式）
  - `src/services/agRuntime.test.ts`（+2：裁决跨回合保留/裁决后清除）
  - `scripts/test_governance.test.mjs`（AG 前端测试计数 96→98）
  - `A-CORE-C-R-01_ADUS.md`（generate）、`A-AG-M-T-01`（v2.42）、本 Issue Trace、`active_scope.json`
- validation：
  - `npx tsc --noEmit` # passed
  - `npm run test:related` # passed（26 files；agRuntime +2）
  - `cargo test --lib` # passed（335 tests）
  - `npm run governance:generate` # passed（测试用例 920→922）
  - `npm run governance:audit` # **passed（exit 0）**
  - `git diff --check` # passed
- unresolved_items：阶段 4（Prompt）修正"接受/拒绝即人类确认"总括、片段内连续多卡、同文档 Diff 失效；
  阶段 5（验证）V-S7/手动卡修订 + 真实环境登记；auto-continue（裁决后自动下一片段）与 TaskPlan 前端呈现为后续迭代。
