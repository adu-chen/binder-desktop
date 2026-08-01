import { useCallback, useEffect, useState } from "react";
import { createPortal } from "react-dom";
import type { Editor } from "@tiptap/react";
import { activeTableSnapshot, moveActiveTable } from "../../services/edTableOps";

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
}

/**
 * @GOV
 * codes:    BR-ED-DATA-007-RB-ED-ED-EDIT-019
 * type:     RB
 * chain:    ED-EDIT
 * rules:    BR-ED-DATA-007
 * term_ref: TERM-ED-007
 * boundary: in=ActiveTableSelectionAndTableViewWrapper | out=WrapperAnchoredTemporaryTableControls(moveOneBlockOrDeleteViaControlledEditorCommand) | does_not=calculate_cross_container_coordinates_or_enable_drag
 */
export function TableMoveControls({
  editor,
  enabled = true,
}: {
  editor: Editor | null;
  enabled?: boolean;
}) {
  const [view, setView] = useState<MoveControlView | null>(null);

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

  return createPortal(
    <div
      className="ed-table-move-controls"
      role="group"
      aria-label="表格位置移动"
      data-table-id={view.tableId ?? undefined}
      onMouseDown={(event) => {
        event.preventDefault();
        event.stopPropagation();
      }}
    >
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
    </div>,
    view.anchor,
  );
}
