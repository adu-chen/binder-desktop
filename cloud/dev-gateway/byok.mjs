// BYOK 模块（PI-GW-DEV-03/04）：密钥代持，**加密持久化**（磁盘无明文）。
// dev：AES-256-GCM（crypto.mjs，本地自管密钥）存 .data/byok.json；启动解密载入 → 重启不丢 key。
// 正式版 GW-02：KMS/信封加密 + 会话/用户维度 + 删除/轮换（须人类批准方案），迁独立仓。不回显、不写日志。

import { loadJson, saveJson } from './store.mjs';
import { encrypt, decrypt } from './crypto.mjs';

const byokKeys = new Map(); // provider -> 明文 key（仅内存）

// 启动：解密载入持久化密钥（密文 → 明文进内存）。
{
  const stored = loadJson('byok.json', {}); // { provider: ciphertext }
  for (const [provider, blob] of Object.entries(stored)) {
    try {
      byokKeys.set(provider, decrypt(blob));
    } catch {
      /* 密文损坏/换密钥 → 跳过该项 */
    }
  }
}

function persist() {
  const enc = {};
  for (const [provider, key] of byokKeys) enc[provider] = encrypt(key); // 落盘前加密
  saveJson('byok.json', enc);
}

export function setByokKey(provider, key) {
  byokKeys.set(provider, key);
  persist();
}

export function getByokKey(provider) {
  return byokKeys.get(provider);
}
