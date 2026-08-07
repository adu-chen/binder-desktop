/** @vitest-environment jsdom */
import { describe, expect, it } from "vitest";
import { Editor } from "@tiptap/core";
import StarterKit from "@tiptap/starter-kit";
import {
  SelectionPersistExtension,
  SELECTION_PERSIST_CLASS,
  clearPersistedSelection,
  handleSelectionDragDrop,
  lockDragSelection,
  persistSelectionOnBlur,
  selectionPersistKey,
  unlockDragSelection,
  type PersistState,
} from "./SelectionPersistExtension";

function makeEditor() {
  return new Editor({
    extensions: [StarterKit, SelectionPersistExtension],
    content: "<p>hello world</p><p>second paragraph</p>",
  });
}

/** 统一链路 v2：装饰由 decorations 实时派生，断言依据 DOM 中的 span。 */
function domSpanCount(editor: Editor): number {
  return editor.view.dom.querySelectorAll(`.${SELECTION_PERSIST_CLASS}`).length;
}

describe("SelectionPersistExtension · 统一链路 v2（聚焦/失焦同一 decoration 渲染）", () => {
  it("聚焦非空选区：实时渲染 span（与失焦同样式）", () => {
    const editor = makeEditor();
    editor.commands.setTextSelection({ from: 1, to: 12 });
    expect(domSpanCount(editor)).toBeGreaterThan(0);
    editor.destroy();
  });

  it("失焦（selection 被同步清空 + blur 触发重算）：用 lastNonEmpty 保留 span", () => {
    const editor = makeEditor();
    editor.commands.setTextSelection({ from: 1, to: 12 });
    // 失焦同步：selection 变 collapsed（不覆盖记录）。
    editor.commands.setTextSelection({ from: 1, to: 1 });
    const s = selectionPersistKey.getState(editor.state) as PersistState;
    expect(s.lastNonEmpty).toMatchObject({ from: 1, to: 12 });
    // blur 派发 persist 事务触发 decorations 重算（view.focused=false → 用记录渲染）。
    persistSelectionOnBlur(editor.view);
    expect(domSpanCount(editor)).toBeGreaterThan(0);
    editor.destroy();
  });

  it("用户点击编辑器内（focus + collapsed）：清记录恢复原生，无 span", () => {
    const editor = makeEditor();
    editor.commands.setTextSelection({ from: 1, to: 12 });
    expect(domSpanCount(editor)).toBeGreaterThan(0);
    clearPersistedSelection(editor.view);
    editor.commands.setTextSelection({ from: 1, to: 1 });
    expect(domSpanCount(editor)).toBe(0);
    editor.destroy();
  });

  it("空选区从未产生记录时 blur 不渲染 span", () => {
    const editor = makeEditor();
    editor.commands.setTextSelection({ from: 1, to: 1 });
    persistSelectionOnBlur(editor.view);
    expect(domSpanCount(editor)).toBe(0);
    editor.destroy();
  });

  it("新选区记录覆盖旧记录，blur 用最新选区", () => {
    const editor = makeEditor();
    editor.commands.setTextSelection({ from: 1, to: 12 });
    editor.commands.setTextSelection({ from: 14, to: 30 });
    const s = selectionPersistKey.getState(editor.state) as PersistState;
    expect(s.lastNonEmpty).toMatchObject({ from: 14, to: 30 });
    editor.destroy();
  });

  it("切换文档（setContent 新内容）前 clear 记录：lastNonEmpty 清零，旧选区不残留新 doc", () => {
    const editor = makeEditor();
    editor.commands.setTextSelection({ from: 1, to: 12 });
    expect((selectionPersistKey.getState(editor.state) as PersistState).lastNonEmpty).not.toBeNull();
    // 模拟 Editor.tsx 切换文档：clearPersistedSelection 后 setContent 新文档。
    clearPersistedSelection(editor.view);
    editor.commands.setContent("<p>new doc content</p>", { emitUpdate: false });
    const s = selectionPersistKey.getState(editor.state) as PersistState;
    expect(s.lastNonEmpty).toBeNull();
    // 失焦回退也不再渲染旧选区（记录已清）。
    persistSelectionOnBlur(editor.view);
    expect(domSpanCount(editor)).toBe(0);
    editor.destroy();
  });

  it("draglock：锁定原选区，原生拖选篡改 selection 不更新 lastNonEmpty，decorations 用锁范围", () => {
    const editor = makeEditor();
    // 原选区 1-12（hello world）
    editor.commands.setTextSelection({ from: 1, to: 12 });
    lockDragSelection(editor.view);
    let s = selectionPersistKey.getState(editor.state) as PersistState;
    expect(s.dragLock).toMatchObject({ from: 1, to: 12 });
    expect(s.lastNonEmpty).toMatchObject({ from: 1, to: 12 });
    // 模拟原生拖选篡改：selection 从 6 重新拖出 6-9（锁定期间必须忽略）。
    editor.commands.setTextSelection({ from: 6, to: 9 });
    s = selectionPersistKey.getState(editor.state) as PersistState;
    expect(s.lastNonEmpty).toMatchObject({ from: 1, to: 12 });
    expect(s.dragLock).toMatchObject({ from: 1, to: 12 });
    // decorations 用锁定范围渲染（而非篡改后的 6-9）。
    expect(domSpanCount(editor)).toBeGreaterThan(0);
    editor.destroy();
  });

  it("dragunlock：释放锁后 lastNonEmpty 保持锁定范围，selection 篡改恢复跟随", () => {
    const editor = makeEditor();
    editor.commands.setTextSelection({ from: 1, to: 12 });
    lockDragSelection(editor.view);
    // 拖出场景：编辑器无 drop → dragend 解锁，lastNonEmpty 仍为原选区（持久化高亮正确）。
    unlockDragSelection(editor.view);
    let s = selectionPersistKey.getState(editor.state) as PersistState;
    expect(s.dragLock).toBeNull();
    expect(s.lastNonEmpty).toMatchObject({ from: 1, to: 12 });
    // 解锁后 selection 篡改恢复跟随。
    editor.commands.setTextSelection({ from: 14, to: 30 });
    s = selectionPersistKey.getState(editor.state) as PersistState;
    expect(s.lastNonEmpty).toMatchObject({ from: 14, to: 30 });
    editor.destroy();
  });

  it("clear（用户点击编辑器内）同时清 dragLock 与 lastNonEmpty", () => {
    const editor = makeEditor();
    editor.commands.setTextSelection({ from: 1, to: 12 });
    lockDragSelection(editor.view);
    clearPersistedSelection(editor.view);
    const s = selectionPersistKey.getState(editor.state) as PersistState;
    expect(s.dragLock).toBeNull();
    expect(s.lastNonEmpty).toBeNull();
    editor.destroy();
  });

  it("handleSelectionDragDrop（move）：删锁定范围而非被篡改的当前 selection，插入 drop 位置，解锁", () => {
    const editor = new Editor({
      extensions: [StarterKit, SelectionPersistExtension],
      content: "<p>hello world</p><p>second paragraph</p>",
      editorProps: {
        handleDrop: (view, event, slice, move) => handleSelectionDragDrop(view, event, slice, move),
      },
    });
    // 原选区 1-12；dragstart 锁定。
    editor.commands.setTextSelection({ from: 1, to: 12 });
    lockDragSelection(editor.view);
    // 模拟原生拖选篡改：selection 变 6-9（锁定期间忽略，但 drop 必须删锁定范围 1-12）。
    editor.commands.setTextSelection({ from: 6, to: 9 });
    const docBefore = editor.state.doc;
    // jsdom 无 elementFromPoint，stub posAtCoords 返回第二段内位置（原 pos≈14，删 1-12 后映射）。
    const realPosAtCoords = editor.view.posAtCoords.bind(editor.view);
    editor.view.posAtCoords = () => ({ pos: 20, inside: -1 });
    // 构造 move 型 drop：slice=原选区内容（"hello world"），drop 到第二段头部（pos≈14）。
    const mockEvent = { clientX: 0, clientY: 0, preventDefault: () => {} } as unknown as DragEvent;
    const handled = handleSelectionDragDrop(editor.view, mockEvent, editor.state.doc.slice(1, 12), true);
    editor.view.posAtCoords = realPosAtCoords;
    expect(handled).toBe(true);
    // 锁定范围（1-12）被删除（hello world 移走）→ doc 变化。
    expect(editor.state.doc).not.toBe(docBefore);
    // drop 后锁已释放。
    const s = selectionPersistKey.getState(editor.state) as PersistState;
    expect(s.dragLock).toBeNull();
    editor.destroy();
  });

  it("handleSelectionDragDrop（copy，move=false）：不接管返回 false（交回 PM 原生复制）", () => {
    const editor = new Editor({
      extensions: [StarterKit, SelectionPersistExtension],
      content: "<p>hello world</p><p>second paragraph</p>",
      editorProps: {
        handleDrop: (view, event, slice, move) => handleSelectionDragDrop(view, event, slice, move),
      },
    });
    editor.commands.setTextSelection({ from: 1, to: 12 });
    lockDragSelection(editor.view);
    const mockEvent = { clientX: 0, clientY: 0, preventDefault: () => {} } as unknown as DragEvent;
    const handled = handleSelectionDragDrop(editor.view, mockEvent, editor.state.doc.slice(1, 12), false);
    expect(handled).toBe(false);
    // copy 语义：不删原选区，锁保持（由 dragend 释放）。
    const s = selectionPersistKey.getState(editor.state) as PersistState;
    expect(s.dragLock).not.toBeNull();
    expect(editor.state.doc.textBetween(1, 12)).toBe("hello world");
    editor.destroy();
  });

  it("组件级 editorProps.handleDOMEvents 桥接（Editor.tsx 同构）：blur 挂载、focus 清除", () => {
    const editor = new Editor({
      extensions: [StarterKit, SelectionPersistExtension],
      content: "<p>hello world</p><p>second paragraph</p>",
      editorProps: {
        handleDOMEvents: {
          blur: (view) => {
            persistSelectionOnBlur(view);
            return false;
          },
          focus: (view) => {
            clearPersistedSelection(view);
            return false;
          },
        },
      },
    });
    editor.commands.setTextSelection({ from: 1, to: 12 });
    expect(domSpanCount(editor)).toBeGreaterThan(0);
    // blur 事件：仍应保留（lastNonEmpty + view.focused=false 渲染）。
    editor.view.dom.dispatchEvent(new FocusEvent("blur", { bubbles: false }));
    expect(domSpanCount(editor)).toBeGreaterThan(0);
    // 用户点击编辑器空白处：focus → clear 记录 + selection collapsed → span 移除。
    clearPersistedSelection(editor.view);
    editor.commands.setTextSelection({ from: 1, to: 1 });
    expect(domSpanCount(editor)).toBe(0);
    editor.destroy();
  });
});
