import { useEffect, useReducer } from "react";
import type { Editor } from "@tiptap/react";
import { ZOOM, formatZoom } from "./zoom";

/**
 * 编辑器工具栏（ED · PI-FE-05 编辑工具）。经 ED 声明的 TipTap command 接口触发富文本编辑
 * （X-FORBID-005 唯一受控通道：工具栏=用户编辑，走 SM-ED-01→脏态→SM-WS-01，不绕过编辑接口、
 * 不直改文档状态）。工具栏是纯视图控件，不另立落盘态（X-FORBID-007）。
 *
 * 工具映射已注册扩展集（extensions/index.ts）：撤销/重做(StarterKit) · 段落/标题(StarterKit) ·
 * 字号(FontSize) · 粗斜下删(StarterKit+Underline) · 上下标(Sub/Superscript) · 颜色(Color)/高亮(Highlight)
 * · 有序无序列表/引用/代码块(StarterKit) · 对齐(TextAlign) · 链接(Link)/图片(Image)/表格(Table*)。
 */

const FONT_SIZES = [12, 14, 16, 18, 20, 24, 28, 36] as const;
const HIGHLIGHT_COLOR = "#fde68a"; // 默认高亮（暖黄，与设计色板协调）

/** 工具栏按钮：active 高亮 + disabled；mousedown 阻止默认避免编辑器失焦。 */
function TbBtn({
  onClick,
  active,
  disabled,
  title,
  children,
}: {
  onClick: () => void;
  active?: boolean;
  disabled?: boolean;
  title: string;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      className={`ed-tb-btn${active ? " active" : ""}`}
      title={title}
      aria-label={title}
      aria-pressed={active}
      disabled={disabled}
      onMouseDown={(e) => e.preventDefault()}
      onClick={onClick}
    >
      {children}
    </button>
  );
}

function Sep() {
  return <span className="ed-tb-sep" aria-hidden="true" />;
}

export function EditorToolbar({
  editor,
  zoom,
  onZoomIn,
  onZoomOut,
  onZoomReset,
}: {
  editor: Editor | null;
  /** 编辑预览区当前缩放比例（1 = 100%）。 */
  zoom: number;
  onZoomIn: () => void;
  onZoomOut: () => void;
  onZoomReset: () => void;
}) {
  // 订阅编辑器事务 → 刷新按钮 active 态（选区/格式变化即时反映）。
  const [, tick] = useReducer((n: number) => n + 1, 0);
  useEffect(() => {
    if (!editor) return;
    const cb = () => tick();
    editor.on("transaction", cb);
    return () => {
      editor.off("transaction", cb);
    };
  }, [editor]);

  if (!editor) return null;
  const chain = () => editor.chain().focus();

  // 当前块类型（段落/标题级别）。
  const blockValue = editor.isActive("heading", { level: 1 })
    ? "h1"
    : editor.isActive("heading", { level: 2 })
      ? "h2"
      : editor.isActive("heading", { level: 3 })
        ? "h3"
        : "p";
  const onBlockChange = (v: string) => {
    if (v === "p") chain().setParagraph().run();
    else chain().toggleHeading({ level: Number(v.slice(1)) as 1 | 2 | 3 }).run();
  };

  // 当前字号（textStyle.fontSize，如 "16px"）。
  const rawSize = (editor.getAttributes("textStyle").fontSize as string | undefined) ?? "";
  const curSize = rawSize.replace("px", "");
  const onSizeChange = (v: string) => {
    if (!v) chain().unsetFontSize().run();
    else chain().setFontSize(`${v}px`).run();
  };

  const promptLink = () => {
    const prev = (editor.getAttributes("link").href as string | undefined) ?? "";
    const url = window.prompt("链接地址（留空移除）", prev);
    if (url === null) return; // 取消
    if (url === "") chain().extendMarkRange("link").unsetLink().run();
    else chain().extendMarkRange("link").setLink({ href: url }).run();
  };
  const promptImage = () => {
    const url = window.prompt("图片地址（URL）");
    if (url) chain().setImage({ src: url }).run();
  };

  return (
    <div className="ed-toolbar" role="toolbar" aria-label="编辑工具栏">
      <TbBtn title="撤销 ⌘Z" onClick={() => chain().undo().run()} disabled={!editor.can().undo()}>
        ↶
      </TbBtn>
      <TbBtn title="重做 ⇧⌘Z" onClick={() => chain().redo().run()} disabled={!editor.can().redo()}>
        ↷
      </TbBtn>
      <Sep />

      <select
        className="ed-tb-select"
        title="段落 / 标题"
        value={blockValue}
        onChange={(e) => onBlockChange(e.target.value)}
      >
        <option value="p">正文</option>
        <option value="h1">标题 1</option>
        <option value="h2">标题 2</option>
        <option value="h3">标题 3</option>
      </select>
      <select
        className="ed-tb-select"
        title="字号"
        value={curSize}
        onChange={(e) => onSizeChange(e.target.value)}
      >
        <option value="">字号</option>
        {FONT_SIZES.map((s) => (
          <option key={s} value={s}>
            {s}
          </option>
        ))}
      </select>
      <Sep />

      <TbBtn title="加粗 ⌘B" active={editor.isActive("bold")} onClick={() => chain().toggleBold().run()}>
        <b>B</b>
      </TbBtn>
      <TbBtn title="斜体 ⌘I" active={editor.isActive("italic")} onClick={() => chain().toggleItalic().run()}>
        <i>I</i>
      </TbBtn>
      <TbBtn title="下划线 ⌘U" active={editor.isActive("underline")} onClick={() => chain().toggleUnderline().run()}>
        <u>U</u>
      </TbBtn>
      <TbBtn title="删除线" active={editor.isActive("strike")} onClick={() => chain().toggleStrike().run()}>
        <s>S</s>
      </TbBtn>
      <TbBtn title="上标" active={editor.isActive("superscript")} onClick={() => chain().toggleSuperscript().run()}>
        x²
      </TbBtn>
      <TbBtn title="下标" active={editor.isActive("subscript")} onClick={() => chain().toggleSubscript().run()}>
        x₂
      </TbBtn>
      <Sep />

      <label className="ed-tb-color" title="文字颜色">
        <span aria-hidden="true">A</span>
        <input
          type="color"
          value={(editor.getAttributes("textStyle").color as string | undefined) ?? "#000000"}
          onChange={(e) => chain().setColor(e.target.value).run()}
        />
      </label>
      <TbBtn
        title="高亮"
        active={editor.isActive("highlight")}
        onClick={() => chain().toggleHighlight({ color: HIGHLIGHT_COLOR }).run()}
      >
        ✏︎
      </TbBtn>
      <Sep />

      <TbBtn title="无序列表" active={editor.isActive("bulletList")} onClick={() => chain().toggleBulletList().run()}>
        •
      </TbBtn>
      <TbBtn title="有序列表" active={editor.isActive("orderedList")} onClick={() => chain().toggleOrderedList().run()}>
        1.
      </TbBtn>
      <TbBtn title="引用" active={editor.isActive("blockquote")} onClick={() => chain().toggleBlockquote().run()}>
        ❝
      </TbBtn>
      <TbBtn title="代码块" active={editor.isActive("codeBlock")} onClick={() => chain().toggleCodeBlock().run()}>
        {"</>"}
      </TbBtn>
      <Sep />

      <TbBtn title="左对齐" active={editor.isActive({ textAlign: "left" })} onClick={() => chain().setTextAlign("left").run()}>
        ⇤
      </TbBtn>
      <TbBtn title="居中" active={editor.isActive({ textAlign: "center" })} onClick={() => chain().setTextAlign("center").run()}>
        ↔
      </TbBtn>
      <TbBtn title="右对齐" active={editor.isActive({ textAlign: "right" })} onClick={() => chain().setTextAlign("right").run()}>
        ⇥
      </TbBtn>
      <TbBtn title="两端对齐" active={editor.isActive({ textAlign: "justify" })} onClick={() => chain().setTextAlign("justify").run()}>
        ☰
      </TbBtn>
      <Sep />

      <TbBtn title="链接" active={editor.isActive("link")} onClick={promptLink}>
        🔗
      </TbBtn>
      <TbBtn title="插入图片" onClick={promptImage}>
        🖼
      </TbBtn>
      <TbBtn
        title="插入表格（3×3）"
        onClick={() => chain().insertTable({ rows: 3, cols: 3, withHeaderRow: true }).run()}
      >
        ▦
      </TbBtn>

      {/* 缩放：编辑预览区视觉缩放，比例显示于中间（点击复位 100%）——纯呈现层，不改文档模型/不产脏态。 */}
      <span className="ed-tb-spacer" aria-hidden="true" />
      <TbBtn title="缩小" onClick={onZoomOut} disabled={zoom <= ZOOM.min}>
        −
      </TbBtn>
      <button
        type="button"
        className="ed-tb-zoom"
        title="重置缩放（100%）"
        aria-label={`当前缩放 ${formatZoom(zoom)}，点击重置为 100%`}
        onMouseDown={(e) => e.preventDefault()}
        onClick={onZoomReset}
      >
        {formatZoom(zoom)}
      </button>
      <TbBtn title="放大" onClick={onZoomIn} disabled={zoom >= ZOOM.max}>
        +
      </TbBtn>
    </div>
  );
}
