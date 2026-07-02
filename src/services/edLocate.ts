import type { Editor } from "@tiptap/react";
import type { Node as PMNode } from "@tiptap/pm/model";

/**
 * ed.locate_block / ed.get_cursor_ref / ed.get_selection_ref（PI-AG-08H · PI-ED-10 · A-ED-I-P-01 §4）。
 *
 * 块级锚（BR-ED-DATA-001）+ 字符锚（CharAnchor，BR-ED-DATA-004）：blockId + 块内纯文本偏移
 * {start,end}——偏移定义在**块内纯文本平面**（textBetween 语义，跨 md/docx/html/纯文本统一，
 * 叶子内联节点以 1 字符占位与 PM pos 对齐）。锚有效期 = 编辑会话内（重开必换锚，PI-ED-09 实测）。
 * 光标/选区经**按需查询**读取（非常驻投影——选区高频变化，常驻投影引发重渲染风暴且非业务态）。
 * 精确计算经注册的编辑器实例（ProseMirror state）；无实例时选区查询回落 DOM 近似（块级，无偏移）。
 */

export type LocateResult = "located" | "anchor_lost";

// 编辑器实例注册点（Editor 挂载时注册、卸载置空）；查询函数按需读取。
let activeEditor: Editor | null = null;

/** 注册当前活动编辑器实例（Editor.tsx onCreate/卸载调用）。 */
export function registerEditor(editor: Editor | null): void {
  activeEditor = editor;
}

/** 叶子内联节点占位符（textBetween leafText）：1 字符对齐 PM pos 中 leaf nodeSize=1。 */
const LEAF_CHAR = "￼";

/** 块内纯文本偏移（BR-ED-DATA-004 偏移平面）：块内容位置 → 纯文本偏移。 */
export function charOffsetInBlock(block: PMNode, posInContent: number): number {
  const clamped = Math.max(0, Math.min(posInContent, block.content.size));
  return block.textBetween(0, clamped, undefined, LEAF_CHAR).length;
}

/** 块的纯文本总长（偏移上界）。 */
export function blockTextLength(block: PMNode): number {
  return block.textBetween(0, block.content.size, undefined, LEAF_CHAR).length;
}

export interface SelectionRef {
  /** 引用来源文档（相对路径；无活动文档时为空串）。 */
  path: string;
  blockId: string;
  /** 块内纯文本偏移区间（CharAnchor）；DOM 回落时无偏移（0..0 表示仅块级锚）。 */
  start: number;
  end: number;
  text: string;
}

export interface CursorRef {
  blockId: string;
  offset: number;
}

/** 文档顶层块定位辅助：返回 pos 所在顶层块 {node, blockPos}；不在任何块内 → null。 */
function topBlockAt(doc: PMNode, pos: number): { node: PMNode; blockPos: number } | null {
  let p = 0;
  for (let i = 0; i < doc.childCount; i++) {
    const node = doc.child(i);
    if (pos >= p && pos <= p + node.nodeSize) return { node, blockPos: p };
    p += node.nodeSize;
  }
  return null;
}

/**
 * 选区 → 字符锚纯逻辑（BR-ED-DATA-004 核心，供查询接口与单测共用）。
 * 单块选区：{blockId, start, end}（块内纯文本偏移）；**跨块选区锚定首块**（start..首块文本尾，
 * text 仍为完整选区文本）——多块锚列表为显式延后债务（A-ED-X-L-02）。折叠/无 blockId 块 → null。
 */
export function resolveSelectionAnchor(
  doc: PMNode,
  from: number,
  to: number,
): { blockId: string; start: number; end: number } | null {
  if (from >= to) return null;
  const hit = topBlockAt(doc, from);
  if (!hit) return null;
  const blockId = (hit.node.attrs.blockId as string | null) ?? "";
  if (!blockId) return null;
  const contentStart = hit.blockPos + 1;
  const blockEnd = hit.blockPos + hit.node.nodeSize - 1; // 块内容终点
  const start = charOffsetInBlock(hit.node, from - contentStart);
  const end =
    to <= blockEnd
      ? charOffsetInBlock(hit.node, to - contentStart)
      : blockTextLength(hit.node); // 跨块：截首块文本尾
  return { blockId, start, end };
}

/**
 * @GOV
 * codes:    BR-ED-DATA-004-QUERY-ED-ED-EDIT-007
 * type:     QUERY
 * chain:    ED-EDIT
 * rules:    BR-ED-DATA-004
 * term_ref: TERM-ED-004, TERM-ED-005
 * boundary: in=editorSelectionState | out=SelectionRefOrCursorRef(blockIdWithInBlockPlainTextOffsets)
 */
/// 字符锚/选区引用查询守则（BR-ED-DATA-004）：偏移在块内纯文本平面（跨格式统一）；按需查询非常驻
/// 投影；锚有效期=编辑会话内（重开必换锚）；无实例/折叠选区/无 blockId → null 或 DOM 块级回落，
/// 不产生错位锚。定位失败（块删除/偏移越界）回 anchor_lost 明确信号（locateBlock），不静默错位。

/** 光标位置查询（ed.get_cursor_ref）：光标所在块 + 块内纯文本偏移；无实例/无光标块锚 → null。 */
export function getCursorRef(): CursorRef | null {
  const ed = activeEditor;
  if (!ed) return null;
  const { head } = ed.state.selection;
  const hit = topBlockAt(ed.state.doc, head);
  if (!hit) return null;
  const blockId = (hit.node.attrs.blockId as string | null) ?? "";
  if (!blockId) return null;
  return { blockId, offset: charOffsetInBlock(hit.node, head - (hit.blockPos + 1)) };
}

/** 选区引用查询（ed.get_selection_ref）：SelectionRef 含字符锚 + 选区文本；折叠选区 → null。
 *  activePath 由调用方业务态传入会破坏"按需自足"，此处从注册实例反查不可得 → 由 captureSelectionRef
 *  的消费方（Composer）补 path；本函数回 path 空串占位。 */
export function getSelectionRef(): SelectionRef | null {
  const ed = activeEditor;
  if (!ed) return domSelectionFallback();
  const { from, to } = ed.state.selection;
  const anchor = resolveSelectionAnchor(ed.state.doc, from, to);
  if (!anchor) return null;
  const text = ed.state.doc.textBetween(from, to, "\n", LEAF_CHAR).trim();
  if (!text) return null;
  return { path: "", ...anchor, text };
}

/** DOM 块级回落（无编辑器实例时）：仅 blockId + 文本，无偏移（start=end=0 表示块级锚）。 */
function domSelectionFallback(): SelectionRef | null {
  if (typeof window === "undefined") return null;
  const sel = window.getSelection();
  if (!sel || sel.isCollapsed || sel.rangeCount === 0) return null;
  const text = sel.toString().trim();
  if (!text) return null;
  let node: Node | null = sel.anchorNode;
  let el: Element | null = node && node.nodeType === 1 ? (node as Element) : (node?.parentElement ?? null);
  while (el && !el.getAttribute("data-block-id")) el = el.parentElement;
  const blockId = el?.getAttribute("data-block-id") ?? "";
  if (!blockId) return null;
  return { path: "", blockId, start: 0, end: 0, text };
}

/**
 * 捕获当前编辑器选区 → 引用（PI-AG-09F / PI-ED-10 升级）：优先 PM 精确字符锚，回落 DOM 块级。
 * 保留旧函数名兼容既有消费方（Composer）。
 */
export function captureSelectionRef(): SelectionRef | null {
  return getSelectionRef();
}

/** 在当前编辑器实时定位块（可选字符锚区间校验，BR-ED-DATA-001/004）：
 *  块不存在 → anchor_lost；带区间且 end 超块文本长（偏移越界）→ anchor_lost。 */
export function locateBlock(
  blockId: string | undefined | null,
  range?: { start: number; end: number },
): LocateResult {
  if (!blockId) return "anchor_lost";
  const ed = activeEditor;
  if (ed) {
    let found: PMNode | null = null;
    ed.state.doc.forEach((node) => {
      if (!found && node.attrs.blockId === blockId) found = node;
    });
    if (!found) return "anchor_lost";
    if (range && (range.start < 0 || range.end < range.start || range.end > blockTextLength(found))) {
      return "anchor_lost";
    }
    return "located";
  }
  // DOM 回落（无实例）：仅块级存在性。
  if (typeof document === "undefined") return "anchor_lost";
  const sel =
    typeof CSS !== "undefined" && CSS.escape
      ? `[data-block-id="${CSS.escape(blockId)}"]`
      : `[data-block-id="${blockId.replace(/"/g, '\\"')}"]`;
  return document.querySelector(sel) ? "located" : "anchor_lost";
}

/** 选区拖拽引用的自定义 MIME（PI-ED-10 #6：拖出编辑器 → AG 输入面位置引用）。 */
export const SELECTION_REF_MIME = "application/x-binder-selection-ref";
