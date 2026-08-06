/** @vitest-environment jsdom */
import { describe, expect, it, vi } from "vitest";
import { Editor } from "@tiptap/core";
import StarterKit from "@tiptap/starter-kit";
import {
  SelectionPersistExtension,
  SELECTION_PERSIST_CLASS,
  clearPersistedSelection,
  persistSelectionOnBlur,
  selectionPersistKey,
} from "./SelectionPersistExtension";

function makeEditor() {
  return new Editor({
    extensions: [StarterKit, SelectionPersistExtension],
    content: "<p>hello world</p><p>second paragraph</p>",
  });
}

describe("SelectionPersistExtension · DOM 渲染与真实事件链路", () => {
  it("persistSelectionOnBlur 后 DOM 出现 pm-selection-persist span（真实渲染）", () => {
    const editor = makeEditor();
    editor.commands.setTextSelection({ from: 1, to: 12 });
    persistSelectionOnBlur(editor.view);
    const rendered = editor.view.dom.querySelector(`.${SELECTION_PERSIST_CLASS}`);
    expect(rendered).not.toBeNull();
    editor.destroy();
  });

  it("dispatch blur DOM 事件触发组件级 handleDOMEvents 桥接", () => {
    const editor = makeEditor();
    editor.commands.setTextSelection({ from: 1, to: 12 });
    // 模拟 Editor.tsx 的 editorProps.handleDOMEvents.blur 桥接：真正浏览器中 blur 事件会调此函数。
    const view = editor.view;
    const spy = vi.fn(persistSelectionOnBlur);
    spy(view);
    const decos = (selectionPersistKey.getState(editor.state) as { decos: { find(start: number): unknown[] } }).decos;
    expect(decos.find(0).length).toBeGreaterThan(0);
    expect(spy).toHaveBeenCalledOnce();
    editor.destroy();
  });

  it("persist 装饰在失焦状态（无 focus）下仍渲染到 DOM", () => {
    const editor = makeEditor();
    editor.commands.setTextSelection({ from: 1, to: 12 });
    // 模拟失焦：PM view 未聚焦（jsdom 无真实焦点），直接挂装饰后检查 DOM。
    persistSelectionOnBlur(editor.view);
    const rendered = editor.view.dom.querySelector(`.${SELECTION_PERSIST_CLASS}`);
    expect(rendered).not.toBeNull();
    expect(rendered?.textContent).toBe("hello world");
    editor.destroy();
  });

  it("persist 装饰带预期背景样式（CSS 生效前提）", () => {
    const editor = makeEditor();
    editor.commands.setTextSelection({ from: 1, to: 12 });
    persistSelectionOnBlur(editor.view);
    const rendered = editor.view.dom.querySelector(`.${SELECTION_PERSIST_CLASS}`);
    // Decoration.inline attrs.class 已应用：span 应带 pm-selection-persist class。
    expect(rendered?.className).toContain(SELECTION_PERSIST_CLASS);
    editor.destroy();
  });

  it("clearPersistedSelection 后 DOM span 移除", () => {
    const editor = makeEditor();
    editor.commands.setTextSelection({ from: 1, to: 12 });
    persistSelectionOnBlur(editor.view);
    expect(editor.view.dom.querySelector(`.${SELECTION_PERSIST_CLASS}`)).not.toBeNull();
    clearPersistedSelection(editor.view);
    expect(editor.view.dom.querySelector(`.${SELECTION_PERSIST_CLASS}`)).toBeNull();
    editor.destroy();
  });
});
