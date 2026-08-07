// BYOK 模块（PI-GW-DEV-03/04/06）：密钥代持，**信封加密持久化入库**（库内无明文）。
// 存储：SQLite byok_keys（db.mjs）；加密：信封 v2（crypto.mjs → kms.mjs，DEK 由 KEK 包裹）。
// 启动解封载入内存（密文→明文，仅内存）→ 重启不丢 key。正式版：会话/用户维度 + 删除/轮换 + 托管 KMS，迁独立仓。
// 不回显、不写日志。

import { db } from './db.mjs';
import { encrypt, decrypt } from './crypto.mjs';

const byokKeys = new Map(); // provider -> 明文 key（仅内存）

const selAll = db.prepare('SELECT provider, envelope FROM byok_keys');
const upsert = db.prepare(
  `INSERT INTO byok_keys(provider, envelope, updated_at) VALUES(?, ?, ?)
   ON CONFLICT(provider) DO UPDATE SET envelope = excluded.envelope, updated_at = excluded.updated_at`,
);

// 启动：解封载入持久化密钥（信封密文 → 明文进内存）。
for (const row of selAll.all()) {
  try {
    byokKeys.set(row.provider, decrypt(row.envelope));
  } catch {
    /* 信封损坏 / KEK 变更 → 跳过该项（不静默吞掉真实逻辑，仅容错单条） */
  }
}

export function setByokKey(provider, key) {
  byokKeys.set(provider, key);
  upsert.run(provider, encrypt(key), Date.now()); // 落库前信封加密
}

export function getByokKey(provider) {
  return byokKeys.get(provider);
}

const del = db.prepare('DELETE FROM byok_keys WHERE provider = ?');

/** 脱敏 preview（不回显明文，PI-GW-02）。 */
function previewKey(key) {
  if (typeof key !== 'string' || key.length <= 8) return '***';
  return `${key.slice(0, 3)}***${key.slice(-4)}`;
}

/** 已配置 BYOK provider 列表（脱敏 preview）。 */
export function listByokProviders() {
  const out = [];
  for (const [provider, key] of byokKeys) {
    out.push({ provider, key_preview: previewKey(key) });
  }
  return out;
}

/** 删除指定 provider 的 BYOK 密钥，立即生效（A-GW-M-D-01 PI-GW-02）。 */
export function deleteByokKey(provider) {
  byokKeys.delete(provider);
  del.run(provider);
}
