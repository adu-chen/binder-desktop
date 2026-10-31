#!/usr/bin/env node
// 生成 adus.md 的自动生成区。
// 来源一：设计文档（.md 文件）中的 MODULE / DOMAIN / REQ / REQ_MAP / RULE / CHAIN /
//        CONSTRAINT / TERM / STATE_MACHINE 标注块
// 来源二：源代码（.ts / .tsx 文件）中的 @GOV 标注
// 来源三：triage 报告文件（*_triage.md）
// adus.md 元信息头部（文档头 + 边界说明 + 变更记录，## 【自动生成区】之前的内容）保持不变。
// 用法：npm run governance:generate（或 node scripts/generate_adus.mjs）


import { readFileSync, writeFileSync, readdirSync, statSync, existsSync, mkdirSync } from 'fs';
import { join, relative, dirname, extname, basename } from 'path';
import { fileURLToPath } from 'url';
import {
  buildCoverageMapFromInventory,
  buildTestInventory,
  renderTestCapabilityIndex,
} from './test_governance.mjs';
import {
  auditProductCapabilityRegistry,
  buildCapabilityView,
  loadProductCapabilityRegistry,
  renderProductCapabilityIndex,
} from './product_capability.mjs';
import {
  displayDeferralRecord,
  displayTriageRecord,
  loadTriageRegistry,
} from './triage_registry.mjs';
import {
  activeDocumentMayOwnRule,
  extractDocumentStatus,
  ruleRegistryStatus,
  ruleSourceStatus,
  VALID_RULE_SOURCE_STATUSES,
} from './rule_lifecycle.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT      = join(__dirname, '..');

// ── governance.config.json 读取 ───────────────────────────────────────────
const configPath = join(ROOT, 'governance.config.json');
if (!existsSync(configPath)) {
  console.error('错误：未找到 governance.config.json，请先在项目根目录创建该文件');
  process.exit(1);
}
const govConfig = JSON.parse(readFileSync(configPath, 'utf8'));
if (!govConfig.adus_path || !govConfig.adus_path.trim()) {
  console.error('错误：governance.config.json 中 adus_path 为空，请先完成 bootstrap 流程');
  process.exit(1);
}

const PROJECT_ROOT = join(ROOT, (govConfig.project_root || '.').trim());
const DOCS      = join(PROJECT_ROOT, 'docs');
const SRC       = join(PROJECT_ROOT, 'src');
const SRC_TAURI = join(PROJECT_ROOT, 'src-tauri', 'src'); // Rust 后端源码根
const OUTPUT    = join(ROOT, govConfig.adus_path.trim());

// 自动生成区的起始标记
const AUTO_MARKER = '## 【自动生成区】';

// 跳过的目录（源代码扫描）
const SKIP_DIRS = new Set(['node_modules', 'generated', 'build', 'dist', '__mocks__', 'target']);

// 治理控制文档本身不扫描；APC 是宪章文档、ADUS 是自动生成索引，两者不含业务标注块，同样排除
const GOVERNANCE_FILES = new Set(['ADU.md', 'adu.md', 'ADUS.md', 'adus.md', 'ADP.md', 'AGENTS.md', 'CLAUDE.md']);
const APC_PATH  = govConfig.apc_path  ? govConfig.apc_path.trim()  : null;
const ADUS_PATH = govConfig.adus_path ? govConfig.adus_path.trim() : null;

// ── 文件收集 ──────────────────────────────────────────────────────────────

function collectSourceFiles(dir, results = []) {
  let entries;
  try { entries = readdirSync(dir); } catch { return results; }
  for (const entry of entries) {
    if (SKIP_DIRS.has(entry)) continue;
    const full = join(dir, entry);
    const stat = statSync(full);
    if (stat.isDirectory()) {
      collectSourceFiles(full, results);
    } else if (entry.endsWith('.ts') || entry.endsWith('.tsx') || entry.endsWith('.rs') || entry.endsWith('.mjs')) {
      results.push(full);
    }
  }
  return results;
}

function collectDesignDocs(dir, results = []) {
  let entries;
  try { entries = readdirSync(dir); } catch { return results; }
  for (const entry of entries) {
    if (SKIP_DIRS.has(entry)) continue;
    const full = join(dir, entry);
    const stat = statSync(full);
    if (stat.isDirectory()) {
      // 不递归到 scripts/ 自身
      if (entry !== 'scripts') collectDesignDocs(full, results);
    } else if (entry.endsWith('.md') && !GOVERNANCE_FILES.has(basename(full))) {
      const relFull = relative(ROOT, full);
      // 跳过 triage 报告、APC 宪章和 ADUS 索引
      if (!basename(full).endsWith('_triage.md') && relFull !== APC_PATH && relFull !== ADUS_PATH) results.push(full);
    }
  }
  return results;
}

function collectTriageFiles(dir, results = []) {
  let entries;
  try { entries = readdirSync(dir); } catch { return results; }
  for (const entry of entries) {
    if (SKIP_DIRS.has(entry)) continue;
    const full = join(dir, entry);
    const stat = statSync(full);
    if (stat.isDirectory()) {
      if (entry !== 'scripts') collectTriageFiles(full, results);
    } else if (entry.endsWith('_triage.md')) {
      results.push(full);
    }
  }
  return results;
}

// ── 设计文档标注块解析 ────────────────────────────────────────────────────

function parseAnnotationBlocks(filePath) {
  const src = readFileSync(filePath, 'utf8');
  const documentStatus = extractDocumentStatus(src);
  const rules       = [];
  const chains      = [];
  const constraints = [];
  const modules     = [];
  const domains     = [];
  const terms       = [];
  const requirements = [];
  const requirementMaps = [];
  const stateMachines = [];

  const blockRe = /<!--[ \t]*(MODULE|DOMAIN|REQ_MAP|REQ|RULE|CHAIN|CONSTRAINT|TERM|STATE_MACHINE)[ \t]*\r?\n([\s\S]*?)-->/g;
  let m;
  while ((m = blockRe.exec(src)) !== null) {
    const kind    = m[1];
    const body    = m[2];
    const fields  = parseKVBlock(body);

    if (kind === 'RULE') {
      if (fields['rule_id']) rules.push({
        ...fields,
        documentStatus,
        sourceFile: relative(ROOT, filePath),
      });
    } else if (kind === 'CHAIN') {
      if (fields['chain_id']) chains.push({ ...fields, sourceFile: relative(ROOT, filePath) });
    } else if (kind === 'CONSTRAINT') {
      if (fields['constraint_id']) constraints.push({ ...fields, sourceFile: relative(ROOT, filePath) });
    } else if (kind === 'MODULE') {
      if (fields['module_code']) modules.push({ ...fields, sourceFile: relative(ROOT, filePath) });
    } else if (kind === 'DOMAIN') {
      if (fields['domain_code']) domains.push({ ...fields, sourceFile: relative(ROOT, filePath) });
    } else if (kind === 'TERM') {
      if (fields['term_id']) terms.push({ ...fields, sourceFile: relative(ROOT, filePath) });
    } else if (kind === 'REQ') {
      if (fields['req_id']) requirements.push({ ...fields, sourceFile: relative(ROOT, filePath) });
    } else if (kind === 'REQ_MAP') {
      if (fields['req_id']) requirementMaps.push({ ...fields, sourceFile: relative(ROOT, filePath) });
    } else if (kind === 'STATE_MACHINE') {
      if (fields['machine_id']) stateMachines.push({ ...fields, sourceFile: relative(ROOT, filePath) });
    }
  }
  return { rules, chains, constraints, modules, domains, terms, requirements, requirementMaps, stateMachines };
}

function parseKVBlock(body) {
  const result = {};
  for (const line of body.split('\n')) {
    const idx = line.indexOf(':');
    if (idx === -1) continue;
    const key = line.slice(0, idx).trim();
    const val = line.slice(idx + 1).trim();
    if (key) result[key] = val;
  }
  return result;
}

// ── triage 报告解析 ───────────────────────────────────────────────────────

function parseTriageRecords(filePath) {
  const src = readFileSync(filePath, 'utf8');
  const records = [];
  // 按 codes: 字段拆分记录块
  const blocks = src.split(/(?=^codes:)/m);
  for (const block of blocks) {
    if (!block.trim()) continue;
    const fields = {};
    for (const line of block.split('\n')) {
      const idx = line.indexOf(':');
      if (idx === -1) continue;
      const key = line.slice(0, idx).trim();
      const val = line.slice(idx + 1).trim();
      if (key) fields[key] = val;
    }
    if (fields['codes'] && fields['分类']) {
      records.push({
        codes:    fields['codes'],
        file:     fields['文件'] ?? '',
        type:     fields['分类'],
        resolved: fields['已解决'] ?? '否',
      });
    }
  }
  return records;
}

// ── @GOV 解析 ─────────────────────────────────────────────────────────────

function parseGovBlocks(filePath) {
  const src   = readFileSync(filePath, 'utf8');
  const lines = src.split('\n');
  const blocks = [];

  let inGov = false, govLines = [], startLine = 0;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    if (!inGov && line.includes('@GOV')) {
      inGov = true; govLines = []; startLine = i + 1;
      continue;
    }
    if (inGov) {
      if (line.includes('*/')) {
        const block = parseGovContent(govLines, filePath, startLine);
        if (block) blocks.push(block);
        inGov = false; govLines = [];
      } else {
        govLines.push(line);
      }
    }
  }
  return blocks;
}

function extractField(lines, fieldName) {
  const prefixes = [`* ${fieldName}:`, `*  ${fieldName}:`];
  let value = null, capturing = false;

  for (const line of lines) {
    const t = line.trim();
    if (prefixes.some(p => t.startsWith(p))) {
      value = t.replace(/^\*\s+[\w]+:\s*/, '').trim();
      capturing = true;
      continue;
    }
    if (capturing) {
      const isCont = t.startsWith('*') &&
        !t.match(/^\*\s+(codes|type|chain|rules|boundary)\s*:/);
      if (isCont) value += ' ' + t.replace(/^\*\s*/, '').trim();
      else capturing = false;
    }
  }
  return value;
}

function parseGovContent(lines, filePath, startLine) {
  const codesRaw = extractField(lines, 'codes');
  const type     = extractField(lines, 'type');
  const chainRaw = extractField(lines, 'chain');
  const rulesRaw = extractField(lines, 'rules');

  if (!codesRaw || !type || !chainRaw) return null;

  const chains = chainRaw.split(',').map(s => s.trim()).filter(Boolean);
  const rules  = (rulesRaw && rulesRaw !== '[]')
    ? rulesRaw.split(',').map(s => s.trim()).filter(Boolean)
    : [];

  // codes 字段可能包含多行（续行以逗号结尾），清理空白后合并
  const codesClean = codesRaw.replace(/\s+/g, ' ').trim();

  return {
    codes: codesClean,
    type,
    chains,
    rules,
    file: relative(ROOT, filePath),
    line: startLine,
  };
}

// ── Markdown 表格 ──────────────────────────────────────────────────────────

function mdTable(headers, rows) {
  if (!rows.length) return '';
  const widths = headers.map((h, i) =>
    Math.max(h.length, ...rows.map(r => String(r[i] ?? '').length))
  );
  const pad = (s, w) => String(s ?? '').padEnd(w);
  return [
    `| ${headers.map((h, i) => pad(h, widths[i])).join(' | ')} |`,
    `| ${widths.map(w => '-'.repeat(w)).join(' | ')} |`,
    ...rows.map(r => `| ${r.map((c, i) => pad(c, widths[i])).join(' | ')} |`),
  ].join('\n');
}

// ── 模块编码表 ─────────────────────────────────────────────────────────────

function buildModuleTable(allModules) {
  if (!allModules.length) {
    return `（设计文档中存在 \`<!-- MODULE -->\` 标注块后自动生成）

格式：
\`\`\`
| module_code | module_name | description | 来源文档 |
|-------------|-------------|-------------|---------|
\`\`\``;
  }
  const rows = allModules.map(m => [
    m['module_code'] ?? '',
    m['module_name'] ?? '',
    m['description'] ?? '',
    m['sourceFile'] ?? '',
  ]);
  return mdTable(['module_code', 'module_name', 'description', '来源文档'], rows);
}

// ── 规则域注册表 ───────────────────────────────────────────────────────────

const BUILTIN_DOMAINS = [
  ['STATE',   '状态语义、生命周期、阶段闭合'],
  ['DATA',    '数据结构、字段定义、schema'],
  ['PERSIST', '持久化、恢复、重建'],
  ['VERIFY',  '验证、审计、一致性检查'],
  ['GOV',     '文档治理、命名、变更控制'],
  ['OBS',     '可观测性、错误处理、恢复'],
];

function buildDomainRegistry(allDomains) {
  const rows = [
    ...BUILTIN_DOMAINS.map(([code, desc]) => [code, desc, '内置']),
    ...allDomains.map(d => [d['domain_code'] ?? '', d['description'] ?? '', d['sourceFile'] ?? '']),
  ];
  return mdTable(['domain_code', 'description', '来源'], rows);
}

// ── 需求注册表与需求映射表 ───────────────────────────────────────────────

function buildRequirementRegistry(allRequirements) {
  if (!allRequirements.length) {
    return `（需求文档中存在 \`<!-- REQ -->\` 标注块后自动生成）`;
  }
  const rows = allRequirements.map(requirement => [
    requirement['req_id'] ?? '',
    requirement['title'] ?? '',
    requirement['owner_module'] ?? '',
    requirement['status'] ?? '',
    requirement['summary'] ?? '',
    requirement['sourceFile'] ?? '',
  ]);
  return mdTable(['req_id', 'title', 'owner_module', 'status', 'summary', '来源文档'], rows);
}

function buildRequirementMapRegistry(allRequirementMaps) {
  if (!allRequirementMaps.length) {
    return `（技术设计文档中存在 \`<!-- REQ_MAP -->\` 标注块后自动生成）`;
  }
  const uniqueMappings = new Map();
  for (const mapping of allRequirementMaps) {
    const key = [mapping['req_id'], mapping['target_type'], mapping['target_id'], mapping['status']].join('|');
    if (!uniqueMappings.has(key)) uniqueMappings.set(key, { ...mapping, sourceFiles: new Set() });
    uniqueMappings.get(key).sourceFiles.add(mapping['sourceFile'] ?? '');
  }
  const rows = [...uniqueMappings.values()].map(mapping => [
    mapping['req_id'] ?? '',
    mapping['target_type'] ?? '',
    mapping['target_id'] ?? '',
    mapping['status'] ?? '',
    mapping['reason'] ?? '',
    [...mapping.sourceFiles].filter(Boolean).join(', '),
  ]);
  return mdTable(['req_id', 'target_type', 'target_id', 'status', 'reason', '来源文档'], rows);
}

// ── 规则注册表 ─────────────────────────────────────────────────────────────

function buildRuleRegistry(allRules, coverageMap = new Map()) {
  if (!allRules.length) {
    return `（设计文档中存在 \`<!-- RULE -->\` 标注块后自动生成）

格式：
\`\`\`
| rule_id | 域 | 主链路 | 来源文档 | registry_status | 测试覆盖 |
|---------|-----|-------|---------|----------------|---------|
\`\`\``;
  }
  const rows = allRules.map(r => {
    const ruleId  = r['rule_id'] ?? '';
    const entries = coverageMap.get(ruleId);
    const coverageStr = entries ? entries.join(', ') : 'MISSING';
    return [
      ruleId,
      r['域'] ?? r['domain'] ?? '',
      r['主链路'] ?? r['main_chain'] ?? '',
      r['sourceFile'] ?? '',
      ruleRegistryStatus(r),
      ruleRegistryStatus(r) === 'superseded' ? 'N/A (superseded)' : coverageStr,
    ];
  });
  return mdTable(['rule_id', '域', '主链路', '来源文档', 'registry_status', '测试覆盖'], rows);
}

// ── 链路注册表 ─────────────────────────────────────────────────────────────

function buildChainRegistry(allChains) {
  if (!allChains.length) {
    return `（设计文档中存在 \`<!-- CHAIN -->\` 标注块后自动生成）

格式：
\`\`\`
| chain_id | 主责模块 | status | 集成测试 |
|----------|---------|--------|---------|
\`\`\``;
  }
  const rows = allChains.map(c => [
    c['chain_id'] ?? '',
    c['主责模块'] ?? c['owner_module'] ?? '',
    c['status'] ?? 'active',
    c['integration_test'] ?? c['集成测试'] ?? c['integration_tests'] ?? '',
  ]);
  return mdTable(['chain_id', '主责模块', 'status', '集成测试'], rows);
}

// ── 跨模块约束表 ───────────────────────────────────────────────────────────

function buildConstraintTable(allConstraints) {
  if (!allConstraints.length) {
    return `（设计文档中存在 \`<!-- CONSTRAINT -->\` 标注块后自动生成）

格式：
\`\`\`
| constraint_id | 摘要 | 涉及模块 | 链路影响 | 来源文档 |
|---------------|------|---------|---------|---------|
\`\`\``;
  }
  const rows = allConstraints.map(c => [
    c['constraint_id'] ?? '',
    c['摘要'] ?? c['summary'] ?? '',
    c['涉及模块'] ?? c['modules'] ?? '',
    c['链路影响'] ?? c['chains'] ?? '',
    c['sourceFile'] ?? '',
  ]);
  return mdTable(['constraint_id', '摘要', '涉及模块', '链路影响', '来源文档'], rows);
}

// ── 术语注册表 ─────────────────────────────────────────────────────────────

function buildTermRegistry(allTerms) {
  if (!allTerms.length) {
    return `（设计文档中存在 \`<!-- TERM -->\` 标注块后自动生成）

格式：
\`\`\`
| term_id | 所属链路 | 正式中文名 | 正式英文名 | 禁用别名 | 来源文档 |
|---------|---------|-----------|-----------|---------|---------|
\`\`\``;
  }
  const rows = allTerms.map(t => [
    t['term_id'] ?? '',
    t['chains'] ?? t['chain'] ?? '',
    t['zh'] ?? '',
    t['en'] ?? '',
    t['forbidden'] ?? '',
    t['sourceFile'] ?? '',
  ]);
  return mdTable(['term_id', '所属链路', '正式中文名', '正式英文名', '禁用别名', '来源文档'], rows);
}

// ── 状态机注册表 ─────────────────────────────────────────────────────────

function buildStateMachineRegistry(allStateMachines) {
  if (!allStateMachines.length) {
    return `（设计文档中存在 \`<!-- STATE_MACHINE -->\` 标注块后自动生成）`;
  }
  const rows = allStateMachines.map(machine => [
    machine['machine_id'] ?? '',
    machine['owner_module'] ?? '',
    machine['status'] ?? '',
    machine['source_type'] ?? '',
    machine['chain'] ?? '',
    machine['rules'] ?? '',
    machine['initial_state'] ?? '',
    machine['terminal_states'] ?? '',
    machine['sourceFile'] ?? '',
  ]);
  return mdTable([
    'machine_id', 'owner_module', 'status', 'source_type', 'chain', 'rules',
    'initial_state', 'terminal_states', '来源文档',
  ], rows);
}

// ── 链路视图 ───────────────────────────────────────────────────────────────

function buildChainView(govBlocks, knownChainIds) {
  const chainMap = new Map();
  const unknownChains = new Set();

  for (const b of govBlocks) {
    const primaryChain = b.chains[0] ?? 'UNKNOWN';
    const consumerChains = b.chains.slice(1);
    if (!chainMap.has(primaryChain)) chainMap.set(primaryChain, []);
    chainMap.get(primaryChain).push({ block: b, consumerChains });
    for (const chain of b.chains) {
      if (knownChainIds.size > 0 && !knownChainIds.has(chain)) {
        unknownChains.add(chain);
      }
    }
  }

  if (!chainMap.size) {
    return `（源代码中存在 @GOV 标注后自动生成）

格式：
\`\`\`
#### [CHAIN-ID]

| codes | type | rules | 文件 | 行号 |
|-------|------|-------|------|------|
\`\`\``;
  }

  const parts = [...chainMap.keys()].sort().map(chainId => {
    const rows = chainMap.get(chainId).map(({ block: b, consumerChains }) => [
      b.codes,
      b.type,
      b.rules.length ? b.rules.join(', ') : '[]',
      b.file,
      String(b.line),
      consumerChains.length ? consumerChains.join(', ') : '-',
    ]);
    const warning = unknownChains.has(chainId)
      ? `\n> ⚠ P1：chain_id \`${chainId}\` 未在链路注册表中找到对应 <!-- CHAIN --> 标注块\n`
      : '';
    return `#### ${chainId}${warning}\n\n` + mdTable(['codes', 'type', 'rules', '文件', '行号', 'consumer_chains'], rows);
  });

  return parts.join('\n\n');
}

// ── 规则视图 ───────────────────────────────────────────────────────────────

function buildRuleView(govBlocks) {
  const ruleMap = new Map();
  for (const b of govBlocks) {
    for (const rule of b.rules) {
      if (!ruleMap.has(rule)) ruleMap.set(rule, []);
      ruleMap.get(rule).push(b);
    }
  }

  if (!ruleMap.size) {
    return `（源代码中存在 @GOV 标注后自动生成）

格式：
\`\`\`
#### [rule_id]

| codes | type | chain | 文件 | 行号 |
|-------|------|-------|------|------|
\`\`\``;
  }

  return [...ruleMap.keys()].sort().map(ruleId => {
    const rows = ruleMap.get(ruleId).map(b => [
      b.codes,
      b.type,
      b.chains.join(', '),
      b.file,
      String(b.line),
    ]);
    return `#### ${ruleId}\n\n` + mdTable(['codes', 'type', 'chain', '文件', '行号'], rows);
  }).join('\n\n');
}

// ── triage / 标注延期汇总 ─────────────────────────────────────────────────

function buildTriageSummary(records) {
  if (!records.length) return '（暂无有效 triage 记录）';
  const rows = records.map(displayTriageRecord).map(record => [
    record.codes, record.file, record.classification, record.resolved,
  ]);
  return mdTable(['codes', '文件', '分类', '已解决'], rows);
}

function buildAnnotationDeferralSummary(records) {
  if (!records.length) return '（暂无有效标注延期记录）';
  const rows = records.map(displayDeferralRecord).map(record => [
    record.symbol, record.file, record.plan, record.reviewDate, record.resolved,
  ]);
  return mdTable(['符号', '文件', '承接计划', '复核日期', '已解决'], rows);
}

// ── timestamp churn 防护 ──────────────────────────────────────────────────

function contentChanged(oldContent, newContent) {
  const strip = s => s.split('\n')
    .filter(line => !line.includes('最后生成') && !line.includes('Last generated'))
    .join('\n');
  return strip(oldContent) !== strip(newContent);
}

// ── 主流程 ─────────────────────────────────────────────────────────────────

function main() {
  // 1. 扫描源代码 @GOV 块（前端 src/ 的 .ts/.tsx + Rust 后端 src-tauri/src/ 的 .rs）
  const srcFiles  = [...collectSourceFiles(SRC), ...collectSourceFiles(SRC_TAURI), ...collectSourceFiles(join(PROJECT_ROOT, 'scripts'))];
  const govBlocks = [];
  for (const f of srcFiles) govBlocks.push(...parseGovBlocks(f));

  // 2. 扫描设计文档标注块
  const docFiles = collectDesignDocs(DOCS);
  const allRules = [], allChains = [], allConstraints = [], allModules = [], allDomains = [], allTerms = [];
  const allRequirements = [], allRequirementMaps = [], allStateMachines = [];
  for (const f of docFiles) {
    const {
      rules, chains, constraints, modules, domains, terms,
      requirements, requirementMaps, stateMachines,
    } = parseAnnotationBlocks(f);
    allRules.push(...rules);
    allChains.push(...chains);
    allConstraints.push(...constraints);
    allModules.push(...modules);
    allDomains.push(...domains);
    allTerms.push(...terms);
    allRequirements.push(...requirements);
    allRequirementMaps.push(...requirementMaps);
    allStateMachines.push(...stateMachines);
  }

  for (const rule of allRules) {
    const sourceStatus = ruleSourceStatus(rule);
    if (!VALID_RULE_SOURCE_STATUSES.has(sourceStatus)) {
      throw new Error(`RULE_SOURCE_STATUS_INVALID: ${rule.rule_id} in ${rule.sourceFile} has status=${sourceStatus}`);
    }
    if (!activeDocumentMayOwnRule(rule.documentStatus, rule)) {
      throw new Error(
        `RULE_SOURCE_DOCUMENT_INACTIVE: ${rule.rule_id} in ${rule.sourceFile} is active but document status is ${rule.documentStatus || 'missing'}`,
      );
    }
  }

  // 3. 扫描正式 triage 与标注延期记录。两者与 audit 共享严格解析器。
  const triageRegistry = loadTriageRegistry(ROOT);
  if (triageRegistry.findings.length) {
    const messages = triageRegistry.findings.map(finding => `${finding.type}: ${finding.message}`).join('\n');
    throw new Error(`triage registry is invalid:\n${messages}`);
  }

  // 4. 动态发现前端、Rust 内联与 Rust 集成测试，并派生 covers 与模块能力索引。
  const testInventory = buildTestInventory(ROOT);
  const coverageMap = buildCoverageMapFromInventory(testInventory);
  const testCapabilityIndex = renderTestCapabilityIndex(testInventory);
  const productCapability = loadProductCapabilityRegistry(ROOT);
  const productCapabilityAudit = auditProductCapabilityRegistry(ROOT, productCapability.registry);
  if (productCapabilityAudit.status !== 'passed') {
    const messages = productCapabilityAudit.findings.map(finding => `${finding.type}: ${finding.message}`).join('\n');
    throw new Error(`product capability registry is invalid:\n${messages}`);
  }
  const productCapabilityIndex = renderProductCapabilityIndex(buildCapabilityView(productCapability.registry), productCapabilityAudit);

  const timestamp       = new Date().toISOString();
  const knownChainIds   = new Set(allChains.map(c => c['chain_id']).filter(Boolean));
  const moduleTable     = buildModuleTable(allModules);
  const domainRegistry  = buildDomainRegistry(allDomains);
  const requirementRegistry = buildRequirementRegistry(allRequirements);
  const requirementMapRegistry = buildRequirementMapRegistry(allRequirementMaps);
  const chainRegistry   = buildChainRegistry(allChains);
  const ruleRegistry    = buildRuleRegistry(allRules, coverageMap);
  const constraintTable = buildConstraintTable(allConstraints);
  const termRegistry    = buildTermRegistry(allTerms);
  const stateMachineRegistry = buildStateMachineRegistry(allStateMachines);
  const chainView       = buildChainView(govBlocks, knownChainIds);
  const ruleView        = buildRuleView(govBlocks);
  const triageSummary   = buildTriageSummary(triageRegistry.triageRecords);
  const annotationDeferralSummary = buildAnnotationDeferralSummary(triageRegistry.deferralRecords);

  // 5. 读取现有 adus.md，提取元信息头部（自动生成区之前的内容）
  let metadataHeader = '';
  let existingContent = '';
  if (existsSync(OUTPUT)) {
    existingContent = readFileSync(OUTPUT, 'utf8');
    const markerIdx = existingContent.indexOf('\n' + AUTO_MARKER);
    if (markerIdx !== -1) {
      metadataHeader = existingContent.slice(0, markerIdx);
    } else {
      metadataHeader = existingContent;
    }
  }

  // 首次生成时的默认模板
  if (!metadataHeader.trim()) {
    const today = new Date().toISOString().slice(0, 10);
    metadataHeader = `---
文档编号：CORE-GOV-01
文档状态：A
负责模块：CORE
文档职责：ADU 派生检索索引
上游约束：CORE-GOV-00
直接承接：ADU / ADP / CLAUDE 的规则、链路、代码块检索
使用边界：不定义治理规则，不定义开发过程，不替代 adu.md、ADP.md 或 CLAUDE.md
变更要求：运行 npm run governance:generate 刷新自动生成区
---

# ADU 派生检索索引

本文件由 \`governance:generate\` 自动生成。
所有索引内容来自设计文档标注块、源代码 @GOV 标注、测试覆盖注释和 triage 报告。
运行 \`npm run governance:generate\` 刷新。

边界：
- adu.md 是静态治理规则源。
- ADP.md 是开发过程运行时协议。
- CLAUDE.md 是 AI 会话入口与执行路由协议。
- adus.md 只提供派生检索索引，不定义规则、不定义过程、不承载人类裁决。

自动生成边界：
- \`## 【自动生成区】\` 之前仅保留元信息头部，包括文档头、边界说明和变更记录。
- 元信息头部不得写入规则、链路、模块、约束、裁决或索引内容。
- \`## 【自动生成区】\` 及之后内容禁止人工编辑，应由 \`npm run governance:generate\` 刷新。

---

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| ${today} | v1.0 | 初始版本 |`;
  }

  // 5. 去掉头部末尾所有分隔线，避免多次运行后重复堆叠
  const trimmedMetadataHeader = metadataHeader.trimEnd().replace(/(\n\s*---\s*)+$/, '');

  // 6. 拼装完整输出
  const autoArea = `
## 【自动生成区】

<!-- 自动生成。禁止人工编辑。-->
<!-- 运行：npm run governance:generate -->
<!-- 最后生成时间：${timestamp} -->

### 模块编码表

${moduleTable}

### 规则域注册表

${domainRegistry}

### 需求注册表

${requirementRegistry}

### 需求映射表

${requirementMapRegistry}

### 链路注册表

${chainRegistry}

### 规则注册表

${ruleRegistry}

### 跨模块约束表（X-INDEX）

${constraintTable}

### 术语注册表

${termRegistry}

### 状态机注册表

${stateMachineRegistry}

### 链路视图

${chainView}

### 规则视图

${ruleView}

### 测试能力索引

${testCapabilityIndex}

### 产品能力索引

${productCapabilityIndex}

### triage 汇总

${triageSummary}

### 标注延期汇总

${annotationDeferralSummary}
`;

  const newContent = trimmedMetadataHeader + '\n\n---\n' + autoArea;
  const stats = [
    `@GOV 块: ${govBlocks.length}`,
    `模块标注: ${allModules.length}`,
    `域标注: ${allDomains.length}`,
    `需求标注: ${allRequirements.length}`,
    `需求映射: ${allRequirementMaps.length}`,
    `规则标注: ${allRules.length}`,
    `链路标注: ${allChains.length}`,
    `约束标注: ${allConstraints.length}`,
    `triage 记录: ${triageRegistry.triageRecords.length}`,
    `标注延期: ${triageRegistry.deferralRecords.length}`,
    `术语标注: ${allTerms.length}`,
    `状态机标注: ${allStateMachines.length}`,
    `测试资产: ${testInventory.totals.assets}`,
    `测试用例: ${testInventory.totals.tests}`,
  ].join('，');

  if (existingContent && !contentChanged(existingContent, newContent)) {
    console.log(`adus.md 内容无变化，跳过写入 — ${stats}`);
    return;
  }
  mkdirSync(dirname(OUTPUT), { recursive: true });
  writeFileSync(OUTPUT, newContent, 'utf8');
  console.log(`adus.md 已生成 — ${stats} — ${timestamp}`);
}

main();
