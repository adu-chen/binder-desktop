import { Extension } from "@tiptap/core";
import { Plugin, PluginKey, TextSelection } from "@tiptap/pm/state";
import { Decoration, DecorationSet, type EditorView } from "@tiptap/pm/view";
import type { Transaction } from "@tiptap/pm/state";

/**
 * SelectionPersistExtension → ED-SELECTION-PERSIST-01。
 *
 * 失焦（blur）后浏览器原生 `::selection` 不再渲染高亮，视觉上选区"消失"（实际 ProseMirror
 * selection state 仍保留）。本扩展在 blur 时把选区范围挂一层 inline Decoration
 * （class `pm-selection-persist`）作为视觉副本，保持高亮直到下次光标操作：
 *   - focus（用户点击编辑器内，含空白处）→ 移除装饰，恢复原生行为。
 * 不修改 selection state、不替换 DOM、不阻挡编辑（X-FORBID-005）。
 *
 * 关键设计（失焦时序）：WKWebView 失焦时可能**先**清除 DOM selection → PM 同步 selection 变
 * collapsed，随后 blur 事件才到达。因此插件 state 记录**最后非空 TextSelection**，blur 挂装饰
 * 使用该记录而非 blur 时已可能被清空的实时 selection；失焦同步事务（collapsed）不覆盖记录。
 *
 * 事件桥接说明：ProseMirror 的 `someProp("handleDOMEvents")` 只取**第一个**非空
 * handleDOMEvents（组件级 editorProps 优先，遮蔽插件级）。因此 blur/focus 的事件监听由 Editor.tsx
 * 的 `editorProps.handleDOMEvents` 承担，经下方导出函数桥接本扩展（persist/clear 事务）。
 */

export const selectionPersistKey = new PluginKey<PersistState>("selectionPersist");

export const SELECTION_PERSIST_CLASS = "pm-selection-persist";

/** 观测开关：DevTools 中执行 window.__ED_SEL_PERSIST_DEBUG__=true 后，真实链路打点。 */
declare global {
  interface Window {
    __ED_SEL_PERSIST_DEBUG__?: boolean;
  }
}

/** 插件 state：装饰集 + 最后非空选区记录（失焦时序安全）。 */
export interface PersistState {
  decos: DecorationSet;
  /** 最后观察到的非空 TextSelection（from/to）；失焦同步事务不覆盖。 */
  lastNonEmpty: { from: number; to: number } | null;
}

/** blur 挂持久化装饰的纯逻辑：用记录的（或当前）非空 TextSelection 产出 inline Decoration。 */
export function buildPersistDecorations(
  doc: import("@tiptap/pm/model").Node,
  range: { from: number; to: number },
): DecorationSet {
  return DecorationSet.create(doc, [
    Decoration.inline(range.from, range.to, { class: SELECTION_PERSIST_CLASS }),
  ]);
}

/** Editor.tsx blur 桥接：派发 persist 事务（用插件记录的 lastNonEmpty 挂装饰）。 */
export function persistSelectionOnBlur(view: EditorView): void {
  // 观测层（ED-SELECTION-PERSIST-01 诊断）：真实链路打点，确认 blur 是否到达。
  if (window.__ED_SEL_PERSIST_DEBUG__) {
    const sel = view.state.selection;
    const domSel = window.getSelection();
    // eslint-disable-next-line no-console
    console.info("[ED-SEL-PERSIST] blur", {
      pmSelection: sel instanceof TextSelection ? [sel.from, sel.to] : "non-text",
      pmEmpty: sel instanceof TextSelection ? sel.empty : undefined,
      domSelection: domSel?.rangeCount ? domSel.toString().slice(0, 24) : null,
      lastNonEmpty: selectionPersistKey.getState(view.state)?.lastNonEmpty ?? null,
    });
  }
  view.dispatch(view.state.tr.setMeta(selectionPersistKey, "persist"));
}

/** Editor.tsx focus 桥接：clear 事务（移除装饰，恢复原生）。已有装饰才派发，避免无谓事务。 */
export function clearPersistedSelection(view: EditorView): void {
  const current = selectionPersistKey.getState(view.state);
  if (!current || !current.decos.find(0, 1e9).length) return;
  // 观测层（ED-SELECTION-PERSIST-01 诊断）：确认 focus 清除触发。
  if (window.__ED_SEL_PERSIST_DEBUG__) {
    // eslint-disable-next-line no-console
    console.info("[ED-SEL-PERSIST] focus → clear（装饰被移除）");
  }
  view.dispatch(view.state.tr.setMeta(selectionPersistKey, "clear"));
}

/**
 * 事务应用逻辑：
 *   - persist 信号 → 用记录的 lastNonEmpty（若无则当前 selection 若非空）挂装饰。
 *   - clear 信号 → 清装饰与记录。
 *   - selectionSet 事务 → 更新 lastNonEmpty（仅非空选区覆盖；collapsed 不覆盖，失焦同步不破坏记录）。
 *   - 其他事务 → 装饰随 mapping 平移保留。
 */
function applySelectionPersist(tr: Transaction, old: PersistState): PersistState {
  const incoming = tr.getMeta(selectionPersistKey) as "persist" | "clear" | undefined;

  if (incoming === "clear") {
    return { decos: DecorationSet.empty, lastNonEmpty: null };
  }

  // 先处理 selection 记录更新（persist 事务自身不改变 selection）。
  let lastNonEmpty = old.lastNonEmpty;
  if (tr.selectionSet && !tr.getMeta(selectionPersistKey)) {
    const sel = tr.selection;
    if (sel instanceof TextSelection && !sel.empty) {
      lastNonEmpty = { from: sel.from, to: sel.to };
    }
    // collapsed：保留既有记录（失焦同步场景），不覆盖。
  }

  if (incoming === "persist") {
    const range = lastNonEmpty
      ?? (tr.selection instanceof TextSelection && !tr.selection.empty
        ? { from: tr.selection.from, to: tr.selection.to }
        : null);
    if (!range) return { decos: DecorationSet.empty, lastNonEmpty };
    const decos = buildPersistDecorations(tr.doc, range);
    // 观测层（ED-SELECTION-PERSIST-01 诊断）：确认 persist 挂载的 from/to。
    if (window.__ED_SEL_PERSIST_DEBUG__) {
      // eslint-disable-next-line no-console
      console.info("[ED-SEL-PERSIST] persist → 挂装饰", { range });
    }
    return { decos, lastNonEmpty };
  }

  // 其他事务：装饰随 mapping 平移保留。
  return { decos: old.decos.map(tr.mapping, tr.doc), lastNonEmpty };
}

export const SelectionPersistExtension = Extension.create({
  name: "selectionPersist",

  addProseMirrorPlugins() {
    return [
      new Plugin<PersistState>({
        key: selectionPersistKey,
        state: {
          init: () => ({ decos: DecorationSet.empty, lastNonEmpty: null }),
          apply(tr, old) {
            return applySelectionPersist(tr, old);
          },
        },
        props: {
          decorations(state) {
            return selectionPersistKey.getState(state)?.decos ?? DecorationSet.empty;
          },
        },
      }),
    ];
  },
});
