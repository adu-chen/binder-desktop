import { Extension } from "@tiptap/core";
import { Plugin, PluginKey } from "@tiptap/pm/state";

/**
 * BlockIdExtension（ED-EDIT · PI-FE-05）。
 * 为顶层内容块（Block, TERM-ED-001）赋稳定 blockId，是 diff 定位（DE）、@mention 引用锚（REF）
 * 与分页切分的统一定位基准（A-ED-M-T-01 §4）。生成算法（此处时间戳+随机占位）为 ED-02 实现期债务。
 */

const BLOCK_TYPES = ["paragraph", "heading", "blockquote", "codeBlock", "listItem"];

/** 不透明 blockId 生成（占位实现，实现期换 ULID；A 类债务 A-ED-M-T-01 §10）。 */
function genBlockId(): string {
  return `b_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 8)}`;
}

const blockIdPluginKey = new PluginKey("blockId");

/**
 * @GOV
 * codes:    BR-ED-DATA-001-DATA-ED-ED-EDIT-001
 * type:     DATA
 * chain:    ED-EDIT
 * rules:    BR-ED-DATA-001
 * term_ref: TERM-ED-001
 * boundary: in=editorBlockNode | out=stableBlockId(unchangedOnEdit,invalidatedOnDeleteNoReuse)
 */
/// 内容块稳定 blockId 守则（BR-ED-DATA-001）：每个块创建即赋不透明 blockId；不删除该块的普通编辑
/// 不改变 blockId；块删除则其 id 失效不复用；分裂/复制产生的重复 id 由新块改取新 id（原块保持稳定）。
export const BlockIdExtension = Extension.create({
  name: "blockId",

  addGlobalAttributes() {
    return [
      {
        types: BLOCK_TYPES,
        attributes: {
          blockId: {
            default: null,
            // 持久化到 DOM/序列化，使普通编辑期间 id 随节点稳定保留。
            parseHTML: (el) => el.getAttribute("data-block-id"),
            renderHTML: (attrs) =>
              attrs.blockId ? { "data-block-id": attrs.blockId } : {},
          },
        },
      },
    ];
  },

  addProseMirrorPlugins() {
    return [
      new Plugin({
        key: blockIdPluginKey,
        // 文档变更后补齐缺失 / 去重 blockId：缺失即赋新；重复（复制/分裂）令后出现者改取新 id，
        // 保证既有块 id 稳定、删除不复用、全文唯一。
        appendTransaction: (_transactions, _oldState, newState) => {
          const seen = new Set<string>();
          const fixes: { pos: number; id: string }[] = [];
          newState.doc.descendants((node, pos) => {
            if (!BLOCK_TYPES.includes(node.type.name)) return;
            const id: string | null = node.attrs.blockId ?? null;
            if (id === null || seen.has(id)) {
              fixes.push({ pos, id: genBlockId() });
            } else {
              seen.add(id);
            }
          });
          if (fixes.length === 0) return null;
          const tr = newState.tr;
          for (const fix of fixes) {
            const node = tr.doc.nodeAt(fix.pos);
            if (!node) continue;
            tr.setNodeMarkup(fix.pos, undefined, { ...node.attrs, blockId: fix.id });
            seen.add(fix.id);
          }
          tr.setMeta("addToHistory", false); // id 维护不进撤销栈
          return tr;
        },
      }),
    ];
  },
});
