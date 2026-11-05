# Issue Trace · PI-AG-TASKRUN-21 预览回执与 TaskRun 执行解耦

| 项 | 内容 |
|---|---|
| 状态 | READY（自动化通过；真实复验待执行） |
| 严重级别 | P1（用户接受/保存卡片被当成任务继续信号） |
| 发现来源 | 2026-11-01 设计讨论 + 第 3/5 回合事实 |
| 影响模块 | AG / DE / ED / 前端 |
| 人类确认 | YES（“用户对内容的裁决不能与任务的执行、循环、进程挂钩”） |

## 1. 错误耦合

- `acknowledgePreviewAndContinue` 在预览 ack 后 `startTurn`；
- `begin_preview_continuation` 消费 checkpoint 才能继续；
- 用户不接受/不保存，TaskRun 停在 active；
- 接受/保存被当作任务继续/恢复条件。

这混淆了任务执行、内容预览、评审持久化三层。

## 2. 修复

- `acknowledgePreview` 只调用 `ag_ack_preview` 并记录技术动作；**不移交 TaskRun 执行控制**。
- 删除前端 `pendingPreviewContinuations`/`drainPendingPreviewContinuation`；
- 新增 `maybeAutoContinueTask`：片段 `done.stopReason=partial|waiting_preview` 且有 active TaskRun 时，
  自动以同一 taskRunId 开下一片段；无新 diff/候选步骤时暂停，避免无进展循环；
- `waiting_preview` 只等 800ms 技术预览写入，不等用户接受；
- 接受/拒绝/保存仍只走 DE/ED 与文件路径，不调用 `startTurn`。

## 3. 验证

- 前端 `agRuntime.test.ts`：ack 不再启动 `ag_start_turn`；新增 partial 自动续跑用例。
- `cargo test` 538 + 5 + 4；前端 689；`governance:audit` 通过。
- 真实长文复验待执行。

## 4. 未解决

- 调度器目前在前端（过渡版）；页面关闭/刷新时不能续跑，后续应迁到 Rust TaskRun 调度器。
- 无进展判定使用 diff 数 + observed/candidate steps；内容型任务可能需要在 completion 中显式携带 remaining steps。
