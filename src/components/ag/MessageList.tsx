import { useEffect, useRef } from "react";
import { useAgStore } from "../../store/agStore";
import { ToolCallCard } from "./ToolCallCard";

/**
 * 消息流（A-SYS-C-T-02 §3 对话区 · PI-FE-06）。
 * 渲染 AgentTurn 会话消息（含流式回显光标）+ ToolCall 占位演示卡（6 态）。
 */
export function MessageList() {
  const { messages, toolCalls } = useAgStore();
  const endRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    endRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  return (
    <div className="msglist">
      {messages.length === 0 && (
        <p className="muted msglist-empty">向 AI 描述你想要的修改，生成的改动将以 Diff 卡呈现，确认后才落盘。</p>
      )}
      {messages.map((m) => (
        <div key={m.id} className={`msg msg-${m.role}${m.error ? " msg-error" : ""}`}>
          <span className="label msg-role">{m.role === "user" ? "你" : "AI"}</span>
          <div className="msg-body">
            {m.content}
            {m.streaming && <span className="msg-caret" aria-hidden />}
          </div>
        </div>
      ))}

      <div className="toolcalls">
        <span className="label">工具调用 · 6 态（占位演示）</span>
        {toolCalls.map((t) => (
          <ToolCallCard key={t.id} tool={t} />
        ))}
      </div>

      <div ref={endRef} />
    </div>
  );
}
