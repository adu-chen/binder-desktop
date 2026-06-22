/* binder-parts.jsx — Logo, Left Rail, side-panel headers + 5 module panels */

const { useState, useMemo, useRef, useEffect } = React;

/* ─── Logo · Binder Core VI · D-06 ───
   Block-stack mark — 段落条堆叠的 B silhouette.
   8 bars on 120×120 grid · padX/padY = 14 · barH = 8 · accentBar = 3.
   Compact = 4 bars [100,56,100,72] · barH = 16 — used when size ≤ 24px.
   Favicon = no accent · solid ink/paper. */
const BINDER_BARS = [100, 64, 82, 100, 72, 100, 56, 100];
const BINDER_BARS_COMPACT = [100, 56, 100, 72];

function BinderMark({
  size = 28,
  variant = 'accented',          // 'accented' | 'default' | 'negative' | 'compact' | 'favicon'
  ink = 'var(--ink)',
  paper = 'var(--bg-page)',
  accent = 'var(--rust)',
  accentBar = 3,
  inSurface = false,             // true → renders mark only (no surrounding tile)
  style,
}) {
  // auto-switch to compact at very small sizes
  const v = variant === 'accented' || variant === 'default' || variant === 'negative'
    ? (size <= 22 ? 'compact' : variant)
    : variant;

  const isCompact = v === 'compact';
  const isInverted = v === 'negative';
  const isFavicon = v === 'favicon';
  const showAccent = v === 'accented';

  const bars = isCompact ? BINDER_BARS_COMPACT : BINDER_BARS;
  const barH = isCompact ? 16 : 8;
  const padX = 14, padY = 14;
  const innerW = 120 - 2 * padX;
  const rhythm = bars.length === 1 ? 0 : (120 - 2 * padY - barH) / (bars.length - 1);

  const barFill = isInverted ? paper : ink;
  const accentFill = isFavicon ? barFill : accent;

  return (
    <svg
      viewBox="0 0 120 120"
      width={size} height={size}
      style={{ display: 'block', flexShrink: 0, ...style }}
    >
      {isInverted && <rect width="120" height="120" fill={ink} />}
      {bars.map((w, i) => {
        const barW = innerW * (w / 100);
        const y = padY + i * rhythm;
        const fill = (showAccent && i === accentBar) ? accentFill : barFill;
        return <rect key={i} x={padX} y={y} width={barW} height={barH} fill={fill} />;
      })}
    </svg>
  );
}

/* ─── Wordmark · binder. ─── */
function BinderWordmark({ size = 24, color = 'var(--ink)', dot = true }) {
  return (
    <span style={{
      fontFamily: "'IBM Plex Sans', 'Inter', sans-serif",
      fontWeight: 500, fontSize: size,
      letterSpacing: -size * 0.04,
      color, lineHeight: 0.95, display: 'inline-block',
    }}>
      binder{dot && <span style={{ color: 'var(--rust)' }}>.</span>}
    </span>
  );
}

/* ─── Left rail (5 modules) ─── */
const RAIL_ITEMS = [
  { id: 'files',     glyph: 'WS',  num: 'I·A', label: '工作区' },
  { id: 'memory',    glyph: 'ME',  num: 'I·B', label: '记忆库' },
  { id: 'knowledge', glyph: 'KN',  num: 'I·C', label: '知识库' },
  { id: 'templates', glyph: 'TM',  num: 'I·D', label: '模板库' },
  { id: 'timeline',  glyph: 'TL',  num: 'I·E', label: '时间轴' },
];

function LeftRail({ active, onChange }) {
  return (
    <div className="rail">
      <div style={{ marginBottom: 12, padding: '0 4px' }}>
        <BinderMark size={28} variant="accented" />
      </div>
      {RAIL_ITEMS.map(it => (
        <button key={it.id}
          className={`rail-btn ${active === it.id ? 'active' : ''}`}
          onClick={() => onChange(it.id)}
          title={it.label}
        >
          <span className="rail-btn-glyph">{it.glyph}</span>
          <span className="rail-btn-num">{it.num}</span>
        </button>
      ))}
      <div className="rail-spacer" />
      <button className="rail-btn" title="设置">
        <span className="rail-btn-glyph">·</span>
        <span className="rail-btn-num">SET</span>
      </button>
    </div>
  );
}

/* ─── Side panel chrome ─── */
function SideHeader({ num, name, count, action }) {
  return (
    <div className="side-hd">
      <div className="side-hd-title">
        <span className="side-hd-num">{num}</span>
        <span className="side-hd-name">{name}</span>
      </div>
      <div style={{display:'flex', alignItems:'baseline', gap: 10}}>
        {count != null && <span className="side-hd-count">{count}</span>}
        {action}
      </div>
    </div>
  );
}

function SideSearch({ placeholder, value, onChange }) {
  return (
    <div className="side-search">
      <span className="side-search-glyph">⌕</span>
      <input
        placeholder={placeholder}
        value={value || ''}
        onChange={e => onChange && onChange(e.target.value)}
      />
      <span className="side-search-glyph">⌘F</span>
    </div>
  );
}

/* ─── 1. Workspace · File tree ─── */
function FileTreeNode({ node, depth, openMap, setOpen, activeFile, onPick }) {
  const M = window.MOCK;
  const isFolder = node.type === 'folder';
  const isOpen = openMap[node.id] !== undefined ? openMap[node.id] : node.open;
  const isActive = !isFolder && activeFile === node.id;
  const indent = 14 + depth * 14;
  return (
    <>
      <div
        className={`tree-row ${isActive ? 'active' : ''} ${node.dirty ? 'dirty' : ''}`}
        style={{ paddingLeft: indent }}
        onClick={() => isFolder ? setOpen(node.id, !isOpen) : onPick && onPick(node.id)}
      >
        {isFolder
          ? <span className="tree-chev">{isOpen ? '▾' : '▸'}</span>
          : <span className="tree-chev"></span>
        }
        <span className="tree-icon">
          {isFolder
            ? (isOpen ? '▢' : '▢')
            : (M.FILE_ICONS[node.type] || '·')
          }
        </span>
        <span className="tree-name">{node.name}</span>
        {node.dirty && <span className="tree-dot">●</span>}
      </div>
      {isFolder && isOpen && node.children && node.children.map(c => (
        <FileTreeNode key={c.id} node={c} depth={depth+1}
          openMap={openMap} setOpen={setOpen}
          activeFile={activeFile} onPick={onPick} />
      ))}
    </>
  );
}

function FileTreePanel({ activeFile, onPickFile }) {
  const M = window.MOCK;
  const [openMap, setOpenMap] = useState({});
  const setOpen = (id, v) => setOpenMap(m => ({...m, [id]: v}));
  const [q, setQ] = useState('');
  return (
    <>
      <SideHeader num="I·A" name={`工作区 · ${M.WS_NAME}`} count="9 项"
        action={<span className="t-label" style={{cursor:'default'}}>+ 新建</span>}
      />
      <SideSearch placeholder="过滤文件…" value={q} onChange={setQ} />
      <div className="side-body">
        <div className="tree">
          <div className="tree-section">
            <span>当前工作区</span>
            <span>{M.WS_PATH}</span>
          </div>
          {M.FILE_TREE.map(n => (
            <FileTreeNode key={n.id} node={n} depth={0}
              openMap={openMap} setOpen={setOpen}
              activeFile={activeFile} onPick={onPickFile} />
          ))}
        </div>
      </div>
    </>
  );
}

/* ─── 2. Memory ─── */
function MemoryPanel() {
  const M = window.MOCK;
  return (
    <>
      <SideHeader num="I·B" name="记忆库" count={`${M.MEMORIES.length} 条`}
        action={<span className="t-label">+ 新建</span>}
      />
      <SideSearch placeholder="检索记忆…" />
      <div className="side-body">
        <div className="tree-section"><span>分层</span><span>WS · USER</span></div>
        {M.MEMORIES.map(m => (
          <div className="list-item" key={m.id}>
            <div className="list-item-hd">
              <div className="list-item-title">{m.title}</div>
              <span className={`list-item-meta`} style={{color: m.layer==='WS' ? 'var(--rust)' : 'var(--forest)'}}>{m.layer}</span>
            </div>
            <div className="list-item-body">{m.body}</div>
            <div className="list-item-tags">
              {m.tags.map((t,i) => <span className="tag" key={i}>{t}</span>)}
              <span className="tag" style={{marginLeft:'auto', color:'var(--ink-5)'}}>{m.updated}</span>
            </div>
          </div>
        ))}
      </div>
    </>
  );
}

/* ─── 3. Knowledge ─── */
function KnowledgePanel() {
  const M = window.MOCK;
  return (
    <>
      <SideHeader num="I·C" name="知识库" count={`${M.KNOWLEDGE.length} 条`}
        action={<span className="t-label">+ 入库</span>}
      />
      <SideSearch placeholder="检索 / 拖拽以引用…" />
      <div className="side-body">
        <div className="tree-section"><span>条目</span><span>FTS · CHUNK · CITE</span></div>
        {M.KNOWLEDGE.map(k => (
          <div className="list-item" key={k.id}>
            <div className="list-item-hd">
              <div className="list-item-title">{k.title}</div>
              <span className={`tag ${k.tagKind}`}>{k.tag}</span>
            </div>
            <div className="list-item-body">{k.body}</div>
            <div className="list-item-tags">
              <span className="tag">{k.refs} 处引用</span>
              <span className="tag" style={{marginLeft:'auto', color:'var(--ink-5)'}}>{k.updated}</span>
            </div>
          </div>
        ))}
      </div>
    </>
  );
}

/* ─── 4. Templates ─── */
function TemplatesPanel() {
  const M = window.MOCK;
  return (
    <>
      <SideHeader num="I·D" name="模板库 · 工作流" count={`${M.TEMPLATES.length} 个`}
        action={<span className="t-label">+ 新建</span>}
      />
      <SideSearch placeholder="检索模板…" />
      <div className="side-body">
        <div className="tree-section"><span>工作流模板</span><span>RUNTIME · COMPILE</span></div>
        {M.TEMPLATES.map(t => (
          <div className="list-item" key={t.id}>
            <div className="list-item-hd">
              <div style={{display:'flex', alignItems:'baseline', gap:8, minWidth:0, flex:1}}>
                <span className="list-item-meta" style={{color:'var(--ink-3)'}}>{t.code}</span>
                <div className="list-item-title">{t.title}</div>
              </div>
              <span className={`tag ${t.tagKind}`}>{t.status}</span>
            </div>
            <div className="list-item-body">{t.body}</div>
            <div className="list-item-tags">
              <span className="tag">{t.steps} 步</span>
              <span className="tag">编译就绪</span>
            </div>
          </div>
        ))}
      </div>
    </>
  );
}

/* ─── 5. Timeline ─── */
function TimelinePanel({ currentNode, onPick }) {
  const M = window.MOCK;
  return (
    <>
      <SideHeader num="I·E" name="时间轴" count="≤ 50 · 轮询"
        action={<span className="t-label">刷新</span>}
      />
      <SideSearch placeholder="按文件 / 关键词过滤…" />
      <div className="side-body">
        <div className="tree-section"><span>事件流</span><span>WS 级 · 文件 · 结构</span></div>
        {M.TIMELINE.map((it, i) => (
          <div
            key={it.id}
            className={`tl-item ${currentNode === it.id ? 'current' : ''}`}
            onClick={() => onPick && onPick(it.id)}
          >
            <div className="tl-time">
              <b>{it.time}</b>
              <span>{it.date}</span>
              <span style={{marginTop:4, color: it.kind === 'diff' ? 'var(--rust)' : 'var(--ink-5)'}}>#{it.id}</span>
            </div>
            <div className="tl-body">
              <div className="tl-title">{it.title}</div>
              <div className="tl-desc">{it.desc}</div>
              {it.pending && <div style={{marginTop:6}}><span className="tag rust">PENDING</span></div>}
              {it.highlight && <div style={{marginTop:6}}><span className="tag forest">恢复目标</span></div>}
            </div>
          </div>
        ))}
      </div>
    </>
  );
}

window.BinderParts = { BinderMark, BinderWordmark, LeftRail, FileTreePanel, MemoryPanel, KnowledgePanel, TemplatesPanel, TimelinePanel };
