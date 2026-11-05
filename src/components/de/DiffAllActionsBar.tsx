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
  // 只读历史卡不是可裁决项：计数与动作必须一致，否则会出现“显示 1 条待裁决、点全部接受无反应”。
  const pending = diffs.filter((d) => d.state === "pending" && !d.readonly);
  const pendingCount = pending.length;
  // 同一 TaskRun 的分块卡合并显示，避免长任务被误认为多个互不相关的任务。
  const taskCount = new Set(
    pending.map((diff) => diff.taskRunId).filter((taskRunId): taskRunId is string => Boolean(taskRunId)),
  ).size;

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
      <span className="diffbar-count">
        {pendingCount} 条待裁决{taskCount > 1 ? ` · ${taskCount} 个任务` : ""}
      </span>
      <div className="diffbar-actions">
        <button className="diffcard-accept" onClick={() => void acceptAllDiffs()}>全部接受</button>
        <button className="ghost" onClick={() => void rejectAllDiffs()}>全部拒绝</button>
      </div>
    </div>
  );
}
