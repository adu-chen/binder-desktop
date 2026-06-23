import { EXPIRED_REASON_TEXT, type DiffView } from "../../store/deStore";
import { acceptDiff, rejectDiff, generateSampleDiff } from "../../services/deService";

/**
 * DiffCard（DiffCard / TERM-DE-003 · 参考稿 §6.1）· PI-FE-07。
 * 8 状态视觉互斥：pending / validating / applying / accepted / rejected / expired / conflict / error。
 * 与 GreenReview（编辑器装饰层）一一对应，同一 deStore 单源 → 双向同步（X-INTER-001）。
 */

const STATE_BADGE: Record<DiffView["state"], string> = {
  pending: "PENDING",
  validating: "校验中",
  applying: "写入中",
  accepted: "ACCEPTED",
  rejected: "已拒绝",
  expired: "已失效",
  conflict: "冲突",
  error: "错误",
};

export function DiffCard({ diff }: { diff: DiffView }) {
  const { diffId, title, before, after, state } = diff;
  const busy = state === "validating" || state === "applying";

  return (
    <div className={`diffcard diffcard-${state}`}>
      <div className="diffcard-head">
        <span className="diffcard-title">{title}</span>
        <span className="diffcard-badge">{STATE_BADGE[state]}</span>
      </div>

      <div className="diffcard-body">
        <div className="diffcard-line diffcard-before">- {before}</div>
        <div className="diffcard-line diffcard-after">+ {after}</div>
      </div>

      {state === "applying" && <div className="diffcard-progress"><span /></div>}
      {state === "validating" && <div className="diffcard-hint">校验原文匹配中…</div>}
      {state === "accepted" && (
        <div className="diffcard-hint diffcard-ok">✓ 已写入 · 时间轴 #{diff.timelineSeq}</div>
      )}
      {state === "rejected" && <div className="diffcard-hint">⌫ 已拒绝，文件未改动</div>}
      {state === "conflict" && (
        <div className="diffcard-hint diffcard-warn">⌖ 与 {diff.conflictWith} 范围重叠，接受已禁用</div>
      )}
      {state === "expired" && diff.expiredReason && (
        <div className="diffcard-hint diffcard-warn">! {EXPIRED_REASON_TEXT[diff.expiredReason]}</div>
      )}
      {state === "error" && <div className="diffcard-hint diffcard-err">× {diff.errorCode}</div>}

      <div className="diffcard-actions">
        {state === "pending" && (
          <>
            <button className="diffcard-accept" onClick={() => void acceptDiff(diffId)}>接受 ⏎</button>
            <button className="ghost" onClick={() => void rejectDiff(diffId)}>拒绝 ⌫</button>
          </>
        )}
        {state === "conflict" && (
          <>
            <button className="diffcard-accept" disabled>接受 ⏎</button>
            <button className="ghost" onClick={() => void rejectDiff(diffId)}>拒绝 ⌫</button>
          </>
        )}
        {state === "expired" && (
          <>
            <button onClick={() => void generateSampleDiff()}>重新生成</button>
            <button className="ghost" onClick={() => void rejectDiff(diffId)}>关闭</button>
          </>
        )}
        {state === "error" && (
          <>
            <button onClick={() => void acceptDiff(diffId)}>重试 ↻</button>
            <button className="ghost" onClick={() => void rejectDiff(diffId)}>拒绝</button>
          </>
        )}
        {busy && <button disabled>处理中…</button>}
      </div>
    </div>
  );
}
