/** @vitest-environment jsdom */
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Editor } from "@tiptap/react";

vi.mock("../../services/edTableOps", () => ({
  activeTableSnapshot: vi.fn(() => ({
    tableId: "table_1",
    capabilities: ["set_cell_background", "delete_row", "delete_column"],
    moveDirections: ["up"],
  })),
  moveActiveTable: vi.fn(() => ({ ok: true })),
}));

vi.mock("../../services/edImageOps", () => ({
  activeImageSnapshot: vi.fn(() => ({
    blockId: "image-block",
    imagePos: 12,
    standalone: true,
    moveDirections: ["up"],
  })),
  moveActiveImage: vi.fn(() => true),
  deleteActiveImage: vi.fn(() => true),
}));

import { EditorStatusBar } from "./EditorStatusBar";
import { EditorToolbar } from "./EditorToolbar";
import {
  activeImageSnapshot,
  deleteActiveImage,
  moveActiveImage,
} from "../../services/edImageOps";
import { ImageMoveControls } from "./ImageMoveControls";
import { activeTableSnapshot, moveActiveTable } from "../../services/edTableOps";
import { TableMoveControls } from "./TableMoveControls";

// Migrated intact from src/components/ed/EditorStatusBar.test.tsx
{
const editor = {
  storage: { characterCount: { characters: () => 2, words: () => 1 } },
  state: {
    selection: { from: 1, to: 1, head: 1 },
    doc: {
      textContent: "红字",
      content: { size: 2 },
      textBetween: () => "红字",
    },
  },
  on: () => undefined,
  off: () => undefined,
} as never;

describe("EditorStatusBar · DOCX 样式风险告知", () => {
  afterEach(cleanup);

  it("有转换风险时显示可访问的样式提示", () => {
    render(<EditorStatusBar editor={editor} tabState="saved" converting={false} styleNotices={["颜色片段未能恢复"]} />);
    expect(screen.getByLabelText("样式提示：颜色片段未能恢复").textContent).toContain("样式提示：1");
  });

  it("无风险时不显示样式提示", () => {
    render(<EditorStatusBar editor={editor} tabState="saved" converting={false} />);
    expect(screen.queryByText(/样式提示：/)).toBeNull();
  });
});
}
// Migrated intact from src/components/ed/EditorToolbar.test.tsx
{
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
}

// Migrated intact from src/components/ed/ImageMoveControls.test.tsx
{
afterEach(() => {
  cleanup();
  document.body.innerHTML = "";
  vi.clearAllMocks();
});

// covers: BR-ED-DATA-001
// covers: BR-ED-DATA-003
describe("ImageMoveControls", () => {
  it("控件直接挂到选中图片 NodeView 上方，遵循可移动方向且删除走受控操作", () => {
    const viewDom = document.createElement("div");
    const imageView = document.createElement("span");
    imageView.className = "ed-img-view selected";
    viewDom.append(imageView);
    document.body.append(viewDom);
    const editor = {
      view: {
        dom: viewDom,
        nodeDOM: () => imageView,
      },
      on: vi.fn(),
      off: vi.fn(),
    } as unknown as Editor;

    render(<ImageMoveControls editor={editor} />);

    const group = screen.getByRole("group", { name: "图片操作" });
    expect(group.parentElement).toBe(imageView);
    expect(group.style.top).toBe("");
    expect(group.style.left).toBe("");
    expect(group.getAttribute("data-block-id")).toBe("image-block");
    expect(group.getAttribute("data-standalone")).toBe("true");
    expect(screen.getByRole("button", { name: "图片上移一块" })).toHaveProperty("disabled", false);
    expect(screen.getByRole("button", { name: "图片下移一块" })).toHaveProperty("disabled", true);

    const mouseDown = new MouseEvent("mousedown", { bubbles: true, cancelable: true });
    group.dispatchEvent(mouseDown);
    expect(mouseDown.defaultPrevented).toBe(true);

    fireEvent.click(screen.getByRole("button", { name: "图片上移一块" }));
    expect(moveActiveImage).toHaveBeenCalledWith(editor, "up");

    fireEvent.click(screen.getByRole("button", { name: "删除图片" }));
    expect(deleteActiveImage).toHaveBeenCalledWith(editor);
    expect(screen.queryByRole("group", { name: "图片操作" })).toBeNull();
  });

  it("混排图片仍提供删除，但两个块移动方向都禁用", () => {
    vi.mocked(activeImageSnapshot).mockReturnValue({
      blockId: "mixed",
      imagePos: 8,
      standalone: false,
      moveDirections: [],
    });
    const viewDom = document.createElement("div");
    const imageView = document.createElement("span");
    imageView.className = "ed-img-view selected";
    viewDom.append(imageView);
    document.body.append(viewDom);
    const editor = {
      view: { dom: viewDom, nodeDOM: () => imageView },
      on: vi.fn(),
      off: vi.fn(),
    } as unknown as Editor;

    render(<ImageMoveControls editor={editor} />);

    expect(screen.getByRole("button", { name: "图片上移一块" })).toHaveProperty("disabled", true);
    expect(screen.getByRole("button", { name: "图片下移一块" })).toHaveProperty("disabled", true);
    expect(screen.getByRole("button", { name: "删除图片" })).toHaveProperty("disabled", false);
  });
});
}

// Migrated intact from src/components/ed/TableMoveControls.test.tsx
{
afterEach(() => {
  cleanup();
  document.body.innerHTML = "";
  vi.clearAllMocks();
});

// covers: BR-ED-DATA-007
describe("TableMoveControls", () => {
  it("控件直接挂到活动 TableView wrapper，遵循可移动方向且 mousedown 不丢失选区", () => {
    const viewDom = document.createElement("div");
    const wrapper = document.createElement("div");
    wrapper.className = "tableWrapper";
    const table = document.createElement("table");
    const cellText = document.createTextNode("A");
    const paragraph = document.createElement("p");
    const cell = document.createElement("td");
    const row = document.createElement("tr");
    const body = document.createElement("tbody");
    paragraph.append(cellText);
    cell.append(paragraph);
    row.append(cell);
    body.append(row);
    table.append(body);
    wrapper.append(table);
    viewDom.append(wrapper);
    document.body.append(viewDom);
    const run = vi.fn(() => true);
    const chain: Record<string, (...args: unknown[]) => unknown> = {};
    chain.focus = () => chain;
    chain.deleteTable = () => chain;
    chain.run = run;
    const editor = {
      state: {
        selection: {
          from: 12,
          $from: {
            depth: 3,
            node: (depth: number) => ({ type: { name: depth === 2 ? "table" : "paragraph" } }),
            before: () => 10,
          },
        },
      },
      view: {
        dom: viewDom,
        // 真实 domAtPos 通常返回单元格段落内的 Text；控件须向上识别 TableView 的 table。
        domAtPos: () => ({ node: cellText, offset: 0 }),
        nodeDOM: () => wrapper,
      },
      chain: () => chain,
      on: vi.fn(),
      off: vi.fn(),
    } as unknown as Editor;

    render(
      <TableMoveControls
        editor={editor}
      />,
    );

    const group = screen.getByRole("group", { name: "表格位置移动" });
    expect(group.parentElement).toBe(wrapper);
    expect(group.style.top).toBe("");
    expect(group.style.left).toBe("");
    const up = screen.getByRole("button", { name: "表格上移一块" }) as HTMLButtonElement;
    const down = screen.getByRole("button", { name: "表格下移一块" }) as HTMLButtonElement;
    expect(up.disabled).toBe(false);
    expect(down.disabled).toBe(true);

    const mouseDown = new MouseEvent("mousedown", { bubbles: true, cancelable: true });
    group.dispatchEvent(mouseDown);
    expect(mouseDown.defaultPrevented).toBe(true);

    fireEvent.click(up);
    expect(moveActiveTable).toHaveBeenCalledWith("up");

    fireEvent.click(screen.getByRole("button", { name: "删除表格" }));
    expect(run).toHaveBeenCalledOnce();
    expect(screen.queryByRole("group", { name: "表格位置移动" })).toBeNull();
  });

  it("tableId 补发前仍显示删除入口，移动方向暂时禁用", () => {
    vi.mocked(activeTableSnapshot).mockReturnValueOnce(null);
    const viewDom = document.createElement("div");
    const wrapper = document.createElement("div");
    wrapper.className = "tableWrapper";
    const table = document.createElement("table");
    wrapper.append(table);
    viewDom.append(wrapper);
    document.body.append(viewDom);
    const editor = {
      state: { selection: { from: 1, $from: { depth: 0 } } },
      view: {
        dom: viewDom,
        domAtPos: () => ({ node: table, offset: 0 }),
        nodeDOM: () => table,
      },
      chain: vi.fn(),
      on: vi.fn(),
      off: vi.fn(),
    } as unknown as Editor;

    render(
      <TableMoveControls
        editor={editor}
      />,
    );

    expect(screen.getByRole("button", { name: "表格上移一块" })).toHaveProperty("disabled", true);
    expect(screen.getByRole("button", { name: "表格下移一块" })).toHaveProperty("disabled", true);
    expect(screen.getByRole("button", { name: "删除表格" })).toBeTruthy();
  });

  it("相同 ProseMirror 选区未更新时，pointerup 仍可从真实 DOM 唤起控件", async () => {
    const viewDom = document.createElement("div");
    const outside = document.createElement("p");
    const wrapper = document.createElement("div");
    wrapper.className = "tableWrapper";
    const table = document.createElement("table");
    wrapper.append(table);
    viewDom.append(outside, wrapper);
    document.body.append(viewDom);
    let pointerInsideTable = false;
    const editor = {
      state: { selection: { from: 1, $from: { depth: 0 } } },
      view: {
        dom: viewDom,
        domAtPos: () => ({ node: pointerInsideTable ? table : outside, offset: 0 }),
        nodeDOM: () => null,
      },
      chain: vi.fn(),
      on: vi.fn(),
      off: vi.fn(),
    } as unknown as Editor;

    render(
      <TableMoveControls
        editor={editor}
      />,
    );
    expect(screen.queryByRole("group", { name: "表格位置移动" })).toBeNull();

    pointerInsideTable = true;
    fireEvent.pointerUp(viewDom);

    await waitFor(() => {
      expect(screen.getByRole("group", { name: "表格位置移动" }).parentElement).toBe(wrapper);
    });
  });
});
}
