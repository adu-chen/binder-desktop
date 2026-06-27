# Issue Trace · PI-AG-09I diff 卡 after 真预览（B 类打磨）

承接 PI-AG-08C/F（propose_edit diff 卡 after 一直是占位 → 显示真实提案内容）。propose_edit 的内容经
既有 tool_request.params 已传达前端，捕获即用，无需改后端/契约。

## Scope Lock

```
task_type: CODE_CHANGE
scope_id: PI-AG-09I
allowed_files:
  - src/services/agService.ts   # tool_request(propose_edit) 捕获 new_text/content；diff_proposed 用作 after 预览
  - .governance/active_scope.json / issues/PI-AG-09I.md
forbidden_files: ADU/ADP/CLAUDE/APC / ipc/mod.rs / edStore / deStore
confirmation: YES
```

## Expected Behavior

- `agService`：本回合内从 `tool_request`（tool_name=propose_edit）的 params 捕获 `new_text`（块模式）或
  `content`（整篇）作为 after 预览文本（turn 内重置，与 lastEditBlockId 同源）。
- `diff_proposed`：`deStore.addDiff` 的 `after` 用捕获的预览片段（截断）；无则回退占位文字。

## Validation Commands

```
npx tsc --noEmit
# 端到端：让 AI 改某块 → diff 卡 after 显示真实新内容（人类自验）
```

验收：tsc 干净；propose_edit 后 diff 卡 after 显示真实提案内容片段；无捕获时回退占位（不报错）。

## 后续

- 真正的 before/after 行级 diff 渲染（当前 before=现内容尾片、after=提案片段）；BYOK 客户端入口。
```
