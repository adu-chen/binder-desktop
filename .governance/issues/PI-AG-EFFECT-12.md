# Issue Trace · PI-AG-EFFECT-12 write_content 未知效果导致后续写入被永久阻断

| 项 | 内容 |
|---|---|
| 状态 | READY（自动化通过；需重启应用验证旧事实收敛） |
| 严重级别 | P1（长任务在成功生成一张卡后，下一回合直接 `AG_EFFECT_PENDING` 中断） |
| 发现来源 | 2026-10-31 21:17 真实回合 `t-1793503047510-1` 日志 + workspace.db 事实核对 |
| 影响模块 | AG / DE / SYS |
| 人类确认 | YES（2026-10-31「检查最近一轮日志，为什么还是中断了」） |

## 1. 现场事实

- 最近回合 `t-1793503047510-1`（21:17）在 `write_content` 执行时返回 `AG_EFFECT_PENDING`，终态 `error`，未重试（`retriable=false`）。
- 21:17 的 `interaction_result` 自身记录 `unknownEffects=0, unsettledEffects=0`，与守卫结论矛盾。
- 往前一回合 `t-1793468034118-1`（20:54）的 `write_content`
  `call_00_MIo8G6tNzkFdc6miXSTG8616` 执行成功并生成 `diff_730d007f...`，但回执为
  `executionStatus=succeeded, completionStatus=blocked, effectStatus=unknown, nextAction=inspect_effect`。
- 该 diff 后续已被接受并保存：`pending_diffs.status=accepted, effect_status=saved`。
- 但 `ag-effect:` 结算事实仍保留 `unknown`，`start_fact` 将其视为未结算，导致 **整个 workspace 的后续写入被阻断**。

## 2. 根因

| 编号 | 根因 | 位置 |
|---|---|---|
| R1 | `tool_call_receipt` 只把 `propose_edit` 的 PendingDiff 映射为 `proposal_created`；`write_content` 走同一 DE 路径却落入兜底 `unknown` | src-tauri/src/ag/tool_exec.rs |
| R2 | `start_fact` 只对 `propose_edit + AG_TOOL_ARG` 做历史兼容，未用 PendingDiff 的后续裁决状态收敛 `unknown` | src-tauri/src/ag/capability_runtime.rs |
| R3 | 结果：一次成功提案留下永久 unknown 事实，下一次写入必被 `AG_EFFECT_PENDING` 拒绝 | 同上 |

## 3. 修复

1. `write_content` 生成 PendingDiff 时与 `propose_edit` 一样返回
   `succeeded / waiting / proposal_created / await_user`。
2. 已结算 `unknown` 且引用 `pending_diff` 的事实，按后续裁决收敛：
   - `accepted + effect_status=saved` → 已解决；
   - `rejected` / `expired` → 无待生效修改，已解决；
   - `pending` / `awaiting_save` → 仍阻断新写入。
3. 会话删除的未结算检查使用同一收敛规则，避免误删或误留。

## 4. 相关规则与链路

- BR-AG-STATE-002 / BR-AG-STATE-005
- BR-SYS-DATA-008（效果事实）
- BR-DE-DATA-001（PendingDiff 生命周期）
- AG-TURN → AG-TOOL → DE-DIFF → AG-TURN

## 5. 生产者块

- `tool_call_receipt`（src-tauri/src/ag/tool_exec.rs）
- `start_fact` / `resolved_pending_diff_effect` / `delete_settled_session_facts`（src-tauri/src/ag/capability_runtime.rs）

## 6. 消费者块

- `AG_EFFECT_PENDING` 写入守卫
- 后续 `write_content` / `propose_edit` 执行路径
- 会话删除 guard

## 7. 空规则块

无。

## 8. boundary 假设

- 收敛只释放“是否还有未结算写效果”的守卫，不把 unknown 回执改写为成功，不补造业务完成证据。
- 仍拒绝自动重放；同一 toolCallId 的重复执行继续失败关闭。
- 只认 `pending_diff` 引用；无引用的 unknown 仍保持阻断。

## 9. 运行时假设

- PendingDiff 状态由 DE/ED 正常裁决链路写入；本修复只读它，不写它。
- `accepted` 但尚未 `saved` 时仍视为未结算，避免在保存前并发写同一文档。

## 10. 疑似不变量破坏

- 风险：把 rejected/expired 当成“已成功”。约束：只释放守卫，不改写回执或终态。
- 风险：把 accepted 但未保存当成已解决。约束：要求 `effect_status=saved`。
- 风险：跨 workspace 复用。约束：查询带 `workspace_id`。

## 11. 验证计划

- `tool_call_receipt`：`write_content` + PendingDiff → ProposalCreated/Waiting/AwaitUser。
- `resolved_pending_diff_clears_stale_unknown_effect`：accepted+saved / rejected 放行；pending 仍阻断。
- 全量 `cargo test`、`tsc`、前端 689、`governance:audit`。
- 真实应用：重启后旧 `unknown` 事实应因 diff 已 saved 而收敛，下一次写入不再 `AG_EFFECT_PENDING`。

## 12. Task Runtime Report（2026-10-31）

| 项 | 结果 |
|---|---|
| task_type / phase | CODE_CHANGE / VALIDATION → READY（仅效果回执与收敛） |
| changed_files | `src-tauri/src/ag/tool_exec.rs`、`src-tauri/src/ag/capability_runtime.rs`、`docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md`、`docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md`、`docs/40_validation/A-VAL-X-V-04_Agent1.0验证卡.md`、本文件 |
| Rust | `cargo test`：544 lib + 5 core_chain + 4 sm_broadcast 全通过 |
| 前端 | 本工作包未改前端；沿用 689 用例基线 |
| 治理 | `governance:audit` findings/warnings 为空 |
| 验证边界 | 未重启真实应用；旧库收敛需下一次启动/写入时验证 |
| unresolved_items | 真实重启复验；21:17 回合已失败不可回滚，用户可重新发起 |
