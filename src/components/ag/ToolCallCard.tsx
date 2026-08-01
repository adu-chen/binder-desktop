import type { ToolCallView } from "../../store/agStore";

/**
 * ToolCallCard（ToolCall / TERM-AG-002 · 参考稿 §6.2）· PI-FE-06。
 * 6 状态视觉互斥：pending（排队）/ executing（旋转器+进度）/ done（OK+耗时）/
 * awaiting_confirmation（危险授权门，锈红边，需用户授权）/ failed（可重试/跳过）/ skipped。
 * 本 PI 以占位演示态呈现；真实工具回合由 SM-AG-01 tool 段驱动（后续 PI）。
 */

const STATE_LABEL: Record<ToolCallView["state"], string> = {
  pending: "排队",
  executing: "执行中",
  done: "完成",
  awaiting_confirmation: "需授权",
  failed: "失败",
  skipped: "跳过",
};

export function ToolCallCard({ tool }: { tool: ToolCallView }) {
  const { name, state, detail, readonly } = tool;
  return (
    <div className={`toolcard toolcard-${state}`}>
      <div className="toolcard-head">
        {state === "executing" && <span className="toolcard-spinner" aria-hidden />}
        <span className="toolcard-name">{name}</span>
        <span className="toolcard-state">{STATE_LABEL[state]}</span>
      </div>
      {detail && <div className="toolcard-detail">{detail}</div>}
      {state === "executing" && (
        <div className="toolcard-bar">
          <span style={{ width: "62%" }} />
        </div>
      )}
      {state === "awaiting_confirmation" && !readonly && (
        <div className="toolcard-actions">
          <button className="toolcard-allow">授权</button>
          <button className="ghost">拒绝</button>
        </div>
      )}
      {state === "failed" && !readonly && (
        <div className="toolcard-actions">
          <button>重试</button>
          <button className="ghost">跳过</button>
        </div>
      )}
    </div>
  );
}
