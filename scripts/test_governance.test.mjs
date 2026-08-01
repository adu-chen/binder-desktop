import { describe, expect, it } from 'vitest';
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from 'fs';
import { tmpdir } from 'os';
import { join } from 'path';
import {
  DEFAULT_ROOT,
  auditTestGovernance,
  buildRelatedTestPlan,
  buildTestInventory,
  expandAffectedModules,
  extractCovers,
  extractRuleModules,
  globToRegExp,
  loadTestGovernance,
  matchesGlob,
} from './test_governance.mjs';
import {
  activeRecords,
  deferralKey,
  loadTriageRegistry,
} from './triage_registry.mjs';

describe('Test Governance', () => {
  const { registry } = loadTestGovernance(DEFAULT_ROOT);
  const inventory = buildTestInventory(DEFAULT_ROOT, registry);

  it('支持稳定的路径 glob，且 **/ 可匹配零级或多级目录', () => {
    expect(matchesGlob('src/components/ed/Editor.test.tsx', 'src/components/ed/**/*.test.tsx')).toBe(true);
    expect(matchesGlob('src/components/ed/deep/Editor.test.tsx', 'src/components/ed/**/*.test.tsx')).toBe(true);
    expect(matchesGlob('src/components/ag/Editor.test.tsx', 'src/components/ed/**/*.test.tsx')).toBe(false);
    expect(globToRegExp('src-tauri/src/**/*.rs').test('src-tauri/src/lib.rs')).toBe(true);
  });

  it('从 covers 与规则引用提取正式模块，不把自然语言当映射', () => {
    const source = [
      '// covers: BR-ED-DATA-001',
      '// covers: BR-DE-STATE-001',
      'const note = "ED is text only";',
    ].join('\n');
    expect(extractCovers(source)).toEqual(['BR-ED-DATA-001', 'BR-DE-STATE-001']);
    expect(extractRuleModules(source)).toEqual(['ED', 'DE']);
  });

  it('依赖扩展只执行一跳，避免下游模块递归放大全项目', () => {
    expect(expandAffectedModules(['AG'], registry)).toEqual(['AG', 'SYS']);
    expect(expandAffectedModules(['ED'], registry)).toEqual(['AG', 'DE', 'ED', 'SYS']);
  });

  it('直接修改测试文件时直选该文件，不伪造业务源变更', () => {
    const path = 'src/components/ed/extensions/EditorExtensions.test.ts';
    const plan = buildRelatedTestPlan(DEFAULT_ROOT, [path], registry, inventory);
    expect(plan.status).toBe('passed');
    expect(plan.direct_tests).toEqual([path]);
    expect(plan.selected_frontend_tests).toEqual([path]);
    expect(plan.full_rust).toBe(false);
    expect(plan.commands.find((entry) => entry.id === 'frontend-related')?.args).toContain(path);
  });

  it('Rust 内联测试承载文件仍按生产源码触发 Rust 全量与治理审计', () => {
    const path = 'src-tauri/src/lib.rs';
    const plan = buildRelatedTestPlan(DEFAULT_ROOT, [path], registry, inventory);
    expect(plan.status).toBe('passed');
    expect(plan.direct_tests).not.toContain(path);
    expect(plan.mapping).toContainEqual(expect.objectContaining({
      file: path,
      source: 'path_rule_or_gov',
    }));
    expect(plan.full_rust).toBe(true);
    expect(plan.commands.map((entry) => entry.id)).toEqual(expect.arrayContaining([
      'rust-full', 'governance-audit',
    ]));
  });

  it('直接修改 Rust 集成测试时触发 Rust 全量但不伪造生产源码变化', () => {
    const path = 'src-tauri/tests/core_chain.rs';
    const plan = buildRelatedTestPlan(DEFAULT_ROOT, [path], registry, inventory);
    expect(plan.status).toBe('passed');
    expect(plan.direct_tests).toEqual([path]);
    expect(plan.full_rust).toBe(true);
    expect(plan.commands.map((entry) => entry.id)).toEqual(['rust-full']);
  });

  it('高风险业务变更触发所属模块、直接消费者与完整 Rust 回归', () => {
    const plan = buildRelatedTestPlan(DEFAULT_ROOT, ['src/services/edService.ts'], registry, inventory);
    expect(plan.status).toBe('passed');
    expect(plan.direct_modules).toContain('ED');
    expect(plan.affected_modules).toEqual(expect.arrayContaining(['ED', 'DE', 'AG', 'SYS']));
    expect(plan.selected_frontend_tests).toContain('src/services/edRuntime.test.ts');
    expect(plan.commands.map((entry) => entry.id)).toEqual(expect.arrayContaining([
      'frontend-related', 'typescript', 'rust-full', 'governance-audit',
    ]));
  });

  it('治理能力变更只触发治理自测和审计，不冒充业务模块变化', () => {
    const plan = buildRelatedTestPlan(DEFAULT_ROOT, ['scripts/test_governance.mjs'], registry, inventory);
    expect(plan.status).toBe('passed');
    expect(plan.direct_modules).toEqual(['CORE']);
    expect(plan.commands.map((entry) => entry.id)).toEqual(['test-governance', 'governance-audit']);
  });

  it('无法映射的可执行变更失败关闭', () => {
    const plan = buildRelatedTestPlan(DEFAULT_ROOT, ['src/unknown/newCapability.ts'], registry, inventory);
    expect(plan.status).toBe('failed');
    expect(plan.unmapped_executable).toEqual(['src/unknown/newCapability.ts']);
  });

  it('真实测试 inventory 全部具备 owner、layer 且治理 audit 通过', () => {
    expect(inventory.totals.frontend_files).toBeGreaterThanOrEqual(21);
    expect(inventory.totals.rust_inline_files).toBeGreaterThan(0);
    expect(inventory.totals.rust_integration_files).toBe(2);
    expect(inventory.system_validation).toHaveLength(2);
    expect(inventory.modules.ED.frontend_owned).toBe(7);
    expect(inventory.assets
      .filter((asset) => asset.kind === 'frontend' && asset.owner === 'ED')
      .reduce((sum, asset) => sum + asset.test_count, 0)).toBe(207);
    expect(inventory.modules.SYS.frontend_owned).toBe(3);
    expect(inventory.assets
      .filter((asset) => asset.kind === 'frontend' && asset.owner === 'SYS')
      .reduce((sum, asset) => sum + asset.test_count, 0)).toBe(52);
    expect(inventory.modules.WS.frontend_owned).toBe(2);
    expect(inventory.assets
      .filter((asset) => asset.kind === 'frontend' && asset.owner === 'WS')
      .reduce((sum, asset) => sum + asset.test_count, 0)).toBe(28);
    expect(inventory.modules.DE.frontend_owned).toBe(2);
    expect(inventory.assets
      .filter((asset) => asset.kind === 'frontend' && asset.owner === 'DE')
      .reduce((sum, asset) => sum + asset.test_count, 0)).toBe(58);
    expect(inventory.modules.AG.frontend_owned).toBe(3);
    expect(inventory.assets
      .filter((asset) => asset.kind === 'frontend' && asset.owner === 'AG')
      .reduce((sum, asset) => sum + asset.test_count, 0)).toBe(73);
    expect(inventory.modules.PRV.frontend_owned).toBe(1);
    expect(inventory.assets
      .filter((asset) => asset.kind === 'frontend' && asset.owner === 'PRV')
      .reduce((sum, asset) => sum + asset.test_count, 0)).toBe(7);
    expect(inventory.assets.every((asset) => asset.owner && asset.layer !== 'unclassified')).toBe(true);
    expect(auditTestGovernance(DEFAULT_ROOT, registry, inventory).status).toBe('passed');
  }, 10_000);
});

describe('Triage Registry', () => {
  function withRegistryFiles(files, callback) {
    const root = mkdtempSync(join(tmpdir(), 'binder-triage-registry-'));
    try {
      for (const [relativePath, content] of Object.entries(files)) {
        const path = join(root, relativePath);
        mkdirSync(join(path, '..'), { recursive: true });
        writeFileSync(path, content);
      }
      callback(root);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  }

  it('只接受七类正式 triage，并保留文件与 codes 的结构化字段', () => {
    withRegistryFiles({
      'docs/ED_triage.md': [
        'codes: CODE-ED-001',
        '文件: src/ed.ts',
        '分类: VALID_EMPTY',
        '原因: pure helper',
        '已采取动作: documented',
        '需要人类介入: 否',
        '已解决: 否',
      ].join('\n'),
    }, root => {
      const registry = loadTriageRegistry(root);
      expect(registry.findings).toEqual([]);
      expect(registry.triageRecords).toHaveLength(1);
      expect(registry.triageRecords[0].fields.codes).toBe('CODE-ED-001');
      expect(registry.triageRecords[0].sourceFile).toBe('src/ed.ts');
    });
  });

  it('拒绝 BLOCK_UNANNOTATED 和缺字段，避免把负空间发现当作正式分类', () => {
    withRegistryFiles({
      'docs/CORE_triage.md': [
        'codes: CODE-CORE-001',
        '文件: src/core.ts',
        '分类: BLOCK_UNANNOTATED',
        '原因: missing annotation',
        '已采取动作: deferred',
        '需要人类介入: 否',
        '已解决: 否',
        '',
        'codes: CODE-CORE-002',
        '文件: src/core.ts',
        '分类: VALID_EMPTY',
      ].join('\n'),
    }, root => {
      const registry = loadTriageRegistry(root);
      expect(registry.findings.map(finding => finding.type)).toEqual(expect.arrayContaining([
        'TRIAGE_RECORD_INVALID',
      ]));
      expect(registry.findings.some(finding => finding.classification === 'BLOCK_UNANNOTATED')).toBe(true);
    });
  });

  it('标注延期以文件与符号组成唯一键，不因同名符号而跨文件匹配', () => {
    withRegistryFiles({
      'docs/CORE_标注延期清单.md': [
        'symbol: App',
        '文件: src/shell/App.tsx',
        '原因: shell scaffold',
        '承接计划: PI-SYS-05',
        '退出条件: add @GOV when business logic lands',
        '复核日期: 2026-08-01',
        '需要人类介入: 否',
        '已解决: 否',
      ].join('\n'),
    }, root => {
      const registry = loadTriageRegistry(root);
      const records = activeRecords(registry.deferralRecords);
      const keys = new Set(records.map(record => deferralKey(record.sourceFile, record.fields.symbol)));
      expect(keys.has('src/shell/App.tsx::App')).toBe(true);
      expect(keys.has('src/other/App.tsx::App')).toBe(false);
    });
  });
});
