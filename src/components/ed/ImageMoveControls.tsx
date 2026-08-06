import { useCallback, useEffect, useState } from "react";
import { createPortal } from "react-dom";
import type { Editor } from "@tiptap/react";
import {
  activeImageSnapshot,
  deleteActiveImage,
  moveActiveImage,
} from "../../services/edImageOps";
import { ContextMenu, type ContextMenuDef } from "../shell/ContextMenu";

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
  alt: string | null;
  width: string | null;
  alignment: "left" | "center" | "right";
}

function currentImageAlignment(editor: Editor, imagePos: number): ImageControlView["alignment"] {
  const resolved = (editor as Partial<Editor>).state?.doc?.resolve(imagePos);
  const textAlign = resolved?.parent.attrs.textAlign;
  return textAlign === "center" || textAlign === "right" ? textAlign : "left";
}

/**
 * @GOV
 * codes:    BR-ED-DATA-001-RB-ED-ED-EDIT-024, BR-ED-DATA-003-RB-ED-ED-EDIT-024
 * type:     RB
 * chain:    ED-EDIT
 * rules:    BR-ED-DATA-001, BR-ED-DATA-003
 * boundary: in=SelectedImageNodeAndImageNodeView | out=ImageAnchoredTemporaryControls(editAltOrResetWidthOrSetStandaloneParagraphAlignmentOrDocumentFlowMoveOrDelete)
 */
export function ImageMoveControls({
  editor,
  enabled = true,
}: {
  editor: Editor | null;
  enabled?: boolean;
}) {
  const [view, setView] = useState<ImageControlView | null>(null);
  // 图片右键（CTX-MENU-01）：删除图片快捷入口（stopPropagation 避免冒泡到编辑器右键）。
  const [ctxMenu, setCtxMenu] = useState<{ x: number; y: number } | null>(null);

  const update = useCallback(() => {
    if (!enabled || !editor) {
      setView(null);
      return;
    }
    const snapshot = activeImageSnapshot(editor);
    const anchor = selectedImageElement(editor);
    const attributes = (editor as Partial<Editor>).getAttributes?.("image") ?? {};
    setView(snapshot && anchor
      ? {
          anchor,
          blockId: snapshot.blockId,
          standalone: snapshot.standalone,
          moveDirections: snapshot.moveDirections,
          alt: typeof attributes.alt === "string" ? attributes.alt : null,
          width: typeof attributes.width === "string" ? attributes.width : null,
          alignment: currentImageAlignment(editor, snapshot.imagePos),
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
  // 图片右键（CTX-MENU-01）：仅提供「删除图片」（移动/对齐等已有浮动控件）。
  const onContextMenu = (event: React.MouseEvent) => {
    event.preventDefault();
    event.stopPropagation();
    setCtxMenu({ x: event.clientX, y: event.clientY });
  };
  const ctxMenuItems: ContextMenuDef[] | null = ctxMenu
    ? [{ type: "item", id: "delete", label: "删除图片", danger: true, onSelect: remove }]
    : null;
  const setAlt = () => {
    const alt = window.prompt("图片替代文本（留空清除）", view.alt ?? "");
    if (alt === null) return;
    editor?.chain().focus().updateAttributes("image", { alt: alt || null }).run();
    requestAnimationFrame(update);
  };
  const resetWidth = () => {
    editor?.chain().focus().updateAttributes("image", { width: null, height: null }).run();
    requestAnimationFrame(update);
  };
  const setAlignment = (alignment: ImageControlView["alignment"]) => {
    if (!editor || !view.standalone) return;
    const snapshot = activeImageSnapshot(editor);
    if (!snapshot) return;
    const $image = editor.state.doc.resolve(snapshot.imagePos);
    if ($image.depth <= 0 || $image.parent.type.name !== "paragraph") return;
    editor.view.dispatch(editor.state.tr.setNodeMarkup($image.before($image.depth), undefined, {
      ...$image.parent.attrs,
      textAlign: alignment,
    }));
    requestAnimationFrame(update);
  };
  const moveHint = view.standalone ? undefined : "混排文字中的行内图片不能按块移动";

  return createPortal(
    <div
      className="ed-image-move-controls"
      role="group"
      aria-label="图片操作"
      data-block-id={view.blockId ?? undefined}
      data-standalone={view.standalone ? "true" : "false"}
      onContextMenu={onContextMenu}
      onMouseDown={(event) => {
        event.preventDefault();
        event.stopPropagation();
      }}
    >
      <span className="ed-context-label">图片</span>
      <button
        type="button"
        aria-label="编辑图片替代文本"
        title="编辑图片替代文本"
        onClick={setAlt}
      >
        Alt
      </button>
      <button
        type="button"
        aria-label="恢复图片原始宽度"
        title={view.width ? "恢复图片原始宽度" : "图片当前使用原始宽度"}
        disabled={!view.width}
        onClick={resetWidth}
      >
        宽度
      </button>
      {(["left", "center", "right"] as const).map((alignment) => (
        <button
          key={alignment}
          type="button"
          aria-label={`图片${alignment === "left" ? "左对齐" : alignment === "center" ? "居中" : "右对齐"}`}
          title={view.standalone ? `图片${alignment === "left" ? "左对齐" : alignment === "center" ? "居中" : "右对齐"}` : "仅独立图片段落支持对齐"}
          aria-pressed={view.alignment === alignment}
          disabled={!view.standalone}
          onClick={() => setAlignment(alignment)}
        >
          {alignment === "left" ? "左" : alignment === "center" ? "中" : "右"}
        </button>
      ))}
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
      <span className="ed-context-hint" title="拖动图片右下角手柄可调整宽度">拖动调宽</span>
      {ctxMenu && ctxMenuItems && (
        <ContextMenu items={ctxMenuItems} x={ctxMenu.x} y={ctxMenu.y} onClose={() => setCtxMenu(null)} />
      )}
    </div>,
    view.anchor,
  );
}
