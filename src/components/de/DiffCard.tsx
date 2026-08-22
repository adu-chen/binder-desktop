import { useEffect, useState } from "react";
import { EXPIRED_REASON_TEXT, type DiffView } from "../../store/deStore";
import { acceptDiff, rejectDiff, generateSampleDiff } from "../../services/deService";
import { revealBlock } from "../../services/edLocate";
import { ContextMenu, type ContextMenuDef } from "../shell/ContextMenu";

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
  if (diff.state === "pending" && diff.draftPreviewState === "queued") {
    return { text: "待确认建议已生成，正在等待编辑器预览；尚未写入文件", tone: "muted" };
  }
  if (diff.state === "pending" && diff.draftPreviewState === "applied") {
    return { text: "编辑器预览已显示；接受将确认，保存后才写入文件", tone: "warn" };
  }
  if (diff.state === "validating") return { text: "校验原文匹配中…", tone: "muted" };
  if (diff.state === "applying") return { text: "写入中…", tone: "warn" };
  if (diff.effectStatus === "discarded_on_abnormal_exit") {
    return { text: "已接受，但未保存内容已丢弃", tone: "warn" };
  }
  if (diff.state === "expired" && diff.terminalReason === "abnormal_exit") {
    return { text: "因上次异常退出已失效", tone: "warn" };
  }
  if (diff.state === "accepted" && diff.effectStatus === "saved") {
    return { text: "已接受并保存", tone: "ok" };
  }
  if (diff.state === "accepted" && diff.effectStatus === "awaiting_save") {
    return { text: "已接受 · 待保存", tone: "warn" };
  }
  if (diff.state === "accepted" && diff.draftApplied) {
    return { text: diff.draftSaved ? "已保存" : "已接受 · 待保存", tone: "ok" };
  }
  if (diff.state === "accepted") return { text: `已写入 · 时间轴 #${diff.timelineSeq ?? "-"}`, tone: "ok" };
  if (diff.state === "rejected") return { text: "已拒绝，文件未改动", tone: "muted" };
  if (diff.state === "conflict") {
    return diff.errorCode
      ? { text: `目标块已在提案后变化（${diff.errorCode}），已停止回滚`, tone: "warn" }
      : { text: `与 ${diff.conflictWith ?? "其他修改"} 范围重叠，接受已禁用`, tone: "warn" };
  }
  if (diff.state === "expired" && diff.expiredReason) return { text: EXPIRED_REASON_TEXT[diff.expiredReason], tone: "warn" };
  if (diff.state === "error" && diff.draftPreviewState === "failed") {
    return { text: `编辑器预览未应用，文档未因该建议改变（${diff.errorCode ?? "未知错误"}）`, tone: "err" };
  }
  if (diff.state === "error") return { text: diff.errorCode ?? "写入失败", tone: "err" };
  return null;
}

function tableCellText(diff: DiffView, row: number, column: number, phase: "before" | "after"): string | null {
  const table = phase === "before" ? diff.tableBefore : diff.tableAfter;
  const cell = table?.cells.find((candidate) => (
    row >= candidate.row
    && row < candidate.row + candidate.rowspan
    && column >= candidate.column
    && column < candidate.column + candidate.colspan
  ));
  return cell?.text ?? null;
}

function tableCellShading(diff: DiffView, row: number, column: number, phase: "before" | "after"): string | null {
  const table = phase === "before" ? diff.tableBefore : diff.tableAfter;
  const cell = table?.cells.find((candidate) => (
    row >= candidate.row
    && row < candidate.row + candidate.rowspan
    && column >= candidate.column
    && column < candidate.column + candidate.colspan
  ));
  return cell?.backgroundColor ?? null;
}

function tableOperationDetails(diff: DiffView): string[] {
  if (!diff.tableOpsPayload) return [];
  return diff.tableOpsPayload.operations.map((operation) => {
    switch (operation.op) {
      case "set_cell_text": {
        const label = `R${operation.at.row + 1}C${operation.at.column + 1}`;
        const before = tableCellText(diff, operation.at.row, operation.at.column, "before");
        const after = tableCellText(diff, operation.at.row, operation.at.column, "after");
        return `${label}：${before === null ? "（应用前待解析）" : previewText(before)} → ${after === null ? previewText(operation.text) : previewText(after)}`;
      }
      case "merge_cells":
        return `合并范围：R${operation.from.row + 1}C${operation.from.column + 1} 至 R${operation.to.row + 1}C${operation.to.column + 1}`;
      case "split_cell":
        return `拆分单元格：R${operation.at.row + 1}C${operation.at.column + 1}`;
      case "set_header_row":
        return `首行表头：${diff.tableBefore?.headerRow ? "是" : "否"} → ${operation.enabled ? "是" : "否"}`;
      case "set_cell_background":
        return "";
      case "move_table":
        return `文档流位置：第 ${(diff.tableBefore?.ordinal ?? 0) + 1} 个表格 → 第 ${(diff.tableAfter?.ordinal ?? diff.tableBefore?.ordinal ?? 0) + 1} 个表格`;
      default:
        return "";
    }
  }).filter(Boolean);
}

function imageOperationDetails(diff: DiffView): Array<{ title: string; details: string[] }> {
  if (!diff.imageOpsPayload) return [];
  return diff.imageOpsPayload.operations.map((operation) => {
    if (operation.op === "insert_image") {
      const beforeIds = new Set(diff.imageBefore?.map((image) => image.imageId) ?? []);
      const inserted = diff.imageAfter?.find((image) => (
        !beforeIds.has(image.imageId)
        &&
        image.projectSourcePath === operation.sourcePath
        && image.projectSourceHash === operation.sourceHash
        && image.blockId !== operation.anchorBlockId
      ));
      return {
        title: `插入图片 · ${operation.sourcePath}`,
        details: [
          `位置：Block ${operation.anchorBlockId} ${operation.position === "before" ? "之前" : "之后"}`,
          `替代文本：${operation.alt === undefined || operation.alt === "" ? "（无）" : operation.alt}`,
          `宽度：${operation.width === undefined ? "保留图片默认宽度" : `${operation.width}px`}`,
          `资源：${operation.mediaRef}`,
          `来源哈希：${operation.sourceHash}`,
          `结果：${inserted ? `已生成图片实例 ${inserted.imageId}` : "等待应用到当前草稿"}`,
        ],
      };
    }
    const previous = diff.imageBefore?.find((image) => (
      image.imageId === operation.imageId && image.fingerprint === operation.fingerprint
    ));
    const current = diff.imageAfter?.find((image) => image.imageId === operation.imageId);
    if (operation.op === "resize_image") {
      return {
        title: `缩放图片 · ${operation.imageId}`,
        details: [
          `宽度：${previous?.width ?? "默认宽度"} → ${current?.width ?? `${operation.width}px`}`,
          `高度：${previous?.height ?? "按比例"} → ${current?.height ?? "按比例"}`,
          `位置：${previous?.blockId ? `Block ${previous.blockId}` : "当前图片节点"}`,
        ],
      };
    }
    if (operation.op === "move_image") {
      return {
        title: `移动图片 · ${operation.imageId}`,
        details: [
          `方向：${operation.direction === "up" ? "向上" : "向下"}`,
          `文档流位置：第 ${(previous?.ordinal ?? 0) + 1} 张 → 第 ${(current?.ordinal ?? previous?.ordinal ?? 0) + 1} 张`,
          `Block：${previous?.blockId ?? "（无）"} → ${current?.blockId ?? previous?.blockId ?? "（无）"}`,
        ],
      };
    }
    if (operation.op === "set_image_layout") {
      return {
        title: `图片段落对齐 · ${operation.imageId}`,
        details: [
          `对齐：${previous?.alignment ?? "left"} → ${current?.alignment ?? operation.alignment}`,
          `位置：${previous?.blockId ? `Block ${previous.blockId}` : "独立图片段落"}`,
        ],
      };
    }
    return {
      title: `删除文档图片 · ${operation.imageId}`,
      details: [
        `原位置：${previous?.blockId ? `Block ${previous.blockId}` : `第 ${(previous?.ordinal ?? 0) + 1} 张图片`}`,
        `原替代文本：${previous?.alt ?? "（无）"}`,
        `原尺寸：${previous?.width ?? "默认宽度"}${previous?.height ? ` × ${previous.height}` : ""}`,
        `原资源：${previous?.mediaRef ?? "（等待当前图片快照）"}`,
        `原项目来源：${previous?.projectSourcePath ?? "（非项目来源）"}`,
        `原来源哈希：${previous?.projectSourceHash ?? "（无）"}`,
        "项目源文件：保留，不执行删除",
        `结果：${diff.imageAfter?.some((image) => image.imageId === operation.imageId) ? "目标仍存在（需重新校验）" : "文档图片节点已移除"}`,
      ],
    };
  });
}

function nodeText(value: unknown): string {
  if (!value || typeof value !== "object") return "";
  if (Array.isArray(value)) return value.map(nodeText).join("");
  const node = value as Record<string, unknown>;
  if (node.type === "hardBreak") return "\n";
  if (typeof node.text === "string") return node.text;
  return nodeText(node.content);
}

function nodeBlockId(value: unknown): string | null {
  if (!value || typeof value !== "object") return null;
  const attrs = (value as Record<string, unknown>).attrs;
  if (!attrs || typeof attrs !== "object") return null;
  const blockId = (attrs as Record<string, unknown>).blockId;
  return typeof blockId === "string" && blockId ? blockId : null;
}

function blockOperationDetails(diff: DiffView): Array<{
  key: string;
  blockId: string;
  nodeType: string;
  before: string;
  after: string;
  op: string;
  placement?: string;
}> {
  return diff.blockOpsPayload?.operations.map((operation, index) => {
    const insertion = operation.op === "insert_before" || operation.op === "insert_after" || operation.op === "append";
    const insertedId = insertion ? nodeBlockId(operation.afterNode) : null;
    const placement = operation.op === "insert_before"
      ? `插入到 ${operation.blockId} 之前`
      : operation.op === "insert_after"
        ? `插入到 ${operation.blockId} 之后`
        : operation.op === "append"
          ? "追加到文档末尾"
          : undefined;
    return {
      key: `${index}:${operation.op}:${operation.blockId}:${insertedId ?? ""}`,
      blockId: insertedId ?? operation.blockId,
      nodeType: operation.expectedNodeType,
      before: nodeText(operation.beforeNode),
      after: operation.op === "delete_node" ? "【已删除】" : nodeText(operation.afterNode),
      op: operation.op,
      placement,
    };
  }) ?? [];
}

/**
 * @GOV
 * codes:    BR-DE-DATA-002-RB-DE-DE-DIFF-003
 * type:     RB
 * chain:    DE-DIFF
 * rules:    BR-DE-DATA-002
 * term_ref: TERM-DE-003, TERM-ED-008
 * boundary: in=ImageOperationDiffPayloadAndBeforeAfterSnapshots | out=StructuredImageOperationReviewCard(insertDeleteResizeMoveAlignmentWithBeforeAfterValues)
 */
export function DiffCard({ diff }: { diff: DiffView }) {
  const { diffId, title, before, after, state } = diff;
  const busy = state === "validating" || state === "applying";
  const readonly = Boolean(diff.readonly);
  const [expanded, setExpanded] = useState(false);
  const [locateFailed, setLocateFailed] = useState(false);
  // DiffCard 右键（CTX-MENU-01）：仅 pending 且可裁决态提供接受/拒绝快捷入口。
  const [ctxMenu, setCtxMenu] = useState<{ x: number; y: number } | null>(null);
  const longPreview = before.length > PREVIEW_LIMIT || after.length > PREVIEW_LIMIT || /\n/.test(before + after);
  const hasStyleOps = Boolean(diff.styleOps && diff.styleOps.length > 0);
  const hasBlockOps = Boolean(diff.blockOpsPayload);
  const hasTableOps = Boolean(diff.tableOps && diff.tableOps.length > 0);
  const hasImageOps = Boolean(diff.imageOpsPayload || (diff.imageOps && diff.imageOps.length > 0));
  const tableDetails = hasTableOps ? tableOperationDetails(diff) : [];
  const imageDetails = hasImageOps ? imageOperationDetails(diff) : [];
  const blockDetails = hasBlockOps ? blockOperationDetails(diff) : [];
  const tableShadingDetails = diff.tableOpsPayload?.operations.flatMap((operation) => {
    if (operation.op !== "set_cell_background") return [];
    return [{
      label: `R${operation.at.row + 1}C${operation.at.column + 1}`,
      before: tableCellShading(diff, operation.at.row, operation.at.column, "before"),
      after: tableCellShading(diff, operation.at.row, operation.at.column, "after") ?? operation.color,
    }];
  }) ?? [];
  const canExpand = longPreview || hasStyleOps || hasBlockOps || hasTableOps || hasImageOps;
  const location = diff.locationLabel ?? (diff.blockId ? "目标块" : "整篇");
  const locationText = diff.relPath ? `${diff.relPath} · ${location}` : location;
  const beforeView = previewText(before);
  const afterView = previewText(after);
  const detail = statusText(diff, locateFailed);
  const styleSummary = hasStyleOps ? `样式 ${diff.styleOps?.length ?? 0} 项` : "";
  const blockSummary = hasBlockOps
    ? `结构化操作 ${diff.blockOpsPayload?.operations.length ?? 0} 项`
    : "";
  const tableSummary = hasTableOps ? `表格操作 ${diff.tableOps?.length ?? 0} 项` : "";
  const imageSummary = hasImageOps
    ? `图片操作 ${diff.imageOpsPayload?.operations.length ?? diff.imageOps?.length ?? 0} 项`
    : "";
  const inlineStatus = [detail?.text, blockSummary, styleSummary, tableSummary, imageSummary].filter(Boolean).join(" · ");
  const canRetryError = state === "error" && Boolean(diff.draftApplied);
  const previewQueued = state === "pending" && diff.draftPreviewState === "queued";

  useEffect(() => {
    setLocateFailed(false);
  }, [diff.diffId, diff.blockId, state]);

  const onLocate = () => {
    setLocateFailed(revealBlock(diff.blockId) === "anchor_lost");
  };

  // DiffCard 右键菜单（仅 pending 且非 readonly：接受/拒绝；动作复用 de.accept/reject_diff）。
  const onContextMenu = (event: React.MouseEvent) => {
    event.preventDefault();
    if (state !== "pending" || readonly || previewQueued) return;
    setCtxMenu({ x: event.clientX, y: event.clientY });
  };
  const ctxMenuItems: ContextMenuDef[] | null = ctxMenu && state === "pending" && !readonly && !previewQueued
    ? [
        { type: "item", id: "accept", label: "接受", onSelect: () => void acceptDiff(diffId) },
        { type: "item", id: "reject", label: "拒绝", onSelect: () => void rejectDiff(diffId) },
      ]
    : null;

  return (
    <div
      className={`diffcard diffcard-${state}${expanded ? " diffcard-expanded" : ""}`}
      onContextMenu={onContextMenu}
    >
      <div className="diffcard-row diffcard-row-title">
        <span className="diffcard-title" title={title}>{title}</span>
        {/* AG-TASK-01 F1b：批次徽标（第 x/y 批）。 */}
        {diff.batch && (
          <span className="diffcard-badge diffcard-batch" title={`批次 ${diff.batch.id}`}>
            第 {diff.batch.index}/{diff.batch.total} 批
          </span>
        )}
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
          {state === "pending" && !readonly && (
            <>
              <button
                className="diffcard-accept"
                disabled={previewQueued}
                title={previewQueued ? "等待编辑器完成预览后才能接受" : undefined}
                onClick={() => void acceptDiff(diffId)}
              >
                {previewQueued ? "预览准备中…" : "接受 ⏎"}
              </button>
              <button className="ghost" onClick={() => void rejectDiff(diffId)}>拒绝 ⌫</button>
            </>
          )}
          {state === "conflict" && !readonly && (
            <>
              <button className="diffcard-accept" disabled>接受 ⏎</button>
              <button className="ghost" onClick={() => void rejectDiff(diffId)}>拒绝 ⌫</button>
            </>
          )}
          {state === "expired" && !readonly && (
            <>
              <button onClick={() => void generateSampleDiff()}>重新生成</button>
              <button className="ghost" onClick={() => void rejectDiff(diffId)}>关闭</button>
            </>
          )}
          {state === "error" && !readonly && (
            <>
              {canRetryError
                ? <button onClick={() => void acceptDiff(diffId)}>重试 ↻</button>
                : <span className="diffcard-status diffcard-status-error">需重新生成修改建议</span>}
              <button className="ghost" onClick={() => void rejectDiff(diffId)}>
                {canRetryError ? "拒绝" : "关闭"}
              </button>
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
          {hasBlockOps && (
            <div className="diffcard-block-ops">
              {blockDetails.map((operation) => (
                <div key={operation.key} className="diffcard-block-operation">
                  <strong>
                    {operation.blockId} · {operation.nodeType}
                    {operation.op === "delete_node" ? "（删除）" : ""}
                    {operation.placement ? `（${operation.placement}）` : ""}
                  </strong>
                  <div className="diffcard-detail-line diffcard-detail-before">
                    <span className="diffcard-mark">-</span>
                    <pre>{operation.before || "（空）"}</pre>
                  </div>
                  <div className="diffcard-detail-line diffcard-detail-after">
                    <span className="diffcard-mark">+</span>
                    <pre className={operation.op === "delete_node" ? "diffcard-deleted" : undefined}>{operation.after || "（空）"}</pre>
                  </div>
                </div>
              ))}
            </div>
          )}
          {hasTableOps && (
            <div className="diffcard-table-ops">
              <div className="diffcard-table-shape">
                {diff.tableBefore ? `${diff.tableBefore.rows}×${diff.tableBefore.columns}` : "表格"}
                {diff.tableAfter ? ` → ${diff.tableAfter.rows}×${diff.tableAfter.columns}` : ""}
              </div>
              {diff.tableOps?.map((operation, index) => (
                <span key={index} className="diffcard-style-tag">{operation}</span>
              ))}
              {tableDetails.map((detail, index) => (
                <div key={index} className="diffcard-table-detail">{detail}</div>
              ))}
              {tableShadingDetails.map((detail, index) => (
                <div key={index} className="diffcard-table-shading-detail">
                  <span className="diffcard-table-shading-label">{detail.label}</span>
                  <span
                    className={`diffcard-table-shading-swatch${detail.before ? "" : " is-empty"}`}
                    style={detail.before ? { backgroundColor: detail.before } : undefined}
                    aria-label={`原底纹 ${detail.before ?? "无"}`}
                  />
                  <code>{detail.before ?? "无底纹"}</code>
                  <span aria-hidden="true">→</span>
                  <span
                    className={`diffcard-table-shading-swatch${detail.after ? "" : " is-empty"}`}
                    style={detail.after ? { backgroundColor: detail.after } : undefined}
                    aria-label={`新底纹 ${detail.after ?? "无"}`}
                  />
                  <code>{detail.after ?? "无底纹"}</code>
                </div>
              ))}
            </div>
          )}
          {hasImageOps && (
            <div className="diffcard-image-ops">
              <div className="diffcard-image-counts">
                图片实例：{diff.imageBefore?.length ?? "待解析"} → {diff.imageAfter?.length ?? "待应用"}
              </div>
              {imageDetails.map((operation, index) => (
                <div key={index} className="diffcard-image-operation">
                  <strong>{operation.title}</strong>
                  {operation.details.map((detail, detailIndex) => (
                    <div key={detailIndex} className="diffcard-image-detail">{detail}</div>
                  ))}
                </div>
              ))}
              {!imageDetails.length && diff.imageOps?.map((operation, index) => (
                <span key={index} className="diffcard-style-tag">{operation}</span>
              ))}
            </div>
          )}
        </div>
      )}
      {ctxMenu && ctxMenuItems && (
        <ContextMenu items={ctxMenuItems} x={ctxMenu.x} y={ctxMenu.y} onClose={() => setCtxMenu(null)} />
      )}
    </div>
  );
}
