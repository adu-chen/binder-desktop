import { Image } from "@tiptap/extension-image";

/**
 * 可交互图片（PI-ED-08）：width/height/media identity 都是图片节点属性；NodeView 只提供选中态与
 * 拖宽手柄。`data-display` 仅兼容旧会话 HTML，不属于 BDocx 持久能力：Pandoc 会在 DOCX 往返时丢弃
 * 该自定义属性，独立图片改由“段落仅含图片”的真实文档结构识别。
 */

/** 拖拽调宽的最小宽度（px）。 */
const MIN_WIDTH = 40;

/** Pandoc 会把 DOCX `wp:extent` 序列化成 CSS 物理长度；编辑器统一换算为 96dpi CSS px。 */
export function imageCssLengthToPx(raw: string | null | undefined): string | null {
  if (!raw) return null;
  const match = raw.trim().match(/^([0-9]+(?:\.[0-9]+)?)(px|in|cm|mm|pt|pc)?$/i);
  if (!match) return null;
  const value = Number(match[1]);
  if (!Number.isFinite(value) || value <= 0) return null;
  const factors: Record<string, number> = {
    px: 1,
    in: 96,
    cm: 96 / 2.54,
    mm: 96 / 25.4,
    pt: 96 / 72,
    pc: 16,
  };
  const px = value * factors[(match[2] ?? "px").toLowerCase()];
  return String(Math.max(1, Math.round(px)));
}

function styleDimension(element: HTMLElement, property: "width" | "height"): string | null {
  const direct = imageCssLengthToPx(element.getAttribute(property));
  if (direct) return direct;
  const style = element.getAttribute("style") ?? "";
  const value = style
    .split(";")
    .map((declaration) => declaration.split(":"))
    .find(([name]) => name?.trim().toLowerCase() === property)?.[1];
  return imageCssLengthToPx(value);
}

function editorZoom(element: HTMLElement): number {
  const inlineZoom = Number.parseFloat(element.style.zoom);
  if (Number.isFinite(inlineZoom) && inlineZoom > 0) return inlineZoom;
  const zoom = Number.parseFloat(window.getComputedStyle(element).zoom);
  return Number.isFinite(zoom) && zoom > 0 ? zoom : 1;
}

/**
 * @GOV
 * codes:    BR-ED-DATA-003-EFFECT-ED-ED-EDIT-020, BR-ED-STATE-001-EFFECT-ED-ED-EDIT-020
 * type:     EFFECT
 * chain:    ED-EDIT
 * rules:    BR-ED-DATA-003, BR-ED-STATE-001
 * term_ref: TERM-ED-008
 * boundary: in=SelectedImageNodeAndZoomNormalizedMouseDrag | out=LogicalStateImageWidthAttributeTransaction | effect=memory | does_not=write_DiskState_or_media_resource
 */
export const ResizableImage = Image.extend({
  name: "image",

  addAttributes() {
    return {
      ...this.parent?.(),
      /** 显示宽度（px；null=原始尺寸）。 */
      width: {
        default: null as string | null,
        parseHTML: (el) => styleDimension(el as HTMLElement, "width"),
        renderHTML: (attrs) => (attrs.width ? { width: attrs.width } : {}),
      },
      /** 来源图片高度（px）；人工拖动调宽后清空，让浏览器按固有比例重新计算。 */
      height: {
        default: null as string | null,
        parseHTML: (el) => styleDimension(el as HTMLElement, "height"),
        renderHTML: (attrs) => (attrs.height ? { height: attrs.height } : {}),
      },
      /** 可持久的 Pandoc 相对媒体身份；src=asset:// 只是当前进程显示投影。 */
      binderMediaSrc: {
        default: null as string | null,
        parseHTML: (el) => el.getAttribute("data-binder-media-src"),
        renderHTML: (attrs) => (
          attrs.binderMediaSrc ? { "data-binder-media-src": attrs.binderMediaSrc } : {}
        ),
      },
      /** 代理插入图片的 workspace 相对来源；仅用于未保存草稿缓存重建。 */
      binderProjectSrc: {
        default: null as string | null,
        parseHTML: (el) => el.getAttribute("data-binder-project-src"),
        renderHTML: (attrs) => (
          attrs.binderProjectSrc ? { "data-binder-project-src": attrs.binderProjectSrc } : {}
        ),
      },
      /** 与 binderProjectSrc 配对的来源 SHA-256；来源变化时 DOC 明确拒绝重建。 */
      binderProjectHash: {
        default: null as string | null,
        parseHTML: (el) => el.getAttribute("data-binder-project-hash"),
        renderHTML: (attrs) => (
          attrs.binderProjectHash ? { "data-binder-project-hash": attrs.binderProjectHash } : {}
        ),
      },
      /** 旧会话显示兼容；不作为 BDocx inline/block 持久契约。 */
      display: {
        default: "inline",
        parseHTML: (el) => (el.getAttribute("data-display") === "block" ? "block" : "inline"),
        renderHTML: (attrs) => (attrs.display === "block" ? { "data-display": "block" } : {}),
      },
    };
  },

  addNodeView() {
    return ({ node, editor, getPos }) => {
      let current = node;
      const isBlock = () => current.attrs.display === "block";
      // Image 配置为 inline node，NodeView DOM 必须保持行内容器；旧 block 投影只通过 CSS 兼容。
      const wrap = document.createElement("span");
      wrap.className = "ed-img-view";
      if (isBlock()) wrap.classList.add("ed-img-block");

      const img = document.createElement("img");
      img.className = "editor-image";
      const syncImg = () => {
        img.src = current.attrs.src as string;
        if (current.attrs.alt) img.alt = current.attrs.alt as string;
        if (current.attrs.width) img.setAttribute("width", String(current.attrs.width));
        else img.removeAttribute("width");
        if (current.attrs.height) img.setAttribute("height", String(current.attrs.height));
        else img.removeAttribute("height");
      };
      syncImg();

      // 右下角调宽手柄（选中时显示）；document 捕获阶段监听兼容 WebKit 丢失 pointer capture 的情况。
      const handle = document.createElement("span");
      handle.className = "ed-img-handle";
      handle.contentEditable = "false";
      handle.setAttribute("role", "separator");
      handle.setAttribute("aria-label", "拖拽调整图片宽度");
      handle.setAttribute("aria-orientation", "vertical");
      handle.setAttribute("title", "拖拽调整图片宽度");
      let removeDragListeners: (() => void) | null = null;

      const commitWidth = (width: number) => {
        img.style.removeProperty("width");
        const pos = typeof getPos === "function" ? getPos() : undefined;
        if (typeof pos !== "number") {
          syncImg();
          return;
        }
        const imageNode = editor.state.doc.nodeAt(pos);
        if (!imageNode || imageNode.type.name !== "image") {
          syncImg();
          return;
        }
        const nextWidth = String(width);
        if (String(imageNode.attrs.width ?? "") === nextWidth) {
          syncImg();
          return;
        }
        editor.view.dispatch(editor.state.tr.setNodeMarkup(pos, undefined, {
          ...imageNode.attrs,
          width: nextWidth,
          height: null,
        }));
      };

      const beginResize = (event: MouseEvent) => {
        if (event.button !== 0) return;
        event.preventDefault();
        event.stopPropagation();
        removeDragListeners?.();
        const ownerDocument = wrap.ownerDocument;
        const zoom = editorZoom(editor.view.dom);
        const startWidth = Math.max(MIN_WIDTH, img.getBoundingClientRect().width / zoom);
        const parentWidth = wrap.parentElement?.getBoundingClientRect().width ?? 0;
        const maxWidth = parentWidth > 0 ? Math.max(MIN_WIDTH, parentWidth / zoom) : Number.POSITIVE_INFINITY;
        const startX = event.clientX;
        let previewWidth = Math.min(maxWidth, startWidth);
        let moved = false;

        const move = (moveEvent: MouseEvent) => {
          moveEvent.preventDefault();
          moved = moved || moveEvent.clientX !== startX;
          previewWidth = Math.max(
            MIN_WIDTH,
            Math.min(maxWidth, Math.round(startWidth + (moveEvent.clientX - startX) / zoom)),
          );
          img.style.width = `${previewWidth}px`;
          img.setAttribute("width", String(previewWidth));
        };
        const finish = (upEvent: MouseEvent) => {
          upEvent.preventDefault();
          removeDragListeners?.();
          if (moved) commitWidth(Math.round(previewWidth));
          else {
            img.style.removeProperty("width");
            syncImg();
          }
        };
        removeDragListeners = () => {
          ownerDocument.removeEventListener("mousemove", move, true);
          ownerDocument.removeEventListener("mouseup", finish, true);
          removeDragListeners = null;
        };
        ownerDocument.addEventListener("mousemove", move, true);
        ownerDocument.addEventListener("mouseup", finish, true);
      };
      handle.addEventListener("mousedown", beginResize);

      wrap.append(img, handle);
      return {
        dom: wrap,
        update(next) {
          if (next.type.name !== "image") return false;
          current = next;
          wrap.classList.toggle("ed-img-block", isBlock());
          syncImg();
          return true;
        },
        selectNode() {
          wrap.classList.add("selected");
        },
        deselectNode() {
          wrap.classList.remove("selected");
        },
        ignoreMutation(m) {
          // 手柄拖动的 DOM 预览变更不回灌 ProseMirror（提交经 command）。
          return m.type !== "selection";
        },
        stopEvent(event) {
          return event.target === handle || handle.contains(event.target as Node);
        },
        destroy() {
          removeDragListeners?.();
          handle.removeEventListener("mousedown", beginResize);
        },
      };
    };
  },
});
