# Issue Trace · PI-GW-DEV-02 dev GW Anthropic function-calling（可选，不强制 key）

承接 PI-GW-DEV-01 / PI-AG-08B（OpenAI 兼容已支持 tools → Anthropic 补齐）/ A-GW-M-D-01 §3.2（Anthropic
适配器：Messages API 格式差异）。让 Anthropic 也支持函数调用，且**保持可选**——客户端默认 deepseek，
Anthropic 仅在显式选择 + 配 key 时走，未配 GW_ANTHROPIC_KEY 不影响其他供应商。

## Scope Lock

```
task_type: CODE_CHANGE
scope_id: PI-GW-DEV-02
allowed_files:
  - cloud/dev-gateway/server.mjs   # streamAnthropic 请求翻译 + tools 翻译 + tool_use 流式解析
  - cloud/dev-gateway/README.md     # Anthropic 工具说明
  - .governance/active_scope.json / issues/PI-GW-DEV-02.md
forbidden_files: ADU/ADP/CLAUDE/客户端 APC/云 APC / ipc/mod.rs / lib.rs（客户端不变）
confirmation: YES
```

## Expected Behavior

- **请求翻译**（OpenAI 格式 → Anthropic）：`role:system` 抽出为顶层 `system`；`assistant` 带 `tool_calls`
  → content `[{type:tool_use,id,name,input}]`；`role:tool` → `user` 的 `[{type:tool_result,tool_use_id,content}]`；
  普通 user/assistant 文本原样。（修此前 system 在 messages 里被 Anthropic 拒的问题。）
- **tools 翻译**：OpenAI `{type:function,function:{name,description,parameters}}` → Anthropic `{name,description,input_schema}`。
- **tool_use 流式解析**：`content_block_start(tool_use)` 起一个调用（id/name），`content_block_delta(input_json_delta)`
  累积参数，`message_delta.stop_reason=tool_use` → emit 归一化 `tool_calls`（与 OpenAI 适配器同形，客户端循环零改）。
- **可选性**：仅 provider=anthropic 时执行；未配 key → 既有 401（不强制）；deepseek/openai 路径不变。

## Validation Commands

```
node --check cloud/dev-gateway/server.mjs
# 可选（需自有 Anthropic key）：provider=anthropic 调 /invoke/stream，验证文本流 + 工具调用
```

验收：node --check 通过；streamOpenAiCompat 路径不动（deepseek/openai 不受影响）；Anthropic 走翻译 + tool_use；
未配 Anthropic key 不影响默认路径。

## 后续

- 正式 GW（独立仓 + TS）将以此为参考实现 Anthropic 适配器；多 content block / 思维链 / 图片等延后。
