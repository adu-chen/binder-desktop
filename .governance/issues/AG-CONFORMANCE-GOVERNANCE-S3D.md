# Issue Trace — AG-CONFORMANCE-GOVERNANCE-S3D：显式编辑目标集合注册

文档状态：A
任务类型：GOVERNANCE_CHANGE
治理阶段：VALIDATION
任务状态：READY
创建时间：2026-08-28
上游基线：AG-CONFORMANCE-GOVERNANCE-S2；HD-S2-04 已确认。

## 1. 用户可见症状

设计要求 ChangeProposal 只能指向当前文档或用户显式指向的既有文档，但生产实现目前把集合外既有文档标为跨文档后继续放行。直接改成硬拒会误伤同一 AgentTurn 澄清新增引用、多文档任务及显式 TaskRun continuation，因为现有设计没有定义目标集合的追加事件与继承边界。

## 2. 入口点

- ADUS `AG-TURN` 下的 `BR-AG-DATA-003-DATA-AG-AG-TURN-030`：从 ReferencePayload 等输入组装 `declared_targets`。
- ADUS `AG-TURN` 下的 `BR-AG-DATA-003-GUARD-AG-AG-TURN-001`：目标集合守卫。
- ADUS `AG-TOOL` 下的 `BR-AG-DATA-003-EFFECT-AG-AG-TOOL-017`：ChangeProposal → PendingDiff 生产入口。
- ADUS `DE-DIFF` 下的 `BR-AG-DATA-003-GUARD-DE-DE-DIFF-004`：DE 侧协同复核。

## 3. 相关规则

- `BR-AG-DATA-003`：当前文档 + 用户显式指向的既有文档组成修改目标边界；集合外目标应拒绝。
- `BR-SYS-STATE-007`：澄清补充在同一 AgentTurn 内恢复。
- `BR-SYS-STATE-008`：任务级 continuation 只沿用显式 TaskRun 身份。
- `BR-AG-DATA-001/002`：LogicalState 与 PromptPackage 上下文边界，不等同内容修改授权。

## 4. 相关链路

主链 `AG-TURN → AG-TOOL → DE-DIFF`；只有需要人类确认扩展任务级目标时才交叉 `DecisionRequest / TaskRun`，不得把普通只读工具自动解释为授权事件。

## 5. 生产者块

- `turn_reference.rs::build_turn_reference_runtime`：当前只在一次后端调用开始时，从 mentions、block_refs、ReferencePayload 建立可变向量；未定义稳定实体、追加事件或 continuation 继承。
- `context.rs::guard_target_set`：注册 boundary 声明拒绝，但实现返回 `AcceptCrossDocument`。
- `edit_runtime.rs::ag_propose_edit`：把 AG/DE 两侧未声明判定合并为 `cross_doc` 展示标记，仍生成 PendingDiff。
- `de/generate.rs::target_declared`：只返回布尔判定，调用方未据此硬拒。

## 6. 消费者块

- `propose_edit` 消费目标集合并生成 ChangeProposal。
- DiffProjection / DiffCard 消费 `cross_doc` 展示标记；它是审阅提示，不应替代修改授权。
- 澄清恢复会合并同一 AgentTurn 的 ReferencePayload、mentions、blockRefs；这是现有明确追加事实。
- TaskRun 的 DecisionRequest / preview continuation 会开启后续执行片段，但当前没有可继承的目标集合权威态。

## 7. 路径上的空规则块

没有 `rules: []`；缺口是静态概念和事件定义。ADUS 术语注册表没有表示该集合的 TERM，`declared_targets` 只是未注册实现名，无法唯一约束其作用域和继承。

## 8. boundary 假设

- 引用上下文、只读访问权限和内容修改目标授权是三个不同概念。
- 当前文档与用户输入携带的显式文档 ReferencePayload 可形成初始集合。
- 同一 AgentTurn 的澄清补充可将新显式引用追加到原集合。
- read/search/list 的成功只证明读取发生，不证明用户授权修改该目标。
- 任务级 continuation 若要保持多文档语义，必须继承经确认冻结的集合，不能重新从模型输出推断。

## 9. 运行时假设

- 简单 AgentTurn 可以仅在内存持有集合，不因此创建 TaskRun。
- 动态发现后请求扩大修改范围属于任务级裁决，可在需要时创建 TaskRun。
- 若选择跨 continuation 继承，后续代码阶段需要后端权威持久态；是否新增 schema 必须在代码 Scope 单独确认。

## 10. 疑似不变量破坏

- 把 read_doc / search_docs 成功自动当作追加，会允许模型自行扩大写目标。
- 只认首次发送引用、不承认同 turn_id 澄清新增引用，会破坏 BR-SYS-STATE-007。
- 硬拒前不定义 TaskRun continuation 继承，会使已确认多文档任务在下一片段失去目标授权。
- 继续使用 `cross_doc` 提示代替授权，会保持规则与生产实现冲突。

## 11. 验证计划

1. 人类裁决目标集合的追加授权来源和 continuation 继承策略。
2. 在 AG 技术设计中注册独立 TERM，并补强 BR-AG-DATA-003 完整正文。
3. 运行 `governance:generate`，确认 TERM、RULE、CHAIN 映射唯一。
4. 运行 `test:governance` 与 `governance:audit`。
5. 规则阶段提交后，另开 CODE_CHANGE Scope 实施生产行为。

## 人类裁决

- `HD-S2-04`（CONFIRMED）：先注册显式目标集追加规则，再硬拒集合之外的既有文档；显式引用、多文档和延续语义不得被误伤。
- `HD-S3D-01`（CONFIRMED）：采用方案 1。初始集合来自当前文档和用户显式 ReferencePayload；同 AgentTurn 澄清可追加；工具发现的新目标必须以冻结路径清单发起 DecisionRequest，经确认后追加到 TaskRun 权威集合，显式 continuation 继承；read/search 本身不追加。

## Task Runtime Report

- scope：仅治理记录、AG 技术设计和机器生成 ADUS；未修改生产代码、测试或数据库。
- changed_files：`.governance/active_scope.json`、本 Issue Trace、`docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md`、机器生成的 `docs/00_core/A-CORE-C-R-01_ADUS.md`。
- implementation：注册 TERM-AG-019 `ExplicitEditTargetSet`；补强 BR-AG-DATA-003 的初始集合、同 AgentTurn 澄清追加、scope DecisionRequest 确认追加、TaskRun continuation 继承、CreateDocumentBundle 建盘后追加及集合外硬拒边界。
- mapping_chain：REQ-AG-001 → BR-AG-DATA-003 → TERM-AG-019 → AG-TURN / AG-TOOL / DE-DIFF 已完整注册；ADUS 术语数 69→70。
- semantic_review：规则正文与 HD-S3D-01 一致；现有 `AcceptCrossDocument` 生产行为仍与新规则冲突，已明确限定为下一 CODE_CHANGE Scope 的实现目标，不以本阶段结构审计通过伪装代码符合。
- validation：`governance:generate` 通过（303 个 @GOV 块、70 个术语、104 个测试资产、948 个用例）；`test:governance` 19/19 通过；`governance:audit` 通过且无 findings；`git diff --check` 通过。
- stop_conditions：无；HD-S3D-01 已解除 SC-05。
- unresolved_items：CapabilityGrant 最小化独立处理；生产实现、持久化、测试另开 CODE_CHANGE Scope。

## 读取完整性协议

- 定位只使用 ADUS 的 `BR-AG-DATA-003` 规则来源、AG-TURN / AG-TOOL / DE-DIFF 链路视图及其注册块。
- RULE 从 `<!-- RULE` 开始读取完整标注及相邻完整正文；生产块从 `/** @GOV` 开始读取完整标注与完整声明/函数。
- 同一 codes 只登记一次；跨链只列承接关系，不复制为多个事实。

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-08-28 | v1.0 | 建立 S3D DESIGN Scope；完成 BR-AG-DATA-003 规则链映射并记录目标集合追加授权的三种裁决方案。 |
| 2026-08-28 | v1.1 | 人类确认 HD-S3D-01 方案 1：显式输入/同回合澄清直接追加；工具发现目标经冻结清单 DecisionRequest 确认后追加并由 TaskRun continuation 继承。 |
| 2026-08-28 | v1.2 | 注册 TERM-AG-019 并补强 BR-AG-DATA-003；完成 ADUS 生成、测试治理、结构审计和语义比对，规则阶段进入 READY。 |
