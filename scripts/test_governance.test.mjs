import { describe, expect, it } from 'vitest';
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
