// CLOUD-GATE 模块（PI-GW-DEV-03）：真门禁——验令牌 + 订阅 → 裁决（替桩恒过）。
// X-SEQ-001 门禁前置 / X-FORBID-002 服务端裁决：客户端不自决、不本地假通过；无/过期令牌一律 denied。

import { verifyToken } from './auth.mjs';
import { isSubscribed } from './lic.mjs';

/** 裁决：{ verdict: 'passed'|'denied', userId?, reason?, reauth_required? }。 */
export function checkGate(token) {
  const session = verifyToken(token);
  if (!session) {
    return { verdict: 'denied', reason: 'invalid_or_expired_token', reauth_required: true };
  }
  if (!isSubscribed(session.userId)) {
    return { verdict: 'denied', reason: 'no_subscription' };
  }
  return { verdict: 'passed', userId: session.userId };
}
