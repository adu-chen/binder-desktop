# Issue Trace · PI-AG-08B 真实工具（只读 3：read_doc/list_docs/search_text）

承接 A-AG-X-L-02 PI-AG-08（执行半，08A 注册表已交付）/ A-AG-M-T-01 BR-AG-STATE-002（工具循环：注册/
权限校验 + 上限 + 结果回注）/ X-FORBID-006（写意图不直写，propose_edit 后续）/ X-FORBID-004（只读工具限
workspace）/ A-AG-I-P-01 §2.1（AgTurnEvent::ToolRequest/ToolResult）。让模型经 function-calling 真正
浏览/搜索工作区文档。

## Scope Lock

```
task_type: CODE_CHANGE
scope_id: PI-AG-08B
allowed_files:
  - cloud/dev-gateway/server.mjs        # GW 转发 tools + 累积 OpenAI tool_calls → emit tool_calls 事件
  - src-tauri/src/ag/tool_exec.rs       # 新建：exec_read_only（list_docs/read_doc/search_text 经 WS）
  - src-tauri/src/ag/tool_registry.rs   # tool_definitions_json（OpenAI function 定义，3 只读）
  - src-tauri/src/ag/mod.rs             # pub mod tool_exec
  - src-tauri/src/ent/gateway.rs        # stream_invoke→stream_round（GwToolCall/RoundOutcome）
  - src-tauri/src/lib.rs                # ag_start_turn 工具循环 + AppState（解析 workspace 根）
  - .governance/active_scope.json / issues/PI-AG-08B.md
forbidden_files: ADU/ADP/CLAUDE/APC / ipc/mod.rs / edStore / EditorTab
confirmation: YES
```

## Expected Behavior

- GW：`/invoke/stream` 接 `tools` 转发 provider；流式累积 `delta.tool_calls`（按 index 拼 id/name/arguments），
  `finish_reason=tool_calls` → emit `event: tool_calls`（交客户端执行，工具在客户端）。
- 客户端 `ent::gateway::stream_round`：一轮 GW 调用；content→Chunk；tool_calls→返回 ToolCalls（不 emit Done）；
  done 且无工具→emit Done。`ag_start_turn` 循环（≤6 轮，BR-AG-STATE-002 上限）：ToolCalls→入 assistant.tool_calls
  消息→逐个 validate_requested_tool（注册/权限）+ exec_read_only（限 workspace 根，X-FORBID-004）→ emit
  ToolRequest/ToolResult→tool 结果回注→续轮；Done→结束。
- `exec_read_only`：list_docs/search_text（`workspace::listing`）、read_doc（`resolve_in_workspace` 守界 +
  `save::read_doc`）；未注册/未接线工具回 Reject/Err（回注模型，不中断）。propose_edit 写/diff 后续。

## Validation Commands

```
cargo build --lib && cargo test --lib 'ag::tool'        # 9/9（含 tool_exec/tool_registry）
node --check cloud/dev-gateway/server.mjs
# 端到端（GW+真实 key）：① 给 list_docs 工具问"列文档"→ GW emit tool_calls（已验）
#                       ② 回灌 tool 结果 → 模型据结果作答（已验）
```

验收：cargo build 干净 + ag::tool 9/9；GW emit tool_calls 已实测；工具循环第 2 轮模型据结果作答已实测；
未注册工具被 validate 拒绝回注；越界路径被 resolve_in_workspace 拒绝。

## 后续

- **propose_edit**（写/diff）：ChangeProposal→de.generate_diff→DiffProposed 卡片（X-FORBID-006），需锚定位。
- 前端工具卡片（ToolRequest/ToolResult→UI）；Anthropic function-calling（格式不同）；完整 SM-AG-02 驱动。
