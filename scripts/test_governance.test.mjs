import { describe, expect, it } from 'vitest';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'fs';
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
import {
  auditProductCapabilityRegistry,
  buildCapabilityView,
  renderCapabilityAtlas,
  writeProductCapabilityArtifacts,
} from './product_capability.mjs';
import {
  activeDocumentMayOwnRule,
  activeReferenceMayUseRule,
  extractDocumentStatus,
  ruleRegistryStatus,
  ruleSourceStatus,
} from './rule_lifecycle.mjs';

describe('Test Governance', () => {
  const { registry } = loadTestGovernance(DEFAULT_ROOT);
  const inventory = buildTestInventory(DEFAULT_ROOT, registry);

  it('规则生命周期由 RULE 真源状态派生，并阻断退役规则的活动引用', () => {
    expect(extractDocumentStatus('文档状态：R\n')).toBe('R');
    expect(ruleSourceStatus({})).toBe('active');
    expect(ruleRegistryStatus({})).toBe('registered');
    expect(activeDocumentMayOwnRule('A', {})).toBe(true);
    expect(activeDocumentMayOwnRule('R', {})).toBe(false);

    const retired = { status: 'superseded' };
    expect(ruleRegistryStatus(retired)).toBe('superseded');
    expect(activeDocumentMayOwnRule('R', retired)).toBe(true);
    expect(activeReferenceMayUseRule(ruleSourceStatus(retired))).toBe(false);
  });

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

  it('ADUS 完整登记 Agent 需求、需求映射与状态机', () => {
    const parseBlocks = (source, type) => [...source.matchAll(
      new RegExp(`<!--[ \\t]*${type}[ \\t]*\\r?\\n([\\s\\S]*?)-->`, 'g'),
    )].map((match) => Object.fromEntries(match[1]
      .split(/\r?\n/)
      .map((line) => {
        const separator = line.indexOf(':');
        return separator < 0
          ? null
          : [line.slice(0, separator).trim(), line.slice(separator + 1).trim()];
      })
      .filter(Boolean)));

    const requirementSource = readFileSync(
      join(DEFAULT_ROOT, 'docs/01_modules/AG/A-AG-M-D-01_代理功能需求描述.md'),
      'utf8',
    );
    const designSources = [
      'docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md',
      'docs/01_modules/AG/A-AG-M-T-02_提示词架构设计.md',
      'docs/01_modules/AG/A-AG-M-S-01_代理状态机设计.md',
    ].map((path) => readFileSync(join(DEFAULT_ROOT, path), 'utf8'));
    const requirements = parseBlocks(requirementSource, 'REQ');
    const mappings = designSources.flatMap((source) => parseBlocks(source, 'REQ_MAP'));
    const activeRequirements = requirements.filter((entry) => entry.status === 'active');
    const deferredRequirements = requirements
      .filter((entry) => entry.status === 'deferred')
      .map((entry) => entry.req_id)
      .sort();

    expect(requirements).toHaveLength(17);
    expect(activeRequirements).toHaveLength(13);
    expect(deferredRequirements).toEqual([
      'REQ-AG-010', 'REQ-AG-012', 'REQ-AG-014', 'REQ-AG-016',
    ]);
    for (const requirement of activeRequirements) {
      expect(mappings.some((mapping) => (
        mapping.req_id === requirement.req_id && mapping.status === 'mapped'
      )), `${requirement.req_id} 缺少 mapped REQ_MAP`).toBe(true);
    }

    const adus = readFileSync(
      join(DEFAULT_ROOT, 'docs/00_core/A-CORE-C-R-01_ADUS.md'),
      'utf8',
    );
    expect(adus).toContain('### 需求注册表');
    expect(adus).toContain('### 需求映射表');
    expect(adus).toContain('### 状态机注册表');
    expect(adus).toMatch(/\|\s*REQ-AG-010\s*\|[^\n]*\|\s*deferred\s*\|/);
    expect(adus).toMatch(/\|\s*REQ-AG-009\s*\|\s*CONSTRAINT\s*\|\s*X-SHARED-002\s*\|\s*mapped\s*\|/);
    for (const machineId of ['SM-AG-01', 'SM-AG-02', 'SM-AG-03', 'SM-AG-04']) {
      expect(adus).toMatch(new RegExp(`\\|\\s*${machineId}\\s*\\|\\s*AG\\s*\\|`));
    }
  });

  it('产品能力图谱变更强制触发其自测、治理自测与审计', () => {
    const plan = buildRelatedTestPlan(DEFAULT_ROOT, ['.governance/product-capability-registry.json'], registry, inventory);
    expect(plan.status).toBe('passed');
    expect(plan.direct_modules).toEqual(['CORE']);
    expect(plan.commands.map((entry) => entry.id)).toEqual(expect.arrayContaining([
      'product-capability', 'test-governance', 'governance-audit',
    ]));
    expect(plan.commands.find((entry) => entry.id === 'product-capability')?.display).toBe('npm run product:test');
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
    expect(inventory.modules.ED.frontend_owned).toBe(11);
    expect(inventory.assets
      .filter((asset) => asset.kind === 'frontend' && asset.owner === 'ED')
      .reduce((sum, asset) => sum + asset.test_count, 0)).toBe(243); // ED-TEXT-SAVE-REPAIR-01：旧草稿保存恢复 +2，错误展示重试 +1。
    expect(inventory.modules.SYS.frontend_owned).toBe(3);
    expect(inventory.assets
      .filter((asset) => asset.kind === 'frontend' && asset.owner === 'SYS')
      .reduce((sum, asset) => sum + asset.test_count, 0)).toBe(59);
    expect(inventory.modules.WS.frontend_owned).toBe(2);
    expect(inventory.assets
      .filter((asset) => asset.kind === 'frontend' && asset.owner === 'WS')
      .reduce((sum, asset) => sum + asset.test_count, 0)).toBe(42); // 2026-08-22 PDF 预览路由回归 +1（WorkspaceUI：pdf 走只读预览，修复 WS_NOT_UTF8 误开二进制，人类批准基线调整 40→41）；2026-08-24 WS-RESOURCE-WRITE-GUARD-01 +1（WorkspaceUI：BR-WS-STATE-009 dirty 会话空目录删除确认提示，人类批准基线调整 41→42）
    expect(inventory.modules.DE.frontend_owned).toBe(2);
    expect(inventory.assets
      .filter((asset) => asset.kind === 'frontend' && asset.owner === 'DE')
      .reduce((sum, asset) => sum + asset.test_count, 0)).toBe(78); // AG-1.0-IMP-CONSECUTIVE-PROPOSAL-01：deRuntime +4（连续 Proposal 前卡失效，BR-SYS-STATE-008，人类批准基线调整 69→73）；AG-PROPOSAL-BATCH-WP7-01：deRuntime +3（缺组/乱序、失败关闭、兄弟卡独立，73→76）；AG-UI-RECOVERY-WP8-01：deRuntime/DiffReview +2（持久化批次恢复与缺组裁决门禁，人类确认 76→78）
    expect(inventory.modules.AG.frontend_owned).toBe(3);
    expect(inventory.assets
      .filter((asset) => asset.kind === 'frontend' && asset.owner === 'AG')
      .reduce((sum, asset) => sum + asset.test_count, 0)).toBe(163); // AgentUI 54 + agRuntime 88 + assistRuntime 21；新增失败正文保存/旧缓存恢复两项，物理测试文件不增加。
    expect(inventory.modules.PRV.frontend_owned).toBe(2);
    expect(inventory.assets
      .filter((asset) => asset.kind === 'frontend' && asset.owner === 'PRV')
      .reduce((sum, asset) => sum + asset.test_count, 0)).toBe(19);
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

// covers: BR-PCG-DATA-001
// covers: BR-PCG-STATE-001
// covers: BR-PCG-VERIFY-001
// covers: BR-PCG-VERIFY-002
describe('Product Capability Graph', () => {
  function withCapabilityFixture(callback) {
    const root = mkdtempSync(join(tmpdir(), 'binder-pcg-'));
    try {
      mkdirSync(join(root, 'docs'), { recursive: true });
      mkdirSync(join(root, 'src'), { recursive: true });
      writeFileSync(join(root, 'docs', 'authority.md'), `<!-- REQ\nreq_id: REQ-DEMO-001\n-->\n<!-- RULE\nrule_id: BR-PCG-DATA-001\n-->\n<!-- RULE\nrule_id: BR-PCG-STATE-001\n-->\n<!-- RULE\nrule_id: BR-PCG-VERIFY-001\n-->\n<!-- RULE\nrule_id: BR-PCG-VERIFY-002\n-->\n<!-- CHAIN\nchain_id: PCG-GENERATE\n-->\n<!-- TERM\nterm_id: TERM-PCG-001\n-->\nvalidation_id: VAL-DEMO-001\n`);
      writeFileSync(join(root, 'src', 'demo.test.ts'), 'export {};\n');
      callback(root);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  }

  function validCapabilityRegistry() {
    return {
      schema_version: 1,
      product_areas: [{ id: 'AREA-1', name: '文档协作' }],
      journeys: [{ id: 'JOURNEY-1', area_id: 'AREA-1', name: '编辑文档', stages: ['编辑'] }],
      capabilities: [{
        id: 'CAP-1', area_id: 'AREA-1', journey_ids: ['JOURNEY-1'], name: '安全保存', summary: '用户可以安全保存文档。', term_refs: ['TERM-PCG-001'],
        variants: [{
          id: 'VAR-1', name: '保存 Markdown', stage: '编辑', actor: 'user', formats: ['markdown'], operation: 'save', support: 'supported', side_effect: true, canonical_chain: 'PCG-GENERATE',
          requirements: ['REQ-DEMO-001'], rules: ['BR-PCG-DATA-001', 'BR-PCG-STATE-001', 'BR-PCG-VERIFY-001', 'BR-PCG-VERIFY-002'], terms: ['TERM-PCG-001'], acceptance: ['用户保存后可重新打开相同内容。'],
          evidence: { tests: ['src/demo.test.ts'], validations: ['VAL-DEMO-001'] },
          state: { definition: 'confirmed', delivery: 'implemented', automation: 'passed', system_validation: 'passed', release: 'available' },
        }],
      }],
    };
  }

  it('接受具有完整追溯、主链和独立状态的注册表', () => withCapabilityFixture((root) => {
    const audit = auditProductCapabilityRegistry(root, validCapabilityRegistry());
    expect(audit.status).toBe('passed');
    expect(audit.summary).toMatchObject({ product_areas: 1, journeys: 1, capabilities: 1, variants: 1 });
  }));

  it('拒绝副作用变体缺失正式主链，并拒绝以自动化结果伪造可发布状态', () => withCapabilityFixture((root) => {
    const registry = validCapabilityRegistry();
    const variant = registry.capabilities[0].variants[0];
    delete variant.canonical_chain;
    variant.state.system_validation = 'pending';
    const audit = auditProductCapabilityRegistry(root, registry);
    expect(audit.findings.map((finding) => finding.type)).toEqual(expect.arrayContaining(['PCG_CANONICAL_CHAIN_MISSING', 'PCG_RELEASE_PROMOTION_INVALID']));
  }));

  it('验证 fallback、证据路径和失效证据', () => withCapabilityFixture((root) => {
    const registry = validCapabilityRegistry();
    const variant = registry.capabilities[0].variants[0];
    variant.fallbacks = [{ fallback_of: 'OTHER-CHAIN', when: '', result: '' }];
    variant.evidence.tests = ['src/missing.test.ts'];
    variant.evidence_stale = true;
    const audit = auditProductCapabilityRegistry(root, registry);
    expect(audit.findings.map((finding) => finding.type)).toEqual(expect.arrayContaining(['PCG_FALLBACK_INVALID', 'PCG_TEST_EVIDENCE_UNKNOWN', 'PCG_EVIDENCE_STALE']));
  }));

  it('生成可点击的产品故事地图和格式矩阵', () => withCapabilityFixture((root) => {
    const registry = validCapabilityRegistry();
    const audit = auditProductCapabilityRegistry(root, registry);
    const html = renderCapabilityAtlas(buildCapabilityView(registry), audit);
    expect(html).toContain('Binder 产品能力沙盘');
    expect(html).toContain('data-variant-id="VAR-1"');
    expect(html).toContain('格式支持矩阵');
    expect(html).toContain('系统验证');
  }));

  it('只向生成目录写 HTML、覆盖 JSON 与主链报告', () => withCapabilityFixture((root) => {
    const registry = validCapabilityRegistry();
    const audit = auditProductCapabilityRegistry(root, registry);
    writeProductCapabilityArtifacts(root, { product_capability_output_dir: 'generated' }, buildCapabilityView(registry), audit);
    expect(readFileSync(join(root, 'generated', 'product-capability-atlas.html'), 'utf8')).toContain('VAR-1');
    expect(JSON.parse(readFileSync(join(root, 'generated', 'product-capability-coverage.json'), 'utf8')).variants).toHaveLength(1);
    expect(readFileSync(join(root, 'generated', 'product-capability-chain-audit.md'), 'utf8')).toContain('正式主链');
  }));

  it('AG STATE_MACHINE rules 闭包覆盖关键规则且不混入外部效果规则', () => {
    const text = readFileSync(join(DEFAULT_ROOT, 'docs/01_modules/AG/A-AG-M-S-01_代理状态机设计.md'), 'utf8');
    const blocks = [...text.matchAll(/<!--\s*STATE_MACHINE\s*([\s\S]*?)-->/g)]
      .map((match) => {
        const body = match[1];
        const id = /machine_id:\s*(SM-AG-\d+)/.exec(body)?.[1];
        const rules = (/rules:\s*(.+)/.exec(body)?.[1] ?? '')
          .split(',')
          .map((rule) => rule.trim())
          .filter(Boolean);
        return { id, rules };
      })
      .filter((block) => block.id);
    const required = {
      'SM-AG-01': ['BR-AG-STATE-001', 'BR-AG-STATE-002', 'BR-AG-STATE-005', 'BR-SYS-STATE-007', 'BR-SYS-STATE-008', 'BR-AG-DATA-003', 'BR-AG-DATA-008', 'BR-AG-OBS-001'],
      'SM-AG-02': ['BR-AG-STATE-002', 'BR-AG-DATA-011', 'BR-AG-DATA-012', 'BR-AG-DATA-013', 'BR-SYS-STATE-008', 'BR-AG-OBS-001'],
      'SM-AG-03': ['BR-AG-STATE-003', 'BR-AG-DATA-002', 'BR-AG-DATA-007'],
      'SM-AG-04': ['BR-SYS-STATE-008', 'BR-AG-STATE-001', 'BR-AG-STATE-002', 'BR-CORE-STATE-001'],
    };
    const failures = [];
    for (const [id, expected] of Object.entries(required)) {
      const block = blocks.find((entry) => entry.id === id);
      if (!block) {
        failures.push(`${id}: STATE_MACHINE block missing`);
        continue;
      }
      for (const rule of expected) {
        if (!block.rules.includes(rule)) failures.push(`${id}: missing ${rule}`);
      }
      for (const rule of block.rules) {
        if (!/^BR-(AG|SYS|CORE)-/.test(rule)) failures.push(`${id}: external rule ${rule}`);
      }
    }
    expect(failures).toEqual([]);
  });
});
