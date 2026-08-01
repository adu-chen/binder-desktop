/** @vitest-environment jsdom */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Editor } from "@tiptap/react";

vi.mock("../../services/edTableOps", () => ({
  activeTableSnapshot: () => ({
    capabilities: ["set_cell_background", "delete_row", "delete_column"],
    moveDirections: ["up", "down"],
  }),
}));

import { EditorToolbar } from "./EditorToolbar";

afterEach(cleanup);

// covers: BR-ED-DATA-007
describe("EditorToolbar · TableCellShading", () => {
  it("表格上下文提供底纹颜色与清除入口，并通过 setCellAttribute 修改所选单元格", () => {
    const calls: Array<[string, ...unknown[]]> = [];
    const chain: Record<string, (...args: unknown[]) => unknown> = {};
    const fluent = (name: string) => (...args: unknown[]) => {
      calls.push([name, ...args]);
      return chain;
    };
    for (const name of [
      "focus", "setCellAttribute", "addRowBefore", "addRowAfter", "addColumnBefore", "addColumnAfter",
      "deleteRow", "deleteColumn", "toggleHeaderRow", "mergeCells", "splitCell", "deleteTable",
      "undo", "redo", "setParagraph", "toggleHeading", "unsetFontSize", "setFontSize", "toggleBold",
      "toggleItalic", "toggleUnderline", "toggleStrike", "toggleSuperscript", "toggleSubscript",
      "setColor", "toggleHighlight", "toggleBulletList", "toggleOrderedList", "toggleBlockquote",
      "toggleCodeBlock", "setTextAlign", "insertTable",
    ]) chain[name] = fluent(name);
    chain.run = () => true;

    const editor = {
      chain: () => chain,
      isActive: (name: string) => name === "table",
      // 锚点本身无底纹时，多单元格选区仍可能包含有底纹单元格；清除入口必须保持可用。
      getAttributes: (name: string) => name === "tableCell" ? { backgroundColor: null } : {},
      can: () => ({
        undo: () => true,
        redo: () => true,
        mergeCells: () => true,
        splitCell: () => true,
      }),
      on: vi.fn(),
      off: vi.fn(),
    } as unknown as Editor;

    render(
      <EditorToolbar
        editor={editor}
        zoom={1}
        onZoomIn={vi.fn()}
        onZoomOut={vi.fn()}
        onZoomReset={vi.fn()}
      />,
    );

    fireEvent.change(screen.getByLabelText("单元格底纹颜色"), { target: { value: "#ff0000" } });
    expect(calls).toContainEqual(["setCellAttribute", "backgroundColor", "#FF0000"]);

    fireEvent.click(screen.getByRole("button", { name: "清除所选单元格底纹" }));
    expect(calls).toContainEqual(["setCellAttribute", "backgroundColor", null]);
    expect(screen.queryByRole("button", { name: "表格上移一块" })).toBeNull();
    expect(screen.queryByRole("button", { name: "表格下移一块" })).toBeNull();
    expect(screen.queryByRole("button", { name: "删除表格" })).toBeNull();
  });

  it("选中图片时不再显示无法跨 BDocx 持久化的 inline/block 切换入口", () => {
    const chain: Record<string, (...args: unknown[]) => unknown> = {};
    for (const name of [
      "focus", "undo", "redo", "setParagraph", "toggleHeading", "unsetFontSize", "setFontSize",
      "toggleBold", "toggleItalic", "toggleUnderline", "toggleStrike", "toggleSuperscript",
      "toggleSubscript", "setColor", "toggleHighlight", "toggleBulletList", "toggleOrderedList",
      "toggleBlockquote", "toggleCodeBlock", "setTextAlign", "insertTable",
    ]) chain[name] = () => chain;
    chain.run = () => true;
    const editor = {
      chain: () => chain,
      isActive: (name: string) => name === "image",
      getAttributes: (name: string) => name === "image" ? { display: "block" } : {},
      can: () => ({ undo: () => true, redo: () => true }),
      on: vi.fn(),
      off: vi.fn(),
    } as unknown as Editor;

    render(
      <EditorToolbar
        editor={editor}
        zoom={1}
        onZoomIn={vi.fn()}
        onZoomOut={vi.fn()}
        onZoomReset={vi.fn()}
      />,
    );

    expect(screen.queryByTitle("切换为行内嵌入")).toBeNull();
    expect(screen.queryByTitle("切换为独占块")).toBeNull();
  });

  it("BDocx 禁止远程 URL 插图并提示使用受控本地拖入", () => {
    const chain: Record<string, (...args: unknown[]) => unknown> = {};
    for (const name of [
      "focus", "undo", "redo", "setParagraph", "toggleHeading", "unsetFontSize", "setFontSize",
      "toggleBold", "toggleItalic", "toggleUnderline", "toggleStrike", "toggleSuperscript",
      "toggleSubscript", "setColor", "toggleHighlight", "toggleBulletList", "toggleOrderedList",
      "toggleBlockquote", "toggleCodeBlock", "setTextAlign", "insertTable",
    ]) chain[name] = () => chain;
    chain.run = () => true;
    const editor = {
      chain: () => chain,
      isActive: () => false,
      getAttributes: () => ({}),
      can: () => ({ undo: () => true, redo: () => true }),
      on: vi.fn(),
      off: vi.fn(),
    } as unknown as Editor;

    render(
      <EditorToolbar
        editor={editor}
        zoom={1}
        onZoomIn={vi.fn()}
        onZoomOut={vi.fn()}
        onZoomReset={vi.fn()}
        localImageDropOnly
      />,
    );

    expect((screen.getByRole("button", {
      name: "请将本地 PNG/JPEG 图片拖入文档",
    }) as HTMLButtonElement).disabled).toBe(true);
  });
});
