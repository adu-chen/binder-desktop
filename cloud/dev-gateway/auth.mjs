// AUTH 模块（PI-GW-DEV-03/06，dev 最小实现）：签发/校验会话令牌，持久化入 SQLite（db.mjs sessions 表）。
// 重启不丢登录。正式版：用户库 + 真实凭据（magic link/密码）+ JWT 签名（CLOUD-01 决策后），迁独立仓。

import { randomBytes } from 'node:crypto';
import { db } from './db.mjs';

const TTL_MS = 24 * 3600 * 1000;

const insSession = db.prepare('INSERT INTO sessions(token, user_id, exp) VALUES(?, ?, ?)');
const selSession = db.prepare('SELECT user_id, exp FROM sessions WHERE token = ?');
const delSession = db.prepare('DELETE FROM sessions WHERE token = ?');
const purgeExpired = db.prepare('DELETE FROM sessions WHERE exp < ?');

// 启动清理过期会话。
purgeExpired.run(Date.now());

/** 登录（dev：任意 email 即发令牌；无 email 为 dev 用户）。回 { token, userId, expiresAt }。 */
export function login(email) {
  const userId = email && String(email).trim() ? `u_${String(email).trim()}` : 'u_dev';
  const token = randomBytes(24).toString('hex');
  const exp = Date.now() + TTL_MS;
  insSession.run(token, userId, exp);
  return { token, userId, expiresAt: exp };
}

/** 校验令牌：有效且未过期 → { userId }；否则 null（过期即清理）。 */
export function verifyToken(token) {
  if (!token) return null;
  const row = selSession.get(token);
  if (!row) return null;
  if (Date.now() > row.exp) {
    delSession.run(token);
    return null;
  }
  return { userId: row.user_id };
}

export function logout(token) {
  if (token) delSession.run(token);
}
