import { useEffect, useReducer } from "react";
import type { Editor } from "@tiptap/react";
import { ZOOM, formatZoom } from "./zoom";
import type { PaperKind, PaperOrientation } from "../../store/edViewPrefsStore";
import type { EditFormat } from "../../store/edStore";
import { activeTableSnapshot } from "../../services/edTableOps";
import { normalizeTableCellShading } from "./extensions/TableCellShading";

const PAPER_KINDS: PaperKind[] = ["A4", "A3", "A5", "Letter"];

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
  paper,
  orientation,
  onPaperChange,
  onOrientationChange,
  localImageDropOnly = false,
  editFormat,
  sourceView,
  onToggleSourceView,
}: {
  editor: Editor | null;
  /** 编辑预览区当前缩放比例（1 = 100%）。 */
  zoom: number;
  onZoomIn: () => void;
  onZoomOut: () => void;
  onZoomReset: () => void;
  /** 纸张选择（PI-ED-06，仅分页文档显示；undefined = 不显示纸张控件）。 */
  paper?: PaperKind;
  orientation?: PaperOrientation;
  onPaperChange?: (p: PaperKind) => void;
  onOrientationChange?: (o: PaperOrientation) => void;
  /** BDocx 图片必须经本地受控导入，不能把远程 URL 直接写入持久文档。 */
  localImageDropOnly?: boolean;
  /** 编辑器格式：markdown 模式下禁用不支持 Markdown 序列化的样式按钮。 */
  editFormat?: EditFormat;
  /** Markdown 文档源码视图开关状态。 */
  sourceView?: boolean;
  /** Markdown 文档切换源码/富文本视图回调。 */
  onToggleSourceView?: () => void;
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
  const isMarkdownMode = editFormat === "markdown";
  const inTable = editor.isActive("table");
  const activeTable = inTable ? activeTableSnapshot() : null;
  const supportsTable = (capability: NonNullable<typeof activeTable>["capabilities"][number]) => Boolean(activeTable?.capabilities.includes(capability));
  const activeCellShading = normalizeTableCellShading(
    editor.getAttributes(editor.isActive("tableHeader") ? "tableHeader" : "tableCell").backgroundColor,
  );

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

  // 当前字号保留来源单位；工具栏新设定使用 pt，与 DOCX 字号语义一致。
  const rawSize = (editor.getAttributes("textStyle").fontSize as string | undefined) ?? "";
  const curSize = rawSize ? String(parseFloat(rawSize)) : "";
  const onSizeChange = (v: string) => {
    if (!v) chain().unsetFontSize().run();
    else chain().setFontSize(`${v}pt`).run();
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
    // 两排固定分组（PI-ED-07 #11）：行1=历史/块型/字号/文本样式，行2=块级/插入/纸张/缩放；
    // 窄窗口时各行尾部溢出可横向滚出（滚动条隐藏）。
    <div className="ed-toolbar" role="toolbar" aria-label="编辑工具栏">
      {inTable && (
        <div className="ed-table-toolbar" role="group" aria-label="表格操作">
          <span className="ed-table-toolbar-label">表格</span>
          <span className="ed-table-toolbar-hint">拖拽列边界可调整列宽</span>
          <TbBtn title="上方插行" onClick={() => chain().addRowBefore().run()}>⤒行</TbBtn>
          <TbBtn title="下方插行" onClick={() => chain().addRowAfter().run()}>⤓行</TbBtn>
          <TbBtn title="左侧插列" onClick={() => chain().addColumnBefore().run()}>⇤列</TbBtn>
          <TbBtn title="右侧插列" onClick={() => chain().addColumnAfter().run()}>⇥列</TbBtn>
          <TbBtn title="删除行" disabled={!supportsTable("delete_row")} onClick={() => chain().deleteRow().run()}>−行</TbBtn>
          <TbBtn title="删除列" disabled={!supportsTable("delete_column")} onClick={() => chain().deleteColumn().run()}>−列</TbBtn>
          <TbBtn title="表头行切换" onClick={() => chain().toggleHeaderRow().run()}>Ⓗ</TbBtn>
          <TbBtn title="合并选中单元格（先拖选矩形区域）" disabled={!editor.can().mergeCells()} onClick={() => chain().mergeCells().run()}>合并</TbBtn>
          <TbBtn title="拆分当前合并单元格" disabled={!editor.can().splitCell()} onClick={() => chain().splitCell().run()}>拆分</TbBtn>
          <label className="ed-table-shading" title="设置所选单元格底纹">
            <span>底纹</span>
            <input
              type="color"
              aria-label="单元格底纹颜色"
              value={activeCellShading ?? "#FFF2CC"}
              onChange={(event) => chain().setCellAttribute("backgroundColor", event.target.value.toUpperCase()).run()}
            />
          </label>
          <TbBtn
            title="清除所选单元格底纹"
            onClick={() => chain().setCellAttribute("backgroundColor", null).run()}
          >
            清底纹
          </TbBtn>
        </div>
      )}
      <div className="ed-tb-row">
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
        title={isMarkdownMode ? "Markdown 模式不支持字号修改" : "字号"}
        value={curSize}
        disabled={isMarkdownMode}
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
      <TbBtn title={isMarkdownMode ? "Markdown 不支持下划线" : "下划线 ⌘U"} active={editor.isActive("underline")} disabled={isMarkdownMode} onClick={() => chain().toggleUnderline().run()}>
        <u>U</u>
      </TbBtn>
      <TbBtn title="删除线" active={editor.isActive("strike")} onClick={() => chain().toggleStrike().run()}>
        <s>S</s>
      </TbBtn>
      <TbBtn title={isMarkdownMode ? "Markdown 不支持上标" : "上标"} active={editor.isActive("superscript")} disabled={isMarkdownMode} onClick={() => chain().toggleSuperscript().run()}>
        x²
      </TbBtn>
      <TbBtn title={isMarkdownMode ? "Markdown 不支持下标" : "下标"} active={editor.isActive("subscript")} disabled={isMarkdownMode} onClick={() => chain().toggleSubscript().run()}>
        x₂
      </TbBtn>
      <Sep />

      {!isMarkdownMode && (
        <>
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
        </>
      )}
      {isMarkdownMode && <span className="ed-tb-hint" title="文字颜色和高亮不支持 Markdown 格式">MD</span>}
      </div>

      <div className="ed-tb-row">
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

      <TbBtn title={isMarkdownMode ? "Markdown 不支持左对齐" : "左对齐"} active={editor.isActive({ textAlign: "left" })} disabled={isMarkdownMode} onClick={() => chain().setTextAlign("left").run()}>
        ⇤
      </TbBtn>
      <TbBtn title={isMarkdownMode ? "Markdown 不支持居中" : "居中"} active={editor.isActive({ textAlign: "center" })} disabled={isMarkdownMode} onClick={() => chain().setTextAlign("center").run()}>
        ↔
      </TbBtn>
      <TbBtn title={isMarkdownMode ? "Markdown 不支持右对齐" : "右对齐"} active={editor.isActive({ textAlign: "right" })} disabled={isMarkdownMode} onClick={() => chain().setTextAlign("right").run()}>
        ⇥
      </TbBtn>
      <TbBtn title={isMarkdownMode ? "Markdown 不支持两端对齐" : "两端对齐"} active={editor.isActive({ textAlign: "justify" })} disabled={isMarkdownMode} onClick={() => chain().setTextAlign("justify").run()}>
        ☰
      </TbBtn>
      <Sep />

      <TbBtn title="链接" active={editor.isActive("link")} onClick={promptLink}>
        🔗
      </TbBtn>
      <TbBtn
        title={localImageDropOnly ? "请将本地 PNG/JPEG 图片拖入文档" : "插入图片"}
        disabled={localImageDropOnly}
        onClick={promptImage}
      >
        🖼
      </TbBtn>
      <TbBtn
        title="插入表格（3×3）"
        onClick={() => chain().insertTable({ rows: 3, cols: 3, withHeaderRow: true }).run()}
      >
        ▦
      </TbBtn>

      <span className="ed-tb-spacer" aria-hidden="true" />

      {/* 纸张选择（PI-ED-06，分页文档）：视图偏好，不落盘文档（BR-ED-DATA-003）。 */}
      {paper !== undefined && (
        <>
          <select
            className="ed-tb-select"
            title="纸张"
            value={paper}
            onChange={(e) => onPaperChange?.(e.target.value as PaperKind)}
          >
            {PAPER_KINDS.map((k) => (
              <option key={k} value={k}>
                {k}
              </option>
            ))}
          </select>
          <TbBtn
            title={orientation === "landscape" ? "切换为竖排" : "切换为横排"}
            active={orientation === "landscape"}
            onClick={() => onOrientationChange?.(orientation === "landscape" ? "portrait" : "landscape")}
          >
            {orientation === "landscape" ? "▭" : "▯"}
          </TbBtn>
          <Sep />
        </>
      )}

      {/* 缩放：编辑预览区视觉缩放，比例显示于中间（点击复位 100%）——纯呈现层，不改文档模型/不产脏态。 */}
      {isMarkdownMode && onToggleSourceView && (
        <>
          <TbBtn
            title={sourceView ? "切换到富文本视图" : "切换到 Markdown 源码视图"}
            active={sourceView}
            onClick={onToggleSourceView}
          >
            {sourceView ? "WYSIWYG" : "MD"}
          </TbBtn>
          <Sep />
        </>
      )}
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
    </div>
  );
}
