import type { Editor } from "@tiptap/react";
import type { Node as PMNode } from "@tiptap/pm/model";
import { generateJSON } from "@tiptap/core";
import { buildEditorExtensions } from "../components/ed/extensions";
import type { ContentFormat, EditFormat } from "../store/edStore";
import { makeDocReference, makeSelectionReference, writeReferencePayloadData } from "./referencePayload";

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
let activeEditorBinding: ActiveEditorBinding | null = null;

export interface ActiveEditorBinding {
  relPath: string;
  loadSeq: number;
  editFormat: EditFormat;
  contentFormat?: ContentFormat;
}

/** 注册当前活动编辑器实例（Editor.tsx onCreate/卸载调用）。 */
export function registerEditor(editor: Editor | null): void {
  activeEditor = editor;
  activeEditorBinding = null;
}

/** 取当前注册的编辑器实例（进程内受控查询共用；样式协议 edStyleOps 消费）。 */
export function getActiveEditor(): Editor | null {
  return activeEditor;
}

/** 保存结果（结构对齐 edService.SaveOutcome，本文件不反向依赖 edService 避免循环引用）。 */
export interface ActiveSaveOutcome {
  ok: boolean;
  message?: string;
  conflict?: boolean;
}

// 活动编辑器保存处理器注册点（Editor.tsx 挂载时注册其 saveCurrent，卸载置空）。
// 关闭前处理（APP-CLOSE-DIFF-LIFECYCLE-01）复用同一保存入口逐个保存脏文档，不重复实现各格式的
// 保存分支（docx/html_file/markdown/text），避免重复维护同一份格式路由逻辑。
let activeSaveHandler: (() => Promise<ActiveSaveOutcome>) | null = null;

/** 注册当前活动编辑器的保存处理器（Editor.tsx 调用；卸载时传 null 解除注册）。 */
export function registerActiveSaveHandler(fn: (() => Promise<ActiveSaveOutcome>) | null): void {
  activeSaveHandler = fn;
}

/** 触发当前活动编辑器保存（无注册处理器时返回失败，不静默成功）。 */
export async function triggerActiveSave(): Promise<ActiveSaveOutcome> {
  if (!activeSaveHandler) {
    return { ok: false, message: "无活动可保存的编辑器" };
  }
  return activeSaveHandler();
}

export function bindActiveEditorDocument(binding: ActiveEditorBinding): void {
  activeEditorBinding = binding;
}

export function clearActiveEditorBinding(): void {
  activeEditorBinding = null;
}

export function getActiveEditorBinding(): ActiveEditorBinding | null {
  return activeEditorBinding;
}

export function isActiveEditorBoundTo(
  relPath: string | null,
  loadSeq: number,
  editFormat: EditFormat,
  contentFormat?: ContentFormat,
): boolean {
  return Boolean(
    relPath &&
    activeEditorBinding &&
    activeEditorBinding.relPath === relPath &&
    activeEditorBinding.loadSeq === loadSeq &&
    activeEditorBinding.editFormat === editFormat &&
    (contentFormat === undefined || (activeEditorBinding.contentFormat ?? "text") === contentFormat),
  );
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

export interface BlockSnapshot {
  blockId: string | null;
  index: number;
  text: string;
  pos: number;
  size: number;
  contentFrom: number;
  contentTo: number;
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

function findBlockById(doc: PMNode, blockId: string): { node: PMNode; pos: number } | null {
  let hit: { node: PMNode; pos: number } | null = null;
  doc.descendants((node, pos) => {
    if (hit) return false;
    if (node.attrs.blockId === blockId) {
      hit = { node, pos };
      return false;
    }
    return true;
  });
  return hit;
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

/** 按当前块列表的 0 起块号解析稳定 blockId；用于 text/md 的 propose_edit block_index 回连编辑器锚。 */
export function getTopBlockIdByIndex(doc: PMNode, index: number): string | null {
  if (!Number.isInteger(index) || index < 0 || index >= doc.childCount) return null;
  const blockId = (doc.child(index).attrs.blockId as string | null) ?? "";
  return blockId || null;
}

function snapshotTopBlock(node: PMNode, pos: number, index: number): BlockSnapshot {
  const blockId = (node.attrs.blockId as string | null) ?? null;
  return {
    blockId,
    index,
    text: node.textBetween(0, node.content.size, "\n", LEAF_CHAR),
    pos,
    size: node.nodeSize,
    contentFrom: pos + 1,
    contentTo: pos + node.nodeSize - 1,
  };
}

/**
 * @GOV
 * codes:    BR-ED-DATA-004-QUERY-ED-ED-EDIT-010
 * type:     QUERY
 * chain:    ED-EDIT
 * rules:    BR-ED-DATA-004
 * term_ref: TERM-ED-004, TERM-ED-005
 * boundary: in=editorDocumentTree | out=BlockSnapshot(blockIdTextAndCurrentSessionPosition)
 */
/// 当前编辑器块快照查询守则（BR-ED-DATA-004）：按会话内 blockId 或 0 起 block_index 读取目标块
/// 的当前纯文本与 ProseMirror 位置，用于 AG 修改建议和 DE 草稿回滚；查询只读，不修正文档。
export function getTopBlockSnapshotByIndex(doc: PMNode, index: number): BlockSnapshot | null {
  if (!Number.isInteger(index) || index < 0 || index >= doc.childCount) return null;
  let pos = 0;
  for (let i = 0; i < index; i++) pos += doc.child(i).nodeSize;
  return snapshotTopBlock(doc.child(index), pos, index);
}

export function findTopBlockSnapshotById(doc: PMNode, blockId: string): BlockSnapshot | null {
  if (!blockId) return null;
  let pos = 0;
  for (let index = 0; index < doc.childCount; index++) {
    const node = doc.child(index);
    if (node.attrs.blockId === blockId) return snapshotTopBlock(node, pos, index);
    pos += node.nodeSize;
  }
  return null;
}

/** 从当前注册编辑器解析 0 起块号对应的稳定 blockId；无活动编辑器或越界时返回 null。 */
export function getActiveBlockIdByIndex(index: number): string | null {
  const ed = activeEditor;
  if (!ed) return null;
  return getTopBlockIdByIndex(ed.state.doc, index);
}

export function getActiveBlockSnapshotByIndex(index: number): BlockSnapshot | null {
  const ed = activeEditor;
  if (!ed) return null;
  return getTopBlockSnapshotByIndex(ed.state.doc, index);
}

export function getActiveBlockSnapshotById(blockId: string): BlockSnapshot | null {
  const ed = activeEditor;
  if (!ed) return null;
  return findTopBlockSnapshotById(ed.state.doc, blockId);
}

export function getActiveDocumentContent(format: EditFormat, contentFormat?: ContentFormat): string | null {
  const ed = activeEditor;
  if (!ed) return null;
  if (contentFormat === "html" || format === "html" || format === "html_file" || (format === "markdown" && contentFormat !== "text")) {
    return ed.getHTML();
  }
  return ed.getText({ blockSeparator: "\n" });
}

function textToDoc(text: string) {
  return {
    type: "doc",
    content: text.split("\n").map((line) =>
      line ? { type: "paragraph", content: [{ type: "text", text: line }] } : { type: "paragraph" },
    ),
  };
}

export type ApplyEditOp =
  | { op: "replace_text"; blockId: string; start: number; end: number; text: string }
  | { op: "replace_block"; blockId: string; text: string }
  | { op: "replace_document"; content: string; format: EditFormat; contentFormat?: ContentFormat };

export type ApplyEditResult = { ok: true } | { ok: false; reason: string };

function posFromCharOffsetInBlock(block: PMNode, offset: number): number {
  let remaining = Math.max(0, offset);
  let pos = 0;
  let done = false;
  block.content.forEach((child) => {
    if (done) return;
    const len = child.isText ? (child.text?.length ?? 0) : child.isLeaf ? 1 : 0;
    if (remaining <= len) {
      pos += child.isText ? remaining : remaining > 0 ? child.nodeSize : 0;
      done = true;
      return;
    }
    remaining -= len;
    pos += child.nodeSize;
  });
  return pos;
}

/**
 * @GOV
 * codes:    BR-ED-STATE-001-EFFECT-ED-ED-EDIT-012
 * type:     EFFECT
 * chain:    ED-EDIT
 * rules:    BR-ED-STATE-001
 * term_ref: TERM-ED-001, TERM-ED-004
 * boundary: in=applyEditOps(blockOrDocumentScopedEditorMutation) | out=editorDocumentTree(dirtyViaOnUpdateOrAnchorLost)
 */
/// ED 统一编辑入口守则（BR-ED-STATE-001）：上游候选、回滚与显式辅助写入都经当前编辑器实例修改
/// LogicalState；定位失败返回 anchor_lost，不直接写磁盘，不由 DisplayState 反写 LogicalState。
export function applyEditOps(ops: ApplyEditOp[]): ApplyEditResult {
  const ed = activeEditor;
  if (!ed) return { ok: false, reason: "no_active_editor" };
  for (const op of ops) {
    if (op.op === "replace_document") {
      const located = setActiveDocumentContent(op.content, op.format, op.contentFormat);
      if (located !== "located") return { ok: false, reason: "replace_document_failed" };
      continue;
    }
    const found = findBlockById(ed.state.doc, op.blockId);
    if (!found) return { ok: false, reason: "anchor_lost" };
    const contentFrom = found.pos + 1;
    const contentTo = found.pos + found.node.nodeSize - 1;
    if (op.op === "replace_block") {
      ed.chain().focus().insertContentAt({ from: contentFrom, to: contentTo }, op.text).run();
      continue;
    }
    const textLen = blockTextLength(found.node);
    if (op.start < 0 || op.end < op.start || op.end > textLen) {
      return { ok: false, reason: "invalid_range" };
    }
    const from = contentFrom + posFromCharOffsetInBlock(found.node, op.start);
    const to = contentFrom + posFromCharOffsetInBlock(found.node, op.end);
    ed.chain().focus().insertContentAt({ from, to }, op.text).run();
  }
  return { ok: true };
}

/**
 * @GOV
 * codes:    BR-ED-STATE-001-EFFECT-ED-ED-EDIT-011
 * type:     EFFECT
 * chain:    ED-EDIT
 * rules:    BR-ED-STATE-001
 * term_ref: TERM-ED-001
 * boundary: in=blockScopedEditorMutation | out=editorDocumentTree(blockContentReplacedDirtyViaOnUpdate)
 */
/// ED 受控块替换守则（BR-ED-STATE-001）：DE 草稿裁决只能经当前编辑器实例替换目标 Block 内容，
/// 普通事务触发 onUpdate 脏态；不直接写文件、不绕过 WS save。
export function replaceActiveBlockText(blockId: string, text: string): LocateResult {
  return applyEditOps([{ op: "replace_block", blockId, text }]).ok ? "located" : "anchor_lost";
}

export function setActiveDocumentContent(content: string, format: EditFormat, contentFormat?: ContentFormat): LocateResult {
  const ed = activeEditor;
  if (!ed) return "anchor_lost";
  try {
    if (contentFormat === "html" || format === "html" || format === "html_file" || (format === "markdown" && contentFormat !== "text")) {
      ed.commands.setContent(generateJSON(content, buildEditorExtensions()), { emitUpdate: true });
    } else {
      ed.commands.setContent(textToDoc(content), { emitUpdate: true });
    }
    return "located";
  } catch {
    return "anchor_lost";
  }
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
    const found = findBlockById(ed.state.doc, blockId);
    if (!found) return "anchor_lost";
    if (range && (range.start < 0 || range.end < range.start || range.end > blockTextLength(found.node))) {
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

/** 定位并滚动到当前编辑器中的块；只移动视图/光标，不改文档内容。 */
export function revealBlock(blockId: string | undefined | null): LocateResult {
  if (!blockId) return "anchor_lost";
  const ed = activeEditor;
  if (ed) {
    const found = findBlockById(ed.state.doc, blockId);
    if (!found) return "anchor_lost";
    const selectionPos = Math.min(found.pos + 1, ed.state.doc.content.size);
    ed.chain().focus().setTextSelection(selectionPos).scrollIntoView().run();
    return "located";
  }
  if (typeof document === "undefined") return "anchor_lost";
  const sel =
    typeof CSS !== "undefined" && CSS.escape
      ? `[data-block-id="${CSS.escape(blockId)}"]`
      : `[data-block-id="${blockId.replace(/"/g, '\\"')}"]`;
  const el = document.querySelector(sel);
  if (!el) return "anchor_lost";
  el.scrollIntoView({ block: "center" });
  return "located";
}

/** 选区拖拽引用的自定义 MIME（PI-ED-10 #6：拖出编辑器 → AG 输入面位置引用）。 */
export const SELECTION_REF_MIME = "application/x-binder-selection-ref";

/** 文档引用拖拽的自定义 MIME（文件树/标签栏 → AG 输入面整文档引用）。 */
export const DOC_REF_MIME = "application/x-binder-doc-ref";

export interface DocRefPayload {
  relPath: string;
}

interface TransferLike {
  getData(type: string): string;
}

interface WritableTransferLike extends TransferLike {
  setData(type: string, data: string): void;
}

export function writeDocRefData(dataTransfer: WritableTransferLike, relPath: string): void {
  const payload: DocRefPayload = { relPath };
  writeReferencePayloadData(dataTransfer, makeDocReference(relPath));
  dataTransfer.setData(DOC_REF_MIME, JSON.stringify(payload));
  dataTransfer.setData("text/plain", relPath);
}

export function readDocRefData(dataTransfer: TransferLike): DocRefPayload | null {
  const raw = dataTransfer.getData(DOC_REF_MIME);
  if (!raw) return null;
  try {
    const parsed = JSON.parse(raw) as Partial<DocRefPayload>;
    return typeof parsed.relPath === "string" && parsed.relPath.trim() ? { relPath: parsed.relPath } : null;
  } catch {
    return null;
  }
}

export function writeSelectionRefData(dataTransfer: WritableTransferLike, ref: SelectionRef): void {
  writeReferencePayloadData(dataTransfer, makeSelectionReference(ref));
  dataTransfer.setData(SELECTION_REF_MIME, JSON.stringify(ref));
  dataTransfer.setData("text/plain", ref.text);
}

export function readSelectionRefData(dataTransfer: TransferLike): SelectionRef | null {
  const raw = dataTransfer.getData(SELECTION_REF_MIME);
  if (!raw) return null;
  try {
    const parsed = JSON.parse(raw) as Partial<SelectionRef>;
    if (
      typeof parsed.path !== "string" ||
      typeof parsed.blockId !== "string" ||
      typeof parsed.start !== "number" ||
      typeof parsed.end !== "number" ||
      typeof parsed.text !== "string"
    ) {
      return null;
    }
    return {
      path: parsed.path,
      blockId: parsed.blockId,
      start: parsed.start,
      end: parsed.end,
      text: parsed.text,
    };
  } catch {
    return null;
  }
}
