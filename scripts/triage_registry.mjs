import { existsSync, readdirSync, readFileSync, statSync } from 'fs';
import { basename, join, relative } from 'path';

export const TRIAGE_FILE_SUFFIX = '_triage.md';
export const ANNOTATION_DEFERRAL_FILE_SUFFIX = '_标注延期清单.md';
export const VALID_TRIAGE_CLASSIFICATIONS = new Set([
  'VALID_EMPTY',
  'CALLER_OWNS_RULE',
  'ADAPTER_ONLY',
  'RULE_MISSING',
  'RULE_UNMAPPED',
  'LEGACY_COMPAT',
  'NEEDS_HUMAN_DECISION',
]);

const SKIP_DIRECTORIES = new Set(['.git', 'node_modules', 'target', 'dist', 'build', 'generated', '__mocks__']);
const TRIAGE_FIELDS = ['codes', '文件', '分类', '原因', '已采取动作', '需要人类介入', '已解决'];
const DEFERRAL_FIELDS = ['symbol', '文件', '原因', '承接计划', '退出条件', '复核日期', '需要人类介入', '已解决'];
const YES_NO = new Set(['是', '否']);

function collectFiles(root, suffix, results = []) {
  let entries;
  try { entries = readdirSync(root); } catch { return results; }
  for (const entry of entries) {
    if (SKIP_DIRECTORIES.has(entry)) continue;
    const fullPath = join(root, entry);
    const stat = statSync(fullPath);
    if (stat.isDirectory()) collectFiles(fullPath, suffix, results);
    else if (entry.endsWith(suffix)) results.push(fullPath);
  }
  return results;
}

function parseFields(block) {
  const fields = {};
  for (const line of block.split('\n')) {
    const separator = line.indexOf(':');
    if (separator === -1) continue;
    const key = line.slice(0, separator).trim();
    const value = line.slice(separator + 1).trim();
    if (key) fields[key] = value;
  }
  return fields;
}

function parseRecords(root, filePath, firstField, requiredFields, kind) {
  const source = readFileSync(filePath, 'utf8');
  const records = [];
  const findings = [];
  const blocks = source.split(new RegExp(`(?=^${firstField}:\\s*)`, 'm'));
  for (const block of blocks) {
    if (!new RegExp(`^${firstField}:\\s*`, 'm').test(block)) continue;
    const fields = parseFields(block);
    const missing = requiredFields.filter(field => !fields[field]);
    const registryFile = relative(root, filePath);
    const identifier = fields[firstField] || '(missing)';
    if (missing.length) {
      findings.push({
        type: 'TRIAGE_RECORD_INVALID',
        message: `${kind} record is missing required fields`,
        registry_file: registryFile,
        identifier,
        missing_fields: missing,
      });
      continue;
    }
    records.push({
      kind,
      registryFile,
      sourceFile: fields['文件'],
      resolved: fields['已解决'],
      needsHuman: fields['需要人类介入'],
      fields,
    });
  }
  return { records, findings };
}

function validateRecords(records, kind) {
  const findings = [];
  const seen = new Set();
  for (const record of records) {
    const identifier = kind === 'triage' ? record.fields.codes : `${record.sourceFile}::${record.fields.symbol}`;
    if (!YES_NO.has(record.resolved) || !YES_NO.has(record.needsHuman)) {
      findings.push({
        type: 'TRIAGE_RECORD_INVALID',
        message: `${kind} record uses an invalid 是/否 field`,
        registry_file: record.registryFile,
        identifier,
      });
    }
    if (seen.has(identifier)) {
      findings.push({
        type: 'TRIAGE_RECORD_DUPLICATE',
        message: `${kind} record key must be unique`,
        registry_file: record.registryFile,
        identifier,
      });
    }
    seen.add(identifier);
    if (kind === 'triage' && !VALID_TRIAGE_CLASSIFICATIONS.has(record.fields['分类'])) {
      findings.push({
        type: 'TRIAGE_RECORD_INVALID',
        message: 'triage record has an unsupported classification',
        registry_file: record.registryFile,
        identifier,
        classification: record.fields['分类'],
      });
    }
    if (kind === 'annotation_deferral' && !/^\d{4}-\d{2}-\d{2}$/.test(record.fields['复核日期'])) {
      findings.push({
        type: 'TRIAGE_RECORD_INVALID',
        message: 'annotation deferral has an invalid 复核日期',
        registry_file: record.registryFile,
        identifier,
      });
    }
  }
  return findings;
}

export function loadTriageRegistry(root) {
  const triageFiles = collectFiles(root, TRIAGE_FILE_SUFFIX);
  const deferralFiles = collectFiles(root, ANNOTATION_DEFERRAL_FILE_SUFFIX);
  const triageResults = triageFiles.map(filePath => parseRecords(root, filePath, 'codes', TRIAGE_FIELDS, 'triage'));
  const deferralResults = deferralFiles.map(filePath => parseRecords(root, filePath, 'symbol', DEFERRAL_FIELDS, 'annotation_deferral'));
  const triageRecords = triageResults.flatMap(result => result.records);
  const deferralRecords = deferralResults.flatMap(result => result.records);
  const findings = [
    ...triageResults.flatMap(result => result.findings),
    ...deferralResults.flatMap(result => result.findings),
    ...validateRecords(triageRecords, 'triage'),
    ...validateRecords(deferralRecords, 'annotation_deferral'),
  ];
  return { triageFiles, deferralFiles, triageRecords, deferralRecords, findings };
}

export function activeRecords(records) {
  return records.filter(record => record.resolved === '否');
}

export function isReviewOverdue(reviewDate, today = new Date().toISOString().slice(0, 10)) {
  return reviewDate < today;
}

export function deferralKey(sourceFile, symbol) {
  return `${sourceFile}::${symbol}`;
}

export function sourceFileExists(root, sourceFile) {
  return Boolean(sourceFile) && existsSync(join(root, sourceFile));
}

export function displayTriageRecord(record) {
  return {
    codes: record.fields.codes,
    file: record.sourceFile,
    classification: record.fields['分类'],
    resolved: record.resolved,
  };
}

export function displayDeferralRecord(record) {
  return {
    symbol: record.fields.symbol,
    file: record.sourceFile,
    plan: record.fields['承接计划'],
    reviewDate: record.fields['复核日期'],
    resolved: record.resolved,
  };
}

export function isRegistryFile(filePath) {
  const name = basename(filePath);
  return name.endsWith(TRIAGE_FILE_SUFFIX) || name.endsWith(ANNOTATION_DEFERRAL_FILE_SUFFIX);
}
