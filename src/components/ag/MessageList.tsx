import { Fragment, useEffect, useRef } from "react";
import { activeSession, useAgStore, type TurnCurrentAction } from "../../store/agStore";
import { useDeStore } from "../../store/deStore";
import type { MessageContentPart, ReferencePayload } from "../../services/referencePayload";
import { DiffList } from "../de/DiffList";
import { ToolCallCard } from "./ToolCallCard";
import { PlanCard } from "./PlanCard";
import { MessageFlow } from "./MessageFlow";
import { startTurn } from "../../services/agService";

/**
 * @GOV
 * codes:    BR-AG-UI-001-RB-AG-AG-TURN-014
 * type:     RB
 * chain:    AG-TURN
 * rules:    BR-AG-UI-001
 * term_ref: TERM-AG-011
 * boundary: in=ActiveSessionMessagesToolCallsAndAnchoredDiffs | out=orderedMessageStreamRender | effect=memory | caller_context=agentPanelMessageListRender | does_not=render_user_message_as_markdown_or_move_diffs_off_turn
 *
 * 消息流（A-SYS-C-T-02 §3 · PI-FE-06 / PI-AG-UI-01 · AG-MSG-RENDER-01）。
 * 渲染活动会话的 AgentTurn 消息（含流式回显光标）+ ToolCall 状态卡。
 * assistant 消息经 MessageFlow 按回合内时序（text/tool/diff 消息片段）交错渲染；
 * 人类消息靠右、AI 消息靠左（需求2，CSS .msg-user 对齐控制）。
 */
export function MessageList() {
  const state = useAgStore();
  const { diffs } = useDeStore();
  const session = activeSession(state);
  const { messages, toolCalls } = session;
  const endRef = useRef<HTMLDivElement>(null);
  /** 贴底跟随标记（修复流式会话流跳动）：用户向上滚动离开底部阈值后暂停自动拉底，回到底部后恢复。
   *  流式 chunk 高频到达时若每次都 smooth 滚动，动画互相打断会造成上下抖动，故贴底时用非动画直接置底。 */
  const stickToBottomRef = useRef(true);
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

  // 滚动容器（.chatpanel-scroll）贴底状态跟踪：用户向上滚动查看历史/卡片 → 暂停自动跟随；
  // 回到底部阈值内 → 恢复跟随。避免流式/卡片更新把用户强制拉回底部造成会话流跳动。
  useEffect(() => {
    const el = endRef.current?.closest(".chatpanel-scroll") as HTMLElement | null;
    if (!el) return;
    const update = () => {
      stickToBottomRef.current = el.scrollHeight - el.scrollTop - el.clientHeight < 48;
    };
    el.addEventListener("scroll", update, { passive: true });
    update();
    return () => el.removeEventListener("scroll", update);
  }, []);

  // 内容变化（流式 chunk / 工具卡 / diff 卡）时：仅贴底状态跟随，直接置底（无动画，避免 smooth 竞争抖动）。
  useEffect(() => {
    if (!stickToBottomRef.current) return;
    const el = endRef.current?.closest(".chatpanel-scroll") as HTMLElement | null;
    if (!el) return;
    el.scrollTop = el.scrollHeight;
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
            {m.role === "assistant" ? (
              <MessageFlow message={m} toolCalls={toolCallsForMessage(m.id)} diffs={diffsForMessage(m.id)} />
            ) : (
              <div className="msg-body">
                {m.parts?.length ? <MessageParts parts={m.parts} /> : m.content}
                {m.streaming && <span className="msg-caret" aria-hidden />}
              </div>
            )}
          </div>
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

      {/* AG-1.0-TASKRUN-REWORK-CONTINUE-01：任务计划呈现（TERM-AG-018，可展示操作计划；最近计划）。 */}
      {session.taskPlans && session.taskPlans.length > 0 && (
        <PlanCard key={session.taskPlans[0].planId} plan={session.taskPlans[0]} />
      )}

      {/* AG-1.0-DECISION-CARD-01：任务级裁决已迁移至 Composer 上方的 DecisionBar（独立固定区），
          信息流内不再渲染裁决卡；此处仅保留实时当前动作状态条。 */}
      {state.currentAction && !state.pendingDecision && (
        <div className="turn-statusbar">
          <span className="turn-statusbar-spinner" aria-hidden />
          <span className="turn-statusbar-label">{currentActionLabel(state.currentAction)}</span>
        </div>
      )}

      <div ref={endRef} />
    </div>
  );
}

/** 实时当前动作人读文案（step_progress action → 中文）。 */
function currentActionLabel(action: TurnCurrentAction): string {
  switch (action.action) {
    case "reading":
      return `正在读取 ${action.label}`;
    case "tool":
      return `正在执行工具 ${action.label}`;
    case "generating":
      return "正在生成回复…";
    case "proposing":
      return `正在生成修改建议 ${action.label}`;
    case "deciding":
      return action.label || "等待裁决…";
    case "previewing":
      return action.label || "正在等待编辑器预览…";
  }
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
