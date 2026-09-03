---
文档编号：AG-X-D-05
文档状态：R
负责模块：AG
文档职责：WP-AG10-03 历史治理情况参考
上游约束：R-AG-X-D-01、R-AG-X-D-04、A-AG-X-L-04、A-AG-M-T-01、A-AG-M-T-02、A-REF-M-T-01
直接承接：无（仅供历史审计与问题定位参考）
使用边界：L4 reference-only；不定义上下文选择真值，当前需求与设计以 A-AG-M-D-01、A-AG-M-T-01/T-02、A-AG-I-P-01 为准
变更要求：不再随实现同步；发现差异时回到对应 Active 控制真源处理
---

# WP-AG10-03 · Context Layer 情况说明（历史快照）

> **Reference only**：本文保留 2026-08 阶段实现审计上下文，不再作为当前需求、设计、协议或实现状态的控制源。

> [!IMPORTANT]
> 以下全部内容是 2026-08 时点快照。“当前”“必须”“不得”“冻结”“验证”和“会话入口”等用语仅记录当时的审计口径，不产生现行规范、Scope、任务授权或验收证据。
> 本文不含正式 `REQ` / `REQ_MAP` / `RULE` / `CHAIN` / `CONSTRAINT` / `TERM` 注册块；其中的标识符只是 Active 真源的历史引用指针，代码、测试和计划不得映射到本文。
> 需要当前结论时，必须回到文档头列出的 Active 需求、设计、状态机、协议与计划，并重新核验代码和测试。

## 1. 历史模块定位与快照时裁决

该模块回答“本轮哪些文档事实可以进入模型、来自哪里、是否新鲜、以什么可信等级进入”。Binder 的核心
差异在于当前编辑器 LogicalState 可能尚未保存，因此上下文不能简单等同于磁盘文件。

快照时等级：`Integrated`。当时的当前文档、显式引用和必要历史已接入真实主回合；`agent-edit-1.0` 下自动
AST/KB/TPL 召回关闭。上下文选择的部分编排仍位于 `src-tauri/src/lib.rs`，尚未完全收敛为独立 ContextBundle。

## 2. 职责、输入与输出

| 项目 | 内容 |
|---|---|
| 输入 | IntentResult、OpenDocSnapshot/LogicalState、活动选区、ReferencePayload、session history、预算 |
| 输出 | `PromptDataEnvelopeV1` 集合、当前文档 prompt context、确定性保留的历史、裁剪/来源 trace |
| 负责 | 来源验证、workspace 限界、新鲜度、LogicalState 优先、引用内容 data-only 与规范化身份供给、历史归属及预算 |
| 不负责 | system 指令、工具授权、知识检索算法、Diff 生成、模型回答 |

Envelope 当前字段：source_type、source_id、provenance、freshness、content_hash、trust_level、token_count、payload。

## 3. 快照时实现地图

- `src-tauri/src/ag/context.rs`：来源优先级、历史归属/沿用、裁剪辅助、`PromptDataEnvelopeV1`。
- `src-tauri/src/ag/blocks.rs`：`AgentReadContext`、OpenDocSnapshot 与 LogicalState 读取投影。
- `src-tauri/src/lib.rs`：`build_current_doc_prompt_context`、引用格式化、历史选择与 envelope 组装。
- `src/reference/context.rs` 及 REF 相关服务：selection/reference 校验与降级呈现。
- `src/services/agRuntime.test.ts`、Rust context/reference/prompt tests：直接消费者证据。

## 4. 快照时选择流程

IntentResult.required_context
↓
需要当前文档时读取发起回合的 OpenDocSnapshot
↓ 已打开文档必须使用 LogicalState；不得用 DiskState/loadedContent 兜底
构建当前文档完整块视图或长文档 manifest
↓
校验用户显式 ReferencePayload；只读注入并保留来源
↓
历史按 session、当前文档归属和显式 IntentCarryover 选择最近项
↓
形成 CurrentFact / UserProvidedData / HistoricalData envelopes
↓
预算超限在 provider 前拒绝或确定性裁剪

## 5. 历史冻结记录与可变项（不产生现行规则）

当时记录为冻结的事项：

- 已打开文档 `LogicalState > DiskState`；当前快照缺失或不一致必须阻断，不能读取旧磁盘内容伪装当前事实。
- 所有 workspace 路径限 active workspace。
- 显式引用内容是只读 data-only；Context Layer 只供给用户可见、规范化的 workspace 文档身份，由
  ExplicitEditTargetSet owner 决定成员资格，本层不自行授予写工具；selection 失效不能猜测为当前事实。
- 历史默认弱化；跨文档历史只有当前轮显式沿用时才以弱标签进入。
- 文档、引用、历史都是 data-only，不因正文含“调用工具/忽略规则”而成为指令。
- `agent-edit-1.0` 自动 AST/KB/TPL 调用数为 0。

当时记录为可变的事项：token 估算、长文档阈值、裁剪算法、ContextBundle 物理结构；当时要求变化保持来源和裁剪可追踪。

## 6. 快照时验证记录（不得作为当前证据）

- 当前编辑器实时正文优先于 store 载入快照和磁盘内容。
- 引用 A、当前文档 B 时，A 内容作为 data-only 引用，A/B 的规范化身份均可供 ExplicitEditTargetSet owner
  消费；是否具备写工具仍由 IntentResult 与 CapabilityGrant 决定。
- 无显式沿用时跨文档历史丢弃；显式“刚才/继续”时只作弱历史注入。
- Prompt trace 的 history envelope 与实际进入模型的历史使用同一选择结果。
- 长文档注入 manifest，编辑前必须 `read_doc/list_blocks`；自动补强为 None。

独立验证条件：LogicalState/磁盘冲突、selection 过期、引用越界、多 session/workspace、长文档、预算边界、
跨文档历史、提示注入文本和自动补强调用数均有正负断言；无需调用真实 provider 即可验证。

## 7. 快照时风险记录（需重新核验）

- Context 逻辑分散在 `context.rs`、`blocks.rs`、REF 和 `lib.rs`，尚无单一 `ContextBundleV1` 输出对象。
- token_count 当前使用字符数近似，不是 provider tokenizer 的精确结果。
- 当前文档 P0 与总 Prompt 预算之间采用“长文档 manifest + 工具再读”，需继续核验极端文档和多引用行为。
- `PromptPackageV1` 保存 envelope 元数据，但最终 messages 由另一装配函数构建，存在未来双表示漂移风险。
- 自动 AST/KB/TPL 已关闭，但显式 Memory/Knowledge/Template ReferencePayload 仍可进入，需持续验证来源标识。

## 8. 历史会话入口（已关闭）

> 以下阅读顺序、候选范围和任务描述只保留历史，不得直接复用。当前工作必须从 Active 真源重新立项。

必读顺序：A-AG-M-T-01 BR-AG-DATA-001/002 → A-AG-M-T-02 BR-AG-DATA-004/005 →
A-REF-M-T-01 → `context.rs` → `blocks.rs` → `lib.rs` 当前文档/引用/历史装配 → tests。

当时的候选修改范围：Context 领域文件、主编排中的 context 部分和相关 reference/tests。若改变 IntentResult、Prompt
messages 或工具 readable_scope，须扩展到对应 owner，不得在 Context 会话隐式改写其契约。

历史任务描述（不得直接复用）：

> 治理 WP-AG10-03 Context Layer。核验什么进入模型、什么禁止进入、LogicalState 优先、引用内容 data-only 与身份供给、历史
> 归属、预算与 provenance。不要新增自动记忆/知识/模板召回。若修改，建立独立 Scope，并证明固定输入能
> 产生确定的 envelope/裁剪结果，且模型调用前即可验证全部数据边界。

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-08-08 | v1.0 | 建立 Context Layer 来源、选择流程、可信边界、验证条件、结构风险与独立治理入口 |
| 2026-08-28 | v1.1 | AG-CONFORMANCE-DESIGN-S3D-COMPANION：分离引用内容的 data-only 信任边界与规范化引用身份供给；Context Layer 不授予写工具，ExplicitEditTargetSet owner 消费身份。 |
| 2026-08-31 | v1.2 | 文档真源收敛：全文定性为非规范时点快照，规则 ID 仅作引用指针；历史化冻结、验证、风险和会话入口。 |
