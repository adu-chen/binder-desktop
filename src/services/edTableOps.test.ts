import { describe, expect, it } from "vitest";
import { Schema } from "@tiptap/pm/model";
import { EditorState, Selection, type Transaction } from "@tiptap/pm/state";
import { TableMap, tableNodes } from "@tiptap/pm/tables";
import type { Editor } from "@tiptap/core";
import { registerEditor } from "./edLocate";
import {
  activeTableSnapshot,
  moveActiveTable,
  parseTableEditPayload,
  planTableOperations,
  projectTables,
} from "./edTableOps";

// covers: BR-ED-DATA-007
// 表格语义操作：快照以 tableId+fingerprint 定位；操作在临时 EditorState 完整规划，任一失败不改变原 doc。
const base = tableNodes({ tableGroup: "block", cellContent: "paragraph+", cellAttributes: {} });
const schema = new Schema({
  nodes: {
    doc: { content: "block+" },
    paragraph: { group: "block", content: "inline*" },
    text: { group: "inline" },
    table: { ...base.table, attrs: { ...base.table.attrs, tableId: { default: null } } },
    table_row: base.table_row,
    table_cell: { ...base.table_cell, attrs: { ...base.table_cell.attrs, backgroundColor: { default: null } } },
    table_header: { ...base.table_header, attrs: { ...base.table_header.attrs, backgroundColor: { default: null } } },
  },
  marks: { strong: {} },
});

function cell(text: string, attrs?: Record<string, unknown>) {
  return schema.nodes.table_cell.createAndFill(attrs, schema.nodes.paragraph.create(null, schema.text(text)))!;
}

function table() {
  return schema.nodes.table.create({ tableId: "table_1" }, [
    schema.nodes.table_row.create(null, [cell("A"), cell("B")]),
    schema.nodes.table_row.create(null, [cell("C"), cell("D")]),
  ]);
}

function state() {
  return EditorState.create({ schema, doc: schema.nodes.doc.create(null, [schema.nodes.paragraph.create(null, schema.text("前言")), table(), schema.nodes.paragraph.create(null, schema.text("后记"))]) });
}

describe("edTableOps · 结构快照与原子规划", () => {
  it("投影表格行列、单元格和会话锚", () => {
    const tables = projectTables(state().doc);
    expect(tables).toHaveLength(1);
    expect(tables[0]).toMatchObject({ tableId: "table_1", rows: 2, columns: 2 });
    expect(tables[0].cells.map((x) => x.text)).toEqual(["A", "B", "C", "D"]);
    expect(tables[0].cells.every((x) => x.contentKind === "plain_text")).toBe(true);
    expect(tables[0].cells.every((x) => x.backgroundColor === null)).toBe(true);
    expect(tables[0].columnWidths).toEqual([null, null]);
    expect(tables[0].capabilities).toContain("set_cell_background");
    expect(tables[0].capabilities).toContain("move_table");
    expect(tables[0].moveDirections).toEqual(["up", "down"]);
    expect(tables[0].fingerprint).toHaveLength(8);
  });

  it("合法批次在临时状态完整应用：插行、合并、单元格写入", () => {
    const beforeState = state();
    const before = projectTables(beforeState.doc)[0];
    const planned = planTableOperations(beforeState, {
      tableId: before.tableId,
      fingerprint: before.fingerprint,
      operations: [
        { op: "insert_row", at: 2 },
        { op: "merge_cells", from: { row: 1, column: 0 }, to: { row: 1, column: 1 } },
        { op: "set_cell_text", at: { row: 2, column: 0 }, text: "新增" },
      ],
    });
    expect(planned.ok).toBe(true);
    if (!planned.ok) return;
    expect(planned.after.rows).toBe(3);
    expect(planned.after.cells.some((x) => x.text === "新增")).toBe(true);
    // 规划不触碰原状态，保证后续操作失败时可整体拒绝。
    expect(projectTables(beforeState.doc)[0].rows).toBe(2);
  });

  it("指纹失配或删除唯一列均整批拒绝，原文不变", () => {
    const beforeState = state();
    const before = projectTables(beforeState.doc)[0];
    expect(planTableOperations(beforeState, { tableId: before.tableId, fingerprint: "stale", operations: [{ op: "insert_row", at: 1 }] })).toMatchObject({ ok: false, reason: "table_fingerprint_mismatch" });

    const oneColumn = EditorState.create({
      schema,
      doc: schema.nodes.doc.create(null, [schema.nodes.table.create({ tableId: "one" }, [schema.nodes.table_row.create(null, [cell("唯一")])])]),
    });
    const only = projectTables(oneColumn.doc)[0];
    expect(planTableOperations(oneColumn, { tableId: only.tableId, fingerprint: only.fingerprint, operations: [{ op: "delete_column", column: 0 }] })).toMatchObject({ ok: false, reason: "table_operation_failed:delete_column" });
    expect(projectTables(oneColumn.doc)[0].columns).toBe(1);
  });

  it("不可信载荷必须字段齐全；错误坐标和错误方向不得进入执行器", () => {
    expect(parseTableEditPayload({ tableId: "t", fingerprint: "f", operations: [{ op: "merge_cells", from: { row: 0, column: 0 } }] })).toBeUndefined();
    expect(parseTableEditPayload({ tableId: "t", fingerprint: "f", operations: [{ op: "move_table", direction: "left" }] })).toBeUndefined();
    expect(parseTableEditPayload({ tableId: "t", fingerprint: "f", operations: [{ op: "set_cell_text", at: { row: 0, column: 0 }, text: "安全" }] })).toMatchObject({ operations: [{ op: "set_cell_text" }] });
    expect(parseTableEditPayload({ tableId: "t", fingerprint: "f", operations: [{ op: "set_cell_background", at: { row: 0, column: 0 }, color: "#ffe699" }] })).toMatchObject({ operations: [{ op: "set_cell_background", color: "#FFE699" }] });
    expect(parseTableEditPayload({ tableId: "t", fingerprint: "f", operations: [{ op: "set_cell_background", at: { row: 0, column: 0 }, color: "transparent" }] })).toBeUndefined();
  });

  it("代理不得将富内容单元格退化为纯文本，且无可拆分或移动时能力会收紧", () => {
    const strong = schema.marks.strong.create();
    const rich = schema.nodes.table_cell.createAndFill(null, schema.nodes.paragraph.create(null, schema.text("粗体", [strong])))!;
    const onlyTable = schema.nodes.table.create({ tableId: "rich" }, [schema.nodes.table_row.create(null, [rich])]);
    const richState = EditorState.create({ schema, doc: schema.nodes.doc.create(null, [onlyTable]) });
    const snapshot = projectTables(richState.doc)[0];
    expect(snapshot.cells[0].contentKind).toBe("rich_content");
    expect(snapshot.capabilities).not.toContain("set_cell_text");
    expect(snapshot.capabilities).not.toContain("split_cell");
    expect(snapshot.capabilities).not.toContain("move_table");
    expect(snapshot.moveDirections).toEqual([]);
    expect(planTableOperations(richState, {
      tableId: snapshot.tableId,
      fingerprint: snapshot.fingerprint,
      operations: [{ op: "set_cell_text", at: { row: 0, column: 0 }, text: "不应覆盖" }],
    })).toMatchObject({ ok: false, reason: "table_operation_failed:set_cell_text" });
  });

  it("底纹操作只改节点属性、进入指纹，并保留富内容子树", () => {
    const strong = schema.marks.strong.create();
    const rich = schema.nodes.table_cell.createAndFill(
      null,
      schema.nodes.paragraph.create(null, schema.text("粗体", [strong])),
    )!;
    const richState = EditorState.create({
      schema,
      doc: schema.nodes.doc.create(null, [
        schema.nodes.table.create({ tableId: "shading" }, [schema.nodes.table_row.create(null, [rich])]),
      ]),
    });
    const before = projectTables(richState.doc)[0];
    const planned = planTableOperations(richState, {
      tableId: before.tableId,
      fingerprint: before.fingerprint,
      operations: [{ op: "set_cell_background", at: { row: 0, column: 0 }, color: "#ffe699" }],
    });
    expect(planned.ok).toBe(true);
    if (!planned.ok) return;
    expect(planned.after.cells[0]).toMatchObject({
      text: "粗体",
      contentKind: "rich_content",
      backgroundColor: "#FFE699",
    });
    expect(planned.after.fingerprint).not.toBe(before.fingerprint);
    const markedText = planned.state.doc.firstChild?.firstChild?.firstChild?.firstChild?.firstChild;
    expect(markedText?.marks.map((mark) => mark.type.name)).toContain("strong");
  });

  it("合并单元格只允许左上锚设置或清除底纹", () => {
    const merged = schema.nodes.table.create({ tableId: "merged" }, [
      schema.nodes.table_row.create(null, [cell("合并", { colspan: 2 })]),
    ]);
    const mergedState = EditorState.create({ schema, doc: schema.nodes.doc.create(null, [merged]) });
    const before = projectTables(mergedState.doc)[0];
    expect(planTableOperations(mergedState, {
      tableId: before.tableId,
      fingerprint: before.fingerprint,
      operations: [{ op: "set_cell_background", at: { row: 0, column: 1 }, color: "#FF0000" }],
    })).toMatchObject({ ok: false, reason: "table_operation_failed:set_cell_background" });

    const colored = planTableOperations(mergedState, {
      tableId: before.tableId,
      fingerprint: before.fingerprint,
      operations: [{ op: "set_cell_background", at: { row: 0, column: 0 }, color: "#FF0000" }],
    });
    expect(colored.ok).toBe(true);
    if (!colored.ok) return;
    expect(colored.after.cells[0].backgroundColor).toBe("#FF0000");
    const cleared = planTableOperations(colored.state, {
      tableId: colored.after.tableId,
      fingerprint: colored.after.fingerprint,
      operations: [{ op: "set_cell_background", at: { row: 0, column: 0 }, color: null }],
    });
    expect(cleared.ok && cleared.after.cells[0].backgroundColor).toBeNull();
  });

  it("顶层表格可以文档流上移，模拟结果与实际执行使用同一规划器", () => {
    const beforeState = state();
    const before = projectTables(beforeState.doc)[0];
    const planned = planTableOperations(beforeState, {
      tableId: before.tableId,
      fingerprint: before.fingerprint,
      operations: [{ op: "move_table", direction: "up" }],
    });
    expect(planned.ok).toBe(true);
    if (!planned.ok) return;
    expect(planned.state.doc.firstChild?.type.name).toBe("table");
  });

  it("人工移动后保持同一 tableId 的表格选区，邻近控件可继续操作", () => {
    let editorState = state();
    const tablePos = editorState.doc.child(0).nodeSize;
    const tableNode = editorState.doc.child(1);
    const firstCellOffset = TableMap.get(tableNode).map[0];
    editorState = editorState.apply(editorState.tr.setSelection(
      Selection.near(editorState.doc.resolve(tablePos + 1 + firstCellOffset + 1), 1),
    ));
    const fakeEditor = {
      get state() {
        return editorState;
      },
      view: {
        dispatch(transaction: Transaction) {
          editorState = editorState.apply(transaction);
        },
        focus() {},
      },
    } as unknown as Editor;
    registerEditor(fakeEditor);
    try {
      expect(activeTableSnapshot()?.tableId).toBe("table_1");
      const moved = moveActiveTable("up");
      expect(moved.ok).toBe(true);
      expect(editorState.doc.firstChild?.type.name).toBe("table");
      expect(activeTableSnapshot()?.tableId).toBe("table_1");
    } finally {
      registerEditor(null);
    }
  });
});
