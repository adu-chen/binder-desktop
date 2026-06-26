/**
 * ed.locate_block（PI-AG-08H）：块级锚的**实时**定位。
 *
 * A-ED / types/mod.rs：真实 TipTap 块定位归 ED 前端，后端消费 LocateResult。本实现查询编辑器渲染 DOM
 * 里的 `[data-block-id]`（TipTap BlockIdExtension 渲染），真实反映**当前编辑器**是否仍存在该块——
 * 不需访问编辑器实例。供 AG 块编辑落卡前校验锚是否失效（用户在 AI 思考期间改动文档 → 目标块已不在）。
 */

export type LocateResult = "located" | "anchor_lost";

/** 在当前编辑器实时 DOM 中按 data-block-id 定位块。空 id 或未渲染该块 → anchor_lost。 */
export function locateBlock(blockId: string | undefined | null): LocateResult {
  if (!blockId) return "anchor_lost";
  if (typeof document === "undefined") return "anchor_lost";
  const sel =
    typeof CSS !== "undefined" && CSS.escape
      ? `[data-block-id="${CSS.escape(blockId)}"]`
      : `[data-block-id="${blockId.replace(/"/g, '\\"')}"]`;
  return document.querySelector(sel) ? "located" : "anchor_lost";
}
