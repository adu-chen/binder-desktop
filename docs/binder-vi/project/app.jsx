/* app.jsx — Binder Core VI manual · D-06 final · organized into chapters */

// ── Cover (refined for final VI manual)
function Cover() {
  return (
    <div style={{ width: '100%', height: '100%', background: D_palette.ink, color: D_palette.paper, position: 'relative', fontFamily: 'IBM Plex Sans, sans-serif' }}>
      <div style={{ position: 'absolute', inset: 0, padding: '44px 56px', display: 'flex', flexDirection: 'column', justifyContent: 'space-between' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 16 }}>
          <LogoD size={42} ink={D_palette.paper}/>
          <div className="mono" style={{ fontSize: 11, letterSpacing: 1.6, color: 'rgba(244,241,234,0.5)' }}>BINDER · CORE / VI · v1.0 · 2026</div>
        </div>

        <div>
          <div className="mono" style={{ fontSize: 11, letterSpacing: 1.8, color: D_palette.accent, marginBottom: 14 }}>
            FINAL · MARK D-06 SELECTED
          </div>
          <div style={{ fontFamily: 'Instrument Serif, serif', fontSize: 92, lineHeight: 0.95, letterSpacing: -2.6, color: D_palette.paper }}>
            Binder Core<br/><span style={{ color: 'rgba(244,241,234,0.55)' }}>Visual Identity</span>
          </div>
          <div className="sc" style={{ fontSize: 15, color: 'rgba(244,241,234,0.7)', marginTop: 22, maxWidth: 580, lineHeight: 1.7 }}>
            完整 VI 规范 — 标志、色彩、字体、辅助、文具、印刷、数字、周边 ——
            围绕<span style={{ color: D_palette.paper }}>"段落块 + 一处点缀"</span>展开。
          </div>
        </div>

        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 14, paddingBottom: 4 }}>
          {[
            ['§01', 'Logo',       '6 cards'],
            ['§02', 'Color',      '2 cards'],
            ['§03', 'Type',       '1 card'],
            ['§04', 'Aux',        '2 cards'],
            ['§05', 'Stationery', '1 card'],
            ['§06', 'Print',      '1 card'],
            ['§07', 'Digital',    '6 cards'],
            ['§08', 'Merch',      '2 cards'],
          ].map(([ch, name, count]) => (
            <div key={ch} style={{ borderTop: '1px solid rgba(244,241,234,0.15)', paddingTop: 8 }}>
              <div className="mono" style={{ fontSize: 10, color: D_palette.accent, letterSpacing: 1.4 }}>{ch}</div>
              <div style={{ fontFamily: "'IBM Plex Sans', sans-serif", fontWeight: 500, fontSize: 14, marginTop: 2 }}>{name}</div>
              <div className="mono" style={{ fontSize: 9, color: 'rgba(244,241,234,0.45)', marginTop: 2 }}>{count}</div>
            </div>
          ))}
        </div>
      </div>

      <div style={{ position: 'absolute', top: 18, right: 22 }}>
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="rgba(244,241,234,0.3)" strokeWidth="1"><path d="M0 0 H14 V14"/></svg>
      </div>
      <div style={{ position: 'absolute', bottom: 18, left: 22 }}>
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="rgba(244,241,234,0.3)" strokeWidth="1"><path d="M0 0 V14 H14"/></svg>
      </div>
    </div>
  );
}

// ──────────────────────────────────────────────
function App() {
  return (
    <DesignCanvas>

      <DCSection id="ch-00" title="00 · 概述" subtitle="Cover & Essence — what Binder is, in one page">
        <DCArtboard id="cover"   label="00.0 · Cover"   width={920} height={580}><Cover/></DCArtboard>
        <DCArtboard id="essence" label="00.1 · Essence" width={760} height={520}><D_Essence/></DCArtboard>
      </DCSection>

      <DCSection id="ch-01" title="01 · 标志 Logo" subtitle="D-06 · 段落块 + 一处点缀 · 6 张">
        <DCArtboard id="primary"      label="01.1 · Primary Lockup"   width={720} height={520}><D1_Hero/></DCArtboard>
        <DCArtboard id="variants"     label="01.2 · 8 Variants Picked" width={920} height={580}><D2_Variants/></DCArtboard>
        <DCArtboard id="construction" label="01.3 · Construction"     width={680} height={520}><D4_System/></DCArtboard>
        <DCArtboard id="approved"     label="01.4 · Approved Versions" width={760} height={560}><D_Approved/></DCArtboard>
        <DCArtboard id="misuse"       label="01.5 · Misuse / 六不准"   width={760} height={520}><D_Misuse/></DCArtboard>
        <DCArtboard id="dynamic"      label="01.6 · Dynamic Mark"     width={760} height={520}><D3_Dynamic/></DCArtboard>
      </DCSection>

      <DCSection id="ch-02" title="02 · 色彩 Color" subtitle="暖纸 · 编辑器墨 · 一抹批注红">
        <DCArtboard id="palette" label="02.1 · Palette"      width={680} height={560}><D5_Color/></DCArtboard>
        <DCArtboard id="ratio"   label="02.2 · Usage Ratios" width={680} height={520}><D_ColorRatio/></DCArtboard>
      </DCSection>

      <DCSection id="ch-03" title="03 · 字体 Type" subtitle="IBM Plex Sans / Mono · Noto SC · Instrument Serif">
        <DCArtboard id="type" label="03.1 · Type System" width={760} height={520}><D6_Type/></DCArtboard>
      </DCSection>

      <DCSection id="ch-04" title="04 · 辅助 Aux" subtitle="纹样 · 图标 · 语言">
        <DCArtboard id="aux"   label="04.1 · Pattern & Icons" width={760} height={520}><D7_Aux/></DCArtboard>
        <DCArtboard id="voice" label="04.2 · Voice & Tone"    width={760} height={600}><D_Voice/></DCArtboard>
      </DCSection>

      <DCSection id="ch-05" title="05 · 文具 Stationery" subtitle="名片 · 信纸 · 信封">
        <DCArtboard id="stationery" label="05.1 · Cards / Letterhead" width={900} height={540}><D_Stationery/></DCArtboard>
      </DCSection>

      <DCSection id="ch-06" title="06 · 印刷 Print" subtitle="海报系列 · A2">
        <DCArtboard id="posters" label="06.1 · Poster Series" width={840} height={580}><D_Poster/></DCArtboard>
      </DCSection>

      <DCSection id="ch-07" title="07 · 数字 Digital" subtitle="图标 · 产品 · 网站 · 社交 · 幻灯片">
        <DCArtboard id="icons"     label="07.1 · App Icon Set"   width={820} height={540}><D_AppIcons/></DCArtboard>
        <DCArtboard id="ui-light"  label="07.2 · Product · Light" width={820} height={520}><D9_Digital/></DCArtboard>
        <DCArtboard id="ui-dark"   label="07.3 · Product · Dark"  width={820} height={520}><D_DarkUI/></DCArtboard>
        <DCArtboard id="web"       label="07.4 · Landing Page"    width={860} height={520}><D_Web/></DCArtboard>
        <DCArtboard id="slides"    label="07.5 · Slides"          width={820} height={540}><D_Slides/></DCArtboard>
        <DCArtboard id="social"    label="07.6 · Social Kit"      width={900} height={540}><D_Social/></DCArtboard>
      </DCSection>

      <DCSection id="ch-08" title="08 · 周边 Merch" subtitle="周边 · 贡献者凭证 · 包装">
        <DCArtboard id="merch"  label="08.1 · Merch Lineup"      width={680} height={520}><D10_Merch/></DCArtboard>
        <DCArtboard id="contrib" label="08.2 · Contributor Pass"  width={760} height={520}><D_ContribPass/></DCArtboard>
      </DCSection>

      <DCSection id="archive" title="附录 · 早期探索" subtitle="A · B · C — for reference only">
        <DCArtboard id="a1" label="A · Bracket [B]"  width={720} height={520}><A1_Hero/></DCArtboard>
        <DCArtboard id="b1" label="B · Block ▢"      width={720} height={520}><B1_Hero/></DCArtboard>
        <DCArtboard id="c1" label="C · Diff ±"       width={720} height={520}><C1_Hero/></DCArtboard>
      </DCSection>

    </DesignCanvas>
  );
}

ReactDOM.createRoot(document.getElementById('root')).render(<App/>);
