import StarterKit from "@tiptap/starter-kit";
import { Extension, type Extensions } from "@tiptap/core";
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
import { TableMap } from "@tiptap/pm/tables";
import { Plugin, PluginKey, type EditorState } from "@tiptap/pm/state";
import { Decoration, DecorationSet, type EditorView } from "@tiptap/pm/view";
import { CharacterCount } from "@tiptap/extension-character-count";
import { TaskItem, TaskList } from "@tiptap/extension-list";
import { Markdown } from "@tiptap/markdown";
import { BlockIdExtension } from "./BlockIdExtension";
import { TableIdExtension } from "./TableIdExtension";
import { DiffDecorationExtension } from "./DiffDecorationExtension";
import { PaginationExtension } from "./Pagination";
import { FontSize } from "./FontSize";
import { TableCellShading } from "./TableCellShading";
import { ImageIdExtension } from "./ImageIdExtension";

const tableResizeAffordanceKey = new PluginKey("binderTableResizeAffordance");

/** 当前选区所属表格；只为人工正在操作的表显示持久列边界，不干扰非激活表格。 */
function selectedTable(state: EditorState) {
  const $from = state.selection.$from;
  for (let depth = $from.depth; depth > 0; depth -= 1) {
    const node = $from.node(depth);
    if (node.type.name === "table") return { node, pos: $from.before(depth) };
  }
  return null;
}

/**
 * WebKit 对 prosemirror-tables 的隐式悬停及窗口级 pointermove 监听均不稳定。
 * 本扩展在当前表格所有可调列边界放置显式 widget，并由该 widget通过文档捕获阶段处理鼠标拖动，
 * 写入同构的 colwidth 事务；不依赖浏览器是否继续向原插件派发移动事件。
 */
function editorZoom(view: EditorView): number {
  const zoom = Number.parseFloat(window.getComputedStyle(view.dom).zoom);
  return Number.isFinite(zoom) && zoom > 0 ? zoom : 1;
}

function measuredColumnWidth(view: EditorView, cellPos: number): number | null {
  const $cell = view.state.doc.resolve(cellPos);
  const cell = $cell.nodeAfter;
  if (!cell) return null;
  const widths = Array.isArray(cell.attrs.colwidth) ? cell.attrs.colwidth : null;
  const explicit = widths?.[widths.length - 1];
  if (typeof explicit === "number" && explicit > 0) return explicit;
  const dom = view.nodeDOM(cellPos) as HTMLElement | null;
  if (!dom) return null;
  const total = dom.getBoundingClientRect().width / editorZoom(view);
  const known = (widths ?? []).reduce<number>((sum, width) => sum + (typeof width === "number" && width > 0 ? width : 0), 0);
  const unknown = Math.max(1, Number(cell.attrs.colspan ?? 1) - (widths ?? []).filter((width) => typeof width === "number" && width > 0).length);
  return Math.max(25, (total - known) / unknown);
}

/** 复刻 prosemirror-tables 的列宽属性更新，只修改当前边界所对应的逻辑列。 */
function writeColumnWidth(view: EditorView, cellPos: number, width: number): void {
  const $cell = view.state.doc.resolve(cellPos);
  const cell = $cell.nodeAfter;
  const table = $cell.node(-1);
  if (!cell || table.type.name !== "table") return;
  const start = $cell.start(-1);
  const map = TableMap.get(table);
  const column = map.colCount($cell.pos - start) + Number(cell.attrs.colspan ?? 1) - 1;
  const tr = view.state.tr;
  for (let row = 0; row < map.height; row += 1) {
    const mapIndex = row * map.width + column;
    if (row > 0 && map.map[mapIndex] === map.map[mapIndex - map.width]) continue;
    const offset = map.map[mapIndex];
    const target = table.nodeAt(offset);
    if (!target) continue;
    const targetIndex = Number(target.attrs.colspan ?? 1) === 1 ? 0 : column - map.colCount(offset);
    const widths = Array.isArray(target.attrs.colwidth) ? [...target.attrs.colwidth] : Array(Number(target.attrs.colspan ?? 1)).fill(0);
    if (widths[targetIndex] === width) continue;
    widths[targetIndex] = width;
    tr.setNodeMarkup(start + offset, undefined, { ...target.attrs, colwidth: widths });
  }
  if (tr.docChanged) view.dispatch(tr);
}

const TableResizeAffordance = Extension.create({
  name: "tableResizeAffordance",

  addProseMirrorPlugins() {
    let editorView: EditorView | null = null;
    return [new Plugin({
      key: tableResizeAffordanceKey,
      view(view) {
        editorView = view;
        return { destroy: () => { editorView = null; } };
      },
      props: {
        decorations(state) {
          const table = selectedTable(state);
          if (!table) return DecorationSet.empty;
          const map = TableMap.get(table.node);
          const decorations: Decoration[] = [];
          for (let column = 0; column < map.width; column += 1) {
            for (let row = 0; row < map.height; row += 1) {
              const index = row * map.width + column;
              const cellOffset = map.map[index];
              // 仅为该列右边界的起始单元格放一个句柄，合并单元格不重复生成。
              if (
                (column < map.width - 1 && map.map[index] === map.map[index + 1]) ||
                (row > 0 && map.map[index] === map.map[index - map.width])
              ) continue;
              const cell = table.node.nodeAt(cellOffset);
              if (!cell) continue;
              const cellPos = table.pos + 1 + cellOffset;
              const widgetPos = cellPos + cell.nodeSize - 1;
              decorations.push(Decoration.widget(widgetPos, () => {
                const handle = document.createElement("span");
                handle.className = "bdocx-column-resize-handle";
                handle.setAttribute("contenteditable", "false");
                handle.setAttribute("role", "separator");
                handle.setAttribute("aria-label", `拖拽调整第 ${column + 1} 列宽度`);
                handle.addEventListener("mousedown", (event) => {
                  if (!editorView) return;
                  const view = editorView;
                  const startWidth = measuredColumnWidth(view, cellPos);
                  if (startWidth === null) return;
                  event.preventDefault();
                  event.stopPropagation();
                  const startX = event.clientX;
                  const move = (moveEvent: MouseEvent) => {
                    moveEvent.preventDefault();
                    const width = Math.max(25, Math.round(startWidth + (moveEvent.clientX - startX) / editorZoom(view)));
                    writeColumnWidth(view, cellPos, width);
                  };
                  const finish = () => {
                    document.removeEventListener("mousemove", move, true);
                    document.removeEventListener("mouseup", finish, true);
                  };
                  // 捕获阶段监听不依赖 WebKit 是否继续向原始 widget/窗口派发 pointermove，
                  // 也不读取已被 WebKit 弃用且可能恒为 0 的 MouseEvent.which。
                  document.addEventListener("mousemove", move, true);
                  document.addEventListener("mouseup", finish, true);
                });
                return handle;
              }, { side: 1 }));
            }
          }
          return DecorationSet.create(state.doc, decorations);
        },
      },
    })];
  },
});

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
    // 官方 TipTap Markdown 与当前 3.27.1 扩展栈对齐；缩进遵循 CommonMark（四空格/Tab 为代码块）。
    // Markdown 保存前仍须经过 markdownCodec 的能力与语义往返校验，不能直接把 getMarkdown() 写盘。
    Markdown.configure({ indentation: { style: "space", size: 2 }, markedOptions: { gfm: true } }),
    TaskList,
    TaskItem.configure({ nested: true }),
    // 图片（docx 嵌入图，允许 base64/inline）；PI-ED-08：拖拽调宽 + inline/block 嵌入切换（NodeView）。
    ResizableImage.configure({ inline: true, allowBase64: true, HTMLAttributes: { class: "editor-image" } }),
    // ImageOperation 会话锚；宽度、顺序与对齐变化只更新 fingerprint，不改变 imageId。
    ImageIdExtension,
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
    // 列宽只支持直接拖拽：放大热区以保证可发现性；数值输入、均分列和行高不在本期范围。
    Table.configure({ resizable: true, handleWidth: 8, lastColumnResizable: true }),
    TableRow,
    TableHeader,
    TableCell,
    // TableCellShading：td/th 的不透明纯色属性，供人工操作、TableOperation 与 DOCX w:shd 往返共用。
    TableCellShading,
    // 常驻列边界 widget：仅在选中表格时显示，以兼容 WebKit 的鼠标事件链写入 colwidth。
    TableResizeAffordance,
    // 表格会话锚：TableOperation / 结构化 Diff 只引用 tableId，不使用可漂移的序号或文本。
    TableIdExtension,
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
