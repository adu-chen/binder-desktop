# Issue Trace · PI-AG-09E REF @mention 供给（完整 PI-AG-09 收尾）

承接 A-AG-X-L-02 PI-AG-09 / A-AG-M-T-01 BR-AG-DATA-001（来源限定：REF @mention；显式注入优先保留、不
静默丢弃）/ A-AG-M-T-02 BR-AG-DATA-005（引用层 P1）/ X-FORBID-004。补上 PI-AG-09 唯一余项：用户显式
引用文档。

## Scope Lock

```
task_type: CODE_CHANGE
scope_id: PI-AG-09E
allowed_files:
  - src-tauri/src/lib.rs              # ag_start_turn 接 mentions → 读引用文档 → P1「[用户显式引用]」注入 + 历史预算扣减
  - src/components/ag/Composer.tsx    # 文档引用选择器（wsStore.docs 只读，chips，发送传 rel_paths）
  - src/services/agService.ts         # startTurn opts.mentions → 传 ag_start_turn
  - .governance/active_scope.json / issues/PI-AG-09E.md
forbidden_files: ADU/ADP/CLAUDE/APC / ipc/mod.rs / ag/context.rs（复用不改）/ edStore / wsStore（只读）
confirmation: YES
```

## Expected Behavior

- `ag_start_turn`：新增 `mentions: Vec<String>`（rel_paths）；逐个 `resolve_in_workspace`（X-FORBID-004）+
  `read_doc`（含 docx Pandoc）→ 每篇 `cap_text(MENTION_CAP=4000)` → 拼「[用户显式引用] …高优先，请据此回答」
  （P1，显式优先保留、不静默丢弃 BR-AG-DATA-001）；置于 历史后 / 当前文档前；历史预算扣除引用字符。空则不注入。
- `Composer`：「@引用」入口列 `wsStore.docs`（非目录）多选 → chips（可移除）；发送时 `startTurn(text,{mentions})` 后清空。
- `agService.startTurn`：`opts.mentions` → `invoke(..., { mentions })`。

## Validation Commands

```
cargo build --lib && cargo test --lib 'ag::'
npx tsc --noEmit
# 端到端：@引用 另一篇文档 → 提问 → AI 据被引用文档内容回答（人类自验）
```

验收：cargo build + ag 测试过；tsc 干净；引用文档被读入注入、限 workspace、超长封顶、显式优先（不被预算丢弃）。

## 后续

- 块级 @mention（引用某块，经 ed.locate_block 精确锚 + reference::inject build_injection）；
  内联 @ 自动补全（替选择器按钮）；引用去重/失效提示。
