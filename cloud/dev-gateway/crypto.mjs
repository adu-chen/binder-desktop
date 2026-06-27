// 加密（PI-GW-DEV-04，dev GW-02-lite）：AES-256-GCM，本地自管密钥（store.loadOrCreateKey）。
// BYOK 密钥落盘前加密——磁盘无明文。正式版 GW-02：KMS/信封加密（安全敏感，须人类批准），迁独立仓。

import { randomBytes, createCipheriv, createDecipheriv } from 'node:crypto';
import { loadOrCreateKey } from './store.mjs';

const KEY = loadOrCreateKey();

/** 明文 → base64(iv|tag|ciphertext)。 */
export function encrypt(plain) {
  const iv = randomBytes(12);
  const cipher = createCipheriv('aes-256-gcm', KEY, iv);
  const enc = Buffer.concat([cipher.update(String(plain), 'utf8'), cipher.final()]);
  const tag = cipher.getAuthTag();
  return Buffer.concat([iv, tag, enc]).toString('base64');
}

/** base64(iv|tag|ciphertext) → 明文；失败抛错（不静默）。 */
export function decrypt(b64) {
  const buf = Buffer.from(b64, 'base64');
  const iv = buf.subarray(0, 12);
  const tag = buf.subarray(12, 28);
  const enc = buf.subarray(28);
  const decipher = createDecipheriv('aes-256-gcm', KEY, iv);
  decipher.setAuthTag(tag);
  return Buffer.concat([decipher.update(enc), decipher.final()]).toString('utf8');
}
