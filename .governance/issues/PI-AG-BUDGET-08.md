# Issue Trace · PI-AG-BUDGET-08 只读/写入额度分离与单一额度来源

| 项 | 内容 |
|---|---|
| 状态 | SUPERSEDED（被 PI-AG-LIMIT-REMOVE-14 取代） |
| 严重级别 | P1（模型把剩余工具次数读成“额度只剩个位数”，停止推进） |
| 发现来源 | 2026-10-31 长文日志 + 人类确认的 v3 计划 M2（D1 选择 B） |
| 影响模块 | AG / SYS |
| 人类确认 | YES（2026-10-31「按 b 继续」：内部拆分，模型只看到读写两项，只读按剩余轮次折算） |

> 2026-10-31 更新：本工作包建立的只读 100/写入 5/轮次/纠错/收尾额度已被 PI-AG-LIMIT-REMOVE-14 整体删除；本文仅保留历史记录。

## 1. 用户可见症状

- 预算行同时暴露 `maxModelCalls/remainingModelCalls/maxToolCalls/remainingToolCalls/maxCorrections/finalReserve`，模型把剩余模型轮次误读为工具额度，提前收工。
- 只读分页与写入提案共用同一 16 次计数器，读得多就写不了；CapabilityGrant 又携带第二套 `maxToolCalls/remainingToolCalls`，同一回合出现两个额度来源。

## 2. 根因

| 编号 | 根因 | 位置 |
|---|---|---|
| R1 | `ExecutionBudgetPolicy` 只有单一 `max_tool_calls`，读/写/控制共用 | src-tauri/src/ag/tool_registry.rs |
| R2 | 预算行与 RuntimeControl 的 `capabilityGrant.budgets` 同时暴露工具额度 | src-tauri/src/ag/turn_prompt.rs、src-tauri/src/ag/prompt_runtime.rs |
| R3 | 预算文案把模型轮次、纠错、预留与工具额度并列 | src-tauri/src/ag/turn_prompt.rs |

## 3. 相关规则

- BR-AG-STATE-002（工具循环与授权）
- BR-AG-STATE-005（额度收尾与真实终态）
- BR-SYS-STATE-008（TaskRun/回合状态）

## 4. 相关链路

AG-TURN → AG-TOOL → AG-TURN

## 5. 生产者块

- `ExecutionBudgetPolicy` / `CapabilityFactorsV2` / `CapabilityBudgetsV2`（src-tauri/src/ag/tool_registry.rs）
- `prepare_turn_prompt` 预算行（src-tauri/src/ag/turn_prompt.rs）
- `insert_runtime_control_message`（src-tauri/src/ag/prompt_runtime.rs）

## 6. 消费者块

- `authorize_one_tool_call` 的扣减与 `preflight_tool_acquisition_allowed`
- `ensure_round_budget` / `continuation_execution_budget`
- `settle_completed_turn` 的 `terminal_reason` / `completion_stage`

## 7. 路径上的空规则块

无。

## 8. boundary 假设

- 不改变工具注册表权限分级与 `allowed_tools` 授权语义，只改变额度分类与呈现。
- TaskControl 工具（propose_decision / plan_workspace_operation）不计写入 5 次，仍受模型轮次与纠错上限。
- CapabilityGrant 仍是授权对象；其 `budgets` 仅供运行时核对，不进入模型上下文。

## 9. 运行时假设

- 只读调用只扣只读额度，写入调用只扣写入额度；`edit_blocked` 无效果，不扣写。
- 模型可见额度只有 `[TRUSTED EXECUTION BUDGET]` 一行；只读显示值按剩余模型轮次折算（`min(remainingRead, remainingRounds×4)`）。
- 旧在途无冻结记录的回合沿用 6/6 硬边界。

## 10. 疑似不变量破坏

- 风险：只读额度耗尽后仍允许写入，或反之。约束：`allowed_tools` 按类别投影；grant 校验拒绝无该类额度的工具。
- 风险：TaskControl 被误计入写入，决策链在第 6 次被拒。约束：`tool_budget_kind` 将 TaskControl 归为 Control，不扣读写。
- 风险：模型仍看到两套数字。约束：RuntimeControl 投影移除 `budgets`。

## 11. 验证计划

- 单测：`tool_budget_kind` 分类、grant 按类别投影、`continuation_execution_budget` 读写分别计数、预算文案只含两项、RuntimeControl 不含 `budgets`。
- 回归：`cargo test`（542）、前端 689、`tsc`、`governance:audit`。
- 真实长文复现：待人工执行。

## 12. Task Runtime Report（2026-10-31）

| 项 | 结果 |
|---|---|
| task_type / phase | CODE_CHANGE / VALIDATION → READY（仅本工作包） |
| changed_files | `src-tauri/src/ag/tool_registry.rs`、`src-tauri/src/ag/turn_tool_loop.rs`、`src-tauri/src/ag/turn_prompt.rs`、`src-tauri/src/ag/prompt_runtime.rs`、`src-tauri/src/ag/turn_runtime.rs`、`src-tauri/src/ag/turn_events.rs`、`src-tauri/src/ag/verification.rs`、`docs/01_modules/AG/A-AG-I-P-01_代理接口协议.md`、`docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md`、`docs/40_validation/A-VAL-X-V-04_Agent1.0验证卡.md`、本文件 |
| Rust | `cargo test`：542 lib + 5 core_chain + 4 sm_broadcast 全通过；新增 `tool_budget_kind_separates_read_write_and_control`、`grant_projects_only_tools_whose_budget_category_remains` |
| 前端 | `npx vitest run --exclude ".p1-spike/**"`：27 文件 / 689 用例全通过；`npx tsc --noEmit` 通过 |
| 治理 | `scripts/test_governance.test.mjs` 29/29 通过；`npm run governance:audit` findings/warnings 为空；`test:plan` 通过，ADUS 重新生成（测试资产 114 / 用例 1220） |
| 验证边界 | 未运行真实 Provider/真实长文；只读显示值只是按轮次折算，不是可用性保证；不改变授权、裁决与保存语义 |
| unresolved_items | 真实长文复现；M3 锚点、M4 控制冲突未开始；`test:related` 的 `npm test` 仍会扫到本机未跟踪 `.p1-spike`；`.governance/active_scope.json` 未切换，本工作包以本 Issue Trace 留痕 |
