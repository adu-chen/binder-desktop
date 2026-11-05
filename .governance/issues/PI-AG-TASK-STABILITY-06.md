# PI-AG-TASK-STABILITY-06 — 长任务分块、终态事实与可恢复执行

| 项 | 内容 |
|---|---|
| 状态 | READY（全阶段授权，一并优化） |
| 严重级别 | P1 |
| 发现来源 | 复杂任务（4000 字正文 + 3000 字续写）多次停滞/中断；短任务闭环已通过 |
| 影响模块 | AG / DE / ED / SYS |
| 人类确认 | YES（2026-10-31：确认全阶段授权、自动续跑、输出预算不固定、分块卡交互） |

## 1. 目标

1. 长内容生成不再要求单次输出巨大 JSON；
2. 任务中断后可从检查点续跑，不重复生成、不重复提交；
3. 模型不能自行宣布预算耗尽/完成，终态由 Runtime 按证据裁决；
4. 保持现有唯一写路径：DE 预览 → 接受 → 保存。

## 2. 关键事实（代码核对）

- `propose_edit` 返回即由 DE 预览写入编辑器 LogicalState，`edStore.updateContent` 使 `sessionVersion+1`；
- preview checkpoint（`task_preview_checkpoints`）与 `ag_ack_preview` 已存在，ack 后自动开启续跑 Turn；
- `task_runs` 已存在，可扩展 plan/progress；
- 新块 id 由 `ensure_new_block_id` 生成，但未返回给模型（缺口）；
- `AG_GW_STREAM_TIMEOUT` 已标 `retriable=true`，回合循环未消费（缺口）；
- 模型 `stopReason` 直接进入终态，未校验预算/授权证据（缺口）。

## 3. 交互契约（冻结）

- 长任务按块生成，每块生成一张 pending DiffCard，同属一个 TaskRun；
- 生成过程中卡片逐张出现，可逐张接受，也可等待全部完成；
- TaskRun 完成前不自动保存；完成后“全部接受”+ 一次保存；
- 中断保留已生成卡片与检查点，续跑从下一块继续，不重复；
- 不引入第二份内容草稿：草稿只在编辑器 LogicalState。

## 4. 分阶段范围

| 阶段 | 内容 |
|---|---|
| 1 | 终态 reasonCode + Runtime 证据校验；Attempt 记录；无副作用超时重试；done 保障 |
| 2 | `write_content` 语义工具（编译到 block_ops）；回执返回 `createdBlockIds`；动态 chunk 上限 |
| 3 | 分块链路：preview ack 续跑 + 新 base/锚点；`task_runs.plan/progress` |
| 4 | 自动续跑与恢复：人为中断续跑、TaskRun 分组接受、effect_unknown 核对 |
| 5 | Prompt/Schema 动态裁剪；模型路由评估（最后） |

## 5. 验证

- `npx tsc --noEmit`、前端全量、`cargo test`、`npm run governance:audit`；
- E1–E6（见方案 v3 验证矩阵），E5 必须 mock 断流；
- UI 行为由人类复核（卡片、接受、保存、续跑）。

## 6. 风险

- base 漂移：每块必须用 ack 后新 base；
- 重复应用：依赖 draftApplied + 新块 id 存在性检测；
- 卡数量：按 TaskRun 分组，全部接受；
- 终态误判：reasonCode 只由 Runtime 证据确认。
