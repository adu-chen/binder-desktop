#!/usr/bin/env node

import {
  existsSync,
  readFileSync,
  readdirSync,
  statSync,
} from 'node:fs';
import { execFileSync, spawnSync } from 'node:child_process';
import { dirname, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url));
export const DEFAULT_ROOT = resolve(SCRIPT_DIR, '..');
const FRONTEND_TEST_RE = /\.(?:test|spec)\.[cm]?[jt]sx?$/;
const MODULE_FROM_RULE_RE = /\bBR-([A-Z]+)-[A-Z]+-[0-9]+\b/g;
const COVER_RE = /^\s*\/\/\s*covers:\s*(\S+)\s*$/gm;
const FOCUSED_OR_SKIPPED_RE = /(?:\b(?:describe|it|test)(?:\.concurrent)?\.(?:only|skip|todo)|\b(?:xdescribe|xit|xtest))\s*\(/;
const RUST_IGNORED_RE = /^\s*#\s*\[ignore(?:\([^\]]*\))?\]/m;

function normalizePath(value) {
  return value.split(sep).join('/').replace(/^\.\//, '').replace(/\/$/, '');
}

function unique(values) {
  return [...new Set(values.filter(Boolean))];
}

function walkFiles(root, relativeRoot, skipDirectories, results = []) {
  const absoluteRoot = join(root, relativeRoot);
  let entries;
  try {
    entries = readdirSync(absoluteRoot);
  } catch {
    return results;
  }
  for (const entry of entries) {
    if (skipDirectories.has(entry)) continue;
    const absolutePath = join(absoluteRoot, entry);
    const relativePath = normalizePath(relative(root, absolutePath));
    let stat;
    try {
      stat = statSync(absolutePath);
    } catch {
      continue;
    }
    if (stat.isDirectory()) walkFiles(root, relativePath, skipDirectories, results);
    else if (stat.isFile()) results.push(relativePath);
  }
  return results;
}

export function globToRegExp(glob) {
  const normalized = normalizePath(glob);
  let source = '^';
  for (let index = 0; index < normalized.length; index += 1) {
    const char = normalized[index];
    if (char === '*') {
      if (normalized[index + 1] === '*') {
        index += 1;
        if (normalized[index + 1] === '/') {
          index += 1;
          source += '(?:.*/)?';
        } else {
          source += '.*';
        }
      } else {
        source += '[^/]*';
      }
    } else if (char === '?') {
      source += '[^/]';
    } else {
      source += char.replace(/[|\\{}()[\]^$+?.]/g, '\\$&');
    }
  }
  return new RegExp(`${source}$`);
}

export function matchesGlob(filePath, glob) {
  return globToRegExp(glob).test(normalizePath(filePath));
}

function matchesAny(filePath, globs = []) {
  return globs.some((glob) => matchesGlob(filePath, glob));
}

export function loadTestGovernance(root = DEFAULT_ROOT) {
  const governancePath = join(root, 'governance.config.json');
  if (!existsSync(governancePath)) throw new Error('governance.config.json 不存在');
  const governance = JSON.parse(readFileSync(governancePath, 'utf8'));
  const registryRelative = governance.test_governance_path;
  if (!registryRelative) throw new Error('governance.config.json 缺少 test_governance_path');
  const registryPath = join(root, registryRelative);
  if (!existsSync(registryPath)) throw new Error(`测试治理注册表不存在: ${registryRelative}`);
  const registry = JSON.parse(readFileSync(registryPath, 'utf8'));
  return { governance, registry, registryPath, registryRelative };
}

export function extractCovers(source) {
  return [...source.matchAll(COVER_RE)].map((match) => match[1]);
}

export function extractCoverCases(source, kind) {
  const lines = source.split(/\r?\n/);
  const cases = [];
  const pending = [];
  let awaitingRustTest = false;
  for (const line of lines) {
    const trimmed = line.trim();
    const cover = /^\/\/\s*covers:\s*(\S+)/.exec(trimmed);
    if (cover) {
      pending.push(cover[1]);
      continue;
    }
    if (kind === 'rust_inline' || kind === 'rust_integration') {
      if (/^#\s*\[\s*(?:tokio::)?test\s*\]/.test(trimmed)) {
        awaitingRustTest = true;
        continue;
      }
      if (awaitingRustTest) {
        const fn = /^\s*(?:pub\s+)?(?:async\s+)?fn\s+([A-Za-z0-9_]+)/.exec(line);
        if (fn) {
          for (const rule of pending) cases.push({ rule, test: fn[1] });
          pending.length = 0;
          awaitingRustTest = false;
        }
      }
      continue;
    }
    const it = /^\s*(?:it|test)(?:\.(?:each|concurrent))?\s*\(\s*(['"])(.+?)\1/.exec(line);
    if (it) {
      for (const rule of pending) cases.push({ rule, test: it[2] });
      pending.length = 0;
    }
  }
  for (const rule of pending) cases.push({ rule, test: null });
  return cases;
}

export function extractRuleModules(source) {
  return unique([...source.matchAll(MODULE_FROM_RULE_RE)].map((match) => match[1]));
}

function modulesMatchingPath(filePath, registry, kind = 'source') {
  const matched = [];
  for (const module of registry.modules ?? []) {
    const includes = kind === 'test' ? module.test_globs : module.source_globs;
    const excludes = kind === 'test' ? module.test_exclude_globs : module.source_exclude_globs;
    if (matchesAny(filePath, includes) && !matchesAny(filePath, excludes)) matched.push(module.code);
  }
  return matched;
}

function modulesForContent(root, filePath) {
  const absolutePath = join(root, filePath);
  if (!existsSync(absolutePath)) return [];
  try {
    return extractRuleModules(readFileSync(absolutePath, 'utf8'));
  } catch {
    return [];
  }
}

export function inferModulesForPath(root, registry, filePath, kind = 'source') {
  const normalized = normalizePath(filePath);
  const pathModules = modulesMatchingPath(normalized, registry, kind);
  const contentModules = /^(?:src\/|src-tauri\/|docs\/01_modules\/)/.test(normalized)
    ? modulesForContent(root, normalized)
    : [];
  const docsMatch = /^docs\/01_modules\/([A-Z]+)\//.exec(normalized);
  return unique([...pathModules, ...(docsMatch ? [docsMatch[1]] : []), ...contentModules]);
}

function classifyLayer(filePath, kind, registry) {
  if (kind === 'rust_integration') return 'integration';
  if (kind === 'rust_inline') return 'unit';
  const rule = (registry.layer_rules ?? []).find((entry) => matchesAny(filePath, entry.globs));
  return rule?.layer ?? 'unclassified';
}

function countTests(source, kind) {
  if (kind === 'rust_inline' || kind === 'rust_integration') {
    return (source.match(/^\s*#\s*\[test\]/gm) ?? []).length;
  }
  return (source.match(/^\s*(?:it|test)(?:\.(?:each|concurrent))?\s*\(/gm) ?? []).length;
}

function buildAsset(root, registry, filePath, kind) {
  const source = readFileSync(join(root, filePath), 'utf8');
  const covers = extractCovers(source);
  const coverModules = unique(covers.map((rule) => /^BR-([A-Z]+)-/.exec(rule)?.[1]));
  const testPathModules = modulesMatchingPath(filePath, registry, 'test');
  const sourcePathModules = modulesMatchingPath(filePath, registry, 'source');
  // 治理自测中的规则编号是测试数据，不得反向污染业务模块归属。
  const contentModules = filePath.startsWith('scripts/') ? [] : extractRuleModules(source);
  const modules = unique([...testPathModules, ...sourcePathModules, ...coverModules, ...contentModules]);
  return {
    path: filePath,
    kind,
    layer: classifyLayer(filePath, kind, registry),
    owner: testPathModules[0] ?? sourcePathModules[0] ?? coverModules[0] ?? contentModules[0] ?? null,
    modules,
    covers: unique(covers),
    cover_cases: extractCoverCases(source, kind),
    test_count: countTests(source, kind),
    focused_or_skipped: FOCUSED_OR_SKIPPED_RE.test(source),
    rust_ignored: kind.startsWith('rust_') && RUST_IGNORED_RE.test(source),
  };
}

export function buildTestInventory(root = DEFAULT_ROOT, providedRegistry = null) {
  const registry = providedRegistry ?? loadTestGovernance(root).registry;
  const skipDirectories = new Set(registry.discovery?.skip_directories ?? []);
  const frontendFiles = unique((registry.discovery?.frontend_roots ?? [])
    .flatMap((entry) => walkFiles(root, entry, skipDirectories)))
    .filter((file) => FRONTEND_TEST_RE.test(file));
  const rustSourceFiles = walkFiles(root, registry.discovery.rust_source_root, skipDirectories)
    .filter((file) => file.endsWith('.rs'));
  const rustInlineFiles = rustSourceFiles.filter((file) => {
    try {
      return /#!?\s*\[cfg\s*\(\s*test\s*\)\s*\]/.test(readFileSync(join(root, file), 'utf8'));
    } catch {
      return false;
    }
  });
  const rustIntegrationFiles = walkFiles(root, registry.discovery.rust_integration_root, skipDirectories)
    .filter((file) => file.endsWith('.rs'));
  const assets = [
    ...frontendFiles.map((file) => buildAsset(root, registry, file, 'frontend')),
    ...rustInlineFiles.map((file) => buildAsset(root, registry, file, 'rust_inline')),
    ...rustIntegrationFiles.map((file) => buildAsset(root, registry, file, 'rust_integration')),
  ].sort((a, b) => a.path.localeCompare(b.path));

  const modules = {};
  for (const module of registry.modules ?? []) {
    const related = assets.filter((asset) => asset.modules.includes(module.code));
    const ownedFrontend = assets.filter((asset) => asset.kind === 'frontend' && asset.owner === module.code);
    modules[module.code] = {
      risk: module.risk,
      assets: related.length,
      tests: related.reduce((sum, asset) => sum + asset.test_count, 0),
      frontend_owned: ownedFrontend.length,
      layers: Object.fromEntries(unique(related.map((asset) => asset.layer)).sort().map((layer) => [
        layer,
        related.filter((asset) => asset.layer === layer).length,
      ])),
      baseline_frontend_test_files: module.baseline_frontend_test_files,
      target_frontend_test_files: module.target_frontend_test_files,
    };
  }
  return {
    schema_version: registry.schema_version,
    totals: {
      assets: assets.length,
      frontend_files: frontendFiles.length,
      rust_inline_files: rustInlineFiles.length,
      rust_integration_files: rustIntegrationFiles.length,
      tests: assets.reduce((sum, asset) => sum + asset.test_count, 0),
    },
    modules,
    system_validation: registry.system_validation ?? [],
    assets,
  };
}

function collectRegisteredRules(root, registry) {
  const skipDirectories = new Set(registry.discovery?.skip_directories ?? []);
  const docs = walkFiles(root, 'docs', skipDirectories).filter((file) => file.endsWith('.md'));
  const rules = new Set();
  for (const file of docs) {
    const source = readFileSync(join(root, file), 'utf8');
    for (const match of source.matchAll(/^\s*(?:rule_id|规则编号)\s*:\s*(BR-[A-Z]+-[A-Z]+-[0-9]+)\s*$/gm)) {
      rules.add(match[1]);
    }
  }
  return rules;
}

function collectExecutableSources(root, registry, inventory) {
  const skipDirectories = new Set(registry.discovery?.skip_directories ?? []);
  const testPaths = new Set(inventory.assets.map((asset) => asset.path));
  return walkFiles(root, '.', skipDirectories)
    .filter((file) => matchesAny(file, registry.policy?.executable_globs))
    .filter((file) => !testPaths.has(file))
    .filter((file) => !/\.d\.ts$/.test(file));
}

export function auditTestGovernance(root = DEFAULT_ROOT, providedRegistry = null, providedInventory = null) {
  const registry = providedRegistry ?? loadTestGovernance(root).registry;
  const inventory = providedInventory ?? buildTestInventory(root, registry);
  const findings = [];
  const warnings = [];
  const moduleCodes = new Set();
  if (registry.schema_version !== 1) findings.push({ type: 'TEST_REGISTRY_SCHEMA', message: 'schema_version 必须为 1' });
  for (const module of registry.modules ?? []) {
    if (!module.code || moduleCodes.has(module.code)) {
      findings.push({ type: 'TEST_MODULE_DUPLICATE', module: module.code ?? null });
    }
    moduleCodes.add(module.code);
    if (!['critical', 'high', 'medium', 'low'].includes(module.risk)) {
      findings.push({ type: 'TEST_MODULE_RISK_INVALID', module: module.code, risk: module.risk });
    }
  }
  for (const module of registry.modules ?? []) {
    if (!Array.isArray(module.source_globs) || module.source_globs.length === 0) {
      findings.push({ type: 'TEST_MODULE_SOURCE_GLOBS_MISSING', module: module.code });
    }
    for (const downstream of module.downstream ?? []) {
      if (!moduleCodes.has(downstream)) {
        findings.push({ type: 'TEST_MODULE_DOWNSTREAM_UNKNOWN', module: module.code, downstream });
      }
    }
    if (!Number.isInteger(module.baseline_frontend_test_files)
      || !Number.isInteger(module.target_frontend_test_files)
      || module.target_frontend_test_files > module.baseline_frontend_test_files
      || module.target_frontend_test_files < 0) {
      findings.push({ type: 'TEST_MODULE_FILE_BUDGET_INVALID', module: module.code });
    }
  }
  for (const asset of inventory.assets) {
    if (!asset.owner || asset.modules.length === 0) {
      findings.push({ type: 'TEST_OWNER_MISSING', file: asset.path });
    }
    if (asset.layer === 'unclassified') {
      findings.push({ type: 'TEST_LAYER_MISSING', file: asset.path });
    }
    if (registry.policy?.forbid_focused_or_skipped_tests && asset.focused_or_skipped) {
      findings.push({ type: 'TEST_FOCUSED_OR_SKIPPED', file: asset.path });
    }
    if (!registry.policy?.allow_rust_ignore && asset.rust_ignored) {
      findings.push({ type: 'TEST_RUST_IGNORED', file: asset.path });
    }
  }
  const registeredRules = collectRegisteredRules(root, registry);
  for (const asset of inventory.assets) {
    for (const rule of asset.covers) {
      if (!registeredRules.has(rule)) findings.push({ type: 'TEST_COVERS_UNKNOWN_RULE', file: asset.path, rule });
    }
  }
  for (const module of registry.modules ?? []) {
    const summary = inventory.modules[module.code];
    const current = summary?.frontend_owned ?? 0;
    if (current > module.baseline_frontend_test_files) {
      findings.push({
        type: 'TEST_FILE_BUDGET_EXCEEDED',
        module: module.code,
        current,
        baseline: module.baseline_frontend_test_files,
      });
    }
    if (current > module.target_frontend_test_files) {
      warnings.push({
        type: 'TEST_FILE_CONSOLIDATION_DEBT',
        module: module.code,
        current,
        target: module.target_frontend_test_files,
      });
    }
  }
  const executableSources = collectExecutableSources(root, registry, inventory);
  for (const file of executableSources) {
    if (inferModulesForPath(root, registry, file, 'source').length === 0) {
      findings.push({ type: 'TEST_SOURCE_OWNER_MISSING', file });
    }
  }
  const validationIds = new Set();
  for (const validation of registry.system_validation ?? []) {
    if (!validation.id || validationIds.has(validation.id)) {
      findings.push({ type: 'TEST_SYSTEM_VALIDATION_ID_INVALID', id: validation.id ?? null });
    }
    validationIds.add(validation.id);
    if (!['G5', 'G6'].includes(validation.phase)) {
      findings.push({ type: 'TEST_SYSTEM_VALIDATION_PHASE_INVALID', id: validation.id, phase: validation.phase });
    }
    if (!['automated', 'partial', 'manual'].includes(validation.automation)) {
      findings.push({ type: 'TEST_SYSTEM_VALIDATION_AUTOMATION_INVALID', id: validation.id });
    }
    if (!['active', 'deferred', 'blocked', 'passed'].includes(validation.status)) {
      findings.push({ type: 'TEST_SYSTEM_VALIDATION_STATUS_INVALID', id: validation.id });
    }
    for (const module of validation.modules ?? []) {
      if (!moduleCodes.has(module)) {
        findings.push({ type: 'TEST_SYSTEM_VALIDATION_MODULE_UNKNOWN', id: validation.id, module });
      }
    }
    for (const evidence of validation.automated_evidence ?? []) {
      if (!existsSync(join(root, evidence))) {
        findings.push({ type: 'TEST_SYSTEM_VALIDATION_EVIDENCE_MISSING', id: validation.id, evidence });
      }
    }
    if (validation.automation !== 'automated' && !validation.manual_evidence) {
      findings.push({ type: 'TEST_SYSTEM_VALIDATION_MANUAL_EVIDENCE_MISSING', id: validation.id });
    }
  }
  return {
    status: findings.length === 0 ? 'passed' : 'failed',
    checks: {
      assets_discovered: inventory.totals.assets,
      frontend_files: inventory.totals.frontend_files,
      rust_inline_files: inventory.totals.rust_inline_files,
      rust_integration_files: inventory.totals.rust_integration_files,
      tests_discovered: inventory.totals.tests,
      executable_sources_checked: executableSources.length,
      unmapped_executable_sources: findings.filter((entry) => entry.type === 'TEST_SOURCE_OWNER_MISSING').length,
      unowned_tests: findings.filter((entry) => entry.type === 'TEST_OWNER_MISSING').length,
      focused_or_skipped: findings.filter((entry) => entry.type === 'TEST_FOCUSED_OR_SKIPPED').length,
      system_validations: (registry.system_validation ?? []).length,
    },
    findings,
    warnings,
    inventory,
  };
}

function moduleMap(registry) {
  return new Map((registry.modules ?? []).map((module) => [module.code, module]));
}

export function expandAffectedModules(directModules, registry) {
  const byCode = moduleMap(registry);
  const expanded = new Set(directModules);
  for (const code of directModules) {
    const module = byCode.get(code);
    if (!module || !['critical', 'high'].includes(module.risk)) continue;
    for (const downstream of module.downstream ?? []) expanded.add(downstream);
  }
  return [...expanded].sort();
}

function isIgnoredChange(filePath, registry) {
  return matchesAny(filePath, registry.discovery?.ignored_change_globs);
}

function isExecutableChange(filePath, registry) {
  return matchesAny(filePath, registry.policy?.executable_globs);
}

function isProductCapabilityChange(filePath) {
  return filePath === '.governance/product-capability-registry.json' ||
    filePath === 'governance.config.json' ||
    filePath === 'docs/00_core/A-CORE-C-P-02_产品能力图谱规范.md' ||
    filePath === 'docs/40_validation/A-VAL-X-V-03_产品能力图谱验证.md' ||
    /^scripts\/(?:product_capability|generate_product_capability_atlas|audit_product_capability)(?:\.test)?\.mjs$/.test(filePath);
}

function command(id, program, args, reason) {
  return { id, program, args, display: [program, ...args].join(' '), reason };
}

export function buildRelatedTestPlan(
  root = DEFAULT_ROOT,
  changedFiles = [],
  providedRegistry = null,
  providedInventory = null,
) {
  const registry = providedRegistry ?? loadTestGovernance(root).registry;
  const inventory = providedInventory ?? buildTestInventory(root, registry);
  const normalizedChanges = unique(changedFiles.map(normalizePath)).filter((file) => !isIgnoredChange(file, registry));
  // Rust inline tests share a file with production code. They are inventory assets, but changing
  // that file is still a production-source change and must not take the TEST_ONLY fast path.
  const directTestPaths = new Set(inventory.assets
    .filter((asset) => normalizedChanges.includes(asset.path) && asset.kind !== 'rust_inline')
    .map((asset) => asset.path));
  const nonTestChanges = normalizedChanges.filter((file) => !directTestPaths.has(file));
  const directModules = new Set();
  const mapping = [];
  const unmappedExecutable = [];
  let governanceChanged = false;
  let productCapabilityChanged = false;
  let frontendFull = false;
  let rustFull = false;
  let typescriptChanged = false;

  for (const file of normalizedChanges) {
    const asset = inventory.assets.find((candidate) => candidate.path === file);
    if (asset && asset.kind !== 'rust_inline') {
      asset.modules.forEach((module) => directModules.add(module));
      mapping.push({ file, modules: asset.modules, source: 'direct_test' });
      if (asset.kind === 'rust_integration') rustFull = true;
      continue;
    }
    const modules = inferModulesForPath(root, registry, file, 'source');
    modules.forEach((module) => directModules.add(module));
    mapping.push({ file, modules, source: modules.length ? 'path_rule_or_gov' : 'unmapped' });
    if (isExecutableChange(file, registry) && modules.length === 0) unmappedExecutable.push(file);
    if (matchesAny(file, registry.policy?.governance_globs)) governanceChanged = true;
    if (isProductCapabilityChange(file)) {
      governanceChanged = true;
      productCapabilityChanged = true;
    }
    if (matchesAny(file, registry.policy?.full_frontend_globs)) frontendFull = true;
    if (matchesAny(file, registry.policy?.full_rust_globs)) rustFull = true;
    if (/^src\/.*\.(?:ts|tsx)$/.test(file)) typescriptChanged = true;
    if (/^src-tauri\/(?:src|tests)\/.*\.rs$/.test(file)) rustFull = true;
  }

  const affectedModules = nonTestChanges.length > 0
    ? expandAffectedModules([...directModules], registry)
    : [...directModules].sort();
  const selectedFrontend = new Set([...directTestPaths].filter((file) => {
    return inventory.assets.find((asset) => asset.path === file)?.kind === 'frontend';
  }));
  // 控制文档与治理脚本不按 CORE 的测试文件集合扩散；它们由明确的治理自测入口负责。
  // 这避免新增治理测试后，任意治理改动被误升级为无关的前端回归。
  if (nonTestChanges.length > 0 && !governanceChanged) {
    for (const asset of inventory.assets) {
      if (asset.kind !== 'frontend') continue;
      if (asset.modules.some((module) => affectedModules.includes(module))) selectedFrontend.add(asset.path);
    }
  }

  const byCode = moduleMap(registry);
  if (nonTestChanges.length > 0 && affectedModules.some((code) => byCode.get(code)?.requires_rust_on_related)) {
    rustFull = true;
  }
  if (affectedModules.length >= 6) frontendFull = true;

  const commands = [];
  if (productCapabilityChanged) {
    commands.push(command('product-capability', 'npm', ['run', 'product:test'], '产品能力注册表、规则或生成/审计脚本发生变化'));
  }
  if (governanceChanged) {
    commands.push(command('test-governance', 'npm', ['run', 'test:governance'], '测试治理或控制文档发生变化'));
  }
  const selectedWithoutGovernanceSelfTest = [...selectedFrontend]
    .filter((file) => file !== 'scripts/test_governance.test.mjs')
    .sort();
  if (frontendFull) {
    commands.push(command('frontend-full', 'npm', ['test'], '共享配置或广泛模块影响，升级前端全量回归'));
  } else if (selectedWithoutGovernanceSelfTest.length > 0) {
    commands.push(command(
      'frontend-related',
      'npx',
      ['vitest', 'run', ...selectedWithoutGovernanceSelfTest],
      `运行 ${affectedModules.join(', ') || '直接变更'} 的相关前端测试`,
    ));
  }
  if (typescriptChanged) {
    commands.push(command('typescript', 'npx', ['tsc', '--noEmit'], 'TypeScript 业务源文件发生变化'));
  }
  if (rustFull) {
    commands.push(command(
      'rust-full',
      'cargo',
      ['test', '--manifest-path', 'src-tauri/Cargo.toml'],
      'Rust 源码、Cargo 配置或高风险跨层模块受影响，保守运行完整 Rust 测试',
    ));
  }
  if (nonTestChanges.length > 0 || governanceChanged) {
    commands.push(command('governance-audit', 'npm', ['run', 'governance:audit'], '非测试文件变化必须通过治理硬门'));
  }

  const failed = registry.policy?.fail_on_unmapped_executable && unmappedExecutable.length > 0;
  return {
    status: failed ? 'failed' : 'passed',
    changed_files: normalizedChanges,
    mapping,
    direct_modules: [...directModules].sort(),
    affected_modules: affectedModules,
    direct_tests: [...directTestPaths].sort(),
    selected_frontend_tests: [...selectedFrontend].sort(),
    full_frontend: frontendFull,
    full_rust: rustFull,
    unmapped_executable: unmappedExecutable,
    commands,
  };
}

function gitLines(root, args) {
  try {
    return execFileSync('git', args, { cwd: root, encoding: 'utf8' })
      .split('\n').map((line) => line.trim()).filter(Boolean);
  } catch {
    return [];
  }
}

export function discoverChangedFiles(root = DEFAULT_ROOT, options = {}) {
  if (options.files?.length) return unique(options.files.map(normalizePath));
  if (options.base) return gitLines(root, ['diff', '--name-only', `${options.base}...HEAD`]);
  if (options.staged) return gitLines(root, ['diff', '--cached', '--name-only', '--diff-filter=ACMR']);
  return unique([
    ...gitLines(root, ['diff', '--name-only', '--diff-filter=ACMR']),
    ...gitLines(root, ['diff', '--cached', '--name-only', '--diff-filter=ACMR']),
    ...gitLines(root, ['ls-files', '--others', '--exclude-standard']),
  ]);
}

export function runRelatedTestPlan(plan, root = DEFAULT_ROOT) {
  if (plan.status !== 'passed') {
    throw new Error(`存在未映射可执行变更: ${plan.unmapped_executable.join(', ')}`);
  }
  const results = [];
  for (const entry of plan.commands) {
    process.stdout.write(`\n[test-governance] ${entry.reason}\n$ ${entry.display}\n`);
    const result = spawnSync(entry.program, entry.args, { cwd: root, stdio: 'inherit' });
    results.push({ id: entry.id, status: result.status });
    if (result.status !== 0) {
      const error = new Error(`命令失败: ${entry.display}`);
      error.results = results;
      throw error;
    }
  }
  return results;
}

export function buildCoverageMapFromInventory(inventory) {
  const map = new Map();
  for (const asset of inventory.assets) {
    const represented = new Set();
    for (const entry of asset.cover_cases ?? []) {
      represented.add(entry.rule);
      if (!map.has(entry.rule)) map.set(entry.rule, []);
      map.get(entry.rule).push(`${asset.path}::${entry.test ?? 'file-level'}`);
    }
    for (const rule of asset.covers) {
      if (represented.has(rule)) continue;
      if (!map.has(rule)) map.set(rule, []);
      map.get(rule).push(`${asset.path}::file-level`);
    }
  }
  for (const entries of map.values()) entries.sort();
  return map;
}

export function renderTestCapabilityIndex(inventory) {
  const header = '| module | risk | test assets | test cases | layers | frontend baseline → target |\n'
    + '|---|---|---:|---:|---|---:|';
  const rows = Object.entries(inventory.modules).map(([code, summary]) => {
    const layers = Object.entries(summary.layers).map(([layer, count]) => `${layer}:${count}`).join(', ') || 'none';
    return `| ${code} | ${summary.risk} | ${summary.assets} | ${summary.tests} | ${layers} | ${summary.frontend_owned}/${summary.baseline_frontend_test_files} → ${summary.target_frontend_test_files} |`;
  });
  const validationHeader = '| validation | phase | modules | automation | status |\n'
    + '|---|---|---|---|---|';
  const validationRows = (inventory.system_validation ?? []).map((entry) => (
    `| ${entry.id} | ${entry.phase} | ${(entry.modules ?? []).join(', ')} | ${entry.automation} | ${entry.status} |`
  ));
  return [
    `动态发现：前端测试文件 ${inventory.totals.frontend_files}；Rust 内联测试文件 ${inventory.totals.rust_inline_files}；Rust 集成测试文件 ${inventory.totals.rust_integration_files}；测试用例约 ${inventory.totals.tests}。`,
    '',
    header,
    ...rows,
    '',
    '系统验证登记（自动化证据与人工验证证据分开）：',
    '',
    validationHeader,
    ...validationRows,
  ].join('\n');
}

function parseCli(argv) {
  const commandName = argv[0] && !argv[0].startsWith('--') ? argv[0] : 'select';
  const args = commandName === argv[0] ? argv.slice(1) : argv;
  const options = { files: [] };
  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (arg === '--json') options.json = true;
    else if (arg === '--staged') options.staged = true;
    else if (arg === '--base') options.base = args[++index];
    else if (arg === '--files') {
      while (args[index + 1] && !args[index + 1].startsWith('--')) options.files.push(args[++index]);
    } else if (!arg.startsWith('--')) options.files.push(arg);
  }
  return { commandName, options };
}

function printPlan(plan) {
  console.log(`status: ${plan.status}`);
  console.log(`changed: ${plan.changed_files.join(', ') || '(none)'}`);
  console.log(`direct modules: ${plan.direct_modules.join(', ') || '(none)'}`);
  console.log(`affected modules: ${plan.affected_modules.join(', ') || '(none)'}`);
  if (plan.unmapped_executable.length) console.log(`unmapped executable: ${plan.unmapped_executable.join(', ')}`);
  for (const entry of plan.commands) console.log(`- ${entry.display}  # ${entry.reason}`);
}

async function main() {
  const { commandName, options } = parseCli(process.argv.slice(2));
  if (commandName === 'inventory') {
    const inventory = buildTestInventory(DEFAULT_ROOT);
    if (options.json) console.log(JSON.stringify(inventory, null, 2));
    else console.log(renderTestCapabilityIndex(inventory));
    return;
  }
  if (commandName === 'audit') {
    const result = auditTestGovernance(DEFAULT_ROOT);
    const printable = options.json ? result : {
      status: result.status,
      checks: result.checks,
      findings: result.findings,
      warnings: result.warnings,
      modules: result.inventory.modules,
    };
    console.log(JSON.stringify(printable, null, 2));
    if (result.status !== 'passed') process.exitCode = 1;
    return;
  }
  if (commandName === 'select' || commandName === 'run') {
    const changedFiles = discoverChangedFiles(DEFAULT_ROOT, options);
    const plan = buildRelatedTestPlan(DEFAULT_ROOT, changedFiles);
    if (options.json) console.log(JSON.stringify(plan, null, 2));
    else printPlan(plan);
    if (plan.status !== 'passed') {
      process.exitCode = 1;
      return;
    }
    if (commandName === 'run') runRelatedTestPlan(plan, DEFAULT_ROOT);
    return;
  }
  throw new Error(`未知命令: ${commandName}`);
}

const invokedPath = process.argv[1] ? pathToFileURL(resolve(process.argv[1])).href : '';
if (import.meta.url === invokedPath) {
  main().catch((error) => {
    console.error(`[test-governance] ${error.message}`);
    process.exitCode = 1;
  });
}
