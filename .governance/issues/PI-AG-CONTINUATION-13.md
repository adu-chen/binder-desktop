# Issue Trace · PI-AG-CONTINUATION-13 多卡并存、预览回执与轮次续跑

| 项 | 内容 |
|---|---|
| 状态 | READY（自动化通过；额度相关条目已被 PI-AG-LIMIT-REMOVE-14 取代） |
| 严重级别 | P1（长任务无法自动续工；pending 卡被新卡自动失效） |
| 发现来源 | 2026-10-31 21:29/21:30/21:31 三个真实回合的 DB 事件与 diff 投影 |
| 影响模块 | AG / DE / ED / SYS |
| 人类确认 | YES（2026-10-31「pending diff 卡不影响后续 diff…新 diff 不影响已有卡生命周期；任务要能自己续工」） |

## 1. 现场事实

| 回合 | 事实 |
|---|---|
| 21:29 `t-1793503757782-1` | `write_content` 成功生成 `diff_b1150…`（base 3），checkpoint 为 `awaiting_ack`，但回合以 `partial` 结束，TaskRun 被标 `failed`；`previewChunkCount=0` |
| 21:30 `t-1793503847575-2` | 新 TaskRun 生成 `diff_0c2a…`（base 4）；前端 `expireStalePendingForPath` 立即把 `diff_b1150` 置 `expired(DE_CONSECUTIVE_PROPOSAL_ADVANCED)`；本回合同样 `partial`、TaskRun `failed` |
| 21:31 `t-1793503916184-3` | 无 TaskRun；12 个模型轮次被 `AG_TOOL_ARG`/`AG_READ_VERSION_STALE`/`AG_TOOL_RESULT_TOO_LARGE`/游标错误消耗，`remainingModelCalls=2` → `budgetClosing=true`，而只读/写入仍剩 92/5；终态 `budget_exhausted` |

## 2. 根因

| 编号 | 根因 | 位置 |
|---|---|---|
| R1 | `de.list_diffs` 的 detail projection 没有返回 checkpoint 标志；前端 `projectionFromDetail` 永远拿不到 `preview_checkpoint_required`，`previewContinuationRequired=false`，Editor 不预览、不 ack | src-tauri/src/de/routes.rs |
| R2 | `completion_settles_as_failed` 把 `partial` 一律判失败；即使还有待预览 checkpoint，TaskRun 也被标 failed，无法续跑 | src-tauri/src/ag/turn_tool_loop.rs |
| R3 | `begin_resume_task_run` 只要求 `content_chunk > 0`；预览未 ack 时永远不能显式续跑 | src-tauri/src/ag/task.rs |
| R4 | 前端在新 Proposal 到达和预览成功时自动 `expireStalePendingForPath`，把旧的 pending 卡置 expired；这是 REQ-DE-009 旧规则，与“多块 pending 卡并存、统一接受”冲突 | src/services/agService.ts、src/services/deService.ts |
| R5 | 模型可见预算行只显示只读/写入，隐藏 12 个模型轮次；实际瓶颈是轮次，模型看到 92 次只读仍以为额度充足 | src-tauri/src/ag/turn_prompt.rs |
| R6 | 回合以 `budget_exhausted` 结束时前端没有续跑；`begin_resume_task_run` 又拒绝无 chunk 的任务 | src/services/agService.ts、src-tauri/src/ag/task.rs |
| R7 | 预览 ack 常早于父回合 done；ack 后立即 `startTurn` 会被 `AG_TURN_ACTIVE` 拒绝，且没有重试/排队，导致自动续跑静默丢失 | src/services/agService.ts |
| R8 | 用户发“继续”时没有接回 active TaskRun，而是新建无任务身份回合；模型看到 pending 卡后只能停 | src/services/agService.ts、src-tauri/src/ag/prompts.rs |

## 3. 修复

1. `de.list_diffs` detail 返回 `preview_checkpoint_required`（按 `task_preview_checkpoints` 是否存在 awaiting/acknowledged 判定），Editor 才能应用并 ack。
2. 有 pending preview checkpoint 时，`partial` 不再把 TaskRun 判 failed；`settle_turn(Completed)` 保持 active。
3. `begin_resume_task_run` 允许“预览块 / 已验证锚点 / 待回执预览”任一进度。
4. 新 Proposal 与预览成功不再自动失效其他 pending 卡；保留多卡并存，接受/拒绝/保存时按各自 base 校验。
5. 预算行增加“执行轮次剩余 N”；轮次耗尽时前端对 active TaskRun 自动续跑（每 TaskRun 最多 3 次）。
6. 文档：REQ-DE-009、DE 状态机、SYS 共享契约、AG 技术设计同步为多卡并存。
7. 预览 ack 在父回合未终态时先排队，`done/error` 后再续跑，避免 `AG_TURN_ACTIVE`。
8. 新消息以“继续/接着/往下/续写/continue”开头时自动接回 active TaskRun；提示词不再要求先接受 pending 卡。

## 4. 验证

- `cargo test`：545 lib + 5 + 4 全通过；新增 `resume_allows_verified_anchors_without_preview_chunks`；终态 `remainingRounds` 改为扣减已用轮次。
- 前端 690 用例、`tsc`、`governance:audit` 通过。
- 应用已由 `tauri dev` 自动重建（binary 21:48）；待复验：生成第一张卡 → 预览 ack → 自动进入下一块；第二张卡生成时第一张仍 pending；轮次耗尽时自动续跑而非 failed。

## 5. 未解决

- 21:31 的工具参数错误（view+blocks、view+limit_chars、expected_hash 过期、结构结果过大）仍会消耗模型轮次，需要在工具 schema/说明层继续收敛。
- 旧库里已 `expired` 的 `diff_b1150` 不会复活；新任务不受影响。
- 自动续跑最多 3 次/任务；超过后需用户显式继续。
