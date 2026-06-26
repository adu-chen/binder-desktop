# Issue Trace · PI-GW-DEV-01 临时本地 dev AI 网关

承接 A-GW-M-D-01（REQ-GW-001 供应商适配 / REQ-GW-002 统一流式 / REQ-GW-005 CLOUD-GATE / GW-01 双模）
的**最小 dev 子集**，让客户端可连上 localhost GW、用用户 key 跑通真实流式 AI，替换桩回复。

## ⚠️ 豁免留痕（unresolved_items / CLAUDE.md §9）

- **EXEMPTION-GW-DEV-01**：云 APC（A-CLOUD-C-P-01 §1/§3）规定「云代码开发启动时迁出至独立仓库」。
  人类 **2026-06-26 明确授权**在本客户端仓库 `cloud/dev-gateway/` 临时建本地 dev GW，偏离该约定。
  **正式 GW 仍须**：迁独立仓库、改 TypeScript、补 BYOK 加密（GW-02 待决）、真实 AUTH/LIC/CLOUD-GATE、
  内容不留存合规审计（REQ-GW-006）。本 dev 版为一次性开发脚手架，不进生产、不随云端 ADUS 注册。
- `cloud/dev-gateway/` 为代码目录（非 docs/02_cloud/），不含注册类治理块/@GOV，不污染客户端 ADUS。

## Scope Lock

```
task_type: CODE_CHANGE
scope_id: PI-GW-DEV-01
allowed_files:
  - cloud/dev-gateway/server.mjs       # 零依赖 Node ESM dev GW
  - cloud/dev-gateway/package.json
  - cloud/dev-gateway/README.md
  - cloud/dev-gateway/.gitignore
  - .governance/active_scope.json
  - .governance/issues/PI-GW-DEV-01.md
forbidden_files:
  - ADU/ADP/CLAUDE/客户端 APC/云 APC
  - src-tauri/** / src/**              # 客户端接线（PI-AG-10B/11）归后续，不在本 PI
confirmation: YES
```

## Expected Behavior

- `GET /health` → `{ ok, providers }`。
- `POST /gate/check` → `{ verdict: "passed", dev: true }`（dev 恒通过；真实裁决归服务端 GW+AUTH+LIC，X-FORBID-002，正式版补）。
- `POST /invoke/stream` body `{ provider, model, messages, api_key?, max_tokens? }` → 归一化 SSE：
  `event: chunk {text}` / `event: done {stop_reason}` / `event: error {code,message,retriable}`（对齐 A-GW-M-D-01 §3.4）。
- 三供应商适配（REQ-GW-001）：`openai`/`deepseek` 走 OpenAI 兼容（Bearer + chat/completions SSE，共享适配）；
  `anthropic` 走 Messages API（x-api-key + anthropic-version + content_block_delta 解析）。
- 密钥来源：BYOK（`body.api_key`）优先，否则平台 env `GW_<PROVIDER>_KEY`。**密钥不落盘、不写日志**（REQ-GW-006 dev 落实）。

## Validation Commands

```
node --check cloud/dev-gateway/server.mjs     # 语法
node cloud/dev-gateway/server.mjs &            # 启动
curl -s localhost:8787/health                  # {ok:true,...}
curl -s -XPOST localhost:8787/gate/check       # {verdict:"passed",dev:true}
# /invoke/stream 需真实 provider key：curl -XPOST .../invoke/stream -d '{"provider":"deepseek","model":"deepseek-chat","messages":[...],"api_key":"sk-..."}'
```

验收：node --check 通过；server 启动监听；/health 与 /gate/check 返回预期 JSON；/invoke/stream 在提供
真实 key 时流式回 chunk→done（人类用自有 key 自验）；server.mjs 无密钥硬编码、无 @GOV/注册块。

## 后续（解桩端到端）

1. 客户端 PI-AG-10B（ag_start_turn → Channel<AgTurnEvent>）+ PI-AG-11（route_start_turn 桩换真实 HTTP SSE → localhost dev GW → 翻译为 AgTurnEvent）。
2. BYOK 入口：客户端临时输入框 → `ent::byok::upload_byok` 真实 deliver 到 dev GW（替换 stub）。
3. 正式化：迁独立仓库 + TS + GW-02 加密 + AUTH/LIC/真实门禁（EXEMPTION-GW-DEV-01 解除）。
