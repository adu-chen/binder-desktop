# Issue Trace · PI-AG-09F REF 块级 @mention（选区精确锚）

承接 PI-AG-09E（整篇 @mention → 块级精确）/ A-AG-M-T-01 BR-AG-DATA-001（显式注入优先保留）/ A-DOC
（data-block-id 块锚）/ ed.locate（PI-AG-08H 前端实时块定位）。让用户引用文档的**某一块/选区**，而非整篇。

## Scope Lock

```
task_type: CODE_CHANGE
scope_id: PI-AG-09F
allowed_files:
  - src-tauri/src/lib.rs              # BlockRef 结构 + ag_start_turn block_refs 入参 + 并入显式引用注入
  - src/services/edLocate.ts          # captureSelectionRef（选区文本 + 上溯 data-block-id 块锚）
  - src/components/ag/Composer.tsx     # 「@引用选区」入口 + 块引用 chips
  - src/services/agService.ts          # 传 blockRefs
  - .governance/active_scope.json / issues/PI-AG-09F.md
forbidden_files: ADU/ADP/CLAUDE/APC / ipc/mod.rs / ag/context.rs / edStore / wsStore
confirmation: YES
```

## Expected Behavior

- `edLocate.captureSelectionRef() -> { blockId, text } | null`：读 `window.getSelection()` 选区文本，从锚点
  上溯最近 `[data-block-id]` 块；无选区/不在块内 → null。
- `Composer`：「@引用选区」按钮 → captureSelectionRef → 加块引用 chip（rel_path=当前 openPath、blockId、text 摘要）；
  发送时随 `startTurn(text, { mentions, blockRefs })` 传出，后清空。
- 后端 `ag_start_turn`：新增 `block_refs: Vec<BlockRef{rel_path,block_id,text}>`（serde camelCase）；并入
  「[用户显式引用]」P1 块——「引用 文档「rel_path」第 [block_id] 块的选中内容：text」（cap_text 封顶，显式
  优先保留不静默丢弃 BR-AG-DATA-001）。

## Validation Commands

```
cargo build --lib && cargo test --lib 'ag::'
npx tsc --noEmit
# 端到端：编辑器选中一段 → @引用选区 → 提问 → AI 据该选区精确作答（人类自验）
```

验收：cargo build + ag 测试过；tsc 干净；选区块锚捕获正确；块引用注入精确到块、封顶、显式优先。

## 后续

- 引用非当前文档的块（picker 列目标文档块）；经 reference::inject build_injection + ed.locate 校验锚有效性；样式收尾。
