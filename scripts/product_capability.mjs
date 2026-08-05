import { existsSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url));
export const DEFAULT_ROOT = resolve(SCRIPT_DIR, '..');
export const SUPPORT_STATES = new Set(['supported', 'partial', 'unsupported', 'unknown', 'not_applicable']);
export const DEFINITION_STATES = new Set(['draft', 'confirmed', 'changed_needs_review']);
export const DELIVERY_STATES = new Set(['not_started', 'in_progress', 'implemented']);
export const AUTOMATION_STATES = new Set(['none', 'partial', 'passed', 'failed']);
export const SYSTEM_VALIDATION_STATES = new Set(['not_required', 'pending', 'passed', 'failed', 'blocked']);
export const RELEASE_STATES = new Set(['planned', 'internal', 'available', 'not_ready', 'unsupported']);

const skippedDirectories = new Set(['node_modules', 'target', 'dist', 'build', 'generated']);
const unique = values => [...new Set(values.filter(Boolean))];
const html = value => String(value ?? '').replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;').replaceAll("'", '&#39;');

function walkFiles(root, relativeDirectory, results = []) {
  let entries = [];
  try { entries = readdirSync(join(root, relativeDirectory)); } catch { return results; }
  for (const entry of entries) {
    if (skippedDirectories.has(entry)) continue;
    const absolute = join(root, relativeDirectory, entry);
    const path = relative(root, absolute).replaceAll('\\', '/');
    if (statSync(absolute).isDirectory()) walkFiles(root, path, results);
    else results.push(path);
  }
  return results;
}

function blockFields(body) {
  return Object.fromEntries(body.split('\n').flatMap((line) => {
    const at = line.indexOf(':');
    return at < 0 ? [] : [[line.slice(0, at).trim(), line.slice(at + 1).trim()]];
  }).filter(([key]) => key));
}

export function collectAuthorityIds(root = DEFAULT_ROOT) {
  const result = { requirements: new Set(), rules: new Set(), chains: new Set(), terms: new Set(), validations: new Set(), files: new Set(walkFiles(root, '.')) };
  for (const file of walkFiles(root, 'docs').filter(path => path.endsWith('.md'))) {
    const source = readFileSync(join(root, file), 'utf8');
    for (const match of source.matchAll(/<!--\s*(REQ|RULE|CHAIN|TERM)\s*([\s\S]*?)-->/g)) {
      const fields = blockFields(match[2]);
      const key = { REQ: 'requirements', RULE: 'rules', CHAIN: 'chains', TERM: 'terms' }[match[1]];
      const id = fields[{ REQ: 'req_id', RULE: 'rule_id', CHAIN: 'chain_id', TERM: 'term_id' }[match[1]]];
      if (id) result[key].add(id);
    }
    for (const match of source.matchAll(/^\s*validation_id\s*:\s*([A-Z0-9-]+)\s*$/gm)) result.validations.add(match[1]);
  }
  return result;
}

export function loadProductCapabilityRegistry(root = DEFAULT_ROOT) {
  const config = JSON.parse(readFileSync(join(root, 'governance.config.json'), 'utf8'));
  if (!config.product_capability_path) throw new Error('governance.config.json 缺少 product_capability_path');
  const registryPath = join(root, config.product_capability_path);
  if (!existsSync(registryPath)) throw new Error(`产品能力注册表不存在: ${config.product_capability_path}`);
  return { config, registryPath, registry: JSON.parse(readFileSync(registryPath, 'utf8')) };
}

function add(findings, type, message, details = {}) { findings.push({ type, message, ...details }); }
function ids(items, field, findings, type) {
  const seen = new Set();
  for (const item of items ?? []) {
    if (!item?.[field] || seen.has(item[field])) add(findings, type, '标识符缺失或重复', { id: item?.[field] ?? null });
    else seen.add(item[field]);
  }
  return seen;
}
function enumValue(findings, value, allowed, path, label) {
  if (!allowed.has(value)) add(findings, 'PCG_STATE_INVALID', `${label} 取值无效`, { path, value });
}

export function auditProductCapabilityRegistry(root = DEFAULT_ROOT, suppliedRegistry = null) {
  const registry = suppliedRegistry ?? loadProductCapabilityRegistry(root).registry;
  const findings = [], warnings = [], authority = collectAuthorityIds(root);
  if (registry.schema_version !== 1) add(findings, 'PCG_SCHEMA_VERSION', 'schema_version 必须为 1', { value: registry.schema_version });
  const areaIds = ids(registry.product_areas, 'id', findings, 'PCG_AREA_DUPLICATE');
  const journeyIds = ids(registry.journeys, 'id', findings, 'PCG_JOURNEY_DUPLICATE');
  const capabilityIds = ids(registry.capabilities, 'id', findings, 'PCG_CAPABILITY_DUPLICATE');
  const variantIds = new Set();
  for (const journey of registry.journeys ?? []) {
    if (!areaIds.has(journey.area_id)) add(findings, 'PCG_AREA_REFERENCE_MISSING', 'Journey 引用了未知 ProductArea', { journey: journey.id });
    if (!journey.stages?.length) add(findings, 'PCG_JOURNEY_STAGES_MISSING', 'Journey 必须定义用户阶段', { journey: journey.id });
  }
  for (const capability of registry.capabilities ?? []) {
    if (!areaIds.has(capability.area_id)) add(findings, 'PCG_AREA_REFERENCE_MISSING', 'ProductCapability 引用了未知 ProductArea', { capability: capability.id });
    for (const journeyId of capability.journey_ids ?? []) if (!journeyIds.has(journeyId)) add(findings, 'PCG_JOURNEY_REFERENCE_MISSING', 'ProductCapability 引用了未知 Journey', { capability: capability.id, journey_id: journeyId });
    if (!capability.variants?.length) { add(findings, 'PCG_VARIANT_MISSING', 'ProductCapability 至少需要一个 CapabilityVariant', { capability: capability.id }); continue; }
    for (const variant of capability.variants) {
      if (!variant.id || variantIds.has(variant.id)) add(findings, 'PCG_VARIANT_DUPLICATE', 'CapabilityVariant 标识符缺失或重复', { variant: variant.id ?? null });
      else variantIds.add(variant.id);
      if (!variant.name || !variant.stage || !variant.actor || !variant.operation || !variant.formats?.length) add(findings, 'PCG_VARIANT_DESCRIPTOR_MISSING', 'CapabilityVariant 缺少名称、阶段、操作者、操作或格式', { variant: variant.id });
      enumValue(findings, variant.support, SUPPORT_STATES, variant.id, 'support');
      const state = variant.state ?? {};
      enumValue(findings, state.definition, DEFINITION_STATES, variant.id, 'definition');
      enumValue(findings, state.delivery, DELIVERY_STATES, variant.id, 'delivery');
      enumValue(findings, state.automation, AUTOMATION_STATES, variant.id, 'automation');
      enumValue(findings, state.system_validation, SYSTEM_VALIDATION_STATES, variant.id, 'system_validation');
      enumValue(findings, state.release, RELEASE_STATES, variant.id, 'release');
      if (state.definition === 'confirmed' && !variant.acceptance?.length) add(findings, 'PCG_ACCEPTANCE_MISSING', '已确认变体缺少验收点', { variant: variant.id });
      if (state.definition === 'confirmed' && !variant.requirements?.length && !variant.internal_source) add(findings, 'PCG_REQUIREMENT_REFERENCE_MISSING', '已确认变体必须引用 REQ 或 internal_source', { variant: variant.id });
      for (const id of variant.requirements ?? []) if (!authority.requirements.has(id)) add(findings, 'PCG_REQUIREMENT_UNKNOWN', '引用了未知 REQ', { variant: variant.id, id });
      for (const id of variant.rules ?? []) if (!authority.rules.has(id)) add(findings, 'PCG_RULE_UNKNOWN', '引用了未知 RULE', { variant: variant.id, id });
      for (const id of unique([...(capability.term_refs ?? []), ...(variant.terms ?? [])])) if (!authority.terms.has(id)) add(findings, 'PCG_TERM_UNKNOWN', '引用了未知 TERM', { variant: variant.id, id });
      if (variant.side_effect && !variant.canonical_chain) add(findings, 'PCG_CANONICAL_CHAIN_MISSING', '有副作用变体必须声明 canonical_chain', { variant: variant.id });
      if (variant.canonical_chain && !authority.chains.has(variant.canonical_chain)) add(findings, 'PCG_CHAIN_UNKNOWN', 'canonical_chain 未登记', { variant: variant.id, chain: variant.canonical_chain });
      for (const fallback of variant.fallbacks ?? []) if (fallback.fallback_of !== variant.canonical_chain || !fallback.when || !fallback.result) add(findings, 'PCG_FALLBACK_INVALID', 'fallback 必须绑定主链并声明条件和结果', { variant: variant.id });
      for (const path of variant.evidence?.tests ?? []) if (!authority.files.has(path)) add(findings, 'PCG_TEST_EVIDENCE_UNKNOWN', '测试证据文件不存在', { variant: variant.id, path });
      for (const id of variant.evidence?.validations ?? []) if (!authority.validations.has(id)) add(findings, 'PCG_VALIDATION_UNKNOWN', '系统验证证据未登记', { variant: variant.id, id });
      if (state.automation === 'passed' && !variant.evidence?.tests?.length) add(findings, 'PCG_AUTOMATION_EVIDENCE_MISSING', 'automation=passed 但没有测试证据', { variant: variant.id });
      if (state.system_validation === 'passed' && !variant.evidence?.validations?.length) add(findings, 'PCG_SYSTEM_VALIDATION_EVIDENCE_MISSING', 'system_validation=passed 但没有验证证据', { variant: variant.id });
      if (state.release === 'available' && (state.automation !== 'passed' || !['passed', 'not_required'].includes(state.system_validation))) add(findings, 'PCG_RELEASE_PROMOTION_INVALID', 'release=available 需要自动化与系统验证闭合', { variant: variant.id });
      if (variant.evidence_stale) add(findings, 'PCG_EVIDENCE_STALE', '上游变化后证据待复核', { variant: variant.id });
      if (variant.duplicate_candidates?.length) warnings.push({ type: 'PCG_DUPLICATE_CANDIDATE', message: '存在待治理确认的重复实现候选', variant: variant.id, candidates: variant.duplicate_candidates });
      if (state.definition === 'confirmed' && !(variant.evidence?.tests?.length || variant.evidence?.validations?.length)) warnings.push({ type: 'PCG_EVIDENCE_EMPTY', message: '已确认变体尚无证据', variant: variant.id });
    }
  }
  const variants = (registry.capabilities ?? []).flatMap(capability => capability.variants ?? []);
  const count = (field, values) => Object.fromEntries([...values].map(value => [value, variants.filter(variant => field(variant) === value).length]));
  return { status: findings.length ? 'failed' : 'passed', findings, warnings, authority, registry, summary: { product_areas: areaIds.size, journeys: journeyIds.size, capabilities: capabilityIds.size, variants: variants.length, support: count(variant => variant.support, SUPPORT_STATES), automation: count(variant => variant.state?.automation, AUTOMATION_STATES), system_validation: count(variant => variant.state?.system_validation, SYSTEM_VALIDATION_STATES) } };
}

export function buildCapabilityView(registry) {
  const areas = new Map((registry.product_areas ?? []).map(area => [area.id, area]));
  const journeys = new Map((registry.journeys ?? []).map(journey => [journey.id, journey]));
  return { areas: [...areas.values()], journeys: [...journeys.values()], variants: (registry.capabilities ?? []).flatMap(capability => (capability.variants ?? []).map(variant => ({ ...variant, capability_id: capability.id, capability_name: capability.name, capability_summary: capability.summary, area_name: areas.get(capability.area_id)?.name ?? capability.area_id, journey_names: (capability.journey_ids ?? []).map(id => journeys.get(id)?.name ?? id), term_refs: unique([...(capability.term_refs ?? []), ...(variant.terms ?? [])]) }))) };
}

export function stateSummary(state) {
  if (state.release === 'unsupported') return '明确不支持';
  if (state.system_validation === 'passed' && state.automation === 'passed') return '系统验证通过';
  if (state.automation === 'passed') return '自动化已通过，待系统验证';
  if (state.automation === 'partial') return '自动化部分覆盖';
  return state.delivery === 'implemented' ? '已实现，待补证据' : '待开发';
}

export function renderProductCapabilityIndex(view, audit) {
  const rows = view.variants.map(variant => [
    variant.id,
    variant.name,
    variant.journey_names.join('、'),
    variant.formats.join(' / '),
    variant.canonical_chain ?? '—',
    variant.state.automation,
    variant.state.system_validation,
    stateSummary(variant.state),
  ]);
  const table = ['| 变体 | 能力 | 用户旅程 | 格式 | 正式主链 | 自动化 | 系统验证 | 当前结论 |', '|---|---|---|---|---|---|---|---|', ...rows.map(row => `| ${row.join(' | ')} |`)].join('\n');
  return `> 来源：\`.governance/product-capability-registry.json\`；由 \`product:generate\` 和 \`product:audit\` 派生。\n> ${audit.status === 'passed' ? '注册表追溯审计通过。' : `注册表有 ${audit.findings.length} 个阻断问题，能力结论不可晋级。`}\n\n${table}`;
}

export function renderCapabilityAtlas(view, audit) {
  const stages = unique(view.journeys.flatMap(journey => journey.stages ?? []));
  const cards = stages.map(stage => `<section class="stage"><h3>${html(stage)}</h3>${view.variants.filter(variant => variant.stage === stage).map(variant => `<button class="card ${html(variant.support)}" data-variant-id="${html(variant.id)}"><small>${html(variant.actor === 'agent' ? '代理' : '用户')} · ${html(variant.formats.join(' / '))}</small><b>${html(variant.name)}</b><span>${html(stateSummary(variant.state))}</span></button>`).join('') || '<p>暂无已登记能力</p>'}</section>`).join('');
  const formats = unique(view.variants.flatMap(variant => variant.formats));
  const matrix = view.variants.map(variant => `<tr><th>${html(variant.name)}</th>${formats.map(format => `<td>${variant.formats.includes(format) ? html(variant.support) : '—'}</td>`).join('')}</tr>`).join('');
  const gaps = view.variants.filter(variant => variant.state.automation !== 'passed' || !['passed', 'not_required'].includes(variant.state.system_validation));
  const clientScript = `const variants=${JSON.stringify(view.variants).replaceAll('<', '\\u003c')};const esc=v=>String(v??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));const detail=document.querySelector('#detail');document.querySelectorAll('[data-variant-id]').forEach(button=>button.addEventListener('click',()=>{const v=variants.find(item=>item.id===button.dataset.variantId);if(!v)return;detail.innerHTML='<h2>'+esc(v.name)+'</h2><p>'+esc(v.capability_summary)+'</p><dl><dt>格式</dt><dd>'+esc(v.formats.join(' / '))+'</dd><dt>操作</dt><dd>'+esc(v.operation)+'</dd><dt>支持边界</dt><dd>'+esc(v.support)+'</dd><dt>正式主链</dt><dd>'+esc(v.canonical_chain||'不适用')+'</dd></dl><h3>状态</h3><ul>'+Object.entries(v.state).map(([k,value])=>'<li>'+esc(k)+'：'+esc(value)+'</li>').join('')+'</ul><h3>验收点</h3><ul>'+v.acceptance.map(item=>'<li>'+esc(item)+'</li>').join('')+'</ul><h3>证据</h3><p>测试：'+esc(v.evidence.tests.join('、')||'暂无')+'<br>验证：'+esc(v.evidence.validations.join('、')||'暂无')+'</p>'; }));`;
  return `<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Binder 产品能力沙盘</title><style>body{margin:0;background:#f5f7fb;color:#1b2430;font:14px Inter,"PingFang SC",system-ui}.page{max-width:1440px;margin:auto;padding:32px}.hero{display:flex;justify-content:space-between;gap:24px;align-items:end}.hero h1{margin:0}.hero p{color:#5b6775}.summary span{display:inline-block;background:#e6edf7;border-radius:999px;padding:5px 9px;margin:3px}.story{display:grid;grid-template-columns:repeat(auto-fit,minmax(190px,1fr));gap:12px;margin:24px 0}.stage,.panel{background:#fff;border:1px solid #e1e7ef;border-radius:14px;padding:15px}.stage{min-height:240px}.stage h3,.panel h2{margin-top:0}.card{display:block;width:100%;text-align:left;margin:8px 0;padding:10px;border:1px solid #dce4ee;border-left:4px solid #7894b2;background:#fbfcfe;border-radius:9px;cursor:pointer}.card b,.card span,.card small{display:block;margin:4px 0}.card small,.card span{font-size:12px;color:#657487}.card.supported{border-left-color:#2a9d68}.card.partial{border-left-color:#d2941d}.layout{display:grid;grid-template-columns:minmax(0,1fr) 340px;gap:18px}.matrix{overflow:auto}.matrix table{border-collapse:collapse;width:100%}.matrix th,.matrix td{padding:9px;text-align:left;border-bottom:1px solid #edf0f4}.gap li{margin:8px 0}@media(max-width:850px){.hero,.layout{display:block}.panel{margin:12px 0}}</style></head><body><main class="page"><header class="hero"><div><p>PCG-SPEC · 自动生成，禁止手工编辑</p><h1>Binder 产品能力沙盘</h1><p>按用户完成文档协作的叙事浏览产品能力；点击卡片查看范围、证据与主链。</p></div><div class="summary"><span>${view.variants.length} 个能力变体</span><span>${audit.summary.automation.passed ?? 0} 自动化通过</span><span>${gaps.length} 待闭合</span></div></header><section class="story" aria-label="产品故事地图">${cards}</section><section class="layout"><div><section class="panel" id="detail"><h2>选择一个能力查看详情</h2><p>产品定义、实现、自动化、系统验证和发布承诺独立呈现。</p></section><section class="panel matrix"><h2>格式支持矩阵</h2><table><thead><tr><th>能力变体</th>${formats.map(format => `<th>${html(format)}</th>`).join('')}</tr></thead><tbody>${matrix}</tbody></table></section></div><aside><section class="panel"><h2>当前缺口</h2><ul class="gap">${gaps.map(variant => `<li><b>${html(variant.name)}</b><br>${html(stateSummary(variant.state))}</li>`).join('')}</ul></section><section class="panel"><h2>注册表健康</h2><p>${audit.status === 'passed' ? '追溯与主链审计通过。' : `发现 ${audit.findings.length} 个阻断问题。`}</p><p>${audit.warnings.length ? `${audit.warnings.length} 个待治理候选。` : '暂无重复实现候选。'}</p></section></aside></section></main><script>${clientScript}</script></body></html>`;
}

export function renderChainAuditMarkdown(audit, view) {
  const variants = view.variants.filter(variant => variant.side_effect);
  const list = (records, fallback) => records.length ? records.map(record => `- **${record.type}**：${record.message}（${record.variant ?? record.capability ?? 'registry'}）`).join('\n') : fallback;
  return `# 产品能力主链审计报告\n\n> 自动生成，禁止手工编辑。\n\n## 正式主链\n\n| 能力变体 | 正式主链 | fallback | 状态 |\n|---|---|---|---|\n${variants.map(variant => `| ${variant.name} | ${variant.canonical_chain ?? 'MISSING'} | ${(variant.fallbacks ?? []).map(fallback => fallback.when).join('；') || '无'} | ${stateSummary(variant.state)} |`).join('\n')}\n\n## 阻断问题\n\n${list(audit.findings, '- 无阻断问题。')}\n\n## 重复候选与警告\n\n${list(audit.warnings, '- 无待确认候选。')}\n`;
}

export function writeProductCapabilityArtifacts(root, config, view, audit) {
  if (!config.product_capability_output_dir) throw new Error('governance.config.json 缺少 product_capability_output_dir');
  const output = join(root, config.product_capability_output_dir);
  mkdirSync(output, { recursive: true });
  writeFileSync(join(output, 'product-capability-atlas.html'), renderCapabilityAtlas(view, audit));
  writeFileSync(join(output, 'product-capability-coverage.json'), `${JSON.stringify({ generated_at: new Date().toISOString(), summary: audit.summary, audit_status: audit.status, variants: view.variants }, null, 2)}\n`);
  writeFileSync(join(output, 'product-capability-chain-audit.md'), renderChainAuditMarkdown(audit, view));
}
