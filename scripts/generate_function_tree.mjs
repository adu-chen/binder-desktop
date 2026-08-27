#!/usr/bin/env node

/**
 * 功能状态树生成器（审计人类友好视图，独立于 ADUS 生成逻辑）
 *
 * 读取 .governance/function-ledger.json（含 L5 级 feature_audit 审计数组），
 * 过滤 V1 范围，聚合 L1→L2→L3→L4 分支 + L5 原子行为叶子，并对照 ADUS
 * 链路/规则注册表校验 chain_ids / rule_ids 引用，输出自包含单文件 HTML：
 *   docs/00_core/generated/feature-status-tree.html
 *
 * 三态口径（实现健康度，L5 级维护）：
 *   designed  已设计为实现，尚未实现
 *   unclosed  已实现但半实现 / 有缺陷 / 有残留
 *   closed    已实现且正常完整（默认按台账"已实现"状态，最终人工审计兜底确认；
 *             evidence 为判定留痕，非硬性门槛）
 *
 * L4 分支状态按"最差进展优先"聚合（设计 < 未闭合 < 已闭合）。
 * 定位：仅作人工审计与跟进的人类友好视图；不写入、不修改 ADUS 任何内容，
 * 对 ADUS 仅做只读校验。
 */

/** @GOV
 * codes: BR-PCG-DATA-001-RB-PCG-PCG-GENERATE-004
 * type: RB
 * chain: PCG-GENERATE
 * rules: BR-PCG-DATA-001, BR-PCG-STATE-001
 * boundary: in=FunctionLedger feature_audit (L5) + ADUS chain/rule registries (read-only) | out=generated feature-status-tree.html | effect=WS-feature-status-tree | does_not=mutate_adus_or_function_ledger
 * term_ref: TERM-PCG-001
 */

import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(SCRIPT_DIR, '..');
const LEDGER_PATH = join(ROOT, '.governance/function-ledger.json');
const ADUS_PATH = join(ROOT, 'docs/00_core/A-CORE-C-R-01_ADUS.md');
const OUTPUT_DIR = join(ROOT, 'docs/00_core/generated');
const OUTPUT_PATH = join(OUTPUT_DIR, 'feature-status-tree.html');

const VERIFY_STATES = ['designed', 'unclosed', 'closed'];
const STATE_LABEL = { designed: '设计', unclosed: '未闭合', closed: '已闭合' };
const EVIDENCE_KINDS = new Set(['atlas_automation', 'test_related', 'validation_card', 'manual_audit']);

const esc = (v) =>
  String(v ?? '')
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;');

/** 解析 ADUS 中的 Markdown 表格首列 ID（链路/规则注册表，只读） */
function parseAdusTableIds(sectionHeader) {
  const text = readFileSync(ADUS_PATH, 'utf8');
  const lines = text.split('\n');
  const ids = new Set();
  let collecting = false;
  for (const line of lines) {
    if (line.includes(sectionHeader)) {
      collecting = true;
      continue;
    }
    if (collecting) {
      if (/^#{2,3}\s/.test(line)) break;
      if (/^\|/.test(line)) {
        const cells = line
          .split('|')
          .map((s) => s.trim())
          .filter(Boolean);
        if (cells.length >= 2 && !/^-+$/.test(cells[0])) ids.add(cells[0]);
      }
    }
  }
  return ids;
}

/** 过滤 V1 范围行，按 L1→L2→L3→L4 聚合成树，L5 为叶子（携带 audit） */
function buildTree(rows, auditById) {
  const root = new Map();
  for (const row of rows) {
    if (row.status.includes('明确不提供') || row.status.includes('VNext')) continue;
    if (!root.has(row.l1)) root.set(row.l1, new Map());
    const l2map = root.get(row.l1);
    if (!l2map.has(row.l2)) l2map.set(row.l2, new Map());
    const l3map = l2map.get(row.l2);
    if (!l3map.has(row.l3)) l3map.set(row.l3, new Map());
    const l4map = l3map.get(row.l3);
    if (!l4map.has(row.l4)) l4map.set(row.l4, []);
    l4map.get(row.l4).push({
      id: row.id,
      name: row.l5,
      detail: row.detail ?? '',
      acceptance: row.acceptance ?? '',
      status: row.status ?? '',
      note: row.note ?? '',
      audit: auditById.get(row.id) ?? null,
    });
  }
  return root;
}

/** 三态聚合：设计 < 未闭合 < 已闭合，取最差进展 */
function aggState(items) {
  const d = items.filter((i) => i.audit?.verify_state === 'designed').length;
  const u = items.filter((i) => i.audit?.verify_state === 'unclosed').length;
  const c = items.filter((i) => i.audit?.verify_state === 'closed').length;
  let state = 'closed';
  if (u > 0) state = 'unclosed';
  else if (d > 0) state = 'designed';
  return { d, u, c, state };
}

function validateTree(root, chains, rules) {
  const errors = [];
  const warn = [];
  const usedIds = new Set();
  let l5Count = 0;
  for (const l1 of root.values()) {
    for (const l2 of l1.values()) {
      for (const l3 of l2.values()) {
        for (const items of l3.values()) {
          for (const item of items) {
            l5Count += 1;
            usedIds.add(item.id);
            const a = item.audit;
            if (!a) {
              errors.push(`L5 缺少 feature_audit 条目: ${item.id}`);
              continue;
            }
            if (!VERIFY_STATES.includes(a.verify_state)) {
              errors.push(`verify_state 非法 (${a.verify_state}): ${item.id}`);
            }
            const ev = a.evidence;
            if (a.verify_state === 'closed' && (!ev || !Array.isArray(ev.refs) || ev.refs.length === 0)) {
              warn.push(`closed 未携带证据（默认按台账已实现状态，建议审计时补充 evidence）: ${item.id}`);
            }
            if (ev && ev.kind && !EVIDENCE_KINDS.has(ev.kind)) {
              errors.push(`evidence.kind 非法 (${ev.kind}): ${item.id}`);
            }
            for (const c of a.chain_ids ?? []) {
              if (!chains.has(c)) errors.push(`未知链路 ${c}: ${item.id}`);
            }
            for (const r of a.rule_ids ?? []) {
              if (!rules.has(r)) errors.push(`未知规则 ${r}: ${item.id}`);
            }
          }
        }
      }
    }
  }
  const ledger = JSON.parse(readFileSync(LEDGER_PATH, 'utf8'));
  for (const a of ledger.feature_audit ?? []) {
    if (!usedIds.has(a.key)) warn.push(`feature_audit 存在但树中无此 L5: ${a.key}`);
  }
  return { errors, warn, l5Count };
}

function computeCounts(root) {
  const counts = { designed: 0, unclosed: 0, closed: 0, total: 0 };
  const chainCount = new Map();
  for (const l1 of root.values()) {
    for (const l2 of l1.values()) {
      for (const l3 of l2.values()) {
        for (const items of l3.values()) {
          for (const item of items) {
            const st = item.audit?.verify_state ?? 'unclosed';
            counts[st] += 1;
            counts.total += 1;
            for (const c of item.audit?.chain_ids ?? []) {
              chainCount.set(c, (chainCount.get(c) ?? 0) + 1);
            }
          }
        }
      }
    }
  }
  return { counts, chainCount };
}

/** 序列化树为 DATA.tree：L4 分支带聚合统计，L5 叶子带审计展示字段 */
function serializeTree(root) {
  const out = [];
  for (const [l1, l2map] of root) {
    const l2s = [];
    for (const [l2, l3map] of l2map) {
      const l3s = [];
      for (const [l3, l4map] of l3map) {
        const l4s = [];
        for (const [l4, items] of l4map) {
          const { d, u, c, state } = aggState(items);
          l4s.push({
            l4,
            state,
            d,
            u,
            c,
            items: items.map((x) => ({
              id: x.id,
              name: x.name,
              state: x.audit?.verify_state ?? 'unclosed',
              chains: x.audit?.chain_ids ?? [],
              rules: x.audit?.rule_ids ?? [],
              evidence: x.audit?.evidence ?? null,
              note: x.audit?.note ?? '',
              detail: x.detail,
              acceptance: x.acceptance,
              old_status: x.status,
            })),
          });
        }
        l3s.push({ l3, l4s });
      }
      l2s.push({ l2, l3s });
    }
    out.push({ l1, l2s });
  }
  return out;
}

function renderHtml(root, meta, counts, chainCount, auditText) {
  const treeJson = serializeTree(root);
  const chainsJson = [...chainCount.entries()]
    .map(([id, n]) => ({ id, n }))
    .sort((a, b) => b.n - a.n);
  const data = { meta, counts, chains: chainsJson, tree: treeJson };
  const dataJson = JSON.stringify(data).replace(/</g, '\\u003c');

  return `<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>Binder 功能状态树</title>
<style>
:root{
  color-scheme: light;
  --surface:#F7F7F8; --surface-muted:#EFEFF2;
  --text:#171717; --text-muted:#52525B; --border:rgba(23,23,23,0.12);
  --brand:#4B3FE3; --brand-soft:#F2F7FF; --brand-text:#1A1759;
  --chart-other:#B9BAC4;
  --success:#1DC981; --warning:#EFAA17;
  --radius:8px; --radius-card:12px; --radius-full:999px;
  --spacer-4:4px; --spacer-8:8px; --spacer-12:12px; --spacer-16:16px; --spacer-20:20px; --spacer-24:24px;
  --font-sans:"SF Pro Text","PingFang SC",system-ui,-apple-system,"Segoe UI",Roboto,sans-serif;
  --font-mono:"JetBrains Mono",ui-monospace,"SF Mono",Menlo,Consolas,monospace;
  --weight-regular:400; --weight-medium:500; --weight-strong:600;
  --text-caption:12px/18px; --text-body:14px/20px; --text-title:16px/24px; --text-code:13px/20px;
}
@media (prefers-color-scheme: dark){
  :root{ color-scheme: dark;
    --surface:#171717; --surface-muted:#262626;
    --text:#E5E5E5; --text-muted:#A1A1AA; --border:rgba(229,229,229,0.12);
    --brand:#6054F1; --brand-soft:#1A1759; --brand-text:#CFD8FF;
  }
}
*{box-sizing:border-box;}
body{margin:0; background:var(--surface); color:var(--text);
  font:var(--weight-regular) var(--text-body) var(--font-sans);}
.wrap{max-width:1180px; margin:0 auto; padding:var(--spacer-24) var(--spacer-20) var(--spacer-24);}
.head{margin-bottom:var(--spacer-16);}
.h-title{font:var(--weight-strong) var(--text-title) var(--font-sans); margin:0 0 var(--spacer-8);}
.h-sub{font:var(--weight-regular) var(--text-caption) var(--font-sans); color:var(--text-muted); margin:0;}
.card{background:var(--surface-muted); border:1px solid var(--border); border-radius:var(--radius-card);
  padding:var(--spacer-16); margin-bottom:var(--spacer-16);}
.bar{display:flex; flex-wrap:wrap; gap:var(--spacer-8); align-items:center;}
.bar + .bar{margin-top:var(--spacer-12);}
.lbl{font:var(--weight-regular) var(--text-caption) var(--font-sans); color:var(--text-muted); flex:none;}
.chip{display:inline-flex; align-items:center; gap:var(--spacer-4); border:1px solid var(--border);
  background:var(--surface); border-radius:var(--radius-full); padding:var(--spacer-4) var(--spacer-12);
  font:var(--weight-medium) var(--text-caption) var(--font-sans); cursor:pointer; color:var(--text-muted); user-select:none;}
.chip[data-on="true"]{border-color:var(--brand); color:var(--brand-text); background:var(--brand-soft);}
.chip .num{font:var(--weight-strong) var(--text-code) var(--font-mono);}
.dot{width:8px; height:8px; border-radius:var(--radius-full); display:inline-block; flex:none;}
.dot-d{background:var(--chart-other);} .dot-u{background:var(--warning);} .dot-s{background:var(--success);}
.btn{border:1px solid var(--border); background:var(--surface); color:var(--text-muted); border-radius:var(--radius);
  padding:var(--spacer-4) var(--spacer-12); font:var(--weight-medium) var(--text-caption) var(--font-sans); cursor:pointer;}
.btn:hover{color:var(--brand-text); border-color:var(--brand);}
.search{flex:1 1 180px; min-width:140px; border:1px solid var(--border); background:var(--surface); color:var(--text);
  border-radius:var(--radius); padding:var(--spacer-8) var(--spacer-12); font:var(--weight-regular) var(--text-body) var(--font-sans);}
.search::placeholder{color:var(--text-muted);}
.summary{display:flex; flex-wrap:wrap; gap:var(--spacer-12); align-items:flex-end; margin-bottom:var(--spacer-16);}
.metric{background:var(--surface); border:1px solid var(--border); border-radius:var(--radius-card); padding:var(--spacer-12) var(--spacer-16);}
.metric .v{font:var(--weight-strong) var(--text-title) var(--font-metric);}
.metric .k{font:var(--weight-regular) var(--text-caption) var(--font-sans); color:var(--text-muted);}
.metric .v.d{color:var(--text-muted);} .metric .v.u{color:var(--warning);} .metric .v.s{color:var(--success);}
.progress{height:8px; border-radius:var(--radius-full); background:var(--surface-muted); border:1px solid var(--border); overflow:hidden; flex:1 1 200px; min-width:120px;}
.progress > i{display:block; height:100%; background:var(--success); border-radius:var(--radius-full);}
.tree{display:flex; flex-direction:column; gap:var(--spacer-4);}
.trow{display:flex; align-items:center; gap:var(--spacer-8); border:1px solid transparent; border-radius:var(--radius);
  padding:var(--spacer-8); cursor:pointer; min-width:0; user-select:none;}
.trow[data-toggle]:hover{background:var(--surface-muted);}
.caret{flex:none; color:var(--text-muted); transition:transform .12s ease;}
.caret[data-open="false"]{transform:rotate(-90deg);}
.tname{font:var(--weight-medium) var(--text-body) var(--font-sans); color:var(--text); white-space:nowrap;
  overflow:hidden; text-overflow:ellipsis; flex:1 1 auto; min-width:0;}
.tid{font:var(--weight-medium) var(--text-code) var(--font-mono); color:var(--text-muted); flex:none;}
.tcount{display:inline-flex; align-items:center; gap:var(--spacer-4); font:var(--weight-medium) var(--text-code) var(--font-mono);
  color:var(--text-muted); flex:none;}
.tcount b{font-weight:600;}
.tc-d{color:var(--text-muted);} .tc-u{color:var(--warning);} .tc-s{color:var(--success);}
.tchildren{margin:var(--spacer-4) 0 0 var(--spacer-16); padding-left:var(--spacer-12); border-left:1px solid var(--border);
  display:flex; flex-direction:column; gap:var(--spacer-4);}
.feature{display:flex; align-items:center; gap:var(--spacer-8); padding:var(--spacer-8); border:1px solid transparent;
  border-radius:var(--radius); min-width:0; flex-wrap:wrap;}
.feature:hover{background:var(--surface-muted);}
.fbadge{display:inline-flex; align-items:center; gap:var(--spacer-4); border-radius:var(--radius-full); padding:0 var(--spacer-8);
  font:var(--weight-medium) var(--text-caption) var(--font-sans); border:1px solid var(--border); background:var(--surface); flex:none;}
.badge-s{color:var(--success); border-color:var(--success);}
.badge-u{color:var(--warning); border-color:var(--warning);}
.badge-d{color:var(--text-muted);}
.tag{display:inline-flex; align-items:center; border:1px solid var(--border); border-radius:var(--radius-full);
  padding:0 var(--spacer-8); font:var(--weight-medium) var(--text-caption) var(--font-sans); color:var(--text-muted);
  background:var(--surface-muted); cursor:pointer; flex:none;}
.tag[data-on="true"]{color:var(--brand-text); border-color:var(--brand); background:var(--brand-soft);}
.tag.ev{color:var(--brand-text); border-color:var(--brand); background:var(--brand-soft);}
.fnote{font:var(--weight-regular) var(--text-caption) var(--font-sans); color:var(--text-muted);}
.fdesc{font:var(--weight-regular) var(--text-caption) var(--font-sans); color:var(--text-muted);}
.empty{font:var(--weight-regular) var(--text-body) var(--font-sans); color:var(--text-muted); padding:var(--spacer-16); display:none;}
</style>
</head>
<body>
<div class="wrap">
  <header class="head">
    <h1 class="h-title">Binder 功能状态树</h1>
    <p class="h-sub">功能审计与跟进视图（L5 原子行为粒度）· 生成时间：${esc(meta.generated_at)} · 数据源：${esc(meta.source)} · 范围：V1（不含「明确不提供 · V1」「概念 · VNext」）</p>
  </header>

  <div class="summary">
    <div class="metric"><div class="v d" data-total-d>0</div><div class="k">设计</div></div>
    <div class="metric"><div class="v u" data-total-u>0</div><div class="k">未闭合</div></div>
    <div class="metric"><div class="v s" data-total-s>0</div><div class="k">已闭合</div></div>
    <div class="metric"><div class="v" data-total>0</div><div class="k">合计</div></div>
    <div class="progress" title="已闭合占比"><i data-progress style="width:0%"></i></div>
  </div>

  <div class="card">
    <div class="bar">
      <span class="lbl">状态</span>
      <button class="chip" data-state="designed" data-on="true"><span class="dot dot-d"></span>设计 <span class="num" data-n-d>0</span></button>
      <button class="chip" data-state="unclosed" data-on="true"><span class="dot dot-u"></span>未闭合 <span class="num" data-n-u>0</span></button>
      <button class="chip" data-state="closed" data-on="true"><span class="dot dot-s"></span>已闭合 <span class="num" data-n-s>0</span></button>
    </div>
    <div class="bar">
      <span class="lbl">链路</span>
      <span id="chain-chips"></span>
    </div>
    <div class="bar">
      <span class="lbl">检索</span>
      <input class="search" id="search" type="text" placeholder="按功能名 / ID / 验收关键词搜索" />
      <button class="btn" id="expand-all">全部展开</button>
      <button class="btn" id="collapse-all">全部折叠</button>
    </div>
  </div>

  <div id="tree-root" class="tree"></div>
  <div class="empty" id="empty">当前筛选下没有匹配的功能。</div>

  <footer class="foot">
    状态口径（L5 原子行为）：设计 = 已设计为实现尚未实现；未闭合 = 已实现但半实现 / 有缺陷 / 有残留；已闭合 = 已实现且正常完整（默认按台账"已实现"状态，最终人工审计兜底）。
    分支（L4）状态为最差进展聚合。链路 / 规则引用经 ADUS 注册表校验（只读）。审计状态由人工维护于 .governance/function-ledger.json 的 feature_audit（key = L5 行 id）。
    校验：${esc(auditText)} · L5 原子行为数：${counts.total}
  </footer>
</div>
<script>
(function(){
  var DATA = ${dataJson};
  var root = document.getElementById('tree-root');
  var chainChipsHost = document.getElementById('chain-chips');

  function escHtml(s){ return String(s==null?'':s).replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;'); }
  function stateLabel(s){ return s==='designed'?'设计':s==='closed'?'已闭合':'未闭合'; }
  function countBadge(node){
    return '<span class="tcount"><b class="tc-d">'+node.d+'</b>/<b class="tc-u">'+node.u+'</b>/<b class="tc-s">'+node.c+'</b></span>';
  }
  function leafHtml(f){
    var cls = f.state==='designed'?'badge-d':f.state==='closed'?'badge-s':'badge-u';
    var dcls = f.state==='designed'?'dot-d':f.state==='closed'?'dot-s':'dot-u';
    var tags = '';
    (f.chains||[]).forEach(function(c){ tags += '<button class="tag" data-chain="'+escHtml(c)+'">'+escHtml(c)+'</button>'; });
    (f.rules||[]).forEach(function(r){ tags += '<button class="tag" data-rule="'+escHtml(r)+'">'+escHtml(r)+'</button>'; });
    var evHtml = '';
    if(f.evidence){
      var evLabels = {atlas_automation:'图谱证据', test_related:'测试通过', validation_card:'验证卡', manual_audit:'人工确认'};
      evHtml = '<span class="tag ev" title="'+escHtml((f.evidence.refs||[]).join(' / '))+'">'+escHtml(evLabels[f.evidence.kind]||f.evidence.kind)+'</span>';
    }
    var descParts = [];
    if(f.detail) descParts.push(f.detail);
    if(f.acceptance) descParts.push('验收：'+f.acceptance);
    var tip = descParts.join('｜');
    return '<div class="feature" data-state="'+f.state+'" data-chains="'+(f.chains||[]).join('|')+'" data-rules="'+(f.rules||[]).join('|')+'" data-text="'+escHtml((f.id+' '+f.name+' '+(descParts.join(' '))+' '+(f.note||'')).toLowerCase())+'" title="'+escHtml(tip)+'">' +
      '<span class="tid">'+escHtml(f.id)+'</span>' +
      '<span class="tname">'+escHtml(f.name)+'</span>' +
      '<span class="fbadge '+cls+'"><span class="dot '+dcls+'"></span>'+stateLabel(f.state)+'</span>' + evHtml + tags +
      (f.note?'<span class="fnote">'+escHtml(f.note)+'</span>':'') +
      (f.old_status?'<span class="fdesc">'+escHtml(f.old_status)+'</span>':'') +
      '</div>';
  }
  function branchHtml(node){
    var caret = '<svg class="caret" data-open="true" viewBox="0 0 8 8" width="8" height="8"><path d="M2 1.5 L6 4 L2 6.5 Z" fill="currentColor"/></svg>';
    var head = '<div class="trow" data-toggle>'+caret+(node.id?'<span class="tid">'+escHtml(node.id)+'</span>':'')+'<span class="tname">'+escHtml(node.name)+'</span>'+countBadge(node)+'</div>';
    var inner;
    if(node.features){
      inner = (node.features||[]).map(leafHtml).join('');
    } else {
      inner = (node.children||[]).map(branchHtml).join('');
    }
    return '<div data-branch>' + head + '<div class="tchildren" data-open="true">' + inner + '</div></div>';
  }

  function buildTreeDom(){
    var html = '';
    DATA.tree.forEach(function(l1){
      var l1ch = l1.l2s.map(function(l2){
        var l2ch = l2.l3s.map(function(l3){
          var l3ch = l3.l4s.map(function(l4){
            return { name: l4.l4, d:l4.d, u:l4.u, c:l4.c, features: l4.items };
          });
          var d=0,u=0,c=0; l3ch.forEach(function(x){ d+=x.d;u+=x.u;c+=x.c; });
          return { name:l3.l3, d:d,u:u,c:c, children:l3ch };
        });
        var d=0,u=0,c=0; l2ch.forEach(function(x){ d+=x.d;u+=x.u;c+=x.c; });
        return { name:l2.l2, d:d,u:u,c:c, children:l2ch };
      });
      var d=0,u=0,c=0; l1ch.forEach(function(x){ d+=x.d;u+=x.u;c+=x.c; });
      html += branchHtml({ id:l1.l1, name:'', d:d,u:u,c:c, children:l1ch });
    });
    root.innerHTML = html;
  }

  function initChips(){
    DATA.chains.forEach(function(ch){
      var b = document.createElement('button');
      b.className = 'chip';
      b.setAttribute('data-chain-chip', ch.id);
      b.setAttribute('data-on', 'true');
      b.innerHTML = escHtml(ch.id) + ' <span class="num">' + ch.n + '</span>';
      chainChipsHost.appendChild(b);
    });
    document.querySelector('[data-state="designed"] [data-n-d]').textContent = DATA.counts.designed;
    document.querySelector('[data-state="unclosed"] [data-n-u]').textContent = DATA.counts.unclosed;
    document.querySelector('[data-state="closed"] [data-n-s]').textContent = DATA.counts.closed;
    document.querySelector('[data-total-d]').textContent = DATA.counts.designed;
    document.querySelector('[data-total-u]').textContent = DATA.counts.unclosed;
    document.querySelector('[data-total-s]').textContent = DATA.counts.closed;
    document.querySelector('[data-total]').textContent = DATA.counts.total;
    document.querySelector('[data-progress]').style.width = (DATA.counts.total? Math.round(DATA.counts.closed/DATA.counts.total*100):0) + '%';
  }

  function onStates(){
    var out = [];
    ['designed','unclosed','closed'].forEach(function(s){
      var c = root.parentNode.querySelector('[data-state="'+s+'"]');
      if(c && c.getAttribute('data-on')==='true') out.push(s);
    });
    return out;
  }
  function selChains(){
    var out = [];
    Array.prototype.forEach.call(chainChipsHost.querySelectorAll('[data-chain-chip]'), function(c){
      if(c.getAttribute('data-on')==='true') out.push(c.getAttribute('data-chain-chip'));
    });
    return out;
  }
  function applyFilter(){
    var states = onStates();
    var chains = selChains();
    var q = document.getElementById('search').value.trim().toLowerCase();
    var anyChain = chains.length > 0;
    root.querySelectorAll('.feature[data-state]').forEach(function(f){
      var st = f.getAttribute('data-state');
      var fchains = (f.getAttribute('data-chains')||'').split('|').filter(Boolean);
      var okState = states.indexOf(st) >= 0;
      var okChain = anyChain ? fchains.some(function(c){ return chains.indexOf(c) >= 0; }) : true;
      var okSearch = !q || (f.getAttribute('data-text')||'').indexOf(q) >= 0;
      f.style.display = (okState && okChain && okSearch) ? '' : 'none';
    });
    root.querySelectorAll('[data-branch]').forEach(function(b){
      var visible = false;
      b.querySelectorAll('.feature[data-state]').forEach(function(f){ if(f.style.display!=='none') visible=true; });
      b.style.display = visible ? '' : 'none';
    });
    root.querySelectorAll('.tag[data-chain]').forEach(function(t){
      t.setAttribute('data-on', String(chains.indexOf(t.getAttribute('data-chain')) >= 0));
    });
    var any = false;
    root.querySelectorAll('.feature[data-state]').forEach(function(f){ if(f.style.display!=='none') any=true; });
    document.getElementById('empty').style.display = any ? 'none' : '';
    var d=0,u=0,c=0;
    root.querySelectorAll('.feature[data-state]').forEach(function(f){
      if(f.style.display!=='none'){
        if(f.getAttribute('data-state')==='designed')d++; else if(f.getAttribute('data-state')==='unclosed')u++; else c++;
      }
    });
    document.querySelector('[data-total-d]').textContent = d;
    document.querySelector('[data-total-u]').textContent = u;
    document.querySelector('[data-total-s]').textContent = c;
    document.querySelector('[data-total]').textContent = d+u+c;
    document.querySelector('[data-progress]').style.width = ((d+u+c)? Math.round(c/(d+u+c)*100):0) + '%';
  }

  root.parentNode.addEventListener('click', function(e){
    var toggle = e.target.closest('.trow[data-toggle]');
    if(toggle){
      var ch = toggle.nextElementSibling;
      if(ch && ch.classList.contains('tchildren')){
        var open = ch.getAttribute('data-open')==='true';
        ch.setAttribute('data-open', String(!open));
        ch.style.display = open ? 'none' : '';
        var caret = toggle.querySelector('.caret');
        if(caret) caret.setAttribute('data-open', String(!open));
      }
      return;
    }
    var stateChip = e.target.closest('.chip[data-state]');
    if(stateChip){ stateChip.setAttribute('data-on', String(stateChip.getAttribute('data-on')!=='true')); applyFilter(); return; }
    var chainChip = e.target.closest('[data-chain-chip]');
    if(chainChip){ chainChip.setAttribute('data-on', String(chainChip.getAttribute('data-on')!=='true')); applyFilter(); return; }
    var tag = e.target.closest('.tag[data-chain]');
    if(tag){
      var cid = tag.getAttribute('data-chain');
      var chip = chainChipsHost.querySelector('[data-chain-chip="'+cid+'"]');
      var cur = chip && chip.getAttribute('data-on')==='true';
      Array.prototype.forEach.call(chainChipsHost.querySelectorAll('[data-chain-chip]'), function(c){ c.setAttribute('data-on','false'); });
      if(!cur && chip) chip.setAttribute('data-on','true');
      applyFilter();
      return;
    }
  });
  document.getElementById('search').addEventListener('input', applyFilter);
  document.getElementById('expand-all').addEventListener('click', function(){ setAll(true); });
  document.getElementById('collapse-all').addEventListener('click', function(){ setAll(false); });
  function setAll(open){
    root.querySelectorAll('[data-branch] > .tchildren').forEach(function(ch){
      ch.setAttribute('data-open', String(open));
      ch.style.display = open ? '' : 'none';
      var row = ch.previousElementSibling;
      var caret = row && row.querySelector('.caret');
      if(caret) caret.setAttribute('data-open', String(open));
    });
  }

  buildTreeDom();
  initChips();
  applyFilter();
})();
</script>
</body>
</html>
`;
}

function main() {
  const ledger = JSON.parse(readFileSync(LEDGER_PATH, 'utf8'));
  const chains = parseAdusTableIds('### 链路注册表');
  const rules = parseAdusTableIds('### 规则注册表');
  const auditById = new Map((ledger.feature_audit ?? []).map((e) => [e.key, e]));

  const root = buildTree(ledger.rows, auditById);
  const { errors, warn, l5Count } = validateTree(root, chains, rules);
  const { counts, chainCount } = computeCounts(root);

  if (errors.length > 0) {
    console.error(JSON.stringify({ status: 'failed', errors, warn, l5Count, counts }, null, 2));
    process.exitCode = 1;
    return;
  }

  const auditText =
    l5Count === 0
      ? '无 V1 功能'
      : `通过（${l5Count} 个 L5 原子行为，链路 ${chains.size} 条 / 规则 ${rules.size} 条注册表校验）`;
  const meta = {
    generated_at: new Date().toISOString(),
    source: '.governance/function-ledger.json → feature_audit（L5 级人工审计）',
  };

  mkdirSync(OUTPUT_DIR, { recursive: true });
  writeFileSync(OUTPUT_PATH, renderHtml(root, meta, counts, chainCount, auditText));
  process.stdout.write(
    `${JSON.stringify({ status: 'passed', output: 'docs/00_core/generated/feature-status-tree.html', counts, chains: [...chainCount.keys()].length, warnings: warn.length }, null, 2)}\n`,
  );
}

main();
