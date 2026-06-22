/* binder-app.jsx — Welcome screen + main BinderApp combining everything */

const { useState: useStateA, useMemo: useMemoA } = React;

const { BinderMark, BinderWordmark, LeftRail, FileTreePanel, MemoryPanel, KnowledgePanel, TemplatesPanel, TimelinePanel } = window.BinderParts;
const { EditorTabs, EditorToolbar, EditorBody, StatusBar, DiffStrip, InlineAssist } = window.BinderEditor;
const { ChatPanel } = window.BinderChat;

/* ─── Side panel router ─── */
function SidePanelRouter({ activeModule, activeFile, onPickFile, currentTimelineNode, onPickTimeline }) {
  switch (activeModule) {
    case 'files':     return <FileTreePanel activeFile={activeFile} onPickFile={onPickFile} />;
    case 'memory':    return <MemoryPanel />;
    case 'knowledge': return <KnowledgePanel />;
    case 'templates': return <TemplatesPanel />;
    case 'timeline':  return <TimelinePanel currentNode={currentTimelineNode} onPick={onPickTimeline} />;
    default:          return <FileTreePanel activeFile={activeFile} onPickFile={onPickFile} />;
  }
}

/* ─── Title bar ─── */
function TitleBar({ path, file, mode, dark }) {
  return (
    <div className="titlebar">
      <div className="tb-lights">
        <span className="tb-light r" />
        <span className="tb-light y" />
        <span className="tb-light g" />
      </div>
      <div style={{display:'flex', alignItems:'center', gap:6}}>
        <BinderMark size={14} variant="accented" />
        <BinderWordmark size={12} dot={true} />
      </div>
      <div className="tb-path">
        {path}/<b>{file}</b>
      </div>
      <span className="tb-meta">{mode.toUpperCase()} · DEEPSEEK</span>
    </div>
  );
}

/* ─── Welcome screen ─── */
function Welcome({ onOpenWorkspace }) {
  const M = window.MOCK;
  const [chat, setChat] = useStateA('');
  return (
    <div style={{position:'relative', height:'100%', display:'grid', gridTemplateRows:'var(--titlebar-h) 1fr var(--statusbar-h)'}}>
      <div className="titlebar">
        <div className="tb-lights"><span className="tb-light r" /><span className="tb-light y" /><span className="tb-light g" /></div>
        <span className="tb-meta">BINDER · 0.1.0</span>
        <div className="tb-path"><b>binder</b> · 无工作区</div>
        <span className="tb-meta">⌘O 打开 · ⌘N 新建</span>
      </div>

      <div className="welcome">
        <div className="welcome-inner">
          <div className="welcome-hd">
            <div className="vi-mark">
              <BinderMark size={48} variant="accented" />
              <div>
                <div className="code">BINDER · CORE · 0.1.0 · NO WORKSPACE</div>
                <div style={{marginTop:6}}>
                  <BinderWordmark size={28} />
                </div>
              </div>
            </div>
            <h1 style={{fontFamily:'var(--font-sans)', fontWeight:500, letterSpacing:'-0.03em'}}>
              Edit one <span style={{color:'var(--rust)'}}>paragraph</span> at a time<span style={{color:'var(--rust)'}}>.</span>
            </h1>
            <p>本地优先的 AI 文档工作台。选择一个目录作为 workspace，或直接发起一段临时对话——agent 工具链与完整工作区能力会在打开 workspace 之后接通。</p>
          </div>

          <div>
            <div className="welcome-col-hd"><span>最近工作区</span><span>4</span></div>
            {M.RECENT_WORKSPACES.map((w, i) => (
              <div className="recent-row" key={i} onClick={onOpenWorkspace}>
                <div style={{display:'flex', justifyContent:'space-between', alignItems:'baseline'}}>
                  <div className="rr-name">{w.name}{w.current && <span style={{marginLeft:8, color:'var(--rust)', fontSize:10, fontFamily:'var(--font-mono)', letterSpacing:'0.08em'}}>· 上次会话</span>}</div>
                  <div className="rr-meta">{w.when}</div>
                </div>
                <div className="rr-path">{w.path}</div>
                <div className="rr-meta">{w.files} 项</div>
              </div>
            ))}
          </div>

          <div>
            <div className="welcome-col-hd"><span>开始</span><span>⌘N · ⌘O</span></div>
            <div className="welcome-actions">
              <button className="welcome-btn primary" onClick={onOpenWorkspace}>
                <span className="welcome-btn-name">打开工作区…</span>
                <span className="welcome-btn-hint">⌘O</span>
              </button>
              <button className="welcome-btn" onClick={onOpenWorkspace}>
                <span className="welcome-btn-name">选择目录新建</span>
                <span className="welcome-btn-hint">⌘N</span>
              </button>
              <button className="welcome-btn">
                <span className="welcome-btn-name">配置 API Key · OpenAI / DeepSeek</span>
                <span className="welcome-btn-hint">⌘,</span>
              </button>
            </div>

            <div className="welcome-quickchat">
              <div className="welcome-col-hd" style={{margin:'0 0 8px', padding:0, border:0}}>
                <span>临时聊天 · 无工作区</span>
                <span>纯对话 · 无工具</span>
              </div>
              <textarea
                placeholder="问点什么…（无工作区时仅启用 chat 模式）"
                value={chat}
                onChange={e => setChat(e.target.value)}
              />
              <div className="qc-foot">
                <span>模型 · deepseek-chat</span>
                <span>⌘⏎ 发送 → 进入全屏聊天</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div className="statusbar">
        <span className="sb-item"><span className="sb-dot warn" /> NO WORKSPACE</span>
        <span className="sb-item">仅 chat 模式可用</span>
        <div className="sb-spacer" />
        <span className="sb-item">UTF-8 · LF</span>
      </div>
    </div>
  );
}

/* ─── Main app ─── */
function BinderApp({
  initialModule = 'files',
  initialFile = 'spec',
  initialMode = 'agent',
  initialMessages = 'diff',     // 'diff' | 'timeline' | 'welcome'
  dark = false,
  showInlineAssist = false,
  initialTimelineNode = null,
}) {
  const M = window.MOCK;
  const [activeModule, setActiveModule] = useStateA(initialModule);
  const [activeFile, setActiveFile] = useStateA(initialFile);
  const [tabs, setTabs] = useStateA(M.OPEN_TABS);
  const [chatMode, setChatMode] = useStateA(initialMode);
  const [chatTabs, setChatTabs] = useStateA(M.CHAT_TABS);
  const [activeChatTab, setActiveChatTab] = useStateA(M.CHAT_TABS[0].id);
  const [zoom, setZoom] = useStateA(100);
  const [diffState, setDiffState] = useStateA('open');  // 'open' | 'accepted' | 'rejected'
  const [diffs, setDiffs] = useStateA(M.PENDING_DIFFS);
  const [tlNode, setTlNode] = useStateA(initialTimelineNode);
  const [assistOpen, setAssistOpen] = useStateA(showInlineAssist);

  const messages = useMemoA(() => {
    if (initialMessages === 'timeline') return M.MESSAGES_TIMELINE;
    if (initialMessages === 'welcome') return M.MESSAGES_WELCOME;
    return M.MESSAGES_DIFF;
  }, [initialMessages]);

  const pickFile = (id) => {
    setActiveFile(id);
    if (!tabs.find(t => t.id === id)) {
      const node = findNode(M.FILE_TREE, id);
      if (node) setTabs(t => [...t, { id: node.id, name: node.name, dirty: false }]);
    }
  };
  const closeTab = (id) => {
    setTabs(t => t.filter(x => x.id !== id));
    if (activeFile === id) {
      const rem = tabs.filter(x => x.id !== id);
      if (rem.length) setActiveFile(rem[0].id);
    }
  };

  const acceptDiff = () => {
    setDiffState('accepted');
    setDiffs(ds => ds.filter(d => d.id !== 'd1'));
  };
  const rejectDiff = () => {
    setDiffState('rejected');
    setDiffs(ds => ds.filter(d => d.id !== 'd1'));
  };
  const acceptAll = () => { setDiffState('accepted'); setDiffs([]); };
  const rejectAll = () => { setDiffState('rejected'); setDiffs([]); };

  const pendingDiffForEditor = (diffState === 'open' && activeFile === 'spec') ? {
    onAccept: acceptDiff,
    onReject: rejectDiff,
  } : null;

  return (
    <div className={`app-shell ${dark ? 'theme-dark' : ''}`}>
      <TitleBar path={M.WS_PATH} file={tabs.find(t => t.id === activeFile)?.name || ''} mode={chatMode} dark={dark} />
      <div className="work">
        <LeftRail active={activeModule} onChange={setActiveModule} />
        <div className="side">
          <SidePanelRouter
            activeModule={activeModule}
            activeFile={activeFile} onPickFile={pickFile}
            currentTimelineNode={tlNode} onPickTimeline={setTlNode}
          />
        </div>
        <div className="center">
          <EditorTabs tabs={tabs} activeId={activeFile} onPick={setActiveFile} onClose={closeTab} />
          <EditorToolbar zoom={zoom} onZoom={setZoom} />
          <EditorBody activeFile={activeFile} pendingDiff={pendingDiffForEditor} />
          {assistOpen && <InlineAssist onClose={() => setAssistOpen(false)} />}
        </div>
        <ChatPanel
          mode={chatMode} setMode={setChatMode}
          tabs={chatTabs} activeTabId={activeChatTab} setActiveTabId={setActiveChatTab}
          messages={messages}
          diffState={diffState}
          onAcceptDiff={acceptDiff} onRejectDiff={rejectDiff}
          composerRefs={[{type:'file', text:'spec.md · §4.7'}, {type:'kn', text:'KN · tauri-commands'}]}
          onNewTab={() => {}}
        />
      </div>
      <DiffStrip diffs={diffs}
        onAcceptAll={acceptAll} onRejectAll={rejectAll}
        onPick={(d) => { setActiveFile(d.file === 'README.md' ? 'readme' : 'spec'); }}
      />
      <StatusBar pendingCount={diffs.length} mode={chatMode} />
    </div>
  );
}

function findNode(list, id) {
  for (const n of list) {
    if (n.id === id) return n;
    if (n.children) {
      const r = findNode(n.children, id);
      if (r) return r;
    }
  }
  return null;
}

window.BinderApp = BinderApp;
window.BinderWelcome = Welcome;
