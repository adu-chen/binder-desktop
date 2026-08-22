# Issue Trace — WS-OPERATION-IMPLEMENTATION-01：WorkspaceOperation 1.0 全链迁移

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-21
上游基线：A-WS-M-T-02、A-WS-M-S-02、A-WS-X-L-01 PI-WS-06，以及 2026-08-21 人类确认的八项决策。

## 1. 用户可见症状

用户请求删除文档时，当前系统会把资源删除包装成 DiffCard，曾出现缺少投影载荷、卡片类型错误与接受后链路不可达。
创建、重命名和移动仍存在 Agent 直调低层命令或空基准新建链，与已冻结的前置裁决、直接 DiskState 执行与 CreateDocumentBundle 不一致。

## 2. 入口点

- `src-tauri/src/lib.rs` 的 Agent 工具循环、`ag_delete_proposal`、`route_apply_delete_diff`、`ensure_new_document_target` 与 command router。
- `src-tauri/src/ag/tool_registry.rs` 的逐回合 CapabilityGrant 和模型工具定义。
- `src-tauri/src/ag/tool_exec.rs::exec_structure_tool` 的现有直接重命名/建目录路径。
- `src-tauri/src/ag/task.rs::propose_decision/resolve_decision/begin_confirmed_continuation` 的 DecisionRequest 与 TaskRun 续片入口。
- `src-tauri/src/workspace/fileops.rs` 的低层创建、重命名、移动和删除原子操作。
- `src/components/ag/DecisionBar.tsx` 的真实用户裁决入口，以及 `src/components/de/DiffCard.tsx` 的旧资源卡分支。

## 3. 相关规则

- `BR-WS-STATE-003`：WorkspaceOperationPlan 前置裁决、确认时重新核验、幂等与直接 DiskState 执行。
- `BR-WS-DATA-003`：不可变计划、逐项回执、WorkspaceOperationEntry 与 WorkspaceTimeline 类型化历史。
- `BR-WS-OBS-002`：DecisionRequest 影响摘要、执行/补偿/失败真实投影。
- `BR-WS-PERSIST-003`：fail-fast、逆序补偿、partial_failed、Interrupted 与新计划恢复。
- `BR-SYS-STATE-008`：TaskRun 真实步骤、确认后续片与重启不自动续跑。
- `BR-CORE-STATE-001~003`：CreateDocumentBundle 初始内容 Diff 仍遵守普通内容三态分流。
- `X-FORBID-004`：所有路径限定在活动 Workspace。
- `X-FORBID-006`：内容 Diff 与 WorkspaceOperation 是两条互斥写路径。

## 4. 相关链路

`AG-TURN → AG-TOOL → WS-OPERATION → WS-TIMELINE`；CreateDocumentBundle 在资源创建成功后才进入 `DE-DIFF → ED-EDIT → WS-SAVE`。

## 5. 生产者块

- `BR-AG-STATE-002-QUERY-AG-AG-TOOL-012` 位于 `src-tauri/src/ag/tool_exec.rs`：现有工具执行入口，须从资源直写迁移为计划产生。
- `BR-SYS-STATE-008-EFFECT-AG-AG-TURN-002` 位于 `src-tauri/src/ag/task.rs`：TaskRun 重启 Interrupted 生产者。
- `BR-WS-DATA-001-EFFECT-WS-WS-FS-001/002/003/004/007` 位于 `src-tauri/src/workspace/fileops.rs`：低层资源原子执行器。
- `BR-WS-DATA-002-EFFECT-WS-WS-TIMELINE-001/002` 位于 `src-tauri/src/workspace/timeline.rs`：现有文档内容时间轴生产者，将扩展统一查询。

## 6. 消费者块

- `src-tauri/src/lib.rs` 的 command router、Agent 工具循环和 DecisionRequest 续片消费 plan_id/receipt。现有旧删除函数未按新链标注，本 Scope 必须删除或封闭并建立新 @GOV 入口。
- `src/services/agService.ts` / `src/store/agStore.ts` / `src/components/ag/DecisionBar.tsx` 消费 DecisionRequest 与执行回执。
- `src/services/wsService.ts` / `src/store/wsStore.ts` / `src/components/ws/TimelinePanel.tsx` 消费计划、执行终态和统一 WorkspaceTimeline。
- `src/store/deStore.ts` / `src/components/de/DiffCard.tsx` 只消费内容 Diff；须移除 delete_doc/delete_folder 资源分支，并承接 CreateDocumentBundle 初始内容 Diff 绑定。
- `src/components/ed/Editor.tsx` / ED 会话服务消费资源创建/补偿回执，在真实文件创建后打开标签，安全补偿成功后关闭。

## 7. 路径上的空规则块

未发现需保留的 `rules: []` 业务块。现有 `ag_delete_proposal`、`route_apply_delete_diff` 和 `exec_structure_tool`
不是合法的新链实现 owner；它们必须被移除、封闭或改造为引用 BR-WS-STATE-003/BR-WS-DATA-003/BR-WS-OBS-002/BR-WS-PERSIST-003 的新块，不得以无规则包装保留。

## 8. boundary 假设

- `lifecycle_assumption=plan_immutable_until_terminal`：DecisionRequest 裁决 plan_id，确认后不允许替换 items/baseline。
- `lifecycle_assumption=content_and_resource_lifecycles_separate`：WorkspaceOperation 不进入 SM-DE-01；CreateDocumentBundle 只绑定资源结果与普通初始内容 Diff。
- `document_snapshot=confirm_time_revalidation`：确认卡上的生成时基线不是执行许可，WS 必须在执行前重新核验真实磁盘、标签与 Diff 事实。
- `document_snapshot=create_bundle_triple_guard`：拒绝初始内容只在资源身份、DiskState 和 LogicalState 均未推进时可补偿删除。

## 9. 运行时假设

- `caller_context=taskrun_confirmed_workspace_operation`：`execute_workspace_operation` 只能由持有真实 resolved DecisionRequest 身份的 TaskRun 调用，不对模型开放。
- `runtime_guard=workspace_and_plan_identity`：workspace_id、plan_id、request_id、task_run_id 必须匹配。
- `runtime_guard=dirty_and_content_diff_impact`：删除/重命名/移动遇到脏标签必须阻断；有效内容 Diff 在确认前只列入影响，成功后才失效。
- `runtime_guard=idempotent_receipt`：重复确认/执行返回同一回执，不重复改变 DiskState。
- `runtime_guard=restart_interrupted_new_plan`：重启后未完成运行一律 Interrupted；继续必须生成新计划和新 DecisionRequest。

## 10. 疑似不变量破坏

- 旧 `propose_edit(delete_doc/delete_folder) → AG_DELETE PendingDiff → ws.apply_delete_diff` 把 WorkspaceOperation 状态塞入内容 Diff 机，直接违反 X-FORBID-006。
- `move_doc/create_folder` 当前在工具执行器中直接改变 DiskState，缺少 WorkspaceOperationPlan、DecisionRequest 和确认时重新核验。
- `base=empty` 新建在确认后才创建文件，与 CreateDocumentBundle 的“先创建真实目标、再投影内容 Diff”顺序相反。
- 旧删除时间轴只记录标签而非可恢复资源快照，无法支持 WorkspaceTimeline 类型化恢复。
- 若 DecisionBar 确认只启动新模型续调而不调用冻结 plan，会再次出现“确认后不执行”。

## 11. 验证计划

- 状态机：完整覆盖 SM-WS-04 Planning/WaitingConfirm/Revalidating/Executing/Compensating/Completed/Rejected/Expired/FailedCompensated/PartialFailed/Interrupted 转移与禁止转移。
- 后端单元/集成：计划不可变、workspace 限界、确认身份、幂等、baseline 过期、dirty 阻断、逐项回执、fail-fast 逆序补偿、partial_failed、重启 Interrupted、恢复新计划。
- CreateDocumentBundle：确认前零副作用；确认后先建盘、再开标签/投影；三重 guard 补偿删除；手动/后续 Agent/外部/路径变化导致旧卡失效且文件保留。
- 前端组件/状态：DecisionBar 真实影响摘要和执行状态；DiffCard 不再接受资源 patch kind；标签/文件树/引用/内容 Diff 在成功后按真实资源结果同步。
- 时间轴：文档内容节点继续走 SM-WS-03；WorkspaceOperationEntry 恢复必须生成新计划和 DecisionRequest，冲突不覆盖。
- 日志序列：`plan_created → decision_proposed → decision_resolved → revalidation → item_receipt* → compensation* → terminal → timeline_entry`；日志只记 ID/状态/数量/错误码，不记正文。
- 治理：为四条新规则补真实 @GOV owner 与 `// covers:` 断言；运行 `test:plan`、同输入 `test:related`、`governance:generate`、`governance:audit`、`git diff --check`；完整读取未裁剪审计原始日志。
- 系统验证：执行 A-VAL-X-V-04 V-C1~C3/V-R1~R2；自动化通过不代替真实 Tauri/WKWebView 观察。

## 分阶段实施门禁

1. **Gate A · 领域与持久化**：WorkspaceOperationPlan/Entry/CreateDocumentBundle、schema 迁移、计划/回执/时间轴存取与状态转移测试。
2. **Gate B · 裁决与执行**：plan/get/cancel/execute、DecisionRequest 身份、重新核验、dirty 阻断、幂等与逐项回执。
3. **Gate C · 补偿与恢复**：fail-fast 逆序补偿、partial_failed、Interrupted、WorkspaceTimeline 类型化恢复。
4. **Gate D · CreateDocumentBundle**：初始内容前生成、确认后先建盘、打开标签/投影内容 Diff、安全拒绝与基线推进。
5. **Gate E · 迁移与跨模块收口**：移除资源 Diff/直调低层命令/空基准新建，同步标签、引用和内容 Diff 失效，完成 V-C/V-R 系统验证。

每个 Gate 必须先运行定向测试并记录未解决项；上一 Gate 的数据/状态契约未稳定时不进入下一 Gate。

## 12. 实施进度

- Gate A 已落地：v14 持久化、WorkspaceOperationPlan/Entry/Receipt/CreateDocumentBundle、SM-WS-04 与重启 Interrupted；定向 Rust 测试通过。
- Gate B 已落地：Agent 只可调用 `plan_workspace_operation`；DecisionRequest 确认时重新核验并执行，拒绝零副作用，重复确认返回同一回执；DecisionBar 显示真实操作清单和执行中状态。
- Gate C 已落地：批量 fail-fast、逆序补偿、资源快照、统一时间轴投影，以及“资源恢复生成新计划和新 DecisionRequest”链路。
- Gate D 已落地：确认后创建真实文件、打开标签、从冻结载荷投影普通内容 Diff；拒绝使用资源身份、DiskState、LogicalState 三重 guard，基线推进时只使 Diff 失效并保留文件；投影失败执行安全补偿。
- Gate E 已落地：资源 patch kind、旧资源 Diff 接受入口、模型直调移动/建目录/新建入口已移除；低层 `ws.create_*`/`ws.delete_*` 仅保留给人工文件树与 WorkspaceOperation 原子执行器，不再对模型注册。
- Gate F 已完成代码修复：确认前统一刷新全部打开会话的 LogicalState 快照并按文档隔离防抖；WorkspaceOperation 逐项进度与补偿进度持久化，资源终态、回执、时间轴、内容 Diff 失效和 CreateDocumentBundle 绑定统一事务提交；Interrupted 恢复按真实磁盘效果生成新计划；前端同步失效 Diff、标签和引用，并投影确认影响与部分失败；CreateDocumentBundle 补齐显式接受基线 guard、Markdown 格式转换、打开/投影失败补偿和 BDocx 身份恢复；无当前文档时仍可查看工作区资源时间轴。人类已验证 clean 单文档删除确认与 TaskRun 自动继续主链；冻结的其余真实 Tauri/WKWebView 场景仍待观察。

当前定向证据：`workspace::operation` 11 项、前端 AG/WS 定向 85 项均真实通过。前端全量 586/586、TypeScript、治理自测 19/19 与治理审计 48/48 均通过；Rust 全量 349/350，唯一失败为范围外 DOC/LibreOffice Worker signal=6，定向复跑仍失败。

## 13. Task Runtime Report

task_type: CODE_CHANGE
governance_phase: IMPLEMENTATION
final_state: READY
issue_trace: `.governance/issues/WS-OPERATION-IMPLEMENTATION-01.md`

changed_files:

- `src-tauri/src/types/mod.rs`、`db/mod.rs`、`workspace/operation.rs`、`workspace/mod.rs`：WorkspaceOperation 领域、v15 持久化、逐项执行日志、状态机、事务化终态、补偿/恢复与 CreateDocumentBundle。
- `src-tauri/src/ag/{bdocx_capabilities,context,prompts,tool_exec,tool_registry}.rs`、`src-tauri/src/lib.rs`：模型权限收口、计划工具、DecisionRequest 执行和新建投影路由。
- `src/services/{agService,deService,edService,wsService}.ts`、`src/store/{agStore,deStore,edStore,tabsStore}.ts`：确认前 LogicalState 刷新、标签/文件树/引用同步、初始内容 Diff、dirty 关闭保护与资源恢复。
- `src/components/ag/DecisionBar.tsx`、`src/components/de/DiffCard.tsx`、`src/components/ed/Editor.tsx`、`src/components/ws/{FileTree,TimelinePanel}.tsx`：确认影响、失败回执、编辑器快照/预览补偿、内容 Diff 隔离与统一时间轴投影。
- `src/services/agRuntime.test.ts`、`src/components/ws/WorkspaceUI.test.tsx`：dirty 刷新、资源确认回执、引用/Diff 同步、部分失败投影和 CreateDocumentBundle 基线测试。
- `docs/00_core/A-CORE-C-R-01_ADUS.md`：由 `governance:generate` 刷新。
- `.governance/active_scope.json`、本 Issue Trace：Scope 与运行证据。

scope:
  mode: explicit
  status: confirmed

phase_gate:
  status: passed
  evidence: Gate A~E 均已落地并通过对应定向测试。

mapping_chain:
  status: complete
  evidence: BR-WS-STATE-003、BR-WS-DATA-003、BR-WS-OBS-002、BR-WS-PERSIST-003 均有真实 @GOV owner 与独立 `// covers:`，治理审计显示 94/94 规则已覆盖。

test_impact:
  changed_files: 使用选择器自动读取当前全部 tracked/untracked 变更
  direct_modules: AG, AST, CBT, CORE, DE, DOC, ED, KB, PCG, PRV, REF, SYS, TPL, WS
  affected_modules: ACCT, AG, AST, CBT, CORE, DE, DOC, ED, ENT, KB, PCG, PRV, REF, SYS, TPL, WS
  selection: frontend-full + TypeScript + rust-full + governance-audit

validation:

- `npm test`：passed，26 文件、586 项。
- `npx tsc --noEmit`：passed。
- `cargo test --manifest-path src-tauri/Cargo.toml`：failed，349/350；唯一失败为禁止修改范围内的 `doc::libreoffice_runtime::tests::docx_to_pdf_end_to_end_when_libreoffice_available`，LO Worker signal=6；定向复跑同样失败。
- WorkspaceOperation 定向 Rust：passed，共 11 项；前端 AG/WS 定向：passed，共 85 项。
- `npm run test:governance`：passed，19 项；测试资产 94、用例 945、无未归属/skip/only。
- `npm run governance:generate`：passed，ADUS 已刷新。
- `npm run governance:audit`：passed；完整原日志 5,559 行、201,681 字节，顶层 status=passed，48 项检查无失败，产品能力 status=passed，无 finding。
- `git diff --check`：passed。
- A-VAL-X-V-04 V-C1~C3/V-R1~R2：未完整执行；人类已验证 clean 单文档删除主链，并于 2026-08-22 明确要求闭合本轮修复。其余场景不登记为 passed，保留为后续验证项。

stop_conditions:

- 无。原 SC-04 已由人类在审阅自动化证据和已验证删除主链后明确要求闭合本轮修复而解除；未执行场景不伪记通过。

recovery:

- 无；本 Scope 已闭合。剩余真实界面场景按后续验证任务处理。

unresolved_items:

- 范围外 DOC/LibreOffice Worker signal=6 已按此前“接受范围外失败，继续”保留，不扩展 Scope 修改。
- 真实界面尚需验证：当前活动 Composer 对重命名/移动引用的即时重绘、dirty 标签阻断、批量部分失败回执、CreateDocumentBundle 旧卡失效/安全补偿、工作区时间轴恢复新裁决。

## 文件范围（Scope Lock 输入）

允许修改：

- `src-tauri/src/types/mod.rs`：WorkspaceOperation/Timeline/CreateDocumentBundle 共享 DTO。
- `src-tauri/src/db/mod.rs`：workspace.db 版本迁移与新表/索引。
- `src-tauri/src/workspace/operation.rs`（新）、`mod.rs`、`fileops.rs`、`timeline.rs`、`lifecycle.rs`：SM-WS-04、低层原子执行、补偿、恢复与重启清算。
- `src-tauri/src/ag/task.rs`、`tool_registry.rs`、`tool_exec.rs`、`context.rs`、`prompts.rs`、`bdocx_capabilities.rs`：Intent/Grant、计划工具、DecisionRequest/TaskRun 绑定与旧路径封闭。
- `src-tauri/src/ipc/mod.rs`、`src-tauri/src/lib.rs`：command/event 契约、路由、Agent 工具循环与 CreateDocumentBundle 交接。
- `src-tauri/tests/core_chain.rs`、`src-tauri/tests/sm_broadcast.rs`、`src-tauri/tests/workspace_operation.rs`（新）：跨模块、状态与持久化验证。
- `src/services/wsService.ts`、`workspaceService.ts`、`agService.ts`、`deService.ts`、`edService.ts`、`referencePayload.ts`：前端契约与事件协调。
- `src/store/wsStore.ts`、`workspaceShell.ts`、`agStore.ts`、`deStore.ts`、`edStore.ts`、`tabsStore.ts`：运行投影、标签/文件树/Diff 同步。
- `src/components/ag/DecisionBar.tsx`、`MessageList.tsx`、`src/components/de/DiffCard.tsx`、`src/components/ed/Editor.tsx`、`src/components/ws/FileTree.tsx`、`FileTreeNode.tsx`、`TimelinePanel.tsx`、`src/shell/Workbench.tsx`：用户裁决、真实状态展示和跨模块回执。
- `src/services/agRuntime.test.ts`、`deRuntime.test.ts`、`workspaceRuntime.test.ts`、`src/components/ed/EditorCore.test.tsx`、`src/components/ws/WorkspaceUI.test.tsx`：前端状态/服务/组件验证。
- `.governance/test-governance.json`、`.governance/product-capability-registry.json`：仅在新增测试资产或产品证据需登记时修改。
- `docs/00_core/A-CORE-C-R-01_ADUS.md`：仅由 `governance:generate` 生成。
- `.governance/issues/WS-OPERATION-IMPLEMENTATION-01.md`、`.governance/active_scope.json`：本 Scope 运行记录。

禁止修改：

- `AGENTS.md`、`ADU.md`、`ADP.md`、`CLAUDE.md`、`governance.config.json`、APC。
- `docs/01_modules/**`、`docs/40_validation/**`：已冻结设计与验证卡，不得为迁就实现反向改规则。
- `src-tauri/src/de/**`、`src-tauri/src/doc/**`：DE 不承载资源操作，DOC/BDocx 转换核心不在本 Scope。
- ENT/GW/ACCT/CBT/AST/KB/TPL/PRV 模块实现与不在允许列表中的文件。
- 若真实实现需要额外文件，必须按 ADP §3.3 停止并请求 Scope 扩展，不得以目录级改写绕过。

## Scope Lock

```yaml
task_type: CODE_CHANGE
allowed_files:
  - 见上述“允许修改”列表
forbidden_files:
  - 见上述“禁止修改”列表
confirmation: YES
scope_reason: 实现已确认的 WorkspaceOperation/CreateDocumentBundle/WorkspaceTimeline 基线，并移除与其冲突的资源 Diff、直调低层命令与空基准新建链
```

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-08-22 | v1.5 | 人类明确要求闭合本轮修复；解除 SC-04 并进入 READY，未执行的真实界面场景继续作为后续验证项保留，不登记为通过。 |
| 2026-08-22 | v1.4 | 完成 Gate F：修复 dirty 快照竞态、逐项持久日志与事务终态、Interrupted/补偿恢复、Diff/引用同步、CreateDocumentBundle 基线与格式闭环、确认/失败/工作区时间轴可见性；记录最终自动化证据与剩余真实界面门禁。 |
| 2026-08-21 | v1.3 | Gate A~E 实现完成；记录完整自动化/治理证据、范围外 LibreOffice 失败及真实 UI 验证 SC-04。 |
| 2026-08-21 | v1.2 | 记录 Gate A–D 落地与 Gate E 收口状态，准备进入完整影响验证。 |
| 2026-08-21 | v1.1 | 人类确认 Scope，进入分阶段实现。 |
| 2026-08-21 | v1.0 | 根据冻结的八项决策建立全链 Issue Trace；将实施分为五个可验证 Gate，锁定旧资源 Diff/直写/空基准新建迁移和未裁剪审计日志要求。 |
