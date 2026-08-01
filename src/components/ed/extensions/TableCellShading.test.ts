/** @vitest-environment jsdom */
import { describe, expect, it } from "vitest";
import { Editor } from "@tiptap/core";
import StarterKit from "@tiptap/starter-kit";
import { Table } from "@tiptap/extension-table";
import { TableRow } from "@tiptap/extension-table-row";
import { TableHeader } from "@tiptap/extension-table-header";
import { TableCell } from "@tiptap/extension-table-cell";
import {
  TableCellShading,
  normalizeTableCellShading,
  parseTableCellShading,
  renderTableCellShading,
} from "./TableCellShading";

// covers: BR-ED-DATA-007
describe("TableCellShading", () => {
  it("只接受不透明六位十六进制或等价 rgb，并统一为大写", () => {
    expect(normalizeTableCellShading("#ffe699")).toBe("#FFE699");
    expect(normalizeTableCellShading("rgb(255, 230, 153)")).toBe("#FFE699");
    expect(normalizeTableCellShading("rgba(255, 230, 153, 1)")).toBe("#FFE699");
    expect(normalizeTableCellShading("#abc")).toBeNull();
    expect(normalizeTableCellShading("rgba(255, 230, 153, 0.5)")).toBeNull();
    expect(normalizeTableCellShading("transparent")).toBeNull();
  });

  it("优先读取稳定 data 属性并同时渲染 data 与 CSS", () => {
    const cell = document.createElement("td");
    cell.setAttribute("data-binder-cell-shading", "#ff0000");
    cell.style.backgroundColor = "rgb(0, 0, 255)";
    expect(parseTableCellShading(cell)).toBe("#FF0000");
    expect(renderTableCellShading("#ffe699")).toEqual({
      "data-binder-cell-shading": "#FFE699",
      style: "background-color: #FFE699",
    });
    expect(renderTableCellShading(null)).toEqual({});
  });

  it("在真实 TipTap schema 中往返 td/th 属性", () => {
    const editor = new Editor({
      extensions: [StarterKit, Table, TableRow, TableHeader, TableCell, TableCellShading],
      content: '<table><tbody><tr><td data-binder-cell-shading="#ffe699"><p>内容</p></td></tr></tbody></table>',
    });
    const cell = editor.state.doc.firstChild?.firstChild?.firstChild;
    expect(cell?.type.name).toBe("tableCell");
    expect(cell?.attrs.backgroundColor).toBe("#FFE699");
    expect(editor.getHTML()).toContain('data-binder-cell-shading="#FFE699"');
    expect(editor.getHTML()).toContain("background-color:");
    editor.destroy();
  });
});
