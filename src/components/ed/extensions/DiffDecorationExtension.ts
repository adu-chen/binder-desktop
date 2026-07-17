import { Extension } from "@tiptap/core";
import { Plugin, PluginKey } from "@tiptap/pm/state";
import { Decoration, DecorationSet } from "@tiptap/pm/view";
import type { EditorView } from "@tiptap/pm/view";
import type { GreenReviewState } from "../../../store/deStore";

/**
 * DiffDecorationExtension → GreenReview（TERM-ED-003 · ED-DECOR · PI-FE-07）。
 * 基于 ProseMirror Decoration 按 blockId 把 diff 审阅态与 before/after 预览叠加在正文之上：
 * 不替换 DOM、不阻挡编辑（X-FORBID-005）；4 态 pending/applying/accepted/expired。与右侧 DiffCard
 * 同源（deStore）→ 双向同步。
 */

// PluginKey 以插件 state（DecorationSet）为类型；GreenReview 映射经 setMeta 透传（meta 不约束类型）。
export const greenReviewKey = new PluginKey<DecorationSet>("greenReview");

export interface GreenReviewEntry {
  state: GreenReviewState;
  before?: string;
  after?: string;
}
export type GreenReviewValue = GreenReviewState | GreenReviewEntry;

/** 由外部（Editor 从 deStore）推送 blockId→GreenReview 映射，触发装饰重算。 */
export function applyGreenReview(view: EditorView, map: Map<string, GreenReviewValue>) {
  view.dispatch(view.state.tr.setMeta(greenReviewKey, map));
}

/** 块引用（文档顺序的 blockId 与位置/尺寸）。 */
export interface BlockRef {
  blockId: string | null | undefined;
  pos: number;
  size: number;
}
/** 装饰规格（位置 + class）。 */
export interface DecoSpec {
  pos: number;
  size: number;
  className: string;
}
/** 编辑器内只读 before/after 预览 widget 规格。 */
export interface PreviewWidgetSpec {
  pos: number;
  key: string;
  state: GreenReviewState;
  before?: string;
  after?: string;
}

const PREVIEW_LIMIT = 240;

function normalizeReview(value: GreenReviewValue | undefined): GreenReviewEntry | null {
  if (!value) return null;
  return typeof value === "string" ? { state: value } : value;
}

function previewKey(blockId: string, entry: GreenReviewEntry): string {
  const raw = `${blockId}\u0000${entry.state}\u0000${entry.before ?? ""}\u0000${entry.after ?? ""}`;
  let hash = 5381;
  for (let i = 0; i < raw.length; i++) hash = (hash * 33) ^ raw.charCodeAt(i);
  return `green-review-preview-${blockId}-${entry.state}-${hash >>> 0}`;
}

function previewText(text: string): string {
  const trimmed = text.trim();
  if (!trimmed) return "（空）";
  return trimmed.length > PREVIEW_LIMIT ? `${trimmed.slice(0, PREVIEW_LIMIT)}...` : trimmed;
}

/**
 * GreenReview 装饰选择纯逻辑（BR-ED-STATE-002 核心，供 buildDecorations 与单测共用）：
 * 仅对 blockId 命中映射的块产出装饰（class `green-review green-review-{态}`）；无 blockId 或未命中
 * （含 rejected 不入映射）→ 无装饰（复位）；空映射 → 空集。不替换 DOM、不阻挡编辑（X-FORBID-005）。
 */
export function selectDecorations(
  blocks: BlockRef[],
  map: Map<string, GreenReviewValue>,
): DecoSpec[] {
  if (map.size === 0) return [];
  const out: DecoSpec[] = [];
  for (const b of blocks) {
    if (!b.blockId) continue; // 无 blockId 不装饰
    const review = normalizeReview(map.get(b.blockId));
    if (!review) continue; // 未命中（复位/rejected）不装饰
    out.push({ pos: b.pos, size: b.size, className: `green-review green-review-${review.state}` });
  }
  return out;
}

/** before/after 预览 widget 选择纯逻辑：只读附着在目标 Block 之后，accepted 仅保留块态装饰。 */
export function selectPreviewWidgets(
  blocks: BlockRef[],
  map: Map<string, GreenReviewValue>,
): PreviewWidgetSpec[] {
  if (map.size === 0) return [];
  const out: PreviewWidgetSpec[] = [];
  for (const b of blocks) {
    if (!b.blockId) continue;
    const review = normalizeReview(map.get(b.blockId));
    if (!review || review.state === "accepted") continue;
    if (review.before === undefined && review.after === undefined) continue;
    out.push({
      pos: b.pos + b.size,
      key: previewKey(b.blockId, review),
      state: review.state,
      before: review.before,
      after: review.after,
    });
  }
  return out;
}

function renderPreviewLine(kind: "before" | "after", mark: "-" | "+", text: string): HTMLElement {
  const row = document.createElement("div");
  row.className = `green-review-preview-line green-review-preview-${kind}`;

  const marker = document.createElement("span");
  marker.className = "green-review-preview-mark";
  marker.textContent = mark;

  const body = document.createElement("span");
  body.className = "green-review-preview-text";
  body.textContent = previewText(text);

  row.append(marker, body);
  return row;
}

function renderPreviewWidget(spec: PreviewWidgetSpec): HTMLElement {
  const wrap = document.createElement("div");
  wrap.className = `green-review-preview green-review-preview-${spec.state}`;
  wrap.setAttribute("contenteditable", "false");
  if (spec.before !== undefined) wrap.append(renderPreviewLine("before", "-", spec.before));
  if (spec.after !== undefined) wrap.append(renderPreviewLine("after", "+", spec.after));
  return wrap;
}

function buildDecorations(doc: import("@tiptap/pm/model").Node, map: Map<string, GreenReviewValue>) {
  if (map.size === 0) return DecorationSet.empty;
  const blocks: BlockRef[] = [];
  doc.descendants((node, pos) => {
    blocks.push({ blockId: node.attrs?.blockId ?? undefined, pos, size: node.nodeSize });
  });
  const nodeDecos = selectDecorations(blocks, map).map((d) =>
    Decoration.node(d.pos, d.pos + d.size, { class: d.className }),
  );
  const previewDecos = selectPreviewWidgets(blocks, map).map((d) =>
    Decoration.widget(d.pos, () => renderPreviewWidget(d), { key: d.key, side: 1 }),
  );
  const decos: Decoration[] = [...nodeDecos, ...previewDecos];
  return DecorationSet.create(doc, decos);
}

/**
 * @GOV
 * codes:    BR-ED-STATE-002-RB-ED-ED-DECOR-001
 * type:     RB
 * chain:    ED-DECOR
 * rules:    BR-ED-STATE-002
 * term_ref: TERM-ED-002, TERM-ED-003
 * boundary: in=greenReviewEntryMap | out=DecorationSet(blockIdAnchoredOverlayPreviewNoDomReplace)
 */
/// GreenReview 装饰守则（BR-ED-STATE-002）：按 blockId 锚把 diff 审阅 4 态叠加为 ProseMirror
/// Decoration，并以只读 widget 呈现 before/after 预览；叠加于正文不替换 DOM、不阻挡编辑（X-FORBID-005）；
/// 映射由 deStore 单源推送，与 DiffCard 双向同步（X-INTER-001）；rejected 不在映射中即无装饰（复位）。
export const DiffDecorationExtension = Extension.create({
  name: "diffDecoration",

  addProseMirrorPlugins() {
    return [
      new Plugin<DecorationSet>({
        key: greenReviewKey,
        state: {
          init: () => DecorationSet.empty,
          apply(tr, old) {
            const incoming = tr.getMeta(greenReviewKey) as Map<string, GreenReviewValue> | undefined;
            if (incoming) return buildDecorations(tr.doc, incoming);
            return tr.docChanged ? old.map(tr.mapping, tr.doc) : old;
          },
        },
        props: {
          decorations(state) {
            return greenReviewKey.getState(state);
          },
        },
      }),
    ];
  },
});
