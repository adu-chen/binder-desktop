# Issue Trace · PI-GW-DEV-05 云端真实化④ LIC 订阅/额度

承接 PI-GW-DEV-03/04（门禁令牌真 + 计量就绪 → 订阅那关真化）/ A-GW-M-D-01 REQ-GW-005（CLOUD-GATE：
订阅裁决）/ REQ-CLOUD-005（额度）。补全 CLOUD-GATE 最后一桩：认证 + **订阅 + 额度**。

## Scope Lock

```
task_type: CODE_CHANGE
scope_id: PI-GW-DEV-05
allowed_files:
  - cloud/dev-gateway/{store,usage,lic,gate,server}.mjs + README.md
  - .governance/active_scope.json / issues/PI-GW-DEV-05.md
forbidden_files: ADU/ADP/CLAUDE/客户端 APC/云 APC / ipc/mod.rs / lib.rs（客户端不动）
confirmation: YES
```

## Expected Behavior

- `store.readLines(name)`：读 JSONL → 解析行数组。`usage.countToday(userId)`：当日该用户调用计数（用计量）。
- `lic.evaluate(userId)` → `{verdict, reason?, used?, limit?}`：订阅（dev free 恒有效）+ 额度（`used = countToday`，
  `>= DAILY_QUOTA` → denied `quota_exceeded`；`DAILY_QUOTA` 默认 50，env `GW_DAILY_QUOTA` 可覆盖）。
- `gate.checkGate`：验令牌 → `lic.evaluate` → 合并 verdict（denied 带 reason/used/limit）。
- `server` 门禁拒绝按类返码：**令牌问题（reauth_required）→ 401**（客户端清令牌重登录）；**额度/订阅 → 429**
  （客户端不重登录，emit 错误）。计量在 gate 过后记（额度判当日「本次之前」）。

## Validation Commands

```
node --check cloud/dev-gateway/*.mjs
# smoke：GW_DAILY_QUOTA=0 + 有效令牌 → /gate/check = denied quota_exceeded；默认配额 → passed(used<limit)
```

验收：node --check；额度=0 时 gate denied quota_exceeded、默认配额 passed；令牌问题→401、额度/订阅→429（分流正确）。

## 后续

- 真实订阅状态/档位/计费（替 dev free 恒有效）；额度窗口/重置策略；用量入 DB；正式 GW-02；迁独立仓。
```
