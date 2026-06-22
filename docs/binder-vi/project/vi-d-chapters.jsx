/* vi-d-chapters.jsx — comprehensive chapters for the D-06 VI manual */

// Shorthand
const _D = D;     // palette
const Mk = (props) => <LogoD {...props}/>;   // primary mark shorthand
const MS = ({ id, title, sub }) => <MetaStrip id={id} title={title} sub={sub} accent={_D.accent}/>;

// Chapter divider — used for section openings
function ChapterDivider({ ch, total, title, en, body }) {
  return (
    <div style={{ width: '100%', height: '100%', background: _D.ink, color: _D.paper, position: 'relative' }}>
      <div style={{ position: 'absolute', inset: '40px 44px', display: 'flex', flexDirection: 'column', justifyContent: 'space-between' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 16 }}>
          <Mk size={48}/>
          <div className="mono" style={{ fontSize: 10, color: 'rgba(244,241,234,0.45)', letterSpacing: 1.6 }}>BINDER · CORE / VI · D-06</div>
        </div>
        <div>
          <div className="mono" style={{ fontSize: 10, color: _D.accent, letterSpacing: 2, marginBottom: 10 }}>{`CHAPTER ${ch} / ${total}`}</div>
          <div style={{ fontFamily: "'Instrument Serif', serif", fontSize: 84, lineHeight: 0.95, letterSpacing: -2 }}>{title}</div>
          <div className="mono" style={{ fontSize: 13, letterSpacing: 0.4, color: 'rgba(244,241,234,0.55)', marginTop: 6 }}>{en}</div>
          <div className="sc" style={{ fontSize: 13, color: 'rgba(244,241,234,0.7)', marginTop: 22, lineHeight: 1.7, maxWidth: 520 }}>{body}</div>
        </div>
        <div style={{ display: 'flex', justifyContent: 'space-between' }}>
          <Caption style={{ color: 'rgba(244,241,234,0.4)' }}>BINDER.DEV</Caption>
          <Caption style={{ color: 'rgba(244,241,234,0.4)' }}>{`§ ${ch}`}</Caption>
        </div>
      </div>
    </div>
  );
}

// ── 00 · Essence / manifesto
function D_Essence() {
  return (
    <div style={{ width: '100%', height: '100%', background: _D.paper, color: _D.ink }}>
      <MS id="VI-D · 00" title="品牌精神 · Brand Essence" sub="ONE-PAGER · 一句话定义"/>
      <div style={{ padding: '36px 48px', display: 'flex', flexDirection: 'column', gap: 24, height: 'calc(100% - 41px)', boxSizing: 'border-box' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 18 }}>
          <Mk size={84}/>
          <div style={{ width: 1, height: 70, background: _D.rule }}/>
          <div>
            <div style={{ fontFamily: "'Instrument Serif', serif", fontSize: 56, lineHeight: 0.92, letterSpacing: -1.4 }}>
              <span style={{ color: _D.accent }}>Diff</span> as the contract.
            </div>
            <div className="sc" style={{ fontSize: 14, color: _D.muted, marginTop: 6 }}>差异即契约。</div>
          </div>
        </div>

        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 14 }}>
          {[
            { en: 'LOCAL', cn: '本地优先', body: '一切数据在你机器上。云端只是工具，不是档案柜。' },
            { en: 'MINIMAL', cn: '最小核心', body: 'Workspace · Editor · Chat · Diff。不多一行。' },
            { en: 'REVIEWED', cn: '逐条审阅', body: 'AI 不写盘。每一处修改都先以 diff 呈现。' },
            { en: 'OPEN', cn: '开源可改', body: '可读、可分叉、可在此之上搭建你自己的工具。' },
          ].map((p) => (
            <div key={p.en} style={{ background: _D.card, border: `1px solid ${_D.rule}`, padding: '14px 16px' }}>
              <div className="mono" style={{ fontSize: 9, color: _D.accent, letterSpacing: 1.6 }}>{p.en}</div>
              <div className="sc" style={{ fontSize: 14, fontWeight: 500, marginTop: 6 }}>{p.cn}</div>
              <div className="sc" style={{ fontSize: 11, color: _D.muted, marginTop: 6, lineHeight: 1.65 }}>{p.body}</div>
            </div>
          ))}
        </div>

        <div style={{ display: 'grid', gridTemplateColumns: '1.4fr 1fr', gap: 24, marginTop: 'auto' }}>
          <div>
            <Caption style={{ marginBottom: 8 }}>① 一句话定义 · ONE-LINE</Caption>
            <div className="sc" style={{ fontSize: 15, lineHeight: 1.75, color: _D.ink }}>
              Binder Core 是一个<span style={{ color: _D.accent }}>本地优先</span>的 AI 文档编辑器最小核心，
              用户通过对话让 AI 修改文档，<span style={{ borderBottom: `1px dashed ${_D.muted}` }}>AI 修改以 diff 形式呈现</span>，
              用户逐条审阅后决定是否接受。
            </div>
          </div>
          <div>
            <Caption style={{ marginBottom: 8 }}>② 命名规约 · NAMING</Caption>
            <div className="mono" style={{ fontSize: 11, lineHeight: 1.7, color: _D.muted }}>
              <div><span style={{ color: _D.ink }}>Binder</span> · 产品族</div>
              <div><span style={{ color: _D.ink }}>Binder Core</span> · 开源版本</div>
              <div><span style={{ color: _D.ink }}>binder<span style={{ color: _D.accent }}>.</span></span> · 小写品牌</div>
              <div><span style={{ color: _D.ink }}>[B]</span> · 简称（聊天、代码）</div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

// ── Logo · Approved variants (richer than D4's lockup row)
function D_Approved() {
  const cells = [
    { bg: _D.paper, label: '正版 / LIGHT',  variant: 'accented', desc: '默认场景 · 暖纸底' },
    { bg: _D.ink,   label: '反白 / DARK',   variant: 'accented', accent: _D.paper, ink: _D.paper, desc: '黑底 · 强调点同样高亮' },
    { bg: _D.accent,label: '红底 / CLAY',   variant: 'accented', ink: _D.paper, accent: _D.paper, desc: '品牌色底 · 全部反白' },
    { bg: _D.paper, label: '单色 / MONO',   variant: 'default',  desc: '单色场景 · 不可用强调色时' },
    { bg: _D.paper, label: '反白 / NEG',    variant: 'negative', desc: '深色物料 · 一体填充' },
    { bg: _D.paper, label: '极小 / MIN',    variant: 'compact',  desc: '≤32px · 四段紧凑' },
  ];
  return (
    <div style={{ width: '100%', height: '100%', background: _D.paper, color: _D.ink }}>
      <MS id="VI-D · 03" title="标志正版 · Approved Variants"/>
      <div style={{ padding: '24px 32px' }}>
        <Caption style={{ marginBottom: 12 }}>① 6 种核准版本 · 任何应用场景都从此挑选</Caption>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 14 }}>
          {cells.map((c, i) => (
            <div key={i} style={{ border: `1px solid ${_D.rule}`, background: _D.card }}>
              <div style={{ background: c.bg, height: 130, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                <LogoD variant={c.variant} size={84} ink={c.ink} accent={c.accent}/>
              </div>
              <div style={{ padding: '10px 12px', borderTop: `1px solid ${_D.rule}` }}>
                <div className="mono" style={{ fontSize: 9, color: _D.accent, letterSpacing: 1.2 }}>{c.label}</div>
                <div className="sc" style={{ fontSize: 11, color: _D.muted, marginTop: 4 }}>{c.desc}</div>
              </div>
            </div>
          ))}
        </div>
        <div style={{ marginTop: 16, padding: '10px 14px', background: _D.card, border: `1px solid ${_D.rule}`, display: 'flex', alignItems: 'center', gap: 14 }}>
          <div className="mono" style={{ fontSize: 9, color: _D.faint, letterSpacing: 0.6 }}>RULE</div>
          <div style={{ flex: 1, height: 1, background: _D.rule }}/>
          <div className="sc" style={{ fontSize: 11, color: _D.muted }}>
            正版 D-06 用于<span style={{ color: _D.ink, fontWeight: 500 }}>所有主用场景</span>。
            其它版本仅用于其指定上下文，禁止混用或换色。
          </div>
        </div>
      </div>
    </div>
  );
}

// ── Logo · Misuse (forbidden applications)
function D_Misuse() {
  const items = [
    { id: 'stretch', label: '禁止变形', cap: '保持纵横比' },
    { id: 'rotate',  label: '禁止旋转', cap: '不可倾斜' },
    { id: 'recolor', label: '禁止换色', cap: '使用核准色版' },
    { id: 'shadow',  label: '禁止阴影', cap: '不加投影 / 描边' },
    { id: 'photo',   label: '禁止贴照片',cap: '不直接置于复杂图像' },
    { id: 'crowd',   label: '禁止挤占', cap: '保留 1 行距安全区' },
  ];
  return (
    <div style={{ width: '100%', height: '100%', background: _D.paper, color: _D.ink }}>
      <MS id="VI-D · 04" title="标志误用 · Misuse" sub="6 个绝对禁止"/>
      <div style={{ padding: '24px 32px' }}>
        <Caption style={{ marginBottom: 12 }}>① 六不准 · DO NOT</Caption>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 14 }}>
          {items.map((it) => (
            <div key={it.id} style={{ background: _D.card, border: `1px solid ${_D.rule}`, position: 'relative' }}>
              <div style={{ height: 130, display: 'flex', alignItems: 'center', justifyContent: 'center', background: _D.paper, position: 'relative', overflow: 'hidden' }}>
                {it.id === 'stretch'  && <div style={{ transform: 'scaleX(1.8)' }}><Mk size={70}/></div>}
                {it.id === 'rotate'   && <div style={{ transform: 'rotate(-18deg)' }}><Mk size={70}/></div>}
                {it.id === 'recolor'  && <LogoD variant="accented" size={70} ink="#6B8E5A" accent="#F4D03F"/>}
                {it.id === 'shadow'   && <div style={{ filter: 'drop-shadow(0 6px 8px rgba(0,0,0,0.4))' }}><Mk size={70}/></div>}
                {it.id === 'photo'    && (
                  <div style={{ width: '100%', height: '100%', position: 'absolute', inset: 0,
                    backgroundImage: 'repeating-linear-gradient(45deg, rgba(0,0,0,0.18) 0 6px, transparent 6px 12px), repeating-linear-gradient(-45deg, rgba(194,74,42,0.25) 0 8px, transparent 8px 16px)',
                    backgroundColor: '#9a8d75',
                    display: 'flex', alignItems: 'center', justifyContent: 'center',
                  }}><Mk size={70}/></div>
                )}
                {it.id === 'crowd' && (
                  <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
                    <div className="mono" style={{ fontSize: 22, color: _D.muted, fontWeight: 700 }}>X</div>
                    <Mk size={70}/>
                    <div className="mono" style={{ fontSize: 22, color: _D.muted, fontWeight: 700 }}>Y</div>
                  </div>
                )}
                {/* red NO mark */}
                <svg width="36" height="36" viewBox="0 0 36 36" style={{ position: 'absolute', top: 8, right: 8 }}>
                  <circle cx="18" cy="18" r="15" fill="none" stroke={_D.accent} strokeWidth="2.4"/>
                  <line x1="8" y1="8" x2="28" y2="28" stroke={_D.accent} strokeWidth="2.4"/>
                </svg>
              </div>
              <div style={{ padding: '10px 12px', borderTop: `1px solid ${_D.rule}` }}>
                <div className="sc" style={{ fontSize: 12, fontWeight: 500 }}>{it.label}</div>
                <div className="mono" style={{ fontSize: 9, color: _D.muted, marginTop: 3, letterSpacing: 0.4 }}>{it.cap}</div>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

// ── Color · Usage ratios
function D_ColorRatio() {
  return (
    <div style={{ width: '100%', height: '100%', background: _D.paper, color: _D.ink }}>
      <MS id="VI-D · 07" title="色彩比例 · Usage Ratios"/>
      <div style={{ padding: '24px 32px' }}>
        <Caption style={{ marginBottom: 12 }}>① 主用比例 · 60 / 35 / 5</Caption>
        <div style={{ display: 'flex', height: 100, marginBottom: 8 }}>
          <div style={{ flex: 60, background: _D.paper, border: `1px solid ${_D.rule}`, padding: 12, position: 'relative' }}>
            <div className="mono" style={{ fontSize: 11, color: _D.muted }}>60%</div>
            <div className="sc" style={{ fontSize: 12, marginTop: 2 }}>D-PAPER · 暖纸</div>
            <div className="mono" style={{ position: 'absolute', bottom: 10, left: 12, fontSize: 9, color: _D.faint }}>背景 · 留白</div>
          </div>
          <div style={{ flex: 35, background: _D.ink, color: _D.paper, padding: 12, position: 'relative' }}>
            <div className="mono" style={{ fontSize: 11, color: 'rgba(244,241,234,0.7)' }}>35%</div>
            <div className="sc" style={{ fontSize: 12, marginTop: 2 }}>D-INK · 编辑器墨</div>
            <div className="mono" style={{ position: 'absolute', bottom: 10, left: 12, fontSize: 9, color: 'rgba(244,241,234,0.5)' }}>文字 · 标志</div>
          </div>
          <div style={{ flex: 5, background: _D.accent, color: _D.paper, padding: 12, position: 'relative' }}>
            <div className="mono" style={{ fontSize: 11 }}>5%</div>
            <div className="sc" style={{ fontSize: 11, marginTop: 2 }}>CLAY</div>
            <div className="mono" style={{ position: 'absolute', bottom: 10, left: 12, fontSize: 9, opacity: 0.7 }}>点缀</div>
          </div>
        </div>

        <Caption style={{ marginTop: 18, marginBottom: 10 }}>② 反向版 · DARK MODE  ·  25 / 70 / 5</Caption>
        <div style={{ display: 'flex', height: 70, marginBottom: 8 }}>
          <div style={{ flex: 25, background: _D.paper, padding: 10 }}>
            <div className="mono" style={{ fontSize: 10, color: _D.muted }}>25% · PAPER</div>
          </div>
          <div style={{ flex: 70, background: _D.ink, color: _D.paper, padding: 10 }}>
            <div className="mono" style={{ fontSize: 10, color: 'rgba(244,241,234,0.7)' }}>70% · INK</div>
          </div>
          <div style={{ flex: 5, background: _D.accent, padding: 10 }}>
            <div className="mono" style={{ fontSize: 10, color: _D.paper }}>5%</div>
          </div>
        </div>

        <Caption style={{ marginTop: 18, marginBottom: 10 }}>③ 实例预览</Caption>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 14 }}>
          <div style={{ padding: 16, background: _D.paper, border: `1px solid ${_D.rule}`, minHeight: 120 }}>
            <Mk size={32}/>
            <div style={{ fontFamily: "'IBM Plex Sans', sans-serif", fontWeight: 500, fontSize: 18, marginTop: 14, letterSpacing: -0.4 }}>
              Edit one <span style={{ color: _D.accent }}>paragraph</span> at a time.
            </div>
            <div className="sc" style={{ fontSize: 11, color: _D.muted, marginTop: 6 }}>留白 + 一处点缀色。无任何额外颜色。</div>
          </div>
          <div style={{ padding: 16, background: _D.ink, color: _D.paper, minHeight: 120 }}>
            <LogoD size={32} ink={_D.paper}/>
            <div style={{ fontFamily: "'IBM Plex Sans', sans-serif", fontWeight: 500, fontSize: 18, marginTop: 14, letterSpacing: -0.4 }}>
              Edit one <span style={{ color: _D.accent }}>paragraph</span> at a time.
            </div>
            <div className="sc" style={{ fontSize: 11, color: 'rgba(244,241,234,0.55)', marginTop: 6 }}>深色场景同样保留点缀色。</div>
          </div>
        </div>
      </div>
    </div>
  );
}

// ── Voice & Tone
function D_Voice() {
  const samples = [
    { tag: 'DO', en: 'Diff as the contract.',         cn: '差异即契约。', kind: 'good' },
    { tag: 'DON\'T', en: 'AI-powered next-gen editor.',  cn: 'AI 驱动的下一代编辑器。', kind: 'bad' },
    { tag: 'DO', en: 'Edit one paragraph at a time.', cn: '一次只改一段。', kind: 'good' },
    { tag: 'DON\'T', en: 'Revolutionary writing experience.', cn: '革命性的写作体验。', kind: 'bad' },
    { tag: 'DO', en: 'Your files. Your machine.',     cn: '你的文件，你的机器。', kind: 'good' },
    { tag: 'DON\'T', en: 'Securely synced to the cloud.', cn: '安全云端同步。', kind: 'bad' },
  ];
  return (
    <div style={{ width: '100%', height: '100%', background: _D.paper, color: _D.ink }}>
      <MS id="VI-D · 11" title="语言 · Voice & Tone"/>
      <div style={{ padding: '24px 32px', display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 24 }}>
        <div>
          <Caption style={{ marginBottom: 10 }}>① 语气准则 · PRINCIPLES</Caption>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
            {[
              ['具体, 不空喊', 'Concrete, not slogans.', '"逐条审阅" — 不是 "无与伦比的体验"'],
              ['短句, 句号', 'Short. With periods.',     '一句一意。少用感叹号。'],
              ['动词在前', 'Verbs first.',               '"Edit · Review · Accept" — 不是 "Editing made easy"'],
              ['你, 不是您', 'Casual second person.',    '"你掌握文档" — 不是 "您将获得控制权"'],
              ['描述, 不承诺', 'Describe, don\'t promise.', '"diff 默认 pending" — 不是 "永不丢失修改"'],
            ].map(([cn, en, body], i) => (
              <div key={i} style={{ display: 'flex', gap: 10 }}>
                <div className="mono" style={{ fontSize: 10, color: _D.accent, width: 18 }}>{`0${i+1}`}</div>
                <div style={{ flex: 1 }}>
                  <div className="sc" style={{ fontSize: 13, fontWeight: 500 }}>{cn} <span className="mono" style={{ fontSize: 10, color: _D.muted, fontWeight: 400 }}>· {en}</span></div>
                  <div className="sc" style={{ fontSize: 11, color: _D.muted, marginTop: 2, lineHeight: 1.6 }}>{body}</div>
                </div>
              </div>
            ))}
          </div>
        </div>
        <div>
          <Caption style={{ marginBottom: 10 }}>② 实例 · DO / DON'T</Caption>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            {samples.map((s, i) => (
              <div key={i} style={{
                background: _D.card, border: `1px solid ${_D.rule}`,
                borderLeft: `3px solid ${s.kind === 'good' ? _D.green : _D.accent}`,
                padding: '10px 14px',
              }}>
                <div className="mono" style={{ fontSize: 9, color: s.kind === 'good' ? _D.green : _D.accent, letterSpacing: 1.2 }}>{s.tag}</div>
                <div style={{ fontFamily: "'IBM Plex Sans', sans-serif", fontWeight: 500, fontSize: 14, marginTop: 4, color: _D.ink, textDecoration: s.kind === 'bad' ? 'line-through' : 'none', textDecorationColor: 'rgba(0,0,0,0.3)' }}>
                  {s.en}
                </div>
                <div className="sc" style={{ fontSize: 11, color: _D.muted, marginTop: 2, textDecoration: s.kind === 'bad' ? 'line-through' : 'none', textDecorationColor: 'rgba(0,0,0,0.2)' }}>{s.cn}</div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

// ── Stationery: business card + letterhead + envelope
function D_Stationery() {
  return (
    <div style={{ width: '100%', height: '100%', background: _D.fog, color: _D.ink }}>
      <MS id="VI-D · 13" title="文具 · Stationery" sub="名片 · 信纸 · 信封"/>
      <div style={{ padding: '24px 28px', display: 'grid', gridTemplateColumns: '1fr 1.05fr 0.9fr', gap: 16 }}>
        {/* Business card */}
        <div>
          <Caption style={{ marginBottom: 10 }}>① 名片 · 85 × 55 MM</Caption>
          <div style={{ background: _D.paper, padding: 16, marginBottom: 10, boxShadow: '0 4px 14px rgba(0,0,0,0.08)', aspectRatio: '85/55', position: 'relative', display: 'flex', alignItems: 'flex-start', gap: 12 }}>
            <Mk size={50}/>
            <div style={{ flex: 1, display: 'flex', flexDirection: 'column', justifyContent: 'space-between', alignSelf: 'stretch' }}>
              <div>
                <div style={{ fontFamily: "'IBM Plex Sans', sans-serif", fontWeight: 500, fontSize: 14, letterSpacing: -0.3 }}>Lin Yiran</div>
                <div className="sc" style={{ fontSize: 10, color: _D.muted, marginTop: 1 }}>林一然 · 核心维护者</div>
              </div>
              <div className="mono" style={{ fontSize: 8, color: _D.faint, lineHeight: 1.5 }}>
                lin@binder.dev<br/>binder.dev
              </div>
            </div>
          </div>
          <div style={{ background: _D.ink, padding: 14, aspectRatio: '85/55', boxShadow: '0 4px 14px rgba(0,0,0,0.08)', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
            <LogoD variant="negative" size={80}/>
          </div>
        </div>

        {/* Letterhead */}
        <div>
          <Caption style={{ marginBottom: 10 }}>② 信纸 · A4</Caption>
          <div style={{ background: _D.paper, aspectRatio: '210/297', boxShadow: '0 4px 14px rgba(0,0,0,0.08)', padding: 18, position: 'relative', overflow: 'hidden' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
              <Mk size={28}/>
              <div className="mono" style={{ fontSize: 7, color: _D.faint, textAlign: 'right', letterSpacing: 0.6, lineHeight: 1.5 }}>
                BINDER.DEV<br/>OPEN SOURCE / CORE
              </div>
            </div>
            <div className="mono" style={{ marginTop: 24, fontSize: 7, color: _D.muted, letterSpacing: 0.8 }}>2026 / 05 / 15</div>
            <div className="sc" style={{ fontSize: 8, color: _D.ink, lineHeight: 1.7, marginTop: 16 }}>
              亲爱的贡献者，<br/><br/>
              感谢你为 Binder Core 提交的 <span className="mono" style={{ color: _D.accent }}>#247</span> 号 PR。
              本地优先这件事，离不开每一个仔细审阅 diff 的人。<br/><br/>
              附上一张贡献者凭证 ——
            </div>
            <div style={{ position: 'absolute', bottom: 18, left: 18, right: 18, display: 'flex', justifyContent: 'space-between' }}>
              <div className="mono" style={{ fontSize: 7, color: _D.faint }}>// signed off</div>
              <div className="mono" style={{ fontSize: 7, color: _D.faint }}>· 1 / 1 ·</div>
            </div>
          </div>
        </div>

        {/* Envelope */}
        <div>
          <Caption style={{ marginBottom: 10 }}>③ 信封 · DL</Caption>
          <div style={{ background: _D.paper, aspectRatio: '220/110', boxShadow: '0 4px 14px rgba(0,0,0,0.08)', padding: 12, position: 'relative', display: 'flex', flexDirection: 'column', justifyContent: 'space-between' }}>
            <div style={{ display: 'flex', alignItems: 'flex-start', gap: 8 }}>
              <Mk size={20}/>
              <div>
                <div className="mono" style={{ fontSize: 7, fontWeight: 600 }}>BINDER · CORE</div>
                <div className="mono" style={{ fontSize: 6, color: _D.muted, marginTop: 2 }}>binder.dev</div>
              </div>
            </div>
            <div className="mono" style={{ fontSize: 7, color: _D.faint, alignSelf: 'flex-end', textAlign: 'right', lineHeight: 1.4 }}>
              · 贴邮处 ·<br/>STAMP
            </div>
          </div>
          <Caption style={{ marginTop: 12, marginBottom: 8 }}>④ 公章 · STAMP</Caption>
          <div style={{ background: _D.paper, padding: 16, boxShadow: '0 4px 14px rgba(0,0,0,0.08)', display: 'flex', alignItems: 'center', justifyContent: 'center', height: 110 }}>
            <div style={{ width: 90, height: 90, border: `2px solid ${_D.accent}`, borderRadius: '50%', display: 'flex', alignItems: 'center', justifyContent: 'center', flexDirection: 'column', color: _D.accent, transform: 'rotate(-6deg)' }}>
              <LogoD variant="compact" size={28} ink={_D.accent} accent={_D.accent}/>
              <div className="mono" style={{ fontSize: 8, marginTop: 2, letterSpacing: 1, fontWeight: 600 }}>SIGNED</div>
              <div className="mono" style={{ fontSize: 6, opacity: 0.7, letterSpacing: 1 }}>BINDER.DEV</div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

// ── Poster (refined)
function D_Poster() {
  return (
    <div style={{ width: '100%', height: '100%', background: _D.fog, color: _D.ink }}>
      <MS id="VI-D · 14" title="海报 · Poster" sub="A2 系列 · 三张同源"/>
      <div style={{ padding: '24px 32px', display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 14 }}>

        {/* Poster 1: hero clay */}
        <div style={{ background: _D.paper, aspectRatio: '420/594', padding: '20px 18px', boxShadow: '0 8px 20px rgba(0,0,0,0.10)', position: 'relative', overflow: 'hidden' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between' }}>
            <Mk size={40}/>
            <Caption>01 / 03</Caption>
          </div>
          <div style={{ marginTop: 36, fontFamily: "'IBM Plex Sans', sans-serif", fontWeight: 500, fontSize: 32, letterSpacing: -1.2, lineHeight: 0.94 }}>
            Edit<br/>one<br/><span style={{ color: _D.accent }}>paragraph</span><br/>at a time.
          </div>
          <div className="sc" style={{ position: 'absolute', bottom: 58, left: 18, right: 18, fontSize: 9, lineHeight: 1.6, color: _D.muted }}>
            AI 修改先以差异呈现，<br/>由你逐条审阅。文档掌握在你手里。
          </div>
          <div style={{ position: 'absolute', bottom: 18, left: 18, right: 18, display: 'flex', justifyContent: 'space-between' }}>
            <Caption>BINDER.DEV</Caption>
            <Caption>OPEN SOURCE</Caption>
          </div>
        </div>

        {/* Poster 2: dark variant */}
        <div style={{ background: _D.ink, color: _D.paper, aspectRatio: '420/594', padding: '20px 18px', boxShadow: '0 8px 20px rgba(0,0,0,0.20)', position: 'relative', overflow: 'hidden' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between' }}>
            <LogoD size={40} ink={_D.paper}/>
            <Caption style={{ color: 'rgba(244,241,234,0.5)' }}>02 / 03</Caption>
          </div>
          <div style={{ marginTop: 36, fontFamily: "'Instrument Serif', serif", fontSize: 56, letterSpacing: -1.6, lineHeight: 0.92 }}>
            <span style={{ color: _D.accent }}>Diff</span><br/>as the<br/>contract.
          </div>
          <div className="sc" style={{ position: 'absolute', bottom: 58, left: 18, right: 18, fontSize: 9, lineHeight: 1.6, color: 'rgba(244,241,234,0.55)' }}>
            差异即契约 —— AI 不写盘，<br/>所有改动等你点头。
          </div>
          <div style={{ position: 'absolute', bottom: 18, left: 18, right: 18, display: 'flex', justifyContent: 'space-between' }}>
            <Caption style={{ color: 'rgba(244,241,234,0.4)' }}>BINDER.DEV</Caption>
            <Caption style={{ color: 'rgba(244,241,234,0.4)' }}>V1.0 · 2026</Caption>
          </div>
        </div>

        {/* Poster 3: claim grid */}
        <div style={{ background: _D.paper, aspectRatio: '420/594', padding: 18, boxShadow: '0 8px 20px rgba(0,0,0,0.10)', position: 'relative', overflow: 'hidden' }}>
          <Mk size={40}/>
          <div style={{ marginTop: 18, display: 'flex', flexDirection: 'column', gap: 5 }}>
            {Array.from({ length: 22 }).map((_, i) => {
              const w = 25 + ((i * 37) % 75);
              const isHero = i === 8;
              return (
                <div key={i} style={{ height: 4, width: `${w}%`, background: isHero ? _D.accent : _D.ink, opacity: isHero ? 1 : 0.2 }}/>
              );
            })}
          </div>
          <div className="mono" style={{ position: 'absolute', bottom: 60, left: 18, right: 18, fontSize: 9, lineHeight: 1.7, color: _D.muted }}>
            ¶ 9 / 22 修改<br/>
            <span style={{ color: _D.accent }}>+ added: "by you"</span>
          </div>
          <div style={{ position: 'absolute', bottom: 18, left: 18, right: 18, display: 'flex', justifyContent: 'space-between' }}>
            <Caption>BINDER.DEV</Caption>
            <Caption>03 / 03</Caption>
          </div>
        </div>
      </div>
    </div>
  );
}

// ── Slide template
function Slide({ children, dark, style }) {
  return (
    <div style={{
      background: dark ? _D.ink : _D.paper,
      color: dark ? _D.paper : _D.ink,
      aspectRatio: '16/9', position: 'relative',
      boxShadow: '0 4px 14px rgba(0,0,0,0.08)',
      overflow: 'hidden',
      padding: 16, ...style,
    }}>{children}</div>
  );
}

function D_Slides() {
  return (
    <div style={{ width: '100%', height: '100%', background: _D.fog, color: _D.ink }}>
      <MS id="VI-D · 15" title="幻灯片模板 · Slides" sub="3 种布局 · 16:9"/>
      <div style={{ padding: '24px 28px', display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 14 }}>

        {/* Title slide */}
        <Slide dark>
          <div style={{ position: 'absolute', inset: 22 }}>
            <LogoD size={32} ink={_D.paper}/>
            <div style={{ marginTop: 'auto', position: 'absolute', bottom: 0, left: 0, right: 0 }}>
              <div style={{ fontFamily: "'Instrument Serif', serif", fontSize: 42, lineHeight: 0.95, letterSpacing: -1 }}>
                Binder Core<br/><span style={{ color: 'rgba(244,241,234,0.55)' }}>v1.0 release</span>
              </div>
              <div className="mono" style={{ fontSize: 9, marginTop: 10, color: 'rgba(244,241,234,0.5)', letterSpacing: 1.4 }}>2026 · 05 · 15 · LIN YIRAN</div>
            </div>
          </div>
        </Slide>

        {/* Section slide */}
        <Slide>
          <div style={{ display: 'flex', height: '100%', alignItems: 'center', gap: 20 }}>
            <Mk size={80}/>
            <div style={{ flex: 1 }}>
              <div className="mono" style={{ fontSize: 9, color: _D.accent, letterSpacing: 1.6 }}>§ 03</div>
              <div style={{ fontFamily: "'IBM Plex Sans', sans-serif", fontWeight: 500, fontSize: 30, letterSpacing: -0.6, marginTop: 4, lineHeight: 1 }}>
                Diff 审阅机制
              </div>
              <div className="sc" style={{ fontSize: 11, color: _D.muted, marginTop: 4 }}>How AI edits land in your files.</div>
            </div>
          </div>
        </Slide>

        {/* Content slide */}
        <Slide>
          <div className="mono" style={{ fontSize: 8, color: _D.accent, letterSpacing: 1.6 }}>§ 03 · MECHANISM</div>
          <div style={{ fontFamily: "'IBM Plex Sans', sans-serif", fontWeight: 500, fontSize: 20, marginTop: 4, letterSpacing: -0.3 }}>
            AI 不写盘 —— 所有修改先 pending
          </div>
          <div style={{ marginTop: 12, display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 10 }}>
            {['对话发起', 'Diff 生成', '逐条审阅'].map((s, i) => (
              <div key={s} style={{ background: '#fff', border: `1px solid ${_D.rule}`, padding: 8, fontSize: 9 }}>
                <div className="mono" style={{ color: i === 1 ? _D.accent : _D.muted, fontSize: 8 }}>{`STEP ${i+1}`}</div>
                <div className="sc" style={{ marginTop: 4, fontWeight: 500 }}>{s}</div>
              </div>
            ))}
          </div>
          <div style={{ marginTop: 'auto', position: 'absolute', bottom: 16, left: 16, right: 16, display: 'flex', justifyContent: 'space-between' }}>
            <div className="mono" style={{ fontSize: 8, color: _D.faint }}>BINDER.DEV</div>
            <div className="mono" style={{ fontSize: 8, color: _D.faint }}>04 / 18</div>
          </div>
        </Slide>

        {/* Quote slide */}
        <Slide>
          <div style={{ position: 'absolute', inset: 22 }}>
            <div style={{ fontFamily: "'Instrument Serif', serif", fontSize: 28, lineHeight: 1.25, letterSpacing: -0.6, maxWidth: '85%' }}>
              "<span style={{ color: _D.accent }}>Diff</span> 是用户掌握文档的最后一道闸门。"
            </div>
            <div style={{ position: 'absolute', bottom: 0, left: 0, right: 0, display: 'flex', alignItems: 'center', gap: 10 }}>
              <Mk size={20}/>
              <div className="mono" style={{ fontSize: 8, color: _D.muted }}>BINDER CORE / DESIGN PRINCIPLE 01</div>
            </div>
          </div>
        </Slide>
      </div>
    </div>
  );
}

// ── Web landing
function D_Web() {
  return (
    <div style={{ width: '100%', height: '100%', background: _D.fog, color: _D.ink }}>
      <MS id="VI-D · 16" title="网站 · Landing Page" sub="binder.dev"/>
      <div style={{ padding: '20px 28px' }}>
        <AppWindow style={{ height: 360 }} title="binder.dev">
          <div style={{ background: _D.paper, color: _D.ink, height: 'calc(100% - 33px)', overflow: 'hidden' }}>
            {/* nav */}
            <div style={{ display: 'flex', alignItems: 'center', padding: '12px 24px', borderBottom: `1px solid ${_D.rule}` }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <Mk size={20}/>
                <div style={{ fontFamily: "'IBM Plex Sans', sans-serif", fontWeight: 500, fontSize: 14 }}>binder<span style={{ color: _D.accent }}>.</span></div>
              </div>
              <div style={{ display: 'flex', gap: 18, marginLeft: 30 }} className="mono">
                {['docs', 'github', 'spec'].map((l) => <div key={l} style={{ fontSize: 10, color: _D.muted, cursor: 'pointer' }}>{l}</div>)}
              </div>
              <div style={{ marginLeft: 'auto', display: 'flex', gap: 10 }}>
                <div className="mono" style={{ fontSize: 10, color: _D.muted, padding: '4px 8px' }}>v1.0</div>
                <div className="mono" style={{ fontSize: 10, color: _D.paper, background: _D.ink, padding: '4px 10px', borderRadius: 1 }}>Download ↓</div>
              </div>
            </div>
            {/* hero */}
            <div style={{ padding: '32px 30px', display: 'grid', gridTemplateColumns: '1.3fr 1fr', gap: 28 }}>
              <div>
                <div className="mono" style={{ fontSize: 10, color: _D.accent, letterSpacing: 1.6 }}>v1.0 · OPEN SOURCE</div>
                <div style={{ fontFamily: "'Instrument Serif', serif", fontSize: 48, letterSpacing: -1.6, lineHeight: 0.95, marginTop: 10 }}>
                  Edit one<br/><span style={{ color: _D.accent }}>paragraph</span><br/>at a time.
                </div>
                <div className="sc" style={{ fontSize: 13, color: _D.muted, marginTop: 16, lineHeight: 1.7, maxWidth: 360 }}>
                  本地优先的 AI 文档编辑器最小核心。AI 改动以 diff 形式呈现，你逐条审阅 ——
                  <span style={{ color: _D.ink, fontWeight: 500 }}>文档始终在你手里。</span>
                </div>
                <div style={{ display: 'flex', gap: 8, marginTop: 18 }}>
                  <div className="mono" style={{ fontSize: 11, padding: '8px 14px', background: _D.ink, color: _D.paper }}>↓ macOS</div>
                  <div className="mono" style={{ fontSize: 11, padding: '8px 14px', background: 'transparent', color: _D.ink, border: `1px solid ${_D.rule}` }}>$ git clone</div>
                </div>
              </div>
              <div style={{ background: _D.ink, padding: 12, fontFamily: 'IBM Plex Mono, monospace', fontSize: 10, color: _D.paper, lineHeight: 1.7 }}>
                <div style={{ color: _D.accent }}>$ binder ./docs</div>
                <div style={{ opacity: 0.55 }}>→ workspace: ~/docs · 14 files</div>
                <div style={{ opacity: 0.55 }}>→ provider: anthropic · agent mode</div>
                <div style={{ marginTop: 8 }}>1 pending diff · spec.md ¶3</div>
                <div style={{ marginTop: 6, padding: '3px 0', background: 'rgba(176,70,55,0.16)', color: '#e89a86' }}>
                  − general-purpose AI editor
                </div>
                <div style={{ padding: '3px 0', background: 'rgba(46,111,74,0.18)', color: '#7fc99e' }}>
                  + local-first AI editor.
                </div>
                <div style={{ marginTop: 8, color: _D.accent }}>[a]ccept · [r]eject<span style={{ background: _D.paper, color: _D.ink, padding: '0 2px', marginLeft: 4 }}>_</span></div>
              </div>
            </div>
          </div>
        </AppWindow>
      </div>
    </div>
  );
}

// ── Social media kit
function D_Social() {
  return (
    <div style={{ width: '100%', height: '100%', background: _D.fog, color: _D.ink }}>
      <MS id="VI-D · 17" title="社交媒体 · Social Kit" sub="AVATAR · OG · GITHUB README"/>
      <div style={{ padding: '24px 32px', display: 'grid', gridTemplateColumns: '0.5fr 1fr 1fr', gap: 14 }}>

        {/* Avatar set */}
        <div>
          <Caption style={{ marginBottom: 10 }}>① 头像 · AVATAR</Caption>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
            <div style={{ width: '100%', aspectRatio: 1, background: _D.paper, borderRadius: '50%', display: 'flex', alignItems: 'center', justifyContent: 'center', boxShadow: '0 4px 14px rgba(0,0,0,0.10)' }}>
              <Mk size={70}/>
            </div>
            <div style={{ width: '100%', aspectRatio: 1, background: _D.ink, borderRadius: '50%', display: 'flex', alignItems: 'center', justifyContent: 'center', boxShadow: '0 4px 14px rgba(0,0,0,0.18)' }}>
              <LogoD size={70} ink={_D.paper}/>
            </div>
          </div>
          <Caption style={{ marginTop: 10 }}>圆形与方形通用</Caption>
        </div>

        {/* OG Image */}
        <div>
          <Caption style={{ marginBottom: 10 }}>② OG 图 · 1200 × 630</Caption>
          <div style={{ background: _D.ink, color: _D.paper, aspectRatio: '12/6.3', position: 'relative', overflow: 'hidden', padding: 22 }}>
            <LogoD size={36} ink={_D.paper}/>
            <div style={{ fontFamily: "'Instrument Serif', serif", fontSize: 34, letterSpacing: -1, lineHeight: 0.95, marginTop: 20 }}>
              <span style={{ color: _D.accent }}>Diff</span> as the contract.
            </div>
            <div className="sc" style={{ fontSize: 11, color: 'rgba(244,241,234,0.6)', marginTop: 8 }}>本地优先的 AI 文档编辑器最小核心 · binder.dev</div>
            <div style={{ position: 'absolute', right: 0, top: 0, bottom: 0, width: '38%' }}>
              {Array.from({ length: 18 }).map((_, i) => {
                const w = 30 + ((i * 41) % 60);
                const accent = i === 6;
                return <div key={i} style={{ position: 'absolute', right: 0, top: 16 + i * 14, width: `${w}%`, height: 4, background: accent ? _D.accent : _D.paper, opacity: accent ? 1 : 0.15 }}/>;
              })}
            </div>
          </div>
          <Caption style={{ marginTop: 8 }}>③ Twitter / X 横幅 · 1500 × 500</Caption>
          <div style={{ background: _D.paper, aspectRatio: '15/5', position: 'relative', padding: 16, marginTop: 6, boxShadow: '0 2px 8px rgba(0,0,0,0.08)' }}>
            <Mk size={42}/>
            <div style={{ fontFamily: "'IBM Plex Sans', sans-serif", fontWeight: 500, fontSize: 17, letterSpacing: -0.4, marginTop: 8 }}>
              Edit one <span style={{ color: _D.accent }}>paragraph</span> at a time.
            </div>
            <div className="mono" style={{ position: 'absolute', bottom: 8, right: 14, fontSize: 8, color: _D.muted }}>binder.dev</div>
          </div>
        </div>

        {/* GitHub README banner */}
        <div>
          <Caption style={{ marginBottom: 10 }}>④ GITHUB README · BANNER</Caption>
          <div style={{ background: _D.paper, aspectRatio: '3/1.4', padding: 16, position: 'relative', boxShadow: '0 2px 8px rgba(0,0,0,0.08)', overflow: 'hidden' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
              <Mk size={48}/>
              <div>
                <div className="mono" style={{ fontSize: 15, fontWeight: 600 }}>binder/core</div>
                <div className="mono" style={{ fontSize: 9, color: _D.muted, marginTop: 2 }}>v1.0 · MIT · ⭐ 2.4k · TS</div>
              </div>
            </div>
            <div className="sc" style={{ fontSize: 11, color: _D.muted, marginTop: 12, lineHeight: 1.6 }}>
              A local-first AI document editor. Diff as the contract.
            </div>
          </div>

          <Caption style={{ marginTop: 12, marginBottom: 10 }}>⑤ Discord / Slack 表情</Caption>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 6 }}>
            {[
              { ink: _D.ink, accent: _D.accent, bg: _D.paper, label: ':binder:' },
              { ink: _D.paper, accent: _D.paper, bg: _D.ink, label: ':binder-dark:' },
              { ink: _D.paper, accent: _D.paper, bg: _D.accent, label: ':binder-clay:' },
              { ink: _D.accent, accent: _D.accent, bg: _D.paper, label: ':binder-pending:' },
            ].map((s, i) => (
              <div key={i} style={{ aspectRatio: 1, background: s.bg, display: 'flex', alignItems: 'center', justifyContent: 'center', borderRadius: 6, padding: 6 }}>
                <LogoD size={36} ink={s.ink} accent={s.accent}/>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

// ── Product UI · Dark
function D_DarkUI() {
  return (
    <div style={{ width: '100%', height: '100%', background: _D.fog, color: _D.ink }}>
      <MS id="VI-D · 19" title="产品界面 · 暗色模式" sub="DARK MODE"/>
      <div style={{ padding: '24px 32px' }}>
        <AppWindow dark style={{ height: 340 }} title="binder · ~/docs/spec.md">
          <div style={{ display: 'flex', height: 'calc(100% - 33px)', background: _D.ink }}>
            {/* file tree */}
            <div style={{ width: 130, borderRight: '1px solid rgba(255,255,255,0.06)', padding: 12, fontSize: 11 }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 14 }}>
                <LogoD size={18} ink={_D.paper}/>
                <div className="mono" style={{ fontSize: 11, fontWeight: 600, color: _D.paper }}>binder<span style={{ color: _D.accent }}>.</span></div>
              </div>
              <div className="mono" style={{ color: 'rgba(244,241,234,0.4)', fontSize: 9, letterSpacing: 0.6, marginBottom: 8 }}>WORKSPACE</div>
              {[
                { n: '▾ docs', muted: false },
                { n: '  intro.md' },
                { n: '  spec.md', active: true },
                { n: '  notes.txt' },
                { n: '▸ .binder', dim: true },
              ].map((f, i) => (
                <div key={i} className="mono" style={{
                  padding: '4px 8px', fontSize: 11,
                  color: f.active ? _D.paper : f.dim ? 'rgba(244,241,234,0.3)' : 'rgba(244,241,234,0.6)',
                  fontWeight: f.active ? 500 : 400,
                  background: f.active ? 'rgba(194,74,42,0.18)' : 'transparent',
                  borderLeft: f.active ? `2px solid ${_D.accent}` : '2px solid transparent',
                }}>{f.n}</div>
              ))}
            </div>
            {/* editor */}
            <div style={{ flex: 1, padding: '18px 22px', borderRight: '1px solid rgba(255,255,255,0.06)', color: _D.paper }}>
              <div style={{ fontFamily: "'IBM Plex Sans', sans-serif", fontWeight: 600, fontSize: 17 }}>spec.md</div>
              <div className="sc" style={{ fontSize: 12, color: 'rgba(244,241,234,0.65)', marginTop: 10, lineHeight: 1.65 }}>
                Binder Core 是一个本地优先的 AI 文档编辑器。
              </div>
              <div style={{ marginTop: 10 }}>
                <div style={{ display: 'flex' }}>
                  <div style={{ width: 3, background: '#b44637' }}/>
                  <div className="mono" style={{ width: 18, textAlign: 'center', color: '#e89a86', fontSize: 11, padding: '3px 0' }}>−</div>
                  <div className="mono" style={{ fontSize: 11, color: '#e89a86', padding: '3px 6px', background: 'rgba(176,70,55,0.16)', flex: 1 }}>所有 diff 自动接受。</div>
                </div>
                <div style={{ display: 'flex' }}>
                  <div style={{ width: 3, background: _D.green }}/>
                  <div className="mono" style={{ width: 18, textAlign: 'center', color: '#7fc99e', fontSize: 11, padding: '3px 0' }}>+</div>
                  <div className="mono" style={{ fontSize: 11, color: '#7fc99e', padding: '3px 6px', background: 'rgba(46,111,74,0.18)', flex: 1 }}>所有 diff 默认 pending，由用户审阅。</div>
                </div>
              </div>
              <div className="sc" style={{ fontSize: 12, color: 'rgba(244,241,234,0.65)', marginTop: 10, lineHeight: 1.65 }}>
                workspace.db 持久化全部状态。
              </div>
            </div>
            {/* chat */}
            <div style={{ width: 160, padding: 12, fontSize: 11, background: '#1d1b18' }}>
              <div className="mono" style={{ color: 'rgba(244,241,234,0.4)', fontSize: 9, letterSpacing: 0.6, marginBottom: 8 }}>CHAT · AGENT</div>
              <div style={{ marginBottom: 8, padding: 8, background: 'rgba(255,255,255,0.04)', borderRadius: 2, fontSize: 10, color: _D.paper }}>
                <div className="mono" style={{ fontSize: 8, color: 'rgba(244,241,234,0.4)' }}>YOU</div>
                <div className="sc" style={{ marginTop: 3 }}>把"自动接受"改成审阅</div>
              </div>
              <div style={{ padding: 8, background: 'rgba(255,255,255,0.04)', border: `1px solid ${_D.accent}`, borderRadius: 2, fontSize: 10, color: _D.paper }}>
                <div className="mono" style={{ fontSize: 8, color: _D.accent }}>BINDER · 1 DIFF</div>
                <div className="mono" style={{ fontSize: 8, color: 'rgba(244,241,234,0.5)', marginTop: 3 }}>spec.md · ¶3</div>
                <div style={{ display: 'flex', gap: 4, marginTop: 8 }}>
                  <div className="mono" style={{ flex: 1, padding: '4px 0', background: _D.green, color: '#fff', textAlign: 'center', fontSize: 8 }}>ACCEPT</div>
                  <div className="mono" style={{ flex: 1, padding: '4px 0', color: 'rgba(244,241,234,0.5)', textAlign: 'center', fontSize: 8, border: '1px solid rgba(255,255,255,0.1)' }}>REJECT</div>
                </div>
              </div>
            </div>
          </div>
        </AppWindow>
      </div>
    </div>
  );
}

// ── App icon grid (full set)
function D_AppIcons() {
  const sizes = [
    { s: 120, name: 'MAIN · 1024',    bg: _D.paper, ink: _D.ink,    accent: _D.accent, r: 26 },
    { s: 120, name: 'DARK · 1024',    bg: _D.ink,   ink: _D.paper,  accent: _D.accent, r: 26 },
    { s: 120, name: 'CLAY · INSIDER', bg: _D.accent,ink: _D.paper,  accent: _D.paper,  r: 26 },
    { s: 80,  name: '512',            bg: _D.paper, ink: _D.ink,    accent: _D.accent, r: 18 },
    { s: 56,  name: '256',            bg: _D.ink,   ink: _D.paper,  accent: _D.accent, r: 12 },
    { s: 40,  name: '128',            bg: _D.paper, ink: _D.ink,    accent: _D.accent, r: 8 },
    { s: 28,  name: '64',             bg: _D.ink,   ink: _D.paper,  accent: _D.accent, r: 6 },
    { s: 16,  name: '32 · FAVICON',   bg: _D.paper, ink: _D.ink,    accent: _D.accent, r: 4 },
  ];
  return (
    <div style={{ width: '100%', height: '100%', background: _D.paper, color: _D.ink }}>
      <MS id="VI-D · 18" title="应用图标 · App Icon Set"/>
      <div style={{ padding: '24px 32px' }}>
        <Caption style={{ marginBottom: 12 }}>① 三种风格 · 各尺寸下都保留视觉重心</Caption>
        <div style={{ display: 'flex', alignItems: 'flex-end', gap: 24, flexWrap: 'wrap' }}>
          {sizes.map((sz, i) => (
            <div key={i} style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 8 }}>
              <div style={{ width: sz.s, height: sz.s, background: sz.bg, borderRadius: sz.r, display: 'flex', alignItems: 'center', justifyContent: 'center', boxShadow: '0 2px 8px rgba(0,0,0,0.10)' }}>
                <LogoD variant={sz.s <= 28 ? 'compact' : 'accented'} size={sz.s * 0.72} ink={sz.ink} accent={sz.accent}/>
              </div>
              <Caption>{sz.name}</Caption>
            </div>
          ))}
        </div>
        <div style={{ marginTop: 18, padding: '10px 14px', background: _D.card, border: `1px solid ${_D.rule}`, display: 'flex', alignItems: 'center', gap: 14 }}>
          <div className="mono" style={{ fontSize: 9, color: _D.faint, letterSpacing: 0.6 }}>RULE</div>
          <div style={{ flex: 1, height: 1, background: _D.rule }}/>
          <div className="sc" style={{ fontSize: 11, color: _D.muted }}>
            ≤ 28PX 切换 <span style={{ color: _D.ink, fontWeight: 500 }}>D-08 紧凑四段</span> · favicon 单色不带强调点
          </div>
        </div>
      </div>
    </div>
  );
}

// ── Contributor pass + packaging
function D_ContribPass() {
  return (
    <div style={{ width: '100%', height: '100%', background: _D.fog, color: _D.ink }}>
      <MS id="VI-D · 22" title="贡献者凭证 · Contributor Pass"/>
      <div style={{ padding: '24px 32px', display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 16 }}>

        {/* Pass — front */}
        <div>
          <Caption style={{ marginBottom: 10 }}>① 凭证 · 正面</Caption>
          <div style={{ aspectRatio: '85/120', background: _D.ink, color: _D.paper, padding: 16, boxShadow: '0 6px 18px rgba(0,0,0,0.20)', display: 'flex', flexDirection: 'column', justifyContent: 'space-between', position: 'relative' }}>
            <div>
              <LogoD variant="negative" size={48}/>
              <div className="mono" style={{ fontSize: 8, color: 'rgba(244,241,234,0.5)', marginTop: 12, letterSpacing: 0.8 }}>CORE · CONTRIBUTOR / 2026</div>
            </div>
            <PhotoSlot style={{ aspectRatio: 1 }} label="PORTRAIT"/>
            <div>
              <div style={{ fontFamily: "'IBM Plex Sans', sans-serif", fontWeight: 500, fontSize: 18, letterSpacing: -0.5 }}>Lin Yiran</div>
              <div className="sc" style={{ fontSize: 10, color: 'rgba(244,241,234,0.5)', marginTop: 2 }}>林一然 · 核心维护者</div>
            </div>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-end' }}>
              <div className="mono" style={{ fontSize: 8, color: _D.accent, letterSpacing: 1.2 }}>NO. 00017</div>
              <div className="mono" style={{ fontSize: 8, color: 'rgba(244,241,234,0.4)' }}>BINDER.DEV</div>
            </div>
          </div>
        </div>

        {/* Pass — back */}
        <div>
          <Caption style={{ marginBottom: 10 }}>② 凭证 · 背面 · 贡献明细</Caption>
          <div style={{ aspectRatio: '85/120', background: _D.paper, padding: 16, boxShadow: '0 6px 18px rgba(0,0,0,0.10)', position: 'relative', display: 'flex', flexDirection: 'column', gap: 8 }}>
            <div className="mono" style={{ fontSize: 8, color: _D.muted, letterSpacing: 0.8 }}>// CONTRIBUTIONS</div>
            {[
              ['#247', 'diff: paragraph-level review'],
              ['#251', 'workspace.db: schema v2'],
              ['#258', 'fix: macOS keychain'],
              ['#264', 'docs: agent tools'],
            ].map(([n, t], i) => (
              <div key={i} className="mono" style={{ fontSize: 9, color: _D.ink, display: 'flex', gap: 8 }}>
                <span style={{ color: _D.accent }}>{n}</span>
                <span style={{ color: _D.muted }}>{t}</span>
              </div>
            ))}
            <div style={{ marginTop: 'auto', borderTop: `1px solid ${_D.rule}`, paddingTop: 10 }}>
              <LogoD variant="compact" size={22}/>
              <div className="mono" style={{ fontSize: 7, color: _D.muted, marginTop: 6, letterSpacing: 0.4 }}>
                Issued 2026-05-15 · Signed by maintainers council<br/>
                <span style={{ color: _D.accent }}>git verified ✓</span>
              </div>
            </div>
          </div>
        </div>

        {/* Packaging */}
        <div>
          <Caption style={{ marginBottom: 10 }}>③ 启动套装 · STARTER KIT</Caption>
          <div style={{ aspectRatio: '85/120', background: _D.paper, boxShadow: '0 6px 18px rgba(0,0,0,0.10)', position: 'relative', overflow: 'hidden' }}>
            <div style={{ position: 'absolute', top: 0, left: 0, right: 0, height: 28, background: _D.ink, color: _D.paper, display: 'flex', alignItems: 'center', padding: '0 12px' }}>
              <div className="mono" style={{ fontSize: 9, letterSpacing: 2 }}>BINDER · STARTER KIT</div>
            </div>
            <div style={{ padding: 16, marginTop: 28 }}>
              <Mk size={70}/>
              <div style={{ fontFamily: "'Instrument Serif', serif", fontSize: 26, lineHeight: 1, marginTop: 14, letterSpacing: -0.6 }}>
                Binder<br/>Core
              </div>
              <div className="sc" style={{ fontSize: 10, color: _D.muted, marginTop: 8 }}>
                · 1 × USB (installer)<br/>· 1 × 笔记本<br/>· 1 × 贡献者凭证<br/>· 1 × 贴纸套装
              </div>
            </div>
            <div className="mono" style={{ position: 'absolute', bottom: 12, left: 14, fontSize: 7, color: _D.faint }}>v1.0 · 2026</div>
            <div className="mono" style={{ position: 'absolute', bottom: 12, right: 14, fontSize: 7, color: _D.faint }}>binder.dev</div>
          </div>
        </div>

      </div>
    </div>
  );
}

Object.assign(window, {
  ChapterDivider,
  D_Essence, D_Approved, D_Misuse,
  D_ColorRatio, D_Voice,
  D_Stationery, D_Poster, D_Slides, D_Web, D_Social,
  D_DarkUI, D_AppIcons, D_ContribPass,
});
