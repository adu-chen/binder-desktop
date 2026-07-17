import { useDeStore, type DiffView } from "../../store/deStore";
import { DiffCard } from "./DiffCard";

/**
 * 对话区 diff 卡列表（A-SYS-C-T-02 §3 · PI-FE-07）。
 * 渲染当前回合产出的 DiffCard（TERM-DE-003）；与编辑器 GreenReview 同源同步。
 */
export function DiffList({ diffs: scopedDiffs, label = "AI 修改建议" }: { diffs?: DiffView[]; label?: string }) {
  const { diffs } = useDeStore();
  const visible = scopedDiffs ?? diffs;
  if (visible.length === 0) return null;
  return (
    <div className="difflist">
      <span className="label">{label} · {visible.length}</span>
      {visible.map((d) => (
        <DiffCard key={d.diffId} diff={d} />
      ))}
    </div>
  );
}
