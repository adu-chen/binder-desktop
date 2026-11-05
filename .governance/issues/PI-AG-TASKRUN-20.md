# Issue Trace · PI-AG-TASKRUN-20 新目标被未完成 TaskRun 挡住

| 项 | 内容 |
|---|---|
| 状态 | READY（自动化通过；真实复验待执行） |
| 严重级别 | P1（新目标跑 21 轮后以 AG_TASK_DB_ERROR 失败） |
| 发现来源 | 2026-11-01 05:36 `t-1793532975534-5` |
| 影响模块 | AG / 前端 |
| 人类确认 | YES（“完成剩余的待优化项目”） |

## 1. 现场事实

- 第 3 回合 `t-1793531337173-3` 生成 9 张卡，TaskRun `…1351028-1` 保持 active；
- 第 5 回合 `t-1793532975534-5` 是新目标（“删除结尾多余续写”），不是 continuation；
- 第 5 回合 21 轮后才在任务投影时命中 `begin_turn`：
  `unfinished task run requires explicit continuation: …1351028-1`；
- 错误被包成 `AG_TASK_DB_ERROR`，不是可理解的“请先续跑/关闭旧任务”。

## 2. 根因

1. `begin_turn` 只要看到同会话非终态 TaskRun 就拒绝新建；
2. 新目标没有显式 continuation，旧 TaskRun 又不会被收束；
3. 守卫在 21 轮后的任务投影才触发，用户先做了大量工作才失败；
4. 前端“继续”的 active TaskRun 读取依赖异步 best-effort 刷新，可能读到旧摘要。

## 3. 修复

- 新增 `task::interrupt_non_terminal_for_session`：新目标（无 continuation、无 pending Decision）
  开始前把同会话非终态 TaskRun 标为 `interrupted`，记录 `task_superseded`；
  不冒充完成、不合并目标，旧卡/旧效果按各自事实保留。
- 前端 `startTurn` 在“继续”意图但 store 无 active TaskRun 时，按需调用
  `listSessionTaskRuns` 核验并水合，再决定是否携带 continuation。
- 预览续跑失败时退回 TaskRun-only 续跑；仍失败则给出可操作的“请发送「继续」”，
  不再只写 console。

## 4. 验证

- `cargo test`：538 lib + 5 + 4，0 failed；新增
  `new_goal_interrupts_non_terminal_task_run_before_reprojection`。
- 前端 688 用例、`tsc`、`governance:audit` 通过。
- 真实复验待执行。

## 5. 未解决

- 新目标自动把旧 TaskRun 标 interrupted 是产品语义选择；若用户希望“同一任务继续”，
  应使用“继续/接着/续写”前缀或预览 ack。
- `partial` + active TaskRun 仍不会自动开下一片段；需用户发“继续”。
