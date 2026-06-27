# Issue Trace · PI-BYOK-DEV-01 BYOK 客户端入口（零残留）

承接 A-ENT-M-T-01 BR-ENT-DATA-001（BYOK 一次性上送、零残留、不豁免门禁）/ X-FORBID-001（客户端不持密钥）/
A-GW-M-D-01 REQ-GW-004（BYOK 代持）。让用户在界面填自己的 provider 密钥（替 env），key 只经网关、客户端零残留。

## Scope Lock

```
task_type: CODE_CHANGE
scope_id: PI-BYOK-DEV-01
allowed_files:
  - cloud/dev-gateway/server.mjs       # POST /byok/keys（内存存）+ /invoke/stream 取密钥优先级 req>byok>env
  - cloud/dev-gateway/README.md
  - src-tauri/src/ent/byok.rs           # upload_to_gateway（async 真实上送 + 零残留清零）
  - src-tauri/src/ent/gateway.rs         # gw_base_url 公开(crate)
  - src-tauri/src/lib.rs                 # byok_upload async 命令 + 注册
  - src/services/agService.ts            # uploadByok
  - src/components/ag/Composer.tsx        # 🔑 key 录入（provider+密码框→保存即送、清空）
  - src/styles.css                        # byok-form 样式
  - .governance/active_scope.json / issues/PI-BYOK-DEV-01.md
forbidden_files: ADU/ADP/CLAUDE/APC/云 APC / ipc/mod.rs / edStore
confirmation: YES
```

## Expected Behavior

- GW `POST /byok/keys {provider,api_key}` → 内存存 `byokKeys[provider]`，回 `{ok,provider}`（不回显密钥）；
  `/invoke/stream` 取密钥优先级：`body.api_key > byokKeys[provider] > env GW_<P>_KEY`。
- 后端 `ent::byok::upload_to_gateway(provider, key)`：async POST 送 GW，**上送后就地 zeroize 原 key**（零残留，
  不回显、不写日志，BR-ENT-DATA-001）；不可达/HTTP 错回统一错误。`byok_upload` 命令注册 generate_handler。
- 前端：Composer「🔑 key」→ provider 选择 + 密码框，保存 → `uploadByok` → **清空输入**（客户端不保留）+ 状态回显（不含密钥）。
- X-FORBID-001：客户端不持久化密钥（仅瞬时输入→送 GW→清空）；dev 门禁桩=Pass（真实裁决归服务端正式版）。

## Validation Commands

```
cargo build --lib && cargo test --lib 'ent::byok'   # 3/3
node --check cloud/dev-gateway/server.mjs && npx tsc --noEmit
# GW /byok/keys 已实测：存 key 回 {ok,provider}（不回显）；非法回错。
# 端到端（app）：🔑 填 deepseek key → 保存 → 后续对话用该 key（人类自验）
```

验收：cargo build + ent::byok 3/3；GW /byok/keys 存/取优先级正确、不回显；tsc 干净；客户端不持久化密钥。

## 后续

- 正式 GW：BYOK 加密存（GW-02）+ 真实 AUTH/会话维度密钥 + 门禁裁决；客户端 zeroize crate 最小化拷贝；
  已配 BYOK 的状态展示（脱敏 preview）/ 删除入口。
