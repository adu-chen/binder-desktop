# Issue Trace — AG-1.0-IMP-CLARIFICATION-01：澄清非终态（R5-A）实现

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-14

## 1. 用户可见症状

冻结基线 R5-A 要求"澄清是等待用户输入的非终态"，但当前实现把澄清建模为 **Completed 终态**：后端
`turn.rs` 转移 `(Assembling, ClarificationRequired) → Completed`，`lib.rs` 澄清分支锁存
`TerminalKind::Completed` 并广播 `Done{stop_reason:"clarification_required"}`；前端 `agStore.ts` 无澄清
等待态、`agService.ts` 把该 done 投影为 completed。澄清回合被终结为"完成"，无法保持"等待用户输入"，
与冻结基线（A-CORE-X-D-02 §4.2）及 D3 登记的设计契约（A-AG-M-S-01 §2.1，BR-SYS-STATE-007）冲突。

## 2. 入口点

- `src-tauri/src/lib.rs:8341-8366`：`ag_start_turn` 主路由的 `intent_result.is_blocking()` 分支
  （发澄清 Chunk → 锁存 Completed → 转移 → Done 广播）。
- `src-tauri/src/ag/turn.rs:130-156`：SM-AG-01 `transition()` 的 `(Assembling, ClarificationRequired) =>
  Completed`。
- 前端投影：`src/store/agStore.ts:40`（AgentTurnStatus 枚举）、`src/services/agService.ts:51-57/1061`
  （AgTurnEvent 与 done 处理）。

## 3. 相关规则

- `BR-SYS-STATE-007`（Agent 1.0 运行时事件真值：澄清是等待输入的非终态、首终态锁存、迟到事件无副作用；
  D1 注册，A-SYS-I-P-01 §3.1）
- `BR-AG-STATE-001`（SM-AG-01：gatePassed 前置、blocking ambiguity 无模型澄清路径、唯一终态）
- 间接：BR-CORE-STATE-001~003（三分流，本次不涉及）

## 4. 相关链路

AG-TURN（回合生命周期）；IPC-CALL（AgTurnEvent 广播）。

## 5. 生产者块

- `BR-AG-STATE-001-RB-AG-AG-TURN-001`（src-tauri/src/ag/turn.rs:119-126，转移引擎 @GOV）
- `BR-AG-STATE-001`（src-tauri/src/ag/turn.rs 内联测试，:253-255 断言 ClarificationRequired→Completed）
- `ag_start_turn` 主路由 @GOV（src-tauri/src/lib.rs:7768 附近，AG-TURN 链路入口）

## 6. 消费者块

- `src/services/agService.ts`（AgTurnEvent 消费、done 投影）
- `src/store/agStore.ts`（AgentTurnStatus 投影与 UI 状态）
- `emit_ag01` 广播（src-tauri/src/lib.rs，状态机投影）

## 7. 路径上的空规则块

本次改动触及的 turn.rs/lib.rs 均有 @GOV 标注。`ag/tool_registry.rs` 无 @GOV 属既有遗漏，不在本 Scope。

## 8. boundary 假设

- `TurnState::Completed` 语义：回合产出处置完成的终态；澄清不再落入该终态。
- `terminal_latch` 语义：首个终态获胜；澄清不是终态，不锁存 latch。
- `BR-AG-STATE-001`"blocking ambiguity 无模型澄清路径"保持：澄清期间不调用门禁/网关/工具。

## 9. 运行时假设

- 澄清回合进入 `AwaitingClarification` 非终态并广播；不锁存终态。
- 用户补充输入后：按既有回合发起路径创建新回合（新 send），原澄清回合作为非终态历史呈现；取消可在澄清
  等待阶段生效（任一活动阶段取消）。
- 澄清 Chunk 文本与 `Done` 语义调整后，前端不把澄清投影为 completed。

## 10. 疑似不变量破坏

- `BR-AG-STATE-001`：澄清不进入 Streaming、不触发模型调用/工具/写盘——保持。
- `terminal_latch`：澄清不占用首个终态；后续真实终态（完成/取消）仍可锁存——保持首终态语义。
- 三分流/其他链路不涉及。

## 11. 验证计划

- 更新 `src-tauri/src/ag/turn.rs` 内联测试：`(Assembling, ClarificationRequired) => AwaitingClarification`，
  断言 `is_terminal(AwaitingClarification) == false`。
- 补 `src/services/agRuntime.test.ts`：澄清事件投影为 clarifying（非 completed）；澄清后取消可用。
- 运行 `npm run test:plan -- --files [changed_files]` 审阅影响计划，再运行同输入的
  `npm run test:related`；运行 `npm run governance:audit`。
- 同步 `docs/01_modules/AG/A-AG-I-P-01_代理接口协议.md`（AgTurnEvent / AgentTurnStatus 契约）。

## 12. 文件范围结论（Scope Lock 输入）

允许修改：
- `src-tauri/src/ag/turn.rs`（TurnState 增 AwaitingClarification、转移改、is_terminal、内联测试）
- `src-tauri/src/lib.rs`（澄清分支：不锁存 Completed、广播非终态）
- `src/services/agService.ts`（AgTurnEvent 澄清语义）
- `src/store/agStore.ts`（AgentTurnStatus 增 clarifying）
- `src/services/agRuntime.test.ts`（补澄清非终态测试）
- `docs/01_modules/AG/A-AG-I-P-01_代理接口协议.md`（接口契约同步）

禁止修改：工作区其他任何未提交改动（用户资产）；其余 src/src-tauri 文件；其他模块文档；治理控制文档；
governance.config.json。

## 13. 人类确认记录

2026-08-14：人类经 AskUserQuestion 明确选择本 Scope（澄清非终态）作为第一个 IMPLEMENTATION Scope，
确认范围与 allowed_files。实现中发现 `ipc/mod.rs`（AgTurnEvent 定义处）与
`scripts/test_governance.test.mjs`（AG 前端测试计数基线 92→94）需纳入，人类批准 Scope 扩展（留痕）。

## 14. Task Runtime Report

- task_type：`CODE_CHANGE`
- governance_phase：`IMPLEMENTATION`
- final_state：`READY`
- issue_trace：本文件
- changed_files：
  - `src-tauri/src/ag/turn.rs`（TurnState 增 AwaitingClarification、澄清转移非终态、AwaitingClarification+Cancel→Cancelled、@GOV rules 增 BR-SYS-STATE-007、澄清非终态测试、covers 拆行）
  - `src-tauri/src/lib.rs`（澄清分支改为 ClarificationRequired 事件 + 不锁存终态/不发 Done + Abortable 挂起等待取消；注册守卫允许已取消回合被新回合取代）
  - `src-tauri/src/ipc/mod.rs`（AgTurnEvent 增 ClarificationRequired）
  - `src/services/agService.ts`（AgTurnEvent 增 clarification_required；clarifyingTurn 追踪；发送前取消澄清回合；done/error 清理）
  - `src/store/agStore.ts`（RustTurnState 增 AwaitingClarification；ACTIVE_TURN_STATES 不含之；AgentTurnStatus 增 clarifying）
  - `src/services/agRuntime.test.ts`（澄清非终态 2 用例）
  - `docs/01_modules/AG/A-AG-I-P-01_代理接口协议.md`（AgTurnEvent 七分支、AgentTurnStatus clarifying、澄清契约说明）
  - `scripts/test_governance.test.mjs`（AG 前端测试计数基线 92→94，人类批准）
  - `docs/00_core/A-CORE-C-R-01_ADUS.md`（generate 更新：测试用例 895→898、BR-SYS-STATE-007 覆盖）
- mapping_chain：BR-SYS-STATE-007（D1 注册）→ turn.rs @GOV + covers 引用，覆盖缺口从 3 减到 2（剩余
  BR-SYS-STATE-008/BR-SYS-DATA-007 为未实现部分，留待后续 Scope）。
- test_impact：
  - changed_files：turn.rs、lib.rs、ipc/mod.rs、agService.ts、agStore.ts、agRuntime.test.ts
  - direct_modules：AG、SYS（IPC）、CORE
  - selection：test:plan 判定前端全量 + tsc + cargo 全量 + audit（lib.rs 广泛影响升级全量）
- validation：
  - `npm run test:related`（npm test 前端全量） # passed：26 files / 568 tests（含澄清 2 用例）
  - `npx tsc --noEmit` # passed
  - `cargo test`（Rust 全量） # passed：330 tests（含 turn 澄清测试）
  - `npm run governance:generate` # passed（测试用例 895→898；BR-SYS-STATE-007 进入 covered）
  - `npm run governance:audit` # failed（2 项，较 D1-D4 的 3 项收敛 1 项）：BR-SYS-STATE-008、BR-SYS-DATA-007
    无代码 @GOV 引用/测试 covers（未实现部分，预期）；其余维度全部 passed
  - `git diff --check` # passed
- automated_tests：前端 568 + Rust 330 全过。
- stop_conditions：无。
- unresolved_items：澄清后用户回复的 UI 续接细节与澄清等待视觉由前端迭代；BR-SYS-STATE-008（连续 Proposal
  前卡失效）与 BR-SYS-DATA-007（外部内容/记忆）为下一实现 Scope 的债务；澄清等待取消在应用关闭场景下的
  广播由 SM-SYS-02 关闭屏障兜底（A-SYS-M-S-02），未单独验证。
