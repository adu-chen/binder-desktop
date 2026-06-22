/* direction-c.jsx — Direction C · Diff +/− · 双向流 · developer-tool 美学 */

const C = {
  ink:    '#18171A',
  paper:  '#F6F4F0',
  fog:    '#E9E6DF',
  rule:   'rgba(24,23,26,0.10)',
  muted:  'rgba(24,23,26,0.5)',
  faint:  'rgba(24,23,26,0.3)',
  accept: '#3D7F58',   // diff +
  reject: '#B0533B',   // diff −
  pulse:  '#7A6D3E',   // single brand accent — muted olive
};

// ─── Logo mark — square ± monogram
// Filled black square split diagonally; + in upper-right, − in lower-left.
function LogoC({ size = 120, color = C.ink, paper = C.paper, inverse = false }) {
  const fg = inverse ? paper : color;
  const bg = inverse ? color : paper;
  return (
    <svg viewBox="0 0 100 100" width={size} height={size} style={{ display: 'block' }}>
      <rect width="100" height="100" fill={fg} rx="0"/>
      {/* diagonal divider */}
      <line x1="14" y1="86" x2="86" y2="14" stroke={bg} strokeWidth="3" strokeLinecap="square"/>
      {/* + plus (top-right) */}
      <g stroke={bg} strokeWidth="6" strokeLinecap="square">
        <line x1="58" y1="32" x2="80" y2="32"/>
        <line x1="69" y1="21" x2="69" y2="43"/>
      </g>
      {/* − minus (bottom-left) */}
      <g stroke={bg} strokeWidth="6" strokeLinecap="square">
        <line x1="20" y1="68" x2="42" y2="68"/>
      </g>
    </svg>
  );
}

// Wordmark — uppercase mono
function WordmarkC({ size = 56, color = C.ink }) {
  return (
    <div className="mono" style={{
      fontSize: size, fontWeight: 600, letterSpacing: size * 0.06,
      color, lineHeight: 1,
    }}>BINDER</div>
  );
}

function MetaC({ id, title, sub }) { return <MetaStrip id={id} title={title} sub={sub} accent={C.pulse}/>; }

// ── C1 · Hero
function C1_Hero() {
  return (
    <div style={{ width: '100%', height: '100%', background: C.paper, color: C.ink, position: 'relative' }}>
      <MetaC id="VI-C · 01" title="主标 · Primary Lockup" sub="方向 C — Diff"/>
      <div style={{ position: 'absolute', inset: '60px 56px', display: 'flex', flexDirection: 'column', justifyContent: 'space-between' }}>
        <div className="mono" style={{ fontSize: 10, letterSpacing: 1.2, color: C.faint, textTransform: 'uppercase' }}>
          ③ +/− 符号融合的方形标 — 致敬 Diff 审阅机制
        </div>

        <div style={{ display: 'flex', alignItems: 'center', gap: 32 }}>
          <LogoC size={128}/>
          <div>
            <div className="mono" style={{ fontSize: 64, fontWeight: 600, letterSpacing: 4, lineHeight: 1, color: C.ink }}>
              BINDER
            </div>
            <div style={{ marginTop: 12 }}>
              <DiffLine kind="-">general AI assistant</DiffLine>
              <DiffLine kind="+">a local-first AI editor — diff as the contract.</DiffLine>
            </div>
            <div className="sc" style={{ fontSize: 12, color: C.muted, marginTop: 14 }}>
              本地优先 · AI 文档编辑器最小核心
            </div>
          </div>
        </div>

        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-end' }}>
          <Caption>① ± SQUARE MONOGRAM</Caption>
          <Caption>② WORDMARK · JETBRAINS MONO 600</Caption>
          <Caption>19 / 24</Caption>
        </div>
      </div>
    </div>
  );
}

// ── C2 · Construction
function C2_System() {
  return (
    <div style={{ width: '100%', height: '100%', background: C.paper, color: C.ink }}>
      <MetaC id="VI-C · 02" title="结构 · Construction & Variants"/>
      <div style={{ padding: '28px 32px', display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 24 }}>

        <div>
          <Caption style={{ marginBottom: 10 }}>① 构造 · 10 × 10 UNITS · DIAG SPLIT</Caption>
          <div style={{ height: 200, position: 'relative', background: '#fff', border: `1px solid ${C.rule}` }} className="gridbg">
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%' }}>
              <LogoC size={140}/>
            </div>
            <div className="mono" style={{ position: 'absolute', top: 4, left: 6, fontSize: 8, color: C.faint }}>0,0</div>
            <div className="mono" style={{ position: 'absolute', bottom: 4, right: 6, fontSize: 8, color: C.faint }}>10,10</div>
          </div>
          <Caption style={{ marginTop: 8 }}>② 最小尺寸 · MIN 20PX</Caption>
        </div>

        <div>
          <Caption style={{ marginBottom: 10 }}>③ 安全距离 · = ½ MARK</Caption>
          <div style={{ position: 'relative', height: 200, background: '#fff', border: `1px solid ${C.rule}`, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
            <div style={{ position: 'absolute', inset: 32, border: `1px dashed ${C.faint}` }}/>
            <LogoC size={110}/>
          </div>
          <div style={{ display: 'flex', gap: 8, marginTop: 12 }}>
            <div style={{ flex: 1, padding: 12, background: '#fff', border: `1px solid ${C.rule}`, display: 'flex', justifyContent: 'center' }}>
              <LogoC size={40}/>
            </div>
            <div style={{ flex: 1, padding: 12, background: C.ink, display: 'flex', justifyContent: 'center' }}>
              <LogoC size={40} inverse/>
            </div>
            <div style={{ flex: 1, padding: 12, background: C.pulse, display: 'flex', justifyContent: 'center' }}>
              <LogoC size={40} color={C.paper}/>
            </div>
          </div>
        </div>

        <div style={{ gridColumn: 'span 2' }}>
          <Caption style={{ marginBottom: 10 }}>④ 组合 · LOCKUP OPTIONS</Caption>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3,1fr)', gap: 12 }}>
            <div style={{ padding: '14px 12px', background: '#fff', border: `1px solid ${C.rule}`, display: 'flex', alignItems: 'center', gap: 10, justifyContent: 'center' }}>
              <LogoC size={36}/>
              <div className="mono" style={{ fontWeight: 600, fontSize: 16, letterSpacing: 1.5 }}>BINDER</div>
            </div>
            <div style={{ padding: '14px 12px', background: '#fff', border: `1px solid ${C.rule}`, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 6 }}>
              <LogoC size={42}/>
              <div className="mono" style={{ fontWeight: 600, fontSize: 12, letterSpacing: 1.4 }}>BINDER</div>
              <div className="mono" style={{ fontSize: 8, color: C.pulse, letterSpacing: 1.4 }}>v1.0</div>
            </div>
            <div style={{ padding: '14px 12px', background: '#fff', border: `1px solid ${C.rule}`, display: 'flex', alignItems: 'center', gap: 10, justifyContent: 'center' }}>
              <div className="mono" style={{ fontWeight: 600, fontSize: 16, letterSpacing: 1.5 }}>BINDER</div>
              <div className="mono" style={{ fontSize: 10, color: C.pulse, letterSpacing: 1 }}>./core</div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

// ── C3 · Color
function C3_Color() {
  return (
    <div style={{ width: '100%', height: '100%', background: C.paper, color: C.ink }}>
      <MetaC id="VI-C · 03" title="色彩 · Color System" sub="MONOCHROME + PULSE OLIVE"/>
      <div style={{ padding: '28px 32px' }}>
        <Caption style={{ marginBottom: 12 }}>① 主色 · CORE</Caption>
        <div style={{ display: 'grid', gridTemplateColumns: '2fr 2fr 1.2fr', gap: 14, marginBottom: 22 }}>
          <Swatch color={C.ink}   name="终端墨"   code="C-INK"   hex="#18171A" large/>
          <Swatch color={C.paper} name="缓冲纸"   code="C-PAPER" hex="#F6F4F0" large/>
          <Swatch color={C.pulse} name="脉冲橄榄" code="C-PULSE" hex="#7A6D3E" large/>
        </div>
        <Caption style={{ marginBottom: 10 }}>② 状态色 · STATE — 仅承担 DIFF 语义</Caption>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12, marginBottom: 18 }}>
          <div style={{ background: '#fff', border: `1px solid ${C.rule}`, padding: 12 }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 6 }}>
              <div style={{ width: 32, height: 32, background: C.accept }}/>
              <div>
                <div className="mono" style={{ fontSize: 10, color: C.accept, letterSpacing: 0.5 }}>+ ACCEPT</div>
                <div className="mono" style={{ fontSize: 10, color: C.muted }}>#3D7F58</div>
              </div>
            </div>
            <DiffLine kind="+">本地优先</DiffLine>
          </div>
          <div style={{ background: '#fff', border: `1px solid ${C.rule}`, padding: 12 }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 6 }}>
              <div style={{ width: 32, height: 32, background: C.reject }}/>
              <div>
                <div className="mono" style={{ fontSize: 10, color: C.reject, letterSpacing: 0.5 }}>− REJECT</div>
                <div className="mono" style={{ fontSize: 10, color: C.muted }}>#B0533B</div>
              </div>
            </div>
            <DiffLine kind="-">云端编辑</DiffLine>
          </div>
        </div>
        <Caption style={{ marginBottom: 10 }}>③ 中性色阶</Caption>
        <div style={{ display: 'flex', height: 44 }}>
          {[1,0.9,0.78,0.64,0.48,0.32,0.2,0.12,0.06,0].map((v, i) => {
            const c = i === 9 ? C.paper : `rgba(24,23,26,${v})`;
            return <div key={i} style={{ flex: 1, background: c }}/>;
          })}
        </div>
      </div>
    </div>
  );
}

// ── C4 · Type
function C4_Type() {
  return (
    <div style={{ width: '100%', height: '100%', background: C.paper, color: C.ink }}>
      <MetaC id="VI-C · 04" title="字体 · Type System" sub="JETBRAINS MONO · INTER · NOTO"/>
      <div style={{ padding: '28px 32px', display: 'grid', gridTemplateColumns: '1fr 1.1fr', gap: 28 }}>
        <div>
          <div className="mono" style={{ fontSize: 10, color: C.faint, letterSpacing: 0.8 }}>DISPLAY · 主标</div>
          <div className="mono" style={{ fontSize: 56, fontWeight: 600, letterSpacing: 3, lineHeight: 1, marginTop: 6 }}>BINDER</div>
          <div className="mono" style={{ fontSize: 9, color: C.faint, marginTop: 6 }}>JetBrains Mono 600 · letter-spacing 0.06em</div>

          <div className="mono" style={{ fontSize: 10, color: C.faint, letterSpacing: 0.8, marginTop: 24 }}>SUBHEAD · UI</div>
          <div className="mono" style={{ fontSize: 22, fontWeight: 500, lineHeight: 1.1, marginTop: 6 }}>$ binder ./docs</div>
          <div className="mono" style={{ fontSize: 9, color: C.faint, marginTop: 6 }}>JetBrains Mono 500</div>

          <div className="mono" style={{ fontSize: 10, color: C.faint, letterSpacing: 0.8, marginTop: 24 }}>BODY · 正文</div>
          <div className="sc" style={{ fontSize: 14, lineHeight: 1.65, marginTop: 6 }}>
            <span style={{ background: 'rgba(61,127,88,0.16)', color: C.accept }}>差异</span> 即 <span style={{ background: 'rgba(176,83,59,0.16)', color: C.reject }}>契约</span>。
          </div>
          <div className="mono" style={{ fontSize: 9, color: C.faint, marginTop: 6 }}>Noto Sans SC · 14/24</div>
        </div>

        <div style={{ borderLeft: `1px solid ${C.rule}`, paddingLeft: 24 }}>
          <Caption style={{ marginBottom: 10 }}>① 字号阶梯 · 全 MONO</Caption>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 6, marginBottom: 18 }}>
            {[
              [56, 'Display'],
              [32, 'H1'],
              [22, 'H2'],
              [16, 'Body'],
              [12, 'UI'],
              [10, 'Caption'],
            ].map(([sz, name]) => (
              <div key={name} style={{ display: 'flex', alignItems: 'baseline', gap: 14 }}>
                <div className="mono" style={{ width: 28, fontSize: 9, color: C.faint }}>{sz}</div>
                <div className="mono" style={{ fontSize: sz, fontWeight: 600, lineHeight: 1, letterSpacing: 1 }}>Bb</div>
                <div className="mono" style={{ fontSize: 9, color: C.faint, marginLeft: 'auto' }}>{name}</div>
              </div>
            ))}
          </div>
          <Caption style={{ marginBottom: 6 }}>② TERMINAL VOICE</Caption>
          <div style={{ background: C.ink, color: C.paper, padding: 14, fontFamily: 'JetBrains Mono, monospace', fontSize: 11, lineHeight: 1.6 }}>
            <div style={{ color: C.pulse }}>$ binder init ./workspace</div>
            <div style={{ opacity: 0.7 }}>→ created .binder/workspace.db</div>
            <div style={{ color: C.pulse, marginTop: 4 }}>$ binder chat</div>
            <div style={{ opacity: 0.7 }}>→ 1 pending diff · spec.md</div>
          </div>
        </div>
      </div>
    </div>
  );
}

// ── C5 · Aux + Icon
function C5_Aux() {
  return (
    <div style={{ width: '100%', height: '100%', background: C.paper, color: C.ink }}>
      <MetaC id="VI-C · 05" title="辅助图形 · Auxiliary & Iconography"/>
      <div style={{ padding: '28px 32px', display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 24 }}>

        <div>
          <Caption style={{ marginBottom: 10 }}>① 辅助图形 · DIFF GUTTER PATTERN</Caption>
          <div style={{ height: 200, background: '#fff', border: `1px solid ${C.rule}`, padding: 0, overflow: 'hidden', display: 'grid', gridTemplateColumns: '24px 1fr' }}>
            <div style={{ background: '#fbfaf6', borderRight: `1px solid ${C.rule}` }}>
              {Array.from({ length: 14 }).map((_, i) => (
                <div key={i} className="mono" style={{ height: 14, fontSize: 8, color: C.faint, textAlign: 'right', paddingRight: 4, lineHeight: 14 }}>{i + 1}</div>
              ))}
            </div>
            <div className="mono" style={{ fontSize: 10, lineHeight: '14px' }}>
              {[
                { k: ' ', t: 'function binder() {' },
                { k: ' ', t: '  // local first' },
                { k: '-', t: '  return cloud.fetch()' },
                { k: '+', t: '  return local.read()' },
                { k: ' ', t: '}' },
                { k: ' ', t: '' },
                { k: ' ', t: 'const diff = ai.suggest()' },
                { k: '-', t: 'diff.apply()' },
                { k: '+', t: 'diff.review()' },
                { k: ' ', t: '' },
                { k: ' ', t: 'workspace.commit()' },
                { k: ' ', t: '' },
                { k: ' ', t: 'export default binder' },
                { k: ' ', t: '' },
              ].map((row, i) => (
                <div key={i} style={{
                  display: 'flex', height: 14,
                  background: row.k === '+' ? 'rgba(61,127,88,0.10)' : row.k === '-' ? 'rgba(176,83,59,0.10)' : 'transparent',
                }}>
                  <div style={{ width: 14, color: row.k === '+' ? C.accept : row.k === '-' ? C.reject : C.faint, textAlign: 'center' }}>{row.k}</div>
                  <div style={{ color: C.ink, opacity: row.k === ' ' ? 0.5 : 0.85 }}>{row.t}</div>
                </div>
              ))}
            </div>
          </div>
          <Caption style={{ marginTop: 8 }}>② 同一审阅栏可作为品牌纹样使用</Caption>
        </div>

        <div>
          <Caption style={{ marginBottom: 10 }}>③ 图标系统 · 14PX TERMINAL GRID</Caption>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4,1fr)', gap: 1, background: C.rule, border: `1px solid ${C.rule}` }}>
            {[
              ['+ accept',   <text x="8" y="11" textAnchor="middle" fill={C.ink} fontFamily="JetBrains Mono" fontSize="12" fontWeight="700">+</text>],
              ['- reject',   <text x="8" y="11" textAnchor="middle" fill={C.ink} fontFamily="JetBrains Mono" fontSize="12" fontWeight="700">−</text>],
              ['± diff',     <text x="8" y="11" textAnchor="middle" fill={C.ink} fontFamily="JetBrains Mono" fontSize="12" fontWeight="700">±</text>],
              ['/ split',    <text x="8" y="12" textAnchor="middle" fill={C.ink} fontFamily="JetBrains Mono" fontSize="13" fontWeight="700">/</text>],
              ['→ flow',     <text x="8" y="12" textAnchor="middle" fill={C.ink} fontFamily="JetBrains Mono" fontSize="13" fontWeight="700">→</text>],
              ['$ shell',    <text x="8" y="12" textAnchor="middle" fill={C.ink} fontFamily="JetBrains Mono" fontSize="13" fontWeight="700">$</text>],
              ['¶ block',    <text x="8" y="12" textAnchor="middle" fill={C.ink} fontFamily="JetBrains Mono" fontSize="13" fontWeight="700">¶</text>],
              ['# tag',      <text x="8" y="12" textAnchor="middle" fill={C.ink} fontFamily="JetBrains Mono" fontSize="13" fontWeight="700">#</text>],
            ].map(([name, glyph], i) => (
              <div key={i} style={{ aspectRatio: 1, background: '#fff', display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', gap: 4, padding: 6 }}>
                <svg viewBox="0 0 16 16" width="22" height="22">{glyph}</svg>
                <div className="mono" style={{ fontSize: 8, color: C.faint, letterSpacing: 0.4 }}>{name}</div>
              </div>
            ))}
          </div>
          <Caption style={{ marginTop: 10 }}>④ 命令面板示意</Caption>
          <div className="mono" style={{ marginTop: 8, padding: 10, background: C.ink, color: C.paper, fontSize: 10, lineHeight: 1.6 }}>
            <div style={{ color: C.pulse }}>&gt; accept all</div>
            <div style={{ opacity: 0.7 }}>  ↳ 3 diffs · spec.md (2), intro.md (1)</div>
            <div style={{ color: C.accept, marginTop: 4 }}>✓ committed.</div>
          </div>
        </div>
      </div>
    </div>
  );
}

// ── C6 · Print
function C6_Print() {
  return (
    <div style={{ width: '100%', height: '100%', background: C.fog, color: C.ink }}>
      <MetaC id="VI-C · 06" title="印刷应用 · Print Applications"/>
      <div style={{ padding: '28px 32px', display: 'grid', gridTemplateColumns: '1fr 1.05fr', gap: 24 }}>
        <div>
          <Caption style={{ marginBottom: 10 }}>① 名片 · 85 × 55 MM · 双面</Caption>
          <div style={{ background: C.ink, color: C.paper, padding: 16, marginBottom: 10, boxShadow: '0 4px 14px rgba(0,0,0,0.18)', aspectRatio: '85/55', position: 'relative', display: 'flex', flexDirection: 'column', justifyContent: 'space-between' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
              <LogoC size={34} inverse/>
              <div className="mono" style={{ fontSize: 14, fontWeight: 600, letterSpacing: 1.5 }}>BINDER</div>
            </div>
            <div>
              <div className="mono" style={{ fontSize: 12, fontWeight: 500 }}>he.shu &lt;he@binder.dev&gt;</div>
              <div className="mono" style={{ fontSize: 9, color: 'rgba(255,255,255,0.5)', marginTop: 4, letterSpacing: 0.4 }}>
                ENGINEER · CORE / OPEN SOURCE
              </div>
            </div>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-end' }}>
              <div className="mono" style={{ fontSize: 9, color: C.pulse }}>./he-shu</div>
              <div className="mono" style={{ fontSize: 9, color: 'rgba(255,255,255,0.4)' }}>v1.0</div>
            </div>
          </div>
          <div style={{ background: C.paper, padding: 16, aspectRatio: '85/55', boxShadow: '0 4px 14px rgba(0,0,0,0.10)', position: 'relative', overflow: 'hidden' }}>
            <div style={{ position: 'absolute', inset: 14 }}>
              <DiffLine kind="-">general-purpose AI</DiffLine>
              <DiffLine kind="+">a local-first AI editor.</DiffLine>
              <DiffLine kind=" ">diff as the contract.</DiffLine>
            </div>
            <div className="mono" style={{ position: 'absolute', bottom: 10, right: 14, fontSize: 9, color: C.muted }}>binder.dev</div>
          </div>
        </div>

        <div>
          <Caption style={{ marginBottom: 10 }}>② 海报 · A2 · TERMINAL POSTER</Caption>
          <div style={{ background: C.ink, color: C.paper, aspectRatio: '420/594', boxShadow: '0 8px 20px rgba(0,0,0,0.20)', padding: '22px 22px', position: 'relative', overflow: 'hidden', fontFamily: 'JetBrains Mono, monospace' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between' }}>
              <LogoC size={36} inverse/>
              <div className="mono" style={{ fontSize: 8, color: 'rgba(255,255,255,0.4)', textAlign: 'right', letterSpacing: 1 }}>
                BINDER / 2026<br/>OPEN SOURCE
              </div>
            </div>
            <div style={{ marginTop: 24, fontSize: 36, fontWeight: 600, lineHeight: 1, letterSpacing: 1 }}>
              <div style={{ color: C.reject }}>− CLOUD</div>
              <div style={{ color: 'rgba(255,255,255,0.4)' }}>· DEFAULTS</div>
              <div style={{ color: C.accept }}>+ LOCAL</div>
            </div>
            <div style={{ position: 'absolute', bottom: 50, left: 22, right: 22, fontSize: 10, lineHeight: 1.7, color: 'rgba(255,255,255,0.7)' }}>
              $ binder ./your-docs<br/>
              <span style={{ color: C.pulse }}>→ workspace mounted · 0 cloud calls.</span>
            </div>
            <div style={{ position: 'absolute', bottom: 18, left: 22, right: 22, display: 'flex', justifyContent: 'space-between' }}>
              <Caption style={{ color: 'rgba(255,255,255,0.5)' }}>BINDER.DEV</Caption>
              <Caption style={{ color: 'rgba(255,255,255,0.5)' }}>v1.0 / CORE</Caption>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

// ── C7 · Digital
function C7_Digital() {
  return (
    <div style={{ width: '100%', height: '100%', background: C.fog, color: C.ink }}>
      <MetaC id="VI-C · 07" title="数字应用 · Digital Applications"/>
      <div style={{ padding: '28px 32px', display: 'grid', gridTemplateColumns: '0.7fr 1.4fr', gap: 24 }}>
        <div>
          <Caption style={{ marginBottom: 10 }}>① 应用图标</Caption>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 14 }}>
              <div style={{ width: 88, height: 88, background: C.ink, borderRadius: 20, display: 'flex', alignItems: 'center', justifyContent: 'center', boxShadow: '0 2px 8px rgba(0,0,0,0.18)' }}>
                <LogoC size={64} inverse/>
              </div>
              <div className="mono" style={{ fontSize: 9, color: C.faint }}>88 · MAIN</div>
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: 14 }}>
              <div style={{ width: 56, height: 56, background: C.paper, borderRadius: 12, display: 'flex', alignItems: 'center', justifyContent: 'center', boxShadow: '0 2px 8px rgba(0,0,0,0.10)' }}>
                <LogoC size={42}/>
              </div>
              <div className="mono" style={{ fontSize: 9, color: C.faint }}>56 · LIGHT</div>
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: 14 }}>
              <div style={{ width: 56, height: 56, background: C.pulse, borderRadius: 12, display: 'flex', alignItems: 'center', justifyContent: 'center', boxShadow: '0 2px 8px rgba(0,0,0,0.10)' }}>
                <LogoC size={42} color={C.paper}/>
              </div>
              <div className="mono" style={{ fontSize: 9, color: C.faint }}>56 · DEV BUILD</div>
            </div>
          </div>
        </div>

        <div>
          <Caption style={{ marginBottom: 10 }}>② 终端 / CLI 入口</Caption>
          <AppWindow style={{ height: 290 }} dark title="binder · ~/docs">
            <div style={{ padding: 16, fontFamily: 'JetBrains Mono, monospace', fontSize: 11, lineHeight: 1.65, color: 'rgba(255,255,255,0.85)' }}>
              <div><span style={{ color: C.pulse }}>he@local</span><span style={{ color: 'rgba(255,255,255,0.5)' }}> ~/docs</span> $ <span>binder chat</span></div>
              <div style={{ color: 'rgba(255,255,255,0.5)', margin: '6px 0' }}>→ workspace: ~/docs · 3 files</div>
              <div style={{ color: 'rgba(255,255,255,0.5)' }}>→ provider: anthropic · agent mode</div>
              <div style={{ marginTop: 12 }}><span style={{ color: C.pulse }}>&gt;</span> 把 spec.md 的产品定位改成中文一句话</div>
              <div style={{ marginTop: 8, color: 'rgba(255,255,255,0.6)' }}>thinking…</div>
              <div style={{ marginTop: 8, color: 'rgba(255,255,255,0.85)' }}>1 pending diff · spec.md ¶1</div>
              <div style={{ marginTop: 8, padding: '4px 0' }}>
                <div style={{ display: 'flex' }}><div style={{ width: 14, color: C.reject }}>−</div><div style={{ color: 'rgba(255,255,255,0.7)' }}>Binder Core is an experiment in local-first editing.</div></div>
                <div style={{ display: 'flex' }}><div style={{ width: 14, color: C.accept }}>+</div><div>本地优先的 AI 文档编辑器最小核心。</div></div>
              </div>
              <div style={{ marginTop: 8, color: C.pulse }}>[a]ccept · [r]eject · [d]iff: <span style={{ color: '#fff' }}>_</span></div>
            </div>
          </AppWindow>
        </div>
      </div>
    </div>
  );
}

// ── C8 · Merch
function C8_Merch() {
  return (
    <div style={{ width: '100%', height: '100%', background: C.fog, color: C.ink }}>
      <MetaC id="VI-C · 08" title="周边 · Merch & VIP"/>
      <div style={{ padding: '28px 32px', display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 18 }}>

        <div>
          <Caption style={{ marginBottom: 10 }}>① T-SHIRT · DEV TEE</Caption>
          <div style={{ aspectRatio: '4/5', background: C.ink, position: 'relative', boxShadow: '0 4px 14px rgba(0,0,0,0.20)', overflow: 'hidden', display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', padding: 18 }}>
            <div className="mono" style={{ fontSize: 11, color: C.reject, lineHeight: 1.6, alignSelf: 'flex-start' }}>− vibes</div>
            <div className="mono" style={{ fontSize: 11, color: C.accept, lineHeight: 1.6, alignSelf: 'flex-start', marginBottom: 16 }}>+ diffs</div>
            <LogoC size={70} inverse/>
            <div className="mono" style={{ fontSize: 9, color: 'rgba(255,255,255,0.5)', marginTop: 10, letterSpacing: 1.5 }}>BINDER · CORE</div>
          </div>
        </div>

        <div>
          <Caption style={{ marginBottom: 10 }}>② COMMITTER PASS</Caption>
          <div style={{ aspectRatio: '85/120', background: C.paper, padding: 14, boxShadow: '0 6px 18px rgba(0,0,0,0.12)', display: 'flex', flexDirection: 'column', justifyContent: 'space-between', borderLeft: `4px solid ${C.pulse}` }}>
            <div>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <LogoC size={28}/>
                <div className="mono" style={{ fontSize: 11, fontWeight: 600, letterSpacing: 1.3 }}>BINDER</div>
              </div>
              <div className="mono" style={{ fontSize: 8, color: C.muted, marginTop: 14, letterSpacing: 0.6 }}>// COMMITTER · 2026</div>
            </div>
            <PhotoSlot style={{ aspectRatio: 1 }} label="PORTRAIT"/>
            <div>
              <div className="mono" style={{ fontSize: 14, fontWeight: 600 }}>he.shu</div>
              <div className="mono" style={{ fontSize: 9, color: C.muted, marginTop: 4 }}>signed-off-by: he@binder.dev</div>
              <div className="mono" style={{ fontSize: 9, color: C.pulse, marginTop: 6 }}>#0017 · core</div>
            </div>
          </div>
        </div>

        <div>
          <Caption style={{ marginBottom: 10 }}>③ STICKER SHEET</Caption>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
            <div style={{ aspectRatio: 1, background: C.ink, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
              <LogoC size={64} inverse/>
            </div>
            <div style={{ aspectRatio: 1, background: C.paper, border: `1px solid ${C.rule}`, padding: 10 }}>
              <div className="mono" style={{ fontSize: 11, color: C.reject, lineHeight: 1.6 }}>− cloud</div>
              <div className="mono" style={{ fontSize: 11, color: C.accept, lineHeight: 1.6 }}>+ local</div>
              <div className="mono" style={{ fontSize: 11, color: C.accept, lineHeight: 1.6 }}>+ first</div>
            </div>
            <div style={{ aspectRatio: 1, background: C.pulse, color: C.paper, display: 'flex', alignItems: 'center', justifyContent: 'center', flexDirection: 'column' }}>
              <div className="mono" style={{ fontSize: 24, fontWeight: 600 }}>±</div>
              <div className="mono" style={{ fontSize: 8, letterSpacing: 1.5 }}>DIFF NATIVE</div>
            </div>
            <div style={{ aspectRatio: 1, background: C.paper, border: `1px solid ${C.rule}`, padding: 10, fontFamily: 'JetBrains Mono', fontSize: 10, color: C.muted, lineHeight: 1.5 }}>
              $ binder<br/>
              <span style={{ color: C.pulse }}>./your</span><br/>
              <span style={{ color: C.pulse }}>./docs</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

Object.assign(window, { C_palette: C, LogoC, WordmarkC, C1_Hero, C2_System, C3_Color, C4_Type, C5_Aux, C6_Print, C7_Digital, C8_Merch });
