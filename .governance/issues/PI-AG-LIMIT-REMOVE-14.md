# Issue Trace · PI-AG-LIMIT-REMOVE-14 删除执行额度系统

| 项 | 内容 |
|---|---|
| 状态 | READY（自动化通过；真实长文复验待执行） |
| 严重级别 | P1（12 轮/100/5/2 次限制持续截断长任务） |
| 发现来源 | 2026-10-31 长文日志 + 人类确认「能放的全放开，不设置次数」 |
| 影响模块 | AG / SYS / 前端 |
| 人类确认 | YES（2026-10-31「按你的建议执行」） |

## 1. 删除内容

- `ExecutionBudgetPolicy`、`RemainingToolBudget`、`ToolBudgetKind`、`CapabilityBudgetsV2`、
  `CapabilityGrantV2.budgets`、`CapabilityFactorsV2` 的预算字段。
- 模型轮次 12、`final_reserve` 2、`budget_closing`、`execution_budget_final_only`、
  `ensure_round_budget`、`AG_TOOL_LOOP_LIMIT`。
- 协议/参数纠错 2 次上限、`AG_MODEL_CORRECTION_LIMIT`、`correctionAttempt`。
- 只读 100 / 写入 5 扣减、`budget_claim_is_invalid`、`execution_budget_*` 事件、
  `terminal_reason`/`turn_metrics` 中的额度字段。
- `[TRUSTED EXECUTION BUDGET]` 整段提示、`prompts.rs` 中额度提示。
- 前端 3 次自动续跑上限与 `budget_exhausted` 触发；预算收尾 UI/`continuationOnly` 分支。
- `write_content` 24 块上限、`block_ops` 单次 320 操作上限。

## 2. 保留内容（硬边界）

Prompt token 预算；工具结果 12,000 字符；`read_doc` 10,000 字；`list_blocks` 50/页；
`block_ops` 单卡 64 操作/64KB；`write_content` 12,000 字符；单 session 单活回合；
`AG_EFFECT_PENDING`；待裁决 DecisionRequest；预览接受/保存与 15s ack；TaskRun 续跑需有进度且无未决裁决；
工具 30s/上下文 5s 等时间边界。

## 3. 行为变化

- `run_round_loop` 改为真正的 `loop { ... }`，只由 final/clarify/error/取消结束；没有 `usize::MAX` 伪装。
- 模型再也看不到任何执行次数；`CapabilityGrant` 无 `budgets`。
- 纠错不再累计，不会因 2 次非法控制/参数终止；用户仍可取消。
- 写入/只读只受上下文、原子块操作、副作用与时间边界约束。

## 4. 验证

- `cargo test`：537 lib + 5 core_chain + 4 sm_broadcast，0 failed。
- `npx tsc --noEmit`：通过。
- `npx vitest run --exclude ".p1-spike/**"`：27 文件 / 688 用例通过。
- `npm run governance:audit`：passed，无 findings/warnings。
- 真实长文与手工 UI 未验证。

## 5. 未解决

- 真实 4000+3000 任务复验；工具参数互斥组合仍会浪费模型调用（无次数上限后不会截断，但会增加延迟）。
- 设计文档已同步；ADUS 需重新生成。
