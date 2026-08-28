# Issue Trace — AG-CONFORMANCE-CODE-S3A：同一 AgentTurn 澄清受控重入

文档状态：A
任务类型：CODE_CHANGE
治理阶段：VALIDATION
任务状态：READY
创建时间：2026-08-27
上游基线：AG-CONFORMANCE-GOVERNANCE-S1 / S2；HD-S2-02 与 HD-S3A-01。

## 1. 用户可见症状

AgentTurn 遇到阻断歧义后虽然展示澄清请求，但运行时只能取消旧回合；用户补充信息会创建新 AgentTurn。界面语义是“继续澄清”，生产身份却断裂。

## 2. 入口点

- `BR-AG-STATE-001-API-AG-AG-TURN-018`：`ag_start_turn`。
- `BR-AG-STATE-001-RB-AG-AG-TURN-001` / `BR-SYS-STATE-007-RB-AG-AG-TURN-001`：SM-AG-01 转移 owner。
- `BR-AG-STATE-001-API-AG-AG-TURN-019`：活动回合取消入口。
- 前端入口只沿 ADUS 登记的 `src/services/agService.ts` AG-TURN 块及其直接 `startTurn` 消费者确认。

## 3. 相关规则

BR-AG-STATE-001、BR-SYS-STATE-007、BR-AG-PERSIST-001、BR-AG-VERIFY-001。TaskRun 仅保持当前兼容行为，不在本阶段整改 BR-SYS-STATE-008。

## 4. 相关链路

主链 AG-TURN；模型/工具继续执行只复用既有 AG-TOOL 下游，不改变其授权和执行语义。

## 5. 生产者块

- `turn.rs::transition`：新增 `ClarificationProvided` 唯一恢复事件。
- `turn_control.rs` 活动回合注册：承载 `{session_id, workspace_id, turn_id, phase, task_run_id}`，负责等待态停放、原子认领和取消竞争。
- `turn_runtime.rs::ag_start_turn`：通过显式 `clarification_resume` 选择 `Idle + Send` 或 `AwaitingClarification + ClarificationProvided`，不得按最近回合猜测。
- `agService.ts::startTurn`：同 session 存在澄清等待时复用原 turn_id、消息身份和已提交上下文。

现有 `history::save_turn` 已按 turn_id upsert，数据库 status 无 CHECK 约束；本方案不需要 schema 变更，也不修改 history owner。

## 6. 消费者块

- `AgTurnEvent::ClarificationRequired` 前端消费者把回合投影为 clarifying、释放输入门控并保留恢复身份。
- Composer 继续调用同一 `startTurn`；service 内部根据当前 session 的澄清等待身份选择受控重入，不改变 Composer API。
- `agStore` 复用原 user/assistant 消息：补充输入并入原 user message，assistant 延续同一流式消息。
- 后端恢复后重新装配 AgentReadContext / IntentResult；仍须经过原 AwaitingGate → Streaming 唯一路径。

## 7. 路径上的空规则块

S2 已确认活动回合注册和前端 `startTurn` 等关键符号未清晰注册；本阶段只补与澄清恢复直接相关的 owner，不顺带治理全部未标注符号。

## 8. boundary 假设

恢复输入必须包含同 session、同 workspace、同 turn_id 的稳定身份；不能把相同文本、最近活动回合或取消标记当作恢复授权。

## 9. 运行时假设

阻断澄清 command 返回时 AgentTurn 保持 AwaitingClarification；后续补充输入以原 turn_id 重入，先原子认领等待态，再触发 ClarificationProvided → Assembling。取消与恢复竞争时只能有一个结果生效。

## 10. 疑似不变量破坏

- history `save_turn` 已确认使用 `ON CONFLICT(turn_id) DO UPDATE`，不会重复新建；需要保证前端传入聚合后的原请求和补充说明。
- ActiveAgentTurn lease 当前会在 command 返回时移除身份，须新增明确 AwaitingClarification phase，使 Drop 只保留该非终态。
- 前端当前始终生成新 turn_id，并先取消澄清回合；必须替换为同 session 映射和原 turn_id 重入。
- TaskRun 当前无条件创建；本阶段不得借澄清改造顺带修正该问题。

## 11. 验证计划

1. 由 ADUS 块和直接调用关系读取完整后端入口、活动回合控制、IPC 事件、前端入口与直接测试。
2. 确定无需 schema 变更的最小稳定身份方案；若需要 schema，触发 Stop Condition。
3. 先补状态机及失败关闭测试，再实现后端和前端受控重入。
4. 运行 changed_files 的 test:plan / test:related、Rust 定向测试、test:governance、governance:generate 与 governance:audit。
5. 核对生成 ADUS 的 AG-TURN 视图可见唯一澄清恢复 owner。

## 12. 实现结果

- `SM-AG-01` 新增唯一恢复事件 `ClarificationProvided`：只允许
  `AwaitingClarification → Assembling`，终态和其他事件不能借此重入。
- 活动回合注册表按 `{session_id, workspace_id, turn_id}` 保留澄清等待身份和原
  `task_run_id`；恢复采用一次性原子认领，错误 session/workspace/turn、重复恢复和终态恢复均失败关闭。
- 阻断澄清在发布 `ClarificationRequired` 后返回 command，不再用 pending future 挂起，也不再以
  abort 伪造 `Cancelled`；等待态仍可由原身份显式取消。
- 前端按 session 复用原 turn_id 和原 user/assistant 消息；补充说明并入原请求，`ReferencePayload`、
  兼容 `mentions/blockRefs` 均保持并去重，不创建第二个 AgentTurn，也不先取消旧回合。
- 编辑器快照在后端调用前失败时恢复到原 clarifying 投影并保留等待身份；错误恢复身份在任何历史
  session/turn 草稿写入前即被拒绝。
- `history::save_turn` 继续按 turn_id upsert；无需数据库 schema 变更。TaskRun 的无条件创建现状未在
  本阶段改变。

## 13. 规则注册结果

- 后端恢复 guard：`BR-SYS-STATE-007-GUARD-AG-AG-TURN-034`、
  `BR-AG-STATE-001-GUARD-AG-AG-TURN-034`。
- 后端等待态 effect：`BR-SYS-STATE-007-EFFECT-AG-AG-TURN-034`、
  `BR-AG-STATE-001-EFFECT-AG-AG-TURN-034`。
- 前端同回合续接 effect：`BR-SYS-STATE-007-EFFECT-AG-AG-TURN-035`、
  `BR-AG-STATE-001-EFFECT-AG-AG-TURN-035`。
- `ag_start_turn` API 块同时登记 `BR-AG-STATE-001` 与 `BR-SYS-STATE-007`，ADUS 可由 AG-TURN
  唯一定位入口、状态机、身份 guard、等待 effect 和前端消费入口。

## 14. Task Runtime Report

```yaml
scope_id: AG-CONFORMANCE-CODE-S3A
task_type: CODE_CHANGE
final_state: READY

mapping_chain:
  status: complete
  evidence:
    - AG-TURN 的 ag_start_turn API、SM-AG-01、后端身份 guard/effect 与前端 effect 已在 ADUS 注册
    - governance:audit 通过，298 个代码块闭合且唯一，0 governance finding

test_impact:
  changed_files: active_scope、Issue Trace、ADUS、V-S6 验证卡、测试治理基线、AG Rust/TS 实现与测试
  direct_modules: AG, CORE, PCG, REF, SYS, WS
  affected_modules: ACCT, AG, AST, CBT, CORE, DE, DOC, ED, ENT, KB, PCG, PRV, REF, SYS, TPL, WS
  selection: frontend-full + TypeScript + rust-full + governance:audit

validation:
  - targeted_frontend: PASS（澄清非终态 4/4）
  - targeted_rust: PASS（clarification 3/3）
  - test_related_non_sandbox: PASS
  - frontend_full: PASS（26 files，591/591）
  - typescript: PASS
  - rust_full: PASS（lib 361/361；core_chain 5/5；sm_broadcast 4/4；doc-tests 0）
  - governance_audit: PASS（0 governance finding；test governance 104 assets / 945 tests）
  - diff_check: PASS
  - real_ui_provider: NOT_RUN

environment_evidence:
  - restricted_sandbox: LibreOffice E2E 两次以 DOC_PREVIEW_WORKER_CRASH signal=6 失败
  - registered_constraint: DOC-LO-SANDBOX-VALIDATION-01 已登记该 AppKit 沙箱限制
  - non_sandbox: 同一 LibreOffice E2E 单测通过，随后完整 test:related 通过

unresolved_items:
  - 真实 Tauri/WKWebView 与真实 provider 的 V-S6 系统验证未执行，验证卡保持 DESIGNED
  - 简单 AgentTurn 的 TaskRun 按需创建留待 S3-B
  - 跨文档显式目标集扩展与目标集外拒绝留待后续独立 Scope
```

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-08-27 | v1.0 | 人类确认同 turn_id 受控重入方案；建立独立 S3-A CODE_CHANGE Scope。 |
| 2026-08-27 | v1.1 | 完成直接调用链确认：无需 schema 迁移；范围修正为 ipc/mod.rs，并纳入 agStore 同回合消息续接。 |
| 2026-08-28 | v1.2 | 完成同一 AgentTurn 受控重入、一次性身份 guard、等待态取消、调用前失败恢复与兼容引用续传；ADUS 注册和非沙箱完整相关回归通过，进入 READY。 |
