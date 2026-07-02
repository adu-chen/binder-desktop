import { Extension } from "@tiptap/core";
import { Plugin, PluginKey } from "@tiptap/pm/state";
import { Decoration, DecorationSet } from "@tiptap/pm/view";
import type { EditorView } from "@tiptap/pm/view";
import type { Node as PMNode } from "@tiptap/pm/model";

/**
 * 分页呈现层扩展（ED-EDIT · BR-ED-DATA-003 · PI-ED-04）。
 * A4/Word 风格分页：按测量的顶层内容块高度在**块边界**插入分页缝（Decoration.widget），
 * 内容连续流经缝隙自然下推——纯呈现层，**不改文档模型、不落盘、不产生脏态**（BR-ED-DATA-003）；
 * 经 ED 声明的 ProseMirror Decoration 扩展点叠加，不替换 DOM、不阻挡编辑（X-FORBID-005）。
 *
 * 切页判定（哪些块前插缝）是纯逻辑 `computePageBreaks`，与 DOM 测量解耦、可单测；实际块高测量、
 * 分页缝渲染属视觉层，jsdom 无布局引擎不可单测，真机验收（A-ED-M-T-01 §10 量化 NFR 延 G5）。
 * 仅在 `enabled` 时分页（docx 编辑默认开、md/纯文本关，人类决策 2026-07-02）；块级不切分：
 * 超整页的单块（大图/长表）独占其页并溢出，为块粒度 A 类债务（A-ED-M-T-01 §11.2）。
 */

/** A4 @96dpi 页面几何（px）——呈现层常量，不参与落盘。 */
export const PAGE_GEOMETRY = {
  /** 纸张宽度 */
  width: 794,
  /** 纸张高度 */
  height: 1123,
  /** 左右页边距 */
  marginX: 76,
  /** 上下页边距 */
  marginY: 60,
  /** 页间视觉间隙（灰色装订缝，不含相邻页上下白边） */
  gap: 28,
} as const;

/** 每页可用内容高度 = 纸高 − 上下页边距（切页高度预算）。 */
export const PAGE_CONTENT_HEIGHT = PAGE_GEOMETRY.height - PAGE_GEOMETRY.marginY * 2;

/**
 * 切页判定纯逻辑（BR-ED-DATA-003 核心，供插件与单测共用）：给定文档顺序的顶层块高度序列与每页可用
 * 内容高度，返回应在其**之前**插入分页缝的块下标（升序、去重）。规则（块级不切分，Word 风格）：
 * 逐块累加，累计高度 + 当前块高超过页高则在当前块前换页、从该块起重新累计；首块不换页；
 * 单块高于整页时独占其页（在其前换页，除非它已是页首）——不产生错位。全放得下 / 空序列 → 无缝。
 */
export function computePageBreaks(blockHeights: number[], pageContentHeight: number): number[] {
  if (pageContentHeight <= 0) return [];
  const breaks: number[] = [];
  let used = 0;
  for (let i = 0; i < blockHeights.length; i++) {
    const h = blockHeights[i];
    // 非首块且当页已有内容、加上本块会溢出 → 本块前换页（本块起为新页首）。
    if (i > 0 && used > 0 && used + h > pageContentHeight) {
      breaks.push(i);
      used = 0;
    }
    used += h;
  }
  return breaks;
}

/** 两个数字数组浅相等（切页结果是否变化的判定，避免无谓重算/派发导致循环）。 */
function sameBreaks(a: number[], b: number[]): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return false;
  return true;
}

interface PaginationState {
  enabled: boolean;
  breaks: number[];
  decorations: DecorationSet;
}

interface PaginationMeta {
  enabled?: boolean;
  breaks?: number[];
}

export const paginationKey = new PluginKey<PaginationState>("pagination");

/** 分页缝 widget：白色上下页边 + 灰色装订缝（含页码），全宽出血覆盖页边距。纯呈现、不可交互。 */
function renderGap(pageNumber: number): HTMLElement {
  const gap = document.createElement("div");
  gap.className = "rm-page-gap";
  gap.contentEditable = "false";
  const gutter = document.createElement("div");
  gutter.className = "rm-page-gap-gutter";
  const num = document.createElement("span");
  num.className = "rm-page-gap-num";
  num.textContent = String(pageNumber);
  gutter.appendChild(num);
  gap.appendChild(gutter);
  return gap;
}

/** 由切页下标 + 文档构建装饰集：每个下标处（块前，side:-1）插入一个分页缝 widget。 */
function buildDecorations(doc: PMNode, breaks: number[]): DecorationSet {
  if (breaks.length === 0) return DecorationSet.empty;
  // 顶层子块的起始位置（childPos[i] = 第 i 个顶层块前的文档位置）。
  const childPos: number[] = [];
  let p = 0;
  doc.forEach((node) => {
    childPos.push(p);
    p += node.nodeSize;
  });
  const decos: Decoration[] = [];
  breaks.forEach((blockIdx, ord) => {
    const pos = childPos[blockIdx];
    if (pos === undefined) return;
    const pageNumber = ord + 2; // 第一个缝之后是第 2 页
    decos.push(Decoration.widget(pos, () => renderGap(pageNumber), { side: -1, key: `pg-${pageNumber}` }));
  });
  return DecorationSet.create(doc, decos);
}

/** 测量顶层内容块的高度序列（含上下外边距，近似每块对页高预算的占用）。
 *  编辑预览区缩放（CSS zoom，src/components/ed/zoom.ts）时 getBoundingClientRect 返回缩放后尺寸，
 *  按 view.dom 的 computed zoom 归一化到未缩放 CSS px——使页高预算（PAGE_CONTENT_HEIGHT，未缩放常量）
 *  自洽，分页缝随内容一同缩放而不漂移（自读 computed 值，不耦合 zoom 视图态）。 */
function measureBlockHeights(view: EditorView): number[] {
  const heights: number[] = [];
  const doc = view.state.doc;
  const zoomRaw = window.getComputedStyle(view.dom).zoom;
  const zoom = parseFloat(zoomRaw) || 1;
  let p = 0;
  doc.forEach((_node) => {
    const pos = p;
    p += _node.nodeSize;
    const dom = view.nodeDOM(pos);
    if (!(dom instanceof HTMLElement)) {
      heights.push(0);
      return;
    }
    const rect = dom.getBoundingClientRect();
    const cs = window.getComputedStyle(dom);
    const mt = parseFloat(cs.marginTop) || 0;
    const mb = parseFloat(cs.marginBottom) || 0;
    // rect 为缩放后尺寸 → 除以 zoom 归一；外边距 computed 值按未缩放处理直接相加。
    heights.push(rect.height / zoom + mt + mb);
  });
  return heights;
}

declare module "@tiptap/core" {
  interface Commands<ReturnType> {
    pagination: {
      /** 开/关分页呈现（docx 编辑开、md/纯文本关）；关闭即复位所有分页缝、无残留。 */
      setPagination: (enabled: boolean) => ReturnType;
    };
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
/// 分页呈现守则（BR-ED-DATA-003）：按测量块高在块边界叠加分页缝 Decoration.widget，内容连续流经缝隙
/// 自然分页——不改文档模型、不落盘、分页重排不产生脏态（仅 setMeta 事务、docChanged=false，不触发
/// onUpdate 脏态）；经 ED 声明的 Decoration 扩展点叠加不替换 DOM、不阻挡编辑（X-FORBID-005）；
/// 开关切换即时复位无残留。打印/导出归 PRV，本扩展仅提供打印友好的分页呈现。
export const PaginationExtension = Extension.create({
  name: "pagination",

  onCreate() {
    // 页面几何写为 CSS 变量（TS 常量为单一来源，CSS 消费，避免两处漂移）。
    const dom = this.editor.view.dom as HTMLElement;
    dom.style.setProperty("--rm-page-width", `${PAGE_GEOMETRY.width}px`);
    dom.style.setProperty("--rm-page-margin-x", `${PAGE_GEOMETRY.marginX}px`);
    dom.style.setProperty("--rm-page-margin-y", `${PAGE_GEOMETRY.marginY}px`);
    dom.style.setProperty("--rm-page-gap", `${PAGE_GEOMETRY.gap}px`);
  },

  addCommands() {
    return {
      setPagination:
        (enabled: boolean) =>
        ({ view }) => {
          view.dispatch(view.state.tr.setMeta(paginationKey, { enabled } as PaginationMeta));
          return true;
        },
    };
  },

  addProseMirrorPlugins() {
    return [
      new Plugin<PaginationState>({
        key: paginationKey,
        state: {
          init: () => ({ enabled: false, breaks: [], decorations: DecorationSet.empty }),
          apply(tr, prev, _oldState, newState) {
            const meta = tr.getMeta(paginationKey) as PaginationMeta | undefined;
            let enabled = prev.enabled;
            let breaks = prev.breaks;
            if (meta?.enabled !== undefined) enabled = meta.enabled;
            if (meta?.breaks !== undefined) breaks = meta.breaks;
            if (!enabled) {
              // 关闭：复位所有分页缝，无残留。
              return { enabled, breaks: [], decorations: DecorationSet.empty };
            }
            // 切页结果或文档变化 → 重建装饰；否则映射旧集保持锚定。
            if (meta?.breaks !== undefined) {
              return { enabled, breaks, decorations: buildDecorations(newState.doc, breaks) };
            }
            return {
              enabled,
              breaks,
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
          // DOM 类切换（分页纸张观感）：随 enabled 变化增删 .rm-paginated。
          const syncClass = (v: EditorView) => {
            const on = paginationKey.getState(v.state)?.enabled ?? false;
            v.dom.classList.toggle("rm-paginated", on);
          };
          syncClass(initialView);
          return {
            update: (view: EditorView) => {
              const st = paginationKey.getState(view.state);
              view.dom.classList.toggle("rm-paginated", st?.enabled ?? false);
              if (!st?.enabled) return;
              // 下一帧测量（避免读取半布局）；仅当切页结果变化才派发 setMeta（收敛、防循环）。
              requestAnimationFrame(() => {
                try {
                  const heights = measureBlockHeights(view);
                  const next = computePageBreaks(heights, PAGE_CONTENT_HEIGHT);
                  const cur = paginationKey.getState(view.state);
                  if (!cur?.enabled) return;
                  if (!sameBreaks(next, cur.breaks)) {
                    view.dispatch(
                      view.state.tr.setMeta(paginationKey, { breaks: next } as PaginationMeta).setMeta("addToHistory", false),
                    );
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
