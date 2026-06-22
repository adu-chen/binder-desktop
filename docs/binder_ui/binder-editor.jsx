/* binder-editor.jsx — Center editor: tabs, toolbar, paper, inline diff, status bar */

const { useState: useStateE, useEffect: useEffectE, useRef: useRefE } = React;

/* ─── Tabs ─── */
function EditorTabs({ tabs, activeId, onPick, onClose }) {
  return (
    <div className="tabs">
      {tabs.map(t => (
        <div key={t.id}
          className={`tab ${t.id === activeId ? 'active' : ''}`}
          onClick={() => onPick(t.id)}
        >
          <span className={`tab-dot ${t.dirty ? '' : 'clean'}`} />
          <span>{t.name}</span>
          <span className="tab-close" onClick={e => { e.stopPropagation(); onClose && onClose(t.id); }}>×</span>
        </div>
      ))}
      <div className="tab-spacer" />
      <div className="tab-actions">
        <button className="tab-action-btn" title="分页预览">≣</button>
        <button className="tab-action-btn" title="文档分析">◧</button>
        <button className="tab-action-btn" title="更多">⋯</button>
      </div>
    </div>
  );
}

function EditorToolbar({ zoom, onZoom }) {
  return (
    <div className="editor-toolbar">
      <button className="tb-btn" title="撤销">↺</button>
      <button className="tb-btn" title="重做">↻</button>
      <span className="tb-sep" />
      <button className="tb-btn" title="正文样式">P</button>
      <button className="tb-btn" title="标题">H</button>
      <span className="tb-sep" />
      <button className="tb-btn" style={{fontWeight:700}} title="粗体">B</button>
      <button className="tb-btn" style={{fontStyle:'italic'}} title="斜体">I</button>
      <button className="tb-btn" style={{textDecoration:'underline'}} title="下划线">U</button>
      <button className="tb-btn" style={{textDecoration:'line-through'}} title="删除线">S</button>
      <span className="tb-sep" />
      <button className="tb-btn" title="无序列表">•≡</button>
      <button className="tb-btn" title="有序列表">1≡</button>
      <button className="tb-btn" title="任务列表">☐≡</button>
      <span className="tb-sep" />
      <button className="tb-btn" title="表格">⊞</button>
      <button className="tb-btn" title="代码">{'</>'}</button>
      <button className="tb-btn" title="引用">❞</button>
      <button className="tb-btn" title="链接">⌒</button>
      <div className="tb-spacer" />
      <span className="t-label" style={{marginRight:10}}>⌘K 局部修改 · ⌘J 续写</span>
      <button className="tb-btn" onClick={() => onZoom(zoom - 10)}>−</button>
      <span className="tb-zoom">{zoom}%</span>
      <button className="tb-btn" onClick={() => onZoom(zoom + 10)}>+</button>
    </div>
  );
}

/* ─── Paper / doc bodies ─── */

function PaperMeta({ doc }) {
  return (
    <div className="paper-meta">
      <span><b>{doc.code}</b> · {doc.type}</span>
      <span>更新 {doc.updated} · v{doc.version}</span>
    </div>
  );
}

/* spec.md content with an inline pending diff block */
function DocSpec({ pendingDiff }) {
  return (
    <div className="doc">
      <PaperMeta doc={window.MOCK.DOC_SPEC.meta} />
      <h1>Binder · 项目介绍</h1>
      <p className="lede">
        Binder 当前是一个基于 Tauri 2 的本地桌面应用，核心形态是
        <em>"工作区 + 编辑器 + AI 协作"</em>的文档工作台。
      </p>

      <h2>I. 项目概述</h2>
      <h3>1.1 当前项目形态</h3>
      <p>应用正常布局由 <code>MainLayout</code> 组织，包含左侧工作区面板、中央编辑/预览区、右侧聊天区、底部状态栏，以及有 pending diff 时出现的底部待确认修改面板。</p>

      <h3>1.2 当前主使用方式</h3>
      <p>从代码实现看，当前主链是：</p>
      <ol>
        <li>选择一个本地目录作为 workspace。</li>
        <li>在文件树中打开、创建、移动、重命名、删除或整理资源。</li>
        <li>在中间区域编辑文本 / HTML / 转换后的文档内容，或预览其他文件类型。</li>
        <li>在右侧聊天区以 <code>chat</code> 或 <code>agent</code> 模式与 AI 协作。</li>
        <li>AI 修改先形成 diff 候选，再由用户确认写入。</li>
      </ol>

      <h2>IV. 当前能力总览</h2>
      <h3>4.7 聊天模式与工具调用</h3>

      {pendingDiff && (
        <div className="diff-block">
          <div className="diff-block-head">
            <span>HUNK · §4.7 · +5 −3 · BY BINDER · 14:33</span>
            <div className="actions">
              <button className="accept" onClick={pendingDiff.onAccept}>接受 ⏎</button>
              <button className="reject" onClick={pendingDiff.onReject}>拒绝 ⌫</button>
            </div>
          </div>
          <div className="diff-row del">
            <span className="gutter">142</span><span className="mark">−</span>
            <span className="content">聊天标签有两种模式，分别是 chat 和 agent，agent 启用工具。</span>
          </div>
          <div className="diff-row del">
            <span className="gutter">143</span><span className="mark">−</span>
            <span className="content">当前工具矩阵真实暴露的工具有：read_file、list_files、search_files、create_file、update_file…</span>
          </div>
          <div className="diff-row add">
            <span className="gutter">142</span><span className="mark">+</span>
            <span className="content">聊天标签当前有 <b>chat</b> 与 <b>agent</b> 两种模式，模式边界由 tab 元数据控制：</span>
          </div>
          <div className="diff-row add">
            <span className="gutter">143</span><span className="mark">+</span>
            <span className="content">　· <b>chat</b> 按纯对话主链运行，<u>不启用任何工具</u>。</span>
          </div>
          <div className="diff-row add">
            <span className="gutter">144</span><span className="mark">+</span>
            <span className="content">　· <b>agent</b> 模式启用工具调用，且要求 tab 已绑定 workspace 才能形成完整闭环。</span>
          </div>
        </div>
      )}

      <p>当前模式边界<span className="selected">非常明确</span>：</p>
      <ul>
        <li><code>chat</code> 模式按纯对话主链运行，不启用工具。</li>
        <li><code>agent</code> 模式才启用工具调用。</li>
        <li><code>agent</code> 模式要求 tab 绑定 workspace 才能形成完整能力闭环。</li>
      </ul>

      <p>当前工具矩阵真实暴露的工具按类别分组如下<span className="caret" /></p>

      <table>
        <thead><tr><th>类别</th><th>工具</th><th className="num">数量</th></tr></thead>
        <tbody>
          <tr><td>读取</td><td><code>read_file</code> · <code>list_files</code> · <code>search_files</code></td><td className="num">3</td></tr>
          <tr><td>资源结构</td><td><code>create_file</code> · <code>delete_file</code> · <code>move_file</code> · <code>rename_file</code> · <code>create_folder</code></td><td className="num">5</td></tr>
          <tr><td>写入</td><td><code>update_file</code> · <code>edit_current_editor_document</code></td><td className="num">2</td></tr>
          <tr><td>依赖</td><td><code>save_file_dependency</code></td><td className="num">1</td></tr>
        </tbody>
      </table>

      <div className="callout">
        <div className="smallcaps">边界 · BOUNDARIES</div>
        <p style={{margin:0}}>当前打开的文件必须走 <code>edit_current_editor_document</code>；非当前文件才走 <code>update_file</code> 或 workspace 级 diff 链。AI 修改默认进入待确认状态，而不是直接改盘。</p>
      </div>
    </div>
  );
}

function DocIntro() {
  return (
    <div className="doc">
      <PaperMeta doc={{code:'PROD-C-R-01', type:'介绍 / 引言', updated:'2026-04-12', version:'0.1.0'}} />
      <h1>合页 · 引言</h1>
      <p className="lede">把一份份零散的文档"装订"在同一个本地工作区里，让编辑、检索、AI 协作共用同一份事实层。</p>
      <h2>定位</h2>
      <p>Binder 不是云端协同平台，也不是单纯的 AI 聊天产品。它是一个本地桌面文档工作台——把文件管理、文本/文档编辑、AI 对话协作、diff 审阅、时间轴恢复，以及知识/记忆/模板等能力接入到同一条 workspace 主链中。</p>
      <h2>命名由来</h2>
      <p>Binder = 活页夹 / 合页。每一份文档都是夹子里的一页，可以增删、可以替换，但夹子本身和它的事实库（<code>workspace.db</code>）保持稳定。</p>
    </div>
  );
}

function DocPersistence() {
  return (
    <div className="doc">
      <PaperMeta doc={{code:'PROD-C-R-07', type:'持久化', updated:'2026-04-10', version:'0.1.0'}} />
      <h1>持久化与本地存储</h1>
      <h2><code>workspace.db</code></h2>
      <p><code>workspace/.binder/workspace.db</code> 是当前项目的核心事实库。<code>SCHEMA_VERSION = 8</code>。</p>
      <table>
        <thead><tr><th>表</th><th>用途</th></tr></thead>
        <tbody>
          <tr><td><code>file_cache</code></td><td>canonical 文件缓存</td></tr>
          <tr><td><code>pending_diffs</code></td><td>待确认改动</td></tr>
          <tr><td><code>timeline_nodes</code></td><td>时间轴事件</td></tr>
          <tr><td><code>agent_tasks</code></td><td>Agent 任务持久化</td></tr>
        </tbody>
      </table>
    </div>
  );
}

function pickDoc(fileId, pendingDiff) {
  switch (fileId) {
    case 'spec': return <DocSpec pendingDiff={pendingDiff} />;
    case 'intro': return <DocIntro />;
    case 'persistence': return <DocPersistence />;
    default: return <DocIntro />;
  }
}

/* ─── Inline Assist (Cmd+K) ─── */
function InlineAssist({ onClose }) {
  const [v, setV] = useStateE('把这一段改写得更书面化');
  return (
    <div className="assist" onClick={e => e.stopPropagation()}>
      <div className="assist-hd">
        <span>INLINE ASSIST · ⌘K · 局部修改</span>
        <span>spec.md · §4.7 · 3 行选中</span>
      </div>
      <div className="assist-input">
        <textarea value={v} onChange={e => setV(e.target.value)} autoFocus />
      </div>
      <div className="assist-foot">
        <span>模型 · deepseek-chat</span>
        <div style={{display:'flex', gap:14}}>
          <span>ESC 取消</span>
          <button onClick={onClose}>⏎ 生成 diff</button>
        </div>
      </div>
    </div>
  );
}

/* ─── Editor body ─── */
function EditorBody({ activeFile, pendingDiff }) {
  return (
    <div className="editor-scroll">
      <div className="paper">
        {pickDoc(activeFile, pendingDiff)}
      </div>
    </div>
  );
}

/* ─── Status bar ─── */
function StatusBar({ pendingCount, mode = 'agent', model = 'deepseek-chat' }) {
  return (
    <div className="statusbar">
      <span className="sb-item"><span className="sb-dot" /> READY</span>
      <span className="sb-item">⎇ <b>main</b></span>
      <span className="sb-item">workspace · <b>{window.MOCK.WS_NAME}</b></span>
      <span className="sb-item">⌘ <b>247</b> files · <b>77,556</b> LOC</span>
      <div className="sb-spacer" />
      <span className="sb-item">MODE · <b>{mode.toUpperCase()}</b></span>
      <span className="sb-item">MODEL · <b>{model}</b></span>
      <span className="sb-item">⌖ Ln 142 · Col 18</span>
      <span className="sb-item">UTF-8 · LF</span>
      {pendingCount > 0 && (
        <span className="sb-item" style={{color:'var(--rust)'}}>
          <span className="sb-dot warn" /> <b>{pendingCount}</b> PENDING DIFF
        </span>
      )}
    </div>
  );
}

/* ─── Bottom Diff Strip ─── */
function DiffStrip({ diffs, onAcceptAll, onRejectAll, onPick }) {
  if (!diffs.length) return null;
  return (
    <div className="diffstrip">
      <div className="diffstrip-label">
        <span className="num">{diffs.length}</span>
        <span>待确认 · PENDING DIFF</span>
      </div>
      <div className="diffstrip-files">
        {diffs.map(d => (
          <div className="diffstrip-file" key={d.id} onClick={() => onPick && onPick(d)}>
            <b>{d.file}</b>
            <span>·</span>
            <span>{d.scope}</span>
            <span className="pm">+{d.added}</span>
            <span className="mn">−{d.removed}</span>
          </div>
        ))}
      </div>
      <div className="diffstrip-actions">
        <button className="diffstrip-btn danger" onClick={onRejectAll}>全部拒绝</button>
        <button className="diffstrip-btn primary" onClick={onAcceptAll}>全部接受 · ⌘⏎</button>
      </div>
    </div>
  );
}

window.BinderEditor = { EditorTabs, EditorToolbar, EditorBody, StatusBar, DiffStrip, InlineAssist };
