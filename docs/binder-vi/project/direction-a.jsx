/* direction-a.jsx — Direction A · Bracket [B] · 极简编辑符号 */

const A = {
  ink:   '#0E0C0A',    // near-black with warm tint
  paper: '#F4F1EA',    // warm paper
  fog:   '#E6E2D8',    // slightly darker paper
  rule:  'rgba(14,12,10,0.12)',
  muted: 'rgba(14,12,10,0.5)',
  faint: 'rgba(14,12,10,0.3)',
  accent:'#C24A2A',    // clay — used VERY sparingly (single-color accent)
  green: '#2E6F4A',    // accept (only on diff visuals)
};

// ─── Logo mark — bracketed B
function LogoA({ size = 160, color = A.ink, paper = null }) {
  const h = size * 0.62;
  return (
    <svg viewBox="0 0 200 124" width={size} height={h} style={{ display: 'block' }}>
      {paper && <rect width="200" height="124" fill={paper}/>}
      {/* left bracket */}
      <path d="M 36 14 L 10 14 L 10 110 L 36 110"
            stroke={color} fill="none" strokeWidth="10"
            strokeLinecap="square" strokeLinejoin="miter"/>
      {/* right bracket */}
      <path d="M 164 14 L 190 14 L 190 110 L 164 110"
            stroke={color} fill="none" strokeWidth="10"
            strokeLinecap="square" strokeLinejoin="miter"/>
      {/* B — set in heavy geometric sans, locked to viewBox */}
      <text x="100" y="86" textAnchor="middle"
            fontFamily="'IBM Plex Sans','Inter',sans-serif"
            fontWeight="600" fontSize="86" fill={color}
            letterSpacing="-2">B</text>
    </svg>
  );
}

// wordmark
function WordmarkA({ size = 56, color = A.ink, sub = true }) {
  return (
    <div style={{ display: 'inline-flex', flexDirection: 'column', alignItems: 'flex-start', gap: 4 }}>
      <div style={{
        fontFamily: "'IBM Plex Sans', 'Inter', sans-serif",
        fontWeight: 500, fontSize: size, letterSpacing: -size * 0.04,
        color, lineHeight: 0.95,
      }}>binder<span style={{ color: A.accent }}>.</span></div>
      {sub && <div className="mono" style={{
        fontSize: size * 0.16, letterSpacing: size * 0.05,
        textTransform: 'uppercase', color: A.muted, marginLeft: 1,
      }}>CORE / LOCAL FIRST EDITOR</div>}
    </div>
  );
}

// ── Artboard A1 · Hero / cover
function A1_Hero() {
  return (
    <div style={{ width: '100%', height: '100%', background: A.paper, color: A.ink, position: 'relative', overflow: 'hidden' }}>
      <MetaStrip id="VI-A · 01" title="主标 · Primary Lockup" sub="方向 A — Bracket" accent={A.accent}/>

      <div style={{ position: 'absolute', inset: '60px 56px', display: 'flex', flexDirection: 'column', justifyContent: 'space-between' }}>
        <div className="mono" style={{ fontSize: 10, letterSpacing: 1.2, color: A.faint, textTransform: 'uppercase' }}>
          ① 中括号包围的字母 B — 致敬编辑器中的光标与代码块
        </div>

        <div style={{ display: 'flex', alignItems: 'center', gap: 36 }}>
          <LogoA size={170}/>
          <div style={{ width: 1, height: 110, background: A.rule }}/>
          <div>
            <div style={{ fontFamily: "'IBM Plex Sans', sans-serif", fontWeight: 500, fontSize: 76, letterSpacing: -3, lineHeight: 0.92, color: A.ink }}>
              binder<span style={{ color: A.accent }}>.</span>
            </div>
            <div className="sc" style={{ fontSize: 13, color: A.muted, marginTop: 10, letterSpacing: 0.4 }}>
              本地优先的 AI 文档编辑器最小核心
            </div>
            <div className="mono" style={{ fontSize: 11, color: A.faint, marginTop: 4, letterSpacing: 0.8, textTransform: 'uppercase' }}>
              A local-first AI editor — minimal core
            </div>
          </div>
        </div>

        <div style={{ display: 'flex', alignItems: 'flex-end', justifyContent: 'space-between' }}>
          <Caption>① ASCII MONOGRAM · [B]</Caption>
          <Caption>② WORDMARK · IBM PLEX SANS 500</Caption>
          <Caption>03 / 24</Caption>
        </div>
      </div>
    </div>
  );
}

// ── Artboard A2 · Logo construction + variants
function A2_System() {
  return (
    <div style={{ width: '100%', height: '100%', background: A.paper, color: A.ink, position: 'relative' }}>
      <MetaStrip id="VI-A · 02" title="结构 · Construction & Variants" accent={A.accent}/>
      <div style={{ padding: '28px 32px', display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 24 }}>

        {/* construction grid */}
        <div>
          <Caption style={{ marginBottom: 10 }}>① 构造网格 · 12 × 12 UNITS</Caption>
          <div className="gridbg" style={{ position: 'relative', height: 200, padding: 16, border: `1px solid ${A.rule}`, background: '#fff' }}>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%' }}>
              <LogoA size={140}/>
            </div>
            <div className="mono" style={{ position: 'absolute', top: 4, left: 6, fontSize: 8, color: A.faint }}>0,0</div>
            <div className="mono" style={{ position: 'absolute', bottom: 4, right: 6, fontSize: 8, color: A.faint }}>12,12</div>
          </div>
          <Caption style={{ marginTop: 8 }}>② 最小尺寸 · MIN HEIGHT 18 PX / 12 MM</Caption>
        </div>

        {/* clear space */}
        <div>
          <Caption style={{ marginBottom: 10 }}>② 安全距离 · CLEAR SPACE = X</Caption>
          <div style={{ position: 'relative', height: 200, background: '#fff', border: `1px solid ${A.rule}`, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
            {/* dashed clearspace box */}
            <div style={{ position: 'absolute', inset: 36, border: `1px dashed ${A.faint}` }}/>
            <LogoA size={110}/>
            <div className="mono" style={{ position: 'absolute', top: 8, left: 8, fontSize: 9, color: A.muted }}>x</div>
          </div>
          <div style={{ display: 'flex', gap: 8, marginTop: 12 }}>
            <div style={{ flex: 1, padding: 8, background: '#fff', border: `1px solid ${A.rule}`, display: 'flex', justifyContent: 'center' }}>
              <LogoA size={32}/>
            </div>
            <div style={{ flex: 1, padding: 8, background: A.ink, display: 'flex', justifyContent: 'center' }}>
              <LogoA size={32} color={A.paper}/>
            </div>
            <div style={{ flex: 1, padding: 8, background: A.accent, display: 'flex', justifyContent: 'center' }}>
              <LogoA size={32} color={A.paper}/>
            </div>
          </div>
        </div>

        {/* lockup variations */}
        <div style={{ gridColumn: 'span 2' }}>
          <Caption style={{ marginBottom: 10 }}>③ 组合应用 · LOCKUP VARIATIONS</Caption>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3,1fr)', gap: 12 }}>
            <div style={{ padding: '16px 12px', background: '#fff', border: `1px solid ${A.rule}`, textAlign: 'center' }}>
              <LogoA size={60}/>
              <div className="mono" style={{ fontSize: 9, color: A.faint, marginTop: 8, letterSpacing: 0.6 }}>MARK ONLY</div>
            </div>
            <div style={{ padding: '16px 12px', background: '#fff', border: `1px solid ${A.rule}`, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 6 }}>
              <LogoA size={42}/>
              <div style={{ fontFamily: "'IBM Plex Sans', sans-serif", fontWeight: 500, fontSize: 18, letterSpacing: -0.6 }}>binder<span style={{ color: A.accent }}>.</span></div>
              <div className="mono" style={{ fontSize: 9, color: A.faint, marginTop: 4, letterSpacing: 0.6 }}>VERTICAL</div>
            </div>
            <div style={{ padding: '16px 12px', background: '#fff', border: `1px solid ${A.rule}`, display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 10 }}>
              <LogoA size={40}/>
              <div style={{ fontFamily: "'IBM Plex Sans', sans-serif", fontWeight: 500, fontSize: 22, letterSpacing: -0.8 }}>binder<span style={{ color: A.accent }}>.</span></div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

// ── Artboard A3 · Color
function A3_Color() {
  return (
    <div style={{ width: '100%', height: '100%', background: A.paper, color: A.ink }}>
      <MetaStrip id="VI-A · 03" title="色彩 · Color System" sub="MONOCHROME + CLAY ACCENT" accent={A.accent}/>
      <div style={{ padding: '28px 32px' }}>
        <Caption style={{ marginBottom: 12 }}>① 主色 · CORE</Caption>
        <div style={{ display: 'grid', gridTemplateColumns: '2fr 2fr 1fr', gap: 14, marginBottom: 24 }}>
          <Swatch color={A.ink}    name="编辑器墨" code="A-INK"   hex="#0E0C0A" large/>
          <Swatch color={A.paper}  name="文档纸"   code="A-PAPER" hex="#F4F1EA" large/>
          <Swatch color={A.accent} name="批注红"   code="A-CLAY"  hex="#C24A2A" large/>
        </div>
        <Caption style={{ marginBottom: 12 }}>② 中性色阶 · NEUTRAL SCALE — 单色系统的全部呼吸</Caption>
        <div style={{ display: 'flex', height: 64 }}>
          {[1,0.92,0.82,0.7,0.58,0.42,0.28,0.15,0.07,0].map((v,i) => {
            const c = `rgba(14,12,10,${v})`;
            return (
              <div key={i} style={{ flex: 1, background: i === 9 ? A.paper : c, position: 'relative' }}>
                <div className="mono" style={{ position: 'absolute', bottom: 4, left: 4, fontSize: 8, color: v > 0.5 ? A.paper : A.ink, opacity: 0.7 }}>
                  {String(i*10).padStart(2,'0')}
                </div>
              </div>
            );
          })}
        </div>
        <Caption style={{ marginTop: 14 }}>③ 状态色 · 仅用于 DIFF 审阅，不出现在品牌物料中</Caption>
        <div style={{ display: 'flex', gap: 10, marginTop: 8 }}>
          <div style={{ flex: 1, padding: '8px 12px', background: 'rgba(46,111,74,0.08)', borderLeft: `3px solid ${A.green}` }}>
            <div className="mono" style={{ fontSize: 10, color: A.green }}>+ ACCEPT  #2E6F4A</div>
          </div>
          <div style={{ flex: 1, padding: '8px 12px', background: 'rgba(180,70,55,0.08)', borderLeft: `3px solid #B44637` }}>
            <div className="mono" style={{ fontSize: 10, color: '#B44637' }}>− REJECT  #B44637</div>
          </div>
        </div>
      </div>
    </div>
  );
}

// ── Artboard A4 · Type
function A4_Type() {
  return (
    <div style={{ width: '100%', height: '100%', background: A.paper, color: A.ink }}>
      <MetaStrip id="VI-A · 04" title="字体 · Type System" sub="IBM PLEX FAMILY · NOTO SC" accent={A.accent}/>
      <div style={{ padding: '28px 32px', display: 'grid', gridTemplateColumns: '1fr 1.2fr', gap: 28 }}>
        <div>
          <div className="mono" style={{ fontSize: 10, color: A.faint, letterSpacing: 0.8 }}>DISPLAY · 标题</div>
          <div style={{ fontFamily: "'IBM Plex Sans', sans-serif", fontWeight: 500, fontSize: 56, letterSpacing: -2, lineHeight: 1, marginTop: 6 }}>Aa 编辑</div>
          <div className="mono" style={{ fontSize: 9, color: A.faint, marginTop: 8 }}>IBM Plex Sans Medium · −0.04em</div>

          <div className="mono" style={{ fontSize: 10, color: A.faint, letterSpacing: 0.8, marginTop: 24 }}>BODY · 正文</div>
          <div style={{ fontFamily: "'IBM Plex Sans', sans-serif", fontSize: 16, lineHeight: 1.55, marginTop: 6 }}>
            Diff as the contract.<br/>
            <span className="sc">差异即契约。</span>
          </div>
          <div className="mono" style={{ fontSize: 9, color: A.faint, marginTop: 6 }}>Plex Sans Regular · Noto Sans SC</div>

          <div className="mono" style={{ fontSize: 10, color: A.faint, letterSpacing: 0.8, marginTop: 24 }}>MONO · UI / 代码</div>
          <div className="mono" style={{ fontSize: 16, lineHeight: 1.45, marginTop: 6 }}>
            workspace.db<br/>
            $ binder ./docs
          </div>
        </div>

        <div style={{ borderLeft: `1px solid ${A.rule}`, paddingLeft: 24 }}>
          <Caption style={{ marginBottom: 10 }}>① 字号阶梯 · TYPE SCALE</Caption>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 6, marginBottom: 22 }}>
            {[
              [56,'h1','显示'],
              [36,'h2','章节'],
              [22,'h3','段题'],
              [16,'body','正文'],
              [13,'meta','说明'],
              [11,'mono','UI'],
            ].map(([sz, n, cn]) => (
              <div key={n} style={{ display: 'flex', alignItems: 'baseline', gap: 14 }}>
                <div className="mono" style={{ width: 38, fontSize: 9, color: A.faint }}>{sz}/{n}</div>
                <div style={{ fontFamily: "'IBM Plex Sans', sans-serif", fontWeight: 500, fontSize: sz, letterSpacing: -sz*0.025, lineHeight: 1 }}>Bb</div>
                <div className="sc" style={{ fontSize: 11, color: A.muted, marginLeft: 'auto' }}>{cn}</div>
              </div>
            ))}
          </div>
          <Caption>② 双语混排 · BILINGUAL SETTING</Caption>
          <div style={{ marginTop: 8, padding: 12, background: '#fff', border: `1px solid ${A.rule}` }}>
            <div className="sc" style={{ fontSize: 14, lineHeight: 1.7, color: A.ink }}>
              Binder Core 是一个<span style={{ color: A.accent }}>本地优先</span>的 AI 文档编辑器最小核心，用户通过对话让 AI 修改文档，AI 修改以 <span className="mono" style={{ fontSize: 13 }}>diff</span> 形式呈现。
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

// ── Artboard A5 · Aux + Icon
function A5_Aux() {
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
    <div style={{ width: '100%', height: '100%', background: A.paper, color: A.ink }}>
      <MetaStrip id="VI-A · 05" title="辅助图形 · Auxiliary & Iconography" accent={A.accent}/>
      <div style={{ padding: '28px 32px', display: 'grid', gridTemplateColumns: '1.1fr 1fr', gap: 24 }}>
        <div>
          <Caption style={{ marginBottom: 10 }}>① 辅助图形 · 字符栅格纹样</Caption>
          <div style={{ height: 200, padding: 18, background: '#fff', border: `1px solid ${A.rule}`, overflow: 'hidden', position: 'relative' }}>
            <div className="mono" style={{ fontSize: 13, lineHeight: 1.2, color: A.faint, whiteSpace: 'pre', letterSpacing: 1 }}>
              {Array.from({length: 14}).map((_, i) => (
                <div key={i}>{
                  Array.from({length: 22}).map((_, j) => {
                    const k = (i*7+j*3) % 13;
                    if (k===0) return '[';
                    if (k===1) return ']';
                    if (k===2) return '+';
                    if (k===3) return '·';
                    if (k===4) return '−';
                    if (k===5) return 'B';
                    return ' ';
                  }).join(' ')
                }</div>
              ))}
            </div>
            <div style={{ position: 'absolute', inset: 0, background: `linear-gradient(180deg, transparent 40%, ${A.paper} 100%)` }}/>
            <div style={{ position: 'absolute', bottom: 12, right: 16, color: A.accent }} className="mono">
              [B] · NOISE PATTERN
            </div>
          </div>
          <Caption style={{ marginTop: 12 }}>② 编辑符号 · 段落 / 流程标记</Caption>
          <div style={{ display: 'flex', gap: 12, marginTop: 8 }}>
            {['¶','§','†','¬','◇'].map((g) => (
              <div key={g} style={{ width: 48, height: 48, background: '#fff', border: `1px solid ${A.rule}`, display: 'flex', alignItems: 'center', justifyContent: 'center', fontFamily: "'IBM Plex Serif',serif", fontSize: 24, color: A.ink }}>{g}</div>
            ))}
          </div>
        </div>

        <div>
          <Caption style={{ marginBottom: 10 }}>③ 图标系统 · 16PX GRID, 1.5 STROKE</Caption>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 1, background: A.rule, border: `1px solid ${A.rule}` }}>
            {icons.map(ic => (
              <div key={ic.id} style={{ aspectRatio: '1', background: '#fff', display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', gap: 6, padding: 8 }}>
                <svg viewBox="0 0 16 16" width="22" height="22" fill="none" stroke={A.ink} strokeWidth="1.4" strokeLinecap="square" strokeLinejoin="miter">
                  <path d={ic.path}/>
                </svg>
                <div className="mono" style={{ fontSize: 8, color: A.faint, letterSpacing: 0.4 }}>{ic.id}</div>
              </div>
            ))}
          </div>
          <Caption style={{ marginTop: 12 }}>④ 图标在卡片中的应用</Caption>
          <div style={{ marginTop: 8, padding: 10, background: '#fff', border: `1px solid ${A.rule}` }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
              <svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke={A.accent} strokeWidth="1.4"><path d="M3 5h6M3 8h4M3 11h6M11 5v6M9 8h4"/></svg>
              <div className="mono" style={{ fontSize: 11, color: A.ink }}>1 pending diff</div>
            </div>
            <DiffLine kind="-">本地优先</DiffLine>
            <DiffLine kind="+">本地优先 + 离线可用</DiffLine>
          </div>
        </div>
      </div>
    </div>
  );
}

// ── Artboard A6 · Print applications
function A6_Print() {
  return (
    <div style={{ width: '100%', height: '100%', background: A.fog, color: A.ink, padding: 0, position: 'relative' }}>
      <MetaStrip id="VI-A · 06" title="印刷应用 · Print Applications" accent={A.accent}/>
      <div style={{ padding: '28px 32px', display: 'grid', gridTemplateColumns: '1fr 1.05fr', gap: 24 }}>

        {/* Business cards */}
        <div>
          <Caption style={{ marginBottom: 10 }}>① 名片 · 85 × 55 MM · 双面</Caption>
          {/* front */}
          <div style={{ background: A.paper, padding: 18, marginBottom: 10, boxShadow: '0 4px 14px rgba(0,0,0,0.08)', aspectRatio: '85/55', position: 'relative', display: 'flex', flexDirection: 'column', justifyContent: 'space-between' }}>
            <LogoA size={36}/>
            <div>
              <div style={{ fontFamily: "'IBM Plex Sans', sans-serif", fontWeight: 500, fontSize: 13, color: A.ink }}>Lin Yiran · 林一然</div>
              <div className="mono" style={{ fontSize: 9, color: A.muted, marginTop: 2 }}>FOUNDER &amp; MAINTAINER</div>
              <div className="mono" style={{ fontSize: 9, color: A.faint, marginTop: 10 }}>lin@binder.dev · binder.dev</div>
            </div>
            <div className="mono" style={{ position: 'absolute', bottom: 12, right: 14, fontSize: 9, color: A.accent, letterSpacing: 1 }}>[B]</div>
          </div>
          {/* back */}
          <div style={{ background: A.ink, color: A.paper, padding: 18, aspectRatio: '85/55', boxShadow: '0 4px 14px rgba(0,0,0,0.08)', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
            <LogoA size={56} color={A.paper}/>
          </div>
        </div>

        {/* poster */}
        <div>
          <Caption style={{ marginBottom: 10 }}>② 海报 · A2 · DIFF AS THE CONTRACT</Caption>
          <div style={{ background: A.paper, aspectRatio: '420/594', boxShadow: '0 8px 20px rgba(0,0,0,0.10)', padding: '22px 20px', position: 'relative', overflow: 'hidden' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between' }}>
              <LogoA size={28}/>
              <div className="mono" style={{ fontSize: 8, color: A.faint, letterSpacing: 1, textAlign: 'right' }}>
                VOL. 01<br/>SPRING 2026
              </div>
            </div>
            <div style={{ marginTop: 30, fontFamily: "'IBM Plex Sans', sans-serif", fontWeight: 500, fontSize: 36, letterSpacing: -1.4, lineHeight: 0.95, color: A.ink }}>
              Diff<br/>as the<br/>contract<span style={{ color: A.accent }}>.</span>
            </div>
            <div className="sc" style={{ position: 'absolute', bottom: 60, left: 20, right: 20, fontSize: 11, lineHeight: 1.6, color: A.muted }}>
              一切 AI 修改先以差异呈现，由你逐条审阅。文档掌握在你手里。
            </div>
            <div style={{ position: 'absolute', bottom: 20, left: 20, right: 20, display: 'flex', justifyContent: 'space-between' }}>
              <Caption>[B] · BINDER.DEV</Caption>
              <Caption>OPEN SOURCE</Caption>
            </div>
          </div>
        </div>

      </div>
    </div>
  );
}

// ── Artboard A7 · Digital applications
function A7_Digital() {
  return (
    <div style={{ width: '100%', height: '100%', background: A.fog, color: A.ink }}>
      <MetaStrip id="VI-A · 07" title="数字应用 · Digital Applications" accent={A.accent}/>
      <div style={{ padding: '28px 32px', display: 'grid', gridTemplateColumns: '0.7fr 1.4fr', gap: 24 }}>
        {/* app icon stack */}
        <div>
          <Caption style={{ marginBottom: 10 }}>① 应用图标 · APP ICON</Caption>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
            {[88, 56, 36].map((s, i) => (
              <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 14 }}>
                <div style={{ width: s, height: s, background: A.paper, borderRadius: s * 0.22, display: 'flex', alignItems: 'center', justifyContent: 'center', boxShadow: '0 2px 8px rgba(0,0,0,0.10)' }}>
                  <LogoA size={s * 0.78}/>
                </div>
                <div className="mono" style={{ fontSize: 9, color: A.faint }}>{s} × {s}</div>
              </div>
            ))}
          </div>
          <div style={{ marginTop: 16, width: 88, height: 88, background: A.ink, borderRadius: 20, display: 'flex', alignItems: 'center', justifyContent: 'center', boxShadow: '0 2px 8px rgba(0,0,0,0.18)' }}>
            <LogoA size={68} color={A.paper}/>
          </div>
          <Caption style={{ marginTop: 8 }}>② DARK VARIANT</Caption>
        </div>

        {/* UI screenshot */}
        <div>
          <Caption style={{ marginBottom: 10 }}>③ 产品界面 · 编辑器 + 聊天面板</Caption>
          <AppWindow style={{ height: 290 }}>
            <div style={{ display: 'flex', height: 'calc(100% - 33px)' }}>
              {/* file tree */}
              <div style={{ width: 100, borderRight: '1px solid rgba(0,0,0,0.06)', padding: 10, fontSize: 10 }}>
                <div className="mono" style={{ color: A.faint, fontSize: 8, letterSpacing: 0.6, marginBottom: 8 }}>WORKSPACE</div>
                {['docs/','  intro.md','  spec.md','notes.txt','README.md'].map((f, i) => (
                  <div key={i} style={{ padding: '2px 0', color: i === 2 ? A.ink : A.muted, fontWeight: i === 2 ? 500 : 400, fontFamily: 'IBM Plex Mono, monospace' }}>{f}</div>
                ))}
              </div>
              {/* editor */}
              <div style={{ flex: 1, padding: '14px 18px', borderRight: '1px solid rgba(0,0,0,0.06)', fontSize: 11, color: A.ink }}>
                <div style={{ fontFamily: "'IBM Plex Sans', sans-serif", fontWeight: 600, fontSize: 14, marginBottom: 10 }}>spec.md</div>
                <div style={{ color: A.muted, lineHeight: 1.6 }}>
                  Binder Core 是一个本地优先的 AI 文档编辑器。
                </div>
                <div style={{ marginTop: 8 }}>
                  <DiffLine kind="-">所有 diff 自动接受。</DiffLine>
                  <DiffLine kind="+">所有 diff 默认 pending，由用户审阅。</DiffLine>
                </div>
                <div style={{ marginTop: 8, color: A.muted, lineHeight: 1.6 }}>
                  workspace.db 持久化全部状态。
                </div>
              </div>
              {/* chat */}
              <div style={{ width: 130, padding: 10, fontSize: 10, background: '#fbfaf6' }}>
                <div className="mono" style={{ color: A.faint, fontSize: 8, letterSpacing: 0.6, marginBottom: 8 }}>CHAT · AGENT</div>
                <div style={{ marginBottom: 8, padding: 6, background: '#fff', borderRadius: 3, border: `1px solid ${A.rule}` }}>
                  <div className="mono" style={{ fontSize: 8, color: A.faint }}>YOU</div>
                  <div style={{ marginTop: 2 }}>把"自动接受"改成审阅</div>
                </div>
                <div style={{ padding: 6, background: '#fff', borderRadius: 3, border: `1px solid ${A.rule}` }}>
                  <div className="mono" style={{ fontSize: 8, color: A.accent }}>BINDER · 1 DIFF</div>
                  <div style={{ marginTop: 4 }}>
                    <div className="mono" style={{ fontSize: 8 }}>spec.md · ¶3</div>
                  </div>
                  <div style={{ display: 'flex', gap: 4, marginTop: 6 }}>
                    <div style={{ flex: 1, padding: 2, background: A.green, color: '#fff', textAlign: 'center', fontSize: 8, fontFamily: 'IBM Plex Mono' }}>ACCEPT</div>
                    <div style={{ flex: 1, padding: 2, background: 'transparent', color: A.muted, textAlign: 'center', fontSize: 8, fontFamily: 'IBM Plex Mono', border: `1px solid ${A.rule}` }}>REJECT</div>
                  </div>
                </div>
              </div>
            </div>
          </AppWindow>
          <div style={{ display: 'flex', gap: 10, marginTop: 12 }}>
            <Caption>EDITOR · TIPTAP 3</Caption>
            <Caption>· DIFF AS PENDING</Caption>
            <Caption>· LOCAL FIRST</Caption>
          </div>
        </div>
      </div>
    </div>
  );
}

// ── Artboard A8 · Merch
function A8_Merch() {
  return (
    <div style={{ width: '100%', height: '100%', background: A.fog, color: A.ink }}>
      <MetaStrip id="VI-A · 08" title="周边 · Merch & VIP" accent={A.accent}/>
      <div style={{ padding: '28px 32px', display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 18 }}>

        {/* tote */}
        <div>
          <Caption style={{ marginBottom: 10 }}>① 帆布袋</Caption>
          <div style={{ aspectRatio: '4/5', background: A.paper, position: 'relative', boxShadow: '0 4px 14px rgba(0,0,0,0.08)', overflow: 'hidden' }}>
            <div style={{ position: 'absolute', top: 0, left: '14%', right: '14%', height: 18, borderLeft: `8px solid ${A.ink}`, borderRight: `8px solid ${A.ink}`, borderTop: 'none' }}/>
            <div style={{ position: 'absolute', inset: '40% 30% auto 30%' }}>
              <LogoA size={90}/>
            </div>
            <div style={{ position: 'absolute', bottom: 22, left: 0, right: 0, textAlign: 'center' }}>
              <div className="mono" style={{ fontSize: 9, color: A.muted, letterSpacing: 2 }}>DIFF AS THE CONTRACT</div>
            </div>
          </div>
        </div>

        {/* VIP / contributor card */}
        <div>
          <Caption style={{ marginBottom: 10 }}>② 贡献者卡 · CONTRIBUTOR</Caption>
          <div style={{ aspectRatio: '85/120', background: A.ink, color: A.paper, padding: 16, boxShadow: '0 6px 18px rgba(0,0,0,0.20)', display: 'flex', flexDirection: 'column', justifyContent: 'space-between' }}>
            <div>
              <LogoA size={42} color={A.paper}/>
              <div className="mono" style={{ fontSize: 8, color: 'rgba(255,255,255,0.5)', marginTop: 16, letterSpacing: 0.8 }}>CORE · CONTRIBUTOR / 2026</div>
            </div>
            <div>
              <div style={{ fontFamily: "'IBM Plex Sans', sans-serif", fontWeight: 500, fontSize: 18, letterSpacing: -0.6 }}>Lin Yiran</div>
              <div className="mono" style={{ fontSize: 8, color: 'rgba(255,255,255,0.5)', marginTop: 4 }}>NO. 000{Math.floor(Math.random()*9)+1}7</div>
            </div>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-end' }}>
              <div className="mono" style={{ fontSize: 8, color: A.accent, letterSpacing: 2 }}>[B]</div>
              <div className="mono" style={{ fontSize: 8, color: 'rgba(255,255,255,0.4)' }}>BINDER.DEV</div>
            </div>
          </div>
        </div>

        {/* sticker pack */}
        <div>
          <Caption style={{ marginBottom: 10 }}>③ 贴纸 · STICKER SET</Caption>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
            <div style={{ aspectRatio: 1, background: A.paper, border: `1px solid ${A.rule}`, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
              <LogoA size={50}/>
            </div>
            <div style={{ aspectRatio: 1, background: A.accent, color: A.paper, display: 'flex', alignItems: 'center', justifyContent: 'center', fontFamily: "'IBM Plex Sans', sans-serif", fontWeight: 600, fontSize: 32, letterSpacing: -1.5 }}>
              [B]
            </div>
            <div style={{ aspectRatio: 1, background: A.ink, color: A.paper, padding: 8, display: 'flex', flexDirection: 'column', justifyContent: 'space-between' }}>
              <div className="mono" style={{ fontSize: 8, letterSpacing: 1 }}>LOCAL FIRST</div>
              <div className="mono" style={{ fontSize: 18, fontWeight: 600 }}>./</div>
            </div>
            <div style={{ aspectRatio: 1, background: A.paper, border: `1px solid ${A.rule}`, padding: 8 }}>
              <DiffLine kind="-" >old</DiffLine>
              <DiffLine kind="+">new</DiffLine>
            </div>
          </div>
        </div>

      </div>
    </div>
  );
}

Object.assign(window, { A_palette: A, LogoA, WordmarkA, A1_Hero, A2_System, A3_Color, A4_Type, A5_Aux, A6_Print, A7_Digital, A8_Merch });
