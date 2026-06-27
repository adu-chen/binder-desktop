# Issue Trace · PI-AG-09G 引用非当前文档的块 + 样式收尾

承接 PI-AG-09F（块级 @mention 仅当前文档选区 → 扩到任意文档的块）+ PI-AG-09E/F 的 mention UI 样式收尾。

## Scope Lock

```
task_type: CODE_CHANGE
scope_id: PI-AG-09G
allowed_files:
  - src-tauri/src/lib.rs              # route_list_blocks（ag.list_blocks）+ dispatch/registry
  - src/services/agService.ts          # listDocBlocks（dispatch callCommand）
  - src/components/ag/Composer.tsx      # 选择器每文档「块▸」展开 → 点块加 blockRef（非当前文档）
  - src/styles.css                      # mention-chip/picker/option/block 样式（append 自有类）
  - .governance/active_scope.json / issues/PI-AG-09G.md
forbidden_files: ADU/ADP/CLAUDE/APC / ipc/mod.rs / edStore / wsStore
confirmation: YES
```

## Expected Behavior

- 后端 `ag.list_blocks(relPath)`：resolve_in_workspace（X-FORBID-004）+ read_doc（含 docx）→ `ag::blocks::parse_blocks`
  （is_html 自判）→ `{ blocks: [{ block_id, text }] }`。注册进 dispatch + build_registry。
- `agService.listDocBlocks(relPath)`：经 dispatch 调 ag.list_blocks → 块列表。
- `Composer`：选择器每文档行加「块▸」展开——调 listDocBlocks 列其块（编号 + 文本摘要），点某块 →
  加 blockRef `{relPath, blockId: block_id??"", text}`（复用 PI-AG-09F block_refs，非当前文档亦可）。
- `styles.css`：补 `.composer-mentions/.mention-chip/.mention-chip-x/.mention-picker/.mention-option/.mention-blocks` 样式。

## Validation Commands

```
cargo build --lib && npx tsc --noEmit
# 端到端：@引用 展开另一文档的块 → 选一块 → 提问 → AI 据该块作答；mention UI 有样式（人类自验）
```

验收：cargo build + tsc 干净；ag.list_blocks 返回块列表、限 workspace；非当前文档块可引用；mention UI 样式生效。

## 后续

- 内联 @ 自动补全；reference::inject 锚校验；块引用去重/计数。
