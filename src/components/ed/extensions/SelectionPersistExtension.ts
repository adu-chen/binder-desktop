import { Extension } from "@tiptap/core";
import { Plugin, PluginKey, TextSelection } from "@tiptap/pm/state";
import { Decoration, DecorationSet, type EditorView } from "@tiptap/pm/view";
import { dropPoint } from "prosemirror-transform";
import type { Slice } from "@tiptap/pm/model";
import type { Transaction } from "@tiptap/pm/state";

/**
 * SelectionPersistExtension → ED-SELECTION-PERSIST-01 / ED-SELECTION-DRAG-01。
 *
 * 统一链路（v2）：选区高亮**始终**由 PM inline Decoration（class `pm-selection-persist`）渲染，
 * 浏览器原生 `::selection` 视觉已透明禁用（styles.css）——原生是"连续大块"（line-box），span 是
 * "分行"（内容盒），两套渲染视觉不一致。统一后：
 *   - 聚焦拖选：`decorations` 实时跟随非空 TextSelection → span 高亮（与失焦同样式）。
 *   - 失焦：WKWebView 清除 DOM selection → PM selection 变 collapsed → 用插件记录的 lastNonEmpty
 *     渲染 span，保持高亮。
 *   - 用户点击编辑器内（focus + collapsed）→ 恢复原生行为（无高亮）。
 *
 * 拖动锁定（ED-SELECTION-DRAG-01）：从**选区中间**按下拖动时，WebKit 原生拖选会把 selection 从
 * 按下点重新拖出（篡改），导致 PM 原生 drop 的 deleteSelection() 删错对象、拖出引用锚错位。
 * dragstart 时 Editor.tsx 派发 draglock 信号锁定原选区范围：拖动期间 lastNonEmpty 不跟随篡改的
 * selectionSet、decorations 用锁定范围渲染（高亮稳定）；自定义 handleDrop 删除锁定范围。
 * 不修改 selection state、不替换 DOM、不阻挡编辑（X-FORBID-005）。
 */

export const selectionPersistKey = new PluginKey<PersistState>("selectionPersist");

export const SELECTION_PERSIST_CLASS = "pm-selection-persist";

export type PersistSignal = "persist" | "clear" | "draglock" | "dragunlock";

/** 插件 state：记录 + 拖动锁。装饰不再存于 state，由 decorations 实时派生。 */
export interface PersistState {
  /** 最后观察到的非空 TextSelection（from/to）；失焦同步事务（collapsed）不覆盖。 */
  lastNonEmpty: { from: number; to: number } | null;
  /** dragstart 锁定的原选区范围（拖动期间生效）；锁定期间 selection 篡改被忽略。 */
  dragLock: { from: number; to: number } | null;
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
  if (!current?.lastNonEmpty && !current?.dragLock) return;
  view.dispatch(view.state.tr.setMeta(selectionPersistKey, "clear"));
}

/**
 * Editor.tsx dragstart 桥接（ED-SELECTION-DRAG-01）：锁定当前非空 TextSelection 为拖动原选区。
 * 之后原生拖选篡改 selection 时，lastNonEmpty 与 decorations 均以锁定范围为准。
 */
export function lockDragSelection(view: EditorView): void {
  const sel = view.state.selection;
  if (!(sel instanceof TextSelection) || sel.empty) return;
  view.dispatch(view.state.tr.setMeta(selectionPersistKey, "draglock"));
}

/** Editor.tsx dragend / 自定义 handleDrop 后桥接：释放拖动锁。 */
export function unlockDragSelection(view: EditorView): void {
  const current = selectionPersistKey.getState(view.state);
  if (!current?.dragLock) return;
  view.dispatch(view.state.tr.setMeta(selectionPersistKey, "dragunlock"));
}

/**
 * 选区拖动 drop 处理（ED-SELECTION-DRAG-01）：从**选区中间**按下拖动时，WebKit 原生拖选会篡改
 * selection（从按下点重新拖出），PM 原生 drop 的 deleteSelection() 因此删错对象。本函数在
 * dragstart 已锁定原选区（插件 dragLock）且为 move（非 copy）时接管：删除**锁定范围**而非
 * 当前被篡改的 selection，再插入 drop 位置。非 move / 未锁定返回 false 交回 PM 原生。
 * 挂载点：Editor.tsx editorProps.handleDrop。
 */
export function handleSelectionDragDrop(
  view: EditorView,
  event: DragEvent,
  slice: Slice,
  move: boolean,
): boolean {
  const lock = selectionPersistKey.getState(view.state)?.dragLock;
  if (!lock || !move) return false;
  const eventPos = view.posAtCoords({ left: event.clientX, top: event.clientY });
  if (!eventPos) return false;
  event.preventDefault();
  const $mouse = view.state.doc.resolve(eventPos.pos);
  const insertPos = slice ? dropPoint(view.state.doc, $mouse.pos, slice) : $mouse.pos;
  const tr = view.state.tr;
  tr.delete(lock.from, lock.to); // 删原选区（而非当前被篡改的 selection）
  const pos = tr.mapping.map(insertPos ?? $mouse.pos);
  const beforeInsert = tr.doc;
  tr.replaceRange(pos, pos, slice);
  if (tr.doc.eq(beforeInsert)) return true;
  const $pos = tr.doc.resolve(pos);
  let end = tr.mapping.map(insertPos ?? $mouse.pos);
  tr.mapping.maps[tr.mapping.maps.length - 1].forEach((_from, _to, _newFrom, newTo) => (end = newTo));
  tr.setSelection(new TextSelection($pos, tr.doc.resolve(end)));
  view.focus();
  view.dispatch(tr.setMeta("uiEvent", "drop"));
  unlockDragSelection(view);
  return true;
}

/**
 * 事务应用逻辑：
 *   - draglock → 用当前非空 TextSelection 记录为 lastNonEmpty + dragLock。
 *   - dragunlock → 清 dragLock（lastNonEmpty 保持锁定范围，供失焦持久化）。
 *   - clear → 清 lastNonEmpty 与 dragLock。
 *   - selectionSet 且非空 → 更新 lastNonEmpty（**拖动锁定期间忽略**，防原生拖选篡改）。
 *   - selectionSet 但 collapsed → 不覆盖记录（失焦同步场景）。
 *   - persist / 其他事务 → 保留记录。
 */
function applySelectionPersist(tr: Transaction, old: PersistState): PersistState {
  const incoming = tr.getMeta(selectionPersistKey) as PersistSignal | undefined;

  if (incoming === "clear") {
    return { lastNonEmpty: null, dragLock: null };
  }
  if (incoming === "draglock") {
    const sel = tr.selection;
    const range =
      sel instanceof TextSelection && !sel.empty ? { from: sel.from, to: sel.to } : null;
    return { lastNonEmpty: range ?? old.lastNonEmpty, dragLock: range };
  }
  if (incoming === "dragunlock") {
    return { lastNonEmpty: old.lastNonEmpty, dragLock: null };
  }

  let lastNonEmpty = old.lastNonEmpty;
  if (tr.selectionSet && incoming !== "persist" && !old.dragLock) {
    const sel = tr.selection;
    if (sel instanceof TextSelection && !sel.empty) {
      lastNonEmpty = { from: sel.from, to: sel.to };
    }
    // collapsed：保留既有记录（失焦同步场景），不覆盖。
  }
  return { lastNonEmpty, dragLock: old.dragLock };
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
          init: () => ({ lastNonEmpty: null, dragLock: null }),
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
            const s = selectionPersistKey.getState(state);
            // 拖动锁定（ED-SELECTION-DRAG-01）：锁定期间高亮稳定在原选区，不跟随原生拖选篡改。
            if (s?.dragLock) {
              return buildPersistDecorations(state.doc, s.dragLock);
            }
            const sel = state.selection;
            // 统一链路：聚焦时非空 TextSelection 也实时渲染 span（与失焦同样式）。
            if (sel instanceof TextSelection && !sel.empty) {
              return buildPersistDecorations(state.doc, { from: sel.from, to: sel.to });
            }
            // 失焦（selection 已被同步清空）：用记录的 lastNonEmpty 保持高亮。
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
