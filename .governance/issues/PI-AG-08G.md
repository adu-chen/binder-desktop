# Issue Trace · PI-AG-08G 真实 blockId 锚

承接 PI-AG-08F（docx 块编辑 → 由位置块号升级为稳定 blockId 锚）/ A-DOC-M-T-01 BR-DOC-DATA-003
（data-block-id 锚机制）/ A-DE-M-T-01 BR-DE-DATA-001（锚失效不产错位 diff）/ X-FORBID-006。

## 背景

PI-AG-08F 用位置 block_index——文档在注入与编辑之间变化时块号会错位、误改它块。本 PI 改用
**data-block-id 稳定锚**：按 id 定位块；id 不在当前内容里 → 明确「锚失效」报错，不产错位 diff。

## Scope Lock

```
task_type: CODE_CHANGE
scope_id: PI-AG-08G
allowed_files:
  - src-tauri/src/ag/blocks.rs        # DocBlock.block_id 捕获 + replace_block_by_id（id 未找到→Err 锚失效）
  - src-tauri/src/lib.rs              # 注入展示 [id=…]；propose_edit 优先 block_id、回退 block_index、content
  - src-tauri/src/ag/tool_registry.rs # propose_edit tool def 增 block_id
  - .governance/active_scope.json / issues/PI-AG-08G.md
forbidden_files: ADU/ADP/CLAUDE/APC / ipc/mod.rs / de/generate.rs / edStore.ts
confirmation: YES
```

## Expected Behavior

- `ag::blocks`：DocBlock 增 `block_id`（HTML 块的 data-block-id 值；text 块 None）；
  `replace_block_by_id(content, block_id, new_text)`——按 id 定位 inner 替换（in-place 非破坏）；**id 未找到→Err
  「锚失效」**（不误改）。
- 注入：HTML 块显示 `Block N [id=b1]: text`，提示优先用 block_id；text 块仍 `Block N`（无持久 id）。
- `propose_edit`：优先 `block_id`（HTML 稳定锚，AG_ANCHOR_LOST 可重试错误）→ 回退 `block_index`（位置）→ `content`（整篇）。
- tool def 增 block_id 属性。

## Validation Commands

```
cargo build --lib && cargo test --lib 'ag::blocks'   # 8/8（含 block_id 捕获/锚替换/锚失效）
```

验收：cargo build + ag::blocks 8/8；block_id 捕获正确；按 id 锚替换与位置无关；未知 id 报锚失效（不误改）。

## 后续

- 与 ED `ed.locate_block`/LocateResult 真实联动（当前 AG 侧自解析 + 整篇重建经 DE Located）；
  块 insert/delete；嵌套/畸形 HTML 健壮性；diff 卡 after 真预览。
