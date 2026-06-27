// 持久化存储（PI-GW-DEV-06，云端真实化⑤）：SQLite（node:sqlite DatabaseSync）替原 .data/ JSON 文件。
// 零依赖——node:sqlite 为 Node 内置（实验特性，dev 可用）。一个连接全模块共享；建表幂等。
// 安全：库内**只存密文信封**（byok_keys.envelope），主密钥(KEK)在库外（kms.mjs，gw.master.key/env）。
// 正式版：迁独立云仓 + 托管 DB（Postgres 等）+ 迁移脚本。.data/ 已 gitignore（gw.db/-wal/-shm 不入仓）。

import { mkdirSync, existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

export const DATA_DIR = join(dirname(fileURLToPath(import.meta.url)), '.data');
if (!existsSync(DATA_DIR)) mkdirSync(DATA_DIR, { recursive: true });

// node:sqlite 是实验特性，构造时发 ExperimentalWarning——dev 脚手架仅静音这一条，其余告警照常。
const origEmitWarning = process.emitWarning.bind(process);
process.emitWarning = (warning, ...rest) => {
  const msg = typeof warning === 'string' ? warning : warning?.message;
  if (typeof msg === 'string' && msg.includes('SQLite is an experimental')) return;
  return origEmitWarning(warning, ...rest);
};

const { DatabaseSync } = await import('node:sqlite');
process.emitWarning = origEmitWarning; // 还原，不长期改变默认告警行为

export const db = new DatabaseSync(join(DATA_DIR, 'gw.db'));
db.exec('PRAGMA journal_mode = WAL;');
db.exec(`
  CREATE TABLE IF NOT EXISTS sessions (
    token   TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    exp     INTEGER NOT NULL
  );
  CREATE TABLE IF NOT EXISTS byok_keys (
    provider   TEXT PRIMARY KEY,
    envelope   TEXT NOT NULL,   -- 信封密文（kms.mjs 加密）；绝不存明文
    updated_at INTEGER NOT NULL
  );
  CREATE TABLE IF NOT EXISTS usage (
    id       INTEGER PRIMARY KEY AUTOINCREMENT,
    ts       TEXT NOT NULL,
    user_id  TEXT NOT NULL,
    provider TEXT,
    model    TEXT              -- 脱敏元数据；绝不含正文（REQ-GW-006）
  );
  CREATE INDEX IF NOT EXISTS idx_usage_user_ts ON usage(user_id, ts);
`);
