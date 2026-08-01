import { Extension } from "@tiptap/core";
import { Plugin, PluginKey } from "@tiptap/pm/state";

const imageIdPluginKey = new PluginKey("binderImageId");

function makeImageId(): string {
  return `image_${crypto.randomUUID()}`;
}

/** 保留首次出现的唯一 ID，只修复缺失或重复项。 */
export function reconcileImageIds(
  ids: Array<string | null>,
  generate: () => string = makeImageId,
): Array<string | null> {
  const used = new Set<string>();
  return ids.map((id) => {
    if (id && !used.has(id)) {
      used.add(id);
      return null;
    }
    let next = generate();
    while (used.has(next)) next = generate();
    used.add(next);
    return next;
  });
}

/**
 * ImageOperation 的会话内稳定图片锚。Pandoc 不保证保留自定义属性，因此重开编辑会话会重新分配；
 * sessionVersion/contentHash 会使旧操作过期，避免跨会话错绑。
 */
export const ImageIdExtension = Extension.create({
  name: "imageId",

  addGlobalAttributes() {
    return [{
      types: ["image"],
      attributes: {
        binderImageId: {
          default: null,
          parseHTML: (element) => element.getAttribute("data-binder-image-id"),
          renderHTML: (attrs) => (
            attrs.binderImageId ? { "data-binder-image-id": attrs.binderImageId } : {}
          ),
        },
      },
    }];
  },

  addProseMirrorPlugins() {
    return [new Plugin({
      key: imageIdPluginKey,
      appendTransaction: (_transactions, _oldState, newState) => {
        const slots: Array<{ pos: number; id: string | null }> = [];
        newState.doc.descendants((node, pos) => {
          if (node.type.name !== "image") return;
          const id = typeof node.attrs.binderImageId === "string" && node.attrs.binderImageId
            ? node.attrs.binderImageId
            : null;
          slots.push({ pos, id });
        });
        const fixes = reconcileImageIds(slots.map((slot) => slot.id));
        if (fixes.every((fix) => fix === null)) return null;
        const transaction = newState.tr;
        fixes.forEach((imageId, index) => {
          if (!imageId) return;
          const node = transaction.doc.nodeAt(slots[index].pos);
          if (node?.type.name === "image") {
            transaction.setNodeMarkup(slots[index].pos, undefined, {
              ...node.attrs,
              binderImageId: imageId,
            });
          }
        });
        transaction.setMeta("addToHistory", false);
        return transaction;
      },
    })];
  },
});
