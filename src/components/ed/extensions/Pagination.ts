import { Extension } from "@tiptap/core";
import { Plugin, PluginKey, type Transaction } from "@tiptap/pm/state";
import { Decoration, DecorationSet } from "@tiptap/pm/view";
import type { EditorView } from "@tiptap/pm/view";
import type { Node as PMNode } from "@tiptap/pm/model";
import {
  getPageGeometry,
  pageContentHeight,
  type PageGeometry,
} from "../../../store/edViewPrefsStore";
import {
  computeClampedViewportScrollTop,
  computeViewportAnchorScrollDelta,
  shouldApplyViewportAnchorCorrection,
} from "./paginationViewportAnchor";
import {
  createPaginationFlowPlanner,
  type PageBreak,
  type PageBreakPlan,
  type RenderedFlowBlock,
  type RenderedFlowFragment,
} from "./paginationFlow";

export type { PageBreak, PageBreakKind, PageBreakPlan } from "./paginationFlow";

/**
 * 分页呈现层扩展（ED-EDIT · BR-ED-DATA-003 · PI-ED-04/PI-ED-06）。
 * A4/Word 风格分页：按测量的视觉流片段在 Block 边界或普通段落视觉行间插入分页缝（Decoration.widget），
 * 内容连续流经缝隙自然下推——纯呈现层，**不改文档模型、不落盘、不产生脏态**（BR-ED-DATA-003）；
 * 经 ED 声明的 ProseMirror Decoration 扩展点叠加，不替换 DOM、不阻挡编辑（X-FORBID-005）。
 *
 * PI-ED-06 页高修复：每个分页缝携带 **fill（页尾留白补足）**——非末页视觉高度恒等所选纸张
 * 内容区预算（Word 语义：内容未满页也补足空白）；末页经 end-filler 撑满整页比例。
 * 纸张几何动态（A4/A3/A5/Letter × 横竖排，edViewPrefsStore 视图偏好）经 setPageGeometry 切换。
 *
 * 切页判定由纯逻辑 `paginationFlow` 完成，与 DOM 测量解耦、可单测；实际视觉行测量和分页缝渲染属
 * 视觉层，jsdom 无布局引擎不可完整验收。仅在 `enabled` 时分页（docx 编辑默认开、md/纯文本关）；
 * 第一阶段只拆分顶层普通段落，复杂容器、标题、引用、代码、表格与媒体仍按原子 Block 分页。
 */

/** 默认几何：A4 竖排（向后兼容导出；动态几何经 setPageGeometry）。 */
export const PAGE_GEOMETRY: PageGeometry = getPageGeometry("A4", "portrait");

/** 默认每页可用内容高度（A4 竖排预算；动态值从当前几何算）。 */
export const PAGE_CONTENT_HEIGHT = pageContentHeight(PAGE_GEOMETRY);

/** 连续内容输入静默期：期间保持旧 break plan，避免慢速输入逐字触发真实分页。 */
export const PAGINATION_INPUT_SETTLE_MS = 450;

/** 分页测量相等判定使用的亚像素容差；布局值本身不再逐页取整。 */
export const PAGINATION_PLAN_EPSILON = 0.5;

/**
 * 分页层主动修正滚动位置后的内部信号。Editor 的光标激活周期据此更新滚动基线，
 * 不把分页结构补偿误判成用户主动滚动。
 */
export const PAGINATION_VIEWPORT_CORRECTION_EVENT = "binder:pagination-viewport-correction";

/**
 * 展平文档块：将 orderedList/bulletList 容器块展开为其 listItem 子节点，
 * 使分页能在列表项之间切分，避免整个列表被视为不可分割块导致页面大空当。
 * 审计修复（2026-07-25）：A+D 方案分页层独立补丁。
 */
function flattenBlocks(doc: PMNode): Array<{ node: PMNode; pos: number }> {
  const blocks: Array<{ node: PMNode; pos: number }> = [];
  let p = 0;
  doc.forEach((node) => {
    if (node.type.name === "orderedList" || node.type.name === "bulletList") {
      let cp = p + 1; // 跳过容器块开口标签
      node.forEach((child) => {
        blocks.push({ node: child, pos: cp });
        cp += child.nodeSize;
      });
    } else {
      blocks.push({ node, pos: p });
    }
    p += node.nodeSize;
  });
  return blocks;
}

/**
 * 原子 Block 分页兼容入口（供既有回归测试共用）。生产测量使用 `paginationFlow` 的 position 断点；
 * 此处用数组下标作为合成 position，规则仍与旧 Block 分页完全等价。
 * **不变量：非溢出页 used + fill = 预算**（每页视觉高度恒等所选纸张内容区）。空序列 → 无缝 + 满页 fill。
 */
export function computePageBreaks(blockHeights: number[], budget: number): PageBreakPlan {
  const planner = createPaginationFlowPlanner(budget);
  blockHeights.forEach((height, index) => {
    planner.append({
      blockPos: index,
      kind: "atomic",
      fragments: [{
        pos: index,
        height,
        kind: "block",
        blockPos: index,
        lineIndex: 0,
        lineCount: 1,
      }],
    });
  });
  return planner.finish();
}

/** 两个方案的分页缝结构是否相等。fill 是稳定 widget 上的 CSS 值，不属于 Decoration 身份。 */
export function sameBreaks(a: PageBreakPlan, b: PageBreakPlan): boolean {
  if (a.breaks.length !== b.breaks.length) return false;
  for (let i = 0; i < a.breaks.length; i++) {
    if (a.breaks[i].pos !== b.breaks[i].pos || a.breaks[i].kind !== b.breaks[i].kind) return false;
  }
  return true;
}

/** 两个完整切页方案近似相等（包含末页补白）。 */
export function samePlan(a: PageBreakPlan, b: PageBreakPlan): boolean {
  if (!sameBreaks(a, b)) return false;
  for (let i = 0; i < a.breaks.length; i++) {
    if (Math.abs(a.breaks[i].fill - b.breaks[i].fill) > PAGINATION_PLAN_EPSILON) return false;
  }
  return Math.abs(a.lastFill - b.lastFill) <= PAGINATION_PLAN_EPSILON;
}

function isValidParagraphLineBreak(doc: PMNode, pos: number): boolean {
  if (pos <= 0 || pos >= doc.content.size) return false;
  try {
    const resolved = doc.resolve(pos);
    return resolved.depth === 1
      && resolved.parent.type.name === "paragraph"
      && resolved.parentOffset > 0
      && resolved.parentOffset < resolved.parent.content.size;
  } catch {
    return false;
  }
}

/**
 * 输入冻结期间同步映射 position 型分页计划。widget 使用 side:-1，因此断点也以 assoc=-1 映射；
 * 被删除或不再位于顶层普通段落内部的行间断点安全丢弃，不保留猜测位置。
 */
export function mapPageBreakPlan(plan: PageBreakPlan, transaction: Transaction): PageBreakPlan {
  const breaks: PageBreak[] = [];
  const seen = new Set<string>();
  for (const pageBreak of plan.breaks) {
    const mapped = transaction.mapping.mapResult(pageBreak.pos, -1);
    if (mapped.deleted) continue;
    if (pageBreak.kind === "paragraph-line"
      && !isValidParagraphLineBreak(transaction.doc, mapped.pos)) continue;
    const key = `${mapped.pos}:${pageBreak.kind}`;
    if (seen.has(key)) continue;
    seen.add(key);
    breaks.push({ ...pageBreak, pos: mapped.pos });
  }
  return {
    breaks: breaks.map((pageBreak, index) => ({ ...pageBreak, id: `pg-${index + 2}` })),
    lastFill: plan.lastFill,
  };
}

/** 普通分页缝仅由分页结构驱动；末页补白改由 CSS 变量独立更新。 */
export function shouldRebuildPageGapDecorations(previous: PageBreakPlan, next: PageBreakPlan): boolean {
  return !sameBreaks(previous, next);
}

/** 分页重测量只能由内容/几何等外部输入触发。
 * `plan` 提交本身会改变分页缝与末页补白的高度，若也作为测量输入便会形成
 * “重排 → 编辑器自身变高 → ResizeObserver → 再重排”的正反馈。 */
export function shouldMeasurePagination(input: {
  wasEnabled: boolean;
  enabled: boolean;
  docChanged: boolean;
  geometryChanged: boolean;
  layoutReset?: boolean;
}): boolean {
  if (!input.enabled) return false;
  return (!input.wasEnabled && input.enabled) || input.docChanged || input.geometryChanged || input.layoutReset === true;
}

/** 内容编辑等待静默窗口；首次启用、载入重置和纸张几何变化立即结算。 */
export function paginationMeasureDelay(input: {
  wasEnabled: boolean;
  enabled: boolean;
  docChanged: boolean;
  geometryChanged: boolean;
  layoutReset?: boolean;
}): number | null {
  if (!shouldMeasurePagination(input)) return null;
  const immediate = !input.wasEnabled || input.geometryChanged || input.layoutReset === true;
  return immediate ? 0 : PAGINATION_INPUT_SETTLE_MS;
}

/** 纸张几何是视图状态；相同值不得重置分页方案。 */
export function samePageGeometry(a: PageGeometry, b: PageGeometry): boolean {
  return a.width === b.width
    && a.height === b.height
    && a.marginX === b.marginX
    && a.marginY === b.marginY
    && a.gap === b.gap;
}

interface PaginationState {
  enabled: boolean;
  plan: PageBreakPlan;
  decorations: DecorationSet;
  /** 同一帧内撤下段内 widget 读取真实可见自然行；Block 页缝继续保留。 */
  measuringNaturalLines: boolean;
  /** 外部输入使布局失效时递增；plan 提交本身不能递增。 */
  measureRevision: number;
  /** 本轮失效应等待的静默期；内容输入为 450ms，首次/几何等布局输入为 0。 */
  measureDelay: number;
}

interface PaginationMeta {
  enabled?: boolean;
  plan?: PageBreakPlan;
  /** 几何变更（纸张/方向切换）：更新 CSS 变量并强制下轮重测。 */
  geometryChanged?: boolean;
  /** 新文档已装载：即使几何相同，也必须丢弃上一文档的分页方案并重新测量。 */
  layoutReset?: boolean;
  /** 本轮分页测量已收敛；供 Editor 按当前 loadSeq 揭示 BDocx。 */
  settled?: boolean;
  /** 仅供同步测量事务：true 时 DecorationSet 暂时排除 paragraph-line 页缝。 */
  measuringNaturalLines?: boolean;
}

const EMPTY_PLAN: PageBreakPlan = { breaks: [], lastFill: 0 };

export const paginationKey = new PluginKey<PaginationState>("pagination");

/** Editor 生命周期只消费 settled 信号，不读取分页插件内部状态。 */
export function isPaginationSettledTransaction(transaction: Transaction): boolean {
  return (transaction.getMeta(paginationKey) as PaginationMeta | undefined)?.settled === true;
}

/** 分页缝 widget：Block 断点用 div，普通段落行间断点用 span，均为纯呈现且不可交互。 */
function renderGap(pageNumber: number, pageBreak: PageBreak): HTMLElement {
  const gap = document.createElement(pageBreak.kind === "paragraph-line" ? "span" : "div");
  gap.className = `rm-page-gap rm-page-gap-${pageBreak.kind === "paragraph-line" ? "inline" : "block"}`;
  // position 会随事务映射而变化，DOM 只能用同一方案内稳定的 id 识别。
  gap.dataset.rmPageBreakId = pageBreak.id;
  gap.dataset.rmPageBreakKind = pageBreak.kind;
  gap.contentEditable = "false";
  gap.setAttribute("aria-hidden", "true");
  const childTag = pageBreak.kind === "paragraph-line" ? "span" : "div";
  const filler = document.createElement(childTag);
  filler.className = "rm-page-gap-fill";
  gap.appendChild(filler);
  const gutter = document.createElement(childTag);
  gutter.className = "rm-page-gap-gutter";
  const num = document.createElement("span");
  num.className = "rm-page-gap-num";
  num.textContent = String(pageNumber);
  gutter.appendChild(num);
  gap.appendChild(gutter);
  return gap;
}

/**
 * 真实行尾测量只撤下会改变段落换行拓扑的 paragraph-line widget。
 * Block 页缝定义纸张边界，任何测量阶段都必须保留。
 */
/**
 * @GOV
 * codes:    BR-ED-DATA-003-GUARD-ED-ED-EDIT-019
 * type:     GUARD
 * chain:    ED-EDIT
 * rules:    BR-ED-DATA-003
 * boundary: in=PageBreakKindAndNaturalLineMeasurementState | out=PhysicalPageGapRenderDecision(blockGapAlwaysRetained)
 */
export function shouldRenderPageBreak(
  pageBreak: Pick<PageBreak, "kind">,
  measuringNaturalLines: boolean,
): boolean {
  return !measuringNaturalLines || pageBreak.kind === "block";
}

/** 由切页方案 + 文档构建装饰集：各缝（块前，side:-1）。末页补白通过 CSS padding-bottom 实现，
 *  不进入 DecorationSet（避免每次 keystroke 重映射 widget 导致 DOM 变更触发滚动跳动）。
 *  审计修复（2026-07-25）：end-fill 从 Decoration.widget 改为 CSS --rm-page-end-fill。 */
function buildDecorations(
  doc: PMNode,
  plan: PageBreakPlan,
  measuringNaturalLines = false,
): DecorationSet {
  if (plan.breaks.length === 0) return DecorationSet.empty;
  const decos: Decoration[] = [];
  plan.breaks.forEach((brk, ord) => {
    if (!shouldRenderPageBreak(brk, measuringNaturalLines)) return;
    if (brk.pos < 0 || brk.pos > doc.content.size) return;
    const pageNumber = ord + 2;
    decos.push(
      Decoration.widget(brk.pos, () => renderGap(pageNumber, brk), {
        side: -1,
        key: brk.id,
        marks: [],
        ignoreSelection: true,
        stopEvent: () => true,
      }),
    );
  });
  return DecorationSet.create(doc, decos);
}

/** 更新稳定分页缝 DOM 的 fill；同一断点 id 的高度变化不得重建 Decoration.widget。 */
function applyGapFills(dom: HTMLElement, plan: PageBreakPlan) {
  const fills = new Map(plan.breaks.map((brk) => [brk.id, Math.max(0, brk.fill)]));
  dom.querySelectorAll<HTMLElement>(".rm-page-gap[data-rm-page-break-id]").forEach((gap) => {
    const value = `${fills.get(gap.dataset.rmPageBreakId ?? "") ?? 0}px`;
    if (gap.style.getPropertyValue("--rm-page-fill") !== value) {
      gap.style.setProperty("--rm-page-fill", value);
    }
  });
}

/** 将末页补白高度写入 CSS 变量，使 .ProseMirror 通过 padding-bottom 撑开。
 *  仅在 lastFill 变化时才更新 DOM，消除每次 keystroke 的 widget 重映射开销。 */
function applyEndFill(dom: HTMLElement, lastFill: number) {
  const value = `${Math.max(0, lastFill)}px`;
  if (dom.style.getPropertyValue("--rm-page-end-fill") === value) return;
  dom.style.setProperty("--rm-page-end-fill", value);
}

type PaginationViewportAnchor = {
  paper: HTMLElement;
  position: number;
  kind: "selection-dom" | "selection-pos" | "block";
  beforeTop: number;
};

function rectIntersectsViewport(
  rect: Pick<DOMRect, "top" | "bottom">,
  viewport: Pick<DOMRect, "top" | "bottom">,
): boolean {
  return rect.bottom > viewport.top && rect.top < viewport.bottom;
}

function scrollContainerFor(view: EditorView): HTMLElement | null {
  return (view.dom as HTMLElement).closest(".editor-paper") as HTMLElement | null;
}

/**
 * 读取浏览器当前实际绘制的光标位置。分页缝是 widget，位于 PM position 的同一边界时
 * `coordsAtPos` 可按 decoration side 返回缝的另一侧；原生 Selection range 才是用户正在看见的
 * 光标位置。仅接受属于本编辑器的选择，避免页面其它控件的 selection 混入。
 */
function renderedSelectionTop(view: EditorView): number | null {
  const selection = view.root instanceof Document
    ? view.root.getSelection()
    : view.root.ownerDocument.getSelection();
  if (!selection?.rangeCount) return null;
  const focusNode = selection.focusNode;
  if (!focusNode || !view.dom.contains(focusNode)) return null;
  try {
    const range = selection.getRangeAt(0);
    const rect = range.getBoundingClientRect();
    if (Number.isFinite(rect.top) && (rect.height > 0 || rect.width > 0)) return rect.top;
    const lineRect = range.getClientRects().item(0);
    return lineRect && Number.isFinite(lineRect.top) ? lineRect.top : null;
  } catch {
    return null;
  }
}

/** `Selection` 在某些 WebKit decoration 更新期间会短暂不可读，按 position 重建 DOM range 作降级。 */
function renderedPositionTop(view: EditorView, position: number): number | null {
  try {
    const range = document.createRange();
    const point = view.domAtPos(position);
    range.setStart(point.node, point.offset);
    range.collapse(true);
    const rect = range.getBoundingClientRect();
    const top = Number.isFinite(rect.top) && (rect.height > 0 || rect.width > 0)
      ? rect.top
      : range.getClientRects().item(0)?.top;
    range.detach();
    return top !== undefined && Number.isFinite(top) ? top : null;
  } catch {
    return null;
  }
}

/**
 * 在结构分页提交前捕获可见锚：优先当前光标，退化为视口首个内容块。
 * 分页不改变 ProseMirror 文档，position 在 decoration 提交后仍可解析。
 */
function captureViewportAnchor(view: EditorView): PaginationViewportAnchor | null {
  const paper = scrollContainerFor(view);
  if (!paper) return null;
  const viewport = paper.getBoundingClientRect();
  const selectionTop = renderedSelectionTop(view);
  if (selectionTop !== null && selectionTop >= viewport.top && selectionTop < viewport.bottom) {
    return {
      paper,
      position: view.state.selection.head,
      kind: "selection-dom",
      beforeTop: selectionTop,
    };
  }
  const positionTop = renderedPositionTop(view, view.state.selection.head);
  if (positionTop !== null && positionTop >= viewport.top && positionTop < viewport.bottom) {
    return {
      paper,
      position: view.state.selection.head,
      kind: "selection-pos",
      beforeTop: positionTop,
    };
  }
  for (const { pos } of flattenBlocks(view.state.doc)) {
    const node = view.nodeDOM(pos);
    if (!(node instanceof HTMLElement)) continue;
    const rect = node.getBoundingClientRect();
    if (rectIntersectsViewport(rect, viewport)) {
      return { paper, position: pos, kind: "block", beforeTop: rect.top };
    }
  }
  return null;
}

function anchorTopAfterLayout(view: EditorView, anchor: PaginationViewportAnchor): number | null {
  if (anchor.kind === "selection-dom") {
    return renderedSelectionTop(view) ?? renderedPositionTop(view, anchor.position);
  }
  if (anchor.kind === "selection-pos") return renderedPositionTop(view, anchor.position);
  const node = view.nodeDOM(anchor.position);
  return node instanceof HTMLElement ? node.getBoundingClientRect().top : null;
}

/**
 * 分页结构变化后只补偿一次。若用户已在该帧手动滚动，则不覆盖用户意图。
 * editor-paper 未缩放时比例为 1；仍运行时校准，避免未来容器缩放引入坐标混算。
 */
function restoreViewportAnchor(view: EditorView, anchor: PaginationViewportAnchor): void {
  const afterTop = anchorTopAfterLayout(view, anchor);
  if (afterTop === null) return;
  const paperRect = anchor.paper.getBoundingClientRect();
  const viewportPixelsPerScrollPixel = paperRect.height / anchor.paper.clientHeight;
  const delta = computeViewportAnchorScrollDelta({
    beforeTop: anchor.beforeTop,
    afterTop,
    viewportPixelsPerScrollPixel,
  });
  if (!shouldApplyViewportAnchorCorrection(delta)) return;
  const target = computeClampedViewportScrollTop({
    currentScrollTop: anchor.paper.scrollTop,
    delta,
    scrollHeight: anchor.paper.scrollHeight,
    clientHeight: anchor.paper.clientHeight,
  });
  if (Math.abs(target - anchor.paper.scrollTop) > 0.5) {
    anchor.paper.scrollTop = target;
    view.dom.dispatchEvent(new CustomEvent(PAGINATION_VIEWPORT_CORRECTION_EVENT, {
      detail: { scrollTop: anchor.paper.scrollTop },
    }));
  }
}

/**
 * 位置差分法块高纯逻辑（2026-07-03 页高修复二轮，供测量与单测共用）：
 * 块 i 实占高 = tops[i+1] − tops[i]——**天然包含 margin 塌陷后的真实间距**。旧法
 * （rect.height + marginTop + marginBottom 逐块求和）把每对相邻塌陷 margin 计了两份，
 * 加上 h1/ul 等 UA 默认 margin 放大，累计导致 fill 补偿不足 → 页偏矮（方形/扁矩形，#8 二轮根因）。
 * 末块 = lastBottom − tops[last] + lastMarginBottom。Σheights = 精确渲染跨度 → fill 补足后
 * 页视觉高度精确等于纸张比例。
 */
export function diffExtents(tops: number[], lastBottom: number, lastMarginBottom: number): number[] {
  const n = tops.length;
  if (n === 0) return [];
  const heights: number[] = [];
  for (let i = 0; i < n - 1; i++) heights.push(Math.max(0, tops[i + 1] - tops[i]));
  heights.push(Math.max(0, lastBottom - tops[n - 1] + lastMarginBottom));
  return heights;
}

export interface RenderedBlockExtent {
  pos: number;
  endPos: number;
  top: number;
  bottom: number;
}

export interface RenderedPageGap {
  id: string;
  /** 分页缝在当前 ProseMirror 文档中的映射位置。 */
  pos: number;
  /** 已归一到未缩放 CSS px 的实际布局高度。 */
  height: number;
}

export type GapPositionBias = "before" | "after";

/** 指定 position 之前的分页缝累计高度；after 包含同 position 的 side:-1 widget，before 不包含。 */
export function gapHeightBeforePosition(
  gaps: RenderedPageGap[],
  pos: number,
  bias: GapPositionBias,
): number {
  return gaps.reduce((height, gap) => {
    const precedes = gap.pos < pos || (bias === "after" && gap.pos === pos);
    return precedes ? height + Math.max(0, gap.height) : height;
  }, 0);
}

/** 可见分页树坐标恢复为无分页缝的连续内容坐标。 */
export function normalizeRenderedPositionY(
  renderedY: number,
  pos: number,
  bias: GapPositionBias,
  gaps: RenderedPageGap[],
): number {
  return renderedY - gapHeightBeforePosition(gaps, pos, bias);
}

/** 从带分页缝的可见布局恢复无缝内容高度序列。
 *  分页缝本身没有外边距，widget 的实际 rect.height 已包含其内部 fill/gutter 形成的全部布局高度。
 *  只读取既有几何，不向正在编辑的 DOM 写入临时测量 class，避免 WebKit 在隐藏/恢复缝时重锚定滚动。 */
export function diffExtentsWithoutPageGaps(
  blocks: RenderedBlockExtent[],
  gaps: RenderedPageGap[],
  lastMarginBottom: number,
): number[] {
  if (blocks.length === 0) return [];
  const tops = blocks.map((block) => normalizeRenderedPositionY(
    block.top,
    block.pos,
    "after",
    gaps,
  ));
  const lastBlock = blocks[blocks.length - 1];
  return diffExtents(
    tops,
    normalizeRenderedPositionY(lastBlock.bottom, lastBlock.endPos, "before", gaps),
    lastMarginBottom,
  );
}

/** rect 坐标 → 未缩放 CSS px 的换算因子（2026-07-04 三轮修复·实测定标）：
 *  真机诊断证实 **WKWebView 的 getBoundingClientRect 不包含 CSS zoom**（rect 即未缩放 CSS px，
 *  原先 /zoom 归一化属过度校正——zoom 0.5 时全部尺寸被放大 2 倍）。不假设引擎行为，运行时自校准：
 *  scale = view.dom 的 rect 宽 ÷ computed 宽（computed 恒为未缩放 CSS px）。rect 含 zoom 的引擎
 *  scale≈zoom、不含的引擎 scale≈1——除以 scale 在两种引擎下都得到正确 CSS px。 */
function rectScale(view: EditorView): number {
  const rectW = view.dom.getBoundingClientRect().width;
  const layoutW = (view.dom as HTMLElement).offsetWidth;
  if (!Number.isFinite(rectW) || !Number.isFinite(layoutW) || layoutW <= 0 || rectW <= 0) return 1;
  return rectW / layoutW;
}

interface MeasurableBlock {
  node: PMNode;
  pos: number;
  dom: HTMLElement | null;
  extent: RenderedBlockExtent;
}

type SegmenterPart = { index: number };
type GraphemeSegmenter = { segment(input: string): Iterable<SegmenterPart> };
type SegmenterConstructor = new (
  locales?: string | string[],
  options?: { granularity: "grapheme" },
) => GraphemeSegmenter;

type NaturalLineRect = Pick<DOMRect, "top" | "bottom" | "left" | "right" | "width" | "height">;

interface ParagraphFlowPositions {
  positions: number[];
  contentEnd: number;
}

function graphemeOffsets(text: string): number[] {
  const Segmenter = (Intl as unknown as { Segmenter?: SegmenterConstructor }).Segmenter;
  if (Segmenter) {
    const segmenter = new Segmenter(undefined, { granularity: "grapheme" });
    return Array.from(segmenter.segment(text), (part) => part.index);
  }
  const offsets: number[] = [];
  let offset = 0;
  for (const codePoint of text) {
    offsets.push(offset);
    offset += codePoint.length;
  }
  return offsets;
}

/**
 * @GOV
 * codes:    BR-ED-DATA-003-GUARD-ED-ED-EDIT-020
 * type:     GUARD
 * chain:    ED-EDIT
 * rules:    BR-ED-DATA-003
 * boundary: in=ProseMirrorParagraphModel | out=NaturalLineSplitEligibility(textOnlyNoHardBreakOrInlineAtom)
 */
export function isParagraphEligibleForNaturalLineSplit(node: PMNode): boolean {
  if (node.type.name !== "paragraph" || !node.isTextblock || node.content.size === 0) return false;
  let eligible = true;
  node.forEach((child) => {
    // 显式 hardBreak 会产生未填满的短行，也可能来自 WebKit 在旧 widget 邻近位置的模型污染。
    // 第一阶段不在含 hardBreak/inline atom 的段落内分页，宁可整段原子换页。
    if (!child.isText) eligible = false;
  });
  return eligible;
}

/** 第一阶段仅允许顶层普通段落；inline atom/media 使整个段落安全降级为原子 Block。 */
function paragraphLineBreakPositions(
  view: EditorView,
  block: MeasurableBlock,
): ParagraphFlowPositions | null {
  if (!isParagraphEligibleForNaturalLineSplit(block.node)
    || !block.dom
    || block.dom.parentElement !== view.dom) return null;

  const contentStart = block.pos + 1;
  const contentEnd = contentStart + block.node.content.size;
  const positions = new Set<number>([contentStart]);
  block.node.forEach((child, offset) => {
    const childStart = contentStart + offset;
    if (child.isText) {
      graphemeOffsets(child.text ?? "").forEach((graphemeOffset) => {
        const position = childStart + graphemeOffset;
        if (position < contentEnd) positions.add(position);
      });
    }
  });
  if (positions.size < 2) return null;
  return {
    positions: [...positions].sort((a, b) => a - b),
    contentEnd,
  };
}

/**
 * 行尾断点的保守验证：后一个 grapheme 必须进入更低的视觉行，且水平方向回到行首一侧。
 * 仅纵坐标改变不足以证明换行——字号、上下标等 inline 样式也会改变字形 top。
 */
/**
 * @GOV
 * codes:    BR-ED-DATA-003-GUARD-ED-ED-EDIT-018
 * type:     GUARD
 * chain:    ED-EDIT
 * rules:    BR-ED-DATA-003
 * boundary: in=AdjacentGraphemeClientRectanglesAndTextDirection | out=NaturalVisualLineBoundaryDecision
 */
export function isNaturalLineReset(
  previous: NaturalLineRect,
  next: NaturalLineRect,
  direction: "ltr" | "rtl",
): boolean {
  const verticallyAdvanced = next.top > previous.top + PAGINATION_PLAN_EPSILON
    && next.bottom > previous.bottom + PAGINATION_PLAN_EPSILON;
  if (!verticallyAdvanced) return false;
  return direction === "rtl"
    ? next.right >= previous.right - PAGINATION_PLAN_EPSILON
    : next.left <= previous.left + PAGINATION_PLAN_EPSILON;
}

/** 读取已经撤下段内 widget 的当前可见正文 grapheme 矩形。 */
function renderedTextRect(
  view: EditorView,
  from: number,
  to: number,
): NaturalLineRect | null {
  try {
    const start = view.domAtPos(from, 1);
    const end = view.domAtPos(to, -1);
    const range = document.createRange();
    range.setStart(start.node, start.offset);
    range.setEnd(end.node, end.offset);
    const clientRect = range.getClientRects().item(0);
    const rect = clientRect ?? range.getBoundingClientRect();
    range.detach();
    if (!Number.isFinite(rect.top)
      || !Number.isFinite(rect.bottom)
      || !Number.isFinite(rect.left)
      || !Number.isFinite(rect.right)
      || (rect.width <= 0 && rect.height <= 0)) return null;
    return rect;
  } catch {
    return null;
  }
}

/**
 * 在当前可见 ProseMirror 中定位自然视觉行起点。调用方必须先在同一帧撤下 paragraph-line widget；
 * 因而测量宽度、字体、marks、缩放与最终编辑排版完全同源，不再使用逻辑副本推测行尾。
 */
function measureParagraphFlowBlock(
  view: EditorView,
  block: MeasurableBlock,
  renderedFlowHeight: number,
): RenderedFlowBlock | null {
  const flowPositions = paragraphLineBreakPositions(view, block);
  if (!flowPositions || !block.dom) return null;
  const { positions, contentEnd } = flowPositions;
  const scale = rectScale(view);
  const rectCache = new Map<number, NaturalLineRect | null>();
  const rectAt = (index: number) => {
    if (!rectCache.has(index)) {
      rectCache.set(index, renderedTextRect(
        view,
        positions[index],
        positions[index + 1] ?? contentEnd,
      ));
    }
    return rectCache.get(index) ?? null;
  };
  const blockRect = block.dom.getBoundingClientRect();
  const blockTop = blockRect.top / scale;
  if (!Number.isFinite(blockTop)
    || !Number.isFinite(renderedFlowHeight)
    || renderedFlowHeight <= PAGINATION_PLAN_EPSILON) return null;

  const writingMode = window.getComputedStyle(block.dom).writingMode;
  if (writingMode && writingMode !== "horizontal-tb") return null;
  const direction = window.getComputedStyle(block.dom).direction === "rtl" ? "rtl" : "ltr";
  let currentIndex = 0;
  let currentRect = rectAt(0);
  if (!currentRect || currentRect.top / scale < blockTop - PAGINATION_PLAN_EPSILON) return null;

  const linePositions = [positions[0]];
  const lineTops = [currentRect.top / scale];
  while (currentIndex < positions.length - 1) {
    let probe = currentIndex + 1;
    let step = 1;
    let probeRect = rectAt(probe);
    if (!probeRect || probeRect.top < currentRect.top - PAGINATION_PLAN_EPSILON) return null;
    while (probeRect.top <= currentRect.top + PAGINATION_PLAN_EPSILON
      && probe < positions.length - 1) {
      step *= 2;
      probe = Math.min(positions.length - 1, currentIndex + step);
      probeRect = rectAt(probe);
      if (!probeRect || probeRect.top < currentRect.top - PAGINATION_PLAN_EPSILON) return null;
    }
    if (probeRect.top <= currentRect.top + PAGINATION_PLAN_EPSILON) break;

    let low = currentIndex + 1;
    let high = probe;
    while (low < high) {
      const mid = Math.floor((low + high) / 2);
      const midRect = rectAt(mid);
      if (!midRect || midRect.top < currentRect.top - PAGINATION_PLAN_EPSILON) return null;
      if (midRect.top > currentRect.top + PAGINATION_PLAN_EPSILON) high = mid;
      else low = mid + 1;
    }
    const nextRect = rectAt(low);
    if (!nextRect || nextRect.top <= currentRect.top + PAGINATION_PLAN_EPSILON) return null;
    const previousRect = rectAt(low - 1);
    if (!previousRect || !isNaturalLineReset(previousRect, nextRect, direction)) return null;
    linePositions.push(positions[low]);
    lineTops.push(nextRect.top / scale);
    currentIndex = low;
    currentRect = nextRect;
  }

  if (linePositions.length < 2) return null;
  const heights: number[] = [];
  for (let index = 0; index < lineTops.length - 1; index++) {
    const from = index === 0 ? blockTop : lineTops[index];
    heights.push(lineTops[index + 1] - from);
  }
  heights.push(renderedFlowHeight - (lineTops[lineTops.length - 1] - blockTop));
  if (heights.some((height) => !Number.isFinite(height) || height <= PAGINATION_PLAN_EPSILON)) return null;
  if (Math.abs(heights.reduce((sum, height) => sum + height, 0) - renderedFlowHeight)
    > PAGINATION_PLAN_EPSILON) return null;

  return {
    blockPos: block.pos,
    kind: "paragraph",
    fragments: heights.map((height, lineIndex): RenderedFlowFragment => ({
      pos: lineIndex === 0 ? block.pos : linePositions[lineIndex],
      height,
      kind: lineIndex === 0 ? "block" : "paragraph-line",
      blockPos: block.pos,
      lineIndex,
      lineCount: heights.length,
    })),
  };
}

/** 测量视觉流并直接生成 position 型分页方案。
 *  先以位置差分取得所有 Block 高度，只对当前页实际放不下的顶层普通段落启动自然视觉行定位。
 *  调用方在同一 rAF 内只撤下旧 paragraph-line widget；Block 页缝一直保留并从坐标中扣除。
 *  无法取得可靠行几何时按原子 Block 继续。 */
function measurePaginationPlan(
  view: EditorView,
  plan: PageBreakPlan,
  budget: number,
  paragraphGapsRendered: boolean,
): PageBreakPlan | null {
  const doc = view.state.doc;
  const scale = rectScale(view);
  const renderedGapHeights = new Map<string, number>();
  view.dom.querySelectorAll<HTMLElement>(".rm-page-gap[data-rm-page-break-id]").forEach((gap) => {
    const id = gap.dataset.rmPageBreakId;
    if (id) renderedGapHeights.set(id, gap.getBoundingClientRect().height / scale);
  });
  const expectedRenderedBreaks = paragraphGapsRendered
    ? plan.breaks
    : plan.breaks.filter((pageBreak) => pageBreak.kind === "block");
  // 当应在场的分页缝尚未挂入 DOM 时，本轮不以不完整坐标重排；paragraph-line 在真实行测量态下
  // 被有意撤下，不属于缺失。
  if (!expectedRenderedBreaks.every((pageBreak) => renderedGapHeights.has(pageBreak.id))) return null;
  const renderedGaps: RenderedPageGap[] = expectedRenderedBreaks.map((pageBreak) => ({
    id: pageBreak.id,
    pos: pageBreak.pos,
    height: renderedGapHeights.get(pageBreak.id) ?? 0,
  }));

  const blocks: MeasurableBlock[] = [];
  let lastBottom = 0;
  let lastDom: HTMLElement | null = null;
  // 展平遍历：容器块（orderedList/bulletList）透明，listItem 直接参与测量。
  flattenBlocks(doc).forEach(({ node, pos }) => {
    const dom = view.nodeDOM(pos);
    if (!(dom instanceof HTMLElement)) {
      blocks.push({
        node,
        pos,
        dom: null,
        extent: { pos, endPos: pos + node.nodeSize, top: lastBottom, bottom: lastBottom },
      });
      return;
    }
    const rect = dom.getBoundingClientRect();
    const top = rect.top / scale;
    const bottom = rect.bottom / scale;
    blocks.push({
      node,
      pos,
      dom,
      extent: { pos, endPos: pos + node.nodeSize, top, bottom },
    });
    lastBottom = bottom;
    lastDom = dom;
  });
  // marginBottom 只有末块需要——大文档（数千块）逐块 getComputedStyle 是主要卡顿源之一（2026-07-04）。
  const lastMb = lastDom ? parseFloat(window.getComputedStyle(lastDom).marginBottom) || 0 : 0;
  const heights = diffExtentsWithoutPageGaps(
    blocks.map((block) => block.extent),
    renderedGaps,
    lastMb,
  );
  const planner = createPaginationFlowPlanner(budget);
  blocks.forEach((block, index) => {
    const measuredHeight = heights[index] ?? 0;
    const atomic: RenderedFlowBlock = {
      blockPos: block.pos,
      kind: "atomic",
      fragments: [{
        pos: block.pos,
        height: measuredHeight,
        kind: "block",
        blockPos: block.pos,
        lineIndex: 0,
        lineCount: 1,
      }],
    };
    if (!planner.wouldOverflow(measuredHeight)) {
      planner.append(atomic);
      return;
    }
    const paragraph = measureParagraphFlowBlock(view, block, measuredHeight);
    planner.append(paragraph ?? atomic);
  });
  return planner.finish();
}

/** 把几何写为编辑器 CSS 变量（TS 值为单一来源，CSS 消费）。 */
function applyGeometryVars(dom: HTMLElement, geo: PageGeometry) {
  dom.style.setProperty("--rm-page-width", `${geo.width}px`);
  dom.style.setProperty("--rm-page-height", `${geo.height}px`);
  dom.style.setProperty("--rm-page-margin-x", `${geo.marginX}px`);
  dom.style.setProperty("--rm-page-margin-y", `${geo.marginY}px`);
  dom.style.setProperty("--rm-page-gap", `${geo.gap}px`);
}

declare module "@tiptap/core" {
  interface Commands<ReturnType> {
    pagination: {
      /** 开/关分页呈现（docx 编辑开、md/纯文本关）；关闭即复位所有分页缝、无残留。 */
      setPagination: (enabled: boolean) => ReturnType;
      /** 切换纸张几何（A4/A3/… × 横竖排，PI-ED-06）：更新 CSS 变量并触发重新切页。 */
      setPageGeometry: (geo: PageGeometry) => ReturnType;
      /** 内容装载后的唯一分页配置入口：重置旧文档 plan 并一次性同步开关/纸张几何。 */
      syncPaginationLayout: (enabled: boolean, geo?: PageGeometry) => ReturnType;
    };
  }
  interface Storage {
    pagination: { geometry: PageGeometry };
  }
}

/**
 * @GOV
 * codes:    BR-ED-DATA-003-RB-ED-ED-EDIT-005
 * type:     RB
 * chain:    ED-EDIT
 * rules:    BR-ED-DATA-003
 * boundary: in=ProseMirrorDocumentAndPageGeometry | out=pageBreakDecorations(presentationOnlyNoModelChangeNoDirty) | delegate=paginationFlow
 */
/// 分页呈现守则（BR-ED-DATA-003）：按测量视觉流在 Block 边界或普通段落视觉行间叠加分页缝
/// Decoration.widget（含页尾补足使每页恒等所选纸张比例，PI-ED-06），不改文档模型、不落盘、分页重排
/// 不产生脏态（仅 setMeta 事务、docChanged=false）；复杂节点与不可测段落原子降级。经 ED 声明的
/// Decoration 扩展点叠加不替换 DOM、不阻挡编辑（X-FORBID-005）；打印/导出归 PRV。
export const PaginationExtension = Extension.create({
  name: "pagination",

  addStorage() {
    return { geometry: PAGE_GEOMETRY };
  },

  onCreate() {
    applyGeometryVars(this.editor.view.dom as HTMLElement, this.storage.geometry);
  },

  addCommands() {
    return {
      setPagination:
        (enabled: boolean) =>
        ({ view }) => {
          const current = paginationKey.getState(view.state);
          if (current?.enabled === enabled) return true;
          const viewportAnchor = current?.enabled ? captureViewportAnchor(view) : null;
          view.dispatch(view.state.tr.setMeta(paginationKey, { enabled } as PaginationMeta));
          if (viewportAnchor) restoreViewportAnchor(view, viewportAnchor);
          return true;
        },
      setPageGeometry:
        (geo: PageGeometry) =>
        ({ view }) => {
          if (samePageGeometry(this.storage.geometry, geo)) return true;
          const current = paginationKey.getState(view.state);
          const viewportAnchor = current?.enabled ? captureViewportAnchor(view) : null;
          this.storage.geometry = geo;
          applyGeometryVars(view.dom as HTMLElement, geo);
          view.dispatch(view.state.tr.setMeta(paginationKey, { geometryChanged: true } as PaginationMeta));
          if (viewportAnchor) restoreViewportAnchor(view, viewportAnchor);
          return true;
        },
      syncPaginationLayout:
        (enabled: boolean, geo?: PageGeometry) =>
        ({ view }) => {
          const current = paginationKey.getState(view.state);
          const viewportAnchor = current?.enabled ? captureViewportAnchor(view) : null;
          const geometryChanged = geo ? !samePageGeometry(this.storage.geometry, geo) : false;
          if (geo && geometryChanged) {
            this.storage.geometry = geo;
            applyGeometryVars(view.dom as HTMLElement, geo);
          }
          view.dispatch(view.state.tr.setMeta(paginationKey, {
            enabled,
            geometryChanged,
            layoutReset: true,
          } as PaginationMeta));
          if (viewportAnchor) restoreViewportAnchor(view, viewportAnchor);
          return true;
        },
    };
  },

  addProseMirrorPlugins() {
    const ext = this;
    return [
      new Plugin<PaginationState>({
        key: paginationKey,
        state: {
          init: () => ({
            enabled: false,
            plan: EMPTY_PLAN,
            decorations: DecorationSet.empty,
            measuringNaturalLines: false,
            measureRevision: 0,
            measureDelay: 0,
          }),
          apply(tr, prev, _oldState, newState) {
            const meta = tr.getMeta(paginationKey) as PaginationMeta | undefined;
            let enabled = prev.enabled;
            let plan = prev.plan;
            const measuringNaturalLines = meta?.measuringNaturalLines ?? prev.measuringNaturalLines;
            const measurementModeChanged = measuringNaturalLines !== prev.measuringNaturalLines;
            if (meta?.enabled !== undefined) enabled = meta.enabled;
            if (meta?.plan !== undefined) plan = meta.plan;
            else if (tr.docChanged) plan = mapPageBreakPlan(prev.plan, tr);
            const requestedDelay = paginationMeasureDelay({
              wasEnabled: prev.enabled,
              enabled,
              docChanged: tr.docChanged,
              geometryChanged: meta?.geometryChanged === true,
              layoutReset: meta?.layoutReset === true,
            });
            const measureRevision = prev.measureRevision + Number(requestedDelay !== null);
            const measureDelay = requestedDelay ?? prev.measureDelay;
            if (!enabled) {
              // 关闭：复位所有分页缝 + end-fill，无残留。
              return {
                enabled,
                plan: EMPTY_PLAN,
                decorations: DecorationSet.empty,
                measuringNaturalLines: false,
                measureRevision,
                measureDelay,
              };
            }
            // 新文档必须清空上一文档 plan；纸张变化则保留旧断点到即时结算，避免先撤缝再加缝。
            if (meta?.layoutReset) {
              return {
                enabled,
                plan: EMPTY_PLAN,
                decorations: DecorationSet.empty,
                measuringNaturalLines: false,
                measureRevision,
                measureDelay,
              };
            }
            // 同步自然行测量会暂时撤下 paragraph-line widget；最终 plan 与测量模式在同一事务恢复。
            // 普通结算仍仅在断点结构变化时重建，fill 继续走稳定 DOM CSS 变量。
            if (meta?.plan !== undefined || measurementModeChanged) {
              return {
                enabled,
                plan,
                decorations: measurementModeChanged || shouldRebuildPageGapDecorations(prev.plan, plan)
                  ? buildDecorations(newState.doc, plan, measuringNaturalLines)
                  : prev.decorations,
                measuringNaturalLines,
                measureRevision,
                measureDelay,
              };
            }
            const mappedDecorations = tr.docChanged
              ? prev.decorations.map(tr.mapping, tr.doc)
              : prev.decorations;
            return {
              enabled,
              plan,
              // 正常输入只映射现有 widget，维持 450ms 冻结；仅断点被删除/降级时按映射后计划收口。
              decorations: tr.docChanged && plan.breaks.length !== prev.plan.breaks.length
                ? buildDecorations(newState.doc, plan, measuringNaturalLines)
                : mappedDecorations,
              measuringNaturalLines,
              measureRevision,
              measureDelay,
            };
          },
        },
        props: {
          decorations(state) {
            return paginationKey.getState(state)?.decorations ?? DecorationSet.empty;
          },
        },
        view: (initialView: EditorView) => {
          const syncClass = (v: EditorView) => {
            const on = paginationKey.getState(v.state)?.enabled ?? false;
            v.dom.classList.toggle("rm-paginated", on);
          };
          syncClass(initialView);
          let measureDebounce: number | null = null;
          let measureFrame: number | null = null;
          let destroyed = false;
          let measureEpoch = 0;
          const clearScheduledMeasure = () => {
            if (measureDebounce !== null) {
              window.clearTimeout(measureDebounce);
              measureDebounce = null;
            }
            if (measureFrame !== null) {
              cancelAnimationFrame(measureFrame);
              measureFrame = null;
            }
          };
          const commitSettledPlan = (
            view: EditorView,
            next: PageBreakPlan,
            epoch: number,
            preMeasurementAnchor: PaginationViewportAnchor | null,
          ) => {
            const cur = paginationKey.getState(view.state);
            if (!cur?.enabled || destroyed || epoch !== measureEpoch) return;
            const viewportAnchor = preMeasurementAnchor ?? captureViewportAnchor(view);
            if (samePlan(next, cur.plan)) {
              view.dispatch(
                view.state.tr
                  .setMeta(paginationKey, {
                    settled: true,
                    measuringNaturalLines: false,
                  } as PaginationMeta)
                  .setMeta("addToHistory", false),
              );
              if (viewportAnchor) restoreViewportAnchor(view, viewportAnchor);
              return;
            }
            view.dispatch(
              view.state.tr
                .setMeta(paginationKey, {
                  plan: next,
                  settled: true,
                  measuringNaturalLines: false,
                } as PaginationMeta)
                .setMeta("addToHistory", false),
            );
            // ProseMirror 已先完成 Decoration/Selection/preserve；这里只校验并修正剩余残差。
            if (viewportAnchor && !destroyed && epoch === measureEpoch) {
              restoreViewportAnchor(view, viewportAnchor);
            }
          };
          const measureAfterLayout = (view: EditorView, epoch: number, retry = 0) => {
            measureFrame = requestAnimationFrame(() => {
              measureFrame = null;
              let preMeasurementAnchor: PaginationViewportAnchor | null = null;
              try {
                if (destroyed || view.composing || epoch !== measureEpoch) return;
                const beforeMeasure = paginationKey.getState(view.state);
                if (!beforeMeasure?.enabled) return;
                const hasParagraphGaps = beforeMeasure.plan.breaks.some(
                  (pageBreak) => pageBreak.kind === "paragraph-line",
                );
                if (hasParagraphGaps) {
                  // 同一 rAF 回调内先撤下段内 widget、强制读取真实可见行盒、再恢复最终 plan。
                  // 浏览器在回调返回前不会绘制中间态；Block 页缝始终保留，物理分页未被关闭。
                  preMeasurementAnchor = captureViewportAnchor(view);
                  view.dispatch(
                    view.state.tr
                      .setMeta(paginationKey, { measuringNaturalLines: true } as PaginationMeta)
                      .setMeta("addToHistory", false),
                  );
                }
                const budget = pageContentHeight(ext.storage.geometry);
                const nextPlan = measurePaginationPlan(
                  view,
                  beforeMeasure.plan,
                  budget,
                  !hasParagraphGaps,
                );
                if (!nextPlan) {
                  if (hasParagraphGaps && paginationKey.getState(view.state)?.measuringNaturalLines) {
                    view.dispatch(
                      view.state.tr
                        .setMeta(paginationKey, { measuringNaturalLines: false } as PaginationMeta)
                        .setMeta("addToHistory", false),
                    );
                    if (preMeasurementAnchor) restoreViewportAnchor(view, preMeasurementAnchor);
                  }
                  if (retry < 2) measureAfterLayout(view, epoch, retry + 1);
                  else {
                    view.dispatch(
                      view.state.tr
                        .setMeta(paginationKey, { settled: true } as PaginationMeta)
                        .setMeta("addToHistory", false),
                    );
                  }
                  return;
                }
                commitSettledPlan(view, nextPlan, epoch, preMeasurementAnchor);
              } catch {
                if (paginationKey.getState(view.state)?.measuringNaturalLines) {
                  view.dispatch(
                    view.state.tr
                      .setMeta(paginationKey, { measuringNaturalLines: false } as PaginationMeta)
                      .setMeta("addToHistory", false),
                  );
                  if (preMeasurementAnchor) restoreViewportAnchor(view, preMeasurementAnchor);
                }
                // 测量异常不能让 BDocx 永久停留在加载遮罩；保留当前 plan 并显式结束本轮。
                if (!destroyed && epoch === measureEpoch && paginationKey.getState(view.state)?.enabled) {
                  view.dispatch(
                    view.state.tr
                      .setMeta(paginationKey, { settled: true } as PaginationMeta)
                      .setMeta("addToHistory", false),
                  );
                }
              }
            });
          };
          const scheduleMeasure = (view: EditorView, delay: number) => {
            const epoch = ++measureEpoch;
            clearScheduledMeasure();
            const st = paginationKey.getState(view.state);
            if (!st?.enabled || view.composing) return;
            measureDebounce = window.setTimeout(() => {
              measureDebounce = null;
              measureAfterLayout(view, epoch);
            }, delay);
          };
          // 只订阅真正的外部布局输入。不能观察整个 ProseMirror：分页 widget/end-fill 本身会改变其高度，
          // 进而把自身渲染误判为内容变化，导致页面越靠后越严重的重排振荡。
          const onExternalLayoutChange = () => scheduleMeasure(initialView, 0);
          const onCompositionStart = () => {
            measureEpoch += 1;
            clearScheduledMeasure();
          };
          const onCompositionEnd = () => scheduleMeasure(initialView, PAGINATION_INPUT_SETTLE_MS);
          const fontSet = typeof document !== "undefined" ? document.fonts : undefined;
          initialView.dom.addEventListener("load", onExternalLayoutChange, true);
          initialView.dom.addEventListener("error", onExternalLayoutChange, true);
          initialView.dom.addEventListener("compositionstart", onCompositionStart);
          initialView.dom.addEventListener("compositionend", onCompositionEnd);
          window.addEventListener("resize", onExternalLayoutChange);
          fontSet?.addEventListener?.("loadingdone", onExternalLayoutChange);
          void fontSet?.ready.then(() => {
            if (!destroyed) onExternalLayoutChange();
          });
          return {
            destroy: () => {
              destroyed = true;
              clearScheduledMeasure();
              initialView.dom.removeEventListener("load", onExternalLayoutChange, true);
              initialView.dom.removeEventListener("error", onExternalLayoutChange, true);
              initialView.dom.removeEventListener("compositionstart", onCompositionStart);
              initialView.dom.removeEventListener("compositionend", onCompositionEnd);
              window.removeEventListener("resize", onExternalLayoutChange);
              fontSet?.removeEventListener?.("loadingdone", onExternalLayoutChange);
            },
            update: (view: EditorView, previousState) => {
              const st = paginationKey.getState(view.state);
              const previous = paginationKey.getState(previousState);
              view.dom.classList.toggle("rm-paginated", st?.enabled ?? false);
              // 同步稳定 gap/end-fill CSS 变量；fill 改变不能重建同一 widget。
              if (st?.enabled) applyGapFills(view.dom as HTMLElement, st.plan);
              applyEndFill(view.dom as HTMLElement, st?.enabled ? (st.plan.lastFill || 0) : 0);
              if (st?.enabled && st.measureRevision !== previous?.measureRevision) {
                scheduleMeasure(view, st.measureDelay);
              }
            },
          };
        },
      }),
    ];
  },
});
