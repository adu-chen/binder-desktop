#!/usr/bin/env node
// 只读治理审计脚本。验证治理框架文件存在且配置正确，
// 并检查 project_root 下的设计文档和源代码是否符合 ADU 治理规则。
// 用法：npm run governance:audit [-- --issue <issue-trace-path>]

import { existsSync, readFileSync, readdirSync, statSync } from 'fs';
import { execFileSync } from 'child_process';
import { basename, dirname, join } from 'path';
import { fileURLToPath } from 'url';
import { auditTestGovernance } from './test_governance.mjs';
import { auditProductCapabilityRegistry } from './product_capability.mjs';
import {
  activeRecords,
  deferralKey,
  isReviewOverdue,
  loadTriageRegistry,
  sourceFileExists,
} from './triage_registry.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, '..');
const issuePath = parseIssueArg(process.argv.slice(2));

// ── governance.config.json 读取 ───────────────────────────────────────────
const configPath = join(ROOT, 'governance.config.json');
let govConfig = { project_root: '.', apc_path: '', adus_path: '' };
if (existsSync(configPath)) {
  try { govConfig = JSON.parse(readFileSync(configPath, 'utf8')); } catch { /* 格式错误时用默认值 */ }
}
const PROJECT_ROOT_REL = (govConfig.project_root || '.').trim();

// ── 必须存在的治理框架文件（相对于 ROOT）────────────────────────────────
const REQUIRED_FILES = [
  'ADU.md',
  'ADP.md',
  'AGENTS.md',
  'CLAUDE.md',
  'governance.config.json',
  '.governance/test-governance.json',
  'scripts/generate_adus.mjs',
  'scripts/audit_taskflow_governance.mjs',
  'scripts/test_governance.mjs',
  'scripts/test_governance.test.mjs',
  'scripts/product_capability.mjs',
  'scripts/generate_product_capability_atlas.mjs',
  'scripts/audit_product_capability.mjs',
];

const LEGACY_ADU_FILES = [
  'adu/adu.md',
  'adu/ADP.md',
  'adu/CLAUDE.md',
  'adu/adus.md',
  'adu/generate_adus.mjs',
];

const REQUIRED_PACKAGE_SCRIPTS = {
  'governance:generate': 'node scripts/generate_adus.mjs',
  'governance:audit': 'node scripts/audit_taskflow_governance.mjs',
  'test:governance': 'vitest run scripts/test_governance.test.mjs && node scripts/test_governance.mjs audit',
  'test:inventory': 'node scripts/test_governance.mjs inventory',
  'test:plan': 'node scripts/test_governance.mjs select',
  'test:related': 'node scripts/test_governance.mjs run',
  'test:full': 'npm test && cargo test --manifest-path src-tauri/Cargo.toml',
  'product:test': 'vitest run scripts/test_governance.test.mjs -t "Product Capability Graph"',
  'product:generate': 'node scripts/generate_product_capability_atlas.mjs && node scripts/generate_function_ledger.mjs',
  'product:audit': 'node scripts/audit_product_capability.mjs',
};

// ADU §4.3 八种代码块类型
const VALID_GOV_TYPES = new Set(['DATA', 'GUARD', 'EFFECT', 'RB', 'API', 'UTIL', 'IO', 'QUERY']);
// Rules whose implementation owner is outside this client repository. Keep this
// list narrow and backed by controlled debt records; do not use it for local gaps.
const CLIENT_OUT_OF_SCOPE_RULES = new Set(['BR-SYS-DATA-004']);

const GOVERNANCE_FILES = new Set(['ADU.md', 'adu.md', 'ADUS.md', 'adus.md', 'ADP.md', 'AGENTS.md', 'CLAUDE.md']);
// APC 是宪章文档、ADUS 是自动生成索引，两者不含业务标注块，排除在扫描外
const APC_PATH  = govConfig.apc_path  ? govConfig.apc_path.trim()  : null;
const ADUS_PATH = govConfig.adus_path ? govConfig.adus_path.trim() : null;

const checks = [];
const governance_findings = [];

function addCheck(check) { checks.push(check); }
function addFinding(type, message, details = {}) { governance_findings.push({ type, message, ...details }); }

function fileExists(relativePath) { return existsSync(join(ROOT, relativePath)); }
function readText(relativePath) { return readFileSync(join(ROOT, relativePath), 'utf8'); }

function collectFiles(relDir, predicate, results = []) {
  const fullDir = join(ROOT, relDir);
  let entries;
  try { entries = readdirSync(fullDir); } catch { return results; }
  for (const entry of entries) {
    const relPath = `${relDir}/${entry}`;
    const fullPath = join(ROOT, relPath);
    const stat = statSync(fullPath);
    if (stat.isDirectory()) {
      collectFiles(relPath, predicate, results);
    } else if (predicate(relPath)) {
      results.push(relPath);
    }
  }
  return results;
}

function parseKVBlock(body) {
  const fields = {};
  for (const line of body.split('\n')) {
    const sep = line.indexOf(':');
    if (sep === -1) continue;
    const key = line.slice(0, sep).trim();
    const val = line.slice(sep + 1).trim();
    if (key) fields[key] = val;
  }
  return fields;
}

// 支持中文键名和英文键名双版本查找
function getField(fields, ...keys) {
  for (const k of keys) { if (fields[k] !== undefined) return fields[k]; }
  return undefined;
}

function parseAnnotationBlocks(relativePath) {
  const src = readText(relativePath);
  const blocks = [];
  const blockRe = /<!--[ \t]*(MODULE|DOMAIN|REQ_MAP|REQ|RULE|CHAIN|CONSTRAINT|TERM|STATE_MACHINE)[ \t]*\r?\n([\s\S]*?)-->/g;
  let match;
  while ((match = blockRe.exec(src)) !== null) {
    const before = src.slice(0, match.index);
    const line = before.split('\n').length;
    blocks.push({ kind: match[1], fields: parseKVBlock(match[2]), file: relativePath, line });
  }
  return blocks;
}

function extractField(lines, fieldName) {
  const pattern = new RegExp(`^\\*\\s+${fieldName}:\\s*(.*)$`);
  let value = null;
  let capturing = false;
  for (const line of lines) {
    const trimmed = line.trim();
    const m = pattern.exec(trimmed);
    if (m) { value = m[1].trim(); capturing = true; continue; }
    if (capturing) {
      const nextField = /^\*\s+(codes|type|chain|rules|boundary|term_ref)\s*:/.test(trimmed);
      if (trimmed.startsWith('*') && !nextField) { value += ` ${trimmed.replace(/^\*\s*/, '').trim()}`; }
      else { capturing = false; }
    }
  }
  return value;
}

function parseGovBlocks(relativePath) {
  const lines = readText(relativePath).split('\n');
  const blocks = [];
  let inGov = false, govLines = [], startLine = 0;
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    if (!inGov && line.includes('@GOV')) { inGov = true; govLines = []; startLine = i + 1; continue; }
    if (!inGov) continue;
    if (line.includes('*/')) {
      blocks.push({
        file: relativePath, line: startLine,
        fields: {
          codes: extractField(govLines, 'codes'),
          type: extractField(govLines, 'type'),
          chain: extractField(govLines, 'chain'),
          rules: extractField(govLines, 'rules'),
          boundary: extractField(govLines, 'boundary'),
          term_ref: extractField(govLines, 'term_ref'),
        },
      });
      inGov = false; govLines = [];
    } else { govLines.push(line); }
  }
  return blocks;
}

function splitCsv(value) {
  if (!value || value === '[]') return [];
  return value.split(',').map(s => s.trim()).filter(Boolean);
}

function parseIssueArg(args) {
  const i = args.indexOf('--issue');
  return i === -1 ? null : (args[i + 1] ?? '');
}

function extractIssueList(content, heading) {
  const section = extractIssueSection(content, heading);
  if (section === null) return [];
  return section.split('\n').map(l => l.trim()).filter(l => l.startsWith('- ')).map(l => l.slice(2).trim()).filter(Boolean);
}

function extractIssueSection(content, heading) {
  const m = new RegExp(`^## ${heading}\\s*$`, 'm').exec(content);
  if (!m) return null;
  const after = content.slice(m.index + m[0].length);
  const next = after.search(/^## /m);
  return next === -1 ? after : after.slice(0, next);
}

function sectionHasText(content, heading) {
  const section = extractIssueSection(content, heading);
  if (section === null) return false;
  return section.split('\n').map(l => l.trim()).some(l => l && !l.startsWith('<!--'));
}

// ADP §3.1 Scope Lock 块解析（围栏代码块，含 task_type 与 confirmation 字段）
function parseScopeLockBlock(content) {
  const fenceRe = /```[\w-]*\n([\s\S]*?)```/g;
  let m;
  while ((m = fenceRe.exec(content)) !== null) {
    const body = m[1];
    if (/^\s*task_type\s*:/m.test(body) && /^\s*confirmation\s*:/m.test(body)) return body;
  }
  return null;
}

function parseScopeLockList(body, key) {
  const out = [];
  let capturing = false;
  for (const line of body.split('\n')) {
    if (new RegExp(`^\\s*${key}\\s*:`).test(line)) { capturing = true; continue; }
    if (!capturing) continue;
    if (/^\s*-\s+/.test(line)) {
      let item = line.replace(/^\s*-\s+/, '');
      const hash = item.indexOf('#');
      if (hash !== -1) item = item.slice(0, hash);
      item = item.trim();
      if (item) out.push(item);
    } else if (/^\s*[\w-]+\s*:/.test(line)) {
      capturing = false; // 下一个顶层字段
    }
  }
  return out;
}

function parseScopeLockField(body, key) {
  const m = new RegExp(`^\\s*${key}\\s*:\\s*(.*)$`, 'm').exec(body);
  return m ? m[1].trim() : null;
}

function getChangedFiles() {
  const output = execFileSync('git', ['status', '--short'], { cwd: ROOT, encoding: 'utf8' });
  return output.split('\n').map(l => l.trimEnd()).filter(Boolean).map(l => {
    const status = l.slice(0, 2);
    const raw = l.slice(3).trim();
    if (status.includes('R') && raw.includes(' -> ')) return raw.split(' -> ').at(-1);
    return raw;
  }).filter(Boolean);
}

function matchesPattern(file, pattern) {
  if (pattern.endsWith('/**')) {
    const prefix = pattern.slice(0, -3);
    return file === prefix || file.startsWith(`${prefix}/`);
  }
  return file === pattern;
}

function matchesAny(file, patterns) { return patterns.some(p => matchesPattern(file, p)); }

function runIssueScopeAudit(relativeIssuePath) {
  if (!relativeIssuePath) { addFinding('OWNER_MISSING', '--issue argument requires a path'); return; }
  if (!fileExists(relativeIssuePath)) { addFinding('OWNER_MISSING', 'Issue Trace file does not exist', { issue: relativeIssuePath }); return; }

  const issueContent = readText(relativeIssuePath);

  // 优先解析 ADP §3.1 Scope Lock 块；缺失时回退到旧版英文标题（向后兼容）
  const scopeBlock = parseScopeLockBlock(issueContent);
  let allowedFiles, forbiddenFiles, confirmation;
  if (scopeBlock) {
    allowedFiles = parseScopeLockList(scopeBlock, 'allowed_files');
    forbiddenFiles = parseScopeLockList(scopeBlock, 'forbidden_files');
    confirmation = (parseScopeLockField(scopeBlock, 'confirmation') || '').toUpperCase();
  } else {
    allowedFiles = extractIssueList(issueContent, 'Allowed Files');
    forbiddenFiles = extractIssueList(issueContent, 'Forbidden Files');
    confirmation = 'YES'; // 旧版无 confirmation 门禁，视为已确认
  }
  const changedFiles = getChangedFiles();
  const malformedFields = [];

  if (allowedFiles.length === 0) malformedFields.push('allowed_files');
  if (forbiddenFiles.length === 0) malformedFields.push('forbidden_files');
  if (!sectionHasText(issueContent, 'Expected Behavior')) malformedFields.push('Expected Behavior');
  if (!sectionHasText(issueContent, 'Validation Commands')) malformedFields.push('Validation Commands');

  for (const field of malformedFields) {
    addFinding('ISSUE_TRACE_MALFORMED', 'Issue Trace is missing a required non-empty section', { issue: relativeIssuePath, field });
  }
  addCheck({ name: 'Issue Trace structure', status: malformedFields.length === 0 ? 'passed' : 'failed', issue: relativeIssuePath, malformed_fields: malformedFields });

  // confirmation 必须为 YES 才允许文件改动（ADP §3.1：PENDING 时不得修改文件）
  if (confirmation !== 'YES' && changedFiles.length > 0) {
    addFinding('SCOPE_LOCK_UNCONFIRMED', 'Scope Lock confirmation is not YES but files were changed', { issue: relativeIssuePath, confirmation: confirmation || null, changed_files: changedFiles.length });
  }
  addCheck({ name: 'Scope Lock confirmation', status: confirmation === 'YES' ? 'passed' : (changedFiles.length === 0 ? 'passed' : 'failed'), confirmation: confirmation || null });

  for (const file of changedFiles) {
    const isForbidden = matchesAny(file, forbiddenFiles);
    const isAllowed = matchesAny(file, allowedFiles);
    if (!isForbidden && isAllowed) continue;
    addFinding('SCOPE_OVERREACH', isForbidden ? 'File matches Issue Trace forbidden scope' : 'File is outside Issue Trace allowed scope', { file, issue: relativeIssuePath });
  }
  addCheck({
    name: 'Issue Trace scope',
    status: governance_findings.some(f => f.type === 'SCOPE_OVERREACH') ? 'failed' : 'passed',
    issue: relativeIssuePath, changed_files: changedFiles, allowed_files: allowedFiles, forbidden_files: forbiddenFiles,
  });
}

// ── 必需治理文件检查 ─────────────────────────────────────────────────────
for (const file of REQUIRED_FILES) {
  addCheck({ name: `required file exists: ${file}`, status: fileExists(file) ? 'passed' : 'failed' });
}

const testGovernanceConfigured = !!(govConfig.test_governance_path && govConfig.test_governance_path.trim());
addCheck({
  name: 'test_governance_path configured',
  status: testGovernanceConfigured ? 'passed' : 'failed',
});
if (testGovernanceConfigured) {
  addCheck({
    name: `test_governance_path target exists: ${govConfig.test_governance_path}`,
    status: fileExists(govConfig.test_governance_path.trim()) ? 'passed' : 'failed',
  });
}

const productCapabilityConfigured = !!(govConfig.product_capability_path && govConfig.product_capability_path.trim());
addCheck({
  name: 'product_capability_path configured',
  status: productCapabilityConfigured ? 'passed' : 'failed',
});
if (productCapabilityConfigured) {
  addCheck({
    name: `product_capability_path target exists: ${govConfig.product_capability_path}`,
    status: fileExists(govConfig.product_capability_path.trim()) ? 'passed' : 'failed',
  });
}

// adus_path 配置的 ADUS 文件必须存在（bootstrap 完成后）
if (govConfig.adus_path && govConfig.adus_path.trim()) {
  addCheck({
    name: `adus_path target exists: ${govConfig.adus_path}`,
    status: fileExists(govConfig.adus_path.trim()) ? 'passed' : 'failed',
  });
}

// apc_path 与 APC 文件检查（bootstrap 完成后两者必须同时有效）
// 判断 bootstrap 是否完成：adus_path 非空视为已完成 bootstrap
const bootstrapComplete = !!(govConfig.adus_path && govConfig.adus_path.trim());
if (bootstrapComplete) {
  const apcConfigured = !!(govConfig.apc_path && govConfig.apc_path.trim());
  addCheck({
    name: 'apc_path configured (bootstrap complete)',
    status: apcConfigured ? 'passed' : 'failed',
    note: apcConfigured ? undefined : 'adus_path 已填写但 apc_path 为空，bootstrap 流程可能未完整执行',
  });
  if (apcConfigured) {
    addCheck({
      name: `apc_path target exists: ${govConfig.apc_path}`,
      status: fileExists(govConfig.apc_path.trim()) ? 'passed' : 'failed',
    });
    if (fileExists(govConfig.apc_path.trim())) {
      const apcContent = readText(govConfig.apc_path.trim());
      const hasDocStatus = /文档状态：\s*A/.test(apcContent);
      addCheck({
        name: 'APC doc_status is A',
        status: hasDocStatus ? 'passed' : 'failed',
        note: hasDocStatus ? undefined : 'APC 文档状态不为 A，APC 未生效',
      });
    }
  }
}

for (const file of LEGACY_ADU_FILES) {
  addCheck({ name: `legacy ADU entry absent: ${file}`, status: fileExists(file) ? 'failed' : 'passed' });
}

// ── package.json 脚本检查 ─────────────────────────────────────────────────
const pkg = JSON.parse(readText('package.json'));
for (const [scriptName, expectedCommand] of Object.entries(REQUIRED_PACKAGE_SCRIPTS)) {
  addCheck({
    name: `package script: ${scriptName}`,
    status: pkg.scripts?.[scriptName] === expectedCommand ? 'passed' : 'failed',
    actual: pkg.scripts?.[scriptName] ?? null,
    expected: expectedCommand,
  });
}
const generateCommand = pkg.scripts?.['governance:generate'];
const auditCommand = pkg.scripts?.['governance:audit'];
addCheck({ name: 'generate and audit scripts are separated', status: generateCommand && auditCommand && generateCommand !== auditCommand ? 'passed' : 'failed' });

// ── 设计文档标注块扫描（project_root/docs） ───────────────────────────────
const docDir = PROJECT_ROOT_REL === '.' ? 'docs' : `${PROJECT_ROOT_REL}/docs`;
const docFiles = collectFiles(docDir, f =>
  f.endsWith('.md') &&
  !basename(f).endsWith('_triage.md') &&
  !GOVERNANCE_FILES.has(basename(f)) &&
  f !== APC_PATH &&
  f !== ADUS_PATH
);
const annotationBlocks = docFiles.flatMap(parseAnnotationBlocks);
const rules = new Set();
const chains = new Set();
const chainStatus = new Map(); // chain_id -> status（active | planned | superseded）
const chainIntegrationTests = new Map(); // chain_id -> integration_test
const modules = new Set();
const constraints = new Set();
const requirements = new Map(); // req_id -> annotation block
const requirementMaps = [];
const requirementMapKeys = new Set();
const duplicateRequirementMappings = [];
const stateMachines = new Map(); // machine_id -> annotation block
const validRequirementStatuses = new Set(['active', 'candidate', 'deferred', 'blocked', 'superseded']);
const validRequirementMapStatuses = new Set(['mapped', 'candidate', 'deferred', 'blocked', 'superseded']);
const validRequirementMapTargets = new Set(['RULE', 'CHAIN', 'CONSTRAINT']);
const validStateMachineStatuses = new Set(['active', 'candidate', 'deferred', 'superseded']);

for (const block of annotationBlocks) {
  if (block.kind === 'RULE') {
    const id = block.fields['rule_id'];
    if (id) rules.add(id);
    // 支持中英文字段名
    if (!getField(block.fields, 'rule_id')) addFinding('OWNER_MISSING', 'RULE block missing rule_id', { file: block.file, line: block.line });
    if (!getField(block.fields, '域', 'domain')) addFinding('OWNER_MISSING', 'RULE block missing 域/domain', { file: block.file, line: block.line });
    if (!getField(block.fields, '主链路', 'main_chain')) addFinding('OWNER_MISSING', 'RULE block missing 主链路/main_chain', { file: block.file, line: block.line });
  }
  if (block.kind === 'CHAIN') {
    const id = block.fields['chain_id'];
    if (id) {
      chains.add(id);
      chainStatus.set(id, (block.fields['status'] || 'active').trim());
      chainIntegrationTests.set(
        id,
        (getField(block.fields, 'integration_test', '集成测试', 'integration_tests') || '').trim(),
      );
    }
    if (!getField(block.fields, 'chain_id')) addFinding('OWNER_MISSING', 'CHAIN block missing chain_id', { file: block.file, line: block.line });
    if (!getField(block.fields, '主责模块', 'owner_module')) addFinding('OWNER_MISSING', 'CHAIN block missing 主责模块/owner_module', { file: block.file, line: block.line });
    if (!getField(block.fields, 'status')) addFinding('OWNER_MISSING', 'CHAIN block missing status', { file: block.file, line: block.line });
  }
  if (block.kind === 'MODULE') {
    const id = block.fields['module_code'];
    if (id) modules.add(id);
    for (const field of ['module_code', 'module_name', 'description']) {
      if (!block.fields[field]) addFinding('OWNER_MISSING', `MODULE block missing ${field}`, { file: block.file, line: block.line });
    }
  }
  if (block.kind === 'CONSTRAINT') {
    const id = block.fields['constraint_id'];
    if (id) constraints.add(id);
    if (!getField(block.fields, 'constraint_id')) addFinding('OWNER_MISSING', 'CONSTRAINT block missing constraint_id', { file: block.file, line: block.line });
    if (!getField(block.fields, '摘要', 'summary')) addFinding('OWNER_MISSING', 'CONSTRAINT block missing 摘要/summary', { file: block.file, line: block.line });
  }
  if (block.kind === 'REQ') {
    const id = getField(block.fields, 'req_id');
    for (const field of ['req_id', 'title', 'owner_module', 'status', 'summary']) {
      if (!getField(block.fields, field)) addFinding('OWNER_MISSING', `REQ block missing ${field}`, { file: block.file, line: block.line });
    }
    if (id && !/^REQ-[A-Z0-9]+-\d{3}$/.test(id)) {
      addFinding('REQ_ID_INVALID', 'REQ req_id must match REQ-[MODULE]-[SEQ]', { file: block.file, line: block.line, req_id: id });
    }
    const status = getField(block.fields, 'status');
    if (status && !validRequirementStatuses.has(status)) {
      addFinding('REQ_STATUS_INVALID', 'REQ status is invalid', { file: block.file, line: block.line, req_id: id, status });
    }
    if (id) {
      if (requirements.has(id)) {
        addFinding('REQ_DUPLICATE', 'REQ req_id is registered more than once', {
          req_id: id, file: block.file, line: block.line, first_file: requirements.get(id).file,
        });
      } else {
        requirements.set(id, block);
      }
    }
  }
  if (block.kind === 'REQ_MAP') {
    for (const field of ['req_id', 'target_type', 'target_id', 'status']) {
      if (!getField(block.fields, field)) addFinding('OWNER_MISSING', `REQ_MAP block missing ${field}`, { file: block.file, line: block.line });
    }
    const reqId = getField(block.fields, 'req_id');
    const targetType = getField(block.fields, 'target_type');
    const targetId = getField(block.fields, 'target_id');
    const status = getField(block.fields, 'status');
    if (targetType && !validRequirementMapTargets.has(targetType)) {
      addFinding('REQ_MAP_TARGET_TYPE_INVALID', 'REQ_MAP target_type is invalid', { file: block.file, line: block.line, req_id: reqId, target_type: targetType });
    }
    if (status && !validRequirementMapStatuses.has(status)) {
      addFinding('REQ_MAP_STATUS_INVALID', 'REQ_MAP status is invalid', { file: block.file, line: block.line, req_id: reqId, status });
    }
    const mappingKey = [reqId, targetType, targetId, status].join('|');
    if (requirementMapKeys.has(mappingKey)) {
      duplicateRequirementMappings.push({ file: block.file, line: block.line, req_id: reqId, target_type: targetType, target_id: targetId, status });
    } else {
      requirementMapKeys.add(mappingKey);
    }
    requirementMaps.push(block);
  }
  if (block.kind === 'STATE_MACHINE') {
    for (const field of ['machine_id', 'owner_module', 'status', 'summary', 'source_type']) {
      if (!getField(block.fields, field)) addFinding('OWNER_MISSING', `STATE_MACHINE block missing ${field}`, { file: block.file, line: block.line });
    }
    const id = getField(block.fields, 'machine_id');
    const status = getField(block.fields, 'status');
    if (id && !/^SM-[A-Z0-9]+-\d{2}$/.test(id)) {
      addFinding('STATE_MACHINE_ID_INVALID', 'STATE_MACHINE machine_id must match SM-[MODULE]-[SEQ]', { file: block.file, line: block.line, machine_id: id });
    }
    if (status && !validStateMachineStatuses.has(status)) {
      addFinding('STATE_MACHINE_STATUS_INVALID', 'STATE_MACHINE status is invalid', { file: block.file, line: block.line, machine_id: id, status });
    }
    if (id) {
      if (stateMachines.has(id)) {
        addFinding('STATE_MACHINE_DUPLICATE', 'STATE_MACHINE machine_id is registered more than once', {
          machine_id: id, file: block.file, line: block.line, first_file: stateMachines.get(id).file,
        });
      } else {
        stateMachines.set(id, block);
      }
    }
  }
}

for (const [reqId, block] of requirements) {
  const ownerModule = getField(block.fields, 'owner_module');
  // 云端需求由独立 APC 管理，当前客户端模块表可能不登记其 owner；在覆盖摘要中保留事实，不作为本项目硬错误。
  void ownerModule;
}

const targetRegistries = { RULE: rules, CHAIN: chains, CONSTRAINT: constraints };
for (const block of requirementMaps) {
  const reqId = getField(block.fields, 'req_id');
  const targetType = getField(block.fields, 'target_type');
  const targetId = getField(block.fields, 'target_id');
  if (reqId && !requirements.has(reqId)) {
    addFinding('REQ_MAP_REQUIREMENT_UNKNOWN', 'REQ_MAP references an unknown REQ', { req_id: reqId, file: block.file, line: block.line });
  }
  const registry = targetRegistries[targetType];
  if (registry && targetId && !registry.has(targetId)) {
    const actualType = Object.entries(targetRegistries).find(([, ids]) => ids.has(targetId))?.[0];
    addFinding(actualType ? 'REQ_MAP_TARGET_TYPE_MISMATCH' : 'REQ_MAP_TARGET_UNKNOWN',
      actualType ? 'REQ_MAP target_id exists under a different target_type' : 'REQ_MAP target_id is not registered', {
        req_id: reqId, target_type: targetType, target_id: targetId, actual_type: actualType ?? null,
        file: block.file, line: block.line,
      });
  }
}

const requirementCoverage = new Map();
for (const [reqId, block] of requirements) {
  const owner = getField(block.fields, 'owner_module') || 'UNKNOWN';
  if (!requirementCoverage.has(owner)) requirementCoverage.set(owner, { total: 0, active: 0, mapped: 0, unmapped_active: [] });
  const summary = requirementCoverage.get(owner);
  summary.total += 1;
  if (getField(block.fields, 'status') === 'active') {
    summary.active += 1;
    const mapped = requirementMaps.some(mapping =>
      getField(mapping.fields, 'req_id') === reqId && getField(mapping.fields, 'status') === 'mapped');
    if (mapped) summary.mapped += 1;
    else summary.unmapped_active.push(reqId);
  }
}

for (const [machineId, block] of stateMachines) {
  const ownerModule = getField(block.fields, 'owner_module');
  if (ownerModule && !modules.has(ownerModule)) {
    addFinding('STATE_MACHINE_OWNER_UNKNOWN', 'STATE_MACHINE owner_module is not registered', { machine_id: machineId, owner_module: ownerModule, file: block.file, line: block.line });
  }
  for (const chainId of splitCsv(getField(block.fields, 'chain'))) {
    if (!chains.has(chainId)) addFinding('STATE_MACHINE_CHAIN_UNKNOWN', 'STATE_MACHINE references an unknown CHAIN', { machine_id: machineId, chain: chainId, file: block.file, line: block.line });
  }
  for (const ruleId of splitCsv(getField(block.fields, 'rules'))) {
    if (!rules.has(ruleId)) addFinding('STATE_MACHINE_RULE_UNKNOWN', 'STATE_MACHINE references an unknown RULE', { machine_id: machineId, rule: ruleId, file: block.file, line: block.line });
  }
}

const mappingFindingTypes = new Set([
  'REQ_ID_INVALID', 'REQ_STATUS_INVALID', 'REQ_DUPLICATE',
  'REQ_MAP_TARGET_TYPE_INVALID', 'REQ_MAP_STATUS_INVALID',
  'REQ_MAP_REQUIREMENT_UNKNOWN', 'REQ_MAP_TARGET_TYPE_MISMATCH', 'REQ_MAP_TARGET_UNKNOWN',
  'STATE_MACHINE_ID_INVALID', 'STATE_MACHINE_STATUS_INVALID', 'STATE_MACHINE_DUPLICATE',
  'STATE_MACHINE_OWNER_UNKNOWN', 'STATE_MACHINE_CHAIN_UNKNOWN', 'STATE_MACHINE_RULE_UNKNOWN',
]);
addCheck({
  name: 'REQ / REQ_MAP / STATE_MACHINE integrity',
  status: governance_findings.some(finding => mappingFindingTypes.has(finding.type)) ? 'failed' : 'passed',
  requirements: requirements.size,
  requirement_mappings: requirementMaps.length,
  unique_requirement_mappings: requirementMapKeys.size,
  duplicate_requirement_mappings: duplicateRequirementMappings,
  state_machines: stateMachines.size,
  coverage_by_module: Object.fromEntries([...requirementCoverage.entries()].sort(([a], [b]) => a.localeCompare(b))),
  note: 'active coverage is reported per module; mapped target and state-machine reference integrity fail closed',
});

if (ADUS_PATH && fileExists(ADUS_PATH)) {
  const adusContent = readText(ADUS_PATH);
  const escapeRegex = value => value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const missingRequirements = [...requirements.keys()].filter(reqId =>
    !new RegExp(`^\\|\\s*${escapeRegex(reqId)}\\s*\\|`, 'm').test(adusContent));
  const missingMappings = requirementMaps.filter(block => {
    const reqId = escapeRegex(getField(block.fields, 'req_id') || '');
    const targetType = escapeRegex(getField(block.fields, 'target_type') || '');
    const targetId = escapeRegex(getField(block.fields, 'target_id') || '');
    return !new RegExp(`^\\|\\s*${reqId}\\s*\\|\\s*${targetType}\\s*\\|\\s*${targetId}\\s*\\|`, 'm').test(adusContent);
  });
  const missingStateMachines = [...stateMachines.keys()].filter(machineId =>
    !new RegExp(`^\\|\\s*${escapeRegex(machineId)}\\s*\\|`, 'm').test(adusContent));
  if (missingRequirements.length || missingMappings.length || missingStateMachines.length) {
    addFinding('ADUS_REGISTRY_STALE', 'ADUS is missing REQ, REQ_MAP, or STATE_MACHINE registrations; run governance:generate', {
      missing_requirements: missingRequirements,
      missing_requirement_mappings: missingMappings.map(block => ({
        req_id: getField(block.fields, 'req_id'), target_type: getField(block.fields, 'target_type'), target_id: getField(block.fields, 'target_id'),
      })),
      missing_state_machines: missingStateMachines,
    });
  }
  addCheck({
    name: 'ADUS requirement and state-machine registry freshness',
    status: missingRequirements.length || missingMappings.length || missingStateMachines.length ? 'failed' : 'passed',
    missing_requirements: missingRequirements.length,
    missing_requirement_mappings: missingMappings.length,
    missing_state_machines: missingStateMachines.length,
  });
}

const deferredChainIntegrationTests = [];
for (const chainId of chains) {
  const status = chainStatus.get(chainId) || 'active';
  if (status === 'planned' || status === 'superseded') continue;
  const integrationTest = chainIntegrationTests.get(chainId) || '';
  if (!integrationTest) {
    addFinding('CHAIN_INTEGRATION_TEST_MISSING', 'active CHAIN missing integration_test', { chain: chainId });
    continue;
  }
  if (integrationTest.startsWith('deferred:')) {
    deferredChainIntegrationTests.push({ chain: chainId, integration_test: integrationTest });
    continue;
  }
  const separator = integrationTest.indexOf('::');
  const testFile = separator === -1 ? '' : integrationTest.slice(0, separator).trim();
  const testName = separator === -1 ? '' : integrationTest.slice(separator + 2).trim();
  if (!testFile || !testName || !fileExists(testFile)) {
    addFinding('CHAIN_INTEGRATION_TEST_INVALID', 'integration_test must resolve to an existing file and named test', {
      chain: chainId,
      integration_test: integrationTest,
      test_file: testFile || null,
    });
  }
}

addCheck({
  name: 'CHAIN integration_test declarations',
  status: governance_findings.some(f => f.type === 'CHAIN_INTEGRATION_TEST_MISSING' || f.type === 'CHAIN_INTEGRATION_TEST_INVALID') ? 'failed' : 'passed',
  chains: chains.size,
  deferred: deferredChainIntegrationTests,
});

addCheck({
  name: 'ADU annotation blocks exist',
  status: rules.size > 0 && chains.size > 0 && modules.size > 0 ? 'passed' : 'failed',
  rules: rules.size, chains: chains.size, modules: modules.size, constraints: constraints.size,
  note: 'constraints 为 0 时不阻断（小型项目可能无跨模块约束）',
});

// legacy heading IDs
const markdownWithLegacyHeadings = docFiles.filter(f => /^## (RULE|CHAIN|MODULE|CONSTRAINT)-/m.test(readText(f)));
addCheck({ name: 'legacy markdown heading IDs absent', status: markdownWithLegacyHeadings.length === 0 ? 'passed' : 'failed', files: markdownWithLegacyHeadings });

// ── 源代码 @GOV 扫描（前端、Rust 后端与受治理脚本） ───────────────────────
const srcDir = PROJECT_ROOT_REL === '.' ? 'src' : `${PROJECT_ROOT_REL}/src`;
const rustDir = PROJECT_ROOT_REL === '.' ? 'src-tauri/src' : `${PROJECT_ROOT_REL}/src-tauri/src`;
const scriptsDir = PROJECT_ROOT_REL === '.' ? 'scripts' : `${PROJECT_ROOT_REL}/scripts`;
const PCG_GOVERNED_ENTRY_SCRIPTS = new Set([
  'scripts/generate_product_capability_atlas.mjs',
  'scripts/audit_product_capability.mjs',
]);
const sourceFiles = [
  ...collectFiles(srcDir, f => f.endsWith('.ts') || f.endsWith('.tsx')),
  ...collectFiles(rustDir, f => f.endsWith('.rs')),
  ...collectFiles(scriptsDir, f => PCG_GOVERNED_ENTRY_SCRIPTS.has(f)),
];

const filesWithLineGov = sourceFiles.filter(f => /^\s*\/\/\s*@GOV/m.test(readText(f)));
addCheck({ name: 'legacy line @GOV absent', status: filesWithLineGov.length === 0 ? 'passed' : 'failed', files: filesWithLineGov });

const govBlocks = sourceFiles.flatMap(parseGovBlocks);
const triageRegistry = loadTriageRegistry(ROOT);
for (const finding of triageRegistry.findings) {
  addFinding(finding.type, finding.message, finding);
}

const activeTriageRecords = activeRecords(triageRegistry.triageRecords);
const triageByCodes = new Map();
for (const record of activeTriageRecords) {
  const codes = record.fields.codes;
  const records = triageByCodes.get(codes) ?? [];
  records.push(record);
  triageByCodes.set(codes, records);
}
const govBlocksByCodes = new Map(govBlocks.map(block => [block.fields.codes, block]));
for (const record of activeTriageRecords) {
  const codes = record.fields.codes;
  const block = govBlocksByCodes.get(codes);
  if (!block) {
    addFinding('TRIAGE_RECORD_ORPHAN', 'triage record does not resolve to a current @GOV block', {
      codes, registry_file: record.registryFile,
    });
  } else if (block.file !== record.sourceFile) {
    addFinding('TRIAGE_RECORD_MISMATCH', 'triage record file differs from its @GOV block', {
      codes, registry_file: record.registryFile, record_file: record.sourceFile, gov_file: block.file,
    });
  } else if (block.fields.rules !== '[]') {
    addFinding('TRIAGE_RECORD_MISMATCH', 'triage record must target an @GOV block with rules: []', {
      codes, registry_file: record.registryFile,
    });
  }
}
for (const block of govBlocks.filter(candidate => candidate.fields.rules === '[]')) {
  const records = triageByCodes.get(block.fields.codes) ?? [];
  if (records.length !== 1) {
    addFinding('TRIAGE_REQUIRED', 'an @GOV block with rules: [] must have exactly one active triage record', {
      codes: block.fields.codes, file: block.file, line: block.line, records: records.length,
    });
  }
}
const referencedRules = new Set();
const referencedChains = new Set();
const codeBlockOwners = new Map();
const CODE_MAPPING_RE = /^(BR-[A-Z0-9]+-[A-Z0-9]+-\d{3})-((DATA|GUARD|EFFECT|RB|API|UTIL|IO|QUERY)-[A-Z0-9]+-[A-Z0-9-]+-\d{3})$/;
const UTIL_CODE_RE = /^(UTIL)-[A-Z0-9]+-[A-Z0-9-]+-\d{3}$/;

for (const block of govBlocks) {
  for (const field of ['codes', 'type', 'chain', 'rules', 'boundary']) {
    if (!block.fields[field]) addFinding('OWNER_MISSING', `@GOV block missing ${field}`, { file: block.file, line: block.line, field });
  }
  if (block.fields.type && !VALID_GOV_TYPES.has(block.fields.type)) {
    addFinding('BOUNDARY_MISMATCH', '@GOV block has unsupported type', { file: block.file, line: block.line, type: block.fields.type });
  }
  for (const chainId of splitCsv(block.fields.chain)) {
    referencedChains.add(chainId);
    if (!chains.has(chainId)) addFinding('CHAIN_MISMATCH', '@GOV references unknown CHAIN', { file: block.file, line: block.line, chain: chainId });
  }
  for (const ruleId of splitCsv(block.fields.rules)) {
    referencedRules.add(ruleId);
    if (!rules.has(ruleId)) addFinding('RULE_MISSING', '@GOV references unknown RULE', { file: block.file, line: block.line, rule: ruleId });
  }

  // ADU §4.2/§4.4：一个 @GOV 块只有一个全局唯一代码块 ID；rules 与 codes 双向闭合，
  // 且代码块 ID 的 TYPE 必须与 type 字段一致。该检查阻止拆分文件复制旧标注后形成多主责假象。
  const declaredRuleIds = splitCsv(block.fields.rules).filter(ruleId => ruleId.startsWith('BR-'));
  const mappedRuleIds = [];
  const blockIds = new Set();
  for (const code of splitCsv(block.fields.codes)) {
    const mapping = CODE_MAPPING_RE.exec(code);
    if (mapping) {
      mappedRuleIds.push(mapping[1]);
      blockIds.add(mapping[2]);
      if (block.fields.type && mapping[3] !== block.fields.type) {
        addFinding('CODE_BLOCK_TYPE_MISMATCH', 'code block ID type differs from @GOV type', {
          file: block.file, line: block.line, code, code_type: mapping[3], declared_type: block.fields.type,
        });
      }
      continue;
    }
    if (UTIL_CODE_RE.test(code)) {
      blockIds.add(code);
      if (block.fields.type !== 'UTIL') {
        addFinding('CODE_BLOCK_TYPE_MISMATCH', 'UTIL code block ID requires type=UTIL', {
          file: block.file, line: block.line, code, declared_type: block.fields.type,
        });
      }
      continue;
    }
    addFinding('CODE_MAPPING_INVALID', 'codes contains an invalid composite mapping ID', {
      file: block.file, line: block.line, code,
    });
  }
  if (blockIds.size !== 1) {
    addFinding('CODE_BLOCK_ID_MIXED', '@GOV block must use exactly one code block ID across all mappings', {
      file: block.file, line: block.line, block_ids: [...blockIds],
    });
  }
  for (const ruleId of declaredRuleIds) {
    if (!mappedRuleIds.includes(ruleId)) {
      addFinding('RULE_CODE_NOT_CLOSED', 'rules entry has no matching codes mapping', {
        file: block.file, line: block.line, rule: ruleId,
      });
    }
  }
  for (const ruleId of mappedRuleIds) {
    if (!declaredRuleIds.includes(ruleId)) {
      addFinding('RULE_CODE_NOT_CLOSED', 'codes mapping is absent from rules', {
        file: block.file, line: block.line, rule: ruleId,
      });
    }
  }
  for (const blockId of blockIds) {
    const owner = codeBlockOwners.get(blockId);
    if (owner && (owner.file !== block.file || owner.line !== block.line)) {
      addFinding('CODE_BLOCK_ID_DUPLICATE', 'code block ID is assigned to more than one @GOV block', {
        block_id: blockId, first_file: owner.file, first_line: owner.line,
        file: block.file, line: block.line,
      });
    } else {
      codeBlockOwners.set(blockId, { file: block.file, line: block.line });
    }
  }
}

addCheck({
  name: '@GOV rule-code closure and code block uniqueness',
  status: governance_findings.some(f => [
    'CODE_MAPPING_INVALID', 'CODE_BLOCK_TYPE_MISMATCH', 'CODE_BLOCK_ID_MIXED',
    'RULE_CODE_NOT_CLOSED', 'CODE_BLOCK_ID_DUPLICATE',
  ].includes(f.type)) ? 'failed' : 'passed',
  code_blocks: codeBlockOwners.size,
});

for (const ruleId of rules) {
  if (CLIENT_OUT_OF_SCOPE_RULES.has(ruleId)) continue;
  if (!referencedRules.has(ruleId)) addFinding('OWNER_MISSING', 'RULE not referenced by any @GOV block', { rule: ruleId });
}
for (const chainId of chains) {
  if (referencedChains.has(chainId)) continue;
  const status = chainStatus.get(chainId) || 'active';
  // planned 尚未实现、superseded 已废弃，均豁免入口标注检测（ADU §3.2）
  if (status === 'planned' || status === 'superseded') continue;
  addFinding('CHAIN_ENTRY_UNANNOTATED', 'CHAIN has no @GOV block referencing it (chain entry not annotated)', { chain: chainId, status });
}

addCheck({
  name: '@GOV blocks and references',
  status: govBlocks.length > 0 && governance_findings.length === 0 ? 'passed' : 'failed',
  gov_blocks: govBlocks.length, referenced_rules: referencedRules.size, referenced_chains: referencedChains.size,
});

// ── 测试覆盖注释（project_root/tests TS + src 前端 co-located + src-tauri Rust 内联 #[cfg(test)]） ──
// covers 注释可出现在：独立 TS 测试文件（tests/），或源码内联测试（Rust #[cfg(test)] mod tests /
// 前端 co-located *.test.tsx）。sourceFiles 已含 src 的 .ts/.tsx 与 src-tauri/src 的 .rs，一并扫描，
// 使 Rust 后端内联覆盖注释被计入（惯用写法，避免对 Rust 项目结构性漏计）。
const testsDir = PROJECT_ROOT_REL === '.' ? 'tests' : `${PROJECT_ROOT_REL}/tests`;
const testFiles = collectFiles(testsDir, f => f.endsWith('.test.ts') || f.endsWith('.spec.ts') || f.endsWith('.test.tsx') || f.endsWith('.spec.tsx'));
const governanceTestFiles = ['scripts/test_governance.test.mjs'].filter(fileExists);
const coveredRules = new Set();
for (const file of [...testFiles, ...governanceTestFiles, ...sourceFiles]) {
  for (const m of readText(file).matchAll(/^\s*\/\/\s*covers:\s*(\S+)\s*$/gm)) {
    coveredRules.add(m[1]);
  }
}
const uncoveredRules = [...rules].filter(id => !CLIENT_OUT_OF_SCOPE_RULES.has(id) && !coveredRules.has(id));
addCheck({ name: 'test coverage comments reference rules', status: uncoveredRules.length === 0 ? 'passed' : 'failed', covered_rules: coveredRules.size, uncovered_rules: uncoveredRules });

if (issuePath !== null) runIssueScopeAudit(issuePath);

// ── TERM 数据收集 ─────────────────────────────────────────────────────────
const termMap = new Map(); // term_id -> { zh, en, chains, forbidden, file }
const termEnIndex = new Map(); // en -> { termId, chains }
const termZhIndex = new Map(); // zh -> { termId, chains }
const term_findings = [];

function addTermFinding(type, message, details = {}) { term_findings.push({ type, message, ...details }); }

for (const block of annotationBlocks) {
  if (block.kind !== 'TERM') continue;
  const termId  = block.fields['term_id'];
  const zh      = block.fields['zh'] ?? '';
  const en      = block.fields['en'] ?? '';
  const chains  = block.fields['chains'] ?? block.fields['chain'] ?? '';
  const forbidden = block.fields['forbidden'] ?? '';

  if (!termId) { addTermFinding('OWNER_MISSING', 'TERM block missing term_id', { file: block.file, line: block.line }); continue; }
  if (!zh)     addTermFinding('OWNER_MISSING', 'TERM block missing zh',      { file: block.file, line: block.line, term_id: termId });
  if (!en)     addTermFinding('OWNER_MISSING', 'TERM block missing en',      { file: block.file, line: block.line, term_id: termId });
  if (!chains) addTermFinding('OWNER_MISSING', 'TERM block missing chains',  { file: block.file, line: block.line, term_id: termId });

  if (termMap.has(termId)) {
    addTermFinding('TERM_DUPLICATE', `TERM term_id="${termId}" duplicated`, {
      term_id: termId,
      file_1: termMap.get(termId).file,
      file_2: block.file,
    });
  }
  termMap.set(termId, { zh, en, chains, forbidden, file: block.file });

  // TERM_DUPLICATE: same en 或 zh 且 chains 有交集
  const chainSet = new Set(chains.split(',').map(s => s.trim()).filter(Boolean));
  if (en) {
    if (termEnIndex.has(en)) {
      const { termId: existId, chains: existChains } = termEnIndex.get(en);
      const overlap = [...chainSet].some(c => new Set(existChains.split(',').map(s => s.trim())).has(c));
      if (overlap) addTermFinding('TERM_DUPLICATE', `TERM en="${en}" duplicated in overlapping chains`, { term_id_1: existId, term_id_2: termId, en });
    } else {
      termEnIndex.set(en, { termId, chains });
    }
  }
  if (zh) {
    if (termZhIndex.has(zh)) {
      const { termId: existId, chains: existChains } = termZhIndex.get(zh);
      const overlap = [...chainSet].some(c => new Set(existChains.split(',').map(s => s.trim())).has(c));
      if (overlap) addTermFinding('TERM_DUPLICATE', `TERM zh="${zh}" duplicated in overlapping chains`, { term_id_1: existId, term_id_2: termId, zh });
    } else {
      termZhIndex.set(zh, { termId, chains });
    }
  }
}

// ── TERM_UNREGISTERED 和 TERM_ORPHAN ──────────────────────────────────────
const referencedTermIds = new Set();
for (const block of govBlocks) {
  if (block.fields.term_ref) {
    // term_ref may list multiple IDs separated by commas — split before lookup.
    for (const termId of splitCsv(block.fields.term_ref)) {
      referencedTermIds.add(termId);
      if (!termMap.has(termId)) {
        addTermFinding('TERM_UNREGISTERED', '@GOV term_ref points to unregistered TERM', {
          file: block.file, line: block.line, term_ref: termId,
        });
      }
    }
  }
}
for (const [termId, info] of termMap) {
  if (!referencedTermIds.has(termId)) {
    addTermFinding('TERM_ORPHAN', 'TERM registered but not referenced by any @GOV term_ref (warning)', {
      term_id: termId, file: info.file,
    });
  }
}

addCheck({
  name: 'TERM integrity (UNREGISTERED / DUPLICATE / ORPHAN)',
  status: term_findings.some(f => ['TERM_UNREGISTERED', 'TERM_DUPLICATE', 'OWNER_MISSING'].includes(f.type)) ? 'failed' : 'passed',
  term_count: termMap.size,
  referenced_terms: referencedTermIds.size,
});

// ── BOUNDARY_ABSTRACT 检测 ────────────────────────────────────────────────
const ABSTRACT_WORDS = new Set([
  'data', 'input', 'output', 'result', 'state', 'value',
  'object', 'item', 'payload', 'request', 'response', 'params', 'param', 'info',
]);

function hasAbstractBoundary(boundary) {
  if (!boundary) return false;
  for (const m of boundary.matchAll(/\b(in|out)\s*=\s*(\w+)/gi)) {
    if (ABSTRACT_WORDS.has(m[2].toLowerCase())) return true;
  }
  return false;
}

const boundaryAbstractBlocks = [];
for (const block of govBlocks) {
  if (hasAbstractBoundary(block.fields.boundary)) {
    boundaryAbstractBlocks.push({ file: block.file, line: block.line, boundary: block.fields.boundary });
    addFinding('BOUNDARY_ABSTRACT', '@GOV boundary uses abstract placeholder in in= or out= (should use business entity name)', {
      file: block.file, line: block.line,
    });
  }
}
addCheck({
  name: 'BOUNDARY_ABSTRACT absent',
  status: boundaryAbstractBlocks.length === 0 ? 'passed' : 'failed',
  violations: boundaryAbstractBlocks.length,
});

// ── new_domain_concept_candidate 检测 ─────────────────────────────────────
const EXPORT_TYPE_RE = /\bexport\s+(?:type|interface|class|enum|abstract\s+class)\s+(\w+)/g;
const COMMON_GENERIC = new Set([
  'Props', 'State', 'Ref', 'Context', 'Config', 'Options', 'Result', 'Error',
  'Event', 'Handler', 'Provider', 'Consumer', 'Component', 'Hook', 'Store',
  'Action', 'Reducer', 'Selector', 'Middleware', 'Plugin', 'Service',
]);
const termEnSet = new Set([...termMap.values()].map(t => t.en).filter(Boolean));
const newDomainCandidates = [];

for (const file of sourceFiles) {
  const src = readText(file);
  for (const m of src.matchAll(EXPORT_TYPE_RE)) {
    const name = m[1];
    if (name.length > 5 && !termEnSet.has(name) && !COMMON_GENERIC.has(name)) {
      newDomainCandidates.push({ name, file });
    }
  }
}
addCheck({
  name: 'new_domain_concept_candidate scan',
  status: 'passed', // informational only, never blocks
  candidates: newDomainCandidates.length,
  note: 'Review candidates manually — may be legitimate new concepts or naming drift',
});

// ── BLOCK_UNANNOTATED 负空间扫描 ──────────────────────────────────────────
// 扫描 src 导出的可执行符号（function / 箭头函数 const / class）。
// 只有精确匹配“文件 + 符号”的有效标注延期记录可以豁免负空间告警。
const activeDeferralRecords = activeRecords(triageRegistry.deferralRecords);
const activeDeferralsByKey = new Map(activeDeferralRecords.map(record => [
  deferralKey(record.sourceFile, record.fields.symbol), record,
]));
const scannedExportKeys = new Set();
const annotatedExportKeys = new Set();
for (const record of activeDeferralRecords) {
  const identifier = deferralKey(record.sourceFile, record.fields.symbol);
  if (!sourceFileExists(ROOT, record.sourceFile)) {
    addFinding('ANNOTATION_DEFERRAL_STALE', 'annotation deferral source file no longer exists', {
      identifier, registry_file: record.registryFile,
    });
  }
  if (isReviewOverdue(record.fields['复核日期'])) {
    addFinding('ANNOTATION_DEFERRAL_STALE', 'annotation deferral review date has passed', {
      identifier, registry_file: record.registryFile, review_date: record.fields['复核日期'],
    });
  }
}

const EXPORT_FN_RE = /^export\s+(?:default\s+)?(?:async\s+)?function\s+(\w+)/;
const EXPORT_CLASS_RE = /^export\s+(?:default\s+)?(?:abstract\s+)?class\s+(\w+)/;
const EXPORT_CONST_FN_RE = /^export\s+const\s+(\w+)\s*(?::[^=]+)?=\s*(?:async\s+)?(?:function\b|\([^)]*\)\s*(?::[^=]*?)?=>|[A-Za-z_$][\w$]*\s*=>)/;
const blockUnannotated = [];

for (const file of sourceFiles) {
  if (/\.(test|spec)\.tsx?$/.test(file)) continue;
  const lines = readText(file).split('\n');
  const govEndLines = [];
  let inGovScan = false;
  for (let i = 0; i < lines.length; i++) {
    if (!inGovScan && lines[i].includes('@GOV')) inGovScan = true;
    else if (inGovScan && lines[i].includes('*/')) { govEndLines.push(i); inGovScan = false; }
  }
  for (let i = 0; i < lines.length; i++) {
    const m = EXPORT_FN_RE.exec(lines[i]) || EXPORT_CLASS_RE.exec(lines[i]) || EXPORT_CONST_FN_RE.exec(lines[i]);
    if (!m) continue;
    const name = m[1];
    if (!name) continue;
    const identifier = deferralKey(file, name);
    scannedExportKeys.add(identifier);
    const annotated = govEndLines.some(e => e < i && i - e <= 5); // 上方 5 行内有 @GOV 块结束
    if (annotated) {
      annotatedExportKeys.add(identifier);
      continue;
    }
    if (activeDeferralsByKey.has(identifier)) continue;
    blockUnannotated.push({ file, line: i + 1, symbol: name });
  }
}
for (const record of activeDeferralRecords) {
  const identifier = deferralKey(record.sourceFile, record.fields.symbol);
  if (annotatedExportKeys.has(identifier)) {
    addFinding('ANNOTATION_DEFERRAL_STALE', 'annotation deferral targets a symbol that now has @GOV', {
      identifier, registry_file: record.registryFile,
    });
  } else if (!scannedExportKeys.has(identifier)) {
    addFinding('ANNOTATION_DEFERRAL_STALE', 'annotation deferral does not target an auditable exported symbol', {
      identifier, registry_file: record.registryFile,
    });
  }
}
addCheck({
  name: 'BLOCK_UNANNOTATED scan',
  status: 'passed', // 警告级，不阻断（负空间扫描存在启发式误报）
  unannotated: blockUnannotated.length,
  note: blockUnannotated.length > 0 ? '导出的可执行符号缺少 @GOV 标注，复核应补标注或登记精确的标注延期记录' : undefined,
});

// ── AI 语义审计候选列表 ────────────────────────────────────────────────────
const semanticReviewCandidates = govBlocks
  .filter(b => splitCsv(b.fields.rules).length > 0 && b.fields.boundary && b.fields.boundary.trim())
  .map(b => ({
    file: b.file,
    line: b.line,
    codes: b.fields.codes,
    rules: splitCsv(b.fields.rules),
    boundary: b.fields.boundary,
    term_ref: b.fields.term_ref ?? null,
    review_instruction: 'Compare rule document text against boundary description: check if core entity names match. Output finding per ADU §5.8 format.',
  }));

// ── 测试治理注册表、动态资产与变更映射能力 ────────────────────────────────
let testGovernanceAudit = null;
try {
  testGovernanceAudit = auditTestGovernance(ROOT);
  addCheck({
    name: 'test governance registry and inventory',
    status: testGovernanceAudit.status,
    ...testGovernanceAudit.checks,
    warnings: testGovernanceAudit.warnings.length,
  });
  for (const finding of testGovernanceAudit.findings) {
    addFinding(finding.type, 'Test governance audit failed', finding);
  }
} catch (error) {
  addCheck({
    name: 'test governance registry and inventory',
    status: 'failed',
    note: error.message,
  });
  addFinding('TEST_GOVERNANCE_UNAVAILABLE', error.message);
}

let productCapabilityAudit = null;
try {
  productCapabilityAudit = auditProductCapabilityRegistry(ROOT);
  addCheck({
    name: 'product capability registry and traceability',
    status: productCapabilityAudit.status,
    variants: productCapabilityAudit.summary.variants,
    warnings: productCapabilityAudit.warnings.length,
  });
  for (const finding of productCapabilityAudit.findings) {
    addFinding(finding.type, 'Product capability audit failed', finding);
  }
} catch (error) {
  addCheck({
    name: 'product capability registry and traceability',
    status: 'failed',
    note: error.message,
  });
  addFinding('PRODUCT_CAPABILITY_UNAVAILABLE', error.message);
}

// ─────────────────────────────────────────────────────────────────────────────

const failedChecks = checks.filter(c => c.status !== 'passed');
const result = {
  status: failedChecks.length === 0 && governance_findings.length === 0 &&
    !term_findings.some(f => ['TERM_UNREGISTERED', 'TERM_DUPLICATE'].includes(f.type)) ? 'passed' : 'failed',
  checks,
  governance_findings,
  term_findings,
  semantic_review_candidates: semanticReviewCandidates,
  new_domain_concept_candidates: newDomainCandidates,
  block_unannotated: blockUnannotated,
  test_governance: testGovernanceAudit ? {
    status: testGovernanceAudit.status,
    checks: testGovernanceAudit.checks,
    findings: testGovernanceAudit.findings,
    warnings: testGovernanceAudit.warnings,
  } : null,
  product_capability: productCapabilityAudit ? {
    status: productCapabilityAudit.status,
    summary: productCapabilityAudit.summary,
    findings: productCapabilityAudit.findings,
    warnings: productCapabilityAudit.warnings,
  } : null,
};

console.log(JSON.stringify(result, null, 2));
if (result.status !== 'passed') process.exitCode = 1;
