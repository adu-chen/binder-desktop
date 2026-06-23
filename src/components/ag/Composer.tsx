import { useState } from "react";
import { useAgStore } from "../../store/agStore";
import { startTurn } from "../../services/agService";

/**
 * Composer（A-SYS-C-T-02 §3 对话区输入 · PI-FE-06）。
 * 输入 + 发送；⌘/Ctrl+Enter 发送。发送中禁用（投影 sending 态）。
 * ⌘K 内联辅助 / ⌘J 自动补全为独立入口（不经此，归 PI-FE-08）。
 */
export function Composer() {
  const { sending } = useAgStore();
  const [text, setText] = useState("");

  const send = () => {
    if (sending || !text.trim()) return;
    void startTurn(text);
    setText("");
  };

  return (
    <div className="composer">
      <textarea
        className="composer-input"
        placeholder="描述你想要的修改…"
        value={text}
        onChange={(e) => setText(e.target.value)}
        onKeyDown={(e) => {
          if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
            e.preventDefault();
            send();
          }
        }}
        rows={3}
      />
      <div className="composer-actions">
        <span className="label">⌘↵ 发送</span>
        <button onClick={send} disabled={sending || !text.trim()}>
          {sending ? "生成中…" : "发送"}
        </button>
      </div>
    </div>
  );
}
