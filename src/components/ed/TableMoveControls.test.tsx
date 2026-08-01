/** @vitest-environment jsdom */
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Editor } from "@tiptap/react";

vi.mock("../../services/edTableOps", () => ({
  activeTableSnapshot: vi.fn(() => ({
    tableId: "table_1",
    moveDirections: ["up"],
  })),
  moveActiveTable: vi.fn(() => ({ ok: true })),
}));

import { activeTableSnapshot, moveActiveTable } from "../../services/edTableOps";
import { TableMoveControls } from "./TableMoveControls";

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
