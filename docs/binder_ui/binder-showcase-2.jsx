/* binder-showcase-2.jsx — Tool call · Tabs / file nodes · Green Review · M2/M3 overlays */

const { Page, ShowFrame } = window.BinderShowcase;

/* ─────────── Tool Call states ─────────── */
function ToolCallShowcase({ state }) {
  const base = {
    name: 'edit_current_editor_document',
    args: { path: 'spec.md', hunks: 2, range: '§4.7' },
  };
  switch (state) {
    case 'pending':
      return (
        <div className="toolcall">
          <div className="toolcall-hd">
            <span className="toolcall-name"><span className="glyph">▸</span>{base.name}</span>
            <span className="toolcall-status pending">QUEUED</span>
          </div>
          <div className="toolcall-body">
            <div className="toolcall-arg"><span className="k">path</span><span className="v">{base.args.path}</span></div>
            <div className="toolcall-arg"><span className="k">range</span><span className="v">{base.args.range}</span></div>
          </div>
        </div>
      );
    case 'executing':
      return (
        <div className="toolcall">
          <div className="toolcall-hd">
            <span className="toolcall-name"><span className="glyph">▸</span>{base.name}</span>
            <span className="toolcall-status pending"><span className="spin" style={{marginRight:6}} />EXECUTING</span>
          </div>
          <div className="toolcall-body">
            <div className="toolcall-arg"><span className="k">path</span><span className="v">{base.args.path}</span></div>
            <div className="toolcall-arg"><span className="k">hunks</span><span className="v">{base.args.hunks}</span></div>
          </div>
          <div className="bar-track"><div className="bar-fill" /></div>
        </div>
      );
    case 'awaiting_confirmation':
      return (
        <div className="toolcall" style={{borderColor: 'var(--rust)'}}>
          <div className="toolcall-hd">
            <span className="toolcall-name"><span className="glyph" style={{color:'var(--rust)'}}>!</span>delete_file</span>
            <span className="toolcall-status" style={{color:'var(--rust)'}}>AUTHORIZE</span>
          </div>
          <div className="toolcall-body">
            <div className="toolcall-arg"><span className="k">path</span><span className="v">workflows/old-draft.md</span></div>
            <div className="toolcall-arg"><span className="k">reason</span><span className="v">cleanup_after_merge</span></div>
          </div>
          <div className="toolcall-foot" style={{justifyContent:'space-between'}}>
            <span style={{color:'var(--ink-3)'}}>需要用户授权才能继续</span>
            <span style={{display:'flex', gap:10}}>
              <button>跳过</button>
              <button style={{color:'var(--forest)'}}>授权 ⏎</button>
            </span>
          </div>
        </div>
      );
    case 'done':
      return (
        <div className="toolcall">
          <div className="toolcall-hd">
            <span className="toolcall-name"><span className="glyph">▸</span>{base.name}</span>
            <span className="toolcall-status">OK · 312 ms</span>
          </div>
          <div className="toolcall-body">
            <div className="toolcall-arg"><span className="k">path</span><span className="v">{base.args.path}</span></div>
            <div className="toolcall-arg"><span className="k">result</span><span className="v">2 hunks · 14 lines staged → diffStore</span></div>
          </div>
          <div className="toolcall-foot">
            <button>展开详情</button>
            <button>查看 Diff</button>
          </div>
        </div>
      );
    case 'failed':
      return (
        <div className="toolcall" style={{borderColor: 'var(--rust)'}}>
          <div className="toolcall-hd">
            <span className="toolcall-name"><span className="glyph">▸</span>read_file</span>
            <span className="toolcall-status fail">FAILED</span>
          </div>
          <div className="toolcall-body">
            <div className="toolcall-arg"><span className="k">path</span><span className="v">assets/missing.docx</span></div>
            <div className="toolcall-arg"><span className="k">error</span><span className="v" style={{color:'var(--rust)'}}>E_NOT_FOUND · 文件不存在或已移动</span></div>
          </div>
          <div className="toolcall-foot">
            <button style={{color:'var(--ink-2)'}}>重试 ↻</button>
            <button>跳过</button>
            <button style={{marginLeft:'auto', color:'var(--ink-5)'}}>已重试 2 次</button>
          </div>
        </div>
      );
    case 'skipped':
      return (
        <div className="toolcall" style={{opacity:0.7}}>
          <div className="toolcall-hd">
            <span className="toolcall-name"><span className="glyph">▸</span>list_files</span>
            <span className="toolcall-status" style={{color:'var(--ink-4)'}}>SKIPPED</span>
          </div>
          <div className="toolcall-body">
            <div className="toolcall-arg"><span className="k">path</span><span className="v">node_modules/</span></div>
            <div className="toolcall-arg"><span className="k">reason</span><span className="v">超出深度上限 · agent 决定跳过</span></div>
          </div>
        </div>
      );
    default: return null;
  }
}

function ToolStatesPage() {
  const M = window.MOCK;
  return (
    <Page code="03" name="Tool Call · 6 状态" sub="STATE MACHINE · 5.5">
      <div className="grid grid-3">
        {M.TOOL_STATES.map(s => (
          <ShowFrame key={s.id} code={s.code} name={s.label} hint={s.en}>
            <ToolCallShowcase state={s.id} />
          </ShowFrame>
        ))}
      </div>

      <div style={{marginTop: 24}}>
        <div className="page-hd-l" style={{padding: '0 0 12px'}}>
          <span style={{fontFamily:'var(--font-mono)', fontSize:10, color:'var(--rust)', letterSpacing:'0.12em', textTransform:'uppercase', fontWeight:600}}>03 · 危险操作门</span>
          <span style={{fontFamily:'var(--font-sans)', fontSize:14, color:'var(--ink)'}}>delete_file / move_file 等需要授权的工具</span>
        </div>
        <ShowFrame code="03·X" name="Authorization Card · 嵌入消息流" hint="独立可见且不被淹没">
          <ToolCallShowcase state="awaiting_confirmation" />
        </ShowFrame>
      </div>
    </Page>
  );
}

/* ─────────── Tab + File Node + DiffAllActionsBar ─────────── */
function TabChip({ state }) {
  const cls = state.id;
  return (
    <div className={`tab-chip ${state.id === 'normal' ? '' : 'active'}`}>
      {state.dirty
        ? <span className="dot" />
        : state.saving
          ? <span className="dot clean" />
          : <span className="dot clean" />}
      <span>spec.md</span>
      {state.badge && (
        <span className={`badge ${state.warn ? 'warn' : ''}`}>{state.badge}</span>
      )}
      {state.saving && <span className="saving thinking-dots"><span></span><span></span><span></span></span>}
    </div>
  );
}
function FNChip({ state }) {
  return (
    <div className={`fn-chip ${state.active ? 'active' : ''} ${state.disabled ? 'disabled' : ''}`}>
      <span className="ico">M↓</span>
      <span>spec.md</span>
      {state.dirty && <span className="badge dot" />}
      {state.badge && !state.dirty && (
        <span className={`badge ${state.warn ? 'warn' : ''}`}>{state.badge}</span>
      )}
    </div>
  );
}

function TabFileNodePage() {
  const M = window.MOCK;
  return (
    <Page code="04" name="Editor Tab · File Tree Node · 状态合集" sub="STATE 5.2 · 5.4">
      <div className="grid grid-2">
        <ShowFrame code="04·A" name="Editor Tab · 6 状态" hint="STATE 5.2">
          <div style={{display:'flex', flexWrap:'wrap', gap:10, alignItems:'flex-start'}}>
            {M.TAB_STATES.map(s => (
              <div key={s.id} style={{display:'flex', flexDirection:'column', gap:6}}>
                <TabChip state={s} />
                <span className="t-label" style={{fontSize:9}}>{s.label}</span>
              </div>
            ))}
          </div>
          <div className="legend">
            <span className="legend-item"><span className="dot" style={{width:6, height:6, borderRadius:'50%', background:'var(--rust)'}}></span>未保存原点</span>
            <span className="legend-item"><span className="legend-dot y"></span>外部已改/警示</span>
            <span className="legend-item">数字 = Diff 数</span>
          </div>
        </ShowFrame>

        <ShowFrame code="04·B" name="File Tree Node · 6 状态" hint="STATE 5.4">
          <div style={{display:'flex', flexDirection:'column', gap:8}}>
            {M.FILE_NODE_STATES.map(s => (
              <div key={s.id} style={{display:'flex', alignItems:'center', gap:14}}>
                <FNChip state={s} />
                <span className="t-label" style={{fontSize:9}}>{s.label}</span>
              </div>
            ))}
          </div>
        </ShowFrame>

        <ShowFrame code="04·C" name="DiffAllActionsBar · 3 态" hint="批量操作栏 · 5.1 + 链路 B">
          <div style={{display:'flex', flexDirection:'column', gap: 10}}>
            <div className="diffstrip" style={{borderRadius:0}}>
              <div className="diffstrip-label">
                <span className="num">3</span>
                <span>待决策 · PENDING DIFF</span>
              </div>
              <div className="diffstrip-files">
                <span className="diffstrip-file"><b>spec.md</b><span>·</span><span>§4.7</span><span className="pm">+11</span><span className="mn">−6</span></span>
                <span className="diffstrip-file"><b>spec.md</b><span>·</span><span>§3.4</span><span className="pm">+3</span><span className="mn">−1</span></span>
                <span className="diffstrip-file"><b>README.md</b><span>·</span><span>head</span><span className="pm">+4</span><span className="mn">−2</span></span>
              </div>
              <div className="diffstrip-actions">
                <button className="diffstrip-btn danger">全部拒绝</button>
                <button className="diffstrip-btn primary">全部接受 · ⌘⏎</button>
              </div>
            </div>
            <span className="t-label" style={{fontSize:9}}>① 初始 · 3 条待决策</span>

            <div className="diffstrip" style={{borderRadius:0}}>
              <div className="diffstrip-label">
                <span className="num" style={{background:'var(--gold)'}}>2/3</span>
                <span>倒序应用中… <span className="spin" /></span>
              </div>
              <div className="diffstrip-files">
                <span className="diffstrip-file" style={{opacity:0.4}}><b>README.md</b><span style={{color:'var(--forest)'}}>✓</span></span>
                <span className="diffstrip-file"><b>spec.md</b><span>·</span><span>§4.7</span><span className="pm">+11</span><span className="mn">−6</span></span>
                <span className="diffstrip-file" style={{opacity:0.5}}><b>spec.md</b><span>·</span><span>§3.4</span><span className="spin" /></span>
              </div>
              <div className="diffstrip-actions">
                <button className="diffstrip-btn" style={{color:'var(--ink-5)', pointerEvents:'none'}}>处理中</button>
              </div>
            </div>
            <span className="t-label" style={{fontSize:9}}>② 倒序执行中 · 从靠后的 Diff 开始</span>

            <div className="diffstrip" style={{borderRadius:0, background:'var(--forest-bg)', borderColor:'var(--forest)'}}>
              <div className="diffstrip-label">
                <span className="num" style={{background:'var(--forest)'}}>✓</span>
                <span>完成 · 2 接受 · 1 失效</span>
              </div>
              <div className="diffstrip-files">
                <span className="diffstrip-file"><b>README.md</b><span style={{color:'var(--forest)'}}>已接受</span></span>
                <span className="diffstrip-file"><b>spec.md</b><span>·</span><span>§4.7</span><span style={{color:'var(--forest)'}}>已接受</span></span>
                <span className="diffstrip-file" style={{borderColor:'var(--gold)'}}><b>spec.md</b><span>·</span><span>§3.4</span><span style={{color:'var(--gold)'}}>已失效</span></span>
              </div>
              <div className="diffstrip-actions">
                <button className="diffstrip-btn">关闭</button>
              </div>
            </div>
            <span className="t-label" style={{fontSize:9}}>③ 全部处理完 · 含 1 条失效</span>
          </div>
        </ShowFrame>

        <ShowFrame code="04·D" name="Status Bar · 4 状态" hint="底部状态条">
          <div style={{display:'flex', flexDirection:'column', gap: 6}}>
            {[
              { kind:'ready',   line:[['sb-dot',null],['READY'],['workspace · binder-docs'],['⌘ 247 files']] },
              { kind:'saving',  line:[['sb-dot warn',null],['SAVING…'],['spec.md'],['自动保存倒计时 2s']] },
              { kind:'pending', line:[['sb-dot warn',null],['1 PENDING DIFF · 等待决策'],['spec.md § §4.7']] },
              { kind:'error',   line:[['sb-dot err',null],['DISK READ-ONLY'],['无法保存 · 切换工作区目录']] },
            ].map((s, i) => (
              <div key={i} className="statusbar" style={{height:24, position:'relative'}}>
                {s.line.map(([cls, txt], j) => (
                  <span key={j} className="sb-item">
                    {cls.startsWith('sb-dot') ? <span className={cls} /> : <b>{cls}</b>}
                    {txt}
                  </span>
                ))}
              </div>
            ))}
          </div>
        </ShowFrame>
      </div>
    </Page>
  );
}

/* ─────────── Green Review section ─────────── */
function GreenReviewPage() {
  return (
    <Page code="05" name="Green Review · 编辑器装饰层" sub="DECORATION · 5.1 ↔ 编辑器">
      <div className="grid grid-2">
        <ShowFrame code="05·A" name="单块 · 4 种状态" hint="装饰层叠加，不替换 DOM">
          <div style={{background:'var(--bg-card)', padding:'14px 16px', border:'0.5px solid var(--rule)'}}>
            <div className="doc">
              <p style={{margin:0}}>当前的工具矩阵真实暴露的工具有：</p>

              <div className="gr-block">
                <div className="gr-block-hd">
                  <span>① <span className="id">PENDING</span> · ¶ 142 · 由 BINDER 生成</span>
                  <span>+11 / −6</span>
                </div>
                <div><span className="del">read_file、list_files、search_files、create_file…（一长串无分组）</span></div>
                <div style={{marginTop:6}}><span className="add">按类别分组：读取 · 资源结构 · 写入 · 依赖。</span></div>
              </div>

              <div className="gr-block applying">
                <div className="gr-block-hd">
                  <span>② <span className="id" style={{color:'var(--gold)'}}>APPLYING</span> · ¶ 168 <span className="spin" style={{marginLeft:6}} /></span>
                  <span>+3 / −1</span>
                </div>
                <div><span className="del">所有工具默认无需授权。</span></div>
                <div style={{marginTop:6}}><span className="add">删除/移动类工具默认需要用户授权。</span></div>
              </div>

              <div className="gr-block accepted">
                <div className="gr-block-hd">
                  <span>③ <span className="id">ACCEPTED</span> · ¶ 194</span>
                  <span>+1 / −0</span>
                </div>
                <div><span style={{color:'var(--ink-2)'}}>chat 模式 <span className="add">不</span>启用任何工具。</span></div>
              </div>

              <div className="gr-block expired">
                <div className="gr-block-hd">
                  <span>④ <span className="id" style={{color:'var(--ink-4)'}}>EXPIRED</span> · ¶ 212 · 原文已被手动编辑</span>
                </div>
                <div style={{color:'var(--ink-4)'}}>装饰层已淡出 · 不再阻挡编辑</div>
              </div>

              <p style={{margin:'8px 0 0', color:'var(--ink-4)', fontSize:12}}>—— 其余正文不被装饰层影响 ——</p>
            </div>
          </div>
        </ShowFrame>

        <ShowFrame code="05·B" name="装饰层与 Diff Card 一一对应" hint="编辑器 ↔ 右侧面板">
          <div style={{display:'grid', gridTemplateColumns:'1fr 1fr', gap:10}}>
            <div style={{background:'var(--bg-card)', padding:'12px', border:'0.5px solid var(--rule)'}}>
              <div className="t-label" style={{marginBottom:6}}>EDITOR · 装饰</div>
              <div className="gr-block" style={{margin:'4px 0'}}>
                <div className="gr-block-hd"><span className="id">¶142</span><span>+11/−6</span></div>
                <div><span className="del">chat 启用工具…</span></div>
                <div><span className="add">chat 不启用任何工具。</span></div>
              </div>
              <div className="gr-block" style={{margin:'4px 0'}}>
                <div className="gr-block-hd"><span className="id">¶168</span><span>+3/−1</span></div>
                <div><span className="add">需要授权…</span></div>
              </div>
            </div>
            <div style={{background:'var(--bg-card)', padding:'12px', border:'0.5px solid var(--rule)'}}>
              <div className="t-label" style={{marginBottom:6}}>CHAT · Diff Cards</div>
              <div style={{display:'flex', flexDirection:'column', gap:8}}>
                <div className="dc is-pending" style={{minHeight:0}}>
                  <div className="dc-hd"><span className="who">¶142</span><span className="badge rust">PENDING</span></div>
                </div>
                <div className="dc is-pending">
                  <div className="dc-hd"><span className="who">¶168</span><span className="badge rust">PENDING</span></div>
                </div>
              </div>
            </div>
          </div>
          <div className="legend" style={{marginTop:8}}>
            <span>在任意一侧 Accept / Reject，另一侧同步更新。</span>
          </div>
        </ShowFrame>
      </div>
    </Page>
  );
}

/* ─────────── Cmd+J / Cmd+K overlays ─────────── */
function AssistOverlaysPage() {
  return (
    <Page code="06" name="M2 续写 · M3 局部修改" sub="OVERLAYS · ⌘J · ⌘K">
      <div className="grid grid-2">
        <ShowFrame code="06·A" name="AutoComplete · ⌘J · 续写建议" hint="3 条建议 · Tab/↩ 选择 · Esc 关闭">
          <div style={{position:'relative', background:'var(--bg-card)', padding:'18px 22px', border:'0.5px solid var(--rule)', minHeight:260}}>
            <div className="doc" style={{maxWidth:'unset'}}>
              <p style={{margin:0}}>Binder 当前是一个本地桌面应用，核心形态是工作区 + 编辑器 + AI 协作的文档工作台。
              <span className="caret" />
              </p>
            </div>
            <div className="ac-popover" style={{position:'absolute', left:18, top:62}}>
              <div className="ac-hd">
                <span>AUTOCOMPLETE · ⌘J · DEEPSEEK</span>
                <span>3 / 3</span>
              </div>
              <div className="ac-row active">
                <span className="idx">↩</span>
                <span>AI 修改不会直接落盘，而是作为可审查的 Diff 进入编辑器装饰层与右侧 Diff Card，由用户逐条决定。</span>
              </div>
              <div className="ac-row">
                <span className="idx">2</span>
                <span>这种"先 Diff 再生效"的设计让 AI 协作的过程具有可追溯性。</span>
              </div>
              <div className="ac-row">
                <span className="idx">3</span>
                <span>所有改动都被记录到 workspace.db，可以从时间轴中回溯到任一时刻的状态。</span>
              </div>
              <div className="ac-foot">
                <span>↑↓ 切换</span>
                <span>TAB 替换</span>
                <span>↩ 选择</span>
                <span>ESC 关闭</span>
              </div>
            </div>
          </div>
        </ShowFrame>

        <ShowFrame code="06·B" name="Inline Assist · ⌘K · 4 阶段" hint="选区 → 输入 → 生成 → 应用">
          <div style={{display:'flex', flexDirection:'column', gap: 10}}>
            {[
              { code:'1', name:'输入指令', body: <InlineAssistStage stage="input" /> },
              { code:'2', name:'AI 生成中', body: <InlineAssistStage stage="generating" /> },
              { code:'3', name:'建议预览', body: <InlineAssistStage stage="preview" /> },
              { code:'4', name:'已 Apply', body: <InlineAssistStage stage="applied" /> },
            ].map(s => (
              <div key={s.code} style={{display:'flex', gap:10, alignItems:'flex-start'}}>
                <div className="t-label" style={{minWidth: 38, paddingTop:6}}>{`06·B·${s.code}`}<br/><span style={{color:'var(--ink-3)', fontFamily:'var(--font-sans)', fontSize:11, textTransform:'none', letterSpacing:0}}>{s.name}</span></div>
                <div style={{flex:1, background:'var(--bg-card)', padding:12, border:'0.5px solid var(--rule)', minHeight: 0}}>
                  {s.body}
                </div>
              </div>
            ))}
          </div>
        </ShowFrame>
      </div>
    </Page>
  );
}

function InlineAssistStage({ stage }) {
  if (stage === 'input') {
    return (
      <div className="assist" style={{position:'relative', left:0, top:0, transform:'none', width:'100%'}}>
        <div className="assist-hd"><span>INLINE ASSIST · ⌘K · 局部修改</span><span>spec.md · §4.7 · 3 行选中</span></div>
        <div className="assist-input"><textarea defaultValue="把这一段改写得更书面化" autoFocus={false} /></div>
        <div className="assist-foot"><span>模型 · deepseek-chat</span><div style={{display:'flex', gap:14}}><span>ESC 取消</span><button>⏎ 生成</button></div></div>
      </div>
    );
  }
  if (stage === 'generating') {
    return (
      <div className="assist" style={{position:'relative', left:0, top:0, transform:'none', width:'100%'}}>
        <div className="assist-hd"><span>INLINE ASSIST · ⌘K · 生成中</span><span><span className="spin" /> 流式输出…</span></div>
        <div className="assist-input" style={{minHeight: 0}}>
          <div style={{fontSize:13, color:'var(--ink-2)', lineHeight:1.55}}>
            聊天的两种模式由 tab 元数据控制：chat 按纯对话主链运行，<span className="thinking-dots"><span></span><span></span><span></span></span>
          </div>
        </div>
        <div className="bar-track"><div className="bar-fill" /></div>
        <div className="assist-foot"><span>已输出 24 字符</span><span>ESC 中断</span></div>
      </div>
    );
  }
  if (stage === 'preview') {
    return (
      <div className="assist" style={{position:'relative', left:0, top:0, transform:'none', width:'100%'}}>
        <div className="assist-hd"><span>INLINE ASSIST · 建议预览</span><span>对比 · BEFORE / AFTER</span></div>
        <div style={{padding:12}}>
          <div className="t-label" style={{marginBottom:4}}>BEFORE · 选区</div>
          <div className="diff-row del" style={{borderRadius:0}}><span className="gutter">142</span><span className="mark">−</span><span className="content">聊天有两种模式，agent 启用工具。</span></div>
          <div className="t-label" style={{margin:'8px 0 4px'}}>AFTER · 建议</div>
          <div className="diff-row add"><span className="gutter">142</span><span className="mark">+</span><span className="content">聊天的两种模式由 tab 元数据控制：chat 按纯对话主链运行，不启用任何工具；agent 启用工具调用。</span></div>
        </div>
        <div className="assist-foot">
          <span>不进入 Diff 链路 · 直接替换选区</span>
          <div style={{display:'flex', gap:14}}>
            <button>继续追问</button>
            <button style={{color:'var(--rust)'}}>取消</button>
            <button style={{color:'var(--forest)', fontWeight:700}}>Apply ⏎</button>
          </div>
        </div>
      </div>
    );
  }
  // applied
  return (
    <div style={{position:'relative', padding:'8px 12px', border:'0.5px solid var(--forest)', background:'var(--forest-bg)', display:'flex', alignItems:'center', gap:10}}>
      <span style={{color:'var(--forest)', fontWeight:700}}>✓</span>
      <span style={{fontSize:12, color:'var(--ink)'}}>已替换选区 · 142–144 行 · 无 Diff 卡片产出</span>
      <span style={{marginLeft:'auto', fontFamily:'var(--font-mono)', fontSize:10, color:'var(--ink-4)'}}>14:37 · 时间轴 #1289</span>
    </div>
  );
}

window.BinderShowcase2 = {
  ToolStatesPage, TabFileNodePage, GreenReviewPage, AssistOverlaysPage,
};
