import { Extension } from "@tiptap/core";
import { Plugin, PluginKey } from "@tiptap/pm/state";

/**
 * TableIdExtension（BDOCX-TABLE-AGENT-EDIT-01）。
 *
 * tableId 是编辑会话内的表格语义锚：代理 TableOperation 与表格 Diff 只可引用此 id，不能以
 * “第 N 个表”或单元格文本猜测目标。Pandoc 导出不保证保留 data-table-id，因此重开会话会重新补发；
 * 旧会话的操作依靠 sessionVersion/contentHash 过期，不会跨会话错放。
 */
const tableIdPluginKey = new PluginKey("binderTableId");

function makeTableId() {
  return `table_${crypto.randomUUID()}`;
}

export const TableIdExtension = Extension.create({
  name: "tableId",

  addGlobalAttributes() {
    return [{
      types: ["table"],
      attributes: {
        tableId: {
          default: null,
          parseHTML: (element) => element.getAttribute("data-table-id"),
          renderHTML: (attrs) => attrs.tableId ? { "data-table-id": attrs.tableId } : {},
        },
      },
    }];
  },

  addProseMirrorPlugins() {
    return [new Plugin({
      key: tableIdPluginKey,
      appendTransaction: (_transactions, _oldState, newState) => {
        const missing: number[] = [];
        const used = new Set<string>();
        newState.doc.descendants((node, pos) => {
          if (node.type.name !== "table") return;
          const id = typeof node.attrs.tableId === "string" ? node.attrs.tableId : "";
          if (!id || used.has(id)) missing.push(pos);
          else used.add(id);
        });
        if (!missing.length) return null;
        const tr = newState.tr;
        for (const pos of missing) {
          const node = tr.doc.nodeAt(pos);
          if (!node) continue;
          let id = makeTableId();
          while (used.has(id)) id = makeTableId();
          used.add(id);
          tr.setNodeMarkup(pos, undefined, { ...node.attrs, tableId: id });
        }
        tr.setMeta("addToHistory", false);
        return tr;
      },
    })];
  },
});
