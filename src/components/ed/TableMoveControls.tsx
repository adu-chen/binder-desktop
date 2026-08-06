import { useCallback, useEffect, useState } from "react";
import { createPortal } from "react-dom";
import type { Editor } from "@tiptap/react";
import { activeTableSnapshot, moveActiveTable } from "../../services/edTableOps";
import { normalizeTableCellShading } from "./extensions/TableCellShading";
import { ContextMenu, type ContextMenuDef } from "../shell/ContextMenu";

function tableElementFromNode(node: Node | null): HTMLElement | null {
  const element = node instanceof Element ? node : node?.parentElement;
  const table = element?.closest("table") ?? element?.querySelector("table") ?? null;
  return table instanceof HTMLElement ? table : null;
}

function activeTableElement(editor: Editor): HTMLElement | null {
  // 真实 TableView 可能用 .tableWrapper 包裹 table；优先从当前选区 DOM 反查，避免依赖
  // nodeDOM(tablePos) 在不同 TipTap/ProseMirror NodeView 下返回 wrapper、table 或 null 的差异。
  const selectionDom = editor.view.domAtPos(editor.state.selection.from).node;
  const selectedTable = tableElementFromNode(selectionDom);
  if (selectedTable) return selectedTable;

  const $from = editor.state.selection.$from;
  let tablePos: number | null = null;
  for (let depth = $from.depth; depth > 0; depth -= 1) {
    if ($from.node(depth).type.name === "table") {
      tablePos = $from.before(depth);
      break;
    }
  }
  if (tablePos === null) return null;
  return tableElementFromNode(editor.view.nodeDOM(tablePos));
}

function tableWrapperFromTable(table: HTMLElement | null): HTMLElement | null {
  const wrapper = table?.closest(".tableWrapper");
  return wrapper instanceof HTMLElement ? wrapper : null;
}

interface MoveControlView {
  anchor: HTMLElement;
  moveDirections: Array<"up" | "down">;
  tableId: string | null;
  capabilities: NonNullable<ReturnType<typeof activeTableSnapshot>>["capabilities"];
  headerRow: boolean;
}

/**
 * @GOV
 * codes:    BR-ED-DATA-007-RB-ED-ED-EDIT-019
 * type:     RB
 * chain:    ED-EDIT
 * rules:    BR-ED-DATA-007
 * term_ref: TERM-ED-007
 * boundary: in=ActiveTableSelectionAndTableViewWrapper | out=WrapperAnchoredTemporaryTableControls(structureOrCellShadingOrDocumentFlowMoveOrDeleteViaControlledEditorCommand) | does_not=calculate_cross_container_coordinates_or_enable_drag
 */
export function TableMoveControls({
  editor,
  enabled = true,
}: {
  editor: Editor | null;
  enabled?: boolean;
}) {
  const [view, setView] = useState<MoveControlView | null>(null);
  // 表格右键（CTX-MENU-01）：删除表格快捷入口（stopPropagation 避免冒泡到编辑器右键）。
  const [ctxMenu, setCtxMenu] = useState<{ x: number; y: number } | null>(null);

  const update = useCallback(() => {
    if (!enabled || !editor) {
      setView(null);
      return;
    }
    const snapshot = activeTableSnapshot(editor);
    const table = activeTableElement(editor);
    const anchor = tableWrapperFromTable(table);
    setView(anchor
      ? {
          anchor,
          moveDirections: snapshot?.moveDirections ?? [],
          tableId: snapshot?.tableId ?? null,
          capabilities: snapshot?.capabilities ?? [],
          headerRow: snapshot?.headerRow ?? false,
        }
      : null);
  }, [editor, enabled]);

  useEffect(() => {
    if (!editor || !enabled) {
      setView(null);
      return;
    }
    let frame = 0;
    const schedule = () => {
      if (frame) cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        frame = 0;
        update();
      });
    };
    update();
    editor.on("selectionUpdate", schedule);
    editor.on("transaction", schedule);
    // WebKit 中再次点击原单元格不一定产生 selectionUpdate；DOM 事件确保控件仍能被唤起。
    editor.view.dom.addEventListener("pointerup", schedule);
    editor.view.dom.addEventListener("keyup", schedule);
    editor.view.dom.addEventListener("focusin", schedule);
    return () => {
      if (frame) cancelAnimationFrame(frame);
      editor.off("selectionUpdate", schedule);
      editor.off("transaction", schedule);
      editor.view.dom.removeEventListener("pointerup", schedule);
      editor.view.dom.removeEventListener("keyup", schedule);
      editor.view.dom.removeEventListener("focusin", schedule);
    };
  }, [editor, enabled, update]);

  if (!view) return null;

  const move = (direction: "up" | "down") => {
    moveActiveTable(direction);
    requestAnimationFrame(update);
  };
  const remove = () => {
    const removed = editor?.chain().focus().deleteTable().run() ?? false;
    if (removed) setView(null);
    else requestAnimationFrame(update);
  };
  // 表格右键（CTX-MENU-01）：仅提供「删除表格」（行/列/合并/底纹等已有浮动控件）。
  const onContextMenu = (event: React.MouseEvent) => {
    event.preventDefault();
    event.stopPropagation();
    setCtxMenu({ x: event.clientX, y: event.clientY });
  };
  const ctxMenuItems: ContextMenuDef[] | null = ctxMenu
    ? [{ type: "item", id: "delete", label: "删除表格", danger: true, onSelect: remove }]
    : null;
  const supports = (capability: MoveControlView["capabilities"][number]) => view.capabilities.includes(capability);
  const safeEditor = editor as Partial<Editor> | null;
  const tableCan = safeEditor?.can?.() as { mergeCells?: () => boolean; splitCell?: () => boolean } | undefined;
  const activeCellShading = normalizeTableCellShading(
    safeEditor?.getAttributes?.(safeEditor.isActive?.("tableHeader") ? "tableHeader" : "tableCell").backgroundColor,
  );

  return createPortal(
    <div
      className="ed-table-move-controls"
      role="group"
      aria-label="表格位置移动"
      data-table-id={view.tableId ?? undefined}
      onContextMenu={onContextMenu}
      onMouseDown={(event) => {
        event.preventDefault();
        event.stopPropagation();
      }}
    >
      <span className="ed-context-label">表格</span>
      <button
        type="button"
        aria-label="上方插行"
        title="上方插行"
        disabled={!supports("insert_row")}
        onClick={() => editor?.chain().focus().addRowBefore().run()}
      >
        +行
      </button>
      <button
        type="button"
        aria-label="右侧插列"
        title="右侧插列"
        disabled={!supports("insert_column")}
        onClick={() => editor?.chain().focus().addColumnAfter().run()}
      >
        +列
      </button>
      <button
        type="button"
        aria-label="删除当前行"
        title="删除当前行"
        disabled={!supports("delete_row")}
        onClick={() => editor?.chain().focus().deleteRow().run()}
      >
        −行
      </button>
      <button
        type="button"
        aria-label="删除当前列"
        title="删除当前列"
        disabled={!supports("delete_column")}
        onClick={() => editor?.chain().focus().deleteColumn().run()}
      >
        −列
      </button>
      <button
        type="button"
        aria-label="切换表头行"
        title={view.headerRow ? "取消表头行" : "设为表头行"}
        disabled={!supports("set_header_row")}
        onClick={() => editor?.chain().focus().toggleHeaderRow().run()}
      >
        表头
      </button>
      <button
        type="button"
        aria-label="合并选中单元格"
        title="合并矩形选区内的单元格"
        disabled={!supports("merge_cells") || !(tableCan?.mergeCells?.() ?? false)}
        onClick={() => editor?.chain().focus().mergeCells().run()}
      >
        合并
      </button>
      <button
        type="button"
        aria-label="拆分当前单元格"
        title="拆分当前合并单元格"
        disabled={!supports("split_cell") || !(tableCan?.splitCell?.() ?? false)}
        onClick={() => editor?.chain().focus().splitCell().run()}
      >
        拆分
      </button>
      <label className="ed-context-color" title="设置所选单元格底纹">
        <span>底纹</span>
        <input
          type="color"
          aria-label="单元格底纹颜色"
          value={activeCellShading ?? "#FFF2CC"}
          disabled={!supports("set_cell_background")}
          onChange={(event) => editor?.chain().focus().setCellAttribute("backgroundColor", event.target.value.toUpperCase()).run()}
        />
      </label>
      <button
        type="button"
        aria-label="表格上移一块"
        title="表格上移一块"
        disabled={!view.moveDirections.includes("up")}
        onClick={() => move("up")}
      >
        ↑
      </button>
      <button
        type="button"
        aria-label="表格下移一块"
        title="表格下移一块"
        disabled={!view.moveDirections.includes("down")}
        onClick={() => move("down")}
      >
        ↓
      </button>
      <button
        type="button"
        className="ed-table-delete"
        aria-label="删除表格"
        title="删除表格"
        onClick={remove}
      >
        ×
      </button>
      {ctxMenu && ctxMenuItems && (
        <ContextMenu items={ctxMenuItems} x={ctxMenu.x} y={ctxMenu.y} onClose={() => setCtxMenu(null)} />
      )}
    </div>,
    view.anchor,
  );
}
