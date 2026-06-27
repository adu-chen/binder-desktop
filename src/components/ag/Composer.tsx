import { useState } from "react";
import { useAgStore } from "../../store/agStore";
import { startTurn } from "../../services/agService";
import { generateSampleDiff } from "../../services/deService";
import { useEdStore } from "../../store/edStore";
import { useWsStore } from "../../store/wsStore";

/**
 * Composer（A-SYS-C-T-02 §3 对话区输入 · PI-FE-06）。
 * 输入 + 发送；⌘/Ctrl+Enter 发送。发送中禁用（投影 sending 态）。
 * 文档引用（@mention，PI-AG-09E）：选工作区文档作显式引用，随消息高优先注入。
 */
export function Composer() {
  const { sending } = useAgStore();
  const { openPath } = useEdStore();
  const { docs } = useWsStore();
  const [text, setText] = useState("");
  const [mentions, setMentions] = useState<string[]>([]);
  const [pickerOpen, setPickerOpen] = useState(false);

  const fileDocs = docs.filter((d) => !d.isDir);

  const send = () => {
    if (sending || !text.trim()) return;
    void startTurn(text, mentions.length ? { mentions } : undefined);
    setText("");
    setMentions([]);
    setPickerOpen(false);
  };

  const toggleMention = (relPath: string) => {
    setMentions((cur) => (cur.includes(relPath) ? cur.filter((p) => p !== relPath) : [...cur, relPath]));
  };

  return (
    <div className="composer">
      {mentions.length > 0 && (
        <div className="composer-mentions">
          {mentions.map((p) => (
            <span key={p} className="mention-chip">
              @{p}
              <button className="mention-chip-x" onClick={() => toggleMention(p)} aria-label="移除引用">
                ×
              </button>
            </span>
          ))}
        </div>
      )}

      {pickerOpen && (
        <div className="mention-picker">
          {fileDocs.length === 0 && <p className="muted">工作区暂无文档</p>}
          {fileDocs.map((d) => (
            <button
              key={d.relPath}
              className={`mention-option${mentions.includes(d.relPath) ? " selected" : ""}`}
              onClick={() => toggleMention(d.relPath)}
            >
              {mentions.includes(d.relPath) ? "✓ " : ""}
              {d.relPath}
            </button>
          ))}
        </div>
      )}

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
        <button
          className="ghost composer-mention-toggle"
          onClick={() => setPickerOpen((v) => !v)}
          title="引用工作区文档（随消息高优先提供给 AI）"
        >
          @引用{mentions.length ? `（${mentions.length}）` : ""}
        </button>
        {/* 示例修改入口：绕过模型，对当前文档生成真实可落盘 diff（PI-FE-11） */}
        <button
          className="ghost composer-sample"
          onClick={() => void generateSampleDiff()}
          disabled={!openPath}
          title={openPath ? "对当前文档生成可落盘的示例修改" : "先打开一个文档"}
        >
          示例修改
        </button>
        <button onClick={send} disabled={sending || !text.trim()}>
          {sending ? "生成中…" : "发送"}
        </button>
      </div>
    </div>
  );
}
