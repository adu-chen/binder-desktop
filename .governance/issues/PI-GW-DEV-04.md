# Issue Trace · PI-GW-DEV-04 云端真实化③ 持久化 + 用量计量

承接 PI-GW-DEV-03（内存态 → 持久化）/ A-GW-M-D-01 REQ-GW-003（用量计量）/ REQ-GW-006（内容不留存）/
REQ-GW-004 + GW-02（BYOK 加密）。会话/BYOK 重启不丢、每次调用记脱敏用量；BYOK 加密存（非明文）。

## Scope Lock

```
task_type: CODE_CHANGE
scope_id: PI-GW-DEV-04
allowed_files:
  - cloud/dev-gateway/{store,crypto,usage}.mjs（新）+ {auth,byok,server}.mjs + .gitignore + README.md
  - .governance/active_scope.json / issues/PI-GW-DEV-04.md
forbidden_files: ADU/ADP/CLAUDE/客户端 APC/云 APC / ipc/mod.rs / lib.rs
confirmation: YES
```

## Expected Behavior

- `store.mjs`：`.data/` 下 JSON 文件读写（loadJson/saveJson/appendLine）+ 本地密钥懒生成（.data/gw.key，32B）。
- `crypto.mjs`：AES-256-GCM 加/解密（dev **GW-02-lite**：本地自管密钥；正式版 KMS/信封加密，须人类批准）。
- `usage.mjs`：`recordUsage({userId,provider,model})` → 追加 `.data/usage.jsonl`——**脱敏、无正文**（REQ-GW-006）。
- `auth`：会话持久化（启动 load + 过期清理；login/logout 落盘）→ **重启不丢登录**（客户端缓存令牌仍有效）。
- `byok`：密钥**加密**持久化（crypto 加密存 .data/byok.json；启动解密载入）→ **重启不丢 key 但磁盘无明文**（守安全）。
- `server` `/invoke/stream`：过门禁后 `recordUsage`（userId+provider+model，**不含正文**）。
- `.gitignore`：补 `.data/`（密钥/会话/用量/BYOK 密文不入仓）。

## Validation Commands

```
node --check cloud/dev-gateway/*.mjs
# smoke：login→重启 GW→同令牌 gate 仍 passed（会话持久化）；byok 存→重启→仍可用（加密持久化）；usage.jsonl 有记录、无正文
```

验收：node --check 全过；会话/BYOK 重启不丢；磁盘 byok.json 为密文（非明文）；usage.jsonl 脱敏无正文；.data/ gitignore。

## 后续

- LIC 真实订阅/额度（用量 → 额度判定）；正式 GW-02（KMS/信封）；DB 替 JSON 文件；迁独立仓。
```
