import { Extension } from "@tiptap/core";
import { Plugin, PluginKey } from "@tiptap/pm/state";
import { Decoration, DecorationSet } from "@tiptap/pm/view";
import type { EditorView } from "@tiptap/pm/view";
import type { Node as PMNode } from "@tiptap/pm/model";
import {
  getPageGeometry,
  pageContentHeight,
  type PageGeometry,
} from "../../../store/edViewPrefsStore";

/**
 * 分页呈现层扩展（ED-EDIT · BR-ED-DATA-003 · PI-ED-04/PI-ED-06）。
 * A4/Word 风格分页：按测量的顶层内容块高度在**块边界**插入分页缝（Decoration.widget），
 * 内容连续流经缝隙自然下推——纯呈现层，**不改文档模型、不落盘、不产生脏态**（BR-ED-DATA-003）；
 * 经 ED 声明的 ProseMirror Decoration 扩展点叠加，不替换 DOM、不阻挡编辑（X-FORBID-005）。
 *
 * PI-ED-06 页高修复：每个分页缝携带 **fill（页尾留白补足）**——非末页视觉高度恒等所选纸张
 * 内容区预算（Word 语义：内容未满页也补足空白）；末页经 end-filler 撑满整页比例。
 * 纸张几何动态（A4/A3/A5/Letter × 横竖排，edViewPrefsStore 视图偏好）经 setPageGeometry 切换。
 *
 * 切页判定（哪些块前插缝 + 各页补足量）是纯逻辑 `computePageBreaks`，与 DOM 测量解耦、可单测；
 * 实际块高测量、分页缝渲染属视觉层，jsdom 无布局引擎不可单测，真机验收。
 * 仅在 `enabled` 时分页（docx 编辑默认开、md/纯文本关，人类决策 2026-07-02）；块级不切分：
 * 超整页的单块（大图/长表）独占其页并溢出（fill=0），为块粒度 A 类债务（A-ED-M-T-01 §11.2）。
 */

/** 默认几何：A4 竖排（向后兼容导出；动态几何经 setPageGeometry）。 */
export const PAGE_GEOMETRY: PageGeometry = getPageGeometry("A4", "portrait");

/** 默认每页可用内容高度（A4 竖排预算；动态值从当前几何算）。 */
export const PAGE_CONTENT_HEIGHT = pageContentHeight(PAGE_GEOMETRY);

/** 单个分页缝：在块 `index` 前换页，`fill` = 上一页页尾补足高度（px，≥0）。 */
export interface PageBreak {
  index: number;
  fill: number;
}

/** 切页结果：分页缝序列 + 末页补足高度（撑满整页比例）。 */
export interface PageBreakPlan {
  breaks: PageBreak[];
  lastFill: number;
}

/**
 * 切页判定纯逻辑（BR-ED-DATA-003 核心，供插件与单测共用）：给定文档顺序的顶层块高度序列与每页
 * 可用内容高度，返回分页缝（块前换页 + 该页页尾补足量）与末页补足量。规则（块级不切分，Word 风格）：
 * 逐块累加，累计 + 当前块高溢出则在当前块前换页，fill = 预算 − 该页已用（补足页尾空白，钳 ≥0——
 * 单块超整页时该页溢出、fill=0）；首块不换页；末页 lastFill 同理补足。
 * **不变量：非溢出页 used + fill = 预算**（每页视觉高度恒等所选纸张内容区）。空序列 → 无缝 + 满页 fill。
 */
export function computePageBreaks(blockHeights: number[], budget: number): PageBreakPlan {
  if (budget <= 0) return { breaks: [], lastFill: 0 };
  const breaks: PageBreak[] = [];
  let used = 0;
  for (let i = 0; i < blockHeights.length; i++) {
    const h = blockHeights[i];
    // 非首块且当页已有内容、加上本块会溢出 → 本块前换页（本块起为新页首），上一页补足页尾。
    if (i > 0 && used > 0 && used + h > budget) {
      breaks.push({ index: i, fill: Math.max(0, Math.round(budget - used)) });
      used = 0;
    }
    used += h;
  }
  return { breaks, lastFill: Math.max(0, Math.round(budget - used)) };
}

/** 两个切页方案近似相等（±1px 容差消浮点测量抖动，避免 RAF 重算循环）。 */
export function samePlan(a: PageBreakPlan, b: PageBreakPlan): boolean {
  if (a.breaks.length !== b.breaks.length) return false;
  if (Math.abs(a.lastFill - b.lastFill) > 1) return false;
  for (let i = 0; i < a.breaks.length; i++) {
    if (a.breaks[i].index !== b.breaks[i].index) return false;
    if (Math.abs(a.breaks[i].fill - b.breaks[i].fill) > 1) return false;
  }
  return true;
}

interface PaginationState {
  enabled: boolean;
  plan: PageBreakPlan;
  decorations: DecorationSet;
}

interface PaginationMeta {
  enabled?: boolean;
  plan?: PageBreakPlan;
  /** 几何变更（纸张/方向切换）：更新 CSS 变量并强制下轮重测。 */
  geometryChanged?: boolean;
}

const EMPTY_PLAN: PageBreakPlan = { breaks: [], lastFill: 0 };

export const paginationKey = new PluginKey<PaginationState>("pagination");

/** 分页缝 widget：页尾补足(fill) + 白色下页边 + 灰色装订缝（含页码）+ 白色上页边。纯呈现、不可交互。 */
function renderGap(pageNumber: number, fill: number): HTMLElement {
  const gap = document.createElement("div");
  gap.className = "rm-page-gap";
  gap.contentEditable = "false";
  if (fill > 0) {
    const filler = document.createElement("div");
    filler.className = "rm-page-gap-fill";
    filler.style.height = `${fill}px`;
    gap.appendChild(filler);
  }
  const gutter = document.createElement("div");
  gutter.className = "rm-page-gap-gutter";
  const num = document.createElement("span");
  num.className = "rm-page-gap-num";
  num.textContent = String(pageNumber);
  gutter.appendChild(num);
  gap.appendChild(gutter);
  return gap;
}

/** 末页补足 widget：撑满整页比例（内容区余量高度的空白纸面）。 */
function renderEndFill(fill: number): HTMLElement {
  const el = document.createElement("div");
  el.className = "rm-page-end-fill";
  el.contentEditable = "false";
  el.style.height = `${fill}px`;
  return el;
}

/** 由切页方案 + 文档构建装饰集：各缝（块前，side:-1）+ 文档末尾 end-filler。 */
function buildDecorations(doc: PMNode, plan: PageBreakPlan): DecorationSet {
  if (plan.breaks.length === 0 && plan.lastFill <= 0) return DecorationSet.empty;
  // 顶层子块的起始位置（childPos[i] = 第 i 个顶层块前的文档位置）。
  const childPos: number[] = [];
  let p = 0;
  doc.forEach((node) => {
    childPos.push(p);
    p += node.nodeSize;
  });
  const decos: Decoration[] = [];
  plan.breaks.forEach((brk, ord) => {
    const pos = childPos[brk.index];
    if (pos === undefined) return;
    const pageNumber = ord + 2; // 第一个缝之后是第 2 页
    decos.push(
      Decoration.widget(pos, () => renderGap(pageNumber, brk.fill), { side: -1, key: `pg-${pageNumber}-${brk.fill}` }),
    );
  });
  if (plan.lastFill > 0) {
    decos.push(
      Decoration.widget(doc.content.size, () => renderEndFill(plan.lastFill), {
        side: 1,
        key: `pg-end-${plan.lastFill}`,
      }),
    );
  }
  return DecorationSet.create(doc, decos);
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

/** rect 坐标 → 未缩放 CSS px 的换算因子（2026-07-04 三轮修复·实测定标）：
 *  真机诊断证实 **WKWebView 的 getBoundingClientRect 不包含 CSS zoom**（rect 即未缩放 CSS px，
 *  原先 /zoom 归一化属过度校正——zoom 0.5 时全部尺寸被放大 2 倍）。不假设引擎行为，运行时自校准：
 *  scale = view.dom 的 rect 宽 ÷ computed 宽（computed 恒为未缩放 CSS px）。rect 含 zoom 的引擎
 *  scale≈zoom、不含的引擎 scale≈1——除以 scale 在两种引擎下都得到正确 CSS px。 */
function rectScale(view: EditorView): number {
  const rectW = view.dom.getBoundingClientRect().width;
  const cssW = parseFloat(window.getComputedStyle(view.dom).width);
  if (!Number.isFinite(rectW) || !Number.isFinite(cssW) || cssW <= 0 || rectW <= 0) return 1;
  return rectW / cssW;
}

/** 测量顶层内容块的高度序列（位置差分法 · 无缝测量，2026-07-04 三轮修复）：
 *  测量前临时隐藏既有分页缝/end-fill（.rm-measuring，同帧内恢复不产生可见闪烁）——差分测到的
 *  永远是**纯内容跨度**，彻底消灭"缝高剔除补偿"的误差与测量-重排震荡循环（真机诊断：补偿误差
 *  ~36px/缝导致 heightsSum 在两组值间交替、fill 永不收敛、控制台日志刷屏）。 */
function measureBlockHeights(view: EditorView): number[] {
  const doc = view.state.doc;
  view.dom.classList.add("rm-measuring"); // 隐藏缝 widget → 强制无缝布局
  try {
    const scale = rectScale(view);
    const tops: number[] = [];
    let lastBottom = 0;
    let lastMb = 0;
    let p = 0;
    doc.forEach((_node) => {
      const pos = p;
      p += _node.nodeSize;
      const dom = view.nodeDOM(pos);
      if (!(dom instanceof HTMLElement)) {
        tops.push(lastBottom); // 不可测块：锚定上一块底部，占位 0 高（序列对齐）
        return;
      }
      const rect = dom.getBoundingClientRect();
      tops.push(rect.top / scale);
      lastBottom = rect.bottom / scale;
      lastMb = parseFloat(window.getComputedStyle(dom).marginBottom) || 0;
    });
    return diffExtents(tops, lastBottom, lastMb);
  } finally {
    view.dom.classList.remove("rm-measuring");
  }
}

/** 页高自检诊断（真机排查用，console 输出；每次切页方案变化输出一次）：
 *  纸张目标高 vs DOM 实测页高（paper 内容顶→首缝顶、缝底→次缝顶、末缝底→纸底）。 */
function diagnosePageHeights(view: EditorView, geo: PageGeometry, heights: number[], plan: PageBreakPlan) {
  try {
    const zoom = rectScale(view); // 自校准因子（引擎无关；命名沿用便于对照旧日志）
    const paper = view.dom.getBoundingClientRect();
    const cs = window.getComputedStyle(view.dom);
    const padTop = parseFloat(cs.paddingTop) || 0;
    const padBottom = parseFloat(cs.paddingBottom) || 0;
    const gaps = Array.from(view.dom.querySelectorAll<HTMLElement>(".rm-page-gap")).map((g) => {
      const r = g.getBoundingClientRect();
      const gutter = g.querySelector<HTMLElement>(".rm-page-gap-gutter")?.getBoundingClientRect();
      return { top: r.top / zoom, bottom: r.bottom / zoom, h: r.height / zoom, gutterH: (gutter?.height ?? 0) / zoom };
    });
    // 每页视觉高：页1 = 纸顶→缝1 的 gutter 顶 + 上页边?（近似口径：相邻缝 gutter 中点差）——
    // 直接输出原始量供人工判读，不做拟合。
    const pageSpans: number[] = [];
    let cursor = paper.top / zoom;
    for (const g of gaps) {
      pageSpans.push(g.top + 0 - cursor); // 纸面段：内容顶到缝顶（含 padding/fill）
      cursor = g.bottom;
    }
    pageSpans.push(paper.bottom / zoom - cursor);
    console.info("[binder:page] 诊断", {
      geometry: { w: geo.width, h: geo.height, marginY: geo.marginY, gap: geo.gap },
      budget: pageContentHeight(geo),
      zoom,
      paper: { w: paper.width / zoom, h: paper.height / zoom, padTop: padTop / zoom, padBottom: padBottom / zoom },
      heightsSum: Math.round(heights.reduce((a, b) => a + b, 0)),
      blocks: heights.length,
      plan: { breaks: plan.breaks, lastFill: plan.lastFill },
      gapRects: gaps.map((g) => ({ h: Math.round(g.h), gutterH: Math.round(g.gutterH) })),
      pageSpans: pageSpans.map((v) => Math.round(v)),
      expectPageVisual: geo.height,
    });
  } catch {
    /* 诊断失败不影响分页 */
  }
}

/** 把几何写为编辑器 CSS 变量（TS 值为单一来源，CSS 消费）。 */
function applyGeometryVars(dom: HTMLElement, geo: PageGeometry) {
  dom.style.setProperty("--rm-page-width", `${geo.width}px`);
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
 * boundary: in=blockContentHeights | out=pageBreakDecorations(presentationOnlyNoModelChangeNoDirty)
 */
/// 分页呈现守则（BR-ED-DATA-003）：按测量块高在块边界叠加分页缝 Decoration.widget（含页尾补足使
/// 每页恒等所选纸张比例，PI-ED-06），内容连续流经缝隙自然分页——不改文档模型、不落盘、分页重排
/// 不产生脏态（仅 setMeta 事务、docChanged=false）；经 ED 声明的 Decoration 扩展点叠加不替换 DOM、
/// 不阻挡编辑（X-FORBID-005）；纸张切换为视图偏好不落盘；开关切换即时复位无残留。打印/导出归 PRV。
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
          view.dispatch(view.state.tr.setMeta(paginationKey, { enabled } as PaginationMeta));
          return true;
        },
      setPageGeometry:
        (geo: PageGeometry) =>
        ({ view }) => {
          this.storage.geometry = geo;
          applyGeometryVars(view.dom as HTMLElement, geo);
          view.dispatch(view.state.tr.setMeta(paginationKey, { geometryChanged: true } as PaginationMeta));
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
          init: () => ({ enabled: false, plan: EMPTY_PLAN, decorations: DecorationSet.empty }),
          apply(tr, prev, _oldState, newState) {
            const meta = tr.getMeta(paginationKey) as PaginationMeta | undefined;
            let enabled = prev.enabled;
            let plan = prev.plan;
            if (meta?.enabled !== undefined) enabled = meta.enabled;
            if (meta?.plan !== undefined) plan = meta.plan;
            if (!enabled) {
              // 关闭：复位所有分页缝，无残留。
              return { enabled, plan: EMPTY_PLAN, decorations: DecorationSet.empty };
            }
            // 几何变更：清空旧方案（旧 fill 失效），待下轮测量重建。
            if (meta?.geometryChanged) {
              return { enabled, plan: EMPTY_PLAN, decorations: DecorationSet.empty };
            }
            // 切页方案变更 → 重建装饰；否则映射旧集保持锚定。
            if (meta?.plan !== undefined) {
              return { enabled, plan, decorations: buildDecorations(newState.doc, plan) };
            }
            return {
              enabled,
              plan,
              decorations: tr.docChanged ? prev.decorations.map(tr.mapping, tr.doc) : prev.decorations,
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
          // 异步内容尺寸变化（图片加载/字体就绪）不触发 PM update → ResizeObserver 兜底调度重测
          // （2026-07-03 页高修复二轮：RAF 测量时 img 高度为 0 导致含图页错位）。
          let roScheduled = false;
          const ro = typeof ResizeObserver !== "undefined"
            ? new ResizeObserver(() => {
                if (roScheduled) return;
                roScheduled = true;
                requestAnimationFrame(() => {
                  roScheduled = false;
                  // 触发一次空 meta 派发走既有 update 测量路径（幂等：samePlan 收敛不重建）。
                  const st = paginationKey.getState(initialView.state);
                  if (st?.enabled) {
                    initialView.dispatch(initialView.state.tr.setMeta("paginationResize", true));
                  }
                });
              })
            : null;
          ro?.observe(initialView.dom);
          return {
            destroy: () => ro?.disconnect(),
            update: (view: EditorView) => {
              const st = paginationKey.getState(view.state);
              view.dom.classList.toggle("rm-paginated", st?.enabled ?? false);
              if (!st?.enabled) return;
              // IME 组合输入期间挂起重测量（N8）：避免 setMeta 重排打断中文/日文组合输入。
              if (view.composing) return;
              // 下一帧测量（避免读取半布局）；仅当切页方案变化超容差才派发（收敛、防循环）。
              requestAnimationFrame(() => {
                try {
                  if (view.composing) return;
                  const heights = measureBlockHeights(view);
                  const budget = pageContentHeight(ext.storage.geometry);
                  const next = computePageBreaks(heights, budget);
                  const cur = paginationKey.getState(view.state);
                  if (!cur?.enabled) return;
                  if (!samePlan(next, cur.plan)) {
                    view.dispatch(
                      view.state.tr
                        .setMeta(paginationKey, { plan: next } as PaginationMeta)
                        .setMeta("addToHistory", false),
                    );
                    // 页高自检诊断（2026-07-03 #1 三轮排查）：方案落定下一帧实测每页视觉高度 vs 纸高。
                    requestAnimationFrame(() => diagnosePageHeights(view, ext.storage.geometry, heights, next));
                  }
                } catch {
                  // view 可能已卸载/销毁（切文档、关闭）——忽略本次测量。
                }
              });
            },
          };
        },
      }),
    ];
  },
});
