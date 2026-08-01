import { Fragment, Slice, type Node as PMNode } from "@tiptap/pm/model";
import { CellSelection, TableMap, addColumnAfter, addColumnBefore, addRowAfter, addRowBefore, deleteColumn, deleteRow, mergeCells, splitCell, toggleHeaderRow } from "@tiptap/pm/tables";
import { Selection, type EditorState, type Transaction } from "@tiptap/pm/state";
import { getActiveEditor } from "./edLocate";
import { normalizeTableCellShading } from "../components/ed/extensions/TableCellShading";

/** 表格单元格的规范化只读投影；坐标为 0 起，跨度按 PM 表格模型给出。 */
export interface TableCellSnapshot {
  row: number;
  column: number;
  rowspan: number;
  colspan: number;
  text: string;
  /** TableCellShading 的显式不透明纯色；null 表示无直接底纹。 */
  backgroundColor: string | null;
  /** 代理仅可写 plain_text 单元格；富内容必须保留给人工编辑，不能被纯文本替换破坏。 */
  contentKind: "plain_text" | "rich_content";
}

/** 代理可读、Diff 可展示的表格结构快照。 */
export interface TableSnapshot {
  tableId: string;
  ordinal: number;
  rows: number;
  columns: number;
  headerRow: boolean;
  /** ProseMirror 当前会话列宽（px）；仅供锚校验与交互反馈，不承诺 DOCX 往返保真。 */
  columnWidths: Array<number | null>;
  /** 当前文档流中真正可执行的移动方向；不能以泛化 move_table 能力猜测边界。 */
  moveDirections: Array<"up" | "down">;
  fingerprint: string;
  cells: TableCellSnapshot[];
  capabilities: TableCapability[];
}

export type TableCapability =
  | "insert_row" | "delete_row" | "insert_column" | "delete_column"
  | "set_header_row" | "merge_cells" | "split_cell" | "set_cell_text"
  | "set_cell_background" | "move_table";

export type TableOperation =
  | { op: "insert_row"; at: number }
  | { op: "delete_row"; row: number }
  | { op: "insert_column"; at: number }
  | { op: "delete_column"; column: number }
  | { op: "set_header_row"; enabled: boolean }
  | { op: "merge_cells"; from: { row: number; column: number }; to: { row: number; column: number } }
  | { op: "split_cell"; at: { row: number; column: number } }
  | { op: "set_cell_text"; at: { row: number; column: number }; text: string }
  | { op: "set_cell_background"; at: { row: number; column: number }; color: string | null }
  | { op: "move_table"; direction: "up" | "down" };

export interface TableEditPayload {
  tableId: string;
  fingerprint: string;
  operations: TableOperation[];
}

export type TableOpResult =
  | { ok: true; before: TableSnapshot; after: TableSnapshot; state: EditorState }
  | { ok: false; reason: string };

type TableCoordinate = { row: number; column: number };

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value && typeof value === "object" && !Array.isArray(value));
}

function isIndex(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= 0;
}

function parseCoordinate(value: unknown): TableCoordinate | null {
  return isRecord(value) && isIndex(value.row) && isIndex(value.column)
    ? { row: value.row, column: value.column }
    : null;
}

/**
 * 将不可信 JSON 收紧为 TableOperation。前端在执行前再校验一次，避免历史 Diff、模型输出或 IPC 载荷
 * 因缺字段在执行器内抛异常。
 */
export function parseTableOperation(value: unknown): TableOperation | null {
  if (!isRecord(value) || typeof value.op !== "string" || value.html !== undefined || value.content !== undefined) return null;
  switch (value.op) {
    case "insert_row": return isIndex(value.at) ? { op: "insert_row", at: value.at } : null;
    case "delete_row": return isIndex(value.row) ? { op: "delete_row", row: value.row } : null;
    case "insert_column": return isIndex(value.at) ? { op: "insert_column", at: value.at } : null;
    case "delete_column": return isIndex(value.column) ? { op: "delete_column", column: value.column } : null;
    case "set_header_row": return typeof value.enabled === "boolean" ? { op: "set_header_row", enabled: value.enabled } : null;
    case "merge_cells": {
      const from = parseCoordinate(value.from);
      const to = parseCoordinate(value.to);
      return from && to && (from.row !== to.row || from.column !== to.column)
        ? { op: "merge_cells", from, to }
        : null;
    }
    case "split_cell": {
      const at = parseCoordinate(value.at);
      return at ? { op: "split_cell", at } : null;
    }
    case "set_cell_text": {
      const at = parseCoordinate(value.at);
      return at && typeof value.text === "string" ? { op: "set_cell_text", at, text: value.text } : null;
    }
    case "set_cell_background": {
      const at = parseCoordinate(value.at);
      const color = value.color === null ? null : normalizeTableCellShading(value.color);
      return at && (value.color === null || color !== null)
        ? { op: "set_cell_background", at, color }
        : null;
    }
    case "move_table": return value.direction === "up" || value.direction === "down" ? { op: "move_table", direction: value.direction } : null;
    default: return null;
  }
}

/** 解析来自 Rust 投影或持久化历史的不可信表格 Diff 载荷。 */
export function parseTableEditPayload(value: unknown): TableEditPayload | undefined {
  if (!isRecord(value) || typeof value.tableId !== "string" || !value.tableId.trim() || typeof value.fingerprint !== "string" || !value.fingerprint.trim() || !Array.isArray(value.operations)) return undefined;
  const operations = value.operations.map(parseTableOperation);
  return operations.length > 0 && operations.every((operation): operation is TableOperation => operation !== null)
    ? { tableId: value.tableId, fingerprint: value.fingerprint, operations }
    : undefined;
}

function hash(value: string): string {
  let h = 0x811c9dc5;
  for (let i = 0; i < value.length; i += 1) {
    h ^= value.charCodeAt(i);
    h = Math.imul(h, 0x01000193);
  }
  return (h >>> 0).toString(16).padStart(8, "0");
}

function isPlainTextCell(cell: PMNode): boolean {
  const paragraph = cell.firstChild;
  if (cell.childCount !== 1 || paragraph?.type.name !== "paragraph") return false;
  let plain = true;
  paragraph.descendants((node) => {
    if (!node.isText || node.marks.length > 0) plain = false;
  });
  return plain;
}

function tableCells(table: PMNode, tablePos: number): TableCellSnapshot[] {
  const map = TableMap.get(table);
  const unique = new Set<number>();
  const cells: TableCellSnapshot[] = [];
  for (let row = 0; row < map.height; row += 1) {
    for (let column = 0; column < map.width; column += 1) {
      const offset = map.map[row * map.width + column];
      if (unique.has(offset)) continue;
      unique.add(offset);
      const cell = table.nodeAt(offset);
      if (!cell) continue;
      cells.push({
        row,
        column,
        rowspan: Number(cell.attrs.rowspan ?? 1),
        colspan: Number(cell.attrs.colspan ?? 1),
        text: cell.textContent,
        backgroundColor: normalizeTableCellShading(cell.attrs.backgroundColor),
        contentKind: isPlainTextCell(cell) ? "plain_text" : "rich_content",
      });
    }
  }
  void tablePos; // tablePos 是调用方的绝对位置；投影只需表内偏移。
  return cells;
}

function columnWidths(table: PMNode): Array<number | null> {
  const map = TableMap.get(table);
  return [...Array(map.width).keys()].map((column) => {
    const cell = table.nodeAt(map.map[column]);
    const widths = Array.isArray(cell?.attrs.colwidth) ? cell?.attrs.colwidth : null;
    // 合并单元格的 colwidth 是跨列数组；必须按该单元格内的相对列取值，不能把第一列宽复制给所有列。
    const rect = map.findCell(map.map[column]);
    const width = widths?.[column - rect.left];
    return typeof width === "number" && Number.isFinite(width) ? width : null;
  });
}

function moveDirectionsFor(doc: PMNode, tablePos: number): Array<"up" | "down"> {
  const $pos = doc.resolve(tablePos);
  if ($pos.depth !== 0) return [];
  const children: PMNode[] = [];
  let index = -1;
  doc.forEach((node, offset, i) => {
    children.push(node);
    if (offset === tablePos) index = i;
  });
  if (index < 0) return [];
  const directions: Array<"up" | "down"> = [];
  if (index > 0 && children[index - 1].type.name !== "table") directions.push("up");
  if (index + 1 < children.length && children[index + 1].type.name !== "table") directions.push("down");
  return directions;
}

function capabilitiesFor(table: PMNode, tablePos: number, doc: PMNode, cells: TableCellSnapshot[]): TableCapability[] {
  const map = TableMap.get(table);
  const capabilities: TableCapability[] = ["insert_row", "insert_column", "set_header_row", "set_cell_background"];
  if (map.height > 1) capabilities.push("delete_row");
  if (map.width > 1) capabilities.push("delete_column");
  if (map.height > 1 || map.width > 1) capabilities.push("merge_cells");
  if (cells.some((cell) => cell.rowspan > 1 || cell.colspan > 1)) capabilities.push("split_cell");
  if (cells.some((cell) => cell.contentKind === "plain_text")) capabilities.push("set_cell_text");
  if (moveDirectionsFor(doc, tablePos).length) capabilities.push("move_table");
  return capabilities;
}

function snapshotFor(table: PMNode, tablePos: number, ordinal: number, doc: PMNode): TableSnapshot | null {
  const tableId = typeof table.attrs.tableId === "string" ? table.attrs.tableId : "";
  if (!tableId) return null;
  const map = TableMap.get(table);
  const cells = tableCells(table, tablePos);
  const headerRow = map.height > 0 && [...Array(map.width).keys()].every((column) => {
    const offset = map.map[column];
    return table.nodeAt(offset)?.type.name === "tableHeader";
  });
  const widths = columnWidths(table);
  const canonical = JSON.stringify({ rows: map.height, columns: map.width, headerRow, columnWidths: widths, cells });
  return { tableId, ordinal, rows: map.height, columns: map.width, headerRow, columnWidths: widths, moveDirections: moveDirectionsFor(doc, tablePos), fingerprint: hash(canonical), cells, capabilities: capabilitiesFor(table, tablePos, doc, cells) };
}

/** 将当前 PM 文档投影为代理可读的表格目录。无 tableId 的表会由 TableIdExtension 随后补齐。 */
export function projectTables(doc: PMNode): TableSnapshot[] {
  const out: TableSnapshot[] = [];
  doc.descendants((node, pos) => {
    if (node.type.name !== "table") return true;
    const snapshot = snapshotFor(node, pos, out.length, doc);
    if (snapshot) out.push(snapshot);
    return false;
  });
  return out;
}

export function listActiveTables(): TableSnapshot[] {
  const editor = getActiveEditor();
  return editor ? projectTables(editor.state.doc) : [];
}

/** 当前光标所在表格的快照；人工工具条与代理能力投影必须使用同一判定。 */
export function activeTableSnapshot(editor = getActiveEditor()): TableSnapshot | null {
  if (!editor) return null;
  // nodesBetween(selection.from, selection.to) 不包含选区所在的祖先节点；从 $from 向上找才能识别单元格内光标。
  let tableId = "";
  for (let depth = editor.state.selection.$from.depth; depth > 0; depth -= 1) {
    const node = editor.state.selection.$from.node(depth);
    if (node.type.name === "table" && typeof node.attrs.tableId === "string") {
      tableId = node.attrs.tableId;
      break;
    }
  }
  return projectTables(editor.state.doc).find((table) => table.tableId === tableId) ?? null;
}

function locateTable(doc: PMNode, tableId: string): { node: PMNode; pos: number; ordinal: number } | null {
  let found: { node: PMNode; pos: number; ordinal: number } | null = null;
  let ordinal = 0;
  doc.descendants((node, pos) => {
    if (node.type.name !== "table") return true;
    const hit = typeof node.attrs.tableId === "string" && node.attrs.tableId === tableId;
    if (hit) found = { node, pos, ordinal };
    ordinal += 1;
    return !hit;
  });
  return found;
}

function selectionNearTable(doc: PMNode, tableId: string): Selection | null {
  const table = locateTable(doc, tableId);
  if (!table) return null;
  const firstCellOffset = TableMap.get(table.node).map[0];
  if (!Number.isInteger(firstCellOffset)) return null;
  const cellPos = table.pos + 1 + firstCellOffset;
  return Selection.near(doc.resolve(Math.min(doc.content.size, cellPos + 1)), 1);
}

function absoluteCellPos(tablePos: number, table: PMNode, row: number, column: number): number | null {
  const map = TableMap.get(table);
  if (!Number.isInteger(row) || !Number.isInteger(column) || row < 0 || column < 0 || row >= map.height || column >= map.width) return null;
  return tablePos + 1 + map.map[row * map.width + column];
}

function withCellSelection(state: EditorState, tableId: string, row: number, column: number, head?: { row: number; column: number }): EditorState | null {
  const table = locateTable(state.doc, tableId);
  if (!table) return null;
  const anchor = absoluteCellPos(table.pos, table.node, row, column);
  const headPos = head ? absoluteCellPos(table.pos, table.node, head.row, head.column) : undefined;
  if (anchor === null || (head && headPos === null)) return null;
  return state.apply(state.tr.setSelection(CellSelection.create(state.doc, anchor, headPos ?? undefined)));
}

function runCommand(state: EditorState, command: (state: EditorState, dispatch?: (tr: Transaction) => void) => boolean): EditorState | null {
  let next: Transaction | null = null;
  if (!command(state, (tr) => { next = tr; }) || !next) return null;
  return state.apply(next);
}

function replaceCellText(state: EditorState, tableId: string, row: number, column: number, text: string): EditorState | null {
  const table = locateTable(state.doc, tableId);
  if (!table) return null;
  const cellPos = absoluteCellPos(table.pos, table.node, row, column);
  if (cellPos === null) return null;
  const cell = state.doc.nodeAt(cellPos);
  const paragraph = state.schema.nodes.paragraph?.createAndFill(null, text ? state.schema.text(text) : undefined);
  if (!cell || !paragraph || !isPlainTextCell(cell)) return null;
  return state.apply(state.tr.replaceWith(cellPos + 1, cellPos + cell.nodeSize - 1, paragraph));
}

function setCellBackground(
  state: EditorState,
  tableId: string,
  row: number,
  column: number,
  color: string | null,
): EditorState | null {
  const table = locateTable(state.doc, tableId);
  if (!table) return null;
  const map = TableMap.get(table.node);
  if (row < 0 || column < 0 || row >= map.height || column >= map.width) return null;
  const offset = map.map[row * map.width + column];
  const rect = map.findCell(offset);
  // 合并单元格只接受左上锚；内部坐标不得悄悄映射到同一节点。
  if (rect.top !== row || rect.left !== column) return null;
  const cellPos = table.pos + 1 + offset;
  const cell = state.doc.nodeAt(cellPos);
  if (!cell || !["tableCell", "tableHeader", "table_cell", "table_header"].includes(cell.type.name)) return null;
  const normalized = color === null ? null : normalizeTableCellShading(color);
  if (color !== null && normalized === null) return null;
  if (normalizeTableCellShading(cell.attrs.backgroundColor) === normalized) return state;
  return state.apply(state.tr.setNodeMarkup(cellPos, undefined, {
    ...cell.attrs,
    backgroundColor: normalized,
  }));
}

function moveTable(state: EditorState, tableId: string, direction: "up" | "down"): EditorState | null {
  const table = locateTable(state.doc, tableId);
  if (!table) return null;
  const $pos = state.doc.resolve(table.pos);
  if ($pos.depth !== 0) return null; // 不支持嵌套表格或列表内部跨容器移动。
  const children: PMNode[] = [];
  let index = -1;
  state.doc.forEach((node, offset, i) => {
    children.push(node);
    if (offset === table.pos) index = i;
  });
  const to = direction === "up" ? index - 1 : index + 1;
  if (index < 0 || to < 0 || to >= children.length || children[to].type.name === "table") return null;
  [children[index], children[to]] = [children[to], children[index]];
  return state.apply(state.tr.replace(0, state.doc.content.size, new Slice(Fragment.fromArray(children), 0, 0)));
}

function applyOne(state: EditorState, tableId: string, op: TableOperation): EditorState | null {
  const table = locateTable(state.doc, tableId);
  if (!table) return null;
  const map = TableMap.get(table.node);
  switch (op.op) {
    case "insert_row": {
      if (!Number.isInteger(op.at) || op.at < 0 || op.at > map.height) return null;
      const row = op.at === map.height ? map.height - 1 : op.at;
      const selected = withCellSelection(state, tableId, row, 0);
      return selected ? runCommand(selected, op.at === map.height ? addRowAfter : addRowBefore) : null;
    }
    case "delete_row": {
      if (map.height <= 1) return null;
      const selected = withCellSelection(state, tableId, op.row, 0);
      return selected ? runCommand(selected, deleteRow) : null;
    }
    case "insert_column": {
      if (!Number.isInteger(op.at) || op.at < 0 || op.at > map.width) return null;
      const column = op.at === map.width ? map.width - 1 : op.at;
      const selected = withCellSelection(state, tableId, 0, column);
      return selected ? runCommand(selected, op.at === map.width ? addColumnAfter : addColumnBefore) : null;
    }
    case "delete_column": {
      if (map.width <= 1) return null;
      const selected = withCellSelection(state, tableId, 0, op.column);
      return selected ? runCommand(selected, deleteColumn) : null;
    }
    case "set_header_row": {
      const now = snapshotFor(table.node, table.pos, table.ordinal, state.doc)?.headerRow;
      if (now === op.enabled) return state;
      const selected = withCellSelection(state, tableId, 0, 0);
      return selected ? runCommand(selected, toggleHeaderRow) : null;
    }
    case "merge_cells": {
      const selected = withCellSelection(state, tableId, op.from.row, op.from.column, op.to);
      return selected ? runCommand(selected, mergeCells) : null;
    }
    case "split_cell": {
      const selected = withCellSelection(state, tableId, op.at.row, op.at.column);
      return selected ? runCommand(selected, splitCell) : null;
    }
    case "set_cell_text":
      return replaceCellText(state, tableId, op.at.row, op.at.column, op.text);
    case "set_cell_background":
      return setCellBackground(state, tableId, op.at.row, op.at.column, op.color);
    case "move_table":
      return moveTable(state, tableId, op.direction);
  }
}

/**
 * @GOV
 * codes:    BR-ED-DATA-007-RB-ED-ED-EDIT-018
 * type:     RB
 * chain:    ED-EDIT, DE-DIFF
 * rules:    BR-ED-DATA-007
 * term_ref: TERM-ED-007, TERM-DOC-005
 * boundary: in=TableEditPayloadAndPreUpdateEditorState | out=TableOpResultWithBeforeAfterTableSnapshot | delegate=applyOne
 */
/** 只模拟，不写入编辑器；任一操作失败返回原因，保证代理预览与实际执行使用同一规则。 */
export function planTableOperations(state: EditorState, payload: TableEditPayload): TableOpResult {
  const normalized = parseTableEditPayload(payload);
  if (!normalized) return { ok: false, reason: "table_operation_invalid" };
  const initial = locateTable(state.doc, normalized.tableId);
  if (!initial) return { ok: false, reason: "table_anchor_lost" };
  const before = snapshotFor(initial.node, initial.pos, initial.ordinal, state.doc);
  if (!before) return { ok: false, reason: "table_id_missing" };
  if (before.fingerprint !== normalized.fingerprint) return { ok: false, reason: "table_fingerprint_mismatch" };
  if (!normalized.operations.length) return { ok: false, reason: "table_operations_empty" };
  let next = state;
  for (const op of normalized.operations) {
    const applied = applyOne(next, normalized.tableId, op);
    if (!applied) return { ok: false, reason: `table_operation_failed:${op.op}` };
    next = applied;
  }
  const finalTable = locateTable(next.doc, normalized.tableId);
  const after = finalTable ? snapshotFor(finalTable.node, finalTable.pos, finalTable.ordinal, next.doc) : null;
  if (!after) return { ok: false, reason: "table_after_anchor_lost" };
  return { ok: true, before, after, state: next };
}

/**
 * @GOV
 * codes:    BR-ED-DATA-007-EFFECT-ED-ED-EDIT-014
 * type:     EFFECT
 * chain:    ED-EDIT, DE-DIFF
 * rules:    BR-ED-DATA-007
 * term_ref: TERM-ED-007, TERM-DOC-005
 * boundary: in=ValidatedTableEditPayload | out=TableOpResultWithLogicalStateTransaction | effect=memory | does_not=write_DiskState
 */
/** 原子应用：先完整 plan，再以单个替换事务写入活动编辑器；人工移动可同时保持目标表格选区。 */
export function applyTableOperations(payload: TableEditPayload, focusTableId?: string): TableOpResult {
  const editor = getActiveEditor();
  if (!editor) return { ok: false, reason: "no_active_editor" };
  const plan = planTableOperations(editor.state, payload);
  if (!plan.ok) return plan;
  const transaction = editor.state.tr.replace(
    0,
    editor.state.doc.content.size,
    new Slice(plan.state.doc.content, 0, 0),
  );
  if (focusTableId) {
    const selection = selectionNearTable(transaction.doc, focusTableId);
    if (selection) transaction.setSelection(selection);
  }
  editor.view.dispatch(transaction);
  if (focusTableId) editor.view.focus();
  return plan;
}

/** 人工邻近控件的文档流移动入口；仅允许顶层表格跨一个非表格块交换位置。 */
export function moveActiveTable(direction: "up" | "down"): TableOpResult | { ok: false; reason: string } {
  const editor = getActiveEditor();
  if (!editor) return { ok: false, reason: "no_active_editor" };
  const snapshot = activeTableSnapshot(editor);
  if (!snapshot) return { ok: false, reason: "table_anchor_lost" };
  return applyTableOperations(
    { tableId: snapshot.tableId, fingerprint: snapshot.fingerprint, operations: [{ op: "move_table", direction }] },
    snapshot.tableId,
  );
}

export function summarizeTableOperation(op: TableOperation): string {
  switch (op.op) {
    case "insert_row": return `在第 ${op.at + 1} 行前插入一行`;
    case "delete_row": return `删除第 ${op.row + 1} 行`;
    case "insert_column": return `在第 ${op.at + 1} 列前插入一列`;
    case "delete_column": return `删除第 ${op.column + 1} 列`;
    case "set_header_row": return op.enabled ? "设为首行表头" : "取消首行表头";
    case "merge_cells": return `合并 R${op.from.row + 1}C${op.from.column + 1} 至 R${op.to.row + 1}C${op.to.column + 1}`;
    case "split_cell": return `拆分 R${op.at.row + 1}C${op.at.column + 1}`;
    case "set_cell_text": return `更新 R${op.at.row + 1}C${op.at.column + 1} 内容`;
    case "set_cell_background": return `${op.color ? `设置 ${op.color}` : "清除"} R${op.at.row + 1}C${op.at.column + 1} 底纹`;
    case "move_table": return op.direction === "up" ? "表格上移一块" : "表格下移一块";
  }
}
