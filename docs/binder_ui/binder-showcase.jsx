/* binder-showcase.jsx — state-level showcases for Diff cards, Tool cards, tabs, file nodes, etc. */

const { useState: useStateS, useEffect: useEffectS } = React;

/* ─── Section Page wrapper ─── */
function Page({ code, name, sub, children }) {
  return (
    <div className="page">
      <div className="page-hd">
        <div className="page-hd-l">
          <span className="code">{code}</span>
          <span className="name">{name}</span>
        </div>
        <span className="sub">{sub}</span>
      </div>
      <div className="page-body">{children}</div>
    </div>
  );
}

function ShowFrame({ code, name, hint, foot, children }) {
  return (
    <div className="sc-frame">
      <div className="sc-frame-hd">
        <span><span className="code">{code}</span> · <span className="name">{name}</span></span>
        <span>{hint}</span>
      </div>
      <div className="sc-frame-body">{children}</div>
      {foot && <div className="sc-frame-foot">{foot}</div>}
    </div>
  );
}

/* ─── Diff Card showcase ─── */
function DiffCardBody({ original = '聊天有两种模式，agent 启用工具。', revised, hideRevised, dim }) {
  return (
    <div className="dc-body" style={{ opacity: dim ? 0.6 : 1 }}>
      <div className="diff-row del">
        <span className="gutter">142</span><span className="mark">−</span>
        <span className="content">{original}</span>
      </div>
      {!hideRevised && (
        <>
          <div className="diff-row add">
            <span className="gutter">142</span><span className="mark">+</span>
            <span className="content">{revised || '聊天当前有 chat 与 agent 两种模式，模式边界由 tab 元数据控制。'}</span>
          </div>
          <div className="diff-row add">
            <span className="gutter">143</span><span className="mark">+</span>
            <span className="content">　· chat 按纯对话主链运行，不启用任何工具。</span>
          </div>
        </>
      )}
    </div>
  );
}

function DiffCardShowcase({ state }) {
  const meta = (
    <span className="meta">spec.md · §4.7 · <span style={{color:'var(--forest)'}}>+11</span> · <span style={{color:'var(--rust)'}}>−6</span></span>
  );

  switch (state) {
    case 'pending':
      return (
        <div className="dc is-pending">
          <div className="dc-hd">
            <span className="who">BINDER · <b>1 HUNK</b></span>
            <span className="badge rust">PENDING</span>
          </div>
          <div className="dc-hd" style={{borderTop:'0.5px solid var(--rule-faint)', borderBottom: 0}}>{meta}<span>等待决策</span></div>
          <DiffCardBody />
          <div className="dc-actions">
            <button className="accept">接受 ⏎</button>
            <button className="reject">拒绝 ⌫</button>
          </div>
        </div>
      );
    case 'validating':
      return (
        <div className="dc is-validating">
          <div className="dc-hd">
            <span className="who">BINDER · <b>1 HUNK</b></span>
            <span className="badge gold"><span className="spin" style={{borderColor:'var(--bg-page)', borderRightColor:'transparent', marginRight:6}} />VALIDATING</span>
          </div>
          <div className="dc-hd" style={{borderTop:'0.5px solid var(--rule-faint)', borderBottom: 0}}>{meta}<span>正在校验原文匹配…</span></div>
          <DiffCardBody dim />
          <div className="bar-track" style={{margin: '4px 0'}}><div className="bar-fill" /></div>
          <div className="dc-actions">
            <button className="accept disabled">接受</button>
            <button className="reject disabled">拒绝</button>
          </div>
        </div>
      );
    case 'applying':
      return (
        <div className="dc is-applying">
          <div className="dc-hd">
            <span className="who">BINDER · <b>1 HUNK</b></span>
            <span className="badge gold"><span className="spin" style={{borderColor:'var(--bg-page)', borderRightColor:'transparent', marginRight:6}} />APPLYING</span>
          </div>
          <div className="dc-hd" style={{borderTop:'0.5px solid var(--rule-faint)', borderBottom: 0}}>{meta}<span>正在写入文档…</span></div>
          <DiffCardBody dim />
          <div className="bar-track" style={{margin: '4px 0'}}><div className="bar-fill" /></div>
          <div className="dc-actions">
            <button className="accept disabled">应用中</button>
            <button className="reject disabled">—</button>
          </div>
        </div>
      );
    case 'accepted':
      return (
        <div className="dc is-accepted">
          <div className="dc-hd">
            <span className="who">BINDER · <b>1 HUNK</b></span>
            <span className="badge forest">ACCEPTED</span>
          </div>
          <div className="dc-hd" style={{borderTop:'0.5px solid var(--rule-faint)', borderBottom: 0}}>{meta}<span>已写入 spec.md · 14:33</span></div>
          <DiffCardBody dim />
          <div className="dc-notice">
            <span className="glyph forest">✓</span>
            <span>已应用。可在时间轴节点 <span style={{color:'var(--ink)', fontFamily:'var(--font-mono)'}}>#1287</span> 回滚。</span>
          </div>
        </div>
      );
    case 'rejected':
      return (
        <div className="dc is-rejected">
          <div className="dc-hd">
            <span className="who">BINDER · <b>1 HUNK</b></span>
            <span className="badge outline">REJECTED</span>
          </div>
          <div className="dc-hd" style={{borderTop:'0.5px solid var(--rule-faint)', borderBottom: 0}}>{meta}<span>用户已拒绝 · 14:34</span></div>
          <DiffCardBody dim />
          <div className="dc-notice">
            <span className="glyph">⌫</span>
            <span>该建议已被拒绝。已记入时间轴。</span>
          </div>
        </div>
      );
    case 'expired':
      return (
        <div className="dc is-expired">
          <div className="dc-hd">
            <span className="who">BINDER · <b>1 HUNK</b></span>
            <span className="badge outline">EXPIRED</span>
          </div>
          <div className="dc-hd" style={{borderTop:'0.5px solid var(--rule-faint)', borderBottom: 0}}>{meta}<span>情况已变化 · 不可应用</span></div>
          <DiffCardBody dim />
          <div className="dc-notice">
            <span className="glyph gold">!</span>
            <div>
              <div><b style={{color:'var(--ink)'}}>原文已被手动编辑</b> · <span className="t-code" style={{fontSize:10, color:'var(--ink-4)'}}>document_revision_advanced</span></div>
              <div style={{marginTop:2}}>如需继续修改请重新向 AI 发起请求。</div>
            </div>
          </div>
          <div className="dc-actions">
            <button className="retry">重新生成</button>
            <button>关闭</button>
          </div>
        </div>
      );
    case 'conflict':
      return (
        <div className="dc is-conflict">
          <div className="dc-hd">
            <span className="who">BINDER · <b>1 HUNK</b></span>
            <span className="badge gold">CONFLICT</span>
          </div>
          <div className="dc-hd" style={{borderTop:'0.5px solid var(--rule-faint)', borderBottom: 0}}>{meta}<span>与 §3.4 的另一条 Diff 重叠</span></div>
          <DiffCardBody dim />
          <div className="dc-notice">
            <span className="glyph gold">⌖</span>
            <div>
              <div><b style={{color:'var(--ink)'}}>范围重叠</b>：与 <span className="t-code" style={{fontSize:11}}>diff_3a8c</span>（§3.4 ↘ §4.8）冲突。</div>
              <div style={{marginTop:2}}>请先处理其中之一。Accept 暂不可用，可单独拒绝。</div>
            </div>
          </div>
          <div className="dc-actions">
            <button className="accept disabled">接受 · 不可用</button>
            <button className="reject">拒绝 ⌫</button>
          </div>
        </div>
      );
    case 'error':
      return (
        <div className="dc is-error">
          <div className="dc-hd">
            <span className="who">BINDER · <b>1 HUNK</b></span>
            <span className="badge rust">ERROR</span>
          </div>
          <div className="dc-hd" style={{borderTop:'0.5px solid var(--rule-faint)', borderBottom: 0}}>{meta}<span>执行失败 · 14:35</span></div>
          <DiffCardBody dim />
          <div className="dc-notice">
            <span className="glyph rust">×</span>
            <div>
              <div><b style={{color:'var(--ink)'}}>写入失败</b> · <span className="t-code" style={{fontSize:10, color:'var(--ink-4)'}}>apply_replace_failed (E_IO)</span></div>
              <div style={{marginTop:2}}>磁盘暂不可写。已重试 2 次。</div>
            </div>
          </div>
          <div className="dc-actions">
            <button className="retry">重试 ↻</button>
            <button className="reject">拒绝 ⌫</button>
          </div>
        </div>
      );
    default:
      return null;
  }
}

function DiffStatesPage() {
  const M = window.MOCK;
  return (
    <Page code="02" name="Diff Card · 8 状态" sub="STATE MACHINE · 5.1">
      <div className="legend" style={{marginBottom: 14}}>
        <span className="legend-item"><span className="legend-dot r"></span>主动 · pending / error</span>
        <span className="legend-item"><span className="legend-dot y"></span>过程 · validating / applying / conflict</span>
        <span className="legend-item"><span className="legend-dot g"></span>完成 · accepted</span>
        <span className="legend-item"><span className="legend-dot k"></span>终态 · rejected / expired</span>
      </div>
      <div className="grid grid-2">
        {M.DIFF_STATES.map(s => (
          <ShowFrame key={s.id} code={s.code} name={s.label} hint={s.en} foot={s.hint}>
            <DiffCardShowcase state={s.id} />
          </ShowFrame>
        ))}
      </div>

      <div style={{marginTop: 24}}>
        <div className="page-hd-l" style={{padding: '0 0 12px'}}>
          <span className="code">02 · 06.x</span>
          <span className="name">Expired · 5 种失效原因</span>
        </div>
        <div className="grid grid-3">
          {M.EXPIRED_REASONS.map((r, i) => (
            <ShowFrame key={r.key} code={`06.${i+1}`} name={r.label} hint={r.en}>
              <div className="dc is-expired">
                <div className="dc-hd">
                  <span className="who">BINDER · <b>1 HUNK</b></span>
                  <span className="badge outline">EXPIRED</span>
                </div>
                <div className="dc-notice">
                  <span className="glyph gold">!</span>
                  <div>
                    <div><b style={{color:'var(--ink)'}}>{r.label}</b></div>
                    <div className="t-code" style={{fontSize:10, color:'var(--ink-4)', marginTop:2}}>{r.key}</div>
                  </div>
                </div>
                <div className="dc-actions">
                  <button className="retry">重新生成</button>
                  <button>关闭</button>
                </div>
              </div>
            </ShowFrame>
          ))}
        </div>
      </div>
    </Page>
  );
}

window.BinderShowcase = { Page, ShowFrame, DiffCardShowcase, DiffStatesPage };
