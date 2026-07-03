import StarterKit from "@tiptap/starter-kit";
import type { Extensions } from "@tiptap/core";
import { Link } from "@tiptap/extension-link";
import { ResizableImage } from "./ResizableImage";
import { TextStyle } from "@tiptap/extension-text-style";
import { Color } from "@tiptap/extension-color";
import { FontFamily } from "@tiptap/extension-font-family";
import { TextAlign } from "@tiptap/extension-text-align";
import { Underline } from "@tiptap/extension-underline";
import { Subscript } from "@tiptap/extension-subscript";
import { Superscript } from "@tiptap/extension-superscript";
import { Highlight } from "@tiptap/extension-highlight";
import { Table } from "@tiptap/extension-table";
import { TableRow } from "@tiptap/extension-table-row";
import { TableHeader } from "@tiptap/extension-table-header";
import { TableCell } from "@tiptap/extension-table-cell";
import { CharacterCount } from "@tiptap/extension-character-count";
import { BlockIdExtension } from "./BlockIdExtension";
import { DiffDecorationExtension } from "./DiffDecorationExtension";
import { PaginationExtension } from "./Pagination";
import { FontSize } from "./FontSize";

/**
 * ED 编辑器扩展注册入口（PI-FE-05）。统一组装 TipTap 扩展集，固定初始化顺序。
 *
 * docx 编辑链（A-DOC-I-P-01 §2）：docx → HTML（docx+styles + preserve-styles.lua）的样式子集需对应扩展
 * 解析为富文本「成熟文本块」，否则内联 style/颜色/字号/表格在编辑器显示为生标记。三级子集 ↔ 扩展：
 *  - 必需子集（往返保真）：标题/段落对齐(TextAlign)/粗斜下划线(StarterKit+Underline)/有序无序列表(StarterKit)
 *    /基础表格(Table*)/嵌入图片(Image)；
 *  - 部分支持（保留并明示）：文本颜色(TextStyle+Color)/字体(FontFamily)/字号(FontSize)；
 *  - 增强：高亮/上下标(Highlight/Sub/Superscript)。
 */
export function buildEditorExtensions(): Extensions {
  return [
    // 基础富文本；禁用 v3 StarterKit 自带 link/underline，改用下方独立配置（避免重复扩展报错）。
    StarterKit.configure({ link: false, underline: false }),
    // tiptap-markdown 已移除（2026-07-03 人类裁定 B）：md 按纯文本忠实编辑（同 txt），不做 markdown
    // 富文本渲染——根除缩进变代码块与保存转义注入；docx/.html 富文本走 generateJSON HTML 路径。
    // 图片（docx 嵌入图，允许 base64/inline）；PI-ED-08：拖拽调宽 + inline/block 嵌入切换（NodeView）。
    ResizableImage.configure({ inline: true, allowBase64: true, HTMLAttributes: { class: "editor-image" } }),
    Link.configure({ openOnClick: false }),
    // 文本样式族（顺序：TextStyle 先于依赖它的 Color/FontFamily/FontSize）。
    TextStyle,
    Color,
    FontFamily,
    FontSize,
    TextAlign.configure({ types: ["heading", "paragraph"] }),
    Underline,
    Subscript,
    Superscript,
    Highlight.configure({ multicolor: true }),
    // 表格族（docx 基础表格，必需子集）。
    Table.configure({ resizable: true }),
    TableRow,
    TableHeader,
    TableCell,
    // 稳定 blockId（BR-ED-DATA-001）——StarterKit 之后，为块类型附全局属性。
    BlockIdExtension,
    // GreenReview 装饰层（BR-ED-STATE-002 / TERM-ED-003）——依赖 blockId 锚定。
    DiffDecorationExtension,
    // 分页呈现层（BR-ED-DATA-003）——按块高在块边界叠加分页缝 widget；仅 docx 编辑默认开（Editor 切换）。
    PaginationExtension,
    // 字数统计（PI-ED-07 状态栏：中文按字符计）。
    CharacterCount,
  ];
}
