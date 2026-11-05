# Issue Trace · PI-AG-TASKRUN-24 内容提案前建立 TaskRun 与 checkpoint

| 项 | 内容 |
|---|---|
| 状态 | READY（自动化通过；真实复验待执行） |
| 严重级别 | P1（TaskRun 晚建导致无 checkpoint、partial failed、无法续跑） |
| 发现来源 | 2026-11-01 08:06 `t-1793544393200-1` 等日志 |
| 影响模块 | AG / DE |
| 人类确认 | YES（“按方案修复”） |

## 1. 现场事实

- 第一轮先 `diff_proposed`（2 张卡），回合末尾才 `task_projected`；
- 写卡时 `task_run_id=None`，`create_preview_checkpoint` 未执行；
- `partial` 收尾时没有 preview checkpoint，TaskRun 被结算 `failed`；
- 第二轮 24 张卡无 TaskRun/checkpoint，任务层无稳定身份。

## 2. 修复

- 新增 `ensure_task_run_for_content_proposal`：
  在 `write_content`/`propose_edit` 授权为 Allow 后、执行前建立 TaskRun，
  记录 `task_projected(reason=content_proposal)`，并 link turn；
- 建立失败则拒绝本次写入（`AG_TASK_PROJECTION_FAILED`），不产生无身份 diff；
- `TurnOutcome::Partial` 新增：`partial` 片段结算为 **active**（有 pending decision 时 Waiting），
  不再 Failed；
- `completion_settles_as_failed` 不再把 `partial` 当失败；blocked/report_only 仍失败；
- `settle_turn` 的 partial 不依赖 diff 接受/保存。

## 3. 验证

- Rust 539 + 5 + 4；新增 `partial_turn_keeps_task_run_active`；
- 前端 689、`tsc`、`governance:audit` 通过；
- 真实长文复验待执行。

## 4. 未解决

- checkpoint 失败时 diff 已持久化的 fail-closed 补偿仍需加强（当前记录失败并清除 required，阻止续跑）；
- 24 张卡分组优化未做；
- 调度器仍是前端过渡版。
