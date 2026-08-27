# Issue Trace — WS-RESOURCE-WRITE-GUARD-01：资源写入口统一护栏（A1/A3 修复）

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-22
上游基线：WS-OPERATION-IMPLEMENTATION-01 审计结论（A1/A3）、A-WS-M-T-01/02、A-AG-M-T-01、A-WS-M-D-01（REQ-WS-006/008/009/010），以及 2026-08-22 人类确认的四项决策（冻结文档修订授权、BR-WS-STATE-009 归属 WS-FS、WS-OPERATION 链推进 active、人工删除可恢复为硬性验收）。

## 1. 用户可见症状

审计发现（非用户症状）：① Edit 意图下 `propose_edit` 指向不存在路径时，内容 Diff 接受后会创建文件，绕过 CreateDocumentBundle 前置裁决链；② 人工文件树的新建/重命名/删除/删目录直写底层命令，删除不保存快照（不可恢复）、无 dirty 阻断（未保存编辑静默丢失）、不失效受影响内容 Diff。

## 2. 入口点

- `src-tauri/src/lib.rs`：`ag_propose_edit`（L7720，`target_exists` 计算 L7776）、`route_create_doc/route_rename_doc/route_delete_doc/route_delete_folder`（L1060-1146）、`route_restore_node`（L1917-1958）。
- `src-tauri/src/ag/tool_registry.rs`：`validate_requested_tool_for_grant` propose_edit 分支（L178-188）。
- `src-tauri/src/workspace/operation.rs`：`has_dirty_affected_session`（L1914）、`snapshot_resource`（L1939）、`invalidate_affected_diffs`（L2244）、`restore_intents_from_entry`（L488-661，依赖 plan 缺失需兼容）。
- `src/components/ws/FileTree.tsx`：`doDelete`（L279-290）、`doDeleteFolder`（L294-310）、`submitRename`（L239-254）、`submitCreate`（L203-236）。
- `src/store/tabsStore.ts` / `src/store/edStore.ts`：关闭前 dirty 检查（`isDirtyTabState`）。

## 3. 相关规则

- `X-FORBID-006`：两条唯一写路径——内容 Diff 不得创建资源；资源操作不得生成 DiffCard。
- `X-FORBID-004`：所有路径限定在活动 Workspace。
- `BR-WS-STATE-003` / `BR-WS-DATA-003` / `BR-WS-OBS-002` / `BR-WS-PERSIST-003`：WorkspaceOperation 前置裁决、冻结计划、影响摘要、fail-fast 补偿与可恢复性。
- `BR-WS-DATA-001` / `BR-WS-OBS-001`：文件操作守界与可观测性。
- `BR-WS-STATE-009`（本次新增）：人工文件树资源操作安全底线——删除/重命名必须复用 dirty 阻断、快照/时间轴与内容 Diff 失效安全件，不得静默丢弃未保存编辑；删除必须可经 WorkspaceTimeline 恢复。
- `REQ-WS-008` / `REQ-WS-010`：REQ_MAP 增补 → `BR-WS-STATE-009`。

## 4. 相关链路

`AG-TURN → AG-TOOL → WS-OPERATION`（Agent 资源路径，本次收窄入口）；`WS-FS`（人工文件树路径，本次补齐安全底线）；`WS-TIMELINE`（人工删除条目投影与恢复）；`DE-DIFF → ED-EDIT → WS-SAVE`（内容 Diff 失效联动，仅消费失效，不改 DE/DOC 核心）。

## 5. 生产者块

- `BR-WS-STATE-003-EFFECT-WS-WS-OPERATION-003`（operation.rs：execute 事务）——复用其 snapshot/entry/diff 失效能力。
- `BR-WS-DATA-003-EFFECT-WS-WS-TIMELINE-003`（operation.rs：append_operation_entry）——人工删除条目复用。
- `BR-AG-STATE-002-QUERY-AG-AG-TOOL-012`（tool_exec.rs）与 `ag_propose_edit`（lib.rs）——F1 防御点。

## 6. 消费者块

- `src/components/ws/FileTree.tsx` / `TimelinePanel.tsx`：人工删除条目经统一 WorkspaceTimeline 呈现与恢复。
- `src/store/deStore.ts`：受影响 Diff 失效投影。
- `src/components/ag/DecisionBar.tsx`：恢复经新 DecisionRequest 裁决（复用既有）。

## 7. 路径上的空规则块

`fileops.rs` 的 `create_doc/create_folder/rename_doc/delete_doc/delete_folder` 为 `rules: []`（CALLER_OWNS_RULE，规则由调用方持有）——本次 `route_*` 作为新规则消费方，fileops 保持 IO 层无规则，不新增空规则块。

## 8. boundary 假设

- `lifecycle_assumption=plan_immutable_until_terminal`：人工删除条目不绑定 plan（plan_id 空），恢复时构造新计划，不复用旧授权。
- `document_snapshot=confirm_time_revalidation`：人工删除在删除时点采集快照，恢复以快照为基线重验。
- `document_snapshot=create_bundle_triple_guard`：不涉及（本次不改 CreateDocumentBundle 核心）。

## 9. 运行时假设

- `runtime_guard=workspace_and_plan_identity`：人工删除条目以 workspace_id + entry_id 定位，恢复走既有身份校验。
- `runtime_guard=dirty_and_content_diff_impact`：人工删除/重命名命中 dirty 会话必须阻断；成功后使受影响有效 Diff 失效。
- `runtime_guard=idempotent_receipt`：重复删除（目标已不存在）返回既有错误，不产生重复条目。

## 10. 疑似不变量破坏

- A1：`propose_edit` 到不存在路径 → `ReplaceDocument` → 接受后 `apply_diff_sm` 创建文件，违反 X-FORBID-006"内容 Diff 不得创建资源"。
- A3：人工文件树删除无快照/时间轴（不可恢复）、无 dirty 阻断（未保存编辑丢失）、无 Diff 失效——与 REQ-WS-010 接受标准不符。
- 文档双源冲突：A-WS-M-T-01（人工直写豁免）与 A-WS-M-T-02 §3（用户文件树动作进 Plan 链）并存；A-AG-M-T-01 空基准新建旧表述未收敛。

## 11. 验证计划

- F1：`propose_edit` 不存在路径 → `AG_TARGET_NOT_EXISTS` 且零 diff 持久化；存在路径不回归。
- F2 后端：`route_delete_doc` dirty 阻断、快照 + entry（restore_kind=restore_snapshot）、Diff 失效；非空目录递归快照；rename 源 dirty 阻断与 Diff 失效；无 plan entry 恢复生成新计划/新 DecisionRequest。
- F2 前端：FileTree dirty 删除提示；删除/重命名后标签、本地 Diff、引用同步。
- 治理：为 BR-WS-STATE-009 补真实 @GOV owner 与 `// covers:`；WS-OPERATION 链 status 推进 active 并补 integration_test；运行 `test:plan`、同输入 `test:related`、`governance:generate`、`governance:audit`（完整读取未裁剪原始日志）、`test:governance`、`git diff --check`；语义审计（BR-WS-STATE-003/DATA-003/OBS-002/PERSIST-003 候选）留痕入 Runtime Report。
- 系统验证：人工文件树删除→时间轴可恢复；dirty 删除被阻断；propose_edit 不存在路径被拒——真实 Tauri/WKWebView 观察（自动化通过不代替）。

## 分阶段实施门禁

1. **Gate 1 · F1 工具层防御**：`ag_propose_edit` 不存在路径拒绝（+tool_registry 兜底可选）；A-AG-M-T-01 旧表述收敛；定向测试通过。
2. **Gate 2 · F2 后端**：operation.rs 可见性提升与 restore 无 plan 兼容；`route_delete_doc/delete_folder/rename_doc` dirty 阻断、快照、entry、Diff 失效；Rust 定向测试通过。
3. **Gate 3 · F2 前端**：FileTree dirty 提示与删除/重命名后同步；前端定向测试通过。
4. **Gate 4 · 治理收口**：A-WS-M-T-01/02、A-WS-M-D-01 标注修订（BR-WS-STATE-009 / CHAIN active / REQ 口径）→ generate → audit → test:governance → 全量回归 → Runtime Report。

每个 Gate 先运行定向测试并记录未解决项；上一 Gate 契约未稳定不进入下一 Gate。

## 12. 实施进度

- **Gate 1 已落地（F1 工具层防御）**：`ag_propose_edit` 在计算 `target_exists` 后调用 `guard_propose_edit_target`，目标不存在一律返回 `AG_TARGET_NOT_EXISTS` 并引导 `plan_workspace_operation`，任何意图（含 Edit 之外）均不产生 diff 持久化、不创建资源；`resolve_agent_edit_operation` 中 `None if has_content && !target_exists => ReplaceDocument` 分支保留但标注生产不可达（F1）。A-AG-M-T-01 移除"新文档以空基准 ChangeProposal 创建"旧表述（v2.48）。定向测试 `guard_propose_edit_target_rejects_non_existent_path`（covers: BR-AG-STATE-002）通过。
- **Gate 2 已落地（F2 后端）**：`workspace::operation` 新增 `record_manual_delete` / `record_manual_rename` 安全记录器——dirty 阻断（`WS_DELETE_DIRTY_TARGET` / `WS_RENAME_DIRTY_TARGET`）、删除快照（`wop-snapshot-{operation_id}`）、`append_operation_entry`（restore_kind=manual_delete/manual_rename、plan_id 空）、`invalidate_affected_diffs`（复用既有机制）；`restore_intents_from_entry` 支持无 plan 条目走 `manual_restore_intents` 构造反向意图；`route_delete_doc/delete_folder/rename_doc` 改走安全记录器；`has_dirty_affected_session/snapshot_resource/invalidate_affected_diffs` 提升 pub。定向测试 `manual_delete_blocks_dirty_and_writes_recoverable_entry`、`manual_rename_blocks_dirty_and_invalidates_affected_diffs`（covers: BR-WS-STATE-009）通过。
- **Gate 3 已落地（F2 前端）**：FileTree 新增 `dirtyDocTabsUnder` 辅助，删除/重命名 confirm 文案含"未保存修改"提示，删除文案改为"会记录到工作区时间轴，可从时间轴恢复"；空目录 + dirty 会话时同样必须确认。`WorkspaceUI.test.tsx` 新增 dirty 空目录删除确认测试通过（WS 前端基线 41→42，人类 2026-08-24 批准）。
- **Gate 4 已落地（治理收口）**：A-WS-M-T-01/02、A-WS-M-D-01 标注修订（BR-WS-STATE-009 注册、CHAIN WS-OPERATION active + integration_test、REQ-WS-008/010 REQ_MAP、人工文件树不进入 WS-OPERATION 链说明）；`governance:generate` 通过；`governance:audit` 48 项全部通过、governance_findings=0；`test:governance` 19/19；tsc、前端全量 589、Rust 全量 353+5+4 通过；`git diff --check` 通过。语义审计留痕见 §13。

## 13. Task Runtime Report

task_type: CODE_CHANGE
governance_phase: IMPLEMENTATION
final_state: READY
issue_trace: `.governance/issues/WS-RESOURCE-WRITE-GUARD-01.md`

changed_files:

- `src-tauri/src/lib.rs`：F1 `guard_propose_edit_target`（`ag_propose_edit` 目标不存在拒绝，covers BR-AG-STATE-002）；F2 `route_delete_doc/route_delete_folder/route_rename_doc` 改走 `record_manual_delete/record_manual_rename`。
- `src-tauri/src/workspace/operation.rs`：`record_manual_delete` / `record_manual_rename`（dirty 阻断、删除快照、时间轴条目、Diff 失效，@GOV BR-WS-STATE-009-EFFECT-WS-WS-FS-009/010）；`restore_intents_from_entry` 无 plan 兼容 + `manual_restore_intents` 反向意图；三函数提升 pub；两个新增内联测试。
- `src/components/ws/FileTree.tsx`：`dirtyDocTabsUnder`、删除/重命名 dirty 确认与文案调整。
- `src/components/ws/WorkspaceUI.test.tsx`：dirty 空目录删除确认测试（covers BR-WS-STATE-009 前端）。
- `scripts/test_governance.test.mjs`：WS 前端测试基线断言 41→42（人类 2026-08-24 批准 Scope Lock 扩展）。
- `docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md`（v2.48，空基准新建旧表述收敛）、`docs/01_modules/WS/A-WS-M-T-02_工作区资源操作技术设计.md`（v1.1，BR-WS-STATE-009 / CHAIN active / REQ_MAP）、`A-WS-M-T-01_工作台技术设计.md`（v1.12，人工文件树安全底线）、`A-WS-M-D-01_工作台功能需求描述.md`（v1.11，REQ-WS-008/010 口径）。
- `docs/00_core/A-CORE-C-R-01_ADUS.md`：由 `governance:generate` 刷新。
- `.governance/active_scope.json`、本 Issue Trace：Scope 与运行证据。

scope:
  mode: explicit
  status: confirmed
  note: 2026-08-24 人类批准 Scope Lock 扩展，将 `scripts/test_governance.test.mjs` 基线断言调整纳入允许范围

phase_gate:
  status: passed
  evidence: Gate 1~4 均已落地；F1/F2 后端/F2 前端/治理收口定向测试与全量回归真实通过（见 validation）。

mapping_chain:
  status: complete
  evidence: BR-WS-STATE-009 有真实 @GOV owner（BR-WS-STATE-009-EFFECT-WS-WS-FS-009/010）与独立 `// covers:` 测试；CHAIN WS-OPERATION 已推进 active 并补 integration_test（deferred: G5）；REQ-WS-008/010 REQ_MAP 指向 BR-WS-STATE-009；治理审计显示规则 96 条、covered_rules=95、uncovered_rules=[]。

test_impact:
  changed_files: 使用选择器自动读取当前全部 tracked/untracked 变更（11 个已修改 + 1 个新增 Issue Trace）
  direct_modules: AG, AST, CBT, CORE, DE, DOC, ED, PCG, PRV, REF, SYS, WS
  affected_modules: ACCT, AG, AST, CBT, CORE, DE, DOC, ED, ENT, KB, PCG, PRV, REF, SYS, TPL, WS
  selection: frontend-full + TypeScript + rust-full + governance-audit

validation:

- `test:plan`：passed；plan 与上表 selection 一致，无 unmapped 可执行变更。
- `npm test`（vitest run，--pool=forks maxForks=4 资源受限参数）：passed，26 文件、589 项。
- `npx tsc --noEmit`：passed。
- `cargo test --manifest-path src-tauri/Cargo.toml`：passed；库单元 353、core_chain 5、sm_broadcast 4、main 0、doc-tests 0。范围外 LibreOffice 用例（docx_to_pdf_end_to_end_when_libreoffice_available）本次真实通过，未复现上游 signal=6。
- 定向：`ag_edit_base_tests` 17/17、`workspace::operation` 13/13（含 2 个新增 manual_*）、`WorkspaceUI.test.tsx` 33/33。
- `npm run test:governance`：passed，19 项；测试资产 94、用例 951、unmapped_executable_sources=0、unowned_tests=0、focused_or_skipped=0。
- `npm run governance:generate`：passed，ADUS 已刷新（@GOV 275、规则 96、链路 27、测试用例 951）。
- `npm run governance:audit`：passed；完整原日志 5,580 行、202,800 字节，顶层 status=passed，48 项检查全部 passed，governance_findings=0；semantic_review_candidates=275（涉 BR-WS 47：本次改动 operation.rs 8、lib.rs 4，新增 BR-WS-STATE-009 两处入列）；term_findings 0、new_domain_concept_candidates 165、BLOCK_UNANNOTATED 268（既有债务，见 unresolved_items）。
- `git diff --check`：passed。

语义审计留痕（补 C4 缺口）：BR-WS-STATE-003（5 处候选）、BR-WS-DATA-003（6）、BR-WS-OBS-002（4）、BR-WS-PERSIST-003（4）全部落在既有 WorkspaceOperation 前置裁决/不可变计划/回执/补偿恢复实现上，规则归属与边界语义匹配，无错配；新增 BR-WS-STATE-009 两处候选（record_manual_delete/record_manual_rename @GOV）与人工文件树安全底线语义一致，does_not=enter_decision_request 约束与"人工直写不入 WS-OPERATION 链"口径一致。

stop_conditions:

- 无。人类 2026-08-22 确认四项决策并授权实施，2026-08-24 批准基线调整；无未解除 Stop Condition。

recovery:

- 无；本 Scope 已闭合。真实界面场景按后续验证任务处理。

unresolved_items:

- 真实 Tauri/WKWebView 场景需人工验收（自动化通过不代替）：① 人工文件树删除→时间轴条目→恢复经新 DecisionRequest 裁决；② dirty 会话删除/重命名阻断提示；③ propose_edit 到不存在路径被拒（AG_TARGET_NOT_EXISTS）。
- BLOCK_UNANNOTATED 268 条为既有治理债务，本 Scope 已声明 out_of_scope，未扩大处理。
- semantic_review_candidates 275（涉 BR-WS 47）已复核，其余候选与本次改动无映射关系，按常规治理持续跟踪。

## 文件范围（Scope Lock 输入）

允许修改：

- `src-tauri/src/lib.rs`：F1 防御（ag_propose_edit）、F2 路由改造（delete/rename/create）、route_restore_node 兼容。
- `src-tauri/src/workspace/operation.rs`：提升 `has_dirty_affected_session`/`snapshot_resource`/`invalidate_affected_diffs` 可见性；`restore_intents_from_entry` 无 plan 分支；BR-WS-STATE-009 @GOV owner；内联测试。
- `src-tauri/src/workspace/fileops.rs`、`timeline.rs`、`mod.rs`：删除/重命名快照与投影辅助。
- `src-tauri/src/ag/tool_registry.rs`：propose_edit 兜底约束。
- `src/components/ws/FileTree.tsx`、`src/store/tabsStore.ts`、`edStore.ts`、`deStore.ts`、`agStore.ts`：dirty 提示、删除后同步。
- `src/services/agService.ts`、`wsService.ts`、`workspaceService.ts`：协调与失效联动。
- 测试：`src-tauri/src/workspace/operation.rs`（内联）、`src-tauri/tests/*`、`src/components/ws/WorkspaceUI.test.tsx`、`src/services/*.test.ts`。
- `docs/01_modules/WS/A-WS-M-T-01.md`、`A-WS-M-T-02.md`、`A-WS-M-D-01.md`、`docs/01_modules/AG/A-AG-M-T-01.md`：人类已批准修订。
- `docs/00_core/A-CORE-C-R-01_ADUS.md`：仅由 `governance:generate` 生成。
- `.governance/test-governance.json`、`.governance/issues/WS-RESOURCE-WRITE-GUARD-01.md`、`.governance/active_scope.json`：运行记录。

禁止修改：

- `AGENTS.md`、`ADU.md`、`ADP.md`、`CLAUDE.md`、`governance.config.json`、APC。
- `src-tauri/src/de/**`、`src-tauri/src/doc/**`：DE 内容 Diff 核心、DOC/BDocx 转换核心不重构。
- ENT/GW/ACCT/CBT/AST/KB/TPL/PRV 模块实现与不在允许列表中的文件。
- 若真实实现需要额外文件，按 ADP §3.3 停止并请求 Scope 扩展。

## Scope Lock

```yaml
task_type: CODE_CHANGE
allowed_files:
  - 见上述"允许修改"列表
forbidden_files:
  - 见上述"禁止修改"列表
confirmation: YES
scope_reason: 修复 A1（propose_edit 路径级防御）与 A3（人工文件树安全底线），统一资源写入口护栏并收敛文档 SSOT
```

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-08-22 | v1.0 | 基于 WS-OPERATION-IMPLEMENTATION-01 审计 A1/A3 结论建立 Issue Trace；人类确认四项决策（冻结文档修订授权、BR-WS-STATE-009 归属 WS-FS、WS-OPERATION 链 active、人工删除可恢复硬性验收），Scope Lock confirmation=YES。 |
| 2026-08-24 | v1.1 | 人类批准 Scope Lock 扩展：`scripts/test_governance.test.mjs` 基线断言 41→42（本 Scope WorkspaceUI 新增 BR-WS-STATE-009 dirty 阻断确认测试 +1，需 test:governance 通过）；批准记录见 Scope Lock 下。 |
| 2026-08-24 | v1.2 | Gate 1~4 全部落地并真实验证通过（前端全量 589、Rust 全量 353+5+4、test:governance 19/19、governance:audit 48/48 findings=0、git diff --check）；回填 §12 实施进度与 §13 Task Runtime Report，进入 READY；真实界面人工验收项登记 unresolved_items。 |

## Scope Lock 扩展批准（2026-08-24，人类确认）

- 批准变更文件：`scripts/test_governance.test.mjs`（仅 L139 WS 前端测试基线断言 41→42 及注释留痕）。
- 原因：`acceptance_criteria` 要求 test:governance 全通过，新增测试属本 Scope 必要验证资产，基线计数需同步。
- 该文件此前不在 allowed_files 内，经人类显式批准后纳入；其余 forbidden 边界不变。
