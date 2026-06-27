// AUTH 模块（PI-GW-DEV-03，dev 最小实现）：内存会话 + 签发/校验会话令牌。
// 正式版：持久化用户库 + 真实凭据（magic link/密码）+ JWT 签名（CLOUD-01 决策后），迁独立仓。

import { randomBytes } from 'node:crypto';

const TTL_MS = 24 * 3600 * 1000;
const sessions = new Map(); // token -> { userId, exp }

/** 登录（dev：任意 email 即发令牌；无 email 为 dev 用户）。回 { token, userId, expiresAt }。 */
export function login(email) {
  const userId = email && String(email).trim() ? `u_${String(email).trim()}` : 'u_dev';
  const token = randomBytes(24).toString('hex');
  const exp = Date.now() + TTL_MS;
  sessions.set(token, { userId, exp });
  return { token, userId, expiresAt: exp };
}

/** 校验令牌：有效且未过期 → { userId }；否则 null（过期即清理）。 */
export function verifyToken(token) {
  if (!token) return null;
  const s = sessions.get(token);
  if (!s) return null;
  if (Date.now() > s.exp) {
    sessions.delete(token);
    return null;
  }
  return { userId: s.userId };
}

export function logout(token) {
  if (token) sessions.delete(token);
}
