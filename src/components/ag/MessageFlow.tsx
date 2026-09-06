import type { ChatMessage, ProjectReportView, ToolCallView } from "../../store/agStore";
import type { DiffView } from "../../store/deStore";
import { MessageMarkdown } from "./MessageMarkdown";
import { ToolCallCard } from "./ToolCallCard";
import { DiffList } from "../de/DiffList";
import { ProjectReportCard } from "./ProjectReportCard";

/**
 * @GOV
 * codes:    BR-AG-UI-001-RB-AG-AG-TURN-013
 * type:     RB
 * chain:    AG-TURN
 * rules:    BR-AG-UI-001
 * term_ref: TERM-AG-011
 * boundary: in=AssistantMessageWithSegments(toolCallRefsDiffRefs) | out=orderedTextToolDiffFlow | effect=memory | caller_context=messageFlowRender | does_not=render_user_message_as_markdown_or_alter_turn_tool_diff_state
 */
export function MessageFlow({
  message,
  toolCalls,
  diffs,
  report,
}: {
  message: ChatMessage;
  toolCalls: ToolCallView[];
  diffs: DiffView[];
  report?: ProjectReportView;
}) {
  const segments = message.segments;

  // 错误消息 / 无片段信息（legacy 历史）→ 降级：整段正文 + 全部卡片沉底，不丢数据（BR-AG-PERSIST-001）。
  if (message.error || !segments?.length) {
    return (
      <>
        <div className="msg-body">
          {message.error ? message.content : <MessageMarkdown content={message.content} />}
          {message.streaming && <span className="msg-caret" aria-hidden />}
        </div>
        <CardBlock toolCalls={toolCalls} diffs={diffs} />
        {report && <ProjectReportCard report={report} />}
      </>
    );
  }

  const coveredToolIds = new Set<string>();
  const coveredDiffIds = new Set<string>();
  const lastTextIndex = segments.length - 1;

  return (
    <>
      {segments.map((segment, index) => {
        switch (segment.kind) {
          case "text":
            return (
              <div className="msg-body" key={index}>
                <MessageMarkdown content={segment.text} />
                {message.streaming && index === lastTextIndex && <span className="msg-caret" aria-hidden />}
              </div>
            );
          case "tool": {
            const tool = toolCalls.find((t) => t.id === segment.toolCallId);
            if (!tool) return null;
            coveredToolIds.add(tool.id);
            return <ToolCallCard key={index} tool={tool} />;
          }
          case "diff": {
            const diff = diffs.find((d) => d.diffId === segment.diffId);
            if (!diff) return null;
            coveredDiffIds.add(diff.diffId);
            return <DiffList key={index} diffs={[diff]} />;
          }
          default:
            return null;
        }
      })}
      {/* 未被任何段引用的卡沉底兜底（恢复期中间态 / 段引用尚未就绪），避免丢失。 */}
      <CardBlock
        toolCalls={toolCalls.filter((t) => !coveredToolIds.has(t.id))}
        diffs={diffs.filter((d) => !coveredDiffIds.has(d.diffId))}
      />
      {report && <ProjectReportCard report={report} />}
    </>
  );
}

function CardBlock({ toolCalls, diffs }: { toolCalls: ToolCallView[]; diffs: DiffView[] }) {
  return (
    <>
      {toolCalls.length > 0 && (
        <div className="toolcalls">
          <span className="label">工具调用</span>
          {toolCalls.map((t) => (
            <ToolCallCard key={t.id} tool={t} />
          ))}
        </div>
      )}
      {diffs.length > 0 && <DiffList diffs={diffs} />}
    </>
  );
}
