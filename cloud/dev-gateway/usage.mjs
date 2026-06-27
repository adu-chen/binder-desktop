// 用量计量（PI-GW-DEV-04，REQ-GW-003）：每次调用记**脱敏元数据**——绝不含正文（prompt/输出/工具内容），
// 守 REQ-GW-006 内容不留存。dev 追加 .data/usage.jsonl；正式版入 DB 供 LIC 额度判定。

import { appendLine, readLines } from './store.mjs';

/** 记一条用量：仅 { ts, userId, provider, model }（无正文）。 */
export function recordUsage({ userId, provider, model }) {
  appendLine('usage.jsonl', {
    ts: new Date().toISOString(),
    userId: userId ?? 'unknown',
    provider,
    model,
  });
}

/** 当日某用户调用计数（供 LIC 额度判定，PI-GW-DEV-05）。 */
export function countToday(userId) {
  const today = new Date().toISOString().slice(0, 10);
  return readLines('usage.jsonl').filter(
    (r) => r.userId === userId && typeof r.ts === 'string' && r.ts.startsWith(today),
  ).length;
}
