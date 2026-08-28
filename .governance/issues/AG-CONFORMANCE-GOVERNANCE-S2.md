# Issue Trace — AG-CONFORMANCE-GOVERNANCE-S2：Agent 生产入口与规则注册精确映射审计

文档状态：A
任务类型：GOVERNANCE_CHANGE
治理阶段：VALIDATION
任务状态：READY
创建时间：2026-08-27
上游基线：AG-CONFORMANCE-GOVERNANCE-S1；ADUS 的 AG-TURN、AG-TOOL、WS-OPERATION 链路视图；人类确认只按规则链映射开展审计。

## 1. 用户可见症状

第一阶段已经消除需求与设计中的并行语义，但 ADUS 当前只能证明生产块已经结构注册，不能证明实际生产行为符合最新规则。需要验证 AgentTurn 澄清、失败重试、TaskRun 创建与等待裁决、CapabilityGrant 最小允许集及 CreateDocumentBundle 路径是否真实落在已注册生产链上。

## 2. 入口点

- AG-TURN：ADUS 链路注册表与链路视图登记的 AgentTurn 生产块。
- AG-TOOL：ADUS 链路注册表与链路视图登记的 ToolCall、CapabilityGrant 和工具执行生产块。
- WS-OPERATION：只纳入被 AG-TURN/AG-TOOL 注册块直接交叉引用的 WorkspaceOperationPlan、DecisionRequest 与 CreateDocumentBundle 承接块。

入口函数名与文件位置不通过关键词推断；以 ADUS 已生成代码块索引为唯一定位依据，读取完整注册块后记录。

## 3. 相关规则

- AG-TURN：BR-AG-STATE-001、BR-AG-DATA-001/002/003/004/005、BR-AG-VERIFY-001、BR-AG-PERSIST-001、BR-AG-STATE-004，以及注册交叉规则 BR-SYS-STATE-007/008。
- AG-TOOL：BR-AG-STATE-002、BR-SYS-DATA-006，以及 ADUS 已登记的 ED/DE/WS 交叉规则。
- WS-OPERATION 交叉承接：BR-WS-STATE-003、BR-WS-DATA-003、BR-WS-PERSIST-003、BR-WS-OBS-002。

最终相关规则集合以完整规则原文与代码块引用的交集复核为准，不因文件同属 AG 模块而自动扩大。

## 4. 相关链路

主审计链：AG-TURN → AG-TOOL；资源副作用交叉链：AG-TURN / AG-TOOL → WS-OPERATION。

相邻影响检查：AG-ASSIST、AG-MODEL-CATALOG。两者不纳入本阶段行为符合性整改范围。

## 5. 生产者块

以下清单只登记与六项唯一语义直接相关的生产者。所有定位均来自 ADUS 代码块索引；“未注册”项仅由已注册块的直接调用关系发现，不由仓库关键词检索发现。

| 语义 | ADUS 注册块 | 承载实现与实际输出 | 注册判断 |
|---|---|---|---|
| AgentTurn 主入口 | `BR-AG-STATE-001-API-AG-AG-TURN-018` | `src-tauri/src/ag/turn_runtime.rs:21` `ag_start_turn`；建立历史草稿、TaskRun、IntentResult、门禁、模型与工具循环 | 已注册，但 rules 仅含 BR-AG-STATE-001，实际 TaskRun 副作用未登记 BR-SYS-STATE-008 |
| AgentTurn 状态机 | `BR-AG-STATE-001-RB-AG-AG-TURN-001`、`BR-SYS-STATE-007-RB-AG-AG-TURN-001` | `src-tauri/src/ag/turn.rs:122` `transition`；输出 TurnState | 已注册；实现缺 `ClarificationProvided` 事件和恢复转移 |
| IntentResult | `BR-AG-DATA-004-RB-AG-AG-TURN-015` | `src-tauri/src/ag/context.rs:367` `resolve_intent_result`；输出 IntentResultV1 | 已注册；授权语义仍为旧“乐观/全量工具”模型 |
| CapabilityGrant | 无对应 ADUS 代码块 | `src-tauri/src/ag/tool_registry.rs:39` `resolve_capability_grant`；输出 CapabilityGrantV1 | 由已注册主入口的直接下游发现；关键裁决 owner 未注册 |
| ToolCall 状态机 | `BR-AG-STATE-002-RB-AG-AG-TOOL-001` | `src-tauri/src/ag/tool.rs:50` `transition`；输出 ToolCallState | 已注册；本阶段未发现与六项语义直接冲突 |
| 工具循环 | `BR-AG-STATE-002-RB-AG-AG-TOOL-018`、`BR-SYS-STATE-008-RB-AG-AG-TOOL-018` | `src-tauri/src/ag/turn_tool_loop.rs:24`；执行授权校验、工具调用与 TaskStep 推进 | 已注册；其授权 SSOT 依赖未注册 CapabilityGrant owner |
| TaskRun 状态机 | `BR-SYS-STATE-008-RB-AG-AG-TASK-001`、`BR-AG-STATE-001-RB-AG-AG-TASK-001` | `src-tauri/src/ag/task.rs:51` `transition`；输出 TaskRunStatus | 已注册，但不是持久态写入的唯一入口，且允许 `WaitingConfirm + Finish -> Completed` |
| TaskRun 创建/复用 | 无对应 ADUS 代码块 | `src-tauri/src/ag/task.rs:109` `begin_turn`；直接写 `task_runs` | 已注册主入口直接调用；关键持久态 owner 未注册 |
| DecisionRequest 创建/裁决 | 外层注册块 `BR-SYS-STATE-008-EFFECT-AG-AG-TURN-026`、`BR-AG-STATE-002-API-AG-WS-OPERATION-005` 等 | `src-tauri/src/ag/task_runtime.rs:36/64` 路由；直接下游 `task.rs:500` `propose_decision`、`:560` `resolve_decision` 写 TaskRun 状态 | 外层 API 已注册，真实状态 setter 未单独注册 |
| CreateDocument 资源计划 | `BR-WS-STATE-003-API-WS-WS-OPERATION-002` 等 | `src-tauri/src/workspace/routes.rs:1157`；冻结初始内容、创建 WorkspaceOperationPlan / CreateDocumentBundle / DecisionRequest | 已注册且行为符合 |
| WorkspaceOperation 执行 | `BR-WS-STATE-003-EFFECT-WS-WS-OPERATION-003` 等 | `src-tauri/src/workspace/operation.rs:2537`；校验已裁决请求、基线、执行与补偿 | 已注册且行为符合 |
| CreateDocumentBundle 投影 | `BR-WS-STATE-003-API-WS-WS-OPERATION-004` 等 | `src-tauri/src/workspace/routes.rs:1413`；真实资源建立后投影初始内容 Diff | 已注册且行为符合 |
| 内容提案入口 | `BR-AG-STATE-002-EFFECT-AG-AG-TOOL-017`、`BR-AG-DATA-003-EFFECT-AG-AG-TOOL-017` | `src-tauri/src/ag/edit_runtime.rs:18` `ag_propose_edit`；输出 PendingDiff | 已注册；实际先调用未注册的资源存在性 guard |
| 不存在目标 guard | 无对应 ADUS 代码块 | `src-tauri/src/ag/edit_support.rs:76` `guard_propose_edit_target`；不存在即拒绝 | 当前新建文档唯一入口的关键 guard 未注册 |
| 目标集 guard | `BR-AG-DATA-003-GUARD-AG-AG-TURN-001` | `src-tauri/src/ag/context.rs:1068` `guard_target_set`；输出 TargetDecision | 已注册，但 boundary 与实现互相矛盾 |

## 6. 消费者块

| 生产输出 | 直接消费者 | 证据与结论 |
|---|---|---|
| IntentResult | `prepare_turn_prompt`、`resolve_capability_grant`、复杂目标判定 | 前者由 ADUS 登记的 `turn_prompt.rs` / `prompt_runtime.rs` 承接；CapabilityGrant owner 未注册 |
| CapabilityGrant | `turn_tool_loop`、`validate_requested_tool_for_grant`、工具 schema 过滤 | 工具循环已注册；最终授权校验函数位于 `tool_registry.rs:145`，未在 ADUS 呈现 |
| DecisionRequest | `route_resolve_decision` → `execute_operation_plan` | 交叉链 AG-TURN / AG-TOOL / WS-OPERATION 已注册，且确认身份绑定清楚 |
| CreateDocumentBundle | `route_resolve_decision` 返回值 → `route_project_create_document_bundle` | 仅在 WorkspaceOperation 完成并存在真实 DiskState 后投影 Diff；符合资源先于内容投影 |
| PendingDiff | `src/services/agService.ts:1027` 的编辑器预览承接 | ADUS 已登记 AG-TURN / DE-DIFF / ED-EDIT 交叉块；不把 DiffCard 解释为 TaskRun WaitingConfirm 来源 |
| AgentTurn ClarificationRequired | 活动回合注册与取消控制 | 直接下游只有取消/abort，没有澄清输入消费者；同一 AgentTurn 恢复链断裂 |

## 7. 路径上的空规则块

ADUS 的相关链路视图未显示 `rules: []` 生产块。但这不等于关键决策全部注册：本次通过已注册块的直接调用关系确认至少三类关键 owner 不在 ADUS 中：

- CapabilityGrant 生成与逐工具授权校验：`tool_registry.rs`。
- TaskRun 创建、收束、DecisionRequest 创建/裁决的真实持久态 setter：`task.rs`。
- `propose_edit` 不得创建资源的实际入口 guard：`edit_support.rs`。

因此当前审计能发现“文件级未映射”，但不能证明“注册块内部的关键决策符号均有主责块”。这是 ADUS 呈现/审计粒度缺口，不是空规则块本身。

## 8. boundary 假设

已确认两处 boundary 事实不真实：

1. `BR-AG-DATA-003-GUARD-AG-AG-TURN-001` 声明 `rejectUndeclaredExisting`，实现却对未声明既有文档返回 `AcceptCrossDocument`，并对不存在目标返回 `Accept`。注册边界、注释和实现三者不一致。
2. `BR-AG-STATE-001-API-AG-AG-TURN-018` 声明输出持久化 TaskRun 转移，但 `rules` 未登记 BR-SYS-STATE-008，且实现对所有 AgentTurn 无条件创建/沿用 TaskRun。该块同时存在规则遗漏与行为偏差。

CreateDocument 主路径的 `does_not=createWorkspaceResourceOrWriteLogicalStateOrDiskState` 与实际 `ag_propose_edit` 行为一致；一致性的真正保障来自未注册的 `guard_propose_edit_target`，故当前 boundary 结论依赖链外隐含 guard。

## 9. 运行时假设

运行时事实如下：

- 澄清：`turn_runtime.rs:378-417` 发出 ClarificationRequired 后把同一 async command 永久挂起，只能由 abort 结束；abort 后 TaskRun 和 AgentTurn 被记为 Cancelled。`turn_control.rs:51-52` 明确把补充输入描述为“取消旧回合后发起新回合”。不存在 `ClarificationProvided` 输入通道。
- 失败重试：`turn.rs` 的 TurnError 是冻结终态，`Retry` 不可转移；新请求须携带新 turn_id。该项符合唯一语义。
- TaskRun：`turn_runtime.rs:116-250` 在 IntentResult 与复杂度判定前无条件调用 `begin_turn`；复杂度判断只控制 TaskPlan 创建（`:345-376`）。简单回答/分析/局部编辑同样产生 TaskRun。
- WaitingConfirm：真实持久态仅由 `propose_decision` 写入；普通 pending Diff 和提案步骤不再驱动该状态，符合唯一语义。但注册的 `task::transition` 仍允许 `WaitingConfirm + Finish -> Completed`，且真实 setter 绕开该转移函数直接写库。
- CapabilityGrant：IntentResult 与 CapabilityGrant 仍使用“非 Answer/Clarify 基本全量授权”策略；Analyze/Search/Inspect 可获得内容/资源提案候选，CreateDocument 的 `allowed_tools` 中仍包含 `propose_edit`，仅在调用时由参数约束二次拒绝。它不满足“按目标给出最小允许集”。
- CreateDocument：实际调用时禁止 `propose_edit` 创建不存在资源；资源经 WorkspaceOperationPlan + DecisionRequest 确认后建立，再投影 CreateDocumentBundle 初始内容 Diff。主运行时路径符合唯一语义。

## 10. 疑似不变量破坏

| ID | 等级 | 审计结论 | 分类 | 直接证据 |
|---|---|---|---|---|
| S2-F01 | Critical | 澄清无法在同一 AgentTurn 恢复；当前以取消旧回合、新回合补充实现 | DESIGN_CODE_CONFLICT | `turn.rs:83-100,135-163`；`turn_runtime.rs:378-417`；`turn_control.rs:41-62` |
| S2-F02 | Pass | TurnError 为终态；重试创建新 AgentTurn | CONFORMING | `turn.rs:65-120,135-163` 及终态测试 |
| S2-F03 | Critical | 简单 AgentTurn 也无条件创建 TaskRun | DESIGN_CODE_CONFLICT | `turn_runtime.rs:116-250,339-376`；`task.rs:109-145` |
| S2-F04 | High | WaitingConfirm 的真实写入来源符合 DecisionRequest，但 SM-AG-04 注册实现可非法 Finish，且不是持久态 SSOT | STATE_MACHINE_DRIFT / DECORATIVE_MAPPING | `task.rs:35-85,500-605`；`task_runtime.rs:36-204` |
| S2-F05 | Critical | IntentResult / CapabilityGrant 不是最小允许集，CreateDocument 仍暴露 propose_edit 候选 | DESIGN_CODE_CONFLICT | `context.rs:379-588`；`tool_registry.rs:39-143` |
| S2-F06 | High | CreateDocument 主路径符合；但关键不存在目标 guard 未注册，已注册目标 guard 的 boundary 为假 | CONFORMING_PATH / BOUNDARY_FALSE / RULE_UNMAPPED | `edit_runtime.rs:18-95`；`edit_support.rs:76-89`；`context.rs:1058-1098`；WS 交叉块 |
| S2-F07 | High | CapabilityGrant、TaskRun 持久态 setter、资源存在性 guard 等关键决策 owner 未在 ADUS 注册呈现 | REGISTRATION_GAP | 上述已注册块的直接调用链 |
| S2-F08 | Medium | AG-TURN、AG-TOOL 集成测试在 ADUS 标为 `deferred: phase-5`；部分现有单测固化旧语义 | VALIDATION_GAP | `task.rs:1110-1168`；`tool_registry.rs:648-772`；ADUS 测试能力索引 |

六项唯一语义总判定：2 项符合（失败重试、CreateDocument 主路径），1 项“运行时符合但注册状态机漂移”（WaitingConfirm），3 项不符合（同回合澄清、简单回合 TaskRun 边界、最小 CapabilityGrant）。

## 11. 验证计划

1. 从 ADUS 读取完整链路注册表、规则注册表、术语注册表与 AG-TURN/AG-TOOL/WS-OPERATION 链路视图。
2. 由规则注册表的 source_doc 定位并完整读取相关 RULE 与 STATE_MACHINE 正文。
3. 由链路视图的文件与行号定位并完整读取每个 @GOV 块及其直接承载实现；校验索引行号与块 ID 一致。
4. 构建“唯一语义 → 规则 → 状态机 → 生产块 → 直接测试证据”映射表。
5. 只把可复现证据写入本记录；取舍项或无法唯一裁决项触发 Stop Condition，等待人类确认。
6. 本阶段只改治理记录，交付前运行差异检查，并确认没有生产代码、设计、测试或 ADUS 变化。

## 后续候选修改范围（未授权实施）

按风险隔离建议拆为四个后续 Scope，不合并提交：

1. **S3-A AgentTurn 澄清续接**：`turn.rs`、`turn_runtime.rs`、`turn_control.rs`、前端同一回合输入承接及对应测试；必须先由人类选择续接形态。
2. **S3-B TaskRun 按需创建与状态 SSOT**：`turn_runtime.rs`、`task.rs`、`task_runtime.rs`、类型/迁移/前端展示和测试；先确认简单 AgentTurn 是否允许完全没有 TaskRun 身份。
3. **S3-C IntentResult / CapabilityGrant 最小授权**：`context.rs`、`tool_registry.rs`、`turn_prompt.rs` / `turn_tool_loop.rs` 的直接契约与测试；按设计矩阵实现，不沿用 profile 全量档。
4. **S3-D 注册真实性与审计粒度**：为真实决策 owner 建立唯一 @GOV 主责块，修复虚假 boundary，补 ADUS 的状态机/关键符号呈现与审计；跨文档未声明目标的行为变更须单独裁决，避免影响显式引用、多文档和延续语义。

不建议先“补注释让 ADUS 看起来完整”：S2-F01/F03/F05 是真实行为冲突，注册必须在实现收敛后指向真实 owner。

## 人类裁决

1. **HD-S2-02：同一 AgentTurn 澄清续接形态（CONFIRMED）**。采用第二种：
   - 保持原 command 挂起，新增显式 clarification 输入通道；
   - command 返回非终态，后续 command 以相同 turn_id 受控重入并触发 `ClarificationProvided`。
   command 返回非终态，后续 command 以相同 turn_id 受控重入并触发 `ClarificationProvided`；不得创建新 AgentTurn。
2. **HD-S2-03：简单 AgentTurn 的 TaskRun 边界（CONFIRMED）**。简单 AgentTurn 完全不创建 TaskRun；后续改为需要任务级产物时按需创建，并允许调整当前必填关联。
3. **HD-S2-04：未声明既有跨文档目标（CONFIRMED）**。先注册“显式目标集追加规则”，再硬拒集合之外的既有文档；显式引用、多文档和延续语义不得被误伤。

## 测试证据说明

- `task.rs:1110-1143` 的 SM-AG-04 测试明确断言 `WaitingConfirm + Finish -> Completed`；该绿测是规则冲突证据，不是符合性证据。
- `task.rs:1148-1168` 明确断言每次 `begin_turn` 新建/沿用 TaskRun；未覆盖简单 AgentTurn 不建 TaskRun。
- `tool_registry.rs:648-722` 名为 minimal，但只读档仍包含 `propose_decision`；`:724-772` 明确断言 CreateDocument 的 allowed_tools 含 `propose_edit`，再依靠调用时拒绝。
- ADUS 对 AG-TURN / AG-TOOL 的集成测试状态为 deferred，故当前不能声称生产链端到端符合。

## Task Runtime Report

- scope：仅 `.governance/active_scope.json` 与本 Issue Trace。
- changed_files：`.governance/active_scope.json`、`.governance/issues/AG-CONFORMANCE-GOVERNANCE-S2.md`。
- validation：
  - `git diff --check`：passed。
  - `npm run governance:audit`：passed；295 个 @GOV 块闭合、0 个 unmapped executable source，但 `BLOCK_UNANNOTATED` 同时报告 268 个未标注导出符号且当前不阻断。这证明结构 audit 通过不能替代本轮语义符合性审计。
  - `npm run test:governance`：passed；19/19 治理选择器测试通过，AG 登记为 critical，30 个测试资产、269 个测试用例。
  - `npm run test:plan -- --files ...`：passed；选择 CORE 相关前端测试与治理 audit。
  - `npm run test:related -- --files ...`：passed；2 个测试文件、75 个测试通过，并再次通过治理 audit。
- requirement_validation：规则链映射审计已完成；六项语义逐项结论见 S2-F01–F08。
- unresolved_items：HD-S2-02/03/04；任何生产修改必须另开 Scope。

## 读取完整性协议

- 定位源：只使用 ADUS 已注册的 chain_id、rule_id、source_doc、codes、文件与行号。
- RULE：从 `<!-- RULE` 起读取到标注结束，并连同紧随其后的完整规则正文读取；不截取单句。
- STATE_MACHINE：从 `<!-- STATE_MACHINE` 起读取完整标注块与完整状态机正文。
- 生产块：从 `/** @GOV` 起读取完整标注，核对 `codes` 与 ADUS 一致，并读取其直接承载的完整声明/函数/实现块。
- 准确性：记录源文件、起始行、结束行与内容摘要；若无法可靠确定语法块边界，标记 `evidence_insufficient`，不得自行补全。
- 唯一性：同一 `codes` 只登记一次；跨链复用明确列出全部 chain，不复制成两个“实现事实”。

## 审计结果

审计事实与治理验证均已完成，阶段状态 READY；等待人类对后续实现取舍的确认。本阶段没有修改生产代码、设计、测试或 ADUS。

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-08-27 | v1.0 | 人类确认开启第二阶段；建立只读规则链映射审计 Scope 与完整读取协议。 |
| 2026-08-27 | v1.1 | 完成六项唯一语义的规则→状态机→生产块→测试证据映射；记录三项行为冲突、状态机漂移、虚假 boundary 与关键 owner 注册缺口。 |
| 2026-08-27 | v1.2 | 完成 changed_files 影响计划、相关测试和治理验证；记录结构 audit 通过与语义符合性之间的边界，阶段进入 READY。 |
| 2026-08-27 | v1.3 | 人类确认 HD-S2-02/03/04：同 turn_id 受控重入、简单回合不建 TaskRun、先注册显式目标集追加规则再拒绝集合外目标。 |
