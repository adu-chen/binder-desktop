// 持久化存储（PI-GW-DEV-04，dev）：.data/ 下 JSON 文件 + 本地密钥懒生成。
// 正式版：DB（持久化）+ 密钥经 KMS/信封加密，迁独立仓。.data/ 已 gitignore（密钥/会话/用量/密文不入仓）。

import { readFileSync, writeFileSync, appendFileSync, mkdirSync, existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import { randomBytes } from 'node:crypto';

const DATA_DIR = join(dirname(fileURLToPath(import.meta.url)), '.data');

function ensureDir() {
  if (!existsSync(DATA_DIR)) mkdirSync(DATA_DIR, { recursive: true });
}

export function loadJson(name, fallback) {
  try {
    return JSON.parse(readFileSync(join(DATA_DIR, name), 'utf8'));
  } catch {
    return fallback;
  }
}

export function saveJson(name, obj) {
  ensureDir();
  writeFileSync(join(DATA_DIR, name), JSON.stringify(obj));
}

export function appendLine(name, obj) {
  ensureDir();
  appendFileSync(join(DATA_DIR, name), JSON.stringify(obj) + '\n');
}

/** 读 JSONL → 解析行数组（坏行跳过）。文件不存在 → []。 */
export function readLines(name) {
  try {
    return readFileSync(join(DATA_DIR, name), 'utf8')
      .split('\n')
      .filter(Boolean)
      .map((l) => {
        try {
          return JSON.parse(l);
        } catch {
          return null;
        }
      })
      .filter(Boolean);
  } catch {
    return [];
  }
}

/** 本地加密密钥（懒生成 32B，存 .data/gw.key）。dev GW-02-lite；正式版经 KMS/信封，须人类批准。 */
export function loadOrCreateKey() {
  const p = join(DATA_DIR, 'gw.key');
  try {
    const b = Buffer.from(readFileSync(p, 'utf8'), 'hex');
    if (b.length === 32) return b;
  } catch {
    /* 不存在 → 生成 */
  }
  ensureDir();
  const key = randomBytes(32);
  writeFileSync(p, key.toString('hex'), { mode: 0o600 });
  return key;
}
