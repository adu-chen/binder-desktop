# Issue Trace · PI-AG-08F docx/html 块编辑

承接 PI-AG-08E（块编辑，text → 扩 docx/html）/ A-DOC-M-T-01 BR-DOC-DATA-003（data-block-id 块锚思路）/
X-FORBID-006（修改经 DE diff）。让 docx（editFormat=html）文档也能按块精确改。

## Scope Lock

```
task_type: CODE_CHANGE
scope_id: PI-AG-08F
allowed_files:
  - src-tauri/src/ag/blocks.rs        # 新建：is_html / parse_blocks / block_texts / replace_block（含 HTML 解析）
  - src-tauri/src/ag/mod.rs           # pub mod blocks
  - src-tauri/src/lib.rs              # 注入/propose_edit 改用 ag::blocks，按 active_doc_format 分流
  - src/services/agService.ts         # 传 activeDocFormat = edStore.editFormat（只读）
  - .governance/active_scope.json / issues/PI-AG-08F.md
forbidden_files: ADU/ADP/CLAUDE/APC / ipc/mod.rs / de/generate.rs / doc/block_backfill.rs / edStore.ts
confirmation: YES
```

## Expected Behavior

- `ag::blocks`（无 HTML parser 依赖，dev 级字符串扫描）：
  - `is_html(content, format)`：editFormat=html 或内容以 `<` 开头。
  - `parse_blocks`：text→按段 `\n\n`；html→扫 `data-block-id` 块级元素，记 inner 字节区间 + strip 文本。
  - `block_texts`：各块展示文本（注入用）。
  - `replace_block`：html→原串 **in-place 替换该块 inner（非破坏，保留其余 HTML）** + HTML 转义；text→替块 join。越界 Err。
- `ag_start_turn`：`active_doc_format` 入参 → `doc_is_html` → 块列表注入 + propose_edit 分流。
- propose_edit 块模式：base 用注入给模型的当前内容，按 doc_is_html 经 `replace_block` 重建整篇 → 既有 generate_diff。

## Validation Commands

```
cargo build --lib && cargo test --lib 'ag::blocks'    # 6/6（text + html 解析/替换/转义/越界）
npx tsc --noEmit
# 端到端（app+key）：打开 docx → "把第二段改成X" → propose_edit(block_index,new_text) → diff 卡片 → 接受
```

验收：cargo build + ag::blocks 6/6；HTML 块 in-place 非破坏替换（单测）；tsc 干净；docx/text 自动分流。

## 后续

- 真实 ED blockId 锚定位（对齐 LocateResult，替段落近似 + dev HTML 扫描）；嵌套块/畸形 HTML 健壮性；
  块 insert/delete（非仅 replace）；diff 卡 after 真预览。
