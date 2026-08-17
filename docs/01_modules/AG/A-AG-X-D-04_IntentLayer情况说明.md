---
文档编号：AG-X-D-04
文档状态：A
负责模块：AG
文档职责：WP-AG10-02 独立治理说明
上游约束：A-AG-X-D-01、A-AG-X-L-04、A-AG-M-T-02、A-AG-M-T-01、A-AG-M-D-01
直接承接：Context Layer、Prompt Runtime、Tool Capability、Editor/Diff 目标与变更语义
使用边界：本文描述 IntentResult 当前决策机制，不把 Intent 当作通用语义理解系统，不负责读取大上下文或执行动作
变更要求：IntentResult 字段、枚举、目标或歧义语义变化时同步设计、Prompt、Grant、Diff 目标和全部消费者测试
---

# WP-AG10-02 · Intent Layer 情况说明

## 1. 模块定位与当前裁决

该模块只回答当前轮“要做什么、对谁做、是否允许产生变更、缺少什么信息”。它把当前用户原文转换成
一个结构化、可追踪、可供下游共同消费的决策，不执行工具，也不生成内容或 Diff。

当前等级：`Module Ready`。当前实现是面向 Binder 1.0 中文文档场景的确定性解析器，不是通用分类模型；
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

## 3. 当前实现地图

- `src-tauri/src/ag/context.rs`：Intent 枚举、`IntentResultV1`、`resolve_intent_result` 和单元测试。
- `src-tauri/src/lib.rs`：在门禁、Prompt 和工具之前解析；blocking ambiguity 直接返回澄清消息。
- `src-tauri/src/ag/tool_registry.rs`：消费 `intent_type/target/intent_id` 生成 CapabilityGrant。
- Prompt Runtime：把完整 IntentResult 作为 system authority 元数据注入。

## 4. 当前决策顺序

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

## 5. 当前冻结项与允许变化项

必须冻结：

- 当前轮原文是最高意图信号，历史不能覆盖本轮否定约束。
- `mutation=None` 的 answer/analyze/inspect/search 不得获得 proposal 权限。
- edit 缺少可写目标时进入 blocking clarify，不能猜测文档。
- IntentResult 必须保留原始输入之外的结构化决策，不得用摘要替代原文。
- 下游不得各自重新猜测 allowed tools 或写入目标。

允许变化：中文表达信号、置信度算法、非阻断歧义策略、未来模型辅助分类。任何变化必须保持确定性守卫最终
裁决 mutation/target，不能让分类模型直接扩权。

## 6. 已验证事实

- “只分析，不要修改”输出 Analyze + Mutation None，无 `propose_edit`。
- “把这一段改得正式些”在当前选区输出 Edit + Selection + ProposeEdit。
- 编辑请求无活动文档输出 Clarify + Blocking。
- “你好”不要求当前文档；“这篇写得清楚吗”要求当前 LogicalState。
- Create 输出 NewDocument + ProposeCreate，后续 grant 只允许空基准 proposal。

独立验证矩阵应覆盖：七类 intent、当前文档/选区/新文档/无目标、否定约束、同义表达、历史冲突、跨文档
引用、多目标歧义、置信度边界，以及 blocking 时 provider/tool/proposal 调用数为 0。

## 7. 已知风险与待治理点

- 当前语言识别主要是中文字符串信号，表达覆盖率有限，尚无评测数据集和混淆矩阵。
- 显式引用目前主要作为只读上下文；“修改被引用但非当前文档”的目标语义在 1.0 被有意收窄。
- create_document 的最终新路径由模型在工具参数中提出，IntentResult 只表达 `new_document`，不预先锁定路径。
- `confidence` 目前是固定规则值，不代表经过校准的概率；UI 不应把它解释为统计置信度。
- 当前 `required_tools` 是候选需求，最终授权仍由 CapabilityGrant 决定；两者需防止未来漂移。

## 8. 下一治理会话入口

必读顺序：A-AG-M-T-02 BR-AG-DATA-004/005 → A-AG-M-T-01 BR-AG-DATA-003 → 本文 →
`context.rs` Intent 区 → `lib.rs` blocking/consumer → `tool_registry.rs` grant 消费。

候选修改范围：`context.rs` Intent 类型/解析/测试；若字段或语义变化，必须扩展到 `lib.rs`、Prompt、Tool、
Diff 消费者及接口文档。只增加关键词但不建立样本矩阵，不视为完成。

建议会话任务描述：

> 治理 WP-AG10-02 Intent Layer。以 Binder 文档编辑场景建立当前轮 IntentResult 的正确性和稳定性，核验
> mutation、target、ambiguity、required_tools/context 如何约束下游。不要把它扩成通用意图平台，也不要
> 执行工具。若修改，先建立独立 Scope，并用正向、否定、歧义和历史冲突样本证明不会错误授予写权限。

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-08-08 | v1.0 | 建立 Intent Layer 输入输出、当前规则流程、冻结语义、样本矩阵、风险与独立治理入口 |
