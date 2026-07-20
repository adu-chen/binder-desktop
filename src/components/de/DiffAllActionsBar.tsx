import { useDeStore } from "../../store/deStore";
import { acceptAllDiffs, rejectAllDiffs } from "../../services/deService";

/**
 * DiffAllActionsBar 批量条（参考稿 §6.5 · PI-FE-07 / PI-AG-UI-01）。
 * 呈现位置：代理编辑面板输入框上方（AgentPanel 内）；**无待裁决且非应用中时自隐藏**（需求3）。
 * 展示态：有待裁决（红计数 + 全部接受/拒绝）→ 倒序应用中（金进度，文档靠后先应用避免行号偏移）。
 * 批量完成后直接隐藏，不保留完成态占位。
 */
export function DiffAllActionsBar() {
  const { diffs, batchState } = useDeStore();
  const pendingCount = diffs.filter((d) => d.state === "pending").length;

  if (batchState === "applying") {
    return (
      <div className="diffbar diffbar-applying">
        <span className="label">倒序应用中…（文档靠后的修改先写入）</span>
      </div>
    );
  }
  // 自隐藏：无待裁决、无进行中批次 → 不占位；batch done 也不渲染终态栏。
  if (pendingCount === 0) return null;

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
