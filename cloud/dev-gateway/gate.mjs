// CLOUD-GATE 模块（PI-GW-DEV-03/05）：真门禁——验令牌 + 订阅/额度 → 裁决。
// X-SEQ-001 门禁前置 / X-FORBID-002 服务端裁决：客户端不自决、不本地假通过；无/过期令牌、超额一律 denied。

import { verifyToken } from './auth.mjs';
import { evaluate } from './lic.mjs';

/** 裁决：{ verdict:'passed'|'denied', userId?, reason?, reauth_required?, used?, limit? }。 */
export function checkGate(token) {
  const session = verifyToken(token);
  if (!session) {
    return { verdict: 'denied', reason: 'invalid_or_expired_token', reauth_required: true };
  }
  const lic = evaluate(session.userId);
  if (lic.verdict !== 'passed') {
    return { verdict: 'denied', reason: lic.reason, used: lic.used, limit: lic.limit };
  }
  return { verdict: 'passed', userId: session.userId, used: lic.used, limit: lic.limit };
}
