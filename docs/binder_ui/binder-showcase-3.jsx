/* binder-showcase-3.jsx — Flows · Empty/Loading/Error · Design Tokens */

const { Page, ShowFrame } = window.BinderShowcase;

/* tiny editor + chat mock used inside flow frames */
function MiniLayout({ children }) {
  return (
    <div style={{
      display:'grid', gridTemplateColumns:'130px 1fr 200px',
      border:'0.5px solid var(--rule)', background:'var(--bg-elevated)',
      height: 280, fontSize: 11, overflow:'hidden',
    }}>{children}</div>
  );
}
function MiniSide() {
  return (
    <div style={{borderRight:'0.5px solid var(--rule-faint)', padding:'10px 10px', background:'var(--bg-elevated)'}}>
      <div className="t-label" style={{fontSize:9, marginBottom:6}}>I·A · WORKSPACE</div>
      {[
        ['▾ docs', false, false],
        ['  spec.md', true, true],
        ['  intro.md', false, false],
        ['  README.md', false, false],
        ['▸ assets', false, false],
      ].map(([n, active, dirty], i) => (
        <div key={i} style={{
          padding:'3px 6px', fontFamily:'var(--font-mono)', fontSize:11,
          color: active ? 'var(--ink)' : 'var(--ink-3)',
          background: active ? 'var(--bg-card)' : 'transparent',
          borderLeft: active ? '2px solid var(--rust)' : '2px solid transparent',
          fontWeight: active ? 600 : 400,
          display:'flex', alignItems:'center', gap:6,
        }}>
          <span>{n}</span>
          {dirty && <span style={{marginLeft:'auto', color:'var(--rust)', fontSize:9}}>●</span>}
        </div>
      ))}
    </div>
  );
}
function MiniEditor({ banner, blocks = [], dialog }) {
  return (
    <div style={{borderRight:'0.5px solid var(--rule-faint)', display:'flex', flexDirection:'column', position:'relative', minWidth:0}}>
      <div style={{display:'flex', borderBottom:'0.5px solid var(--rule-faint)', background:'var(--bg-elevated)', height:24}}>
        <div className="tab-chip active" style={{borderRadius:0, border:0, padding:'4px 12px', height:24, alignItems:'center', display:'flex', gap:6, fontSize:10}}>
          <span className="dot" /><span>spec.md</span>
        </div>
        <div className="tab-chip" style={{borderRadius:0, border:0, padding:'4px 12px', height:24, alignItems:'center', display:'flex', gap:6, fontSize:10, color:'var(--ink-4)'}}>
          <span className="dot clean" /><span>intro.md</span>
        </div>
      </div>
      {banner}
      <div style={{flex:1, padding:'10px 14px', background:'var(--bg-page)', overflow:'hidden', fontSize:11, color:'var(--ink-2)'}}>
        <div style={{fontFamily:'var(--font-sans)', fontWeight:600, fontSize:13, marginBottom:6}}>§4.7 · 聊天模式与工具调用</div>
        {blocks.length === 0
          ? <p style={{margin:0, color:'var(--ink-3)', lineHeight:1.55}}>聊天的两种模式由 tab 元数据控制 …</p>
          : blocks
        }
      </div>
      {dialog}
    </div>
  );
}
function MiniChat({ children }) {
  return (
    <div style={{display:'flex', flexDirection:'column', minWidth:0}}>
      <div style={{padding:'6px 10px', borderBottom:'0.5px solid var(--rule-faint)', display:'flex', gap:6, alignItems:'center'}}>
        <span style={{padding:'2px 6px', background:'var(--ink)', color:'var(--bg-page)', fontFamily:'var(--font-mono)', fontSize:9, fontWeight:700, letterSpacing:'0.06em'}}>AGENT</span>
        <span className="t-label" style={{fontSize:9}}>SPEC.MD · DIFF</span>
      </div>
      <div style={{flex:1, padding:'10px', display:'flex', flexDirection:'column', gap:6, overflow:'hidden', fontSize:11}}>
        {children}
      </div>
    </div>
  );
}
function MiniMsg({ who, text, kind = 'asst', dim }) {
  return (
    <div style={{opacity: dim ? 0.55 : 1, padding: kind === 'user' ? '6px 8px' : 0, background: kind === 'user' ? 'var(--bg-card)' : 'transparent', borderLeft: kind === 'user' ? '2px solid var(--ink)' : 'none'}}>
      <div className="t-label" style={{fontSize:8, marginBottom:2}}>{who}</div>
      <div style={{fontSize:11, color:'var(--ink-2)', lineHeight:1.45}}>{text}</div>
    </div>
  );
}

/* ─────────── Chain A: AI 修改当前文档 ─────────── */
function FlowA() {
  return (
    <Page code="07·A" name="链路 A · AI 修改当前文档（核心主链）" sub="5 帧 · M4 + Diff Review">
      <div className="frame-strip" style={{gridTemplateColumns: 'repeat(5, 1fr)'}}>
        {/* 1 */}
        <div className="frame-strip-cell">
          <div className="frame-strip-cell-hd"><b>① 用户发送指令</b><span>idle</span></div>
          <div className="frame-strip-cell-body">
            <MiniLayout>
              <MiniSide />
              <MiniEditor />
              <MiniChat>
                <MiniMsg who="YOU · 14:32" kind="user" text="把 §4.7 的工具矩阵按类别分组，并强调 chat 模式不启用工具。" />
              </MiniChat>
            </MiniLayout>
            <div className="t-label">输入框包含 @spec.md · §4.7 引用</div>
          </div>
        </div>
        {/* 2 */}
        <div className="frame-strip-cell">
          <div className="frame-strip-cell-hd"><b>② AI 思考</b><span>thinking</span></div>
          <div className="frame-strip-cell-body">
            <MiniLayout>
              <MiniSide />
              <MiniEditor />
              <MiniChat>
                <MiniMsg who="YOU · 14:32" kind="user" text="把 §4.7 的工具矩阵按类别分组…" dim />
                <div style={{color:'var(--ink-3)', display:'flex', gap:6, alignItems:'center'}}>
                  <span className="t-label" style={{fontSize:8}}>BINDER</span>
                  <span className="thinking-dots"><span></span><span></span><span></span></span>
                </div>
              </MiniChat>
            </MiniLayout>
            <div className="t-label">输入框禁用 · ESC 可打断</div>
          </div>
        </div>
        {/* 3 */}
        <div className="frame-strip-cell">
          <div className="frame-strip-cell-hd"><b>③ 工具调用</b><span>tool_calling</span></div>
          <div className="frame-strip-cell-body">
            <MiniLayout>
              <MiniSide />
              <MiniEditor />
              <MiniChat>
                <MiniMsg who="BINDER · 14:33" text="读取 §4.7 与工具注册表，按类别重排。" dim />
                <div className="toolcall">
                  <div className="toolcall-hd">
                    <span className="toolcall-name"><span className="glyph">▸</span>read_file</span>
                    <span className="toolcall-status">OK</span>
                  </div>
                </div>
                <div className="toolcall">
                  <div className="toolcall-hd">
                    <span className="toolcall-name"><span className="glyph">▸</span>edit_current…</span>
                    <span className="toolcall-status pending"><span className="spin" style={{marginRight:4}} />EXEC</span>
                  </div>
                  <div className="bar-track"><div className="bar-fill" /></div>
                </div>
              </MiniChat>
            </MiniLayout>
          </div>
        </div>
        {/* 4 */}
        <div className="frame-strip-cell">
          <div className="frame-strip-cell-hd"><b>④ Green Review</b><span>reviewing</span></div>
          <div className="frame-strip-cell-body">
            <MiniLayout>
              <MiniSide />
              <MiniEditor blocks={[
                <div key="b" className="gr-block" style={{margin:'4px 0'}}>
                  <div className="gr-block-hd"><span className="id">¶142 · PENDING</span><span>+11/−6</span></div>
                  <div><span className="del" style={{fontSize:10}}>(原文一长串)</span></div>
                  <div><span className="add" style={{fontSize:10}}>(按类别分组的新版)</span></div>
                </div>
              ]} />
              <MiniChat>
                <div className="dc is-pending">
                  <div className="dc-hd"><span className="who">¶142</span><span className="badge rust">PENDING</span></div>
                  <div className="dc-actions">
                    <button className="accept">接受</button>
                    <button className="reject">拒绝</button>
                  </div>
                </div>
              </MiniChat>
            </MiniLayout>
            <div className="t-label">编辑器 ↔ Diff Card 一一对应</div>
          </div>
        </div>
        {/* 5 */}
        <div className="frame-strip-cell">
          <div className="frame-strip-cell-hd"><b>⑤ 接受写入</b><span>accepted</span></div>
          <div className="frame-strip-cell-body">
            <MiniLayout>
              <MiniSide />
              <MiniEditor blocks={[
                <p key="p" style={{margin:0, fontSize:11, color:'var(--ink-2)', lineHeight:1.55}}>
                  按类别分组：读取 · 资源结构 · 写入 · 依赖。<br/>
                  <span style={{color:'var(--ink-3)'}}>chat 模式不启用任何工具。</span>
                </p>
              ]} />
              <MiniChat>
                <div className="dc is-accepted">
                  <div className="dc-hd"><span className="who">¶142</span><span className="badge forest">ACCEPTED</span></div>
                </div>
                <div className="t-label" style={{fontSize:8, marginTop:4}}>状态栏 · ⌘S 已自动保存</div>
              </MiniChat>
            </MiniLayout>
          </div>
        </div>
      </div>
    </Page>
  );
}

/* ─────────── Chain B: 批量接受 (3 frames) ─────────── */
function FlowB() {
  return (
    <Page code="07·B" name="链路 B · 全部接受 · 倒序执行" sub="3 帧 · DiffAllActionsBar">
      <div className="frame-strip" style={{gridTemplateColumns: 'repeat(3, 1fr)'}}>
        <div className="frame-strip-cell">
          <div className="frame-strip-cell-hd"><b>① 3 条 Diff 同时存在</b><span>pending</span></div>
          <div className="frame-strip-cell-body">
            <div style={{display:'flex', flexDirection:'column', gap:6}}>
              {[1,2,3].map(i => <div key={i} className="gr-block" style={{margin:0}}><div className="gr-block-hd"><span className="id">¶{142 + i*30} · PENDING</span><span>+{i*2}/−1</span></div></div>)}
            </div>
            <div className="diffstrip" style={{height:36, marginTop:'auto'}}>
              <div className="diffstrip-label"><span className="num">3</span><span>待决策</span></div>
              <div style={{flex:1}} />
              <div className="diffstrip-actions"><button className="diffstrip-btn danger">全部拒绝</button><button className="diffstrip-btn primary">全部接受</button></div>
            </div>
          </div>
        </div>
        <div className="frame-strip-cell">
          <div className="frame-strip-cell-hd"><b>② 倒序应用中</b><span>applying</span></div>
          <div className="frame-strip-cell-body">
            <div style={{display:'flex', flexDirection:'column', gap:6}}>
              <div className="gr-block" style={{margin:0, opacity:0.5}}><div className="gr-block-hd"><span className="id">¶142 · 等待</span></div></div>
              <div className="gr-block applying" style={{margin:0}}><div className="gr-block-hd"><span className="id" style={{color:'var(--gold)'}}>¶172 · APPLYING <span className="spin" style={{marginLeft:4}} /></span></div></div>
              <div className="gr-block accepted" style={{margin:0}}><div className="gr-block-hd"><span className="id">¶202 · ✓ ACCEPTED</span></div></div>
            </div>
            <div className="diffstrip" style={{height:36, marginTop:'auto'}}>
              <div className="diffstrip-label"><span className="num" style={{background:'var(--gold)'}}>2/3</span><span>倒序应用中 <span className="spin" /></span></div>
            </div>
            <div className="t-label">从文档靠后的 Diff 开始 · 避免位置偏移</div>
          </div>
        </div>
        <div className="frame-strip-cell">
          <div className="frame-strip-cell-hd"><b>③ 全部完成 · 含失效</b><span>mixed</span></div>
          <div className="frame-strip-cell-body">
            <div style={{display:'flex', flexDirection:'column', gap:6}}>
              <div className="gr-block expired" style={{margin:0}}><div className="gr-block-hd"><span className="id" style={{color:'var(--ink-4)'}}>¶142 · EXPIRED</span><span>原文已被手动编辑</span></div></div>
              <div className="gr-block accepted" style={{margin:0}}><div className="gr-block-hd"><span className="id">¶172 · ✓ ACCEPTED</span></div></div>
              <div className="gr-block accepted" style={{margin:0}}><div className="gr-block-hd"><span className="id">¶202 · ✓ ACCEPTED</span></div></div>
            </div>
            <div className="diffstrip" style={{height:36, marginTop:'auto', background:'var(--forest-bg)', borderColor:'var(--forest)'}}>
              <div className="diffstrip-label"><span className="num" style={{background:'var(--forest)'}}>✓</span><span>2 接受 · 1 失效</span></div>
              <div style={{flex:1}} />
              <div className="diffstrip-actions"><button className="diffstrip-btn">关闭</button></div>
            </div>
          </div>
        </div>
      </div>
    </Page>
  );
}

/* ─────────── Chain C: Diff 失效 ─────────── */
function FlowC() {
  return (
    <Page code="07·C" name="链路 C · Diff 失效处理" sub="3 帧 · expired · 原文被手动编辑">
      <div className="frame-strip" style={{gridTemplateColumns: 'repeat(3, 1fr)'}}>
        <div className="frame-strip-cell">
          <div className="frame-strip-cell-hd"><b>① Diff 生成 · 待决策</b><span>pending</span></div>
          <div className="frame-strip-cell-body">
            <div className="gr-block"><div className="gr-block-hd"><span className="id">¶142 · PENDING</span><span>+11/−6</span></div>
              <div><span className="del">chat 启用工具…</span></div><div><span className="add">chat 不启用任何工具。</span></div>
            </div>
            <div className="dc is-pending">
              <div className="dc-hd"><span className="who">¶142 · spec.md</span><span className="badge rust">PENDING</span></div>
              <div className="dc-actions"><button className="accept">接受</button><button className="reject">拒绝</button></div>
            </div>
          </div>
        </div>
        <div className="frame-strip-cell">
          <div className="frame-strip-cell-hd"><b>② 用户手动编辑了 ¶142</b><span>document_revision_advanced</span></div>
          <div className="frame-strip-cell-body">
            <div className="gr-block applying"><div className="gr-block-hd"><span className="id" style={{color:'var(--gold)'}}>¶142 · 检测变更…</span></div>
              <div style={{color:'var(--ink-2)', fontSize:12}}>chat 启用工具 <span className="caret" /></div>
            </div>
            <div className="t-label">documentRevision 增加 · 校验失败</div>
          </div>
        </div>
        <div className="frame-strip-cell">
          <div className="frame-strip-cell-hd"><b>③ Diff 自动 expired</b><span>expired</span></div>
          <div className="frame-strip-cell-body">
            <div className="gr-block expired"><div className="gr-block-hd"><span className="id" style={{color:'var(--ink-4)'}}>¶142 · EXPIRED</span></div></div>
            <div className="dc is-expired">
              <div className="dc-hd"><span className="who">¶142 · spec.md</span><span className="badge outline">EXPIRED</span></div>
              <div className="dc-notice">
                <span className="glyph gold">!</span>
                <div><b style={{color:'var(--ink)'}}>原文已被手动编辑</b><div style={{marginTop:2, color:'var(--ink-3)'}}>该建议已失效，如需继续修改请重新向 AI 发起请求。</div></div>
              </div>
              <div className="dc-actions"><button className="retry">重新生成</button><button>关闭</button></div>
            </div>
          </div>
        </div>
      </div>
    </Page>
  );
}

/* ─────────── Chain D: 外部文件变更 ─────────── */
function FlowD() {
  return (
    <Page code="07·D" name="链路 D · 外部文件被修改" sub="3 帧 · Banner · external_file_changed">
      <div className="frame-strip" style={{gridTemplateColumns: 'repeat(3, 1fr)'}}>
        <div className="frame-strip-cell">
          <div className="frame-strip-cell-hd"><b>① 外部程序保存了同一文件</b><span>detected</span></div>
          <div className="frame-strip-cell-body">
            <div className="t-label">每 5s 检测一次 mtime</div>
            <div style={{display:'flex', alignItems:'center', gap:10, padding:'10px 12px', background:'var(--bg-card)', border:'0.5px solid var(--rule)', fontSize:12, color:'var(--ink-3)'}}>
              <span className="spin" /> 正在比对 <b style={{color:'var(--ink)'}}>spec.md</b> mtime…
            </div>
          </div>
        </div>
        <div className="frame-strip-cell">
          <div className="frame-strip-cell-hd"><b>② Banner 询问</b><span>banner</span></div>
          <div className="frame-strip-cell-body">
            <MiniLayout>
              <MiniSide />
              <MiniEditor banner={
                <div className="banner">
                  <span className="glyph">!</span>
                  <div><b style={{color:'var(--ink)'}}>此文件已在外部被修改</b><div style={{fontSize:11, color:'var(--ink-3)'}}>VS Code 在 14:36 修改了 spec.md。</div></div>
                  <div className="actions">
                    <button>保留当前编辑</button>
                    <button className="primary">加载外部更改</button>
                  </div>
                </div>
              } />
              <MiniChat>
                <div className="dc is-pending"><div className="dc-hd"><span className="who">¶142</span><span className="badge rust">PENDING</span></div></div>
                <div className="t-label" style={{fontSize:8}}>Pending Diff 暂未失效</div>
              </MiniChat>
            </MiniLayout>
          </div>
        </div>
        <div className="frame-strip-cell">
          <div className="frame-strip-cell-hd"><b>③ 选择"加载外部更改"</b><span>reloaded</span></div>
          <div className="frame-strip-cell-body">
            <div className="t-label">所有 Pending Diff 静默失效</div>
            <div className="dc is-expired">
              <div className="dc-hd"><span className="who">¶142 · spec.md</span><span className="badge outline">EXPIRED</span></div>
              <div className="dc-notice">
                <span className="glyph gold">!</span>
                <div><b style={{color:'var(--ink)'}}>文件已在磁盘上被外部修改</b><div className="t-code" style={{fontSize:10, color:'var(--ink-4)', marginTop:2}}>external_file_changed</div></div>
              </div>
              <div className="dc-actions"><button className="retry">重新生成</button><button>关闭</button></div>
            </div>
          </div>
        </div>
      </div>
    </Page>
  );
}

/* ─────────── Chain F: Cmd+K (already similar) — keep as flow ─────────── */
function FlowF() {
  return (
    <Page code="07·F" name="链路 F · 局部修改 · ⌘K Inline Assist" sub="4 帧 · 不进入 Diff 链路">
      <div className="frame-strip" style={{gridTemplateColumns: 'repeat(4, 1fr)'}}>
        {[
          { hd:'① 选中文本 + ⌘K', s:'input', body: <InlineAssistMini stage="input" /> },
          { hd:'② AI 生成中',    s:'generating', body: <InlineAssistMini stage="generating" /> },
          { hd:'③ 建议预览 / 追问', s:'preview', body: <InlineAssistMini stage="preview" /> },
          { hd:'④ Apply · 直接替换', s:'applied', body: <InlineAssistMini stage="applied" /> },
        ].map((f, i) => (
          <div key={i} className="frame-strip-cell">
            <div className="frame-strip-cell-hd"><b>{f.hd}</b><span>{f.s}</span></div>
            <div className="frame-strip-cell-body">{f.body}</div>
          </div>
        ))}
      </div>
    </Page>
  );
}
function InlineAssistMini({ stage }) {
  return (
    <div style={{position:'relative', background:'var(--bg-card)', padding:12, border:'0.5px solid var(--rule)', minHeight:200}}>
      <div className="doc">
        <p style={{margin:0, fontSize:11, lineHeight:1.55}}>
          <span style={{background:'rgba(194,74,42,0.15)', padding:'0 2px'}}>聊天有两种模式，agent 启用工具。</span>
        </p>
      </div>
      <div className="assist" style={{position:'absolute', left:8, top:42, width:'calc(100% - 16px)'}}>
        {stage === 'input' && <>
          <div className="assist-hd"><span>⌘K</span><span>§4.7 · 1 行</span></div>
          <div className="assist-input"><textarea defaultValue="改写得更书面化" /></div>
          <div className="assist-foot"><span>ESC 取消</span><button>⏎</button></div>
        </>}
        {stage === 'generating' && <>
          <div className="assist-hd"><span>⌘K · 生成中</span><span><span className="spin" /></span></div>
          <div style={{padding:10, fontSize:11, color:'var(--ink-2)'}}>聊天的两种模式由 tab 元数据控制 <span className="thinking-dots"><span></span><span></span><span></span></span></div>
          <div className="bar-track"><div className="bar-fill" /></div>
        </>}
        {stage === 'preview' && <>
          <div className="assist-hd"><span>⌘K · 预览</span><span>BEFORE / AFTER</span></div>
          <div style={{padding:10}}>
            <div className="diff-row del" style={{fontSize:10}}><span className="gutter">142</span><span className="mark">−</span><span className="content">聊天有两种模式，agent 启用工具。</span></div>
            <div className="diff-row add" style={{fontSize:10}}><span className="gutter">142</span><span className="mark">+</span><span className="content">聊天的两种模式由 tab 元数据控制：chat 不启用工具…</span></div>
          </div>
          <div className="assist-foot"><span>继续追问</span><span>取消</span><button style={{color:'var(--forest)'}}>Apply ⏎</button></div>
        </>}
        {stage === 'applied' && <>
          <div style={{padding:'8px 12px', background:'var(--forest-bg)', borderLeft:'3px solid var(--forest)', fontSize:11, color:'var(--ink)', display:'flex', alignItems:'center', gap:8}}>
            <span style={{color:'var(--forest)', fontWeight:700}}>✓</span> 已替换 · 不进入 Diff 链路
          </div>
        </>}
      </div>
    </div>
  );
}

/* ─────────── Chain G: 关闭警告 ─────────── */
function FlowG() {
  return (
    <Page code="07·G" name="链路 G · 关闭应用前 Pending Diff 警告" sub="保护未决策状态">
      <div style={{display:'grid', gridTemplateColumns:'1fr', gap:14, maxWidth:760, margin:'0 auto'}}>
        <ShowFrame code="G" name="确认对话框" hint="MainLayout · onCloseRequested">
          <div style={{position:'relative', minHeight:260, background:'var(--bg-page)', border:'0.5px solid var(--rule)'}}>
            {/* fake backdrop showing app dimmed */}
            <div style={{position:'absolute', inset:0, padding:14, opacity:0.4, pointerEvents:'none', filter:'blur(0.4px)'}}>
              <MiniLayout>
                <MiniSide />
                <MiniEditor />
                <MiniChat>
                  <div className="dc is-pending"><div className="dc-hd"><span className="who">¶142</span><span className="badge rust">PENDING</span></div></div>
                </MiniChat>
              </MiniLayout>
            </div>
            <div className="dialog-mask">
              <div className="dialog">
                <div className="dialog-hd">
                  <div className="code">⚠ CLOSE WARNING</div>
                  <div className="title">你有 <span style={{color:'var(--rust)'}}>5 条</span> 未决策的 AI 修改建议</div>
                </div>
                <div className="dialog-body">
                  关闭后这些建议将丢失。重启后 blockId 可能变化，<b style={{color:'var(--ink)'}}>无法保证还能恢复到当前 Diff</b>。
                  <div className="keypoints">
                    <div className="t-label" style={{marginBottom:4}}>当前 PENDING 分布</div>
                    <div style={{fontFamily:'var(--font-mono)', fontSize:11, color:'var(--ink-3)', lineHeight:1.7}}>
                      spec.md · §4.7 · §3.4 · §2.1 <span style={{color:'var(--rust)'}}>(3)</span><br/>
                      README.md · head <span style={{color:'var(--rust)'}}>(1)</span><br/>
                      workflows/release.md · § Checklist <span style={{color:'var(--rust)'}}>(1)</span>
                    </div>
                  </div>
                </div>
                <div className="dialog-actions">
                  <button className="primary">先处理 Diff</button>
                  <button className="danger">放弃 Diff 并关闭</button>
                </div>
              </div>
            </div>
          </div>
        </ShowFrame>
      </div>
    </Page>
  );
}

/* ─────────── Chain H: 危险操作授权 ─────────── */
function FlowH() {
  return (
    <Page code="07·H" name="链路 H · 危险工具的授权门" sub="delete_file · move_file · rename_file …">
      <div className="frame-strip" style={{gridTemplateColumns: '1fr 1fr'}}>
        <div className="frame-strip-cell">
          <div className="frame-strip-cell-hd"><b>① AI 请求执行 delete_file</b><span>awaiting</span></div>
          <div className="frame-strip-cell-body">
            <MiniLayout>
              <MiniSide />
              <MiniEditor />
              <MiniChat>
                <MiniMsg who="BINDER · 14:40" text="为清理 merge 后的废弃稿，我需要删除一个文件。" />
                <div className="toolcall" style={{borderColor:'var(--rust)'}}>
                  <div className="toolcall-hd">
                    <span className="toolcall-name"><span className="glyph" style={{color:'var(--rust)'}}>!</span>delete_file</span>
                    <span className="toolcall-status" style={{color:'var(--rust)'}}>AUTH</span>
                  </div>
                  <div className="toolcall-body">
                    <div className="toolcall-arg"><span className="k">path</span><span className="v">workflows/old-draft.md</span></div>
                  </div>
                  <div className="toolcall-foot">
                    <span style={{color:'var(--ink-3)'}}>需要授权才能继续</span>
                    <button style={{marginLeft:'auto'}}>跳过</button>
                    <button style={{color:'var(--forest)', fontWeight:700}}>授权 ⏎</button>
                  </div>
                </div>
              </MiniChat>
            </MiniLayout>
          </div>
        </div>
        <div className="frame-strip-cell">
          <div className="frame-strip-cell-hd"><b>② 授权后继续</b><span>continued</span></div>
          <div className="frame-strip-cell-body">
            <MiniLayout>
              <MiniSide />
              <MiniEditor />
              <MiniChat>
                <div className="toolcall">
                  <div className="toolcall-hd">
                    <span className="toolcall-name"><span className="glyph">▸</span>delete_file</span>
                    <span className="toolcall-status">OK · 142 ms</span>
                  </div>
                  <div className="toolcall-body">
                    <div className="toolcall-arg"><span className="k">path</span><span className="v">workflows/old-draft.md</span></div>
                    <div className="toolcall-arg"><span className="k">result</span><span className="v" style={{color:'var(--forest)'}}>已删除 · 已记入时间轴 #1294</span></div>
                  </div>
                </div>
                <MiniMsg who="BINDER · 14:40" text="已删除。是否继续重命名相关引用？" />
              </MiniChat>
            </MiniLayout>
            <div className="t-label">用户拒绝 → AI 收到 cancellation，决定下一步</div>
          </div>
        </div>
      </div>
    </Page>
  );
}

/* ─────────── Empty / Loading / Error ─────────── */
function EmptyLoadingErrorPage() {
  return (
    <Page code="08" name="空态 · 加载态 · 错误态" sub="STATE 8.x · 必须全覆盖">

      <div className="page-hd-l" style={{padding:'4px 0 10px'}}>
        <span style={{fontFamily:'var(--font-mono)', fontSize:10, color:'var(--rust)', letterSpacing:'0.12em', textTransform:'uppercase', fontWeight:600}}>08·A 空态</span>
      </div>
      <div className="grid grid-3">
        <ShowFrame code="A·1" name="未打开工作区" hint="编辑区 · 引导打开">
          <EmptyState
            glyph="▢"
            title="选择一个目录开始"
            body={'Binder 把一份份零散的文档「装订」在同一个本地工作区里。'}
            actions={[['打开工作区…','primary'], ['新建目录',null]]}
          />
        </ShowFrame>
        <ShowFrame code="A·2" name="工作区无文件" hint="文件树空 + 编辑区空">
          <EmptyState
            glyph="¶"
            title="这里还什么都没有"
            body="创建第一份文档，或拖拽文件进来。"
            actions={[['新建 .md','primary'], ['从模板创建',null], ['拖拽导入',null]]}
          />
        </ShowFrame>
        <ShowFrame code="A·3" name="聊天无历史" hint="右侧空态">
          <EmptyState
            small
            glyph="¶"
            title="还没有对话"
            body="把指令、引用或 @文件拖到下方，开始第一次 Agent 协作。"
            actions={[['插入 @ 引用',null], ['选择模型',null]]}
          />
        </ShowFrame>
        <ShowFrame code="A·4" name="搜索无结果" hint="搜索面板">
          <div style={{padding:12, background:'var(--bg-card)', border:'0.5px solid var(--rule)'}}>
            <input className="t-code" style={{width:'100%', padding:6, border:'0.5px solid var(--rule)', background:'var(--bg-page)'}} defaultValue="agent 工具矩阵" />
            <div style={{textAlign:'center', padding:'24px 8px', color:'var(--ink-4)', fontSize:12}}>
              <div style={{fontFamily:'var(--font-mono)', fontSize:22, color:'var(--ink-5)'}}>⌕</div>
              <div style={{marginTop:6}}>未找到匹配</div>
              <div className="t-label" style={{marginTop:4, fontSize:9}}>仅搜索当前工作区 · 包含 .md / .txt / .html</div>
            </div>
          </div>
        </ShowFrame>
        <ShowFrame code="A·5" name="文件树空" hint="文件树面板">
          <div style={{padding:12, background:'var(--bg-card)', border:'0.5px solid var(--rule)', fontFamily:'var(--font-mono)', fontSize:11, color:'var(--ink-4)'}}>
            <div className="t-label" style={{marginBottom:6}}>WORKSPACE · ~/empty-ws</div>
            <div style={{textAlign:'center', padding:'18px 6px'}}>
              <div style={{fontSize:22, color:'var(--ink-5)'}}>▢</div>
              <div style={{marginTop:4, fontSize:11}}>这个目录是空的</div>
              <div className="t-label" style={{marginTop:6, fontSize:9}}>新建 ⌘N · 导入 ⌘I</div>
            </div>
          </div>
        </ShowFrame>
        <ShowFrame code="A·6" name="无 Pending Diff" hint="底部 strip 隐藏 · 状态栏静默">
          <div className="statusbar" style={{position:'relative', height:24, padding:'0 14px'}}>
            <span className="sb-item"><span className="sb-dot" />READY</span>
            <span className="sb-item">workspace · <b>binder-docs</b></span>
            <div className="sb-spacer" />
            <span className="sb-item" style={{color:'var(--ink-5)'}}>无 Pending Diff</span>
          </div>
          <div className="t-label" style={{marginTop:6}}>不主动出现底部 strip · 不主动提示</div>
        </ShowFrame>
      </div>

      <div className="page-hd-l" style={{padding:'24px 0 10px'}}>
        <span style={{fontFamily:'var(--font-mono)', fontSize:10, color:'var(--rust)', letterSpacing:'0.12em', textTransform:'uppercase', fontWeight:600}}>08·B 加载态</span>
      </div>
      <div className="grid grid-3">
        <ShowFrame code="B·1" name="文件加载中 · md / txt" hint="≤ 1.5s · skeleton">
          <PaperSkeleton />
        </ShowFrame>
        <ShowFrame code="B·2" name="文件加载中 · docx" hint="≤ 10s · 进度提示">
          <div style={{padding:'18px 14px', background:'var(--bg-card)', border:'0.5px solid var(--rule)', display:'flex', flexDirection:'column', gap:8, alignItems:'center', textAlign:'center'}}>
            <div className="t-label" style={{color:'var(--rust)'}}>DOCX · PANDOC 转换中</div>
            <div style={{fontSize:13, color:'var(--ink-2)'}}>roadmap-2026.docx</div>
            <div className="bar-track" style={{width:220}}><div className="bar-fill" /></div>
            <div className="t-label" style={{fontSize:9}}>约 3 秒 · 大文件可能需要 10 秒</div>
          </div>
        </ShowFrame>
        <ShowFrame code="B·3" name="AI 思考 vs 流式" hint="区分动效">
          <div style={{display:'flex', flexDirection:'column', gap:10}}>
            <div style={{padding:'10px 12px', background:'var(--bg-card)', border:'0.5px solid var(--rule)'}}>
              <div className="t-label">BINDER · THINKING</div>
              <div style={{marginTop:6, color:'var(--ink-3)', fontSize:12, display:'flex', alignItems:'center', gap:6}}>
                <span className="thinking-dots"><span></span><span></span><span></span></span> 正在思考…
              </div>
            </div>
            <div style={{padding:'10px 12px', background:'var(--bg-card)', border:'0.5px solid var(--rule)'}}>
              <div className="t-label">BINDER · STREAMING</div>
              <div style={{marginTop:6, color:'var(--ink-2)', fontSize:12}}>
                聊天的两种模式由 tab 元数据控制<span className="caret" />
              </div>
            </div>
          </div>
        </ShowFrame>
        <ShowFrame code="B·4" name="Tool 执行中" hint="工具卡内 bar">
          <div className="toolcall">
            <div className="toolcall-hd"><span className="toolcall-name"><span className="glyph">▸</span>search_files</span><span className="toolcall-status pending"><span className="spin" style={{marginRight:6}} />EXEC</span></div>
            <div className="toolcall-body"><div className="toolcall-arg"><span className="k">pattern</span><span className="v">工具矩阵</span></div></div>
            <div className="bar-track"><div className="bar-fill" /></div>
          </div>
        </ShowFrame>
        <ShowFrame code="B·5" name="Diff 接受中" hint="按钮禁用 + bar">
          <div className="dc is-applying">
            <div className="dc-hd"><span className="who">¶142</span><span className="badge gold"><span className="spin" style={{borderColor:'var(--bg-page)', borderRightColor:'transparent', marginRight:4}} />APPLYING</span></div>
            <div className="bar-track"><div className="bar-fill" /></div>
            <div className="dc-actions"><button className="accept disabled">应用中</button><button className="reject disabled">—</button></div>
          </div>
        </ShowFrame>
        <ShowFrame code="B·6" name="自动保存倒计时" hint="状态栏 · 标签">
          <div style={{display:'flex', flexDirection:'column', gap:8}}>
            <div className="statusbar" style={{position:'relative', height:24, padding:'0 14px'}}>
              <span className="sb-item"><span className="sb-dot warn" />SAVING…</span>
              <span className="sb-item">spec.md</span>
              <div className="sb-spacer" />
              <span className="sb-item">自动保存倒计时 <b>2s</b></span>
            </div>
            <div className="tab-chip active" style={{display:'inline-flex'}}>
              <span className="dot clean" />
              <span>spec.md</span>
              <span className="saving thinking-dots"><span></span><span></span><span></span></span>
            </div>
          </div>
        </ShowFrame>
      </div>

      <div className="page-hd-l" style={{padding:'24px 0 10px'}}>
        <span style={{fontFamily:'var(--font-mono)', fontSize:10, color:'var(--rust)', letterSpacing:'0.12em', textTransform:'uppercase', fontWeight:600}}>08·C 错误态</span>
      </div>
      <div className="grid grid-3">
        <ShowFrame code="C·1" name="文件无法打开" hint="格式损坏 / 权限不足">
          <ErrorBlock title="无法打开 broken.docx" code="E_PANDOC_PARSE · 文档结构损坏" actions={[['查看日志',null], ['另存为副本',null]]} />
        </ShowFrame>
        <ShowFrame code="C·2" name="AI 调用失败" hint="网络 / API · 可重试">
          <ErrorBlock title="AI 请求失败" code="E_NETWORK · timeout 30s · DeepSeek" actions={[['重试',{cls:'primary'}], ['切换模型',null]]} />
        </ShowFrame>
        <ShowFrame code="C·3" name="Tool 调用失败 · 可重试" hint="单次失败">
          <ToolCallShowcaseFail />
        </ShowFrame>
        <ShowFrame code="C·4" name="Tool 调用失败 · 已跳过" hint="多次重试后">
          <div className="toolcall" style={{opacity:0.7}}>
            <div className="toolcall-hd"><span className="toolcall-name"><span className="glyph">▸</span>read_file</span><span className="toolcall-status" style={{color:'var(--ink-4)'}}>SKIPPED · 3 次失败</span></div>
            <div className="toolcall-body"><div className="toolcall-arg"><span className="k">path</span><span className="v">missing.docx</span></div></div>
            <div className="toolcall-foot"><span>已跳过 · agent 已继续</span><button style={{marginLeft:'auto'}}>手动处理</button></div>
          </div>
        </ShowFrame>
        <ShowFrame code="C·5" name="工作区权限不足" hint="无法写入">
          <ErrorBlock title="目录不可写" code="E_PERMISSION · ~/work/binder-docs" actions={[['换一个工作区',{cls:'primary'}], ['查看权限',null]]} />
        </ShowFrame>
        <ShowFrame code="C·6" name="模型不可用" hint="API Key 失效 / 余额">
          <ErrorBlock title="DeepSeek API 不可用" code="E_AUTH · 401 · key invalid" actions={[['打开设置',{cls:'primary'}], ['切到 OpenAI',null]]} />
        </ShowFrame>
      </div>
    </Page>
  );
}

function EmptyState({ glyph, title, body, actions = [], small }) {
  return (
    <div style={{
      padding: small ? '16px 14px' : '28px 18px',
      background:'var(--bg-card)', border:'0.5px solid var(--rule)',
      textAlign:'center',
    }}>
      <div style={{fontFamily:'var(--font-mono)', fontSize: small ? 28 : 40, color:'var(--ink-5)'}}>{glyph}</div>
      <div style={{fontSize: small ? 13 : 15, color:'var(--ink)', fontWeight:600, marginTop:8, letterSpacing:'-0.01em'}}>{title}</div>
      <div style={{fontSize:12, color:'var(--ink-3)', marginTop:4, lineHeight:1.55}}>{body}</div>
      <div style={{display:'flex', gap:6, justifyContent:'center', marginTop:14, flexWrap:'wrap'}}>
        {actions.map(([t, opt], i) => (
          <button key={i} style={{
            padding:'5px 12px',
            fontFamily:'var(--font-mono)', fontSize:10, letterSpacing:'0.08em', textTransform:'uppercase',
            fontWeight:600,
            border:'0.5px solid var(--rule-strong)',
            background: opt === 'primary' ? 'var(--ink)' : 'transparent',
            color: opt === 'primary' ? 'var(--bg-page)' : 'var(--ink-2)',
          }}>{t}</button>
        ))}
      </div>
    </div>
  );
}

function ErrorBlock({ title, code, actions = [] }) {
  return (
    <div style={{padding:'12px 14px', background:'var(--bg-card)', border:'0.5px solid var(--rust)', borderLeft:'3px solid var(--rust)'}}>
      <div style={{display:'flex', justifyContent:'space-between', alignItems:'baseline'}}>
        <div style={{fontSize:13, fontWeight:600, color:'var(--ink)'}}>{title}</div>
        <span style={{fontFamily:'var(--font-mono)', fontSize:9, color:'var(--rust)', letterSpacing:'0.08em'}}>ERROR</span>
      </div>
      <div style={{fontFamily:'var(--font-mono)', fontSize:10, color:'var(--ink-4)', marginTop:4, wordBreak:'break-all'}}>{code}</div>
      <div style={{display:'flex', gap:6, marginTop:10}}>
        {actions.map(([t, opt], i) => (
          <button key={i} style={{
            padding:'4px 10px',
            fontFamily:'var(--font-mono)', fontSize:9.5, letterSpacing:'0.08em', textTransform:'uppercase', fontWeight:600,
            border:'0.5px solid var(--rule-strong)',
            background: opt && opt.cls === 'primary' ? 'var(--ink)' : 'transparent',
            color: opt && opt.cls === 'primary' ? 'var(--bg-page)' : 'var(--ink-2)',
          }}>{t}</button>
        ))}
      </div>
    </div>
  );
}

function PaperSkeleton() {
  return (
    <div style={{padding:14, background:'var(--bg-card)', border:'0.5px solid var(--rule)'}}>
      {[260, 320, 280, 360, 200, 340, 220].map((w, i) => (
        <div key={i} style={{
          height: 8, marginBottom: 8, width: w, maxWidth:'100%',
          background: 'linear-gradient(90deg, var(--rule-faint), var(--rule), var(--rule-faint))',
          backgroundSize: '200% 100%',
          animation: 'barslide 1.6s ease-in-out infinite',
        }} />
      ))}
    </div>
  );
}

function ToolCallShowcaseFail() {
  return (
    <div className="toolcall" style={{borderColor:'var(--rust)'}}>
      <div className="toolcall-hd"><span className="toolcall-name"><span className="glyph">▸</span>edit_current…</span><span className="toolcall-status fail">FAILED · E_IO</span></div>
      <div className="toolcall-body"><div className="toolcall-arg"><span className="k">error</span><span className="v" style={{color:'var(--rust)'}}>磁盘暂不可写 · 已重试 1 次</span></div></div>
      <div className="toolcall-foot"><button style={{color:'var(--ink-2)'}}>重试 ↻</button><button>跳过</button></div>
    </div>
  );
}

/* ─────────── Design Tokens page ─────────── */
function TokensPage() {
  const colorTokens = [
    { name: 'D-INK',   code: '--ink',         hex: '#0E0C0A', use: '主文字 / 描边强调' },
    { name: 'D-PAPER', code: '--bg-page',     hex: '#F4F1EA', use: '纸面底色 / 应用背景' },
    { name: 'D-CLAY',  code: '--rust',        hex: '#C24A2A', use: '强调点 / Pending / Reject 文案' },
    { name: 'FOREST',  code: '--forest',      hex: '#2E6F4A', use: 'Accept / 成功 · 仅语义' },
    { name: 'GOLD',    code: '--gold',        hex: '#B58A2E', use: '中间过程 / Conflict / 警示' },
    { name: 'FOG',     code: '--bg-sunken',   hex: '#E6E2D8', use: '深一档底色 · 标题栏 / 状态栏' },
    { name: 'CARD',    code: '--bg-card',     hex: '#FFFFFF', use: '纸张/卡片表面' },
    { name: 'RULE',    code: '--rule',        hex: 'rgba(14,12,10,0.10)', use: '分割线常规' },
  ];
  const typeSizes = [
    [44, 'Display',  'binder.',                           'IBM Plex Sans 500 · −0.04em'],
    [22, 'H1',       '段落级 AI 协作',                     'IBM Plex Sans 600 · −0.01em'],
    [16, 'H2',       '工作区 · 文档 · Diff',               'IBM Plex Sans 600'],
    [14, 'Body',     '正文段落与文档内容',                 'IBM Plex Sans 400 · 1.65'],
    [12, 'Meta',     '次要说明 / 状态文本',                'IBM Plex Sans 400'],
    [10, 'Label',    'CODE · UPPERCASE',                  'IBM Plex Mono 500 · 0.08em tracking'],
  ];
  const spacing = [[4,'XS'],[8,'S'],[12,'M'],[16,'L'],[24,'XL'],[32,'2XL'],[48,'3XL']];
  return (
    <Page code="09" name="Design Tokens · 规范" sub="VI · TYPE · SPACING · Z-INDEX">

      <div className="page-hd-l" style={{padding:'4px 0 10px'}}>
        <span style={{fontFamily:'var(--font-mono)', fontSize:10, color:'var(--rust)', letterSpacing:'0.12em', textTransform:'uppercase', fontWeight:600}}>09·A 颜色</span>
      </div>
      <div className="grid grid-4">
        {colorTokens.map(c => (
          <div className="tok-card" key={c.code}>
            <div className="swatch" style={{background: c.hex}} />
            <div className="name">{c.name}</div>
            <div className="code">{c.code}</div>
            <div className="v">{c.hex}</div>
            <div className="use">{c.use}</div>
          </div>
        ))}
      </div>

      <div className="page-hd-l" style={{padding:'24px 0 10px'}}>
        <span style={{fontFamily:'var(--font-mono)', fontSize:10, color:'var(--rust)', letterSpacing:'0.12em', textTransform:'uppercase', fontWeight:600}}>09·B 字体阶梯</span>
      </div>
      <div style={{background:'var(--bg-card)', border:'0.5px solid var(--rule)', padding:'14px 18px'}}>
        {typeSizes.map(([sz, n, sample, hint]) => (
          <div key={n} style={{display:'grid', gridTemplateColumns:'56px 1fr 220px', alignItems:'baseline', gap:18, padding:'10px 0', borderBottom:'0.5px solid var(--rule-faint)'}}>
            <span className="t-label">{sz}px · {n}</span>
            <span style={{fontSize: sz, fontWeight: sz >= 22 ? 600 : 500, lineHeight: 1.1, letterSpacing: sz >= 22 ? -sz*0.012 : 0, fontFamily:'var(--font-sans)'}}>{sample}</span>
            <span className="t-code" style={{fontSize: 11, color:'var(--ink-4)'}}>{hint}</span>
          </div>
        ))}
      </div>

      <div className="page-hd-l" style={{padding:'24px 0 10px'}}>
        <span style={{fontFamily:'var(--font-mono)', fontSize:10, color:'var(--rust)', letterSpacing:'0.12em', textTransform:'uppercase', fontWeight:600}}>09·C 间距 · 圆角 · 描边</span>
      </div>
      <div className="grid grid-2">
        <div style={{background:'var(--bg-card)', border:'0.5px solid var(--rule)', padding:'14px 18px'}}>
          <div className="t-label" style={{marginBottom:10}}>SPACING SCALE (PX)</div>
          {spacing.map(([n, label]) => (
            <div key={n} style={{display:'flex', alignItems:'center', gap:14, padding:'4px 0'}}>
              <span className="t-code" style={{width:50, color:'var(--ink-3)'}}>{label} · {n}</span>
              <span style={{height:10, background:'var(--ink)', width:n*4}} />
            </div>
          ))}
        </div>
        <div style={{background:'var(--bg-card)', border:'0.5px solid var(--rule)', padding:'14px 18px'}}>
          <div className="t-label" style={{marginBottom:10}}>RADIUS · STROKE</div>
          <div style={{display:'flex', gap:12, alignItems:'flex-end', marginBottom:14}}>
            {[0, 2, 4, 8].map(r => (
              <div key={r} style={{textAlign:'center'}}>
                <div style={{width:46, height:46, background:'var(--ink)', borderRadius:r}} />
                <div className="t-label" style={{fontSize:9, marginTop:4}}>{r}px</div>
              </div>
            ))}
          </div>
          <div className="t-label" style={{marginBottom:6}}>STROKE</div>
          <div style={{display:'flex', flexDirection:'column', gap:6}}>
            {[ ['0.5','rule'], ['1','rule-strong'], ['2','accent rule'] ].map(([w, n]) => (
              <div key={n} style={{display:'flex', alignItems:'center', gap:12}}>
                <span className="t-code" style={{width:80, color:'var(--ink-3)'}}>{w}px · {n}</span>
                <span style={{flex:1, height:0, borderTop:`${w}px solid ${n==='accent rule' ? 'var(--rust)' : n === 'rule-strong' ? 'var(--rule-strong)' : 'var(--rule)'}`}} />
              </div>
            ))}
          </div>
        </div>
      </div>

      <div className="page-hd-l" style={{padding:'24px 0 10px'}}>
        <span style={{fontFamily:'var(--font-mono)', fontSize:10, color:'var(--rust)', letterSpacing:'0.12em', textTransform:'uppercase', fontWeight:600}}>09·D z-index 层级</span>
      </div>
      <div style={{background:'var(--bg-card)', border:'0.5px solid var(--rule)', padding:'14px 18px'}}>
        <table className="doc" style={{width:'100%', fontSize:12}}>
          <thead><tr>
            <th>层级</th><th>组件</th><th>z-index</th><th>说明</th>
          </tr></thead>
          <tbody>
            <tr><td>L1 内容</td><td>编辑器 / 文件树 / 聊天</td><td className="num">0</td><td>常规视图</td></tr>
            <tr><td>L2 装饰</td><td>Green Review · ProseMirror Decoration</td><td className="num">1</td><td>叠加在内容上但不阻挡编辑</td></tr>
            <tr><td>L3 固定栏</td><td>顶栏 · 状态栏 · DiffAllActionsBar</td><td className="num">5</td><td>始终可见</td></tr>
            <tr><td>L4 浮层</td><td>AutoComplete · Inline Assist · Reference Picker</td><td className="num">11</td><td>锚定到光标 / 选区位置</td></tr>
            <tr><td>L5 弹窗</td><td>Confirm Dialog · Authorization Card · 关闭警告</td><td className="num">20</td><td>带遮罩 · 阻挡其他交互</td></tr>
            <tr><td>L6 系统</td><td>Toast · 全局错误条</td><td className="num">30</td><td>跨视图通知</td></tr>
          </tbody>
        </table>
      </div>

    </Page>
  );
}

window.BinderShowcase3 = {
  FlowA, FlowB, FlowC, FlowD, FlowF, FlowG, FlowH,
  EmptyLoadingErrorPage, TokensPage,
};
