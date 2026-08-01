import { useEffect, useRef, useState } from "react";
import { useAgStore } from "../../store/agStore";
import { MessageList } from "./MessageList";
import { Composer } from "./Composer";
import { DiffAllActionsBar } from "../de/DiffAllActionsBar";
import { writeReferencePayloadData } from "../../services/referencePayload";
import { closeAgentSession, deleteAgentSession, newAgentSession, switchAgentSession } from "../../services/agService";

/**
 * 代理编辑面板 AgentPanel（A-SYS-C-T-02 §3 · PI-FE-06 / PI-AG-UI-01）。
 * 顶部=多会话标签栏（新建/关闭/历史重开，运行期内存态，持久化随 BR-AG-PERSIST-001 接线批）；
 * 中部=消息流（DiffCard 嵌入对应回合消息内）；输入框上方=批量裁决条（无待裁决自隐藏）；底部=Composer。
 * 编辑/构建模式切换已移入 Composer 框内（原顶部切换移除，逻辑同源 agStore session.mode）。
 * AI 调用前经订阅门禁硬门（X-SEQ-001，后端 ensure_gate_passed）；客户端不持密钥（X-FORBID-001）。
 */
export function AgentPanel() {
  const { sessions, activeSessionId, sending } = useAgStore();
  const [histOpen, setHistOpen] = useState(false);
  const [deletingSessionId, setDeletingSessionId] = useState<string | null>(null);
  const historyMenuRef = useRef<HTMLDivElement>(null);
  const open = sessions.filter((s) => !s.archived);
  const history = [...sessions].sort((a, b) => b.createdAt - a.createdAt);

  useEffect(() => {
    if (!histOpen) return;
    function handleDocumentMouseDown(event: MouseEvent) {
      const target = event.target;
      if (target instanceof Node && historyMenuRef.current?.contains(target)) return;
      setHistOpen(false);
    }
    document.addEventListener("mousedown", handleDocumentMouseDown);
    return () => document.removeEventListener("mousedown", handleDocumentMouseDown);
  }, [histOpen]);

  async function handleDeleteSession(sessionId: string, title: string) {
    if (deletingSessionId || sending) return;
    const confirmed = window.confirm(
      `确认删除会话“${title}”？该操作会删除此会话已持久化的聊天记录，待裁决修改不会被删除。`,
    );
    if (!confirmed) return;
    setDeletingSessionId(sessionId);
    try {
      await deleteAgentSession(sessionId);
    } catch (e) {
      console.error(`ag.delete_session failed ${JSON.stringify(e)}`);
      window.alert("删除代理编辑会话失败，请稍后重试。");
    } finally {
      setDeletingSessionId(null);
    }
  }

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
              onClick={() => switchAgentSession(s.id)}
              title={s.title}
            >
              <span className="agtab-title">{s.title}</span>
              <button
                className="agtab-x"
                aria-label="关闭会话"
                onClick={(e) => {
                  e.stopPropagation();
                  closeAgentSession(s.id);
                }}
              >
                ×
              </button>
            </div>
          ))}
        </div>
        <div className="agtabs-ops">
          <button className="agtabs-op" title="新建对话" onClick={() => newAgentSession()}>
            ＋
          </button>
          <div className="agtabs-hist-wrap" ref={historyMenuRef}>
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
                  <div key={s.id} className="agtabs-history-item">
                    <button
                      type="button"
                      style={{
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "space-between",
                        gap: "var(--sp-2)",
                        flex: 1,
                        minWidth: 0,
                        border: "none",
                        background: "transparent",
                        color: "inherit",
                        textAlign: "left",
                        padding: 0,
                        cursor: "pointer",
                      }}
                      onClick={() => {
                        switchAgentSession(s.id);
                        setHistOpen(false);
                      }}
                    >
                      <span className="agtabs-history-title">{s.title}</span>
                      <span className="label">
                        {s.id === activeSessionId ? "当前" : s.archived ? "已关闭" : "打开中"}
                      </span>
                    </button>
                    <button
                      className="agtab-x"
                      aria-label={`删除会话 ${s.title}`}
                      disabled={Boolean(deletingSessionId) || sending}
                      onClick={(e) => {
                        e.stopPropagation();
                        void handleDeleteSession(s.id, s.title);
                      }}
                    >
                      {deletingSessionId === s.id ? "..." : "×"}
                    </button>
                  </div>
                ))}
                <p className="muted agtabs-history-note">
                  历史保存在本地 workspace.db；未打开标签点击后加载内容
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
