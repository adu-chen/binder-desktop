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

vi.mock("../../services/edLocate", () => ({
  triggerActiveSave: vi.fn(() => Promise.resolve({ ok: true })),
}));

vi.mock("../../services/fontService", () => ({
  listLocalFonts: vi.fn(() => Promise.resolve(["PingFang SC", "Menlo", "Songti SC"])),
  resetLocalFontCache: vi.fn(),
}));

import { listLocalFonts } from "../../services/fontService";

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
import { triggerActiveSave } from "../../services/edLocate";

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

describe("EditorToolbar · 格式工具集", () => {
  afterEach(cleanup);

  it("字体组：常用子集置顶（仅可用展示）+ 本地字体追加，写入带引号 CSS（ED-FONT-LOCAL-LIBRARY-01）", async () => {
    // document.fonts.check 桩：Arial 可用、Times New Roman 可用、本地 PingFang SC/Menlo 可用、Songti SC 不可用。
    const available = new Set(["Arial", "Times New Roman", "PingFang SC", "Menlo"]);
    Object.defineProperty(document, "fonts", {
      value: { check: vi.fn((spec: string) => [...available].some((name) => spec.includes(name))) },
      configurable: true,
    });
    const chain: Record<string, (...args: unknown[]) => unknown> = {};
    const setFontFamily = vi.fn(() => chain);
    const unsetFontFamily = vi.fn(() => chain);
    for (const name of ["focus"]) chain[name] = () => chain;
    chain.setFontFamily = setFontFamily;
    chain.unsetFontFamily = unsetFontFamily;
    chain.run = () => true;
    const editor = {
      chain: () => chain,
      isActive: () => false,
      getAttributes: () => ({}),
      can: () => ({ undo: () => true, redo: () => true }),
      on: vi.fn(),
      off: vi.fn(),
    } as unknown as Editor;
    render(<EditorToolbar editor={editor} zoom={1} onZoomIn={vi.fn()} onZoomOut={vi.fn()} onZoomReset={vi.fn()} editFormat="html" />);
    await waitFor(() => expect(listLocalFonts).toHaveBeenCalledOnce());
    const select = screen.getByLabelText("字体") as HTMLSelectElement;
    const options = Array.from(select.options).map((o) => o.value);
    // 常用子集：Arial/Times New Roman 可用 → 展示；Helvetica/Georgia/Courier New 不可用 → 隐藏；
    // 本地：PingFang SC/Menlo 可用追加，Songti SC 不可用隐藏；常用置顶。
    expect(options[1]).toBe("Arial");
    expect(options[2]).toBe("Times New Roman");
    expect(options).toContain("PingFang SC");
    expect(options).toContain("Menlo");
    expect(options).not.toContain("Songti SC");
    // 选择多词字体 → setFontFamily 收到带引号 CSS 值。
    fireEvent.change(select, { target: { value: "Times New Roman" } });
    expect(setFontFamily).toHaveBeenCalledWith("'Times New Roman'");
  });
  it("纯文本模式保留两行工具栏，保存复用活动编辑器路由且不渲染富文本或对象操作", async () => {
    const chain: Record<string, (...args: unknown[]) => unknown> = {};
    for (const name of ["focus", "undo", "redo"]) chain[name] = () => chain;
    chain.run = () => true;
    const editor = {
      chain: () => chain,
      isActive: () => false,
      getAttributes: () => ({}),
      can: () => ({ undo: () => true, redo: () => true }),
      on: vi.fn(),
      off: vi.fn(),
    } as unknown as Editor;
    const jumpToLine = vi.fn();
    const exportDocument = vi.fn(() => Promise.resolve({ ok: true, message: "已导出：note.docx" }));
    render(
      <EditorToolbar
        editor={editor}
        zoom={1}
        onZoomIn={vi.fn()}
        onZoomOut={vi.fn()}
        onZoomReset={vi.fn()}
        editFormat="text"
        onJumpToLine={jumpToLine}
        exportTargets={["docx"]}
        onExport={exportDocument}
      />,
    );
    expect(screen.getByLabelText("编辑器工具")).toBeTruthy();
    expect(screen.getByLabelText("文档工具")).toBeTruthy();
    expect(screen.queryByText(/仅提供段落级结构工具/)).toBeNull();
    expect(screen.getByTitle("引用")).toBeTruthy();
    expect(screen.getByTitle("代码块")).toBeTruthy();
    expect(screen.getByTitle("插入分隔线")).toBeTruthy();
    expect(screen.queryByTitle("无序列表")).toBeNull();
    expect(screen.queryByTitle("加粗 ⌘B")).toBeNull();
    expect(screen.queryByLabelText("插入内容")).toBeNull();
    expect(screen.getAllByLabelText("查找和替换").length).toBeGreaterThan(0);
    expect(screen.getAllByLabelText("跳转行号").length).toBeGreaterThan(0);
    expect(screen.getByTitle("导出为 DOCX")).toBeTruthy();
    fireEvent.click(screen.getByTitle("查找下一处"));
    expect(screen.getByText("请输入查找内容")).toBeTruthy();
    fireEvent.change(screen.getByLabelText("跳转行号输入"), { target: { value: "3" } });
    fireEvent.click(screen.getByTitle("跳转到行号"));
    expect(jumpToLine).toHaveBeenCalledWith(3);
    fireEvent.click(screen.getByRole("button", { name: "保存 ⌘S" }));
    await waitFor(() => expect(triggerActiveSave).toHaveBeenCalledOnce());
    expect(screen.getByText("已保存")).toBeTruthy();
    fireEvent.click(screen.getByTitle("导出为 DOCX"));
    await waitFor(() => expect(exportDocument).toHaveBeenCalledWith("docx"));
    expect(screen.getByText("已导出：note.docx")).toBeTruthy();

    cleanup();
    const transaction = { insertText: vi.fn() };
    transaction.insertText.mockImplementation(() => transaction);
    const richChain: Record<string, (...args: unknown[]) => unknown> = {};
    for (const name of ["focus", "undo", "redo", "setTextSelection", "scrollIntoView"]) richChain[name] = () => richChain;
    richChain.run = () => true;
    const dispatch = vi.fn();
    const searchableEditor = {
      chain: () => richChain,
      commands: { focus: vi.fn() },
      state: {
        selection: { from: 1, to: 1 },
        tr: transaction,
        doc: { descendants: (visit: (node: { isText: boolean; text: string }, pos: number) => boolean) => visit({ isText: true, text: "alpha alpha" }, 1) },
      },
      view: { dispatch },
      isActive: () => false,
      getAttributes: () => ({}),
      can: () => ({ undo: () => true, redo: () => true }),
      on: vi.fn(),
      off: vi.fn(),
    } as unknown as Editor;
    render(<EditorToolbar editor={searchableEditor} zoom={1} onZoomIn={vi.fn()} onZoomOut={vi.fn()} onZoomReset={vi.fn()} editFormat="text" />);
    fireEvent.change(screen.getByLabelText("查找内容"), { target: { value: "alpha" } });
    fireEvent.change(screen.getByLabelText("替换内容"), { target: { value: "beta" } });
    fireEvent.click(screen.getByTitle("替换全部匹配"));
    expect(transaction.insertText).toHaveBeenNthCalledWith(1, "beta", 7, 12);
    expect(transaction.insertText).toHaveBeenNthCalledWith(2, "beta", 1, 6);
    expect(dispatch).toHaveBeenCalledWith(transaction);
  });

  it("选中图片时不再显示无法跨 BDocx 持久化的 inline/block 切换入口", () => {
    const chain: Record<string, (...args: unknown[]) => unknown> = {};
    for (const name of [
      "focus", "undo", "redo", "setParagraph", "toggleHeading", "unsetFontSize", "setFontSize",
      "toggleBold", "toggleItalic", "toggleUnderline", "toggleStrike", "toggleSuperscript",
      "toggleSubscript", "setColor", "toggleHighlight", "toggleBulletList", "toggleOrderedList",
      "toggleBlockquote", "toggleCodeBlock", "setTextAlign", "insertTable",
    ]) chain[name] = () => chain;
    const toggleHighlight = vi.fn(() => chain);
    const toggleStrike = vi.fn(() => chain);
    chain.toggleHighlight = toggleHighlight;
    chain.toggleStrike = toggleStrike;
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
    expect(screen.getByLabelText("文字颜色")).toBeTruthy();
    fireEvent.click(screen.getByTitle("删除线"));
    expect(toggleStrike).toHaveBeenCalledOnce();
    expect(screen.getByLabelText("文字背景")).toBeTruthy();
    fireEvent.change(screen.getByLabelText("文字背景"), { target: { value: "#00ff00" } });
    expect(toggleHighlight).toHaveBeenCalledWith({ color: "#00ff00" });
    expect(screen.queryByTitle("清除高亮")).toBeNull();
    expect(screen.queryByTitle("高亮")).toBeNull();
    expect(screen.queryByLabelText("高亮底色")).toBeNull();
    expect(screen.getByTitle("上标")).toBeTruthy();
    expect(screen.getByTitle("下标")).toBeTruthy();
    expect(screen.getByTitle("插入分隔线")).toBeTruthy();
    expect(screen.getByTitle("降低列表层级")).toBeTruthy();
    expect(screen.getByTitle("提高列表层级")).toBeTruthy();
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

  it("弹出组在外部指针或焦点激活时关闭，但保持组内交互", () => {
    const chain: Record<string, (...args: unknown[]) => unknown> = {};
    for (const name of ["focus", "undo", "redo"]) chain[name] = () => chain;
    chain.run = () => true;
    const editor = {
      chain: () => chain,
      isActive: () => false,
      getAttributes: () => ({}),
      can: () => ({ undo: () => true, redo: () => true }),
      on: vi.fn(),
      off: vi.fn(),
    } as unknown as Editor;

    render(<EditorToolbar editor={editor} zoom={1} onZoomIn={vi.fn()} onZoomOut={vi.fn()} onZoomReset={vi.fn()} editFormat="text" />);

    const findMenu = () => {
      const summary = screen.getAllByLabelText("查找和替换").find((element) => element.tagName === "SUMMARY");
      if (!summary) throw new Error("未找到查找菜单开关");
      return summary.closest("details") as HTMLDetailsElement;
    };

    fireEvent.click(screen.getAllByLabelText("查找和替换").find((element) => element.tagName === "SUMMARY")!);
    expect(findMenu().open).toBe(true);

    fireEvent.pointerDown(screen.getByLabelText("查找内容"));
    expect(findMenu().open).toBe(true);

    fireEvent.pointerDown(screen.getByRole("button", { name: "保存 ⌘S" }));
    expect(findMenu().open).toBe(false);

    fireEvent.click(screen.getAllByLabelText("查找和替换").find((element) => element.tagName === "SUMMARY")!);
    expect(findMenu().open).toBe(true);
    fireEvent.focusIn(document.body);
    expect(findMenu().open).toBe(false);
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
    const setNodeMarkup = vi.fn();
    const transaction = { setNodeMarkup };
    setNodeMarkup.mockReturnValue(transaction);
    const dispatch = vi.fn();
    const imageParagraph = { attrs: { blockId: "image-block" }, type: { name: "paragraph" } };
    const editor = {
      state: {
        doc: { resolve: () => ({ parent: imageParagraph, depth: 1, before: () => 0 }) },
        tr: transaction,
      },
      view: {
        dom: viewDom,
        nodeDOM: () => imageView,
        dispatch,
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
    expect(screen.getByRole("button", { name: "编辑图片替代文本" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "恢复图片原始宽度" })).toHaveProperty("disabled", true);
    expect(screen.getByRole("button", { name: "图片左对齐" })).toHaveProperty("disabled", false);
    expect(screen.getByRole("button", { name: "图片居中" })).toHaveProperty("disabled", false);
    expect(screen.getByRole("button", { name: "图片右对齐" })).toHaveProperty("disabled", false);
    fireEvent.click(screen.getByRole("button", { name: "图片居中" }));
    expect(setNodeMarkup).toHaveBeenCalledWith(0, undefined, { blockId: "image-block", textAlign: "center" });
    expect(dispatch).toHaveBeenCalledWith(transaction);
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
    expect(screen.getByRole("button", { name: "图片居中" })).toHaveProperty("disabled", true);
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
    chain.mergeCells = () => chain;
    chain.splitCell = () => chain;
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
      can: () => ({ mergeCells: () => true, splitCell: () => true }),
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
    expect(screen.getByRole("button", { name: "上方插行" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "右侧插列" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "合并选中单元格" })).toHaveProperty("disabled", true);
    expect(screen.getByRole("button", { name: "拆分当前单元格" })).toHaveProperty("disabled", true);
    expect(screen.getByLabelText("单元格底纹颜色")).toBeTruthy();
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
