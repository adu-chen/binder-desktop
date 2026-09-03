---
文档编号：AG-X-D-04
文档状态：R
负责模块：AG
文档职责：WP-AG10-02 历史治理情况参考
上游约束：R-AG-X-D-01、A-AG-X-L-04、A-AG-M-T-02、A-AG-M-T-01、A-AG-M-D-01
直接承接：无（仅供历史审计与问题定位参考）
使用边界：L4 reference-only；不定义 IntentResult 真值，当前需求与设计以 A-AG-M-D-01、A-AG-M-T-01/T-02、A-AG-I-P-01 为准
变更要求：不再随实现同步；发现差异时回到对应 Active 控制真源处理
---

# WP-AG10-02 · Intent Layer 情况说明（历史快照）

> **Reference only**：本文保留 2026-08 阶段实现审计上下文，不再作为当前需求、设计、协议或实现状态的控制源。

> [!IMPORTANT]
> 以下全部内容是 2026-08 时点快照。“当前”“必须”“不得”“冻结”“验证”和“会话入口”等用语仅记录当时的审计口径，不产生现行规范、Scope、任务授权或验收证据。
> 本文不含正式 `REQ` / `REQ_MAP` / `RULE` / `CHAIN` / `CONSTRAINT` / `TERM` 注册块；其中的标识符只是 Active 真源的历史引用指针，代码、测试和计划不得映射到本文。
> 需要当前结论时，必须回到文档头列出的 Active 需求、设计、状态机、协议与计划，并重新核验代码和测试。

## 1. 历史模块定位与快照时裁决

该模块只回答当前轮“要做什么、对谁做、是否允许产生变更、缺少什么信息”。它把当前用户原文转换成
一个结构化、可追踪、可供下游共同消费的决策，不执行工具，也不生成内容或 Diff。

快照时等级：`Module Ready`。当时实现是面向 Binder 1.0 中文文档场景的确定性解析器，不是通用分类模型；
它优先处理否定变更约束和编辑目标缺失，再使用有限语言信号选择 intent。

## 2. 输入与 IntentResultV1

输入：`turn_id`、当前用户原文、当前活动文档路径、当前活动文档是否有选区。

输出字段：

- `intent_version`、`intent_id`
- `intent_type`：answer / inspect / search / analyze / edit / create_document / clarify
- `target`：none / current_document / selection / new_document + `rel_path`
- `operation`、`mutation`
- `constraints`
- `confidence`
- `ambiguity`、`ambiguity_reasons`
- `required_tools`、`required_context`

当前没有开放 organize_workspace、Build/CBT 或多目标写入 intent。

## 3. 快照时实现地图

- `src-tauri/src/ag/context.rs`：Intent 枚举、`IntentResultV1`、`resolve_intent_result` 和单元测试。
- `src-tauri/src/lib.rs`：在门禁、Prompt 和工具之前解析；blocking ambiguity 直接返回澄清消息。
- `src-tauri/src/ag/tool_registry.rs`：消费 `intent_type/target/intent_id` 生成 CapabilityGrant。
- Prompt Runtime：把完整 IntentResult 作为 system authority 元数据注入。

## 4. 快照时决策顺序

当前轮原文
↓
识别“只分析/不要修改”等 mutation 禁止约束
↓
识别 create/edit/search/analyze/inspect 候选
↓
编辑但无活动目标 → clarify + blocking ambiguity
↓
确定 current document / selection / new document / none
↓
生成 required_tools 与 required_context
↓
Context、Prompt、CapabilityGrant、Diff 共同消费同一 intent_id

不相关问答（例如“你好”）即使编辑器打开，也不会自动把当前文档列为 required_context；明确“这篇/本文/
当前文档”或存在当前选区时才关联当前 LogicalState。

## 5. 历史冻结记录与可变项（不产生现行规则）

当时记录为冻结的事项：

- 当前轮原文是最高意图信号，历史不能覆盖本轮否定约束。
- `mutation=None` 的 answer/analyze/inspect/search 不得获得 proposal 权限。
- edit 缺少可写目标时进入 blocking clarify，不能猜测文档。
- IntentResult 不从 ReferencePayload 正文、模型文字或历史路径推导目标授权；用户显式引用身份由
  ExplicitEditTargetSet owner 消费，且成员资格不替代本层 mutation 判定。
- IntentResult 必须保留原始输入之外的结构化决策，不得用摘要替代原文。
- 下游不得各自重新猜测 allowed tools 或写入目标。

当时记录为可变的事项：中文表达信号、置信度算法、非阻断歧义策略、未来模型辅助分类。当时要求任何变化保持确定性守卫最终
裁决 mutation/target，不能让分类模型直接扩权。

## 6. 快照时验证记录（不得作为当前证据）

- “只分析，不要修改”输出 Analyze + Mutation None，无 `propose_edit`。
- “把这一段改得正式些”在当前选区输出 Edit + Selection + ProposeEdit。
- 编辑请求无活动文档输出 Clarify + Blocking。
- “你好”不要求当前文档；“这篇写得清楚吗”要求当前 LogicalState。
- Create 输出 NewDocument + ProposeCreate，后续 grant 只允许空基准 proposal。

独立验证矩阵应覆盖：七类 intent、当前文档/选区/新文档/无目标、否定约束、同义表达、历史冲突、跨文档
引用、多目标歧义、置信度边界，以及 blocking 时 provider/tool/proposal 调用数为 0。

## 7. 快照时风险记录（需重新核验）

- 当前语言识别主要是中文字符串信号，表达覆盖率有限，尚无评测数据集和混淆矩阵。
- 显式引用内容保持只读 data-only；其用户可见 workspace 文档身份可由 ExplicitEditTargetSet owner 作为目标
  授权元数据消费，但 Intent Layer 不自行把引用正文变成 target，也不因此授予写工具。
- create_document 的最终新路径由模型在工具参数中提出，IntentResult 只表达 `new_document`，不预先锁定路径。
- `confidence` 目前是固定规则值，不代表经过校准的概率；UI 不应把它解释为统计置信度。
- 当前 `required_tools` 是候选需求，最终授权仍由 CapabilityGrant 决定；两者需防止未来漂移。

## 8. 历史会话入口（已关闭）

> 以下阅读顺序、候选范围和任务描述只保留历史，不得直接复用。当前工作必须从 Active 真源重新立项。

必读顺序：A-AG-M-T-02 BR-AG-DATA-004/005 → A-AG-M-T-01 BR-AG-DATA-003 → 本文 →
`context.rs` Intent 区 → `lib.rs` blocking/consumer → `tool_registry.rs` grant 消费。

当时的候选修改范围：`context.rs` Intent 类型/解析/测试；若字段或语义变化，当时要求扩展到 `lib.rs`、Prompt、Tool、
Diff 消费者及接口文档。只增加关键词但不建立样本矩阵，不视为完成。

历史任务描述（不得直接复用）：

> 治理 WP-AG10-02 Intent Layer。以 Binder 文档编辑场景建立当前轮 IntentResult 的正确性和稳定性，核验
> mutation、target、ambiguity、required_tools/context 如何约束下游。不要把它扩成通用意图平台，也不要
> 执行工具。若修改，先建立独立 Scope，并用正向、否定、歧义和历史冲突样本证明不会错误授予写权限。

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-08-08 | v1.0 | 建立 Intent Layer 输入输出、当前规则流程、冻结语义、样本矩阵、风险与独立治理入口 |
| 2026-08-28 | v1.1 | AG-CONFORMANCE-DESIGN-S3D-COMPANION：区分 IntentResult 目标判定与 ExplicitEditTargetSet 身份授权；引用正文、模型文字和历史路径不得被 Intent Layer 用于扩权。 |
| 2026-08-31 | v1.2 | 文档真源收敛：全文定性为非规范时点快照，规则 ID 仅作引用指针；历史化冻结、验证、风险和会话入口。 |
