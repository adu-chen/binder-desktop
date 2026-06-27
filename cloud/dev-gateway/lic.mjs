// LIC 模块（PI-GW-DEV-03 占位）：订阅/额度判定。
// dev：所有登录用户视为已订阅、不限额。正式版：真实订阅状态 + 额度判定/计量（REQ-GW-003/CLOUD-005），迁独立仓。

/** 用户是否有有效订阅（dev：恒 true）。 */
export function isSubscribed(_userId) {
  return true;
}
