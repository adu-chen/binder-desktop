/* shared.jsx — common artboard primitives shared across directions */

// Caption row at the bottom of an artboard (or section) — uppercase mono.
function Caption({ children, style }) {
  return (
    <div className="mono" style={{
      fontSize: 10, letterSpacing: 0.8, textTransform: 'uppercase',
      color: 'rgba(0,0,0,0.45)', ...style,
    }}>{children}</div>
  );
}

// Top metadata strip (think CAD title block)
function MetaStrip({ id, title, sub, accent = '#111' }) {
  return (
    <div className="mono" style={{
      display: 'flex', alignItems: 'center', gap: 12,
      padding: '10px 20px', borderBottom: '1px solid rgba(0,0,0,0.08)',
      fontSize: 10, letterSpacing: 0.6, textTransform: 'uppercase',
      color: 'rgba(0,0,0,0.55)',
    }}>
      <span style={{ width: 6, height: 6, borderRadius: 1, background: accent }}/>
      <span style={{ color: 'rgba(0,0,0,0.85)', fontWeight: 600 }}>{id}</span>
      <span style={{ flex: 1, height: 1, background: 'rgba(0,0,0,0.06)' }}/>
      <span>{title}</span>
      {sub && <span style={{ color: 'rgba(0,0,0,0.4)' }}>· {sub}</span>}
    </div>
  );
}

// Crosshair corner marks for an artboard padded area
function Crosshair({ size = 8, color = 'rgba(0,0,0,0.25)' }) {
  return (
    <svg width={size*2} height={size*2} viewBox={`0 0 ${size*2} ${size*2}`} style={{ display: 'block' }}>
      <line x1={size} y1="0" x2={size} y2={size*2} stroke={color} strokeWidth="1" />
      <line x1="0" y1={size} x2={size*2} y2={size} stroke={color} strokeWidth="1" />
    </svg>
  );
}

// Color swatch
function Swatch({ color, name, code, hex, dark = false, large = false }) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 0, minWidth: 0 }}>
      <div style={{
        background: color,
        height: large ? 200 : 96,
        borderRadius: 0,
        boxShadow: 'inset 0 0 0 1px rgba(0,0,0,0.06)',
      }}/>
      <div style={{ padding: '8px 0 0' }}>
        <div className="mono" style={{ fontSize: 9, letterSpacing: 0.5, color: 'rgba(0,0,0,0.4)', textTransform: 'uppercase' }}>{code}</div>
        <div className="sc" style={{ fontSize: 12, fontWeight: 500, color: '#111', marginTop: 1 }}>{name}</div>
        <div className="mono" style={{ fontSize: 10, color: 'rgba(0,0,0,0.5)', marginTop: 2 }}>{hex}</div>
      </div>
    </div>
  );
}

// Striped placeholder block — used for "drop photo here" zones
function PhotoSlot({ label = 'PHOTO PLACEHOLDER', style }) {
  return (
    <div style={{
      backgroundImage: 'repeating-linear-gradient(45deg, rgba(0,0,0,0.04) 0 8px, transparent 8px 16px)',
      background: '#ebe9e3',
      backgroundBlendMode: 'multiply',
      display: 'flex', alignItems: 'center', justifyContent: 'center',
      ...style,
    }}>
      <div className="mono" style={{ fontSize: 10, letterSpacing: 1.4, color: 'rgba(0,0,0,0.4)' }}>{label}</div>
    </div>
  );
}

// A tiny inline diff-line for hero scenes
function DiffLine({ kind = '+', children, dim = false }) {
  const map = {
    '+': { bg: 'rgba(46,140,90,0.10)', stripe: '#2e8c5a', text: '#1b5a3a', sign: '+' },
    '-': { bg: 'rgba(180,70,55,0.10)', stripe: '#b44637', text: '#7a2f24', sign: '−' },
    ' ': { bg: 'transparent', stripe: 'transparent', text: 'rgba(0,0,0,0.5)', sign: ' ' },
  };
  const m = map[kind];
  return (
    <div style={{ display: 'flex', alignItems: 'stretch', background: m.bg, opacity: dim ? 0.55 : 1 }}>
      <div style={{ width: 3, background: m.stripe }} />
      <div className="mono" style={{ width: 18, textAlign: 'center', color: m.text, fontSize: 11, padding: '3px 0' }}>{m.sign}</div>
      <div className="mono" style={{ fontSize: 11, color: m.text, padding: '3px 6px', whiteSpace: 'pre' }}>{children}</div>
    </div>
  );
}

// Faux app window chrome
function AppWindow({ children, style, dark = false, title = 'workspace · ~/binder-demo' }) {
  const bg = dark ? '#171513' : '#fafaf7';
  const fg = dark ? 'rgba(255,255,255,0.7)' : 'rgba(0,0,0,0.55)';
  const border = dark ? 'rgba(255,255,255,0.08)' : 'rgba(0,0,0,0.08)';
  return (
    <div style={{
      background: bg, color: fg, border: `1px solid ${border}`,
      borderRadius: 6, overflow: 'hidden', boxShadow: '0 8px 24px rgba(0,0,0,0.08)',
      ...style,
    }}>
      <div style={{
        display: 'flex', alignItems: 'center', padding: '8px 12px',
        borderBottom: `1px solid ${border}`, gap: 8, fontSize: 10,
      }}>
        <div style={{ display: 'flex', gap: 5 }}>
          <div style={{ width: 9, height: 9, borderRadius: 5, background: '#e87672' }}/>
          <div style={{ width: 9, height: 9, borderRadius: 5, background: '#e8c46c' }}/>
          <div style={{ width: 9, height: 9, borderRadius: 5, background: '#7bbf85' }}/>
        </div>
        <div className="mono" style={{ flex: 1, textAlign: 'center', fontSize: 10, opacity: 0.65 }}>{title}</div>
        <div style={{ width: 36 }}/>
      </div>
      {children}
    </div>
  );
}

Object.assign(window, { Caption, MetaStrip, Crosshair, Swatch, PhotoSlot, DiffLine, AppWindow });
