import { useRef, useState } from "react";
import { resolveDecisionAndContinue } from "../../services/agService";
import { activeSession, agStore, useAgStore, type PendingDecisionView, type TurnCurrentAction } from "../../store/agStore";

/**
 * DecisionBar（TERM-AG-017 · AG-1.0-DECISION-CARD-01）。
 * 任务级裁决的独立固定裁决区（Composer 上方）：
 * - 回合 WaitingConfirm 时模型已停止生成，任务暂停等待用户决策；
 * - 类型化选项：permission（允许本次/拒绝）、plan_choice/scope/manual_confirm（确认并继续/取消）；
 * - confirm/cancel 都作为合法用户事件恢复产生该卡片的原 AgentTurn，由模型据实收束；
 * - 存在待裁决时 Composer 输入门控（见 Composer.send）。
 * 与 DiffCard（内容确认）完全分离：本区只承担授权/决策，不承担内容审阅。
 */

const KIND_LABEL: Record<string, string> = {
  permission: "需要权限",
  plan_choice: "方案确认",
  scope: "范围确认",
  manual_confirm: "人工确认",
};

const OPERATION_LABEL: Record<string, string> = {
  create_document: "新建文档",
  create_folder: "新建文件夹",
  rename_resource: "重命名",
  move_resource: "移动",
  delete_resource: "删除",
  restore_resource: "恢复",
};

export function DecisionBar() {
  const state = useAgStore();
  const decision = state.pendingDecision;
  if (!decision || (decision.sessionId && decision.sessionId !== activeSession(state).id)) return null;
  const sessionId = decision.sessionId ?? activeSession(state).id;
  return <DecisionRequestControls key={JSON.stringify([sessionId, decision.turnId, decision.requestId])} decision={decision} sessionId={sessionId} />;
}

/**
 * @GOV
 * codes: BR-SYS-STATE-007-EFFECT-AG-AG-TURN-058, BR-AG-UI-001-EFFECT-AG-AG-TURN-058
 * type: EFFECT
 * chain: AG-TURN
 * rules: BR-SYS-STATE-007, BR-AG-UI-001
 * term_ref: TERM-AG-017
 * boundary: in=DecisionRequestAndOriginSession | out=IdentityScopedSubmissionFeedbackAndReceiptMessages | does_not=clearSuccessorDecisionOrRetargetReceiptToActiveSession
 */
function DecisionRequestControls({ decision, sessionId }: { decision: PendingDecisionView; sessionId: string }) {
  const state = useAgStore();
  const submitting = useRef(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  const label = KIND_LABEL[decision.kind] ?? "任务确认";
  const summary =
    typeof decision.payload === "object" && decision.payload && "summary" in decision.payload
      ? String((decision.payload as { summary: string }).summary)
      : decision.payload
        ? JSON.stringify(decision.payload)
        : "";
  const payload = typeof decision.payload === "object" && decision.payload
    ? decision.payload as Record<string, unknown>
    : null;
  const isWorkspaceOperation = typeof payload?.workspaceOperationPlanId === "string";
  const detail = typeof payload?.detail === "string" ? payload.detail : "";
  const operations = Array.isArray(payload?.operations)
    ? payload.operations.filter((item): item is Record<string, unknown> => Boolean(item && typeof item === "object"))
    : [];
  const consequences = payload?.consequences && typeof payload.consequences === "object"
    ? payload.consequences as Record<string, unknown>
    : null;
  const affectedSessions = Array.isArray(consequences?.affected_sessions) ? consequences.affected_sessions.map(String) : [];
  const affectedDiffs = Array.isArray(consequences?.affected_diffs) ? consequences.affected_diffs.map(String) : [];
  const operationSources = operations.flatMap((operation) => (
    typeof operation.sourcePath === "string"
      ? [{ path: operation.sourcePath, folder: operation.resourceKind === "folder" }]
      : []
  ));
  const draftAffectedReferences = state.sessions.flatMap((session) => (
    session.composerDraft?.tokens.flatMap((token) => {
      if (token.payload.kind !== "doc" && token.payload.kind !== "selection") return [];
      const relPath = token.payload.relPath;
      return operationSources.some((source) => (
        relPath === source.path
        || (source.folder && relPath.startsWith(`${source.path}/`))
      )) ? [token.label] : [];
    }) ?? []
  ));
  const affectedReferences = [...new Set([
    ...(Array.isArray(consequences?.affected_references) ? consequences.affected_references.map(String) : []),
    ...draftAffectedReferences,
  ])];
  const recursiveCount = typeof consequences?.recursive_count === "number" ? consequences.recursive_count : 0;
  const recoverable = typeof payload?.recoverable === "boolean" ? payload.recoverable : consequences?.recoverable;
  const failurePolicy = typeof payload?.failurePolicy === "string" ? payload.failurePolicy : "";

  const appendOperationMessage = (content: string, error = false) => {
    agStore.pushMessage(sessionId, {
      id: `workspace-operation-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
      role: "assistant",
      content,
      error,
    });
  };

  const act = async (outcome: "confirm" | "cancel") => {
    if (submitting.current || agStore.getSnapshot().pendingDecision !== decision) return;
    submitting.current = true;
    setBusy(true);
    setError("");
    const action: TurnCurrentAction = {
      sessionId,
      turnId: decision.turnId ?? `decision:${decision.requestId}`,
      action: "deciding",
      label: isWorkspaceOperation && outcome === "confirm" ? "正在核验并执行资源操作" : "正在提交你的决定",
    };
    agStore.setCurrentAction(action);
    try {
      const resolution = await resolveDecisionAndContinue(decision.requestId, outcome);
      const receipt = resolution.workspaceOperationReceipt;
      if (receipt && receipt.final_status !== "completed" && receipt.final_status !== "awaiting_content") {
        const failed = receipt.item_receipts.filter((item) => item.status === "failed");
        const compensated = receipt.compensation_receipts.filter((item) => item.status === "compensated");
        appendOperationMessage(
          `资源操作未完整完成：${receipt.final_status}。已补偿 ${compensated.length} 项，失败 ${failed.length} 项。${failed.map((item) => `${item.item_id}${item.error_message ? `：${item.error_message}` : ""}`).join("；")}`,
          true,
        );
      }
      agStore.clearPendingDecision(decision);
    } catch (cause) {
      const value = cause as { code?: string; message?: string; decisionResolved?: boolean };
      setError(value?.message ? `[${value.code ?? "?"}] ${value.message}` : String(cause));
      if (
        value.decisionResolved === true
        || (
          isWorkspaceOperation
          && [
            "WS_OPERATION_BASELINE_DRIFT",
            "WS_OPERATION_DIRTY_TARGET",
            "WS_OPERATION_TERMINAL",
          ].includes(value.code ?? "")
        )
      ) {
        appendOperationMessage(
          `${value.decisionResolved ? "决策已提交；后续处理未完成：" : ""}${value?.message ? `[${value.code ?? "?"}] ${value.message}` : String(cause)}`,
          true,
        );
        agStore.clearPendingDecision(decision);
      }
    } finally {
      agStore.clearCurrentAction(action);
      submitting.current = false;
      setBusy(false);
    }
  };

  return (
    <div className={`decisionbar decisionbar-${decision.kind}`}>
      <div className="decisionbar-head">
        <span className="decisionbar-title">等待你的决策</span>
        <span className="decisionbar-kind">{label}</span>
      </div>
      {summary && <div className="decisionbar-summary">{summary}</div>}
      {detail && <div className="decisionbar-note">{detail}</div>}
      {operations.length > 0 && (
        <ul className="decisionbar-operations">
          {operations.map((operation, index) => (
            <li key={String(operation.itemId ?? index)}>
              {OPERATION_LABEL[String(operation.kind ?? "")] ?? "资源操作"}
              {operation.sourcePath ? `：${String(operation.sourcePath)}` : ""}
              {operation.targetPath ? ` → ${String(operation.targetPath)}` : ""}
            </li>
          ))}
        </ul>
      )}
      {isWorkspaceOperation && consequences && (
        <ul className="decisionbar-operations decisionbar-consequences">
          <li>递归影响：{recursiveCount} 个资源</li>
          <li>受影响标签：{affectedSessions.length ? affectedSessions.join("、") : "无"}</li>
          <li>执行成功后失效的内容 Diff：{affectedDiffs.length ? affectedDiffs.join("、") : "无"}</li>
          <li>受影响引用：{affectedReferences.length ? affectedReferences.join("、") : "无"}</li>
          <li>恢复能力：{recoverable === false ? "不可恢复" : "可从时间轴生成恢复确认"}</li>
          {failurePolicy && <li>失败策略：失败即停止，并按逆序补偿已完成项</li>}
          <li>冲突边界：确认时会重新核验；目标变化、脏会话或目标已存在时不会覆盖执行</li>
        </ul>
      )}
      {error && <div className="decisionbar-error">{error}</div>}
      <div className="decisionbar-actions">
        <button className="decisionbar-primary" disabled={busy} onClick={() => void act("confirm")}>
          {busy ? "处理中…" : isWorkspaceOperation ? "确认并执行" : decision.kind === "permission" ? "允许本次" : "确认并继续"}
        </button>
        <button className="ghost" disabled={busy} onClick={() => void act("cancel")}>
          {decision.kind === "permission" ? "拒绝" : "取消"}
        </button>
      </div>
      <div className="decisionbar-note">
        {isWorkspaceOperation
          ? "任务已暂停；确认后系统会重新核验，并直接执行磁盘资源操作。"
          : "任务已暂停等待你的决策；完成决策后继续执行。"}
      </div>
    </div>
  );
}
