import { useEffect, useState } from "react";
import { EXPIRED_REASON_TEXT, type DiffView } from "../../store/deStore";
import { acceptDiff, rejectDiff, generateSampleDiff } from "../../services/deService";
import { revealBlock } from "../../services/edLocate";

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

const PREVIEW_LIMIT = 140;
function previewText(text: string): string {
  const compact = text.replace(/\s+/g, " ").trim();
  if (!compact) return "（空）";
  return compact.length > PREVIEW_LIMIT ? `${compact.slice(0, PREVIEW_LIMIT)}…` : compact;
}

function statusText(diff: DiffView, locateFailed: boolean): { text: string; tone: "ok" | "warn" | "err" | "muted" } | null {
  if (locateFailed) return { text: "目标块已不可定位，建议重新生成", tone: "warn" };
  if (diff.state === "validating") return { text: "校验原文匹配中…", tone: "muted" };
  if (diff.state === "applying") return { text: "写入中…", tone: "warn" };
  if (diff.state === "accepted" && diff.draftApplied) {
    return { text: diff.draftSaved ? "已保存" : "已接受 · 待保存", tone: "ok" };
  }
  if (diff.state === "accepted") return { text: `已写入 · 时间轴 #${diff.timelineSeq ?? "-"}`, tone: "ok" };
  if (diff.state === "rejected") return { text: "已拒绝，文件未改动", tone: "muted" };
  if (diff.state === "conflict") return { text: `与 ${diff.conflictWith ?? "其他修改"} 范围重叠，接受已禁用`, tone: "warn" };
  if (diff.state === "expired" && diff.expiredReason) return { text: EXPIRED_REASON_TEXT[diff.expiredReason], tone: "warn" };
  if (diff.state === "error") return { text: diff.errorCode ?? "写入失败", tone: "err" };
  return null;
}

export function DiffCard({ diff }: { diff: DiffView }) {
  const { diffId, title, before, after, state } = diff;
  const busy = state === "validating" || state === "applying";
  const [expanded, setExpanded] = useState(false);
  const [locateFailed, setLocateFailed] = useState(false);
  const longPreview = before.length > PREVIEW_LIMIT || after.length > PREVIEW_LIMIT || /\n/.test(before + after);
  const hasStyleOps = Boolean(diff.styleOps && diff.styleOps.length > 0);
  const canExpand = longPreview || hasStyleOps;
  const location = diff.locationLabel ?? (diff.blockId ? "目标块" : "整篇");
  const locationText = diff.relPath ? `${diff.relPath} · ${location}` : location;
  const beforeView = previewText(before);
  const afterView = previewText(after);
  const detail = statusText(diff, locateFailed);
  const styleSummary = hasStyleOps ? `样式 ${diff.styleOps?.length ?? 0} 项` : "";
  const inlineStatus = [detail?.text, styleSummary].filter(Boolean).join(" · ");

  useEffect(() => {
    setLocateFailed(false);
  }, [diff.diffId, diff.blockId, state]);

  const onLocate = () => {
    setLocateFailed(revealBlock(diff.blockId) === "anchor_lost");
  };

  return (
    <div className={`diffcard diffcard-${state}${expanded ? " diffcard-expanded" : ""}`}>
      <div className="diffcard-row diffcard-row-title">
        <span className="diffcard-title" title={title}>{title}</span>
        <span className="diffcard-badge">{STATE_BADGE[state]}</span>
      </div>
      <div className="diffcard-row diffcard-row-location">
        <span className="diffcard-location" title={locationText}>{locationText}</span>
        {diff.blockId && (
          <button type="button" className="ghost diffcard-locate" onClick={onLocate}>
            定位
          </button>
        )}
      </div>

      <div className="diffcard-row diffcard-row-before">
        <span className="diffcard-mark">-</span>
        <span className="diffcard-line diffcard-before" title={before || "（空）"}>{beforeView}</span>
      </div>
      <div className="diffcard-row diffcard-row-after">
        <span className="diffcard-mark">+</span>
        <span className="diffcard-line diffcard-after" title={after || "（空）"}>{afterView}</span>
      </div>

      <div className="diffcard-row diffcard-row-actions">
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
          {canExpand && (
            <button type="button" className="ghost diffcard-expand" onClick={() => setExpanded((v) => !v)}>
              {expanded ? "收起" : "展开"}
            </button>
          )}
        </div>
        {state === "applying" && <span className="diffcard-progress"><span /></span>}
        {inlineStatus && <span className={`diffcard-status diffcard-status-${detail?.tone ?? "muted"}`}>{inlineStatus}</span>}
      </div>

      {expanded && (
        <div className="diffcard-detail">
          <div className="diffcard-detail-line diffcard-detail-before">
            <span className="diffcard-mark">-</span>
            <pre>{before || "（空）"}</pre>
          </div>
          <div className="diffcard-detail-line diffcard-detail-after">
            <span className="diffcard-mark">+</span>
            <pre>{after || "（空）"}</pre>
          </div>
          {/* 样式变更摘要（PI-ED-11 · 裁定 D-3 方案A：回合样式修改经本卡裁决呈现） */}
          {hasStyleOps && (
            <div className="diffcard-styles">
              {diff.styleOps?.map((s, i) => (
                <span key={i} className="diffcard-style-tag">{s}</span>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
