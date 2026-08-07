/** @vitest-environment jsdom */
import { describe, expect, it } from "vitest";
import { Editor } from "@tiptap/core";
import StarterKit from "@tiptap/starter-kit";
import {
  SelectionPersistExtension,
  SELECTION_PERSIST_CLASS,
  clearPersistedSelection,
  persistSelectionOnBlur,
  selectionPersistKey,
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
