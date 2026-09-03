export const VALID_RULE_SOURCE_STATUSES = new Set(['active', 'superseded']);

export function extractDocumentStatus(source) {
  const match = /^文档状态：\s*([ARX])\s*$/m.exec(source);
  return match?.[1] ?? null;
}

export function ruleSourceStatus(fields) {
  return (fields.status || 'active').trim();
}

export function ruleRegistryStatus(fields) {
  return ruleSourceStatus(fields) === 'superseded' ? 'superseded' : 'registered';
}

export function activeDocumentMayOwnRule(documentStatus, fields) {
  return documentStatus === 'A' || ruleSourceStatus(fields) === 'superseded';
}

export function ruleIsSuperseded(fields) {
  return ruleSourceStatus(fields) === 'superseded';
}

export function activeReferenceMayUseRule(status) {
  return status !== 'superseded';
}
