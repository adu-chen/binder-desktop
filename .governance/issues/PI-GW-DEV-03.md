# Issue Trace · PI-GW-DEV-03 云端真实化① 模块骨架 + 最小 AUTH + 真门禁

承接 A-GW-M-D-01 / A-SYS-I-P-03（CLOUD-AUTH/GATE、X-SEQ-001 门禁前置、X-FORBID-002 服务端裁决）。
在本仓 `cloud/dev-gateway/` 隔离迭代云端真实化第①步：拆模块 + 最小 AUTH + 真门禁（替桩恒过）。

## ⚠️ 豁免留痕（CLAUDE.md §9）

- **EXEMPTION-GW-DEV-01 扩展（人类 2026-06-27 授权）**：原 dev GW 驻本仓豁免，扩为「云端真实化在本仓
  `cloud/` 下隔离迭代」。理由：现阶段细节频繁调整，独立云仓库不便迭代。**隔离保证（已验）**：`cloud/` 对
  客户端治理隐形——ADUS 引用 0、`cloud/` 内注册类治理块/@GOV 0、`governance:generate/audit` 不受影响。
  规矩：只在 `cloud/`、不放注册类治理块/@GOV、密钥/.env gitignore、提交只显式 add 自有文件。
  **临时**：成熟后按云 APC §3 迁出独立仓，豁免解除。正式版仍迁仓 + TS + GW-02 加密 + 真实持久化 AUTH/LIC。

## Scope Lock

```
task_type: CODE_CHANGE
scope_id: PI-GW-DEV-03
allowed_files:
  - cloud/dev-gateway/{server,providers,auth,gate,lic,byok}.mjs + README.md
  - .governance/active_scope.json / issues/PI-GW-DEV-03.md
forbidden_files: ADU/ADP/CLAUDE/客户端 APC/云 APC / ipc/mod.rs / lib.rs / Composer.tsx（客户端不动）
confirmation: YES
```

## Expected Behavior

- **模块拆分**：`providers.mjs`（适配/流式/SSE）、`auth.mjs`（内存会话 + login/verifyToken/logout）、
  `gate.mjs`（checkGate=验令牌+订阅）、`lic.mjs`（isSubscribed dev 占位）、`byok.mjs`（密钥存）；`server.mjs` HTTP+路由。
- **AUTH**：`POST /auth/login {email}` → 签发随机会话令牌（内存，TTL 24h）+ userId；`verifyToken` 校验有效/未过期。
- **真门禁**：`POST /gate/check {token}` → 验令牌 + lic.isSubscribed → `{verdict:passed|denied,reason,reauth_required}`
  （替桩恒过；无令牌/过期→denied，X-FORBID-002 服务端裁决）。
- **/invoke/stream**：带 `token` 则真门禁（deny→401，X-SEQ-001 门禁前置）；**无 token → dev-anonymous 放行**
  （不破坏现客户端，令牌强制留作后续客户端登录接线）。密钥优先级 req>BYOK>env 不变。

## Validation Commands

```
node --check cloud/dev-gateway/*.mjs
# smoke：/health；/auth/login→token；/gate/check 带 token=passed、无/错 token=denied
# 端到端（需真实 key）：deepseek 流式仍通（dev-anonymous）
```

验收：node --check 全过；/auth/login 签发令牌；/gate/check 真查（有效令牌 passed、无/过期 denied）；
/invoke/stream tokenless 仍通（dev-anonymous）、带错 token 拒；模块拆分后 deepseek 路径不变。

## 后续

- 客户端登录接线（auto dev-login → 令牌随 invoke 上送 → 门禁强制，去 dev-anonymous）；LIC 真实订阅/额度；
  GW-02 BYOK 加密；持久化（DB）；迁独立仓。
