import { useDeStore, deStore } from "../../store/deStore";

/**
 * DiffAllActionsBar 批量条（参考稿 §6.5 · PI-FE-07）· DiffStrip 区。
 * 3 状态：有待裁决（红计数 + 全部接受/拒绝）→ 倒序应用中（金进度，文档靠后先应用避免行号偏移）
 * → 完成（绿 + 失效统计）。
 */
export function DiffAllActionsBar() {
  const { diffs, batchState, batchExpiredCount } = useDeStore();
  const pendingCount = diffs.filter((d) => d.state === "pending").length;

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
      </div>
    );
  }
  // idle：有待裁决
  return (
    <div className="diffbar diffbar-idle">
      <span className="diffbar-count">{pendingCount} 条待裁决</span>
      <div className="diffbar-actions">
        <button className="diffcard-accept" onClick={() => deStore.acceptAll()}>全部接受</button>
        <button className="ghost" onClick={() => deStore.rejectAll()}>全部拒绝</button>
      </div>
    </div>
  );
}
