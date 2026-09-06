# Issue Trace — AG-PROPOSAL-BATCH-WP7-01

## 1. 用户可见症状

一次 `propose_edit` 即使修改多个不连续内容块，也只生成一张 DiffCard，用户无法分别接受或拒绝各块修改。

## 2. 入口点

- `src-tauri/src/ag/edit_runtime.rs::ag_propose_edit` 校验并持久化 ChangeProposal。
- `src-tauri/src/ag/turn_tool_loop.rs` 记录 ToolResult 并发送 DiffProposed。
- `src/services/agService.ts` 将 DiffProposed 投影到 deStore。
- `src/components/ed/Editor.tsx` 将当前编辑器 PendingDiff 预览到 LogicalState。

## 3. 相关规则

`BR-DE-DATA-001`、`BR-DE-DATA-002`、`BR-DE-DATA-003`、`BR-DE-STATE-001`、
`BR-ED-STATE-001`、`BR-ED-DATA-001`、`BR-AG-DATA-003`、`BR-SYS-STATE-008`。

## 4. 相关链路

`AG-TOOL → DE-DIFF → ED-EDIT → ED-DECOR`；后台落盘和资源操作不在本工作包扩展范围。

## 5. 生产者块

- `RB-AG-AG-TOOL-016`：BlockOperation 校验与规范化。
- `EFFECT-AG-AG-TOOL-017`：ChangeProposal 到 PendingDiff/DiffProjection。
- `GUARD-DE-DE-DIFF-005`：PendingDiff 生成前锚校验。
- `RB-AG-AG-TOOL-018`：工具结果和 DiffProposed 事件输出。

## 6. 消费者块

- `agService.ts`：ToolResult 投影缓存、DiffProposed 消费和消息片段。
- `deStore.ts`：DiffCard/GreenReview 单源投影。
- `Editor.tsx`：当前编辑器 PendingDiff 前向预览。
- `deService.ts`：逐卡接受、拒绝、失效和恢复。
- `DiffCard.tsx`：批次关系与逐卡裁决入口。

## 7. 路径上的空规则块

未发现需要新增规则的空块；本项只实现已登记的 ProposalBatch 规则链。

## 8. boundary 假设

- 1.0 多卡仅适用于当前编辑器稳定 blockId 的 `block_ops` 和 `replace_block`；本轮先闭合一次工具调用可表达多目标的 `block_ops`。
- `replace_document/style_ops/table_ops/image_ops/open_session` 继续保持单卡。
- 每张卡只保存本组操作、目标摘要和局部 inverse；同批卡共享完全相同的生成基线。
- ProposalBatch 只关联，不提供批次终态或全局接受/拒绝。

## 9. 运行时假设

- 后端先校验完整 BlockOperation 集，再分组并在单个数据库事务中写入全部 payload/PendingDiff/投影。
- 前端必须收齐 `group_count` 且验证 index、基线、路径和目标互斥后，才执行一次 ED 前向事务。
- 一次前向事务完成后，每张卡独立保持 Pending；拒绝只执行本卡 reverse。

## 10. 疑似不变量破坏

- `AgentProposeEditResult` 只有单个 `diff_id/projection`，无法表达 ProposalBatch。
- `turn_tool_loop` 把 `proposal_batch_id=diff_id` 且 `group_count=1` 写死。
- 旧 `batch:{id,index,total}` 表达的是模型分批进度，不是 ProposalBatch 裁决分组，且仍在新写入。
- Editor effect 在每张卡到达时立即逐卡前向应用，会让第一张卡推进整篇基准并破坏批内原子性。
- 历史 projection 未持久化 ProposalBatch 身份、共享基线和组序，恢复无法重建真实卡片关系。

## 11. 验证计划

- 后端覆盖默认逐块拆卡、显式相邻不可分合组、非相邻强绑拒绝、组序稳定和整批原子持久化。
- 事件覆盖同一 ProposalBatch 的完整 DiffProposed fan-out 和 ToolResult `projections[]`。
- 前端覆盖缺组/重复/乱序/基线不同/目标重叠失败关闭，以及完整 cohort 一次前向事务。
- 覆盖逐卡接受/拒绝后兄弟卡保持 Pending，拒绝仅反向本卡目标。
- 扫描生产代码不再新写 `batch:{id,index,total}`，运行同输入 test:plan/test:related 与治理审计。

## Expected Behavior

同一次结构化修改先形成 ProposalBatch；不连续或可独立裁决内容块生成多张 DiffCard，相邻且显式声明语义
不可分的块可形成一张卡。同文档同批卡收齐后一次预览，之后分别接受或拒绝，任何一张卡的裁决不改变兄弟卡终态。

## Validation Commands

- `npm run governance:generate`
- `npm run governance:audit -- --issue .governance/issues/AG-PROPOSAL-BATCH-WP7-01.md`
- `npm run test:plan -- --files [...]`
- `npm run test:related -- --files [...]`
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`
- `cargo check --manifest-path src-tauri/Cargo.toml --lib --tests`
- `git diff --check`

## Scope Lock

```yaml
task_type: CODE_CHANGE
governance_phase: IMPLEMENTATION
allowed_files:
  - .governance/active_scope.json
  - .governance/issues/AG-PROPOSAL-BATCH-WP7-01.md
  - docs/00_core/A-CORE-C-R-01_ADUS.md
  - src-tauri/src/db/mod.rs
  - src-tauri/src/types/mod.rs
  - src-tauri/src/ag/block_ops.rs
  - src-tauri/src/ag/bdocx_capabilities.rs
  - src-tauri/src/ag/edit_types.rs
  - src-tauri/src/ag/edit_runtime.rs
  - src-tauri/src/ag/prompts.rs
  - src-tauri/src/ag/turn_tool_loop.rs
  - src-tauri/src/de/generate.rs
  - src-tauri/src/de/lifecycle.rs
  - src-tauri/src/de/store.rs
  - src-tauri/src/de/routes.rs
  - src-tauri/src/workspace/diff_apply.rs
  - src-tauri/src/workspace/routes.rs
  - src-tauri/tests/core_chain.rs
  - src/types/diffProjection.ts
  - src/services/agService.ts
  - src/services/agRuntime.test.ts
  - src/store/deStore.ts
  - src/services/deService.ts
  - src/services/deRuntime.test.ts
  - src/components/ed/Editor.tsx
  - src/components/de/DiffCard.tsx
  - src/components/de/DiffReview.test.tsx
  - scripts/test_governance.test.mjs
forbidden_files:
  - ADU.md、ADP.md、AGENTS.md、governance.config.json、APC
  - 需求、设计、规则、协议、状态机、验证卡和计划文档
  - cloud/dev-gateway/**
  - WorkspaceOperation、后台多卡、跨文档事务、批量接受或保存
  - WP8 UI/恢复总收口与 WP9 其他兼容清理
confirmation: YES
scope_reason: 只实现冻结 WP7 的 ProposalBatch 分组、完整批次事件/持久投影、一次前向预览和逐卡独立裁决。
```

## 人类决策

- 2026-09-04 已确认按推荐方案完整执行；2026-09-05 明确要求继续下一项，`confirmation: YES`。

## Task Runtime Report

```yaml
status: READY
changed_scope:
  - block_ops 默认逐内容块拆卡；相邻且同 adjudicationGroup 的不可分块受控合卡
  - ProposalBatch、PendingDiff 和 DiffProjection 使用显式 proposal_batch_id/group_index/group_count
  - proposal_batches 与 pending_diffs v18 迁移；整批 payload/diff 单事务持久化、失败零残留
  - ToolResult projections[] 与 DiffProposed 逐卡事件共享批次身份和稳定组序
  - 前端收齐 cohort 后校验路径、kind、基线、组序和目标互斥，再执行一次结构化前向事务
  - 每张 DiffCard 独立裁决；兄弟卡保持 pending；旧 batch:{id,index,total} 新写入删除
validation:
  governance_generate: passed; ADUS @GOV blocks=324, test assets=110, test cases=998
  governance_issue_audit: passed
  legacy_batch_write_scan: passed; zero exact legacy production writes
  git_diff_check: passed
  cargo_fmt_check: passed
  cargo_check_lib_tests: passed
  typescript: passed
  frontend_targeted: 137/137 passed
  rust_proposal_batch: 3/3 passed
  rust_schema_and_v18_migration: 2/2 passed
  frontend_full: 600/600 passed
  rust_full_sandbox: 404/405 passed; only LibreOffice Worker signal=6（环境限制，非实现断言失败）
  rust_full_host: lib 405/405 passed; integration core_chain 5/5; sm_broadcast 4/4; doc tests 0/0
  test_plan_same_input: passed; selected frontend full + TypeScript + Rust full + governance audit
  test_related_same_input: passed outside sandbox with approved escalation; exit code 0
unresolved_items: []
```
