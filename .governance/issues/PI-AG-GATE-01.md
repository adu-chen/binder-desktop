# Issue Trace · PI-AG-GATE-01 客户端登录接线 + 门禁端到端强制（云端真实化②）

承接 PI-GW-DEV-03（真门禁就绪）/ A-SYS-I-P-03（X-SEQ-001 门禁前置、X-FORBID-002 服务端裁决不缓存通过）/
A-ENT-M-T-01（调用通道）。让门禁真正端到端强制：客户端登录拿令牌 → 随调用上送 → GW 必裁决。

## Scope Lock

```
task_type: CODE_CHANGE
scope_id: PI-AG-GATE-01
allowed_files:
  - src-tauri/src/ent/gateway.rs   # 会话令牌缓存 + dev auto-login + stream_round 上送 token + 401 清缓存
  - cloud/dev-gateway/server.mjs    # /invoke/stream 去 dev-anonymous，必经 checkGate
  - .governance/active_scope.json / issues/PI-AG-GATE-01.md
forbidden_files: ADU/ADP/CLAUDE/客户端 APC/云 APC / ipc/mod.rs / lib.rs
confirmation: YES
```

## Expected Behavior

- 客户端 `ent::gateway`：`ensure_session_token()` 懒登录（POST /auth/login，dev email）+ 缓存（static Mutex）；
  `stream_round` 取 token 随 `/invoke/stream` body 上送；收 401（gate_denied）→ `clear_session_token`（下轮重登录）
  + 回 Error（retriable，重登录）。**令牌是 auth 凭据**（非门禁裁决缓存）——每次调用仍经服务端 checkGate（X-FORBID-002）。
- GW `/invoke/stream`：**去 dev-anonymous**，无条件 `checkGate(token)`，deny → 401（X-SEQ-001 门禁前置强制）。
- 令牌过期（24h）→ GW 401 → 客户端清缓存 → 下轮自动重登录。

## Validation Commands

```
cargo build --lib
node --check cloud/dev-gateway/server.mjs
# smoke：无 token invoke → 401（已不再 anonymous 放行）；
# 端到端（需真实 key）：客户端自动登录 → 带 token → 门禁过 → 流式 AI（人类自验）
```

验收：cargo build + node --check；GW tokenless invoke → 401（强制）；客户端 auto-login + 上送 token；401 清缓存重登录。

## 后续

- 正式 AUTH（真实凭据/JWT 签名/持久化）；LIC 真实订阅判定（gate 第二关）；登录失败/重认证的前端提示。
```
