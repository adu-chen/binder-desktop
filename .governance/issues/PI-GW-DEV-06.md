# PI-GW-DEV-06 — 云端真实化⑤：正式 GW-02 信封加密 + SQLite DB 替 JSON

- 日期：2026-06-27
- 类型：CODE_CHANGE（cloud-only，dev GW）
- 范围闸：`EXEMPTION-GW-DEV-01`（人类 2026-06-26 授权，云端真实化在本仓 `cloud/` 隔离迭代）
- 治理可见性：`cloud/` 对客户端治理不可见（0 ADUS 引用、0 治理块），不参与 generate/audit
- 触发：人类指令「正式 GW-02（KMS/信封）、DB 替 JSON —— 把这两项推进完成」（接 STAGE-REPORT §7 未竟项）

## 背景

阶段报告把两项列为「决策/基建门控」的未竟项：
- 密钥加密仅 GW-02-lite（单密钥直接 AES-256-GCM）。
- 持久化用 `.data/*.json` 文件，非数据库。

本 PI 在 dev GW 内把这两项推进到**可跑的真实形态**（dev 等价物 + 与正式版同形的接口），正式版仍迁独立云仓换托管 KMS/DB。

## 改动

### ① 正式 GW-02 信封加密（KMS 同形）
- 新增 `kms.mjs`：KMS 抽象 + `LocalKms`。
  - `generateDataKey()` → `{ dek(32B 明文), wrappedDek }`：随机 DEK，KEK 用 AES-256-GCM 包裹 DEK。
  - `unwrapDataKey(wrappedDek)` → DEK：KEK 解包。
  - KEK 取 `env GW_MASTER_KEY`（优先，hex64/base64/派生）或本地 `gw.master.key`（懒生成 0600），**与密文分库存放**。
  - `makeKms()`：`GW_KMS_PROVIDER` 默认 `local`；`aws/gcp` 抛「未在 dev 实现」（不造桩冒充完成），正式仓按同一接口接入。
- 改 `crypto.mjs`：`encrypt/decrypt`（保留旧名，上层零改）→ 信封 v2。每条密文一把独立 DEK；输出自描述信封 JSON `{v,kid,wdek,iv,tag,ct}`；DEK 用后零化。

### ② SQLite DB 替 JSON
- 新增 `db.mjs`：`node:sqlite` `DatabaseSync`（Node 内置，零依赖）。WAL 模式。建表幂等：
  - `sessions(token PK, user_id, exp)`
  - `byok_keys(provider PK, envelope, updated_at)` —— 只存信封密文
  - `usage(id PK, ts, user_id, provider, model)` + 索引 `(user_id, ts)` —— 脱敏，无正文
  - 静音 node:sqlite 一次性 ExperimentalWarning（仅这一条），随即还原默认告警行为。
- 改 `auth.mjs`：会话改 SQLite（login INSERT / verifyToken SELECT+过期删 / logout DELETE / 启动 purge 过期）。
- 改 `byok.mjs`：BYOK 改 SQLite UPSERT，落库前信封加密；启动 SELECT 解封进内存 Map。
- 改 `usage.mjs`：用量改 SQLite（recordUsage INSERT / countToday COUNT WHERE substr(ts,1,10)）。
- 删 `store.mjs`（原 JSON 文件 + 本地单密钥 loadOrCreateKey，已无引用）。

### 文档
- `server.mjs` 头注 + 启动日志：补 db/kms 模块，澄清 BYOK 信封加密入库。
- `README.md`：新增「持久化与加密（PI-GW-DEV-06）」节；修订 BYOK「内存存」旧述、密钥与 gitignore 说明。
- `STAGE-REPORT-AG-Cloud-2026-06-27.md` §7：两项由未竟改为已推进（PI-GW-DEV-06），保留「迁独立仓/托管 KMS·DB」为正式版项。

## 验证（实跑）

- `node --check` 全模块通过（含新 db/kms，删 store 后无悬空引用）。
- 信封单测：round-trip 正确；两次 encrypt 同明文产出不同信封（每条独立 DEK+iv）；GCM 篡改密文被拒（抛错，不静默）。
- DB 单测：`recordUsage` ×2 后 `countToday` 增量 == 2。
- 端到端（fresh `.data`）：login→token；gate/check `passed`(used:0,limit:50)；存 BYOK 探针 key。
- **加密落盘**：探针明文在 `.data/` 所有文件（`gw.db`/`-wal`/`-shm`/`gw.master.key`）命中数 = 0；信封标记 `v2-envelope` present。
- **KEK 分离**：`gw.master.key`（0600）与 `gw.db` 分文件。
- **持久化**：重启后同一令牌仍 `passed`；BYOK 重启后解封回原明文（getByokKey == 探针）。

## 未竟（仍为正式版，决策/基建门控）

- 迁独立云仓 + TypeScript 重写。
- KEK 换托管 KMS（KEK 不出 KMS）；DB 换托管数据库（Postgres 等）+ 迁移脚本。
- 真实订阅态/计费、内容不留存审计合规。
- 安全提醒续办：会话中暴露过的 DeepSeek key 仍须轮换。
