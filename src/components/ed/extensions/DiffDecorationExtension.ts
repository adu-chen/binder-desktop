import { Extension } from "@tiptap/core";
import { Plugin, PluginKey } from "@tiptap/pm/state";
import { Decoration, DecorationSet } from "@tiptap/pm/view";
import type { EditorView } from "@tiptap/pm/view";
import type { GreenReviewState } from "../../../store/deStore";

/**
 * DiffDecorationExtension → GreenReview（TERM-ED-003 · ED-DECOR · PI-FE-07）。
 * 基于 ProseMirror Decoration 按 blockId 把 diff 审阅态叠加在正文之上：不替换 DOM、不阻挡编辑
 * （X-FORBID-005）；4 态 pending/applying/accepted/expired。与右侧 DiffCard 同源（deStore）→ 双向同步。
 */

// PluginKey 以插件 state（DecorationSet）为类型；GreenReview 映射经 setMeta 透传（meta 不约束类型）。
export const greenReviewKey = new PluginKey<DecorationSet>("greenReview");

/** 由外部（Editor 从 deStore）推送 blockId→GreenReviewState 映射，触发装饰重算。 */
export function applyGreenReview(view: EditorView, map: Map<string, GreenReviewState>) {
  view.dispatch(view.state.tr.setMeta(greenReviewKey, map));
}

function buildDecorations(doc: import("@tiptap/pm/model").Node, map: Map<string, GreenReviewState>) {
  if (map.size === 0) return DecorationSet.empty;
  const decos: Decoration[] = [];
  doc.descendants((node, pos) => {
    const blockId: string | undefined = node.attrs?.blockId ?? undefined;
    if (!blockId) return;
    const reviewState = map.get(blockId);
    if (!reviewState) return;
    decos.push(
      Decoration.node(pos, pos + node.nodeSize, { class: `green-review green-review-${reviewState}` }),
    );
  });
  return DecorationSet.create(doc, decos);
}

/**
 * @GOV
 * codes:    BR-ED-STATE-002-RB-ED-ED-DECOR-001
 * type:     RB
 * chain:    ED-DECOR
 * rules:    BR-ED-STATE-002
 * term_ref: TERM-ED-002, TERM-ED-003
 * boundary: in=greenReviewStateMap | out=DecorationSet(blockIdAnchoredOverlayNoDomReplace)
 */
/// GreenReview 装饰守则（BR-ED-STATE-002）：按 blockId 锚把 diff 审阅 4 态叠加为 ProseMirror
/// node Decoration，叠加于正文不替换 DOM、不阻挡编辑（X-FORBID-005）；映射由 deStore 单源推送，
/// 与 DiffCard 双向同步（X-INTER-001）；rejected 不在映射中即无装饰（复位）。
export const DiffDecorationExtension = Extension.create({
  name: "diffDecoration",

  addProseMirrorPlugins() {
    return [
      new Plugin<DecorationSet>({
        key: greenReviewKey,
        state: {
          init: () => DecorationSet.empty,
          apply(tr, old) {
            const incoming = tr.getMeta(greenReviewKey) as Map<string, GreenReviewState> | undefined;
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
