import { Extension } from "@tiptap/core";
import { Plugin, PluginKey } from "@tiptap/pm/state";

/**
 * BlockIdExtension（ED-EDIT · PI-FE-05）。
 * 为顶层内容块（Block, TERM-ED-001）赋稳定 blockId，是 diff 定位（DE）、@mention 引用锚（REF）
 * 与分页切分的统一定位基准（A-ED-M-T-01 §4）。生成算法：block_ + crypto.randomUUID()（§11.1 选型）。
 */

const BLOCK_TYPES = ["paragraph", "heading", "blockquote", "codeBlock", "listItem"];

/** 不透明 blockId 生成：`block_` + crypto.randomUUID()（A-ED-M-T-01 §11.1 选型记录，G-4 定档）。
 *  既有文档中的旧格式 id 依然有效——reconcile 只补缺/去重，不改既有唯一 id。 */
function genBlockId(): string {
  return `block_${crypto.randomUUID()}`;
}

/**
 * blockId 去重/补齐纯逻辑（BR-ED-DATA-001 核心，供 appendTransaction 与单测共用）。
 * 入参为文档顺序的当前 blockId 序列；返回等长修复序列：`null` = 既有且唯一，保持稳定不变；
 * `string` = 该位置 id 缺失或与前序重复，须改取的新唯一 id（不复用已出现 id、全序列唯一）。
 */
export function reconcileBlockIds(
  ids: (string | null)[],
  gen: () => string = genBlockId,
): (string | null)[] {
  const seen = new Set<string>();
  const fixes: (string | null)[] = [];
  for (const id of ids) {
    if (id !== null && !seen.has(id)) {
      seen.add(id);
      fixes.push(null); // 既有且唯一 → 稳定保持
    } else {
      let nid = gen();
      while (seen.has(nid)) nid = gen(); // 不复用/不冲突
      seen.add(nid);
      fixes.push(nid); // 缺失或重复 → 新 id
    }
  }
  return fixes;
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
          // 收集文档顺序的块位置与当前 id，交纯逻辑 reconcileBlockIds 判定修复。
          const slots: { pos: number; id: string | null }[] = [];
          newState.doc.descendants((node, pos) => {
            if (!BLOCK_TYPES.includes(node.type.name)) return;
            slots.push({ pos, id: node.attrs.blockId ?? null });
          });
          const fixes = reconcileBlockIds(slots.map((s) => s.id));
          if (fixes.every((f) => f === null)) return null;
          const tr = newState.tr;
          fixes.forEach((newId, i) => {
            if (newId === null) return;
            const { pos } = slots[i];
            const node = tr.doc.nodeAt(pos);
            if (!node) return;
            tr.setNodeMarkup(pos, undefined, { ...node.attrs, blockId: newId });
          });
          tr.setMeta("addToHistory", false); // id 维护不进撤销栈
          return tr;
        },
      }),
    ];
  },
});
