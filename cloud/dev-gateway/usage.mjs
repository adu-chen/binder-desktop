// 用量计量（PI-GW-DEV-04/06，REQ-GW-003）：每次调用记**脱敏元数据**——绝不含正文（prompt/输出/工具内容），
// 守 REQ-GW-006 内容不留存。入 SQLite usage 表（db.mjs）供 LIC 额度判定；替原 .data/usage.jsonl。

import { db } from './db.mjs';

const insUsage = db.prepare('INSERT INTO usage(ts, user_id, provider, model) VALUES(?, ?, ?, ?)');
const cntToday = db.prepare("SELECT COUNT(*) AS n FROM usage WHERE user_id = ? AND substr(ts, 1, 10) = ?");

/** 记一条用量：仅 { ts, userId, provider, model }（无正文）。 */
export function recordUsage({ userId, provider, model }) {
  insUsage.run(new Date().toISOString(), userId ?? 'unknown', provider ?? null, model ?? null);
}

/** 当日某用户调用计数（供 LIC 额度判定，PI-GW-DEV-05）。 */
export function countToday(userId) {
  const today = new Date().toISOString().slice(0, 10);
  return cntToday.get(userId, today).n;
}
