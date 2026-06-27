// 加密（PI-GW-DEV-06，正式 GW-02 信封加密）：每条密文一把独立 DEK，DEK 由 KMS 包裹（kms.mjs）。
// 替原 GW-02-lite（单密钥直接加密）。保留 encrypt/decrypt 名，上层（byok.mjs）零改动。
// 输出为自描述信封 JSON 串：{ v, kid, wdek(包裹的DEK), iv, tag, ct }。

import { randomBytes, createCipheriv, createDecipheriv } from 'node:crypto';
import { kms } from './kms.mjs';

const VERSION = 'v2-envelope';

/** 明文 → 信封 JSON 串（含包裹的 DEK + AES-256-GCM 密文）。 */
export function encrypt(plain) {
  const { dek, wrappedDek } = kms.generateDataKey();
  try {
    const iv = randomBytes(12);
    const cipher = createCipheriv('aes-256-gcm', dek, iv);
    const ct = Buffer.concat([cipher.update(String(plain), 'utf8'), cipher.final()]);
    const tag = cipher.getAuthTag();
    return JSON.stringify({
      v: VERSION,
      kid: kms.keyId,
      wdek: wrappedDek,
      iv: iv.toString('base64'),
      tag: tag.toString('base64'),
      ct: ct.toString('base64'),
    });
  } finally {
    dek.fill(0); // 零化明文 DEK
  }
}

/** 信封 JSON 串 → 明文；版本不符或验证失败抛错（不静默）。 */
export function decrypt(envelope) {
  const e = JSON.parse(envelope);
  if (e.v !== VERSION) throw new Error(`unknown envelope version: ${e.v}`);
  const dek = kms.unwrapDataKey(e.wdek);
  try {
    const decipher = createDecipheriv('aes-256-gcm', dek, Buffer.from(e.iv, 'base64'));
    decipher.setAuthTag(Buffer.from(e.tag, 'base64'));
    return Buffer.concat([decipher.update(Buffer.from(e.ct, 'base64')), decipher.final()]).toString('utf8');
  } finally {
    dek.fill(0);
  }
}
