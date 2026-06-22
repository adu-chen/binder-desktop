/* binder-chat.jsx — Right side chat: header, tabs, messages, tool cards, diff card, composer */

const { useState: useStateC, useRef: useRefC, useEffect: useEffectC } = React;

/* ─── Header with mode toggle + chat tabs ─── */
function ChatHeader({ mode, onMode, tabs, activeTabId, onPickTab, onNewTab }) {
  return (
    <div className="chat-hd">
      <div className="mode-toggle">
        <button className={mode === 'chat' ? 'active' : ''} onClick={() => onMode('chat')}>Chat</button>
        <button className={mode === 'agent' ? 'active' : ''} onClick={() => onMode('agent')}>Agent</button>
      </div>
      <div className="chat-tabs">
        {tabs.map(t => (
          <span key={t.id}
            className={`chat-tab ${t.id === activeTabId ? 'active' : ''}`}
            onClick={() => onPickTab(t.id)}
          >
            {t.name}
          </span>
        ))}
      </div>
      <button className="chat-add" onClick={onNewTab} title="新建对话">+</button>
    </div>
  );
}

/* ─── Message bits ─── */
function ChipList({ refs }) {
  if (!refs || !refs.length) return null;
  return (
    <div className="chips">
      {refs.map((r, i) => {
        const cls = r.type === 'kn' ? 'kn' : r.type === 'mem' ? 'mem' : 'file';
        const glyph = r.type === 'kn' ? '◐' : r.type === 'mem' ? '◇' : '▤';
        return <span key={i} className={`chip ${cls}`}><span className="gl">{glyph}</span>{r.text}</span>;
      })}
    </div>
  );
}

function ToolCall({ tool }) {
  return (
    <div className="toolcall">
      <div className="toolcall-hd">
        <span className="toolcall-name"><span className="glyph">▸</span>{tool.name}</span>
        <span className={`toolcall-status ${tool.status !== 'done' ? tool.status : ''}`}>
          {tool.status === 'done' ? 'OK' : tool.status === 'pending' ? 'PENDING…' : 'FAIL'}
        </span>
      </div>
      <div className="toolcall-body">
        {Object.entries(tool.args).map(([k, v]) => (
          <div className="toolcall-arg" key={k}>
            <span className="k">{k}</span><span className="v">{String(v)}</span>
          </div>
        ))}
      </div>
      {tool.status === 'pending' && (
        <div className="toolcall-foot">
          <button>授权</button>
          <button>跳过</button>
          <span style={{marginLeft:'auto', color:'var(--ink-5)'}}>等待用户确认</span>
        </div>
      )}
    </div>
  );
}

function DiffCard({ diff, onAccept, onReject, accepted, rejected }) {
  return (
    <div className="diff-card" style={{
      borderColor: accepted ? 'var(--forest)' : rejected ? 'var(--ink-5)' : 'var(--rust)',
      opacity: rejected ? 0.55 : 1,
    }}>
      <div className="diff-card-hd">
        <span className="name">BINDER · <b>{diff.hunks} HUNK</b></span>
        <span className="scope">{diff.file} · <span style={{color:'var(--forest)'}}>+{diff.added}</span> · <span style={{color:'var(--rust)'}}>−{diff.removed}</span></span>
      </div>
      <div className="diff-card-body">
        <div className="diff-row del">
          <span className="gutter">142</span><span className="mark">−</span>
          <span className="content">聊天有两种模式，agent 启用工具。</span>
        </div>
        <div className="diff-row add">
          <span className="gutter">142</span><span className="mark">+</span>
          <span className="content">聊天当前有 chat 与 agent 两种模式，模式边界由 tab 元数据控制。</span>
        </div>
        <div className="diff-row add">
          <span className="gutter">143</span><span className="mark">+</span>
          <span className="content">　· chat 按纯对话主链运行，不启用任何工具。</span>
        </div>
      </div>
      <div style={{padding:'6px 12px', borderTop:'0.5px solid var(--rule-faint)', fontFamily:'var(--font-mono)', fontSize:10, letterSpacing:'0.06em', color:'var(--ink-4)', textTransform:'uppercase'}}>
        {accepted ? '已接受 · 已写入 spec.md' : rejected ? '已拒绝 · 已记入时间轴' : `落点 · ${diff.scope}`}
      </div>
      {!accepted && !rejected && (
        <div className="diff-card-actions">
          <button className="accept" onClick={onAccept}>接受 ⏎</button>
          <button className="reject" onClick={onReject}>拒绝 ⌫</button>
        </div>
      )}
    </div>
  );
}

function Message({ m, diffState, onAccept, onReject }) {
  const isUser = m.kind === 'user';
  return (
    <div className={`msg ${isUser ? 'user' : 'assistant'}`}>
      <div className="msg-hd">
        <span className="who">{m.who}{m.mode ? ` · ${m.mode}` : ''}</span>
        <span className="when">{m.when}</span>
      </div>
      <div className="msg-body">
        {m.body.split('\n').map((p, i) => <p key={i} dangerouslySetInnerHTML={{__html: p
          .replace(/`([^`]+)`/g, '<code>$1</code>')
          .replace(/<u>([^<]+)<\/u>/g, '<u>$1</u>')
        }} />)}
        <ChipList refs={m.refs} />
        {m.tools && m.tools.map((t, i) => <ToolCall key={i} tool={t} />)}
        {m.diff && (
          <DiffCard diff={m.diff}
            onAccept={onAccept} onReject={onReject}
            accepted={diffState === 'accepted'} rejected={diffState === 'rejected'} />
        )}
      </div>
    </div>
  );
}

/* ─── Composer ─── */
function ChatComposer({ mode, model, refs, onSend, value, setValue }) {
  return (
    <div className="composer">
      <div className="composer-row1">
        <span>{mode === 'agent' ? 'AGENT · 可调用 11 工具 · 落点 spec.md' : 'CHAT · 纯对话'}</span>
      </div>
      {refs && refs.length > 0 && (
        <div className="chips" style={{marginBottom:8}}>
          {refs.map((r, i) => {
            const cls = r.type === 'kn' ? 'kn' : r.type === 'mem' ? 'mem' : 'file';
            return <span key={i} className={`chip ${cls}`}><span className="gl">▤</span>{r.text} ×</span>;
          })}
        </div>
      )}
      <div className="composer-input">
        <textarea
          placeholder={mode === 'agent' ? '描述你希望改动的内容，或拖入文件/记忆/知识引用…' : '提问或讨论，不会调用工具…'}
          value={value}
          onChange={e => setValue(e.target.value)}
          onKeyDown={e => {
            if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) { e.preventDefault(); onSend(); }
          }}
        />
        <div className="composer-actions">
          <div style={{display:'flex', gap:6, alignItems:'center'}}>
            <span className="composer-model">▾ {model}</span>
            <span className="kbd">@ 引用</span>
            <span className="kbd">/ 命令</span>
          </div>
          <div style={{display:'flex', gap:6, alignItems:'center'}}>
            <span className="kbd">⌘⏎</span>
            <button className={`composer-send ${mode === 'agent' ? 'rust' : ''} ${!value ? 'disabled' : ''}`} onClick={onSend}>
              {mode === 'agent' ? '执行' : '发送'}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

/* ─── Whole chat panel ─── */
function ChatPanel({
  mode, setMode,
  tabs, activeTabId, setActiveTabId,
  messages,
  diffState, onAcceptDiff, onRejectDiff,
  composerRefs,
  model = 'deepseek-chat',
  onNewTab,
}) {
  const [v, setV] = useStateC('');
  const bodyRef = useRefC(null);
  const sectLabel = mode === 'agent' ? 'CHAT · AGENT' : 'CHAT';
  useEffectC(() => {
    if (bodyRef.current) bodyRef.current.scrollTop = bodyRef.current.scrollHeight;
  }, [messages.length, diffState]);

  return (
    <div className="chat">
      <ChatHeader
        mode={mode} onMode={setMode}
        tabs={tabs} activeTabId={activeTabId} onPickTab={setActiveTabId}
        onNewTab={onNewTab}
      />
      <div style={{
        padding: '8px 14px 6px',
        fontFamily: 'var(--font-mono)', fontSize: 10, letterSpacing: '0.08em',
        color: 'var(--ink-4)', textTransform: 'uppercase',
        borderBottom: '0.5px solid var(--rule-faint)',
        display:'flex', justifyContent:'space-between'
      }}>
        <span>{sectLabel}</span>
        <span>会话 · {tabs.find(t => t.id === activeTabId)?.name || ''}</span>
      </div>
      <div className="chat-body" ref={bodyRef}>
        {messages.map((m, i) => (
          <Message key={i} m={m}
            diffState={diffState}
            onAccept={onAcceptDiff} onReject={onRejectDiff}
          />
        ))}
      </div>
      <ChatComposer
        mode={mode} model={model}
        refs={composerRefs}
        value={v} setValue={setV}
        onSend={() => { setV(''); }}
      />
    </div>
  );
}

window.BinderChat = { ChatPanel };
