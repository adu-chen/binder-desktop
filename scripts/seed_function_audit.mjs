#!/usr/bin/env node

/**
 * 功能状态树 · L5 审计数组种子化脚本（一次性初始化）
 *
 * 读取 .governance/function-ledger.json，为每个 V1 范围内的 L5 原子行为
 * 生成 feature_audit 条目（key = 台账行 id），按"实现健康度"三态：
 *
 *   designed  已设计为实现，尚未实现（旧状态：规划·V1+）
 *   unclosed  已实现但半实现 / 有缺陷 / 有残留（旧状态：已实现·待核验·EP 且带缺陷信号）
 *   closed    已实现且正常完整（旧状态：已实现·待核验·EP / 待审计·EP 无缺陷信号，
 *             默认按正常处理，最终由人工审计兜底确认）
 *
 * 聚合到 L4 分支的状态由生成器按"最差进展优先"（设计 < 未闭合 < 已闭合）计算，
 * 本脚本只维护 L5 级原始状态。
 *
 * V1 范围：排除「明确不提供·V1」「概念·VNext」行。
 * 图谱 automation=passed 的 REQ 覆盖仅预填 evidence 作为佐证徽标，不决定状态。
 *
 * 用法：
 *   node scripts/seed_function_audit.mjs            # 只打印草案，不写文件
 *   node scripts/seed_function_audit.mjs --apply    # 写回 function-ledger.json
 *
 * feature_audit 为人工维护机器源；本脚本仅在首次初始化时执行，
 * --apply 前若已存在 feature_audit 条目则拒绝覆盖（除非 --force）。
 */

/** @GOV
 * codes: BR-PCG-DATA-001-RB-PCG-PCG-GENERATE-003
 * type: RB
 * chain: PCG-GENERATE
 * rules: BR-PCG-DATA-001, BR-PCG-STATE-001
 * boundary: in=FunctionLedger data source + ProductCapability registry automation evidence | out=feature_audit seed draft in function-ledger.json | effect=WS-function-audit-seed | does_not=mutate_rule_or_chain_registry
 * term_ref: TERM-PCG-001
 */

import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(SCRIPT_DIR, '..');
const LEDGER_PATH = join(ROOT, '.governance/function-ledger.json');
const REGISTRY_PATH = join(ROOT, '.governance/product-capability-registry.json');

const STATUS_TO_STATE = {
  '已实现 · 待核验 · EP': 'closed',
  '待审计 · EP': 'closed',
  '规划 · V1+': 'designed',
};
// 已实现行上的缺陷/未决信号词（命中即视为"未闭合"：半实现/有缺陷/有残留）
const SIGNALS = ['补全候选', '待确认', '待专项审计', '残留', '待修复', '缺陷', '缺口', '待定', '不完整', '风险', '待审计'];

const REQ_RE = /REQ-[A-Z]+-\d+/g;

/** 递归收集所有 state.automation === 'passed' 的变体（id + requirements） */
function collectPassedVariants(node, out) {
  if (!node || typeof node !== 'object') return;
  if (node.state && node.state.automation === 'passed' && Array.isArray(node.requirements)) {
    out.push({ id: node.id, requirements: node.requirements });
  }
  for (const v of Object.values(node)) {
    if (Array.isArray(v)) {
      for (const item of v) collectPassedVariants(item, out);
    } else {
      collectPassedVariants(v, out);
    }
  }
}

function reqIdsOf(note) {
  const ids = String(note ?? '').match(REQ_RE) ?? [];
  return [...new Set(ids)];
}

function signalsOf(row) {
  return SIGNALS.filter((s) => String(row.note ?? '').includes(s) || String(row.detail ?? '').includes(s));
}

function buildSeed(rows, passedVariants) {
  const seed = [];
  for (const row of rows) {
    if (row.status.includes('明确不提供') || row.status.includes('VNext')) continue;

    const noteParts = ['种子'];
    let state;

    if (row.status === '已实现 · 待核验 · EP') {
      const sigs = signalsOf(row);
      if (sigs.length > 0) {
        state = 'unclosed';
        noteParts.push(`缺陷信号：${sigs.join('/')}，需人工复核`);
      } else {
        state = 'closed';
        noteParts.push('默认按已实现正常，请人工确认');
      }
    } else if (row.status === '待审计 · EP') {
      state = 'closed';
      noteParts.push('待审计项请人工确认');
    } else if (row.status === '规划 · V1+') {
      state = 'designed';
      noteParts.push('规划项');
    } else {
      state = 'designed';
    }

    // 图谱 automation=passed 证据候选：仅预填 evidence 作为佐证徽标，不决定状态
    const reqs = reqIdsOf(row.note);
    const matched = passedVariants.filter((v) => reqs.length > 0 && reqs.every((r) => v.requirements.includes(r)));
    const evidence = matched.length > 0 ? { kind: 'atlas_automation', refs: [...new Set(matched.map((v) => v.id))] } : null;
    if (evidence) noteParts.push('图谱证据候选（见 evidence），确认后升已闭合');

    seed.push({
      key: row.id,
      verify_state: state,
      chain_ids: [],
      rule_ids: [],
      evidence,
      note: noteParts.join('；'),
      audited_at: null,
    });
  }

  seed.sort((a, b) => a.key.localeCompare(b.key, 'zh-CN'));
  return seed;
}

function main() {
  const apply = process.argv.includes('--apply');
  const force = process.argv.includes('--force');

  const ledger = JSON.parse(readFileSync(LEDGER_PATH, 'utf8'));
  const registry = JSON.parse(readFileSync(REGISTRY_PATH, 'utf8'));
  const passedVariants = [];
  collectPassedVariants(registry, passedVariants);

  const seed = buildSeed(ledger.rows, passedVariants);

  const counts = seed.reduce(
    (a, e) => ((a[e.verify_state] = (a[e.verify_state] ?? 0) + 1), a),
    {},
  );
  const withEvidence = seed.filter((e) => e.evidence).length;
  const withSignal = seed.filter((e) => e.note.includes('缺陷信号')).length;
  const summary = {
    features: seed.length,
    with_defect_signal: withSignal,
    evidence_candidates: withEvidence,
    evidence_variants: passedVariants.map((v) => v.id),
    counts,
  };

  if (!apply) {
    process.stdout.write(`${JSON.stringify({ status: 'dry-run', ...summary, feature_audit: seed }, null, 1)}\n`);
    return;
  }

  if (!force && Array.isArray(ledger.feature_audit) && ledger.feature_audit.length > 0) {
    console.error('feature_audit 已存在，拒绝覆盖。使用 --force 以强制执行。');
    process.exitCode = 1;
    return;
  }

  ledger.schema_version = 2;
  ledger.feature_audit = seed;
  writeFileSync(LEDGER_PATH, `${JSON.stringify(ledger, null, 1)}\n`);
  process.stdout.write(`${JSON.stringify({ status: 'applied', ...summary }, null, 2)}\n`);
}

main();
