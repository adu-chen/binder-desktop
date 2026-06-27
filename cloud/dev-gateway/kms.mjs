// KMS 抽象 + 信封加密（PI-GW-DEV-06，正式 GW-02）：替原「单密钥直接加密」(GW-02-lite)。
//
// 信封加密（KMS 同形）：
//   · KEK（主密钥）只用来 wrap/unwrap 每条记录的 DEK，从不直接加密业务数据。
//   · 每条密文用一把**独立随机 DEK**（数据密钥）加密；DEK 被 KEK 包裹后随密文一起存。
//   · 解密时先用 KEK 解出 DEK，再用 DEK 解密；DEK 用完零化。
// 这正是云 KMS（AWS KMS GenerateDataKey/Decrypt 等）形式化的模式——本地 LocalKms 与真实云 KMS
// 实现同一 { generateDataKey, unwrapDataKey } 接口，正式仓换 provider 即可，KEK 永不离开 KMS。
//
// dev：KEK 取 env GW_MASTER_KEY（优先）或本地 gw.master.key（懒生成 0600）。KEK 与密文分库存放
//      （密文在 gw.db，KEK 在库外文件/env），泄露其一不足以解密。正式版 KEK 入托管 KMS，须人类批准。

import { randomBytes, createCipheriv, createDecipheriv, createHash } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { DATA_DIR } from './db.mjs';

/** 载入/派生 32B KEK + 标识。env 优先（hex64 / base64 / 任意串派生）；否则本地 keyfile 懒生成。 */
function loadKek() {
  const env = process.env.GW_MASTER_KEY;
  if (env && env.trim()) {
    const s = env.trim();
    if (/^[0-9a-fA-F]{64}$/.test(s)) return { kek: Buffer.from(s, 'hex'), keyId: 'env:GW_MASTER_KEY' };
    const b64 = Buffer.from(s, 'base64');
    if (b64.length === 32) return { kek: b64, keyId: 'env:GW_MASTER_KEY' };
    // 非 32B 原料 → SHA-256 派生（dev 容错；正式版要求 32B 随机）。
    return { kek: createHash('sha256').update(s).digest(), keyId: 'env:GW_MASTER_KEY(derived)' };
  }
  const p = join(DATA_DIR, 'gw.master.key');
  try {
    const b = Buffer.from(readFileSync(p, 'utf8').trim(), 'hex');
    if (b.length === 32) return { kek: b, keyId: 'local:gw.master.key' };
  } catch {
    /* 不存在 → 生成 */
  }
  const kek = randomBytes(32);
  writeFileSync(p, kek.toString('hex'), { mode: 0o600 });
  return { kek, keyId: 'local:gw.master.key' };
}

/** 本地 KMS 提供方：KEK 在进程内，用 AES-256-GCM wrap/unwrap DEK。接口与云 KMS 同形。 */
class LocalKms {
  #kek;
  constructor() {
    const { kek, keyId } = loadKek();
    this.#kek = kek;
    this.keyId = keyId;
    this.provider = 'local';
  }

  /** 生成新数据密钥：{ dek: Buffer(32 明文), wrappedDek: base64(iv|tag|ct) }。 */
  generateDataKey() {
    const dek = randomBytes(32);
    const iv = randomBytes(12);
    const c = createCipheriv('aes-256-gcm', this.#kek, iv);
    const ct = Buffer.concat([c.update(dek), c.final()]);
    const tag = c.getAuthTag();
    return { dek, wrappedDek: Buffer.concat([iv, tag, ct]).toString('base64') };
  }

  /** 解包数据密钥：base64(iv|tag|ct) → Buffer(32 明文 DEK)。失败抛错（不静默）。 */
  unwrapDataKey(wrappedDek) {
    const buf = Buffer.from(wrappedDek, 'base64');
    const iv = buf.subarray(0, 12);
    const tag = buf.subarray(12, 28);
    const ct = buf.subarray(28);
    const d = createDecipheriv('aes-256-gcm', this.#kek, iv);
    d.setAuthTag(tag);
    return Buffer.concat([d.update(ct), d.final()]);
  }
}

/** 选 KMS 提供方。dev 默认 local；aws/gcp 在正式云仓实现同一接口（此处不造桩冒充完成）。 */
function makeKms() {
  const provider = (process.env.GW_KMS_PROVIDER || 'local').toLowerCase();
  if (provider === 'local') return new LocalKms();
  throw new Error(
    `GW_KMS_PROVIDER=${provider} 未在 dev GW 实现——正式 KMS(aws/gcp) 在独立云仓按同一 generateDataKey/unwrapDataKey 接口接入`,
  );
}

export const kms = makeKms();
