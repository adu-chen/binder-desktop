import { useState } from "react";
import { useAgStore, agStore } from "../../store/agStore";
import { MessageList } from "./MessageList";
import { Composer } from "./Composer";
import { DiffAllActionsBar } from "../de/DiffAllActionsBar";
import { writeReferencePayloadData } from "../../services/referencePayload";

/**
 * 代理编辑面板 AgentPanel（A-SYS-C-T-02 §3 · PI-FE-06 / PI-AG-UI-01）。
 * 顶部=多会话标签栏（新建/关闭/历史重开，运行期内存态，持久化随 BR-AG-PERSIST-001 接线批）；
 * 中部=消息流（DiffCard 嵌入对应回合消息内）；输入框上方=批量裁决条（无待裁决自隐藏）；底部=Composer。
 * 编辑/构建模式切换已移入 Composer 框内（原顶部切换移除，逻辑同源 agStore session.mode）。
 * AI 调用前经订阅门禁硬门（X-SEQ-001，后端 ensure_gate_passed）；客户端不持密钥（X-FORBID-001）。
 */
export function AgentPanel() {
  const { sessions, activeSessionId } = useAgStore();
  const [histOpen, setHistOpen] = useState(false);
  const open = sessions.filter((s) => !s.archived);
  const history = [...sessions].sort((a, b) => b.createdAt - a.createdAt);

  return (
    <div className="chatpanel">
      <div className="agtabs">
        <div className="agtabs-list" role="tablist">
          {open.map((s) => (
            <div
              key={s.id}
              role="tab"
              aria-selected={s.id === activeSessionId}
              className={`agtab${s.id === activeSessionId ? " active" : ""}`}
              draggable
              onDragStart={(e) =>
                writeReferencePayloadData(e.dataTransfer, {
                  kind: "session",
                  sessionId: s.id,
                  title: s.title,
                  messages: s.messages
                    .filter((m) => !m.error && m.content.trim())
                    .map((m) => ({ role: m.role, content: m.content })),
                })
              }
              onClick={() => agStore.switchSession(s.id)}
              title={s.title}
            >
              <span className="agtab-title">{s.title}</span>
              <button
                className="agtab-x"
                aria-label="关闭会话"
                onClick={(e) => {
                  e.stopPropagation();
                  agStore.closeSession(s.id);
                }}
              >
                ×
              </button>
            </div>
          ))}
        </div>
        <div className="agtabs-ops">
          <button className="agtabs-op" title="新建对话" onClick={() => agStore.newSession()}>
            ＋
          </button>
          <div className="agtabs-hist-wrap">
            <button
              className={`agtabs-op${histOpen ? " active" : ""}`}
              title="历史会话"
              onClick={() => setHistOpen((v) => !v)}
            >
              🕘
            </button>
            {histOpen && (
              <div className="agtabs-history">
                {history.map((s) => (
                  <button
                    key={s.id}
                    className="agtabs-history-item"
                    onClick={() => {
                      agStore.switchSession(s.id);
                      setHistOpen(false);
                    }}
                  >
                    <span className="agtabs-history-title">{s.title}</span>
                    <span className="label">
                      {s.id === activeSessionId ? "当前" : s.archived ? "已关闭" : "打开中"}
                    </span>
                  </button>
                ))}
                <p className="muted agtabs-history-note">
                  历史为本次运行期会话；跨重启持久化随历史批接入
                </p>
              </div>
            )}
          </div>
        </div>
      </div>

      <div className="chatpanel-scroll">
        <MessageList />
      </div>

      {/* diffedit 批量裁决条（PI-AG-UI-01 需求3）：仅在有待裁决/应用中呈现，组件内自隐藏。 */}
      <DiffAllActionsBar />

      <Composer />
    </div>
  );
}
