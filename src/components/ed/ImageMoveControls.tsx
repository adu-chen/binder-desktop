import { useCallback, useEffect, useState } from "react";
import { createPortal } from "react-dom";
import type { Editor } from "@tiptap/react";
import {
  activeImageSnapshot,
  deleteActiveImage,
  moveActiveImage,
} from "../../services/edImageOps";

function selectedImageElement(editor: Editor): HTMLElement | null {
  const snapshot = activeImageSnapshot(editor);
  if (!snapshot) return null;
  const dom = editor.view.nodeDOM(snapshot.imagePos);
  const element = dom instanceof HTMLElement ? dom : dom?.parentElement;
  const imageView = element?.matches(".ed-img-view")
    ? element
    : element?.closest(".ed-img-view");
  return imageView instanceof HTMLElement ? imageView : null;
}

interface ImageControlView {
  anchor: HTMLElement;
  blockId: string | null;
  standalone: boolean;
  moveDirections: Array<"up" | "down">;
}

/**
 * @GOV
 * codes:    BR-ED-DATA-001-RB-ED-ED-EDIT-024, BR-ED-DATA-003-RB-ED-ED-EDIT-024
 * type:     RB
 * chain:    ED-EDIT
 * rules:    BR-ED-DATA-001, BR-ED-DATA-003
 * boundary: in=SelectedImageNodeAndImageNodeView | out=ImageAnchoredTemporaryControls(documentFlowMoveOrDelete)
 */
export function ImageMoveControls({
  editor,
  enabled = true,
}: {
  editor: Editor | null;
  enabled?: boolean;
}) {
  const [view, setView] = useState<ImageControlView | null>(null);

  const update = useCallback(() => {
    if (!enabled || !editor) {
      setView(null);
      return;
    }
    const snapshot = activeImageSnapshot(editor);
    const anchor = selectedImageElement(editor);
    setView(snapshot && anchor
      ? {
          anchor,
          blockId: snapshot.blockId,
          standalone: snapshot.standalone,
          moveDirections: snapshot.moveDirections,
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
    moveActiveImage(editor, direction);
    requestAnimationFrame(update);
  };
  const remove = () => {
    if (deleteActiveImage(editor)) setView(null);
    else requestAnimationFrame(update);
  };
  const moveHint = view.standalone ? undefined : "混排文字中的行内图片不能按块移动";

  return createPortal(
    <div
      className="ed-image-move-controls"
      role="group"
      aria-label="图片操作"
      data-block-id={view.blockId ?? undefined}
      data-standalone={view.standalone ? "true" : "false"}
      onMouseDown={(event) => {
        event.preventDefault();
        event.stopPropagation();
      }}
    >
      <button
        type="button"
        aria-label="图片上移一块"
        title={moveHint ?? "图片上移一块"}
        disabled={!view.moveDirections.includes("up")}
        onClick={() => move("up")}
      >
        ↑
      </button>
      <button
        type="button"
        aria-label="图片下移一块"
        title={moveHint ?? "图片下移一块"}
        disabled={!view.moveDirections.includes("down")}
        onClick={() => move("down")}
      >
        ↓
      </button>
      <button
        type="button"
        className="ed-image-delete"
        aria-label="删除图片"
        title="删除图片"
        onClick={remove}
      >
        ×
      </button>
    </div>,
    view.anchor,
  );
}
