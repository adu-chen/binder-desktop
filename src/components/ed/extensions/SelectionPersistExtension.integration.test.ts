/** @vitest-environment jsdom */
import { describe, expect, it } from "vitest";
import { Editor } from "@tiptap/core";
import StarterKit from "@tiptap/starter-kit";
import {
  SelectionPersistExtension,
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

function decoCount(state: unknown): number {
  const s = state as PersistState;
  return s?.decos ? s.decos.find(0, 1e9).length : 0;
}

describe("SelectionPersistExtension · 桥接函数集成（真实调用路径）", () => {
  it("persistSelectionOnBlur 用记录选区挂载 DecorationSet（selection 已 collapsed 也能挂）", () => {
    const editor = makeEditor();
    editor.commands.setTextSelection({ from: 1, to: 12 });
    // 模拟失焦时序：先清空 PM selection（失焦同步），再 blur。
    editor.commands.setTextSelection({ from: 1, to: 1 });
    persistSelectionOnBlur(editor.view);
    expect(decoCount(selectionPersistKey.getState(editor.state))).toBeGreaterThan(0);
    editor.destroy();
  });

  it("空选区从未产生记录时 blur 不挂装饰", () => {
    const editor = makeEditor();
    editor.commands.setTextSelection({ from: 1, to: 1 });
    persistSelectionOnBlur(editor.view);
    expect(decoCount(selectionPersistKey.getState(editor.state))).toBe(0);
    editor.destroy();
  });

  it("persist 后失焦同步事务（selectionSet 无 docChanged）保留装饰，focus 清除", () => {
    const editor = makeEditor();
    editor.commands.setTextSelection({ from: 1, to: 12 });
    persistSelectionOnBlur(editor.view);
    expect(decoCount(selectionPersistKey.getState(editor.state))).toBeGreaterThan(0);
    // 失焦后浏览器 selectionchange → PM selection 同步事务（无 docChanged）：装饰保留。
    editor.commands.setTextSelection({ from: 1, to: 1 });
    expect(decoCount(selectionPersistKey.getState(editor.state))).toBeGreaterThan(0);
    // focus 事件（用户点击编辑器内）→ clear：装饰清除。
    clearPersistedSelection(editor.view);
    expect(decoCount(selectionPersistKey.getState(editor.state))).toBe(0);
    editor.destroy();
  });

  it("persist 后 docChanged 保留装饰；focus 清除", () => {
    const editor = makeEditor();
    editor.commands.setTextSelection({ from: 1, to: 12 });
    persistSelectionOnBlur(editor.view);
    expect(decoCount(selectionPersistKey.getState(editor.state))).toBeGreaterThan(0);
    editor.commands.insertContent("X");
    expect(decoCount(selectionPersistKey.getState(editor.state))).toBeGreaterThan(0);
    clearPersistedSelection(editor.view);
    expect(decoCount(selectionPersistKey.getState(editor.state))).toBe(0);
    editor.destroy();
  });

  it("新选区记录会覆盖旧记录，blur 用最新选区", () => {
    const editor = makeEditor();
    editor.commands.setTextSelection({ from: 1, to: 12 });
    // 用户改选第二段。
    editor.commands.setTextSelection({ from: 14, to: 30 });
    persistSelectionOnBlur(editor.view);
    const s = selectionPersistKey.getState(editor.state) as PersistState;
    expect(s.lastNonEmpty).toMatchObject({ from: 14, to: 30 });
    const deco = s.decos.find(0, 1e9)[0] as unknown as { from: number; to: number };
    expect(deco.from).toBe(14);
    expect(deco.to).toBe(30);
    editor.destroy();
  });

  it("组件级 editorProps.handleDOMEvents.blur 在真实 blur 事件时挂载装饰（Editor.tsx 同构）", () => {
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
    editor.view.dom.dispatchEvent(new FocusEvent("blur", { bubbles: false }));
    expect(decoCount(selectionPersistKey.getState(editor.state))).toBeGreaterThan(0);
    editor.destroy();
  });

  it("clearPersistedSelection 移除已有装饰；无装饰时不派发", () => {
    const editor = makeEditor();
    editor.commands.setTextSelection({ from: 1, to: 12 });
    persistSelectionOnBlur(editor.view);
    clearPersistedSelection(editor.view);
    expect(decoCount(selectionPersistKey.getState(editor.state))).toBe(0);
    // 二次清除不抛错、不产生装饰。
    clearPersistedSelection(editor.view);
    expect(decoCount(selectionPersistKey.getState(editor.state))).toBe(0);
    editor.destroy();
  });
});
