# Binder dev-gateway（临时本地 AI 网关）

> ⚠️ **临时 dev 脚手架，非生产。** 人类 2026-06-26 明确授权临时偏离云 APC「云代码开发启动迁出独立
> 仓库」约定（见 `.governance/issues/PI-GW-DEV-01.md` EXEMPTION-GW-DEV-01）。正式 AI 网关仍按
> `docs/02_cloud/A-GW-M-D-01` 在**独立云仓库**用 TypeScript 实现，并把 KEK/DB 换成**托管 KMS + 托管数据库**、
> 补真实订阅门禁/内容不留存审计。

让 Binder 客户端连上本地网关、用你自己的 provider key 跑通**真实流式 AI**，替换桩回复。
零依赖（仅 Node ≥18 内置 http + 全局 fetch；持久化用 Node 内置 `node:sqlite`，需 Node ≥ 22.5）。

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
| POST | `/byok/keys` | BYOK 代持：`{provider,api_key}` → **信封加密入库**（SQLite，磁盘无明文），回 `{ok,provider}`（不回显密钥）。客户端经「🔑 key」录入上送 |
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

- **BYOK**：请求体 `api_key`（经 `/byok/keys` 录入的会**信封加密入库**，重启不丢；调用时解封进内存用）。
- **平台**：环境变量 `GW_OPENAI_KEY` / `GW_DEEPSEEK_KEY` / `GW_ANTHROPIC_KEY`（仅内存用，不落盘）。
  - **设一次长期生效**：`cp .env.example .env` 填入 key，之后 `npm start` 会经 `--env-file-if-exists=.env` 自动加载（`.env` 已 gitignore，缺文件也不报错；或仍可 `GW_DEEPSEEK_KEY=… node server.mjs` 临时传）。

平台/请求 key **不落盘、不写日志**；BYOK 落库前信封加密，**磁盘无明文**。`.env`、`.data/` 均已 gitignore——不要把 key 或 DB 提交进仓库。

## 持久化与加密（PI-GW-DEV-06）

- **存储**：SQLite（Node 内置 `node:sqlite`），库文件 `.data/gw.db`（+`-wal`/`-shm`）。
  表：`sessions`（会话令牌）、`byok_keys`（**信封密文**）、`usage`（脱敏用量元数据，无正文）。替原 `.data/*.json`。
- **信封加密（正式 GW-02）**：每条 BYOK 密文用一把**独立随机 DEK**（AES-256-GCM）加密；DEK 被主密钥 **KEK** 包裹后随密文存。
  - **KEK 与密文分离**：KEK 在库外——`env GW_MASTER_KEY`（优先，32B hex/base64）或本地 `.data/gw.master.key`（dev 懒生成，0600）；密文在 `gw.db`。泄露其一不足以解密。
  - **KMS 同形**：`kms.mjs` 的 `LocalKms` 与真实云 KMS 实现同一 `generateDataKey` / `unwrapDataKey` 接口，正式仓 `GW_KMS_PROVIDER=aws|gcp` 换 provider 即可，KEK 永不离开 KMS。
- 配置：`GW_MASTER_KEY`（KEK）、`GW_KMS_PROVIDER`（默认 `local`）。

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
