# Issue Trace — AG-STRATEGY-04：放开整篇替换 + insert 新块唯一 id

文档状态：A
任务类型：CODE_CHANGE
治理阶段：IMPLEMENTATION
创建时间：2026-08-12

## 1. 用户可见症状

多块重写 b-docx 出现错乱（残句、标题块丢失/同 id），根因有二：
1. 既有 b-docx 禁 replace_document → 整篇重写被逼进 block_ops 窄通道（单批 ≤32 + 批次 base 漂移 + insert 新块无独立 id）。
2. block_ops insert/append 新块 afterNode 无 attrs.blockId → 前端 nodeFromJSON 生成的新块无 id → 跨批/后续操作无法锚定。

用户裁决：放开整篇替换是合理逃逸通道，但非多块错乱根治；多块错乱须单独治 + 兜底。

## 2. 入口点

- `src-tauri/src/lib.rs`：AG_RICH_BLOCK_OPERATION_REQUIRED 禁制移除 ReplaceDocument（保留 ReplaceBlock）。
- `src-tauri/src/ag/bdocx_capabilities.rs`：insert/append afterNode 结构改为 type/attrs/content（attrs 含 blockId），与 replace_node 一致。
- `src-tauri/src/ag/block_ops.rs`：insert/append 校验时兜底生成唯一 attrs.blockId。

## 3. 相关规则

- BR-ED-DATA-001 / BR-ED-DATA-003：图片守卫（guard_bdocx_raw_image_edit replace_document=true）不变；identity 校验不变。
- 无规则正文改动。

## 4. 相关链路

propose_edit(replace_document/block_ops) → 校验 → DiffCard → 前端应用。

## 5. 生产者块

无新增 RULE/CHAIN。

## 6. 消费者块

- lib.rs tests（replace_document 对既有 b-docx 放行 + 图片仍拦）。
- block_ops.rs tests（insert/append 兜底生成 attrs.blockId）。
- bdocx_capabilities.rs tests（schema 校验）。

## 7. 空规则块

未发现。

## 8. boundary 假设

- `lifecycle_assumption=`：replace_document 对含图片 b-docx 仍被 guard_bdocx_raw_image_edit 拦截；identity 校验防错文档。
- `document_snapshot=`：insert/append 新块唯一 id 由后端兜底生成（attrs.blockId），前端 nodeFromJSON 依 schema 解析。

## 9. 运行时假设

- 既有 b-docx（无图片）可用 replace_document 整篇重写（HTML content）。
- insert/append 新块 always 带唯一 attrs.blockId（模型未给则由后端生成 new_block_{uuid}）。

## 10. 疑似不变量破坏

- 放开 replace_document 后，富文档整篇替换可能丢失表格/复杂样式（无表格守卫）——identity 校验 + HITL 完整 before/after 兜底，作为已知边界记录。

## 11. 版本历史

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-08-12 | v1.0 | 初始版本 |

## 12. Task Runtime Report — AG-STRATEGY-04

### 12.1 任务结果

- 状态：`READY`。

### 12.2 实际变更

- **lib.rs**：`AG_RICH_BLOCK_OPERATION_REQUIRED` 禁制移除 `ReplaceDocument`（仅保留 `ReplaceBlock`）——既有 b-docx（无图片）放开整篇替换；identity 漂移校验 + `guard_bdocx_raw_image_edit`（replace_document=true 分支）保留；投影 after 侧 insert/append 读 `afterNode.attrs.blockId`。
- **bdocx_capabilities.rs**：insert_before/insert_after/append 的 afterNode 结构改为 `type/attrs/content`（attrs 含可选 blockId），对齐 replace_node。
- **block_ops.rs**：新增 `ensure_new_block_id`——insert/append 新块缺省时兜底生成唯一 `attrs.blockId`（new_block_{uuid}），模型显式给则保留。
- **测试**：block_ops.rs 新增 `insert_new_block_gets_fallback_unique_block_id`。

### 12.3 验证结果

| 验证 | 结果 |
|---|---|
| `cargo test`（Rust 全量） | PASS：317 + 5 + 4，0 失败 |
| `npm run governance:generate` / `governance:audit` | PASS：879 cases / 0 warnings |
| `git diff --check` | PASS |

### 12.4 语义复核

- 既有 b-docx（无图片）可用 replace_document 整篇重写（identity 漂移 + 图片守卫兜底）。
- insert/append 新块 always 带唯一 attrs.blockId，前端 nodeFromJSON 生成带 id 新块，可跨批锚定（多块错乱根治核心）。
- 规则正文不改。

### 12.5 未解决项

- 前端应用后语义校验（块数/内容比对）与快照回滚未实现——replace_document + insert 唯一 id 已解决主要错乱，兜底网后续跟进。
- 表格/复杂样式整篇替换的丢失守卫未实现（仅图片守卫）——含表格 b-docx 整篇 replace_document 可能丢失表格结构，需后续加表格守卫或强制走 block_ops。
- 整篇重写单批化（MAX_OPERATIONS 提升）未做——replace_document 已承担整篇重写，block_ops 保留给局部多块修改（≤32）。
- 真实 G5 运行验证待用户复测。

### 12.6 READY 自检

- 范围/验证/证据/完整/诚实/留痕：通过；无 TODO/桩/跳过。
