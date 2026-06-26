# Issue Trace · PI-AG-08E 块级锚定编辑

承接 A-AG-X-L-02 PI-AG-08（块级编辑，替整文档替换）/ A-AG-M-T-02 BR-AG-DATA-005（当前文档块视图）/
A-AG-M-T-01 X-FORBID-006（修改经 DE diff）。让模型按块号精确改文档，省 token、改动更精确。

## Scope Lock

```
task_type: CODE_CHANGE
scope_id: PI-AG-08E
allowed_files:
  - src-tauri/src/lib.rs              # doc_blocks 助手 + 块列表注入 + ag_propose_edit 块模式 + base_content
  - src-tauri/src/ag/tool_registry.rs # propose_edit tool def 增 block_index/new_text
  - .governance/active_scope.json / issues/PI-AG-08E.md
forbidden_files: ADU/ADP/CLAUDE/APC / ipc/mod.rs / de/generate.rs / edStore.ts
confirmation: YES
```

## Expected Behavior

- `doc_blocks(content)`：按段（`\n\n`）切为非空 trim 块；注入与重建**同一函数**保证块号一致。
- 当前文档注入（ag_start_turn）：改为「Block 0: …\nBlock 1: …」编号列表 + 提示「改某块用 block_index+new_text、改整篇用 content」。
- `propose_edit`：增块模式——`block_index`（来自块列表）+ `new_text`；基准用**注入给模型的当前内容**
  （base_content，保证块号对齐）切块→替目标块→`join("\n\n")` 重建整篇 after_content→既有 generate_diff（Located）
  →persist→DiffProposed。整篇模式（`content`）保留。越界 block_index 报错（不产错位 diff）。
- `tool_definitions_json` propose_edit：path 必填；content 或 (block_index+new_text) 二选一。

## Validation Commands

```
cargo build --lib && cargo test --lib 'ag::'
# 端到端（app+key）：开多段文档 → "把第二段改成X" → 模型 propose_edit(block_index=1,new_text=X) → diff 卡片 → 接受
```

验收：cargo build + ag 测试过；doc_blocks 注入/重建一致（单测）；block_index 越界报错；块模式重建整篇经 DE。

## 后续

- docx/html 块（data-block-id 切块，对齐 ED blockId 锚）；块的 insert/delete（非仅 replace）；
  真实 ED blockId 锚定位（替段落近似），对齐 A-DE-M-T-01 LocateResult。
