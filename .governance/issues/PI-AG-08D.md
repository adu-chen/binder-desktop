# Issue Trace · PI-AG-08D 前端工具卡片（ToolRequest/ToolResult → ToolCallCard）

承接 A-AG-I-P-01 §2.1（AgTurnEvent::ToolRequest/ToolResult）/ A-AG-M-T-01 TERM-AG-002（ToolCall）。
UI 已就绪（MessageList 渲染 toolCalls + ToolCallCard 6 态），仅缺：agStore 无填充 toolCalls 的方法、
agService 未把 PI-AG-08B/08C 的 ToolRequest/ToolResult 事件投影进去。本 PI 补这条投影线。

## Scope Lock

```
task_type: CODE_CHANGE
scope_id: PI-AG-08D
allowed_files:
  - src/store/agStore.ts        # addToolCall / resolveToolCall / clearToolCalls
  - src/services/agService.ts   # ToolRequest→addToolCall、ToolResult→resolveToolCall、startTurn 清空
  - .governance/active_scope.json / issues/PI-AG-08D.md
forbidden_files: ADU/ADP/CLAUDE/APC / EditorTab / edStore / deStore
confirmation: YES
```

## Expected Behavior

- `agStore.addToolCall(name, detail?) -> id`：新增 executing 态 ToolCallView。
- `agStore.resolveToolCall(name, ok, detail?)`：将该 name 最早的 executing 卡置 done/failed（事件无 call id，
  按 name+executing 先到先配；dev 足够）。
- `agStore.clearToolCalls()`：新回合起清空。
- `agService`：`tool_request`→addToolCall（参数摘要 detail）；`tool_result`→resolveToolCall（结果摘要）；
  `startTurn` 起 clearToolCalls。

## Validation Commands

```
npx tsc --noEmit            # agStore/agService 无 TS 错误
# 端到端：app 内问"列出文档" → 看到 list_docs 工具卡 executing→done（人类自验）
```

验收：tsc 干净；toolCalls 由真实事件填充；executing→done/failed 态切换；新回合清空旧卡。

## 后续

- 工具卡 detail 更友好（参数/结果格式化）；awaiting_confirmation 危险授权门（写工具如已接 propose_edit 走 diff 卡，暂不需）。
