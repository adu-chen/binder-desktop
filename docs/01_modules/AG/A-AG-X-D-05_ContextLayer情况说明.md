---
文档编号：AG-X-D-05
文档状态：A
负责模块：AG
文档职责：WP-AG10-03 独立治理说明
上游约束：A-AG-X-D-01、A-AG-X-D-04、A-AG-X-L-04、A-AG-M-T-01、A-AG-M-T-02、A-REF-M-T-01
直接承接：Prompt Runtime、Tool 读取范围、Trace 与文档事实选择
使用边界：本文描述上下文选择和可信数据封装，不定义 Prompt 指令、Intent 决策、知识检索或长期记忆策略
变更要求：来源层级、LogicalState 优先级、裁剪、引用或 envelope schema 变化时同步 Prompt、Trace、工具读取与相关测试
---

# WP-AG10-03 · Context Layer 情况说明

## 1. 模块定位与当前裁决

该模块回答“本轮哪些文档事实可以进入模型、来自哪里、是否新鲜、以什么可信等级进入”。Binder 的核心
差异在于当前编辑器 LogicalState 可能尚未保存，因此上下文不能简单等同于磁盘文件。

当前等级：`Integrated`。当前文档、显式引用和必要历史已接入真实主回合；`agent-edit-1.0` 下自动
AST/KB/TPL 召回关闭。上下文选择的部分编排仍位于 `src-tauri/src/lib.rs`，尚未完全收敛为独立 ContextBundle。

## 2. 职责、输入与输出

| 项目 | 内容 |
|---|---|
| 输入 | IntentResult、OpenDocSnapshot/LogicalState、活动选区、ReferencePayload、session history、预算 |
| 输出 | `PromptDataEnvelopeV1` 集合、当前文档 prompt context、确定性保留的历史、裁剪/来源 trace |
| 负责 | 来源验证、workspace 限界、新鲜度、LogicalState 优先、引用只读、历史归属与预算 |
| 不负责 | system 指令、工具授权、知识检索算法、Diff 生成、模型回答 |

Envelope 当前字段：source_type、source_id、provenance、freshness、content_hash、trust_level、token_count、payload。

## 3. 当前实现地图

- `src-tauri/src/ag/context.rs`：来源优先级、历史归属/沿用、裁剪辅助、`PromptDataEnvelopeV1`。
- `src-tauri/src/ag/blocks.rs`：`AgentReadContext`、OpenDocSnapshot 与 LogicalState 读取投影。
- `src-tauri/src/lib.rs`：`build_current_doc_prompt_context`、引用格式化、历史选择与 envelope 组装。
- `src/reference/context.rs` 及 REF 相关服务：selection/reference 校验与降级呈现。
- `src/services/agRuntime.test.ts`、Rust context/reference/prompt tests：直接消费者证据。

## 4. 当前选择流程

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

## 5. 当前冻结项与允许变化项

必须冻结：

- 已打开文档 `LogicalState > DiskState`；当前快照缺失或不一致必须阻断，不能读取旧磁盘内容伪装当前事实。
- 所有 workspace 路径限 active workspace。
- 显式引用是只读数据，不自动改变写入 target；selection 失效不能猜测为当前事实。
- 历史默认弱化；跨文档历史只有当前轮显式沿用时才以弱标签进入。
- 文档、引用、历史都是 data-only，不因正文含“调用工具/忽略规则”而成为指令。
- `agent-edit-1.0` 自动 AST/KB/TPL 调用数为 0。

允许变化：token 估算、长文档阈值、裁剪算法、ContextBundle 物理结构；变化必须保持来源和裁剪可追踪。

## 6. 已验证事实

- 当前编辑器实时正文优先于 store 载入快照和磁盘内容。
- 引用 A、当前文档 B 时，A 作为显式引用，B 仍是写目标。
- 无显式沿用时跨文档历史丢弃；显式“刚才/继续”时只作弱历史注入。
- Prompt trace 的 history envelope 与实际进入模型的历史使用同一选择结果。
- 长文档注入 manifest，编辑前必须 `read_doc/list_blocks`；自动补强为 None。

独立验证条件：LogicalState/磁盘冲突、selection 过期、引用越界、多 session/workspace、长文档、预算边界、
跨文档历史、提示注入文本和自动补强调用数均有正负断言；无需调用真实 provider 即可验证。

## 7. 已知风险与待治理点

- Context 逻辑分散在 `context.rs`、`blocks.rs`、REF 和 `lib.rs`，尚无单一 `ContextBundleV1` 输出对象。
- token_count 当前使用字符数近似，不是 provider tokenizer 的精确结果。
- 当前文档 P0 与总 Prompt 预算之间采用“长文档 manifest + 工具再读”，需继续核验极端文档和多引用行为。
- `PromptPackageV1` 保存 envelope 元数据，但最终 messages 由另一装配函数构建，存在未来双表示漂移风险。
- 自动 AST/KB/TPL 已关闭，但显式 Memory/Knowledge/Template ReferencePayload 仍可进入，需持续验证来源标识。

## 8. 下一治理会话入口

必读顺序：A-AG-M-T-01 BR-AG-DATA-001/002 → A-AG-M-T-02 BR-AG-DATA-004/005 →
A-REF-M-T-01 → `context.rs` → `blocks.rs` → `lib.rs` 当前文档/引用/历史装配 → tests。

候选修改范围：Context 领域文件、主编排中的 context 部分和相关 reference/tests。若改变 IntentResult、Prompt
messages 或工具 readable_scope，须扩展到对应 owner，不得在 Context 会话隐式改写其契约。

建议会话任务描述：

> 治理 WP-AG10-03 Context Layer。核验什么进入模型、什么禁止进入、LogicalState 优先、引用只读、历史
> 归属、预算与 provenance。不要新增自动记忆/知识/模板召回。若修改，建立独立 Scope，并证明固定输入能
> 产生确定的 envelope/裁剪结果，且模型调用前即可验证全部数据边界。

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-08-08 | v1.0 | 建立 Context Layer 来源、选择流程、可信边界、验证条件、结构风险与独立治理入口 |
