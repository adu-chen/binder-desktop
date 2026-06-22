/* direction-d.jsx — Direction D · Merged · B's block-stack mark + A's warm paper palette */

const D = {
  ink:    '#0E0C0A',      // warm near-black (from A)
  paper:  '#F4F1EA',      // warm paper (from A)
  fog:    '#E6E2D8',
  card:   '#FFFFFF',
  rule:   'rgba(14,12,10,0.10)',
  muted:  'rgba(14,12,10,0.5)',
  faint:  'rgba(14,12,10,0.3)',
  accent: '#C24A2A',      // clay (from A) — used very sparingly
  green:  '#2E6F4A',      // semantic only
};

// ─── LogoD — parametric block-stack mark with 8 variants
// All variants share the same vertical rhythm (12.5pt) and bar height (8pt).
// `compact` doubles the rhythm for chunky small-size use.
const LOGO_VARIANTS = {
  // 8-bar B-silhouette, left-aligned. Recommended primary mark.
  'default':   { bars: [100, 64, 82, 100, 72, 100, 56, 100], align: 'left' },
  // tighter letter-B rhythm — clearer B-letter reading
  'letter':    { bars: [100, 92, 28, 76, 100, 28, 84, 100],  align: 'left' },
  // bars centered — symmetric figure
  'centered':  { bars: [100, 64, 82, 100, 72, 100, 56, 100], align: 'center' },
  // bars right-aligned — mirrored
  'mirror':    { bars: [100, 64, 82, 100, 72, 100, 56, 100], align: 'right' },
  // every bar full width — paragraph slab
  'slab':      { bars: [100, 100, 100, 100, 100, 100, 100, 100], align: 'left' },
  // V1 + a single accent bar (the "edited paragraph")
  'accented':  { bars: [100, 64, 82, 100, 72, 100, 56, 100], align: 'left', accentBar: 3 },
  // inverted — paper bars in an ink square
  'negative':  { bars: [100, 64, 82, 100, 72, 100, 56, 100], align: 'left', invert: true },
  // four chunky bars for tiny sizes
  'compact':   { bars: [100, 56, 100, 72], align: 'left', chunk: true },
};

function LogoD({ variant = 'accented', size = 120, ink = D.ink, paper = D.paper, accent = D.accent, accentBar, style }) {
  const base = LOGO_VARIANTS[variant] || LOGO_VARIANTS.accented;
  // allow overriding which bar carries the accent (used by dynamic version)
  const v = accentBar != null ? { ...base, accentBar } : base;
  const padX = 14;
  const barH = v.chunk ? 16 : 8;
  const n = v.bars.length;
  const totalW = 120 - 2 * padX;
  // distribute bars vertically with even rhythm — top bar at padY, bottom bar ends at 120 - padY
  const padY = 14;
  const rhythm = (120 - 2 * padY - barH) / (n - 1);

  return (
    <svg viewBox="0 0 120 120" width={size} height={size} style={{ display: 'block', ...style }}>
      {v.invert && <rect width="120" height="120" fill={ink}/>}
      {v.bars.map((w, i) => {
        const barW = totalW * (w / 100);
        let x = padX;
        if (v.align === 'center') x = padX + (totalW - barW) / 2;
        if (v.align === 'right')  x = padX + (totalW - barW);
        const y = padY + i * rhythm;
        const fill = v.invert ? paper : (v.accentBar === i ? accent : ink);
        return <rect key={i} x={x} y={y} width={barW} height={barH} fill={fill}/>;
      })}
    </svg>
  );
}

// Convenience wordmark using A's IBM Plex Sans treatment
function WordmarkD({ size = 56, color = D.ink, withDot = true }) {
  return (
    <span style={{
      fontFamily: "'IBM Plex Sans', 'Inter', sans-serif",
      fontWeight: 500, fontSize: size, letterSpacing: -size * 0.04,
      color, lineHeight: 0.95, display: 'inline-block',
    }}>binder{withDot && <span style={{ color: D.accent }}>.</span>}</span>
  );
}

const MetaD = ({ id, title, sub }) => <MetaStrip id={id} title={title} sub={sub} accent={D.accent}/>;

// ─────────────────────────────────────────────────────────────
// D1 · Hero — chosen primary mark
// ─────────────────────────────────────────────────────────────
function D1_Hero() {
  return (
    <div style={{ width: '100%', height: '100%', background: D.paper, color: D.ink, position: 'relative' }}>
      <MetaD id="VI-D · 01" title="主标 · Primary Lockup" sub="Refined · Block mark + Warm paper"/>
      <div style={{ position: 'absolute', inset: '60px 56px', display: 'flex', flexDirection: 'column', justifyContent: 'space-between' }}>
        <div className="mono" style={{ fontSize: 10, letterSpacing: 1.2, color: D.faint, textTransform: 'uppercase' }}>
          段落条堆叠的 B — 致敬 BlockId 段落系统 / 暖纸墨色系
        </div>

        <div style={{ display: 'flex', alignItems: 'center', gap: 40 }}>
          <LogoD variant="default" size={150}/>
          <div style={{ width: 1, height: 120, background: D.rule }}/>
          <div>
            <WordmarkD size={78}/>
            <div className="sc" style={{ fontSize: 13, color: D.muted, marginTop: 14, letterSpacing: 0.4 }}>
              本地优先的 AI 文档编辑器最小核心
            </div>
            <div className="mono" style={{ fontSize: 11, color: D.faint, marginTop: 4, letterSpacing: 0.8, textTransform: 'uppercase' }}>
              A local-first AI editor — diff as the contract.
            </div>
          </div>
        </div>

        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-end' }}>
          <Caption>① BLOCK-STACK MARK · D-01 DEFAULT</Caption>
          <Caption>② IBM PLEX SANS 500 · −0.04EM</Caption>
          <Caption>RECOMMENDED</Caption>
        </div>
      </div>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────
// D2 · Logo variants — the centerpiece exploration
// ─────────────────────────────────────────────────────────────
function VariantCard({ vKey, label, code, note, recommended, large = false }) {
  return (
    <div style={{
      background: D.card, border: `1px solid ${recommended ? D.accent : D.rule}`,
      padding: large ? 18 : 14, position: 'relative',
      display: 'flex', flexDirection: 'column', gap: 8,
    }}>
      {recommended && (
        <div className="mono" style={{
          position: 'absolute', top: -1, right: -1, padding: '3px 8px',
          background: D.accent, color: D.paper, fontSize: 8, letterSpacing: 1.2,
        }}>RECOMMENDED</div>
      )}
      <div className="mono" style={{ fontSize: 9, color: D.faint, letterSpacing: 0.6 }}>{code}</div>
      <div style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', minHeight: large ? 130 : 88, background: D.paper, borderRadius: 0 }}>
        <LogoD variant={vKey} size={large ? 120 : 78}/>
      </div>
      <div>
        <div className="sc" style={{ fontSize: 12, fontWeight: 500, color: D.ink }}>{label}</div>
        <div className="mono" style={{ fontSize: 9, color: D.muted, marginTop: 2, letterSpacing: 0.3 }}>{note}</div>
      </div>
    </div>
  );
}

function D2_Variants() {
  const variants = [
    { k: 'accented', l: '点缀重点',     c: 'D-06', n: '1 BAR · CLAY ACCENT · SELECTED', rec: true },
    { k: 'default',  l: '默认堆叠',     c: 'D-01', n: '8 BARS · L-ALIGN · CLASSIC' },
    { k: 'letter',   l: '字形紧凑',     c: 'D-02', n: 'TIGHTER B-LETTER · GAP' },
    { k: 'centered', l: '居中对称',     c: 'D-03', n: 'CENTERED · SYMMETRIC' },
    { k: 'mirror',   l: '右对齐',       c: 'D-04', n: 'R-ALIGN · MIRRORED' },
    { k: 'slab',     l: '段落平直',     c: 'D-05', n: 'ALL 100% · DOCUMENT SLAB' },
    { k: 'negative', l: '反白',         c: 'D-07', n: 'INK FILL · PAPER BARS' },
    { k: 'compact',  l: '紧凑四段',     c: 'D-08', n: '4 BARS · CHUNKY · MIN-SIZE' },
  ];
  return (
    <div style={{ width: '100%', height: '100%', background: D.paper, color: D.ink, overflow: 'hidden' }}>
      <MetaD id="VI-D · 02" title="标志候选 · Mark Variants" sub="8 EXPLORATIONS · 选你最喜欢的那个"/>
      <div style={{ padding: '20px 28px 28px' }}>
        <div className="mono" style={{ fontSize: 10, color: D.faint, letterSpacing: 1, marginBottom: 14 }}>
          ① 全部基于同一节律 — 12.5pt 行距、8pt 条高、段落条宽度变化
        </div>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 10 }}>
          {variants.map((v) => (
            <VariantCard key={v.k} vKey={v.k} label={v.l} code={v.c} note={v.n} recommended={v.rec}/>
          ))}
        </div>
        <div style={{ marginTop: 16, padding: '10px 14px', background: D.card, border: `1px solid ${D.rule}`, display: 'flex', alignItems: 'center', gap: 14 }}>
          <div className="mono" style={{ fontSize: 9, color: D.faint, letterSpacing: 0.6 }}>NOTES</div>
          <div style={{ flex: 1, height: 1, background: D.rule }}/>
          <div className="sc" style={{ fontSize: 11, color: D.muted }}>
            <span style={{ color: D.ink, fontWeight: 500 }}>D-01</span> 推荐用于主标 ·
            <span style={{ color: D.ink, fontWeight: 500 }}> D-08</span> 用于 ≤32px 极小尺寸 ·
            <span style={{ color: D.ink, fontWeight: 500 }}> D-07</span> 反白场景 ·
            <span style={{ color: D.ink, fontWeight: 500 }}> D-06</span> 作为动态版（不同应用突出不同段落）
          </div>
        </div>
      </div>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────
// D3 · Dynamic mark — the "shifting paragraph" concept
// ─────────────────────────────────────────────────────────────
function D3_Dynamic() {
  // five "moments" each highlighting a different bar
  const moments = [0, 2, 3, 5, 7];
  return (
    <div style={{ width: '100%', height: '100%', background: D.paper, color: D.ink }}>
      <MetaD id="VI-D · 03" title="动态版 · Dynamic Mark" sub="不同物料突出不同段落 — 同一系统的呼吸"/>
      <div style={{ padding: '28px 32px' }}>
        <Caption style={{ marginBottom: 14 }}>① 5 个时刻 · 每次只点亮一段</Caption>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(5, 1fr)', gap: 12 }}>
          {moments.map((accentBar, i) => (
            <div key={i} style={{ background: D.card, border: `1px solid ${D.rule}`, padding: 14, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 8 }}>
              <svg viewBox="0 0 120 120" width="100" height="100">
                {LOGO_VARIANTS.default.bars.map((w, idx) => {
                  const barW = (120 - 28) * (w / 100);
                  const y = 14 + idx * ((120 - 28 - 8) / 7);
                  return <rect key={idx} x={14} y={y} width={barW} height={8} fill={idx === accentBar ? D.accent : D.ink}/>;
                })}
              </svg>
              <div className="mono" style={{ fontSize: 9, color: D.faint, letterSpacing: 0.6 }}>{`¶ ${accentBar + 1}`}</div>
            </div>
          ))}
        </div>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 24, marginTop: 28 }}>
          <div>
            <Caption style={{ marginBottom: 10 }}>② 应用规则</Caption>
            <div className="sc" style={{ fontSize: 12, lineHeight: 1.7, color: D.muted }}>
              主标使用 <span style={{ color: D.ink, fontWeight: 500 }}>D-01 默认版</span>，
              全黑、无高亮 —— 静默且可靠。<br/>
              动态版 D-06 用于<span style={{ color: D.accent, fontWeight: 500 }}>发布物料、活动海报、版本号场景</span>，
              提示"这次有修改"。
            </div>
          </div>
          <div>
            <Caption style={{ marginBottom: 10 }}>③ 永远只点亮一段</Caption>
            <div style={{ background: D.card, border: `1px solid ${D.rule}`, padding: 14, display: 'flex', alignItems: 'center', gap: 14 }}>
              <LogoD variant="accented" size={80}/>
              <div>
                <div className="mono" style={{ fontSize: 10, color: D.accent }}>v1.0.3 · 1 diff</div>
                <div className="sc" style={{ fontSize: 11, color: D.muted, marginTop: 4 }}>每个版本只改一处，<br/>logo 也只点亮一段。</div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────
// D4 · Construction + clear-space + lockups
// ─────────────────────────────────────────────────────────────
function D4_System() {
  return (
    <div style={{ width: '100%', height: '100%', background: D.paper, color: D.ink }}>
      <MetaD id="VI-D · 04" title="结构 · Construction & Clear Space"/>
      <div style={{ padding: '24px 32px', display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 22 }}>

        <div>
          <Caption style={{ marginBottom: 10 }}>① 构造网格 · 12 × 12 UNITS</Caption>
          <div className="gridbg" style={{ position: 'relative', height: 200, background: D.card, border: `1px solid ${D.rule}` }}>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%' }}>
              <LogoD variant="default" size={150}/>
            </div>
            <div className="mono" style={{ position: 'absolute', top: 4, right: 6, fontSize: 8, color: D.faint, textAlign: 'right', lineHeight: 1.5 }}>
              ROWS = 8<br/>RHYTHM = 12.5<br/>BAR-H = 8
            </div>
          </div>
          <Caption style={{ marginTop: 8 }}>② 行宽对应 B 字曲线垂直采样</Caption>
        </div>

        <div>
          <Caption style={{ marginBottom: 10 }}>③ 安全距离 = 行距 (12.5pt)</Caption>
          <div style={{ position: 'relative', height: 200, background: D.card, border: `1px solid ${D.rule}`, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
            <div style={{ position: 'absolute', inset: 32, border: `1px dashed ${D.faint}` }}/>
            <LogoD variant="default" size={120}/>
          </div>
          <Caption style={{ marginTop: 8 }}>④ 最小尺寸 · MIN 20PX · 切换 D-08</Caption>
        </div>

        <div style={{ gridColumn: 'span 2' }}>
          <Caption style={{ marginBottom: 10 }}>⑤ 组合形式 · LOCKUP OPTIONS</Caption>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 10 }}>
            <div style={{ padding: '14px', background: D.card, border: `1px solid ${D.rule}`, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 8 }}>
              <LogoD size={56}/>
              <Caption>MARK ONLY</Caption>
            </div>
            <div style={{ padding: '14px', background: D.card, border: `1px solid ${D.rule}`, display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 10 }}>
              <LogoD size={40}/>
              <WordmarkD size={22}/>
            </div>
            <div style={{ padding: '14px', background: D.card, border: `1px solid ${D.rule}`, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 6 }}>
              <LogoD size={38}/>
              <WordmarkD size={14}/>
              <div className="mono" style={{ fontSize: 8, color: D.muted, letterSpacing: 1.4 }}>· CORE ·</div>
            </div>
            <div style={{ padding: '14px', background: D.ink, display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 10 }}>
              <LogoD size={40} ink={D.paper}/>
              <WordmarkD size={22} color={D.paper}/>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────
// D5 · Color (A's palette)
// ─────────────────────────────────────────────────────────────
function D5_Color() {
  return (
    <div style={{ width: '100%', height: '100%', background: D.paper, color: D.ink }}>
      <MetaD id="VI-D · 05" title="色彩 · Color System" sub="WARM PAPER + INK + CLAY ACCENT"/>
      <div style={{ padding: '28px 32px' }}>
        <Caption style={{ marginBottom: 12 }}>① 主色 · CORE</Caption>
        <div style={{ display: 'grid', gridTemplateColumns: '2fr 2fr 1fr', gap: 14, marginBottom: 22 }}>
          <Swatch color={D.ink}    name="编辑器墨" code="D-INK"   hex="#0E0C0A" large/>
          <Swatch color={D.paper}  name="文档纸"   code="D-PAPER" hex="#F4F1EA" large/>
          <Swatch color={D.accent} name="批注红"   code="D-CLAY"  hex="#C24A2A" large/>
        </div>
        <Caption style={{ marginBottom: 10 }}>② 中性色阶 · NEUTRAL · 单色系统全部呼吸</Caption>
        <div style={{ display: 'flex', height: 56 }}>
          {[1, 0.92, 0.82, 0.7, 0.58, 0.42, 0.28, 0.16, 0.08, 0].map((v, i) => {
            const c = i === 9 ? D.paper : `rgba(14,12,10,${v})`;
            return (
              <div key={i} style={{ flex: 1, background: c, position: 'relative' }}>
                <div className="mono" style={{ position: 'absolute', bottom: 4, left: 4, fontSize: 8, color: v > 0.5 ? D.paper : D.ink, opacity: 0.7 }}>
                  {`N${(i+1)*100}`}
                </div>
              </div>
            );
          })}
        </div>
        <Caption style={{ marginTop: 14 }}>③ 状态色 · 仅承担 DIFF 语义（不出现在品牌物料中）</Caption>
        <div style={{ display: 'flex', gap: 10, marginTop: 8 }}>
          <div style={{ flex: 1, padding: '10px 14px', background: 'rgba(46,111,74,0.08)', borderLeft: `3px solid ${D.green}` }}>
            <div className="mono" style={{ fontSize: 10, color: D.green }}>+ ACCEPT  #2E6F4A</div>
          </div>
          <div style={{ flex: 1, padding: '10px 14px', background: 'rgba(180,70,55,0.08)', borderLeft: `3px solid #B44637` }}>
            <div className="mono" style={{ fontSize: 10, color: '#B44637' }}>− REJECT  #B44637</div>
          </div>
        </div>
      </div>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────
// D6 · Type (A's type system)
// ─────────────────────────────────────────────────────────────
function D6_Type() {
  return (
    <div style={{ width: '100%', height: '100%', background: D.paper, color: D.ink }}>
      <MetaD id="VI-D · 06" title="字体 · Type System" sub="IBM PLEX FAMILY · NOTO SC"/>
      <div style={{ padding: '28px 32px', display: 'grid', gridTemplateColumns: '1fr 1.2fr', gap: 28 }}>
        <div>
          <div className="mono" style={{ fontSize: 10, color: D.faint, letterSpacing: 0.8 }}>DISPLAY · 显示</div>
          <div style={{ fontFamily: "'IBM Plex Sans', sans-serif", fontWeight: 500, fontSize: 56, letterSpacing: -2, lineHeight: 1, marginTop: 6 }}>
            binder<span style={{ color: D.accent }}>.</span>
          </div>
          <div className="mono" style={{ fontSize: 9, color: D.faint, marginTop: 4 }}>IBM Plex Sans · 500 · −0.04em</div>

          <div className="mono" style={{ fontSize: 10, color: D.faint, letterSpacing: 0.8, marginTop: 22 }}>BODY · 正文</div>
          <div style={{ fontFamily: "'IBM Plex Sans', sans-serif", fontSize: 17, lineHeight: 1.55, marginTop: 6 }}>
            Diff as the contract.<br/>
            <span className="sc">差异即契约。</span>
          </div>

          <div className="mono" style={{ fontSize: 10, color: D.faint, letterSpacing: 0.8, marginTop: 22 }}>MONO · UI / 代码</div>
          <div className="mono" style={{ fontSize: 16, lineHeight: 1.45, marginTop: 6 }}>
            $ binder ./docs<br/>workspace.db
          </div>
        </div>

        <div style={{ borderLeft: `1px solid ${D.rule}`, paddingLeft: 24 }}>
          <Caption style={{ marginBottom: 10 }}>① 字号阶梯</Caption>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 6, marginBottom: 22 }}>
            {[
              [56, 'h1', '显示'], [36, 'h2', '章节'], [22, 'h3', '段题'],
              [16, 'body', '正文'], [13, 'meta', '说明'], [11, 'mono', 'UI'],
            ].map(([sz, n, cn]) => (
              <div key={n} style={{ display: 'flex', alignItems: 'baseline', gap: 14 }}>
                <div className="mono" style={{ width: 38, fontSize: 9, color: D.faint }}>{sz}/{n}</div>
                <div style={{ fontFamily: "'IBM Plex Sans', sans-serif", fontWeight: 500, fontSize: sz, letterSpacing: -sz*0.025, lineHeight: 1 }}>Bb</div>
                <div className="sc" style={{ fontSize: 11, color: D.muted, marginLeft: 'auto' }}>{cn}</div>
              </div>
            ))}
          </div>
          <Caption style={{ marginBottom: 8 }}>② 双语混排 · BILINGUAL</Caption>
          <div style={{ padding: 12, background: D.card, border: `1px solid ${D.rule}` }}>
            <div className="sc" style={{ fontSize: 14, lineHeight: 1.7 }}>
              Binder Core 是一个<span style={{ color: D.accent }}>本地优先</span>的 AI 文档编辑器最小核心，AI 修改以 <span className="mono" style={{ fontSize: 13 }}>diff</span> 形式呈现。
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────
// D7 · Aux + Icons
// ─────────────────────────────────────────────────────────────
function D7_Aux() {
  const icons = [
    { id: 'workspace', path: 'M2 4h12v8H2zM2 7h12' },
    { id: 'file',      path: 'M3 2h7l3 3v9H3zM10 2v3h3' },
    { id: 'diff',      path: 'M3 5h6M3 8h4M3 11h6M11 5v6M9 8h4' },
    { id: 'accept',    path: 'M2 8l3 3 7-7' },
    { id: 'reject',    path: 'M3 3l10 10M13 3L3 13' },
    { id: 'search',    path: 'M3 7a4 4 0 1 1 7.5 2.5L14 13l-1 1-3.5-3.5A4 4 0 0 1 3 7z' },
    { id: 'chat',      path: 'M2 3h12v8H7l-3 3v-3H2z' },
    { id: 'block',     path: 'M3 4h10v3H3zM3 9h7v3H3z' },
  ];
  return (
    <div style={{ width: '100%', height: '100%', background: D.paper, color: D.ink }}>
      <MetaD id="VI-D · 07" title="辅助图形 · Auxiliary & Iconography"/>
      <div style={{ padding: '28px 32px', display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 24 }}>

        <div>
          <Caption style={{ marginBottom: 10 }}>① 段落韵律纹样 · PARAGRAPH RHYTHM</Caption>
          <div style={{ height: 220, background: D.card, border: `1px solid ${D.rule}`, padding: 14, overflow: 'hidden' }}>
            {Array.from({ length: 28 }).map((_, i) => {
              const w = 30 + ((i * 37) % 65);
              const isAccent = i === 6 || i === 18;
              return (
                <div key={i} style={{
                  height: 4, marginBottom: 3, width: `${w}%`,
                  background: isAccent ? D.accent : D.ink,
                  opacity: isAccent ? 1 : 0.16,
                }}/>
              );
            })}
          </div>
          <Caption style={{ marginTop: 8 }}>② 同一图形语言 · 标志的延伸</Caption>
        </div>

        <div>
          <Caption style={{ marginBottom: 10 }}>③ 图标系统 · 16PX GRID, 1.5 STROKE</Caption>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 1, background: D.rule, border: `1px solid ${D.rule}` }}>
            {icons.map((ic) => (
              <div key={ic.id} style={{ aspectRatio: 1, background: D.card, display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', gap: 6, padding: 8 }}>
                <svg viewBox="0 0 16 16" width="22" height="22" fill="none" stroke={D.ink} strokeWidth="1.4" strokeLinecap="square" strokeLinejoin="miter">
                  <path d={ic.path}/>
                </svg>
                <div className="mono" style={{ fontSize: 8, color: D.faint, letterSpacing: 0.4 }}>{ic.id}</div>
              </div>
            ))}
          </div>
          <Caption style={{ marginTop: 10 }}>④ DIFF 卡片示意</Caption>
          <div style={{ marginTop: 8, padding: 10, background: D.card, border: `1px solid ${D.rule}` }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
              <LogoD variant="compact" size={16}/>
              <div className="mono" style={{ fontSize: 10, color: D.ink }}>spec.md · 1 pending diff</div>
              <div className="mono" style={{ fontSize: 10, color: D.accent, marginLeft: 'auto' }}>¶3</div>
            </div>
            <DiffLine kind="-">本地优先</DiffLine>
            <DiffLine kind="+">本地优先 · 离线可用</DiffLine>
          </div>
        </div>
      </div>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────
// D8 · Print (using D-01 + dynamic D-06)
// ─────────────────────────────────────────────────────────────
function D8_Print() {
  return (
    <div style={{ width: '100%', height: '100%', background: D.fog, color: D.ink }}>
      <MetaD id="VI-D · 08" title="印刷应用 · Print Applications"/>
      <div style={{ padding: '28px 32px', display: 'grid', gridTemplateColumns: '1fr 1.05fr', gap: 24 }}>

        <div>
          <Caption style={{ marginBottom: 10 }}>① 名片 · 85 × 55 MM · 双面</Caption>
          <div style={{ background: D.paper, padding: 18, marginBottom: 10, boxShadow: '0 4px 14px rgba(0,0,0,0.08)', aspectRatio: '85/55', position: 'relative', display: 'flex', alignItems: 'flex-start', gap: 14 }}>
            <LogoD size={56}/>
            <div style={{ flex: 1, display: 'flex', flexDirection: 'column', justifyContent: 'space-between', alignSelf: 'stretch', paddingTop: 2 }}>
              <div>
                <div style={{ fontFamily: "'IBM Plex Sans', sans-serif", fontWeight: 500, fontSize: 16, letterSpacing: -0.4 }}>Lin Yiran</div>
                <div className="sc" style={{ fontSize: 11, color: D.muted, marginTop: 2 }}>林一然 · 核心维护者</div>
              </div>
              <div className="mono" style={{ fontSize: 9, color: D.faint, lineHeight: 1.5, letterSpacing: 0.3 }}>
                lin@binder.dev<br/>+86 188 0000 0000<br/>binder.dev
              </div>
            </div>
            <div className="mono" style={{ position: 'absolute', bottom: 10, right: 14, fontSize: 8, color: D.accent, letterSpacing: 1.2 }}>D-01</div>
          </div>
          <div style={{ background: D.ink, padding: 18, aspectRatio: '85/55', boxShadow: '0 4px 14px rgba(0,0,0,0.08)', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
            <LogoD variant="negative" size={88}/>
          </div>
        </div>

        <div>
          <Caption style={{ marginBottom: 10 }}>② 海报 · A2 · DYNAMIC D-06 · "今天改了第 4 段"</Caption>
          <div style={{ background: D.paper, aspectRatio: '420/594', boxShadow: '0 8px 20px rgba(0,0,0,0.10)', padding: '22px 22px', position: 'relative', overflow: 'hidden' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
              <LogoD variant="accented" size={48}/>
              <div className="mono" style={{ fontSize: 8, color: D.faint, letterSpacing: 1, textAlign: 'right' }}>
                VOL. 01<br/>SPRING 2026
              </div>
            </div>
            <div style={{ marginTop: 40, fontFamily: "'IBM Plex Sans', sans-serif", fontWeight: 500, fontSize: 40, letterSpacing: -1.6, lineHeight: 0.94, color: D.ink }}>
              Edit<br/>one<br/><span style={{ color: D.accent }}>paragraph</span><br/>at a time<span style={{ color: D.accent }}>.</span>
            </div>
            <div className="sc" style={{ position: 'absolute', bottom: 64, left: 22, right: 22, fontSize: 11, lineHeight: 1.6, color: D.muted }}>
              AI 修改先以差异呈现，<br/>由你逐条审阅。文档掌握在你手里。
            </div>
            <div style={{ position: 'absolute', bottom: 22, left: 22, right: 22, display: 'flex', justifyContent: 'space-between' }}>
              <Caption>BINDER.DEV / CORE</Caption>
              <Caption>OPEN SOURCE</Caption>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────
// D9 · Digital
// ─────────────────────────────────────────────────────────────
function D9_Digital() {
  return (
    <div style={{ width: '100%', height: '100%', background: D.fog, color: D.ink }}>
      <MetaD id="VI-D · 09" title="数字应用 · Digital Applications"/>
      <div style={{ padding: '24px 32px', display: 'grid', gridTemplateColumns: '0.7fr 1.4fr', gap: 22 }}>
        <div>
          <Caption style={{ marginBottom: 10 }}>① 应用图标</Caption>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 14 }}>
              <div style={{ width: 88, height: 88, background: D.paper, borderRadius: 20, display: 'flex', alignItems: 'center', justifyContent: 'center', boxShadow: '0 2px 8px rgba(0,0,0,0.12)' }}>
                <LogoD size={64}/>
              </div>
              <div className="mono" style={{ fontSize: 9, color: D.faint }}>88 · LIGHT</div>
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: 14 }}>
              <div style={{ width: 64, height: 64, background: D.ink, borderRadius: 14, display: 'flex', alignItems: 'center', justifyContent: 'center', boxShadow: '0 2px 8px rgba(0,0,0,0.18)' }}>
                <LogoD size={50} ink={D.paper}/>
              </div>
              <div className="mono" style={{ fontSize: 9, color: D.faint }}>64 · DARK</div>
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: 14 }}>
              <div style={{ width: 64, height: 64, background: D.accent, borderRadius: 14, display: 'flex', alignItems: 'center', justifyContent: 'center', boxShadow: '0 2px 8px rgba(0,0,0,0.18)' }}>
                <LogoD size={50} ink={D.paper}/>
              </div>
              <div className="mono" style={{ fontSize: 9, color: D.faint }}>64 · DEV/INSIDER</div>
            </div>
          </div>
        </div>

        <div>
          <Caption style={{ marginBottom: 10 }}>② 编辑器 + 聊天 · LIGHT</Caption>
          <AppWindow style={{ height: 300 }} title="binder · ~/docs/spec.md">
            <div style={{ display: 'flex', height: 'calc(100% - 33px)' }}>
              <div style={{ width: 110, borderRight: `1px solid ${D.rule}`, padding: 10, fontSize: 10 }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 10 }}>
                  <LogoD size={14}/>
                  <div className="mono" style={{ fontSize: 9, fontWeight: 600 }}>binder<span style={{ color: D.accent }}>.</span></div>
                </div>
                <div className="mono" style={{ color: D.faint, fontSize: 8, letterSpacing: 0.6, marginBottom: 8 }}>WORKSPACE</div>
                {[
                  { n: '▾ docs', dim: false },
                  { n: '  spec.md', active: true },
                  { n: '  intro.md' },
                  { n: '  notes.txt' },
                  { n: '▸ .binder', dim: true },
                ].map((f, i) => (
                  <div key={i} className="mono" style={{
                    padding: '3px 6px', fontSize: 10,
                    color: f.active ? D.ink : f.dim ? D.faint : D.muted,
                    fontWeight: f.active ? 600 : 400,
                    background: f.active ? 'rgba(194,74,42,0.06)' : 'transparent',
                    borderLeft: f.active ? `2px solid ${D.accent}` : '2px solid transparent',
                  }}>{f.n}</div>
                ))}
              </div>
              <div style={{ flex: 1, padding: '14px 18px', borderRight: `1px solid ${D.rule}`, fontSize: 11 }}>
                <div style={{ fontFamily: "'IBM Plex Sans', sans-serif", fontWeight: 600, fontSize: 16, marginBottom: 10 }}>spec.md</div>
                <div style={{ color: D.muted, lineHeight: 1.65 }} className="sc">
                  Binder Core 是一个本地优先的 AI 文档编辑器。
                </div>
                <div style={{ marginTop: 10 }}>
                  <DiffLine kind="-">所有 diff 自动接受。</DiffLine>
                  <DiffLine kind="+">所有 diff 默认 pending，由用户审阅。</DiffLine>
                </div>
                <div style={{ marginTop: 10, color: D.muted, lineHeight: 1.65 }} className="sc">
                  workspace.db 持久化全部状态。
                </div>
              </div>
              <div style={{ width: 130, padding: 10, fontSize: 10, background: '#fbfaf6' }}>
                <div className="mono" style={{ color: D.faint, fontSize: 8, letterSpacing: 0.6, marginBottom: 8 }}>CHAT · AGENT</div>
                <div style={{ marginBottom: 8, padding: 6, background: D.card, borderRadius: 2, border: `1px solid ${D.rule}` }}>
                  <div className="mono" style={{ fontSize: 8, color: D.faint }}>YOU</div>
                  <div className="sc" style={{ marginTop: 2 }}>把"自动接受"改成审阅</div>
                </div>
                <div style={{ padding: 6, background: D.card, borderRadius: 2, border: `1px solid ${D.accent}` }}>
                  <div className="mono" style={{ fontSize: 8, color: D.accent }}>BINDER · 1 DIFF</div>
                  <div className="mono" style={{ fontSize: 8, color: D.muted, marginTop: 2 }}>spec.md · ¶3</div>
                  <div style={{ display: 'flex', gap: 4, marginTop: 6 }}>
                    <div className="mono" style={{ flex: 1, padding: '3px 0', background: D.green, color: '#fff', textAlign: 'center', fontSize: 8 }}>ACCEPT</div>
                    <div className="mono" style={{ flex: 1, padding: '3px 0', color: D.muted, textAlign: 'center', fontSize: 8, border: `1px solid ${D.rule}` }}>REJECT</div>
                  </div>
                </div>
              </div>
            </div>
          </AppWindow>
        </div>
      </div>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────
// D10 · Merch
// ─────────────────────────────────────────────────────────────
function D10_Merch() {
  return (
    <div style={{ width: '100%', height: '100%', background: D.fog, color: D.ink }}>
      <MetaD id="VI-D · 10" title="周边 · Merch & VIP"/>
      <div style={{ padding: '28px 32px', display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 18 }}>

        <div>
          <Caption style={{ marginBottom: 10 }}>① 帆布袋</Caption>
          <div style={{ aspectRatio: '4/5', background: D.paper, position: 'relative', boxShadow: '0 4px 14px rgba(0,0,0,0.08)', overflow: 'hidden' }}>
            <div style={{ position: 'absolute', top: 0, left: '14%', right: '14%', height: 18, borderLeft: `8px solid ${D.ink}`, borderRight: `8px solid ${D.ink}` }}/>
            <div style={{ position: 'absolute', inset: '35% 25% auto 25%' }}>
              <LogoD variant="default" size={120}/>
            </div>
            <div style={{ position: 'absolute', bottom: 18, left: 0, right: 0, textAlign: 'center' }}>
              <div className="mono" style={{ fontSize: 8, color: D.muted, letterSpacing: 2 }}>EDIT ONE PARAGRAPH AT A TIME</div>
            </div>
          </div>
        </div>

        <div>
          <Caption style={{ marginBottom: 10 }}>② 贡献者卡 · CONTRIBUTOR PASS</Caption>
          <div style={{ aspectRatio: '85/120', background: D.ink, color: D.paper, padding: 16, boxShadow: '0 6px 18px rgba(0,0,0,0.20)', display: 'flex', flexDirection: 'column', justifyContent: 'space-between' }}>
            <div>
              <LogoD variant="negative" size={56}/>
              <div className="mono" style={{ fontSize: 8, color: 'rgba(255,255,255,0.5)', marginTop: 14, letterSpacing: 0.8 }}>CORE · CONTRIBUTOR / 2026</div>
            </div>
            <PhotoSlot style={{ aspectRatio: 1, marginTop: 4 }} label="PORTRAIT"/>
            <div>
              <div style={{ fontFamily: "'IBM Plex Sans', sans-serif", fontWeight: 500, fontSize: 17, letterSpacing: -0.4 }}>Lin Yiran</div>
              <div className="sc" style={{ fontSize: 10, color: 'rgba(255,255,255,0.5)', marginTop: 2 }}>林一然 · 核心维护者</div>
            </div>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-end' }}>
              <div className="mono" style={{ fontSize: 8, color: D.accent, letterSpacing: 1.2 }}>NO. 00017</div>
              <div className="mono" style={{ fontSize: 8, color: 'rgba(255,255,255,0.4)' }}>BINDER.DEV</div>
            </div>
          </div>
        </div>

        <div>
          <Caption style={{ marginBottom: 10 }}>③ 贴纸 · STICKER SET</Caption>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
            <div style={{ aspectRatio: 1, background: D.paper, border: `1px solid ${D.rule}`, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
              <LogoD size={56}/>
            </div>
            <div style={{ aspectRatio: 1, background: D.ink, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
              <LogoD variant="negative" size={64}/>
            </div>
            <div style={{ aspectRatio: 1, background: D.accent, color: D.paper, padding: 10, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
              <LogoD variant="accented" size={56} ink={D.paper} accent={D.paper}/>
            </div>
            <div style={{ aspectRatio: 1, background: D.paper, border: `1px solid ${D.rule}`, padding: 10 }}>
              <DiffLine kind="-">old</DiffLine>
              <DiffLine kind="+">new</DiffLine>
              <DiffLine kind=" ">¶</DiffLine>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

Object.assign(window, {
  D_palette: D, LogoD, WordmarkD, LOGO_VARIANTS,
  D1_Hero, D2_Variants, D3_Dynamic, D4_System, D5_Color, D6_Type, D7_Aux, D8_Print, D9_Digital, D10_Merch,
});
