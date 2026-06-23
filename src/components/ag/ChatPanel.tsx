import { useAgStore, agStore, type ChatMode } from "../../store/agStore";
import { MessageList } from "./MessageList";
import { Composer } from "./Composer";

/**
 * Chat 面板（A-SYS-C-T-02 §3 对话区 · PI-FE-06）。
 * chat/agent 模式切换 + 会话标签 + 消息流 + composer。
 * AI 调用前经订阅门禁硬门（X-SEQ-001，后端 ensure_gate_passed）；客户端不持密钥（X-FORBID-001）。
 */
const MODES: { id: ChatMode; label: string }[] = [
  { id: "chat", label: "对话" },
  { id: "agent", label: "构建" },
];

export function ChatPanel() {
  const { mode } = useAgStore();
  return (
    <div className="chatpanel">
      <div className="chatpanel-head">
        <div className="chatpanel-modes">
          {MODES.map((m) => (
            <button
              key={m.id}
              className={`chatmode${mode === m.id ? " active" : ""}`}
              onClick={() => agStore.setMode(m.id)}
              aria-pressed={mode === m.id}
            >
              {m.label}
            </button>
          ))}
        </div>
        <span className="label">会话 · 1</span>
      </div>
      <MessageList />
      <Composer />
    </div>
  );
}
