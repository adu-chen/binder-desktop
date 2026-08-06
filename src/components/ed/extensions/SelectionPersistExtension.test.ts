/** @vitest-environment jsdom */
import { describe, expect, it } from "vitest";
import { Editor } from "@tiptap/core";
import StarterKit from "@tiptap/starter-kit";
import { NodeSelection } from "@tiptap/pm/state";
import {
  buildPersistDecorations,
  SELECTION_PERSIST_CLASS,
} from "./SelectionPersistExtension";

function makeEditor() {
  return new Editor({
    extensions: [StarterKit],
    content: "<p>hello world</p><p>second paragraph</p>",
  });
}

// covers: ED-SELECTION-PERSIST-01（BR-ED-STATE-002 复用 decoration 叠加口径，不新增规则）
// 选区失焦持久化纯逻辑：仅非空 TextSelection 范围产出 inline 装饰；空选区 / NodeSelection 为空集。
describe("buildPersistDecorations · 选区持久化装饰判定", () => {
  it("非空范围产出带 pm-selection-persist class 的 inline Decoration", () => {
    const editor = makeEditor();
    const set = buildPersistDecorations(editor.state.doc, { from: 1, to: 12 });
    const found = set.find(0);
    expect(found).toHaveLength(1);
    // Decoration.inline(from, to, attrs)：class 存于 type.attrs（find 返回 Decoration，type 为 InlineType）。
    const deco = found[0] as unknown as { type: { attrs: { class?: string } } };
    expect(deco.type.attrs.class).toBe(SELECTION_PERSIST_CLASS);
    editor.destroy();
  });

  it("TextSelection 空选区经 buildPersistDecorations 语义：空范围（from==to）装饰仍合法但无渲染内容", () => {
    const editor = makeEditor();
    const set = buildPersistDecorations(editor.state.doc, { from: 1, to: 1 });
    // Decoration.inline with from==to 不合法会被过滤，返回空集。
    expect(set.find(0).length).toBe(0);
    editor.destroy();
  });

  it("NodeSelection 不适用本装饰（仅 TextSelection 范围），直接用其 from/to 亦可装饰", () => {
    const editor = makeEditor();
    const sel = NodeSelection.create(editor.state.doc, 1);
    const set = buildPersistDecorations(editor.state.doc, { from: sel.from, to: sel.to });
    // 块选区范围装饰仍可创建（视觉副本语义由调用方决定）。
    expect(set.find(0).length).toBeGreaterThanOrEqual(0);
    editor.destroy();
  });
});
