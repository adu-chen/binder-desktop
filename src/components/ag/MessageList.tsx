import { Fragment, useEffect, useRef } from "react";
import { activeSession, useAgStore, type ToolCallView } from "../../store/agStore";
import { useDeStore } from "../../store/deStore";
import type { MessageContentPart, ReferencePayload } from "../../services/referencePayload";
import { DiffList } from "../de/DiffList";
import { ToolCallCard } from "./ToolCallCard";
import { startTurn } from "../../services/agService";

/**
 * 消息流（A-SYS-C-T-02 §3 · PI-FE-06 / PI-AG-UI-01）。
 * 渲染活动会话的 AgentTurn 消息（含流式回显光标）+ ToolCall 状态卡。
 * 人类消息靠右、AI 消息靠左（需求2，CSS .msg-user 对齐控制）。
 */
export function MessageList() {
  const state = useAgStore();
  const { diffs } = useDeStore();
  const session = activeSession(state);
  const { messages, toolCalls } = session;
  const endRef = useRef<HTMLDivElement>(null);
  const sessionDiffs = diffs
    .filter((d) => d.sessionId === session.id)
    .sort((a, b) => (a.createdAt ?? 0) - (b.createdAt ?? 0));
  const fallbackDiffs = sessionDiffs.filter((d) => !d.anchorMessageId);
  const diffsForMessage = (messageId: string) => sessionDiffs.filter((d) => d.anchorMessageId === messageId);
  const fallbackToolCalls = toolCalls.filter((t) => !t.anchorMessageId);
  const toolCallsForMessage = (messageId: string) => [
    ...toolCalls.filter((t) => t.anchorMessageId === messageId),
    ...(messages.find((m) => m.id === messageId)?.toolCalls ?? []),
  ];

  useEffect(() => {
    if (typeof endRef.current?.scrollIntoView === "function") {
      endRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [messages, diffs]);

  return (
    <div className="msglist">
      {messages.length === 0 && (
        <p className="muted msglist-empty">向 AI 描述你想要的修改，生成的改动将以 Diff 卡呈现，确认后才落盘。</p>
      )}
      {messages.map((m) => (
        <Fragment key={m.id}>
          <div className={`msg msg-${m.role}${m.error ? " msg-error" : ""}`}>
            <span className="label msg-role">{m.role === "user" ? "你" : "AI"}</span>
            <div className="msg-body">
              {m.parts?.length ? <MessageParts parts={m.parts} /> : m.content}
              {m.streaming && <span className="msg-caret" aria-hidden />}
            </div>
          </div>
          <ToolCallList tools={toolCallsForMessage(m.id)} />
          <DiffList diffs={diffsForMessage(m.id)} />
          {m.role === "user" && m.turnStatus === "draft" && (
            <div className="msg msg-assistant msg-error">
              <span className="label msg-role">AI</span>
              <div className="msg-body">
                本回合未完成。
                <button className="ghost" onClick={() => void startTurn(m.content, { displayParts: m.parts, references: referencesFromParts(m.parts) })}>
                  重试
                </button>
              </div>
            </div>
          )}
        </Fragment>
      ))}

      <DiffList diffs={fallbackDiffs} label="未归属修改建议" />

      {fallbackToolCalls.length > 0 && (
        <div className="toolcalls">
          <span className="label">未归属工具调用</span>
          {fallbackToolCalls.map((t) => (
            <ToolCallCard key={t.id} tool={t} />
          ))}
        </div>
      )}

      <div ref={endRef} />
    </div>
  );
}

function ToolCallList({ tools }: { tools: ToolCallView[] }) {
  if (!tools.length) return null;
  return (
    <div className="toolcalls">
      <span className="label">工具调用</span>
      {tools.map((t) => (
        <ToolCallCard key={t.id} tool={t} />
      ))}
    </div>
  );
}

function referencesFromParts(parts?: MessageContentPart[]): ReferencePayload[] {
  return (parts ?? []).flatMap((part) => (part.type === "reference" ? [part.payload] : []));
}

function MessageParts({ parts }: { parts: MessageContentPart[] }) {
  return (
    <>
      {parts.map((part, index) =>
        part.type === "text" ? (
          <span key={index}>{part.text}</span>
        ) : (
          <span
            key={`${part.refId}-${index}`}
            className={`ref-token ref-token-${part.kind}`}
            data-reference-token="true"
            title={part.label}
          >
            {part.label}
          </span>
        ),
      )}
    </>
  );
}
