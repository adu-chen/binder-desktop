import { Extension } from "@tiptap/core";
import { Plugin, PluginKey, TextSelection } from "@tiptap/pm/state";
import { Decoration, DecorationSet, type EditorView } from "@tiptap/pm/view";
import type { Transaction } from "@tiptap/pm/state";

/**
 * SelectionPersistExtension → ED-SELECTION-PERSIST-01。
 *
 * 统一链路（v2）：选区高亮**始终**由 PM inline Decoration（class `pm-selection-persist`）渲染，
 * 浏览器原生 `::selection` 视觉已透明禁用（styles.css）——原生是"连续大块"（line-box），span 是
 * "分行"（内容盒），两套渲染视觉不一致。统一后：
 *   - 聚焦拖选：`decorations` 实时跟随非空 TextSelection → span 高亮（与失焦同样式）。
 *   - 失焦：WKWebView 清除 DOM selection → PM selection 变 collapsed → 用插件记录的 lastNonEmpty
 *     渲染 span，保持高亮。
 *   - 用户点击编辑器内（focus + collapsed）→ 恢复原生行为（无高亮）。
 * 不修改 selection state、不替换 DOM、不阻挡编辑（X-FORBID-005）。
 *
 * 事件桥接说明：ProseMirror 的 `someProp("handleDOMEvents")` 只取**第一个**非空
 * handleDOMEvents（组件级 editorProps 优先，遮蔽插件级）。因此 blur/focus 的事件监听由 Editor.tsx
 * 的 `editorProps.handleDOMEvents` 承担，经导出函数桥接：blur 延迟派发 persist 事务（待 PM
 * handlers.blur 执行、view.hasFocus() 归 false 后触发重算 → 用记录渲染）；focus 派发 clear 事务
 * （清记录，恢复原生）。
 */

export const selectionPersistKey = new PluginKey<PersistState>("selectionPersist");

export const SELECTION_PERSIST_CLASS = "pm-selection-persist";

/** 插件 state：最后非空选区记录（失焦时序安全）。装饰不再存于 state，由 decorations 实时派生。 */
export interface PersistState {
  /** 最后观察到的非空 TextSelection（from/to）；失焦同步事务（collapsed）不覆盖。 */
  lastNonEmpty: { from: number; to: number } | null;
}

/** 选区高亮装饰纯逻辑：用指定范围产出 inline Decoration。 */
export function buildPersistDecorations(
  doc: import("@tiptap/pm/model").Node,
  range: { from: number; to: number },
): DecorationSet {
  return DecorationSet.create(doc, [
    Decoration.inline(range.from, range.to, { class: SELECTION_PERSIST_CLASS }),
  ]);
}

/** Editor.tsx blur 桥接：派发 persist 事务（触发 decorations 重算，用记录的 lastNonEmpty 渲染）。 */
export function persistSelectionOnBlur(view: EditorView): void {
  view.dispatch(view.state.tr.setMeta(selectionPersistKey, "persist"));
}

/** Editor.tsx focus 桥接：clear 事务（清记录，恢复原生——selection 若非空将实时重新渲染）。 */
export function clearPersistedSelection(view: EditorView): void {
  const current = selectionPersistKey.getState(view.state);
  if (!current?.lastNonEmpty) return;
  view.dispatch(view.state.tr.setMeta(selectionPersistKey, "clear"));
}

/**
 * 事务应用逻辑：
 *   - selectionSet 且非空 TextSelection → 更新 lastNonEmpty（聚焦拖选/点击文本实时跟随）。
 *   - selectionSet 但 collapsed → 不覆盖记录（失焦同步场景）。
 *   - clear 信号 → 清记录（用户点击编辑器内恢复原生）。
 *   - persist / 其他事务 → 保留记录（装饰由 decorations 实时派生，事务仅触发重算）。
 */
function applySelectionPersist(tr: Transaction, old: PersistState): PersistState {
  const incoming = tr.getMeta(selectionPersistKey) as "persist" | "clear" | undefined;

  if (incoming === "clear") {
    return { lastNonEmpty: null };
  }

  let lastNonEmpty = old.lastNonEmpty;
  if (tr.selectionSet && incoming !== "persist") {
    const sel = tr.selection;
    if (sel instanceof TextSelection && !sel.empty) {
      lastNonEmpty = { from: sel.from, to: sel.to };
    }
    // collapsed：保留既有记录（失焦同步场景），不覆盖。
  }
  return { lastNonEmpty };
}

export const SelectionPersistExtension = Extension.create({
  name: "selectionPersist",

  addProseMirrorPlugins() {
    // 插件级 view 引用：decorations 需读 view.focused 区分"失焦保留"与"用户点击空白处"。
    let viewRef: EditorView | null = null;
    return [
      new Plugin<PersistState>({
        key: selectionPersistKey,
        state: {
          init: () => ({ lastNonEmpty: null }),
          apply(tr, old) {
            return applySelectionPersist(tr, old);
          },
        },
        view(view) {
          viewRef = view;
          return {};
        },
        props: {
          decorations(state) {
            const sel = state.selection;
            // 统一链路：聚焦时非空 TextSelection 也实时渲染 span（与失焦同样式）。
            if (sel instanceof TextSelection && !sel.empty) {
              return buildPersistDecorations(state.doc, { from: sel.from, to: sel.to });
            }
            // 失焦（selection 已被同步清空）：用记录的 lastNonEmpty 保持高亮。
            const s = selectionPersistKey.getState(state);
            if (s?.lastNonEmpty && viewRef && !viewRef.hasFocus()) {
              return buildPersistDecorations(state.doc, s.lastNonEmpty);
            }
            return DecorationSet.empty;
          },
        },
      }),
    ];
  },
});
