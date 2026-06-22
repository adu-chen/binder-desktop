/* direction-b.jsx — Direction B · Block ▢ · 段落块抽象几何 */

const B = {
  ink:   '#111418',
  paper: '#FAFAF7',
  fog:   '#EFEEEA',
  rule:  'rgba(17,20,24,0.10)',
  muted: 'rgba(17,20,24,0.5)',
  faint: 'rgba(17,20,24,0.3)',
  accent:'#1B3A6B',   // cool ink-blue — restrained
};

// ─── Logo mark — paragraph block stack (B-silhouette)
// 8 horizontal bars of different widths, stacked, forming a "B" rhythm.
// Bars: full / 60% / 80% / full / 70% / full / 50% / full
function LogoB({ size = 160, color = B.ink, paper = null }) {
  const bars = [100, 64, 82, 100, 72, 100, 56, 100];
  return (
    <svg viewBox="0 0 120 120" width={size} height={size} style={{ display: 'block' }}>
      {paper && <rect width="120" height="120" fill={paper}/>}
      {bars.map((w, i) => (
        <rect key={i} x="14" y={14 + i * 12.5} width={w * 0.92} height="8" fill={color}/>
      ))}
    </svg>
  );
}

// Compact mark — square form for icons
function LogoBSquare({ size = 80, color = B.ink, bg = null }) {
  return (
    <div style={{ width: size, height: size, background: bg ?? 'transparent', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
      <LogoB size={size * 0.78} color={color}/>
    </div>
  );
}

// Section header used inside artboards
function MetaB({ id, title, sub }) {
  return <MetaStrip id={id} title={title} sub={sub} accent={B.accent}/>;
}

// ── B1 · Hero
function B1_Hero() {
  return (
    <div style={{ width: '100%', height: '100%', background: B.paper, color: B.ink, position: 'relative' }}>
      <MetaB id="VI-B · 01" title="主标 · Primary Lockup" sub="方向 B — Block"/>
      <div style={{ position: 'absolute', inset: '60px 56px', display: 'flex', flexDirection: 'column', justifyContent: 'space-between' }}>
        <div className="mono" style={{ fontSize: 10, letterSpacing: 1.2, color: B.faint, textTransform: 'uppercase' }}>
          ② 由段落条堆叠而成的几何 B — 致敬 BlockId 文档段落系统
        </div>

        <div style={{ display: 'flex', alignItems: 'center', gap: 32 }}>
          <LogoB size={140}/>
          <div>
            <div className="serif" style={{ fontWeight: 400, fontSize: 80, letterSpacing: -2, lineHeight: 0.95, color: B.ink }}>
              Binder
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginTop: 8 }}>
              <div style={{ width: 28, height: 2, background: B.accent }}/>
              <div className="mono" style={{ fontSize: 11, color: B.accent, letterSpacing: 1.8, textTransform: 'uppercase' }}>CORE</div>
            </div>
            <div className="sc" style={{ fontSize: 13, color: B.muted, marginTop: 18, lineHeight: 1.6, maxWidth: 340 }}>
              文档由段落组成，<br/>编辑由差异驱动。
            </div>
          </div>
        </div>

        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-end' }}>
          <Caption>① BLOCK-STACK MARK · 8 BARS</Caption>
          <Caption>② DISPLAY · INSTRUMENT SERIF</Caption>
          <Caption>11 / 24</Caption>
        </div>
      </div>
    </div>
  );
}

// ── B2 · Construction & Variants
function B2_System() {
  return (
    <div style={{ width: '100%', height: '100%', background: B.paper, color: B.ink }}>
      <MetaB id="VI-B · 02" title="结构 · Construction & Variants"/>
      <div style={{ padding: '28px 32px', display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 24 }}>

        {/* construction */}
        <div>
          <Caption style={{ marginBottom: 10 }}>① 构造 · 8 BARS · 12.5 PT VERTICAL RHYTHM</Caption>
          <div style={{ position: 'relative', height: 200, background: '#fff', border: `1px solid ${B.rule}` }} className="gridbg">
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%' }}>
              <LogoB size={144}/>
            </div>
            {/* width annotations on right */}
            <div className="mono" style={{ position: 'absolute', top: 12, right: 8, fontSize: 8, lineHeight: 1.5, color: B.faint, textAlign: 'right' }}>
              100%<br/>64%<br/>82%<br/>100%<br/>72%<br/>100%<br/>56%<br/>100%
            </div>
          </div>
          <Caption style={{ marginTop: 8 }}>② 段落宽度即 B 字符曲线投影</Caption>
        </div>

        {/* clearspace + variants */}
        <div>
          <Caption style={{ marginBottom: 10 }}>③ 安全距离 = 单条段落高度</Caption>
          <div style={{ position: 'relative', height: 200, background: '#fff', border: `1px solid ${B.rule}`, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
            <div style={{ position: 'absolute', inset: 28, border: `1px dashed ${B.faint}` }}/>
            <LogoB size={120}/>
          </div>
          <div style={{ display: 'flex', gap: 8, marginTop: 12 }}>
            <div style={{ flex: 1, padding: 12, background: '#fff', border: `1px solid ${B.rule}`, display: 'flex', justifyContent: 'center' }}>
              <LogoB size={40}/>
            </div>
            <div style={{ flex: 1, padding: 12, background: B.ink, display: 'flex', justifyContent: 'center' }}>
              <LogoB size={40} color={B.paper}/>
            </div>
            <div style={{ flex: 1, padding: 12, background: B.accent, display: 'flex', justifyContent: 'center' }}>
              <LogoB size={40} color={B.paper}/>
            </div>
          </div>
        </div>

        {/* lockup options */}
        <div style={{ gridColumn: 'span 2' }}>
          <Caption style={{ marginBottom: 10 }}>④ 组合 · LOCKUP OPTIONS</Caption>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 12 }}>
            <div style={{ padding: '14px 12px', background: '#fff', border: `1px solid ${B.rule}`, display: 'flex', alignItems: 'center', gap: 10, justifyContent: 'center' }}>
              <LogoB size={32}/>
              <div className="serif" style={{ fontSize: 22, lineHeight: 1 }}>Binder</div>
            </div>
            <div style={{ padding: '14px 12px', background: '#fff', border: `1px solid ${B.rule}`, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 4 }}>
              <LogoB size={36}/>
              <div className="serif" style={{ fontSize: 16, lineHeight: 1 }}>Binder</div>
              <div className="mono" style={{ fontSize: 7, letterSpacing: 1.5, color: B.muted }}>· CORE ·</div>
            </div>
            <div style={{ padding: '14px 12px', background: '#fff', border: `1px solid ${B.rule}`, display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 4 }}>
              <div className="serif" style={{ fontSize: 22, lineHeight: 1 }}>Binder</div>
              <div className="mono" style={{ fontSize: 8, color: B.accent, letterSpacing: 1.2, alignSelf: 'flex-start', marginTop: 2 }}>®</div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

// ── B3 · Color
function B3_Color() {
  return (
    <div style={{ width: '100%', height: '100%', background: B.paper, color: B.ink }}>
      <MetaB id="VI-B · 03" title="色彩 · Color System" sub="MONOCHROME + INK BLUE"/>
      <div style={{ padding: '28px 32px' }}>
        <Caption style={{ marginBottom: 12 }}>① 主色 · CORE</Caption>
        <div style={{ display: 'grid', gridTemplateColumns: '2fr 2fr 1.2fr', gap: 14, marginBottom: 24 }}>
          <Swatch color={B.ink}    name="文档墨"   code="B-INK"   hex="#111418" large/>
          <Swatch color={B.paper}  name="档案纸"   code="B-PAPER" hex="#FAFAF7" large/>
          <Swatch color={B.accent} name="航海蓝"   code="B-NAVY"  hex="#1B3A6B" large/>
        </div>
        <Caption style={{ marginBottom: 12 }}>② 中性色阶 · 10 STEPS</Caption>
        <div style={{ display: 'flex', height: 56 }}>
          {[1,0.9,0.78,0.65,0.5,0.36,0.24,0.14,0.07,0.02].map((v, i) => {
            const c = i === 9 ? B.paper : `rgba(17,20,24,${v})`;
            return (
              <div key={i} style={{ flex: 1, background: c, position: 'relative' }}>
                <div className="mono" style={{ position: 'absolute', bottom: 4, left: 4, fontSize: 8, color: v > 0.5 ? B.paper : B.ink, opacity: 0.7 }}>{`B-N${(i+1)*100}`}</div>
              </div>
            );
          })}
        </div>
        <Caption style={{ marginTop: 14 }}>③ 蓝色阶 · NAVY · 用于强调与点缀，不超过 5% 面积</Caption>
        <div style={{ display: 'flex', height: 36, marginTop: 6 }}>
          {[0.06,0.14,0.3,0.5,0.7,1].map((v, i) => {
            const c = i === 5 ? B.accent : `rgba(27,58,107,${v})`;
            return <div key={i} style={{ flex: 1, background: c }}/>;
          })}
        </div>
      </div>
    </div>
  );
}

// ── B4 · Type
function B4_Type() {
  return (
    <div style={{ width: '100%', height: '100%', background: B.paper, color: B.ink }}>
      <MetaB id="VI-B · 04" title="字体 · Type System" sub="INSTRUMENT SERIF · INTER · NOTO"/>
      <div style={{ padding: '28px 32px', display: 'grid', gridTemplateColumns: '1fr 1.1fr', gap: 28 }}>

        <div>
          <div className="mono" style={{ fontSize: 10, color: B.faint, letterSpacing: 0.8 }}>DISPLAY · 标题</div>
          <div className="serif" style={{ fontSize: 64, letterSpacing: -1.6, lineHeight: 0.95, marginTop: 4 }}>Binder</div>
          <div className="mono" style={{ fontSize: 9, color: B.faint, marginTop: 4 }}>Instrument Serif · Regular</div>

          <div className="mono" style={{ fontSize: 10, color: B.faint, letterSpacing: 0.8, marginTop: 22 }}>SUBHEAD · 副标题</div>
          <div className="sans" style={{ fontSize: 24, fontWeight: 500, letterSpacing: -0.6, marginTop: 6 }}>The local-first editor</div>

          <div className="mono" style={{ fontSize: 10, color: B.faint, letterSpacing: 0.8, marginTop: 22 }}>BODY · 正文</div>
          <div className="sc" style={{ fontSize: 14, lineHeight: 1.65, marginTop: 6, color: B.ink }}>
            文档由段落组成。每一段都有自己的身份与历史。
          </div>
          <div className="mono" style={{ fontSize: 9, color: B.faint, marginTop: 4 }}>Noto Sans SC · 14/22</div>
        </div>

        <div style={{ borderLeft: `1px solid ${B.rule}`, paddingLeft: 24 }}>
          <Caption style={{ marginBottom: 10 }}>① 字号阶梯</Caption>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
            {[
              [64, 'serif', 'Display'],
              [40, 'serif', 'H1'],
              [24, 'sans',  'H2'],
              [16, 'sans',  'Body'],
              [12, 'sc',    '注释'],
              [10, 'mono',  'UI'],
            ].map(([sz, klass, name]) => (
              <div key={name} style={{ display: 'flex', alignItems: 'baseline', gap: 12 }}>
                <div className="mono" style={{ width: 28, fontSize: 9, color: B.faint }}>{sz}</div>
                <div className={klass} style={{ fontSize: sz, lineHeight: 1, fontWeight: klass === 'sans' ? 500 : 400 }}>Aa</div>
                <div className="mono" style={{ fontSize: 9, color: B.faint, marginLeft: 'auto' }}>{name}</div>
              </div>
            ))}
          </div>
          <Caption style={{ marginTop: 18 }}>② 双语规则</Caption>
          <div className="sc" style={{ fontSize: 13, lineHeight: 1.7, marginTop: 6, padding: 12, background: '#fff', border: `1px solid ${B.rule}` }}>
            <span className="serif" style={{ fontSize: 18 }}>Diff</span> 即契约 —— 所有 AI 修改先以差异呈现，由你审阅后写入文件。
          </div>
        </div>
      </div>
    </div>
  );
}

// ── B5 · Aux + Icon
function B5_Aux() {
  const icons = [
    { id: 'block',     path: 'M3 3h10v2H3zM3 7h7v2H3zM3 11h10v2H3z' },
    { id: 'workspace', path: 'M2 4h12v9H2zM2 4l3-2h4l3 2' },
    { id: 'diff',      path: 'M3 4h6M3 8h6M3 12h6M11 4v8M9 8h4', stroke: true },
    { id: 'accept',    path: 'M2 8l3 3 7-7',                      stroke: true },
    { id: 'reject',    path: 'M3 3l10 10M13 3L3 13',               stroke: true },
    { id: 'agent',     path: 'M4 3h8v3H4zM4 8h8v5H4zM6 10h4',      stroke: true },
    { id: 'search',    path: 'M7 3a4 4 0 1 0 0 8 4 4 0 0 0 0-8zM10 10l3 3', stroke: true },
    { id: 'paragraph', path: 'M3 3h10v1H3zM3 6h7v1H3zM3 9h10v1H3zM3 12h6v1H3' },
  ];

  return (
    <div style={{ width: '100%', height: '100%', background: B.paper, color: B.ink }}>
      <MetaB id="VI-B · 05" title="辅助图形 · Auxiliary & Iconography"/>
      <div style={{ padding: '28px 32px', display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 24 }}>

        {/* aux pattern: paragraph rhythm */}
        <div>
          <Caption style={{ marginBottom: 10 }}>① 段落韵律纹样 · PARAGRAPH RHYTHM</Caption>
          <div style={{ height: 200, background: '#fff', border: `1px solid ${B.rule}`, padding: 18, overflow: 'hidden' }}>
            {Array.from({ length: 24 }).map((_, i) => {
              const w = 30 + ((i * 37) % 60);
              const isAccent = i % 11 === 3;
              return (
                <div key={i} style={{
                  height: 4, marginBottom: 4,
                  width: `${w}%`,
                  background: isAccent ? B.accent : B.ink,
                  opacity: isAccent ? 1 : 0.16,
                }}/>
              );
            })}
          </div>
          <Caption style={{ marginTop: 10 }}>② 同一图形即文档段落示意</Caption>
        </div>

        <div>
          <Caption style={{ marginBottom: 10 }}>③ 图标系统 · 16PX · STROKE/FILL HYBRID</Caption>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4,1fr)', gap: 1, background: B.rule, border: `1px solid ${B.rule}` }}>
            {icons.map(ic => (
              <div key={ic.id} style={{ aspectRatio: 1, background: '#fff', display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', gap: 4, padding: 6 }}>
                <svg viewBox="0 0 16 16" width="22" height="22"
                  fill={ic.stroke ? 'none' : B.ink}
                  stroke={ic.stroke ? B.ink : 'none'}
                  strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round">
                  <path d={ic.path}/>
                </svg>
                <div className="mono" style={{ fontSize: 8, color: B.faint, letterSpacing: 0.4 }}>{ic.id}</div>
              </div>
            ))}
          </div>
          <Caption style={{ marginTop: 10 }}>④ 文档目录卡示意</Caption>
          <div style={{ marginTop: 8, padding: 12, background: '#fff', border: `1px solid ${B.rule}` }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
              <svg viewBox="0 0 16 16" width="14" height="14" fill={B.ink}><path d="M3 3h10v2H3zM3 7h7v2H3zM3 11h10v2H3z"/></svg>
              <div className="sans" style={{ fontSize: 12, fontWeight: 500 }}>spec.md</div>
              <div className="mono" style={{ fontSize: 9, color: B.accent, marginLeft: 'auto' }}>3 BLOCKS</div>
            </div>
            <div style={{ height: 3, background: B.ink, opacity: 0.16, width: '90%', marginBottom: 3 }}/>
            <div style={{ height: 3, background: B.ink, opacity: 0.16, width: '60%', marginBottom: 3 }}/>
            <div style={{ height: 3, background: B.accent, width: '80%' }}/>
          </div>
        </div>
      </div>
    </div>
  );
}

// ── B6 · Print
function B6_Print() {
  return (
    <div style={{ width: '100%', height: '100%', background: B.fog, color: B.ink }}>
      <MetaB id="VI-B · 06" title="印刷应用 · Print Applications"/>
      <div style={{ padding: '28px 32px', display: 'grid', gridTemplateColumns: '1fr 1.05fr', gap: 24 }}>

        <div>
          <Caption style={{ marginBottom: 10 }}>① 名片 · 85 × 55 MM · 双面</Caption>
          <div style={{ background: B.paper, padding: 18, marginBottom: 10, boxShadow: '0 4px 14px rgba(0,0,0,0.08)', aspectRatio: '85/55', position: 'relative' }}>
            <div style={{ display: 'flex', alignItems: 'flex-start', gap: 16, height: '100%' }}>
              <LogoB size={62}/>
              <div style={{ flex: 1, display: 'flex', flexDirection: 'column', justifyContent: 'space-between', paddingTop: 4 }}>
                <div>
                  <div className="serif" style={{ fontSize: 20, lineHeight: 1 }}>Tao Mu</div>
                  <div className="sc" style={{ fontSize: 11, color: B.muted, marginTop: 2 }}>陶木 · 核心维护者</div>
                </div>
                <div className="mono" style={{ fontSize: 8.5, color: B.faint, letterSpacing: 0.4, lineHeight: 1.5 }}>
                  tao@binder.dev<br/>+86 188 0000 0000<br/>binder.dev
                </div>
              </div>
            </div>
          </div>
          <div style={{ background: B.ink, color: B.paper, padding: 18, aspectRatio: '85/55', boxShadow: '0 4px 14px rgba(0,0,0,0.08)', position: 'relative', overflow: 'hidden' }}>
            <div style={{ position: 'absolute', inset: 16 }}>
              {Array.from({ length: 18 }).map((_, i) => {
                const w = 30 + ((i * 37) % 60);
                const isAccent = i === 4 || i === 11;
                return (
                  <div key={i} style={{
                    height: 3, marginBottom: 3,
                    width: `${w}%`,
                    background: isAccent ? B.accent : B.paper,
                    opacity: isAccent ? 1 : 0.32,
                  }}/>
                );
              })}
            </div>
            <div className="mono" style={{ position: 'absolute', bottom: 12, right: 16, fontSize: 9, color: B.paper, opacity: 0.6, letterSpacing: 1 }}>BINDER · CORE</div>
          </div>
        </div>

        <div>
          <Caption style={{ marginBottom: 10 }}>② 海报 · A2 · BLOCK COMPOSITION</Caption>
          <div style={{ background: B.paper, aspectRatio: '420/594', boxShadow: '0 8px 20px rgba(0,0,0,0.10)', padding: '20px 22px', position: 'relative', overflow: 'hidden' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
              <LogoB size={40}/>
              <div className="mono" style={{ fontSize: 8, color: B.faint, letterSpacing: 1, textAlign: 'right' }}>
                ISSUE 01<br/>2026 / SPRING
              </div>
            </div>
            <div className="serif" style={{ position: 'absolute', top: 90, left: 22, right: 22, fontSize: 40, lineHeight: 0.92, color: B.ink, letterSpacing: -1.4 }}>
              Documents<br/>are blocks.<br/><span style={{ color: B.accent }}>Edits</span><br/>are diffs.
            </div>
            {/* aux bars */}
            <div style={{ position: 'absolute', bottom: 60, left: 22, right: 22 }}>
              {[100, 64, 82, 100, 72, 56].map((w, i) => (
                <div key={i} style={{ height: 4, marginBottom: 5, width: `${w * 0.7}%`, background: i % 5 === 2 ? B.accent : B.ink, opacity: i % 5 === 2 ? 1 : 0.16 }}/>
              ))}
            </div>
            <div style={{ position: 'absolute', bottom: 18, left: 22, right: 22, display: 'flex', justifyContent: 'space-between' }}>
              <Caption>BINDER.DEV / CORE</Caption>
              <Caption>OPEN SOURCE</Caption>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

// ── B7 · Digital
function B7_Digital() {
  return (
    <div style={{ width: '100%', height: '100%', background: B.fog, color: B.ink }}>
      <MetaB id="VI-B · 07" title="数字应用 · Digital Applications"/>
      <div style={{ padding: '28px 32px', display: 'grid', gridTemplateColumns: '0.7fr 1.4fr', gap: 24 }}>
        <div>
          <Caption style={{ marginBottom: 10 }}>① 应用图标</Caption>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
            {[
              { s: 88, bg: B.paper, color: B.ink },
              { s: 56, bg: B.ink, color: B.paper },
              { s: 56, bg: B.accent, color: B.paper },
            ].map((v, i) => (
              <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 14 }}>
                <div style={{ width: v.s, height: v.s, background: v.bg, borderRadius: v.s * 0.22, display: 'flex', alignItems: 'center', justifyContent: 'center', boxShadow: '0 2px 8px rgba(0,0,0,0.12)' }}>
                  <LogoB size={v.s * 0.62} color={v.color}/>
                </div>
                <div className="mono" style={{ fontSize: 9, color: B.faint }}>{v.s}</div>
              </div>
            ))}
          </div>
        </div>

        <div>
          <Caption style={{ marginBottom: 10 }}>② 编辑器界面</Caption>
          <AppWindow style={{ height: 290 }} title="binder · ~/docs/spec.md">
            <div style={{ display: 'flex', height: 'calc(100% - 33px)' }}>
              <div style={{ width: 110, borderRight: `1px solid ${B.rule}`, padding: 10, fontSize: 10 }}>
                <div className="mono" style={{ color: B.faint, fontSize: 8, letterSpacing: 0.6, marginBottom: 8 }}>WORKSPACE</div>
                {['📁 docs','  spec.md','  intro.md','  notes.txt','📁 .binder'].map((f, i) => {
                  const active = i === 1;
                  return (
                    <div key={i} style={{ padding: '3px 6px', marginLeft: i > 0 && i < 4 ? 8 : 0, borderRadius: 3,
                      background: active ? 'rgba(27,58,107,0.08)' : 'transparent',
                      color: active ? B.accent : B.muted,
                      fontFamily: 'Inter, sans-serif',
                      fontWeight: active ? 600 : 400 }}>{f.replace(/^📁\s/, '▾ ').replace(/^\s+/, '  ')}</div>
                  );
                })}
              </div>
              <div style={{ flex: 1, padding: '14px 18px', borderRight: `1px solid ${B.rule}` }}>
                <div className="serif" style={{ fontSize: 18, marginBottom: 10 }}>Binder Core</div>
                <div className="sc" style={{ fontSize: 11, color: B.muted, lineHeight: 1.6 }}>
                  本地优先的 AI 文档编辑器。
                </div>
                <div style={{ marginTop: 8 }}>
                  <DiffLine kind="-">支持 markdown / txt 编辑</DiffLine>
                  <DiffLine kind="+">支持 markdown / txt / 段落级 diff 审阅</DiffLine>
                </div>
                <div className="sc" style={{ fontSize: 11, color: B.muted, lineHeight: 1.6, marginTop: 6 }}>
                  workspace.db 存储所有事实。
                </div>
              </div>
              <div style={{ width: 140, padding: 10, background: '#fbfaf6' }}>
                <div className="mono" style={{ color: B.faint, fontSize: 8, letterSpacing: 0.6, marginBottom: 8 }}>AGENT</div>
                <div style={{ padding: 6, background: '#fff', border: `1px solid ${B.rule}`, marginBottom: 6, fontSize: 10 }}>
                  <div className="mono" style={{ fontSize: 8, color: B.faint }}>YOU</div>
                  <div className="sc" style={{ marginTop: 2 }}>更新 spec 第三段</div>
                </div>
                <div style={{ padding: 6, background: '#fff', border: `1px solid ${B.accent}`, fontSize: 10 }}>
                  <div className="mono" style={{ fontSize: 8, color: B.accent }}>1 PENDING DIFF</div>
                  <div className="mono" style={{ fontSize: 8, color: B.muted, marginTop: 2 }}>spec.md · ¶3</div>
                  <div style={{ display: 'flex', gap: 4, marginTop: 6 }}>
                    <div style={{ flex: 1, padding: '3px 0', background: B.accent, color: B.paper, textAlign: 'center', fontSize: 8 }} className="mono">ACCEPT</div>
                    <div style={{ flex: 1, padding: '3px 0', background: 'transparent', color: B.muted, textAlign: 'center', fontSize: 8, border: `1px solid ${B.rule}` }} className="mono">REJECT</div>
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

// ── B8 · Merch
function B8_Merch() {
  return (
    <div style={{ width: '100%', height: '100%', background: B.fog, color: B.ink }}>
      <MetaB id="VI-B · 08" title="周边 · Merch & VIP"/>
      <div style={{ padding: '28px 32px', display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 18 }}>

        <div>
          <Caption style={{ marginBottom: 10 }}>① 笔记本 · A5 HARDCOVER</Caption>
          <div style={{ aspectRatio: '5/7', background: B.ink, color: B.paper, padding: 24, boxShadow: '0 6px 18px rgba(0,0,0,0.18)', position: 'relative' }}>
            <LogoB size={56} color={B.paper}/>
            <div className="serif" style={{ fontSize: 30, marginTop: 22, lineHeight: 1.05 }}>Binder<br/>Workspace</div>
            <div className="mono" style={{ position: 'absolute', bottom: 24, left: 24, fontSize: 8, color: 'rgba(255,255,255,0.5)', letterSpacing: 1 }}>VOL · 01</div>
            <div className="mono" style={{ position: 'absolute', bottom: 24, right: 24, fontSize: 8, color: B.accent, letterSpacing: 1 }}>2026</div>
          </div>
        </div>

        <div>
          <Caption style={{ marginBottom: 10 }}>② 贡献者徽章 · BADGE</Caption>
          <div style={{ aspectRatio: '85/120', background: B.paper, padding: 16, boxShadow: '0 6px 18px rgba(0,0,0,0.10)', display: 'flex', flexDirection: 'column', justifyContent: 'space-between', position: 'relative' }}>
            <div>
              <LogoB size={50}/>
              <div className="mono" style={{ fontSize: 8, color: B.muted, marginTop: 12, letterSpacing: 1 }}>CONTRIBUTOR · 2026</div>
            </div>
            <PhotoSlot label="PORTRAIT" style={{ aspectRatio: 1, marginTop: 8 }}/>
            <div>
              <div className="serif" style={{ fontSize: 18, lineHeight: 1 }}>Tao Mu</div>
              <div className="sc" style={{ fontSize: 10, color: B.muted, marginTop: 2 }}>陶木 · 核心维护者</div>
              <div className="mono" style={{ fontSize: 8, color: B.accent, marginTop: 6, letterSpacing: 1.2 }}>NO. 00017</div>
            </div>
          </div>
        </div>

        <div>
          <Caption style={{ marginBottom: 10 }}>③ 包装 · BOX</Caption>
          <div style={{ aspectRatio: '4/5', background: B.paper, padding: 18, boxShadow: '0 4px 14px rgba(0,0,0,0.08)', position: 'relative', overflow: 'hidden' }}>
            <div style={{ position: 'absolute', top: 0, left: 0, right: 0, height: 28, background: B.accent, display: 'flex', alignItems: 'center', padding: '0 14px' }}>
              <div className="mono" style={{ fontSize: 9, color: B.paper, letterSpacing: 2 }}>BINDER · STARTER KIT</div>
            </div>
            <div style={{ marginTop: 50 }}>
              <LogoB size={70}/>
              <div className="serif" style={{ fontSize: 24, marginTop: 12, lineHeight: 1 }}>Binder<br/>Core</div>
              <div className="sc" style={{ fontSize: 11, color: B.muted, marginTop: 8 }}>本地优先 · 开源 · 极简</div>
            </div>
            <div className="mono" style={{ position: 'absolute', bottom: 14, left: 14, fontSize: 8, color: B.faint, letterSpacing: 0.6 }}>v1.0 · 2026</div>
            <div className="mono" style={{ position: 'absolute', bottom: 14, right: 14, fontSize: 8, color: B.faint }}>binder.dev</div>
          </div>
        </div>
      </div>
    </div>
  );
}

Object.assign(window, { B_palette: B, LogoB, B1_Hero, B2_System, B3_Color, B4_Type, B5_Aux, B6_Print, B7_Digital, B8_Merch });
