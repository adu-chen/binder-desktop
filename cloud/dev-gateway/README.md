# Binder dev-gateway（临时本地 AI 网关）

> ⚠️ **临时 dev 脚手架，非生产。** 人类 2026-06-26 明确授权临时偏离云 APC「云代码开发启动迁出独立
> 仓库」约定（见 `.governance/issues/PI-GW-DEV-01.md` EXEMPTION-GW-DEV-01）。正式 AI 网关仍按
> `docs/02_cloud/A-GW-M-D-01` 在**独立云仓库**用 TypeScript 实现，并补 BYOK 加密(GW-02)/AUTH/LIC/
> 真实订阅门禁/内容不留存审计。

让 Binder 客户端连上本地网关、用你自己的 provider key 跑通**真实流式 AI**，替换桩回复。
零依赖（仅 Node ≥18 内置 http + 全局 fetch）。

## 运行

```bash
cd cloud/dev-gateway
node server.mjs          # 或 npm start
# [dev-gateway] listening on http://localhost:8787 — providers: openai, deepseek, anthropic
```

端口可改：`GW_PORT=9000 node server.mjs`。

## 端点

| 方法 | 路径 | 说明 |
|---|---|---|
| GET  | `/health` | `{ ok, providers }` |
| POST | `/auth/login` | `{email?}` → `{token,userId,expiresAt}`（dev 会话令牌，TTL 24h，**持久化**重启不丢） |
| POST | `/gate/check` | `{token}` → `{verdict,reason,reauth_required,used,limit}`（真门禁：验令牌 + 订阅 + 额度。配额 env `GW_DAILY_QUOTA`，默认 50；令牌问题→401、超额→429） |
| POST | `/byok/keys` | BYOK 代持：`{provider,api_key}` → 内存存（dev），回 `{ok,provider}`（不回显密钥）。客户端经「🔑 key」录入上送 |
| POST | `/invoke/stream` | 归一化 SSE：`event: chunk {text}` / `done {stop_reason}` / `error {code,message,retriable}` |

`/invoke/stream` 请求体：

```json
{
  "provider": "deepseek",            // openai | deepseek | anthropic
  "model": "deepseek-chat",
  "messages": [{ "role": "user", "content": "你好" }],
  "api_key": "sk-...",               // BYOK；省略则用 env GW_DEEPSEEK_KEY
  "max_tokens": 4096                  // 仅 anthropic 需要
}
```

## 密钥

- **BYOK**：请求体 `api_key`。
- **平台**：环境变量 `GW_OPENAI_KEY` / `GW_DEEPSEEK_KEY` / `GW_ANTHROPIC_KEY`。

密钥**不落盘、不写日志**。`.env` 已 gitignore——不要把 key 提交进仓库。

## 自验（需你自己的 key）

```bash
curl -s localhost:8787/health
curl -s -XPOST localhost:8787/gate/check

curl -N -XPOST localhost:8787/invoke/stream \
  -H 'Content-Type: application/json' \
  -d '{"provider":"deepseek","model":"deepseek-chat","messages":[{"role":"user","content":"用一句话介绍你自己"}],"api_key":"sk-你的key"}'
```

应看到 `event: chunk` 流式文本，末尾 `event: done`。

## 供应商适配

- `openai` / `deepseek`：OpenAI 兼容（`Authorization: Bearer`，`/chat/completions` 流式，`choices[].delta.content`）。
- `anthropic`：Messages API（`x-api-key` + `anthropic-version`）。GW 做请求翻译（OpenAI 格式 messages →
  Anthropic：system 抽顶层、tool_calls→tool_use、tool 结果→tool_result）+ tools 翻译 + tool_use 流式解析
  （归一化为同一 `tool_calls` 事件）。**可选**：仅 provider=anthropic 且配 `GW_ANTHROPIC_KEY`/BYOK 时走，
  不配不影响默认 deepseek。

## 接通客户端（后续）

客户端 `route_start_turn` 当前是桩。接通需 PI-AG-10B（`ag_start_turn` → `Channel<AgTurnEvent>`）+ PI-AG-11
（桩换真实 HTTP SSE 到本网关，SSE → `AgTurnEvent`）。见 `.governance/issues/PI-GW-DEV-01.md` §后续。
