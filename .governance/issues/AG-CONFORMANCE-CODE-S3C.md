# Issue Trace — AG-CONFORMANCE-CODE-S3C：IntentResult 与 CapabilityGrant 最小授权闭环

文档状态：A
任务类型：CODE_CHANGE
治理阶段：VALIDATION
任务状态：READY
创建时间：2026-08-28
上游基线：REQ-AG-007/015；BR-SYS-STATE-007；BR-AG-DATA-004；BR-AG-STATE-002；WP-AG10-00/02/05；AG-CONFORMANCE-GOVERNANCE-S2 的 S2-F05/S2-F07；人类确认 HD-S3C-01。

## 1. 用户可见症状

当前 AgentTurn 的 IntentResult 与 CapabilityGrant 仍沿用旧的宽授权实现：非 Answer/Clarify 回合基本取得全量读取、内容提案、资源计划和 DecisionRequest 工具；CreateDocument 仍在候选集中暴露 `propose_edit`，只在执行时二次拒绝；缺失 RuntimeProfile 会被兼容默认值替换。该生产行为不符合已冻结的最小授权规则，也使 ADUS 无法清晰呈现 CapabilityGrant 的真实 owner。

## 2. 入口点

- AG-TURN：`ag_start_turn` → `resolve_intent_result` → `prepare_turn_prompt`。
- AG-TOOL：`resolve_capability_grant` → 工具 manifest 过滤 → `validate_requested_tool_for_grant` → `turn_tool_loop`。
- 相邻保持：复杂 Edit 的 TaskRun `scope` DecisionRequest、BDocx 动态工具门禁、CreateDocumentBundle 与 WorkspaceOperationPlan。

定位只采用 ADUS 已登记链路/规则/代码块及其直接调用关系；规则、计划和生产块均按完整块读取，不以关键词命中代替语义映射。

## 3. 相关规则与计划

| 层级 | 权威块 | 本 Scope 承接 |
|---|---|---|
| REQUIREMENT | REQ-AG-007 | RuntimeProfile 缺失/未知失败关闭；IntentResult 固定任务类型、目标、副作用、上下文、允许工具与确认点；调用最小工具集 |
| REQUIREMENT | REQ-AG-015 | TaskRun 每一步按 profile、意图、目标、上下文、允许工具和确认点重新收窄 |
| SHARED CONTRACT | BR-SYS-STATE-007 | CapabilityGrant = RuntimeProfile ∩ 任务类型 ∩ 目标 ∩ 副作用 ∩ 格式/动态能力；注册表、Prompt、前端和默认值均非授权源 |
| AG DESIGN | BR-AG-DATA-004 | IntentResult.allowed_tools 是当前 AgentTurn 精确允许集；资源意图只取得必要读取与 plan_workspace_operation；Clarify 零工具 |
| AG DESIGN | BR-AG-STATE-002 | ToolCall 必须同时通过注册、allowed_tools、权限、目标与参数；高级 BDocx 工具必须满足动态能力 |
| PLAN | WP-AG10-00 | 唯一 RuntimeProfile 入口与默认拒绝 |
| PLAN | WP-AG10-02 | answer/inspect/search/analyze/edit/create_document/clarify 的稳定 IntentResult 与目标/歧义矩阵 |
| PLAN | WP-AG10-05 | 每个 Intent 的最小 CapabilityGrant、逐工具校验与注册不等于授权 |

规则链对同一语义无冲突；旧设计变更记录中的“乐观授权”不是当前 active 规则正文，不得继续作为实现依据。

## 4. 实施前偏差与当前生产结果

| 语义 | 当前 owner | 实施前偏差 | 当前结果 |
|---|---|---|---|
| RuntimeProfile | `turn_runtime.rs::require_runtime_profile` / `ag_start_turn` | `None` 被默认成 `agent-edit-1.0` | 缺失/未知均在 Gate、provider、ToolCall 与 Proposal 前失败关闭；已注册 GUARD |
| IntentResult | `context.rs::resolve_intent_result_with_targets` | 字段为 `required_tools`，普通意图宽授权，资源操作混入 Edit | 八类 Intent 使用正式 `allowed_tools`；OrganizeWorkspace 独立，资源意图只获得计划能力 |
| CapabilityGrant | `tool_registry.rs::resolve_capability_grant` | 未注册且按宽意图档生成 | 已注册 DATA owner，并按 RuntimeProfile、任务类型、目标、副作用、格式/动态能力五因素继续收窄 |
| ToolCall 最终校验 | `tool_registry.rs::validate_requested_tool_for_grant` | 上游 CapabilityGrant 过宽且 owner 未注册 | 已注册 GUARD；注册、CapabilityGrant、目标与参数全部失败关闭 |
| Prompt/工具循环消费 | `turn_prompt.rs` / `turn_tool_loop.rs` | 消费旧宽 CapabilityGrant | 消费同一最小 CapabilityGrant，并保持 scope DecisionRequest、BDocx 动态能力、CreateDocumentBundle 与 WorkspaceOperationPlan 语义 |

实施前根因是 IntentResult 与 CapabilityGrant 的来源仍固化旧宽授权策略，并存在 RuntimeProfile 兼容默认值；上述根因已由本 Scope 的生产 owner 修复。PromptPackage/Gate 顺序与 SM-AG-02/04 的独立状态机偏差由后续 S3E 承接，不把它们混写成 S3C 未完成项。

## 5. 修改计划

1. 在模型、Gate 和工具前把缺失/未知 RuntimeProfile 稳定拒绝，并补直接负向测试。
2. 将 IntentResult 字段统一为 `allowed_tools`，建立每种 Intent 的精确候选矩阵；新增 OrganizeWorkspace，避免资源操作落入普通 Edit。
3. CapabilityGrant 按五因素继续收窄：目标范围、ExpectedSideEffect、文档格式/动态能力与当前 TaskRun 确认点共同决定最终工具集。
4. 只为复杂 Edit + TaskRun 保留 `scope` DecisionRequest；CreateDocument 与 OrganizeWorkspace 使用 `plan_workspace_operation` 自身的确认链。
5. 为 CapabilityGrant 生成和逐 ToolCall 校验添加真实 @GOV 主责块；生成 ADUS，核验 chain/rule/boundary 与实现一致。
6. 更新直接测试与验证卡，按 changed_files 运行 test:plan/test:related、test:governance 和 governance:audit。

## 6. 风险与相邻语义保护

- 资源操作若继续误判为 Edit，会在最小授权后失去 WorkspaceOperationPlan；因此 OrganizeWorkspace 必须同步落地。
- 若全局移除 `propose_decision`，会破坏 S3D 的复杂多文档 scope 扩展；本 Scope 仅保留该精确路径，并约束 decision kind。
- BDocx 高级工具不能因为“已注册”而全局开放，也不能因最小化被永久关闭；需由当前格式和动态能力决定。
- 不修改 ExplicitEditTargetSet、WorkspaceOperation、CreateDocumentBundle、DE/ED 预览或保存链。

## 7. 验证计划

- IntentResult：覆盖八类 Intent、否定写意图、缺目标澄清、资源操作不误判内容 Edit、精确 allowed_tools。
- RuntimeProfile：覆盖缺失、未知和唯一合法值，断言拒绝发生在副作用入口前。
- CapabilityGrant：覆盖五因素允许/拒绝矩阵、已注册未授权、目标与参数约束、TaskRun scope DecisionRequest、BDocx 动态能力。
- 回归：现有 AgentTurn、ToolCall、TaskRun、ExplicitEditTargetSet、CreateDocumentBundle 和 WorkspaceOperation 相关测试由影响选择器决定，不手选缩减。
- 治理：ADUS 生成后核验新增 owner 唯一注册；governance:audit 与 test:governance 通过。

## 8. Scope Lock

修改范围、禁止范围、约束、验收标准与 HD-S3C-01 以 `.governance/active_scope.json` 为准。`prompt_runtime.rs` 仅作为 CapabilityGrant 签名变化的直接测试消费者纳入，不改变 Prompt 语义。发现必须改变已冻结语义、触及 WS/DE/ED 生产实现、扩大 DecisionRequest 类型或无法唯一裁决 BDocx 动态能力时立即进入 Stop Condition，等待人类确认。

## Task Runtime Report

- scope：AG-CONFORMANCE-CODE-S3C。
- changed_files：`.governance/active_scope.json`、`.governance/issues/AG-CONFORMANCE-CODE-S3C.md`、`src-tauri/src/ag/context.rs`、`src-tauri/src/ag/tool_registry.rs`、`src-tauri/src/ag/prompt_runtime.rs`、`src-tauri/src/ag/turn_prompt.rs`、`src-tauri/src/ag/turn_runtime.rs`、`src-tauri/src/ag/turn_tool_loop.rs`、`docs/40_validation/A-VAL-X-V-04_Agent1.0验证卡.md`、`docs/00_core/A-CORE-C-R-01_ADUS.md`；均在 Scope Lock 内。
- validation：定向测试通过（IntentResult 17/17、CapabilityGrant/ToolCall 10/10、RuntimeProfile 1/1、内容 Proposal/WorkspaceOperationPlan 终态区分 1/1）；`cargo check` 通过；独立 `test:governance` 通过（19/19，959 个测试用例全部有 owner，0 个跳过/聚焦），独立 `governance:audit` 通过（310 个唯一代码块、0 个未映射可执行源、0 个治理 finding）。`test:plan` 按最终 changed_files 选择前端全量、Rust 全量与治理硬门；最终内容的 `test:related` 连续两次均为前端 592/592、S3C 相关测试及其余 Rust 373/374 通过，唯一失败为范围外 `doc::libreoffice_runtime::tests::docx_to_pdf_end_to_end_when_libreoffice_available` 的 LibreOffice Worker `signal=6`。人类以 HD-S3C-02 明确将不影响 S3C 实现判断的环境/系统验证滞后统一沉淀到待验证列表，不以其反向修改 AG 实现。
- requirement_validation：规则链已闭合到 REQUIREMENT → SHARED/AG RULE → PLAN → 生产 owner → 直接测试 → ADUS；RuntimeProfile 缺失/未知失败关闭，八类 IntentResult 使用精确 allowed_tools，CapabilityGrant 按五因素继续收窄，复杂 Edit + TaskRun 的 scope DecisionRequest 与 BDocx 动态能力保持精确可达。
- unresolved_items：真实 Tauri/provider G5 场景保留验证卡执行；范围外 LibreOffice DOCX→PDF Worker `signal=6` 进入待验证列表，由 DOC 独立治理复核，不作为 S3C 生产实现偏差。

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-08-28 | v1.0 | 人类确认 HD-S3C-01；建立完整 S3C Issue Trace 与 Scope Lock，冻结 RuntimeProfile、IntentResult、CapabilityGrant、OrganizeWorkspace、scope DecisionRequest 和 BDocx 动态门禁边界。 |
| 2026-08-29 | v1.1 | S3C 实现与自动化验证完成：新增 RuntimeProfile 入口失败关闭、八类 IntentResult 精确 allowed_tools、五因素 CapabilityGrant 与逐 ToolCall 校验；ADUS 已注册真实 owner，状态进入 VALIDATION/READY，真实 Tauri/provider G5 继续保留。 |
| 2026-08-29 | v1.2 | 最终 test:related 连续两次被范围外 LibreOffice 端到端测试的 Worker signal=6 阻断；按 ADP SC-04 将状态改为 BLOCKED，不修改 DOC 链路、不缩减选择器，等待人类裁决。 |
| 2026-08-29 | v1.3 | HD-S3C-02：人类明确将“设计与实现已对齐、仅环境/系统测试验证滞后”的事项统一沉淀到待验证列表；S3C 定向实现证据保持通过，范围外 LibreOffice signal=6 转入 DOC 待验证项，解除本 Scope 阻断并进入 READY。 |
