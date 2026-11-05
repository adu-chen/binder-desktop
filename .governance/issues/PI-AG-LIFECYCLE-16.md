# Issue Trace · PI-AG-LIFECYCLE-16 Diff 生命周期为何反复进入模型与阻断

| 项 | 内容 |
|---|---|
| 状态 | READY（自动化通过；真实复验待执行） |
| 严重级别 | P1（模型反复叙述 diff 状态，后续写入被 AG_EFFECT_PENDING 阻断） |
| 发现来源 | 2026-11-01 03:04 `t-1793523898406-1`、03:07 `t-1793524058852-2` |
| 影响模块 | AG / DE / ED |
| 人类确认 | YES（“为什么模型还是这么关心已生成 diff 生命周期和是否保存…全链条审计”） |

## 1. 全链条事实

### 第一回合 `t-1793523898406-1`
- 前几轮读文档；第 5 轮 `write_content` 成功，生成 2 张卡
  `diff_7a3f4ec5…`、`diff_a56566fa…`（同一 proposal batch，2 组）。
- 该回合**没有 TaskRun**：模型路线只有 2 步、1 个目标、confirmationPoints 为空，
  `route_requires_task_projection` 返回 false，因此没有 `task_projected`；也就没有
  `task_preview_checkpoints`。
- 模型随后尝试第二次 `write_content`（续写，90 块），仍用第一次预览前的 base，失败
  `AG_BLOCK_OPERATION_BASE_STALE`。
- `tool_call_receipt` 对“非只读失败”一律返回 `effectStatus=unknown`，于是这次**未写入的
  基线校验失败**被记为未知效果。
- 模型 final 文本准确说明：“2 张卡待裁决、未接受/未保存、续写因 base stale 未提交，下一步重读”。
  它之所以谈生命周期，是因为提示词要求它汇报执行/完成/效果字段，并且上下文事实包含
  proposalFacts/pendingPreviews。

### 第二回合 `t-1793524058852-2`
- 用户继续时，`start_fact` 看到上一回合 `write_content` 的 `Unknown` 效果，返回
  `AG_EFFECT_PENDING`，两个只读工具成功后立即失败，无法再写入。
- 交互结果同时写“0 项效果未知”，与 `AG_EFFECT_PENDING` 自相矛盾。

## 2. 规则链审计（哪里真的依赖 diff 状态）

| 层 | 依赖/来源 | 问题 |
|---|---|---|
| Prompt 系统约束 | `prompts.rs` 要求按 executionStatus/completionStatus/effectStatus 说明；P1 大段描述预览/接受/保存 | 模型被明确要求叙述生命周期；措辞把 pending 与接受绑定 |
| AgentContextFacts | `proposalFacts` 含 state/effectStatus/saved；`pendingPreviews` 含 state/createdBlockIds | 模型能看到生命周期，属于事实投影；不是 bug，但会强化关注 |
| Tool receipt | proposal 成功返回 `completionStatus=waiting`、`effectStatus=proposal_created`、`nextAction=await_user` | 模型据此叙述“等待接受”；`await_user` 容易被理解成整回合停止 |
| 执行守卫 | `start_fact` 对 `Unknown` effect 抛 `AG_EFFECT_PENDING` | **真实阻断**：不是纯提示问题 |
| TaskRun | 简单路线不创建 TaskRun；checkpoint 创建要求 TaskRun | 无 checkpoint，预览接受后无自动续跑主体 |
| Preview continuation | 只处理 `state=pending` | 已 accepted/saved 的卡没有续跑入口 |

结论：模型“关心 diff 生命周期”既是提示词显式要求，也是运行时真的用 effectStatus 做写入守卫；
但当前把**未写入的 base stale**误记为 Unknown，导致守卫错误阻断，是主因。

## 3. 修复

- `tool_call_receipt`：`AG_BLOCK_OPERATION_BASE_STALE`、`AG_TOOL_ARG`、`AG_WRITE_CONTENT_INVALID`
  等写入前校验失败映射为 `NotStarted`，不再 `Unknown`；`nextAction` 按 retriable 给重试/受阻。
- `capability_runtime::legacy_proposal_precommit_failure` 扩展到 `write_content` 与这些
  pre-write 错误码；旧库中已落库的 Unknown 也能被判定为未执行，解除 `AG_EFFECT_PENDING`。
- 提示词：删除“主动按 executionStatus/… 复述”的要求；压缩 P1 生命周期段；明确
  “pending 不阻塞后续批次”“base stale 不是未知效果，重读 currentDocument 后同回合重试，
  不要以‘下一步我会’结束”。
- 之前的 proposal 后 `tools_frozen` 删除仍有效。

## 4. 验证

- `cargo test`：537 lib + 5 + 4；新增 base-stale 映射断言；legacy 测试改为显式构造旧 Unknown 回执。
- `npx tsc --noEmit`、前端 688 用例、`governance:audit` 通过。
- 真实 4000+3000 复验待执行。

## 5. 未解决

- 简单路线（2 步、1 目标）仍不创建 TaskRun，因此无 checkpoint、无自动预览续跑；需要把
  “内容提案产生多卡/长任务”纳入 TaskRun 投影，或在首个 proposal 后补建 TaskRun 与 checkpoint。
- 模型仍可从 `proposalFacts`/`pendingPreviews` 看到生命周期；可考虑只投影 previewed 的
  `createdBlockIds`，把 accepted/saved 从模型事实中移除（需契约变更）。
- accepted/saved diff 仍无续跑入口。
