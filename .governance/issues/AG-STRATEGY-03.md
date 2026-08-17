# Issue Trace — AG-STRATEGY-03：文档感知 + 块 id + 排版意识 + 确认去重

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-11

## 1. 用户可见症状

真实测试暴露 5 类问题：
1. 第一轮「把故事丰富一下吧，排版太烂了。能不能有点节奏？」被判零工具 Answer——动词表无「丰富/排版」，文档指涉词无「故事」，模型看不到内容、无工具，只能文字征询。
2. 修复 b-docx 后出现残余碎片（「快要死了。」「温去温暖它。」）——insert 新块在投影中复用锚点 block_id，标题块与正文块同 id 导致渲染/应用错乱。
3. 无排版/样式意识——模型把"优化排版"实现为文本重排 + 插入 paragraph 文本，不生成 style_ops/heading 结构。
4. 反复文字确认（「认可吗」「说修我就出卡」）——模型自发征询，非权限问题（权限已全量授予）。
5. 其余：insert 新块无法声明独立 id（schema additionalProperties:false 禁 blockId）；模型不自检产出；弱指涉词失效；整篇重写被逼进 block_ops 窄通道。

## 2. 入口点

- `src-tauri/src/ag/context.rs`：EDIT_VERBS/detect_target/document_referenced 词表补强；Answer/Clarify 意图且 has_active_doc 时注入当前文档。
- `src-tauri/src/ag/bdocx_capabilities.rs`：insert/append afterNode 允许可选 blockId。
- `src-tauri/src/ag/block_ops.rs`：insert/append 新块 after_node 保留 blockId（若前端提供）。
- `src-tauri/src/lib.rs`：block_ops 投影 after 侧对 insert/append 新块用独立 id 标注（不再复用锚点 id）。
- `src-tauri/src/ag/prompts.rs`：排版映射（style_ops/heading）、确认去重、生成后自检。

## 3. 相关规则

- BR-AG-DATA-003 / BR-AG-STATE-002：意图与目标语义不变，仅补弱指涉词与兜底注入；安全仍由调用时刻校验 + DiffCard 确认承担。
- 无规则正文改动。

## 4. 相关链路

意图判定 → 上下文注入 → block_ops 校验/投影 → prompt 指导 → 模型 propose_edit → DiffCard。

## 5. 生产者块

无新增 RULE/CHAIN。

## 6. 消费者块

- context.rs tests（动词/指涉/兜底注入）。
- block_ops.rs tests（insert afterNode blockId）。
- lib.rs tests（投影独立 id）。
- prompts.rs tests（排版/确认/自检断言）。

## 7. 空规则块

未发现。

## 8. boundary 假设

- `lifecycle_assumption=`：insert 新块独立 id 由前端 ProseMirror 应用层最终分配；后端投影仅标注不决策应用。
- `document_snapshot=`：Answer 意图兜底注入当前文档内容（只读），不授写工具。

## 9. 运行时假设

- 「丰富/扩写/充实/重排/排版故事/寓言」→ Edit 全量工具 + 注入当前文档。
- Answer/Clarify 且打开文档 → 注入当前文档内容（看得见）。
- insert/append 新块投影用独立 id 标注；模型可显式给新块 blockId。

## 10. 疑似不变量破坏

- 动词表/指涉词扩展不改变 Edit→Create→Search 首命中语义，只增补弱信号覆盖。
- Answer 兜底注入只读内容，不授写工具（HITL 不变）。

## 11. 版本历史

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-08-11 | v1.0 | 初始版本 |

## 12. Task Runtime Report — AG-STRATEGY-03

### 12.1 任务结果

- 状态：`READY`。

### 12.2 实际变更

- **context.rs**：EDIT_VERBS_STRONG 补「丰富/扩写/充实/重排/排版/美化」；detect_target 文档指涉词补「故事/寓言/童话/小说/稿件」；document_referenced 补「这个故事/这篇故事/这个寓言」；Answer/Inspect/Analyze 意图且 has_active_doc && !greeting 时兜底注入当前文档（只读，纯寒暄不注入）。
- **bdocx_capabilities.rs**：insert_before/insert_after/append 的 afterNode 允许可选 blockId。
- **lib.rs**：block_ops 投影 after 侧对 insert/append 新块用 afterNode.blockId 或生成 new_N 独立标注（不再复用锚点 id）。
- **prompts.rs**：补排版映射（style_ops + heading 新块）、确认去重（默认直接 propose_edit）、生成后自检（list_blocks 核对 + 删残句）。
- **测试**：context.rs 新增 `intent_writing_and_layout_requests_are_edit`。

### 12.3 验证结果

| 验证 | 结果 |
|---|---|
| `cargo test`（Rust 全量） | PASS：316 + 5 + 4，0 失败 |
| `npm run governance:generate` / `governance:audit` | PASS：878 cases / 0 warnings |
| `git diff --check` | PASS |

### 12.4 语义复核

- 弱写作/排版请求 → Edit 全量工具 + 注入当前文档（问题1 根因解除）。
- insert 新块独立 id 标注（问题2/5a 投影错乱解除）；真实前端 ProseMirror 应用层 id 分配不在本 scope（见 12.5）。
- prompt 补排版映射/确认去重/自检（问题3/4/5b 缓解）。
- 规则正文不改。

### 12.5 未解决项

- insert 新块在**前端 ProseMirror 应用层**的最终独立 id 分配未在本 scope 改（src/** 前端 out_of_scope）——后端已允许携带 blockId + 投影独立标注，前端应用层是否真正分配新 id 需前端侧跟进验证。
- 整篇重写仍被逼进 block_ops 窄通道（既有 b-docx 禁 replace_document），多块重写错乱风险仍在——需评估是否放开 b-docx 整篇替换。
- 真实 G5 运行验证待用户复测。

### 12.6 READY 自检

- 范围/验证/证据/完整/诚实/留痕：通过；无 TODO/桩/跳过。
