import { deStore, useDeStore } from "../../store/deStore";
import { acceptAllDiffs, rejectAllDiffs } from "../../services/deService";

/**
 * DiffAllActionsBar 批量条（参考稿 §6.5 · PI-FE-07 / PI-AG-UI-01）。
 * 呈现位置：代理编辑面板输入框上方（AgentPanel 内）；**无待裁决且非应用中时自隐藏**（需求3）。
 * 3 状态：有待裁决（红计数 + 全部接受/拒绝）→ 倒序应用中（金进度，文档靠后先应用避免行号偏移）
 * → 完成（绿 + 失效统计，可关闭）。
 */
export function DiffAllActionsBar() {
  const { diffs, batchState, batchExpiredCount } = useDeStore();
  const pendingCount = diffs.filter((d) => d.state === "pending").length;

  // 自隐藏：无待裁决、无进行中批次 → 不占位。
  if (batchState === "idle" && pendingCount === 0) return null;

  if (batchState === "applying") {
    return (
      <div className="diffbar diffbar-applying">
        <span className="label">倒序应用中…（文档靠后的修改先写入）</span>
      </div>
    );
  }
  if (batchState === "done") {
    return (
      <div className="diffbar diffbar-done">
        <span className="label">全部应用完成{batchExpiredCount > 0 ? ` · ${batchExpiredCount} 条失效` : ""}</span>
        <button className="ghost diffbar-x" aria-label="关闭" onClick={() => deStore.setBatch("idle")}>
          ×
        </button>
      </div>
    );
  }
  // idle：有待裁决
  return (
    <div className="diffbar diffbar-idle">
      <span className="diffbar-count">{pendingCount} 条待裁决</span>
      <div className="diffbar-actions">
        <button className="diffcard-accept" onClick={() => void acceptAllDiffs()}>全部接受</button>
        <button className="ghost" onClick={() => void rejectAllDiffs()}>全部拒绝</button>
      </div>
    </div>
  );
}
