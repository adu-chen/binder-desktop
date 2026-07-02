import { Image } from "@tiptap/extension-image";

/**
 * 可交互图片（PI-ED-08 · #3 图片嵌入样式切换 + 大小拖动可调）。
 * 扩展 TipTap Image：`width`（拖拽手柄调整）与 `display`（inline/block 嵌入样式）为**节点属性**——
 * 属用户编辑（改文档模型、产生脏态、经 SM-ED-01→SM-WS-01 保存），非装饰层；序列化为
 * `width=` / `data-display=` 落 HTML（docx 往返保留度与 DOC 联合实测，A 类登记）。
 * NodeView 仅承载选中态手柄呈现与拖拽交互（X-FORBID-005：属性变更经 editor command 受控接口）。
 */

/** 拖拽调宽的最小宽度（px）。 */
const MIN_WIDTH = 40;

export const ResizableImage = Image.extend({
  name: "image",

  addAttributes() {
    return {
      ...this.parent?.(),
      /** 显示宽度（px；null=原始尺寸）。 */
      width: {
        default: null as string | null,
        parseHTML: (el) => el.getAttribute("width"),
        renderHTML: (attrs) => (attrs.width ? { width: attrs.width } : {}),
      },
      /** 嵌入样式：inline=行内随文字，block=独占块居中。 */
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
      const wrap = document.createElement(isBlock() ? "div" : "span");
      wrap.className = "ed-img-view";
      if (isBlock()) wrap.classList.add("ed-img-block");

      const img = document.createElement("img");
      img.className = "editor-image";
      const syncImg = () => {
        img.src = current.attrs.src as string;
        if (current.attrs.alt) img.alt = current.attrs.alt as string;
        if (current.attrs.width) img.setAttribute("width", String(current.attrs.width));
        else img.removeAttribute("width");
      };
      syncImg();

      // 右下角调宽手柄（选中时显示）；拖动按编辑器 zoom 归一化位移。
      const handle = document.createElement("span");
      handle.className = "ed-img-handle";
      handle.contentEditable = "false";
      let dragging = false;
      let startX = 0;
      let startW = 0;
      handle.addEventListener("pointerdown", (e) => {
        e.preventDefault();
        e.stopPropagation();
        dragging = true;
        startX = e.clientX;
        startW = img.getBoundingClientRect().width;
        handle.setPointerCapture(e.pointerId);
      });
      handle.addEventListener("pointermove", (e) => {
        if (!dragging) return;
        const zoom = parseFloat(window.getComputedStyle(editor.view.dom).zoom) || 1;
        const w = Math.max(MIN_WIDTH, Math.round(startW / zoom + (e.clientX - startX) / zoom));
        img.setAttribute("width", String(w)); // 拖动中即时预览（DOM 级）
      });
      const commit = (e: PointerEvent) => {
        if (!dragging) return;
        dragging = false;
        try {
          handle.releasePointerCapture(e.pointerId);
        } catch {
          /* 已释放 */
        }
        const w = img.getAttribute("width");
        const pos = typeof getPos === "function" ? getPos() : undefined;
        if (w !== null && pos !== undefined) {
          // 经受控 command 落属性 → 文档模型变更 → 脏态（用户编辑，X-FORBID-005）。
          editor
            .chain()
            .focus()
            .command(({ tr }) => {
              tr.setNodeMarkup(pos, undefined, { ...current.attrs, width: w });
              return true;
            })
            .run();
        }
      };
      handle.addEventListener("pointerup", commit);
      handle.addEventListener("pointercancel", commit);

      wrap.append(img, handle);
      return {
        dom: wrap,
        update(next) {
          if (next.type.name !== "image") return false;
          if (next.attrs.display !== current.attrs.display) return false; // 嵌入样式变化 → 重建视图（span↔div）
          current = next;
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
      };
    };
  },
});
