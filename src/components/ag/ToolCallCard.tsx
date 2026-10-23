import type { ToolCallView } from "../../store/agStore";

/**
 * ToolCallCard（ToolCall / TERM-AG-002 · 参考稿 §6.2）· PI-FE-06。
 * 6 状态视觉互斥：pending / executing / done / awaiting_confirmation / failed / skipped。
 * ToolCallReceipt 区分执行前拒绝与执行失败；实际确认操作由 DecisionBar 承担。
 */

const STATE_LABEL: Record<ToolCallView["state"], string> = {
  pending: "排队",
  executing: "执行中",
  done: "完成",
  awaiting_confirmation: "需授权",
  failed: "未完成",
  skipped: "跳过",
};

export function ToolCallCard({ tool }: { tool: ToolCallView }) {
  const { name, state, detail, executionReceipt } = tool;
  const stateLabel = executionReceipt?.executionStatus === "rejected"
    ? "受阻，未执行"
    : STATE_LABEL[state];
  return (
    <div className={`toolcard toolcard-${state}`}>
      <div className="toolcard-head">
        {state === "executing" && <span className="toolcard-spinner" aria-hidden />}
        <span className="toolcard-name">{name}</span>
        <span className="toolcard-state">{stateLabel}</span>
      </div>
      {detail && <div className="toolcard-detail">{detail}</div>}
      {state === "executing" && (
        <div className="toolcard-bar">
          <span style={{ width: "62%" }} />
        </div>
      )}
    </div>
  );
}
