import { useDeStore } from "../../store/deStore";
import { DiffCard } from "./DiffCard";

/**
 * 对话区 diff 卡列表（A-SYS-C-T-02 §3 · PI-FE-07）。
 * 渲染当前回合产出的 DiffCard（TERM-DE-003）；与编辑器 GreenReview 同源同步。
 */
export function DiffList() {
  const { diffs } = useDeStore();
  const live = diffs.filter((d) => d.state !== "rejected");
  if (live.length === 0) return null;
  return (
    <div className="difflist">
      <span className="label">AI 修改建议 · {live.length}</span>
      {live.map((d) => (
        <DiffCard key={d.diffId} diff={d} />
      ))}
    </div>
  );
}
