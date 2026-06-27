// LIC 模块（PI-GW-DEV-03/05）：订阅 + 额度判定（CLOUD-GATE 第二关）。
// dev：所有用户 free 档、订阅恒有效；额度 = 当日调用数 vs DAILY_QUOTA（用 PI-GW-DEV-04 计量）。
// 正式版：真实订阅状态/档位/计费 + 额度窗口/重置 + 入 DB，迁独立仓。

import { countToday } from './usage.mjs';

const DAILY_QUOTA = Number(process.env.GW_DAILY_QUOTA ?? 50);

/** 订阅 + 额度裁决：{ verdict:'passed'|'denied', reason?, used?, limit? }。 */
export function evaluate(userId) {
  // 订阅（dev：free 档恒有效；正式版查真实订阅态）。
  const subscribed = true;
  if (!subscribed) return { verdict: 'denied', reason: 'no_subscription' };

  // 额度（当日用量 vs 配额）。
  const used = countToday(userId);
  if (used >= DAILY_QUOTA) {
    return { verdict: 'denied', reason: 'quota_exceeded', used, limit: DAILY_QUOTA };
  }
  return { verdict: 'passed', used, limit: DAILY_QUOTA };
}
