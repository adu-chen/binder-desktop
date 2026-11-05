# Issue Trace · PI-AG-PROPOSAL-FREEZE-15 Diff 卡后工具冻结导致任务停住

| 项 | 内容 |
|---|---|
| 状态 | READY（自动化通过；真实复验待执行） |
| 严重级别 | P1（生成第一张卡后无法继续写续写，用户看到“卡阻断任务”） |
| 发现来源 | 2026-10-31 23:00 `t-1793509238828-1` 事件 |
| 影响模块 | AG / DE / ED |
| 人类确认 | YES（“diff 卡为什么还是一直阻断任务进行，全面分析”） |

## 1. 现场事实

- 23:00:38 回合开始；前 4 轮读取文档。
- 23:01:22 第 5 轮 `write_content` 成功，生成同一 proposal batch 的 2 张卡：
  `diff_0622cb95…`（group 1）、`diff_e2634420…`（group 2），TaskRun 同时建立。
- 第 6 轮模型输出 `clarify`，文本明确问“正文建议卡是否先接受再续写第二段/是否替换续写占位”。
- 两张卡随后被接受并保存；checkpoint 1 已 `acknowledged`，但 `consumed_at` 为空。
- 第 7 轮 `clarificationResume` 后发生 `AG_GW_STREAM_TIMEOUT`（188 chunks / 713 bytes，无 completion），TaskRun 被标 failed。

## 2. 综合原因（三层）

### R1 代码层：proposal 后工具被冻结
`prepare_turn_prompt` 的 `freeze_tools_after_proposal` 传入 `facts.proposed_diff`；一旦本轮已有 proposal，
`CapabilityFactorsV2.tools_frozen=true`，`resolve_capability_grant_v2` 清空所有工具。第 6 轮模型没有任何工具，
只能 final/clarify，无法在同一回合继续提交续写。这是“卡阻断”的机械原因。

### R2 提示层：模型把 pending 当成必须接受
提示词虽已写“pending 不要求先接受”，但仍以 PreviewAck 作为后续批次的起点；模型将“接受正文卡”理解为
继续的前置条件，于是 `clarify` 询问是否先接受。第 6 轮 `clarification_required` 使任务停在等待用户回复，
而不是继续生成。

### R3 生命周期层：接受发生在回合未终态时，续跑无处落地
`acknowledgePreviewAndContinue` 只对 `state=pending` 的 diff 走旧 checkpoint 续跑；两张卡被接受后
state=accepted，checkpoint 虽 acknowledged 但未 consumed。此时旧回合仍在 clarify/stream，等回合失败后
自动续跑又没有“accepted diff + active TaskRun”的入口，任务因此停在已保存的正文之后。

## 3. 修复

- 删除 `freeze_tools_after_proposal` 参数与 `facts.proposed_diff` 传参；`tools_frozen` 只由
  `final_only_continuation`（读失败收尾）决定。proposal 后模型仍保留工具，可在同一回合继续提交后续批次。
- 提示词新增：生成 proposal 后不得以“等待接受/是否先接受/先接受再继续”为由 clarify 或 final；
  pending 卡不阻塞同回合后续批次，应继续提交剩余正文/续写。
- R3 的 accepted-diff 续跑入口尚未实现，列为未解决项；当前修复先消除“proposal 后无工具”这一主阻断。

## 4. 验证

- `cargo test`：537 lib + 5 core_chain + 4 sm_broadcast，0 failed。
- `npx tsc --noEmit`：通过。
- 前端：27 文件 / 688 用例通过。
- 真实 4000+3000 复验待执行。

## 5. 未解决

- 用户提前接受卡片时，accepted diff 不触发 TaskRun 续跑；需要“accepted/saved diff → 新续跑回合”的入口。
- `AG_GW_STREAM_TIMEOUT` 在已有可见输出时不会重试，TaskRun 被标 failed；流超时与 diff 卡无直接关系，
  但会放大“卡后中断”的体感。
