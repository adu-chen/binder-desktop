// BYOK 模块（PI-GW-DEV-03，dev 内存代持）：{ provider: key }。仅内存，不落盘、不记日志、不回显。
// 正式版：加密存（GW-02）+ 会话/用户维度 + 删除/轮换，迁独立仓。

const byokKeys = new Map(); // provider -> key（dev 单用户）

export function setByokKey(provider, key) {
  byokKeys.set(provider, key);
}

export function getByokKey(provider) {
  return byokKeys.get(provider);
}
