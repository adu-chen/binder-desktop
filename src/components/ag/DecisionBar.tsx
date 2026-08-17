import { resolveDecision, resolveDecisionAndContinue } from "../../services/agService";
import { agStore, useAgStore } from "../../store/agStore";

/**
 * DecisionBar（TERM-AG-017 · AG-1.0-DECISION-CARD-01）。
 * 任务级裁决的独立固定裁决区（Composer 上方）：
 * - 回合 WaitingConfirm 时模型已停止生成，任务暂停等待用户决策；
 * - 类型化选项：permission（允许本次/拒绝）、plan_choice/scope/manual_confirm（确认并继续/取消）；
 * - 裁决后 confirm → 自动续下一执行片段；cancel → 终止；
 * - 存在待裁决时 Composer 输入门控（见 Composer.send）。
 * 与 DiffCard（内容确认）完全分离：本区只承担授权/决策，不承担内容审阅。
 */

const KIND_LABEL: Record<string, string> = {
  permission: "需要权限",
  plan_choice: "方案确认",
  scope: "范围确认",
  manual_confirm: "人工确认",
};

export function DecisionBar() {
  const state = useAgStore();
  const decision = state.pendingDecision;
  if (!decision) return null;

  const label = KIND_LABEL[decision.kind] ?? "任务确认";
  const summary =
    typeof decision.payload === "object" && decision.payload && "summary" in decision.payload
      ? String((decision.payload as { summary: string }).summary)
      : decision.payload
        ? JSON.stringify(decision.payload)
        : "";

  const act = async (outcome: "confirm" | "cancel") => {
    try {
      if (outcome === "confirm") {
        await resolveDecisionAndContinue(decision.requestId, outcome);
      } else {
        await resolveDecision(decision.requestId, outcome);
      }
    } finally {
      agStore.setPendingDecision(null);
    }
  };

  return (
    <div className={`decisionbar decisionbar-${decision.kind}`}>
      <div className="decisionbar-head">
        <span className="decisionbar-title">等待你的决策</span>
        <span className="decisionbar-kind">{label}</span>
      </div>
      {summary && <div className="decisionbar-summary">{summary}</div>}
      <div className="decisionbar-actions">
        <button className="decisionbar-primary" onClick={() => void act("confirm")}>
          {decision.kind === "permission" ? "允许本次" : "确认并继续"}
        </button>
        <button className="ghost" onClick={() => void act("cancel")}>
          {decision.kind === "permission" ? "拒绝" : "取消"}
        </button>
      </div>
      <div className="decisionbar-note">任务已暂停等待你的决策；完成决策后继续执行。</div>
    </div>
  );
}
