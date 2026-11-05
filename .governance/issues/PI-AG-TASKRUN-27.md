# Issue Trace · PI-AG-TASKRUN-27 acknowledged checkpoint 不得阻止 TaskRun 完成

| 项 | 内容 |
|---|---|
| 状态 | READY（自动化通过；真实复验待执行） |
| 严重级别 | P1（模型已 stop，TaskRun 仍 active，最后被重启标 interrupted） |
| 发现来源 | 2026-11-01 10:09–10:36 六回合、8 张卡 |
| 影响模块 | AG / DE |
| 人类确认 | YES（“按建议修复”） |

## 1. 现场事实

- 6 个回合同一 TaskRun，连续自动续跑；
- 8 张卡 pending，checkpoint 全部 `acknowledged`，`consumed_at` 为空；
- 第 5/6 回合 `stopReason=stop`，但 `task_status_observed=active`；
- TaskRun 最后被启动清理标 `interrupted`。

## 2. 根因

- `has_pending_preview_checkpoint` 统计 `awaiting_ack` + `acknowledged`；
- `settle_turn` 的 `Completed` 分支遇到它就把状态改 Active；
- 因此模型 stop 后任务永远不 completed，重启变 interrupted。
- 这是 diff 技术回执再次参与任务完成判定。

## 3. 修复

- `has_pending_preview_checkpoint` 只统计 `awaiting_ack`；
- 新增 `has_preview_checkpoint`（awaiting/acknowledged/consumed）仅用于续跑进度；
- `begin_resume_task_run` 允许 `interrupted` 且任何 checkpoint/锚点作为进度，恢复时改回 active；
- 前端“继续”可接回 interrupted 且有 observed/candidate steps 的 TaskRun；
- `settle_turn Completed` 不再被 acknowledged/consumed 拉回 Active。

## 4. 验证

- Rust 539 + 5 + 4；新增 checkpoint 完成后 Completed 断言；
- 前端 689、`tsc`、`governance:audit` 通过；
- 真实复验待执行。

## 5. 未解决

- 重启后自动恢复仍未实现；需要用户发“继续”接回 interrupted。
- 8 张卡粒度过碎仍是独立体验问题。
