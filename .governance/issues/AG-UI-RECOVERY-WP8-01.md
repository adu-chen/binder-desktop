# Issue Trace — AG-UI-RECOVERY-WP8-01

## 1. 用户可见症状

Agent 等待编辑器或用户时仍显示旋转的“工作中”状态；ProjectReport 已由运行时派生但界面不展示；重启后只恢复展示缓存，无法补回未缓存的报告事实；缺失 ProposalBatch 元数据的历史 PendingDiff 仍可能显示为可裁决卡。

## 2. 入口点

- `src/services/agService.ts::startTurn` 消费实时 TurnEvent。
- `src/services/agService.ts::ensureAgentSessionTurns` 恢复会话回合。
- `src/services/deService.ts::loadPersistedDiffs` 恢复 PendingDiff。
- `src/components/ag/MessageList.tsx` 呈现回合信息流和当前动作。

## 3. 相关规则

`BR-AG-STATE-001`、`BR-AG-PERSIST-001`、`BR-AG-UI-001`、`BR-AG-OBS-001`、
`BR-SYS-STATE-007`、`BR-SYS-STATE-008`、`BR-DE-DATA-001`、`BR-DE-STATE-001`。

## 4. 相关链路

`AG-TURN → AG-TOOL` 与 `AG-TURN → DE-DIFF`；本轮只收敛前端消费和恢复投影，不改变生产语义。

## 5. 生产者块

- `turn_events.rs`：append-only TurnEvent 与 `ag.list_turn_events(after_seq)` 安全投影。
- `tool_exec.rs`：从 EvidenceLedger 派生 ProjectReport。
- `de/routes.rs`：从 PendingDiff 直接字段返回 ProposalBatch 元数据。

## 6. 消费者块

- `agStore.ts`：按 seq 单调归约回合门控和 ProjectReport。
- `agService.ts`：实时事件消费、V1/V2 历史读取、事件补流和 Diff 投影。
- `MessageFlow.tsx` / `ProjectReportCard.tsx`：只读报告呈现。
- `deService.ts`：持久化 PendingDiff 恢复。

## 7. 路径上的空规则块

无。本项实现既有规则，不新增 RULE、CHAIN、TERM 或状态。

## 8. boundary 假设

- Channel 按协议 FIFO；重复、倒序和终态后事件由现有 seq reducer 拒绝。
- 历史展示缓存可缺失或为 V1，但 TurnEvent 与 DE 领域事实仍是运行真源。
- ProjectReport 只显示事件携带的运行时派生内容；Intent 内部 projection 不直接展示。

## 9. 运行时假设

- 恢复可按 turn 查询全部安全事件，选择同 turn、合法 seq、终态前的最新 ProjectReport，并以实际最大 seq 初始化游标。
- 恢复查询不注册 Channel、不调用 `ag_start_turn`，因此不会自动续跑。
- ProposalBatch id 非空，`1 <= group_index <= group_count`，实时事件与 DE projection 必须完全一致。

## 10. 疑似不变量破坏

- `MessageList` 对 `previewing/deciding` 也无条件渲染 spinner。
- `projectReports` 已进入 store，但没有 UI 消费者，也未从历史 TurnEvent 重建。
- `projectionFromDetail` 和 `hydratedDiffPatch` 以 `diffId/1/1` 补造缺失批次。
- `loadPersistedDiffs` 只建立空 Pending 卡，不校验 projection/batch。
- V1/V2 union 直接穿透到渲染，缺少显式读取边界转换。

## 11. 验证计划

- UI 覆盖活动动作与等待动作动画互斥、ProjectReport 各字段和 session/turn 隔离。
- Service 覆盖 V1 只读归一、事件恢复报告/最大 seq/终态冻结、不触发执行。
- Diff 覆盖实时/持久化 ProposalBatch 缺失、越界、不一致失败关闭和合法组序。
- 运行同输入 `test:plan` / `test:related`、治理审计、TypeScript 和 diff check。

## Validation Commands

- `npx tsc --noEmit`
- `npx vitest run src/services/agRuntime.test.ts src/components/ag/AgentUI.test.tsx src/services/deRuntime.test.ts src/components/de/DiffReview.test.tsx`
- `npm run governance:generate`
- `npm run governance:audit -- --issue .governance/issues/AG-UI-RECOVERY-WP8-01.md`
- `npm run test:plan -- --files [changed_files]`
- `npm run test:related -- --files [same_changed_files]`
- `git diff --check`

## Expected Behavior

用户看到的工作状态、等待、报告和修改卡完全由当前安全事件与领域事实决定；等待没有“仍在执行”的动画，报告说明已读/未读与不确定性，重启只恢复投影不恢复执行，损坏批次不可裁决。

## Scope Lock

```yaml
task_type: CODE_CHANGE
scope_id: AG-UI-RECOVERY-WP8-01
allowed_files:
  - .governance/active_scope.json
  - .governance/issues/AG-UI-RECOVERY-WP8-01.md
  - docs/00_core/A-CORE-C-R-01_ADUS.md
  - src/services/agService.ts
  - src/services/agRuntime.test.ts
  - src/services/deService.ts
  - src/services/deRuntime.test.ts
  - src/store/agStore.ts
  - src/components/ag/MessageList.tsx
  - src/components/ag/MessageFlow.tsx
  - src/components/ag/DecisionBar.tsx
  - src/components/ag/ProjectReportCard.tsx
  - src/components/ag/AgentUI.test.tsx
  - src/components/de/DiffCard.tsx
  - src/components/de/DiffReview.test.tsx
  - src/styles.css
  - scripts/test_governance.test.mjs
forbidden_files:
  - ADU.md
  - ADP.md
  - AGENTS.md
  - governance.config.json
  - docs/00_core/A-CORE-C-P-01_APC.md
  - src-tauri/**
  - cloud/dev-gateway/**
  - 需求、设计、规则、协议、状态机、验证卡和计划文档
confirmation: YES
scope_reason: 按已冻结 WP-AG10-08 收敛前端事实投影、历史只读恢复、V1 读取归一与 ProposalBatch 失败关闭，不新增能力。
```

## 人类决策

- 2026-09-04 已确认按推荐方案完整执行；2026-09-05 明确要求继续下一步。
- 2026-09-06 确认将 `scripts/test_governance.test.mjs` 纳入 Scope，仅同步本轮新增行为测试的 DE/AG 动态库存基线。

## Task Runtime Report

```yaml
status: READY
changed_scope:
  - Agent UI 工作态/等待态与 session/turn 隔离
  - ProjectReport 安全投影、展示与历史恢复
  - AgentTurnPayloadV1 读取归一为 V2
  - ProposalBatch 实时/持久化失败关闭与分卡裁决门禁
validation:
  typescript: passed
  direct_tests: 176 passed
  governance_generate: passed
  governance_audit_scoped: passed
  test_plan: passed
  test_related_frontend: "passed outside sandbox with identical input: 26 files, 608 tests"
  test_related_rust: "passed outside sandbox with identical input: lib 405, core_chain 5, sm_broadcast 4, doc-tests 0"
  test_related_governance: passed
  sandbox_note: "inside sandbox only docx_to_pdf_end_to_end_when_libreoffice_available failed because LibreOffice worker crashed with signal=6; identical outside-sandbox rerun passed"
  diff_check: passed
  system_validation: "not run: no interactive desktop test in this work package; automated UI/service/state coverage is recorded separately"
unresolved_items: []
```
