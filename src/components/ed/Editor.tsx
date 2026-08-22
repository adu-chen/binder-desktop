import {
  useEffect,
  useRef,
  useState,
  type CSSProperties,
  type DragEvent as ReactDragEvent,
} from "react";
import { useEditor, EditorContent } from "@tiptap/react";
import type { EditorView } from "@tiptap/pm/view";
import type { Node as PMNode } from "@tiptap/pm/model";
import { TextSelection, type Transaction } from "@tiptap/pm/state";
import { generateJSON, type Editor as TiptapEditor } from "@tiptap/core";
import { buildEditorExtensions } from "./extensions";
import {
  isPaginationSettledTransaction,
  PAGINATION_VIEWPORT_CORRECTION_EVENT,
} from "./extensions/Pagination";
import {
  computeClientViewportRect,
  shouldArmPointerActivatedInputScroll,
  shouldSuppressPaginatedSelectionScroll,
  shouldSuppressPointerActivatedInputScroll,
  type ViewportRect,
} from "./extensions/paginationViewportAnchor";
import { isContainerBlock } from "./extensions/BlockIdExtension";
import { applyGreenReview, type GreenReviewValue } from "./extensions/DiffDecorationExtension";
import {
  clearPersistedSelection,
  handleSelectionDragDrop,
  lockDragSelection,
  persistSelectionOnBlur,
  unlockDragSelection,
} from "./extensions/SelectionPersistExtension";
import {
  edStore,
  useEdStore,
  type ContentFormat,
  type EditFormat,
  type EditSession,
  type TabState,
} from "../../store/edStore";
import { deStore, toGreenReview, useDeStore, type DiffView } from "../../store/deStore";
import { assistStore, type AssistTarget } from "../../store/assistStore";
import { ContextMenu, type ContextMenuDef } from "../shell/ContextMenu";
import {
  makePersistedSnapshot,
  persistSessionSnapshot,
  saveDocument,
  saveDocxDocument,
  saveMarkdownDocument,
} from "../../services/edService";
import { importProjectImage, importProjectImageData } from "../../services/docService";
import { exportDoc } from "../../services/prvService";
import { serializeMarkdownDocument, parseMarkdownForEditing } from "./markdown/markdownCodec";
import {
  registerEditor,
  bindActiveEditorDocument,
  clearActiveEditorBinding,
  getSelectionRef,
  getCursorRef,
  getActiveBlockSnapshotById,
  locateBlock,
  applyEditOps,
  getActiveBlockSnapshotByIndex,
  isActiveEditorBoundTo,
  DOC_REF_MIME,
  readDocRefData,
  writeSelectionRefData,
  registerActiveSaveHandler,
} from "../../services/edLocate";
import { makeSelectionReference } from "../../services/referencePayload";
import { startReferenceDrag } from "../../services/referenceDrag";

// REF-REFERENCE-REPAIR-01 F1.3：dragstart 时暂存的原选区引用，供 document 冒泡阶段（PM 内置
// dragstart 执行 clearData() 之后）回写到 DataTransfer，规避自定义 MIME 被清除。
let dragStartSelectionRef: ReturnType<typeof getSelectionRef> = null;
import { applyStyleOps } from "../../services/edStyleOps";
import { applyTableOperations } from "../../services/edTableOps";
import {
  applyImageOperations,
  type ImageOperationApplyResult,
} from "../../services/edImageOps";
import { applyStructuredBlockOperations } from "../../services/edBlockOps";
import { persistImageDiffSnapshots, promotePendingDiffOnOpen } from "../../services/deService";
import { outlineStore, extractOutline } from "../../store/outlineStore";
import { modelStore } from "../../store/modelStore";
import {
  acknowledgePreviewAndContinue,
  compensateCreateDocumentPreviewFailure,
  finishAssist,
  requestAssist,
  type AssistFinishEvent,
} from "../../services/agService";
import { InlineAssistPopover } from "../ag/InlineAssistPopover";
import { AutocompleteOverlay } from "../ag/AutocompleteOverlay";
import { EditorToolbar } from "./EditorToolbar";
import { EditorStatusBar } from "./EditorStatusBar";
import { TableMoveControls } from "./TableMoveControls";
import { ImageMoveControls } from "./ImageMoveControls";
import { ZOOM, stepZoom } from "./zoom";
import { edViewPrefsStore, getPageGeometry, useEdViewPrefs } from "../../store/edViewPrefsStore";
import {
  ASSIST_FLOATING_SIZE,
  computeAssistPlacement,
  intersectAssistRects,
  toAssistRect,
  unionAssistRects,
  type AssistOverlayKind,
  type AssistRect,
} from "./assistPlacement";

/**
 * ED 编辑器宿主（A-SYS-C-T-02 §3 编辑区 · PI-FE-05/07/08）。
 * TipTap 3 运行时 + blockId 扩展 + GreenReview 装饰层；编辑触发脏态（edStore，BR-ED-STATE-001 投影）。
 * GreenReview 与右侧 DiffCard 同源（deStore）→ 双向同步（X-INTER-001）。
 * ⌘K InlineAssist / ⌘J Autocomplete 辅助入口（AG-ASSIST / BR-AG-STATE-003）：均经 ED 受控接口编辑，不产 DiffCard。
 */
const SEED_CONTENT = "开始写作…";

interface PointerActivatedInputScrollGuard {
  documentAtLastDecision: PMNode;
  scrollTopAtPointer: number;
  activationPoint: { x: number; y: number };
  activationRenderedRect: ViewportRect | null;
}

interface PointerActivationEventSnapshot {
  button: number;
  clickCount: number;
  clientX: number;
  clientY: number;
}

const pointerActivatedInputScrollGuards = new WeakMap<EditorView, PointerActivatedInputScrollGuard>();
const pendingPointerActivatedInputScrollFrames = new WeakMap<EditorView, number>();

function dropPointerActivatedInputScrollGuard(view: EditorView): void {
  pointerActivatedInputScrollGuards.delete(view);
}

function clearPointerActivatedInputScrollGuard(view: EditorView): void {
  const pendingFrame = pendingPointerActivatedInputScrollFrames.get(view);
  if (pendingFrame !== undefined) {
    cancelAnimationFrame(pendingFrame);
    pendingPointerActivatedInputScrollFrames.delete(view);
  }
  dropPointerActivatedInputScrollGuard(view);
}

/**
 * 跳转到目标行对应块（搜索命中跳行）：行号 → 顶层块索引（钳制），定位光标 + 滚动到视区。
 * text/source-protected Markdown 按行装载；结构化 Markdown 按顶层块定位。
 */
function jumpToLine(editor: NonNullable<ReturnType<typeof useEditor>>, line: number) {
  const doc = editor.state.doc;
  const idx = Math.min(Math.max(line - 1, 0), Math.max(doc.childCount - 1, 0));
  let pos = 1; // doc 起始内偏移
  for (let j = 0; j < idx; j++) pos += doc.child(j).nodeSize;
  editor.chain().focus().setTextSelection(pos).scrollIntoView().run();
}

function exportTargetsForPath(relPath: string | null): Array<"pdf" | "docx" | "md"> {
  const ext = relPath?.split(".").pop()?.toLowerCase();
  if (ext === "md" || ext === "markdown" || ext === "txt") return ["docx"];
  if (ext === "docx" || ext === "doc") return ["pdf", "md"];
  return [];
}

/** 纯文本或源码保护 Markdown → ProseMirror doc（每行一段落，原文可编辑）。 */
function textToDoc(text: string) {
  const lines = text.split("\n");
  return {
    type: "doc",
    content: lines.map((line) =>
      line ? { type: "paragraph", content: [{ type: "text", text: line }] } : { type: "paragraph" },
    ),
  };
}

/** BDocx 只有当前 loadSeq 的 setContent 与首轮分页均完成后才可揭示。 */
export function isEditorContentReady(
  loadedContent: string | null,
  loadSeq: number,
  settledLoadSeq: number,
): boolean {
  return loadedContent !== null && loadSeq > 0 && settledLoadSeq === loadSeq;
}

function viewportRectFromDOMRect(rect: DOMRect): ViewportRect | null {
  if (
    !Number.isFinite(rect.left)
    || !Number.isFinite(rect.right)
    || !Number.isFinite(rect.top)
    || !Number.isFinite(rect.bottom)
    || (rect.width <= 0 && rect.height <= 0)
  ) {
    return null;
  }
  return { left: rect.left, right: rect.right, top: rect.top, bottom: rect.bottom };
}

function renderedCaretRect(view: EditorView): ViewportRect | null {
  const selection = view.root instanceof Document
    ? view.root.getSelection()
    : view.root.ownerDocument.getSelection();
  const focusNode = selection?.focusNode;
  if (selection?.isCollapsed && selection.rangeCount && focusNode && view.dom.contains(focusNode)) {
    try {
      const range = selection.getRangeAt(0);
      const rect = viewportRectFromDOMRect(range.getBoundingClientRect());
      if (rect) return rect;
      const lineRect = range.getClientRects().item(0);
      if (lineRect) {
        const fallback = viewportRectFromDOMRect(lineRect);
        if (fallback) return fallback;
      }
    } catch {
      // WebKit 在 selection/decorations 同步窗口内可能暂时不给出可读 Range；继续按 PM position 降级。
    }
  }
  try {
    const range = view.dom.ownerDocument.createRange();
    const point = view.domAtPos(view.state.selection.head);
    range.setStart(point.node, point.offset);
    range.collapse(true);
    const rect = viewportRectFromDOMRect(range.getBoundingClientRect())
      ?? (() => {
        const lineRect = range.getClientRects().item(0);
        return lineRect ? viewportRectFromDOMRect(lineRect) : null;
      })();
    range.detach();
    return rect;
  } catch {
    return null;
  }
}

function positionCaretRect(view: EditorView): ViewportRect | null {
  try {
    const rect = view.coordsAtPos(view.state.selection.head, 1);
    if (
      !Number.isFinite(rect.left)
      || !Number.isFinite(rect.right)
      || !Number.isFinite(rect.top)
      || !Number.isFinite(rect.bottom)
    ) {
      return null;
    }
    return { left: rect.left, right: rect.right, top: rect.top, bottom: rect.bottom };
  } catch {
    return null;
  }
}

function currentCaretRect(view: EditorView): ViewportRect | null {
  return renderedCaretRect(view) ?? positionCaretRect(view);
}

function clientViewportRectFor(paper: HTMLElement): ViewportRect | null {
  const paperBorderRect = paper.getBoundingClientRect();
  return computeClientViewportRect({
    borderRect: {
      left: paperBorderRect.left,
      right: paperBorderRect.right,
      top: paperBorderRect.top,
      bottom: paperBorderRect.bottom,
      width: paperBorderRect.width,
      height: paperBorderRect.height,
    },
    offsetWidth: paper.offsetWidth,
    offsetHeight: paper.offsetHeight,
    clientLeft: paper.clientLeft,
    clientTop: paper.clientTop,
    clientWidth: paper.clientWidth,
    clientHeight: paper.clientHeight,
  });
}

/**
 * Range rect 已包含 zoom 的引擎不再换算；WKWebView 若返回未缩放 rect，则按 computed zoom
 * 把相对位移映射回屏幕像素。只用于相对差，不改变绝对坐标。
 */
function renderedCaretDeltaScale(editorDOM: HTMLElement): number {
  const cssZoom = parseFloat(window.getComputedStyle(editorDOM).zoom);
  const safeZoom = Number.isFinite(cssZoom) && cssZoom > 0 ? cssZoom : 1;
  const rectWidth = editorDOM.getBoundingClientRect().width;
  const layoutWidth = editorDOM.offsetWidth;
  const rectScale = Number.isFinite(rectWidth) && rectWidth > 0 && layoutWidth > 0
    ? rectWidth / layoutWidth
    : 1;
  const scale = safeZoom / rectScale;
  return Number.isFinite(scale) && scale > 0 ? scale : 1;
}

function armPointerActivatedInputScrollGuard(
  view: EditorView,
  event: PointerActivationEventSnapshot,
): void {
  const editorDOM = view.dom as HTMLElement;
  const paper = editorDOM.closest(".editor-paper") as HTMLElement | null;
  const viewportRect = paper ? clientViewportRectFor(paper) : null;
  const collapsedTextSelection = view.state.selection instanceof TextSelection && view.state.selection.empty;
  if (
    view.isDestroyed
    || !paper
    || !shouldArmPointerActivatedInputScroll({
      button: event.button,
      clickCount: event.clickCount,
      paginated: editorDOM.classList.contains("rm-paginated"),
      loading: paper.getAttribute("aria-busy") === "true",
      collapsedTextSelection,
      activationPoint: { x: event.clientX, y: event.clientY },
      viewportRect,
    })
  ) {
    dropPointerActivatedInputScrollGuard(view);
    return;
  }
  pointerActivatedInputScrollGuards.set(view, {
    documentAtLastDecision: view.state.doc,
    scrollTopAtPointer: paper.scrollTop,
    activationPoint: { x: event.clientX, y: event.clientY },
    activationRenderedRect: currentCaretRect(view),
  });
}

/**
 * ProseMirror 在 document/root 的 mouseup 监听中完成最终选区；编辑器 DOM 的自定义 mouseup
 * 会更早执行。延后一帧可同时等待原生 selectionchange 与 ProseMirror 选区同步，避免双击/拖选
 * 用旧的折叠光标误建激活周期。
 */
function schedulePointerActivatedInputScrollGuard(view: EditorView, event: MouseEvent): void {
  clearPointerActivatedInputScrollGuard(view);
  const snapshot: PointerActivationEventSnapshot = {
    button: event.button,
    clickCount: event.detail,
    clientX: event.clientX,
    clientY: event.clientY,
  };
  const frame = requestAnimationFrame(() => {
    if (pendingPointerActivatedInputScrollFrames.get(view) !== frame) return;
    pendingPointerActivatedInputScrollFrames.delete(view);
    armPointerActivatedInputScrollGuard(view, snapshot);
  });
  pendingPointerActivatedInputScrollFrames.set(view, frame);
}

function rebasePointerActivatedInputScrollGuard(view: EditorView, event: Event): void {
  const guard = pointerActivatedInputScrollGuards.get(view);
  if (!guard) return;
  const correctedScrollTop = (event as CustomEvent<{ scrollTop?: unknown }>).detail?.scrollTop;
  if (typeof correctedScrollTop === "number" && Number.isFinite(correctedScrollTop)) {
    guard.scrollTopAtPointer = correctedScrollTop;
  }
}

function pointerActivatedInputScrollDecision(
  view: EditorView,
  paper: HTMLElement,
  editorDOM: HTMLElement,
  viewportRect: ViewportRect,
): boolean | null {
  const guard = pointerActivatedInputScrollGuards.get(view);
  if (!guard) return null;
  const documentChanged = view.state.doc !== guard.documentAtLastDecision;
  if (!documentChanged) {
    dropPointerActivatedInputScrollGuard(view);
    return false;
  }
  const currentRenderedRect = currentCaretRect(view);
  const activationRenderedRect = guard.activationRenderedRect ?? currentRenderedRect;
  if (!activationRenderedRect || !currentRenderedRect) {
    dropPointerActivatedInputScrollGuard(view);
    return false;
  }
  const suppress = shouldSuppressPointerActivatedInputScroll({
    documentChangedSinceLastDecision: true,
    scrollTopAtPointer: guard.scrollTopAtPointer,
    currentScrollTop: paper.scrollTop,
    activationPoint: guard.activationPoint,
    activationRenderedRect,
    currentRenderedRect,
    renderedDeltaScale: renderedCaretDeltaScale(editorDOM),
    viewportRect,
  });
  if (!suppress) {
    dropPointerActivatedInputScrollGuard(view);
    return false;
  }
  guard.documentAtLastDecision = view.state.doc;
  guard.activationRenderedRect = activationRenderedRect;
  return true;
}

/**
 * 分页编辑器只接管“光标已经完整可见”的冗余 scrollIntoView。真实越界、显式导航、IME、
 * 非文本选区与不可测量状态全部交还 ProseMirror，保留其原生滚动和更新期视口保持。
 */
function handlePaginatedScrollToSelection(view: EditorView): boolean {
  const editorDOM = view.dom as HTMLElement;
  const paper = editorDOM.closest(".editor-paper") as HTMLElement | null;
  const selection = view.state.selection;
  const paginated = editorDOM.classList.contains("rm-paginated");
  const loading = paper?.getAttribute("aria-busy") === "true";
  const collapsedTextSelection = selection instanceof TextSelection && selection.empty;
  if (!paper || !paginated || loading || !collapsedTextSelection) return false;
  const viewportRect = clientViewportRectFor(paper);
  if (!viewportRect) return false;
  // 指针激活周期以真实点击点 + Range 相对位移判断；不按第几个字符硬编码。
  const pointerDecision = pointerActivatedInputScrollDecision(view, paper, editorDOM, viewportRect);
  if (pointerDecision !== null) return pointerDecision;
  if (view.composing) return false;
  return shouldSuppressPaginatedSelectionScroll({
    paginated,
    loading,
    composing: view.composing,
    collapsedTextSelection,
    renderedRect: renderedCaretRect(view),
    viewportRect,
  });
}

/** 序列化编辑器当前内容为 LogicalState：HTML 与结构化 Markdown 使用 HTML，纯文本/源码保护使用文本。 */
function serialize(
  editor: NonNullable<ReturnType<typeof useEditor>>,
  fmt: EditFormat,
  contentFormat: ContentFormat,
  _path: string | null,
): string {
  if (fmt === "html" || fmt === "html_file" || (fmt === "markdown" && contentFormat === "html")) {
    return editor.getHTML();
  }
  return editor.getText({ blockSeparator: "\n" });
}

/**
 * 将单条待裁决 ImageOperation 应用到当前编辑器，并建立可拒绝恢复的 Diff 快照。
 *
 * 该函数是 Editor pending-diff effect 的可测试边界：只编排既有受控操作、deStore 和快照持久化，
 * 不引入第二套图片编辑实现。
 */
export function applyPendingImageDiffToEditor(
  editor: TiptapEditor,
  diff: DiffView,
  session: EditSession | undefined,
  baseContent: string,
  baseTabState: TabState,
  serializeAfter: () => string,
  persistSnapshots: typeof persistImageDiffSnapshots = persistImageDiffSnapshots,
): ImageOperationApplyResult | null {
  const payload = diff.imageOpsPayload;
  if (!payload) return null;
  if (
    !session
    || session.sessionVersion !== payload.base.sessionVersion
    || session.contentHash !== payload.base.contentHash
  ) {
    deStore.markDraftPreviewFailed(diff.diffId, "ED_IMAGE_SNAPSHOT_STALE");
    return null;
  }
  const applied = applyImageOperations(editor, payload);
  if (!applied) {
    deStore.markDraftPreviewFailed(diff.diffId, "ED_IMAGE_OPERATION_FAILED");
    return null;
  }
  deStore.setImageSnapshots(diff.diffId, applied.before, applied.after, applied.affectedBlockIds);
  deStore.markDraftApplied(
    diff.diffId,
    baseContent,
    baseTabState,
    serializeAfter(),
  );
  void persistSnapshots(
    diff.diffId,
    applied.before,
    applied.after,
    applied.affectedBlockIds,
  ).catch((error) => {
    console.error("[de-diff] 图片审阅快照持久化失败", error);
    deStore.setLocalView(diff.diffId, "error", {
      errorCode: "DE_IMAGE_SNAPSHOT_PERSIST_FAILED",
    });
  });
  return applied;
}

export interface ImageDropAnchor {
  anchorBlockId: string;
  position: "before" | "after";
}

export type ImageDropTransferKind = "project_image" | "local_files";

type ImageIngressSource =
  | { kind: "path"; sourcePath: string }
  | { kind: "data"; sourceName: string; sourceDataUrl: string };

/** 图片选区必须保留 ProseMirror 原生富剪贴板，不能降级成 SelectionRef 的叶子占位字符。 */
export function selectionContainsImage(doc: PMNode, from: number, to: number): boolean {
  let contains = false;
  doc.nodesBetween(from, to, (node) => {
    if (node.type.name === "image") {
      contains = true;
      return false;
    }
    return !contains;
  });
  return contains;
}

/** 剪贴板只接收可由 DOC magic 再校验的 PNG/JPEG File；其他图片格式不进入受控导入链。 */
export function clipboardImageFiles(dataTransfer: DataTransfer): File[] {
  const files = Array.from(dataTransfer.files ?? []);
  if (!files.length && dataTransfer.items) {
    for (const item of Array.from(dataTransfer.items)) {
      if (item.kind !== "file") continue;
      const file = item.getAsFile();
      if (file) files.push(file);
    }
  }
  return files.filter((file) => (
    file.type === "image/png"
    || file.type === "image/jpeg"
    || /\.(png|jpe?g)$/i.test(file.name)
  ));
}

/**
 * DOM 拖放是 WKWebView 的单一人工图片入口：Binder 文件树使用稳定 DocRef，
 * Finder 使用 Files。普通富文本/文档内图片拖动返回 null，继续交给 ProseMirror。
 */
export function imageDropTransferKind(dataTransfer: DataTransfer): ImageDropTransferKind | null {
  const types = Array.from(dataTransfer.types ?? []);
  if (types.includes(DOC_REF_MIME)) return "project_image";
  if (
    types.some((type) => type.toLowerCase() === "files")
    || (dataTransfer.files?.length ?? 0) > 0
    || Array.from(dataTransfer.items ?? []).some((item) => item.kind === "file")
  ) {
    return "local_files";
  }
  return null;
}

function dataTransferFileCount(dataTransfer: DataTransfer): number {
  if (dataTransfer.files?.length) return dataTransfer.files.length;
  return Array.from(dataTransfer.items ?? []).filter((item) => item.kind === "file").length;
}

/** Binder 自身复制的图片 HTML 携稳定媒体身份，可交还 ProseMirror 原生粘贴并复用同一媒体资源。 */
export function isManagedImageClipboardHtml(html: string): boolean {
  return /<img\b[^>]*\bdata-binder-media-src\s*=/i.test(html);
}

/** 某些截图/浏览器只把图片像素放在 HTML data URL 中；仅纯图片且全为 PNG/JPEG 时才接管。 */
export function clipboardImageDataSourcesFromHtml(
  html: string,
): Array<{ sourceName: string; sourceDataUrl: string }> {
  if (!html || typeof DOMParser === "undefined") return [];
  const parsed = new DOMParser().parseFromString(html, "text/html");
  if (parsed.body.textContent?.trim()) return [];
  const images = Array.from(parsed.body.querySelectorAll("img"));
  if (!images.length) return [];
  const sources = images.map((image, index) => {
    const sourceDataUrl = image.getAttribute("src") ?? "";
    const match = sourceDataUrl.match(/^data:(image\/(?:png|jpeg));base64,/i);
    if (!match) return null;
    const extension = match[1].toLowerCase() === "image/jpeg" ? "jpg" : "png";
    return { sourceName: `clipboard-${index + 1}.${extension}`, sourceDataUrl };
  });
  return sources.every((source) => source !== null)
    ? sources as Array<{ sourceName: string; sourceDataUrl: string }>
    : [];
}

function clipboardImageMediaType(file: File): "image/png" | "image/jpeg" {
  if (file.type === "image/jpeg") return "image/jpeg";
  if (file.type === "image/png") return "image/png";
  return /\.jpe?g$/i.test(file.name) ? "image/jpeg" : "image/png";
}

function clipboardImageName(file: File, index: number): string {
  const extension = clipboardImageMediaType(file) === "image/jpeg" ? "jpg" : "png";
  const stem = file.name.replace(/\.[^.]+$/, "").trim() || `clipboard-${index + 1}`;
  return `${stem}.${extension}`;
}

function fileAsDataUrl(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => (
      typeof reader.result === "string"
        ? resolve(reader.result)
        : reject(new Error("本地图片读取结果无效"))
    );
    reader.onerror = () => reject(reader.error ?? new Error("读取本地图片失败"));
    reader.readAsDataURL(file);
  });
}

/** 将 WebView 拖放点映射到一个稳定顶层 Block 锚；绝对坐标只参与本次交互，不进入文档模型。 */
export function imageDropAnchorAtClientPoint(
  editor: TiptapEditor,
  clientX: number,
  clientY: number,
): ImageDropAnchor | null {
  const hit = editor.view.posAtCoords({ left: clientX, top: clientY });
  let hitBlock: { node: PMNode; offset: number } | null = null;
  const candidates: Array<{
    blockId: string;
    rect: DOMRect;
  }> = [];
  editor.state.doc.forEach((node, offset) => {
    const blockId = node.attrs?.blockId;
    if (typeof blockId !== "string" || !blockId.trim()) return;
    const dom = editor.view.nodeDOM(offset);
    if (dom instanceof Element) {
      candidates.push({ blockId, rect: dom.getBoundingClientRect() });
    }
    if (
      hit
      && !hitBlock
      && hit.pos >= offset
      && hit.pos <= offset + node.nodeSize
    ) {
      hitBlock = { node, offset };
    }
  });
  const block = hitBlock as { node: PMNode; offset: number } | null;
  const hitBlockId = block?.node.attrs?.blockId;
  if (block && typeof hitBlockId === "string" && hitBlockId.trim()) {
    const dom = editor.view.nodeDOM(block.offset);
    const rect = dom instanceof Element ? dom.getBoundingClientRect() : null;
    return {
      anchorBlockId: hitBlockId,
      position: rect && clientY < rect.top + rect.height / 2 ? "before" : "after",
    };
  }
  if (!candidates.length) return null;
  const nearest = candidates.reduce((best, candidate) => {
    const distance = clientY < candidate.rect.top
      ? candidate.rect.top - clientY
      : clientY > candidate.rect.bottom
        ? clientY - candidate.rect.bottom
        : 0;
    const bestDistance = clientY < best.rect.top
      ? best.rect.top - clientY
      : clientY > best.rect.bottom
        ? clientY - best.rect.bottom
        : 0;
    return distance < bestDistance ? candidate : best;
  });
  return {
    anchorBlockId: nearest.blockId,
    position: clientY < nearest.rect.top + nearest.rect.height / 2 ? "before" : "after",
  };
}

/** 剪贴板没有落点坐标，按当前光标所在顶层 Block 之后插入独立图片段落。 */
export function imagePasteAnchorAtSelection(editor: TiptapEditor): ImageDropAnchor | null {
  const selectionPos = editor.state.selection.head;
  let anchorBlockId: string | null = null;
  editor.state.doc.forEach((node, offset) => {
    if (
      anchorBlockId
      || selectionPos < offset
      || selectionPos > offset + node.nodeSize
    ) return;
    const blockId = node.attrs?.blockId;
    if (typeof blockId === "string" && blockId.trim()) anchorBlockId = blockId;
  });
  return anchorBlockId ? { anchorBlockId, position: "after" } : null;
}

const sessionSnapshotPersistTimers = new Map<string, ReturnType<typeof globalThis.setTimeout>>();

function isDirtySessionState(tabState: string): boolean {
  return tabState === "dirty" || tabState === "pending_diff" || tabState === "saving";
}

function persistEditorSessionSnapshot(path: string): void {
  const sess = edStore.getSnapshot().sessions[path];
  if (!sess || sess.content === null) return;
  void persistSessionSnapshot(
    makePersistedSnapshot(
      path,
      sess.content,
      sess.editFormat,
      isDirtySessionState(sess.tabState),
      sess.sessionVersion,
      sess.contentFormat,
    ),
  ).catch((e) => {
    console.error(`ed session snapshot persist failed ${String(e)}`);
  });
}

function scheduleEditorSessionSnapshotPersist(path: string): void {
  const existing = sessionSnapshotPersistTimers.get(path);
  if (existing) globalThis.clearTimeout(existing);
  const timer = globalThis.setTimeout(() => {
    sessionSnapshotPersistTimers.delete(path);
    persistEditorSessionSnapshot(path);
  }, 500);
  sessionSnapshotPersistTimers.set(path, timer);
}

function currentBlockIds(editor: NonNullable<ReturnType<typeof useEditor>>): string[] {
  const ids: string[] = [];
  editor.state.doc.descendants((node) => {
    const blockId = node.attrs?.blockId;
    if (typeof blockId === "string" && blockId.trim()) ids.push(blockId);
  });
  return ids;
}

function isSyntheticAfter(after: string): boolean {
  return after === "（样式修改）" || after === "（AI 修改建议，接受后应用到文档）";
}

/** 每标签滚动位置暂存（PI-ED-07 N7，会话级视图瞬态；组件重挂即清）。 */
const scrollMemo = new Map<string, number>();
const ASSIST_ANCHOR_CONTEXT_CHARS = 80;

export function Editor() {
  const { openPath, tabState, loadSeq, loadedContent, editFormat, contentFormat, converting, styleNotices, sourceView } = useEdStore();
  const { diffs } = useDeStore(); // GreenReview 同步用（标签状态投影已移至 center TabStrip）
  const hostRef = useRef<HTMLDivElement>(null);
  const paperRef = useRef<HTMLDivElement>(null);
  const [settledLoadSeq, setSettledLoadSeq] = useState(0);
  const [imageDropError, setImageDropError] = useState<string | null>(null);
  const [imageDropActive, setImageDropActive] = useState(false);
  const [sourceViewText, setSourceViewText] = useState<string>("");
  const sourceViewTextRef = useRef<string>("");
  // 编辑器右键菜单（CTX-MENU-01）：选区态决定菜单项集（有选区→剪切/复制/局部修改；无选区→粘贴/辅助续写）。
  const [ctxMenu, setCtxMenu] = useState<{ x: number; y: number; hasSelection: boolean } | null>(null);
  const editorInstanceRef = useRef<TiptapEditor | null>(null);
  const imageIngressRef = useRef<(
    sources: ImageIngressSource[],
    anchor: ImageDropAnchor,
  ) => Promise<void>>(async () => undefined);
  const pendingPaginationLoadSeqRef = useRef(0);
  const pendingScrollRestoreRef = useRef<{
    path: string;
    loadSeq: number;
    scrollTop: number;
  } | null>(null);
  // 上一活动文档路径（切换标签前据此把当前编辑内容序列化回写其会话 → 不丢草稿）。
  const lastPathRef = useRef<string | null>(null);
  // 视图偏好（PI-ED-06/07：纸张 + 缩放，per-doc localStorage；不落盘文档）。
  const viewPrefs = useEdViewPrefs();
  const paperPrefs = openPath ? (viewPrefs[openPath] ?? edViewPrefsStore.getPrefs(openPath)) : null;
  // 缩放（#10 per-doc 持久）：从视图偏好读；无文档回默认。
  const zoom = paperPrefs?.zoom ?? ZOOM.default;
  const setZoom = (z: number) => {
    if (openPath) edViewPrefsStore.setZoom(openPath, z);
  };

  const restorePendingScroll = (path: string, seq: number) => {
    const pending = pendingScrollRestoreRef.current;
    if (!pending || pending.path !== path || pending.loadSeq !== seq) return;
    pendingScrollRestoreRef.current = null;
    requestAnimationFrame(() => {
      if (
        paperRef.current
        && lastPathRef.current === path
        && edStore.getSnapshot().loadSeq === seq
      ) {
        paperRef.current.scrollTop = pending.scrollTop;
      }
    });
  };

  const prefixBefore = (view: EditorView, pos: number) =>
    view.state.doc.textBetween(Math.max(0, pos - 400), pos, " ");

  const anchorContext = (blockId: string, start: number, end: number) => {
    const block = getActiveBlockSnapshotById(blockId);
    if (!block) return null;
    return {
      before: block.text.slice(Math.max(0, start - ASSIST_ANCHOR_CONTEXT_CHARS), start),
      after: block.text.slice(end, Math.min(block.text.length, end + ASSIST_ANCHOR_CONTEXT_CHARS)),
    };
  };

  const isAssistTargetCurrent = (target: AssistTarget | null): target is AssistTarget => {
    if (!target) return false;
    const snap = edStore.getSnapshot();
    if (snap.openPath !== target.docPath || snap.loadSeq !== target.loadSeq) return false;
    if (locateBlock(target.blockId, { start: target.start, end: target.end }) !== "located") return false;
    const block = getActiveBlockSnapshotById(target.blockId);
    if (!block) return false;
    if (block.text.slice(Math.max(0, target.start - target.before.length), target.start) !== target.before) return false;
    if (block.text.slice(target.end, target.end + target.after.length) !== target.after) return false;
    if (!target.text) return true;
    return block.text.slice(target.start, target.end) === target.text;
  };

  const finishCurrentAssist = (event: AssistFinishEvent) => {
    const snap = assistStore.getSnapshot();
    if (snap.kind !== "none" && snap.requestId) void finishAssist(snap.requestId, event);
    assistStore.close();
  };

  const finishAssistId = (requestId: string, event: AssistFinishEvent) => {
    void finishAssist(requestId, event);
  };

  /**
   * @GOV
   * codes:    BR-AG-STATE-003-RB-AG-AG-ASSIST-007
   * type:     RB
   * chain:    AG-ASSIST
   * rules:    BR-AG-STATE-003
   * term_ref: TERM-AG-003, TERM-AG-004
   * boundary: in=AssistTargetPointerOrSelectionChange
   *           | out=finishAssist(dismissOrAnchorLostAndAssistStoreClosed)
   *           | does_not=show_or_apply_stale_assist_candidate
   */
  const isAssistFloatingTarget = (target: EventTarget | null): boolean => {
    const element = target instanceof Element ? target : target instanceof Node ? target.parentElement : null;
    return Boolean(element?.closest(".assist-pop, .assist-ac"));
  };

  const isEditorSelectionOnAssistTarget = (target: AssistTarget): boolean => {
    if (target.mode === "cursor") {
      const cursor = getCursorRef();
      return Boolean(cursor && cursor.blockId === target.blockId && cursor.offset === target.start && target.start === target.end);
    }
    const selection = getSelectionRef();
    return Boolean(selection && selection.blockId === target.blockId && selection.start === target.start && selection.end === target.end);
  };

  const finishAssistIfInteractionTargetChanged = () => {
    const snap = assistStore.getSnapshot();
    if (snap.kind === "none") return;
    const target = snap.target;
    if (!isAssistTargetCurrent(target)) {
      finishCurrentAssist("anchor_lost");
      return;
    }
    if (!isEditorSelectionOnAssistTarget(target)) finishCurrentAssist("dismiss");
  };

  const isAssistKeyboardScope = (event: KeyboardEvent) => {
    const host = hostRef.current;
    if (!host) return false;
    const target = event.target instanceof Node ? event.target : null;
    const active = document.activeElement;
    return Boolean((target && host.contains(target)) || (active && host.contains(active)));
  };

  const assistBoundaryRect = (view: EditorView, hostRect: AssistRect): AssistRect => {
    const viewportRect = toAssistRect(paperRef.current?.getBoundingClientRect() ?? hostRef.current?.getBoundingClientRect() ?? view.dom.getBoundingClientRect());
    return intersectAssistRects(viewportRect, hostRect) ?? hostRect;
  };

  const domRangeRect = (view: EditorView, from: number, to: number): AssistRect | null => {
    try {
      const range = document.createRange();
      const start = view.domAtPos(from);
      const end = view.domAtPos(to);
      range.setStart(start.node, start.offset);
      range.setEnd(end.node, end.offset);
      const rect = range.getBoundingClientRect();
      range.detach();
      if (rect.width > 0 || rect.height > 0) return toAssistRect(rect);
    } catch {
      // DOM Range can fail at widget boundaries; callers fall back to ProseMirror coords.
    }
    return null;
  };

  const caretRect = (view: EditorView, pos: number): AssistRect => domRangeRect(view, pos, pos) ?? toAssistRect(view.coordsAtPos(pos));

  const selectionVisualRect = (view: EditorView, from: number, to: number): AssistRect => {
    if (from >= to) return caretRect(view, from);
    const rect = domRangeRect(view, from, to);
    if (rect) return rect;
    return unionAssistRects(caretRect(view, from), caretRect(view, to));
  };

  const assistAnchor = (view: EditorView, targetRect: AssistRect, kind: AssistOverlayKind) => {
    const host = hostRef.current;
    if (!host) return null;
    const hostRect = toAssistRect(host.getBoundingClientRect());
    return computeAssistPlacement({
      kind,
      anchorRect: targetRect,
      boundaryRect: assistBoundaryRect(view, hostRect),
      hostRect,
      floatingSize: ASSIST_FLOATING_SIZE[kind],
    });
  };

  const charOffsetToBlockContentPos = (block: PMNode, offset: number): number => {
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
  };

  const assistTargetRange = (view: EditorView, target: AssistTarget): { from: number; to: number } | null => {
    let hit: { node: PMNode; pos: number } | null = null;
    view.state.doc.descendants((node, pos) => {
      if (node.attrs.blockId === target.blockId) {
        hit = { node, pos };
        return false;
      }
      return true;
    });
    const found = hit as { node: PMNode; pos: number } | null;
    if (!found) return null;
    // 审计修复（2026-07-25 · A+D）：容器块不支持字符偏移计算
    if (isContainerBlock(found.node as unknown as { type: { name: string } })) return null;
    const from = found.pos + 1 + charOffsetToBlockContentPos(found.node, target.start);
    const to = found.pos + 1 + charOffsetToBlockContentPos(found.node, target.end);
    return { from, to };
  };

  const assistVisualRectForTarget = (view: EditorView, target: AssistTarget): AssistRect | null => {
    const range = assistTargetRange(view, target);
    if (!range) return null;
    return target.mode === "selection" ? selectionVisualRect(view, range.from, range.to) : caretRect(view, range.from);
  };

  const repositionCurrentAssist = (view: EditorView) => {
    const snap = assistStore.getSnapshot();
    if (snap.kind === "none" || !snap.target || !snap.requestId) return;
    if (!isAssistTargetCurrent(snap.target)) {
      finishCurrentAssist("anchor_lost");
      return;
    }
    const visualRect = assistVisualRectForTarget(view, snap.target);
    const nextAnchor = visualRect ? assistAnchor(view, visualRect, snap.kind) : null;
    if (!nextAnchor) {
      finishCurrentAssist("dismiss");
      return;
    }
    assistStore.setAnchor(snap.requestId, nextAnchor);
  };

  const openInlineAssist = (view: EditorView): boolean => {
    const { from, to, empty } = view.state.selection;
    const docCtx = edStore.getSnapshot();
    if (!docCtx.openPath) return true;
    const visualRect = empty ? caretRect(view, from) : selectionVisualRect(view, from, to);
    const anchor = assistAnchor(view, visualRect, "inline");
    if (!anchor) return true;
    if (empty) {
      const cursor = getCursorRef();
      if (!cursor) return true;
      const ctx = anchorContext(cursor.blockId, cursor.offset, cursor.offset);
      if (!ctx) return true;
      if (!assistStore.claimTrigger("inline")) return true;
      const current = assistStore.getSnapshot();
      if (current.kind !== "none" && current.requestId) finishAssistId(current.requestId, "dismiss");
      assistStore.openInline(anchor, "", {
        docPath: docCtx.openPath,
        loadSeq: docCtx.loadSeq,
        blockId: cursor.blockId,
        start: cursor.offset,
        end: cursor.offset,
        text: "",
        prefix: prefixBefore(view, from),
        before: ctx.before,
        after: ctx.after,
        mode: "cursor",
      });
      return true;
    }
    const ref = getSelectionRef();
    if (!ref) return true;
    const selectedText = view.state.doc.textBetween(from, to, "\n", "￼");
    if (!selectedText) return true;
    const ctx = anchorContext(ref.blockId, ref.start, ref.end);
    if (!ctx) return true;
    if (!assistStore.claimTrigger("inline")) return true;
    const current = assistStore.getSnapshot();
    if (current.kind !== "none" && current.requestId) finishAssistId(current.requestId, "dismiss");
    assistStore.openInline(anchor, selectedText, {
      docPath: docCtx.openPath,
      loadSeq: docCtx.loadSeq,
      blockId: ref.blockId,
      start: ref.start,
      end: ref.end,
      text: selectedText,
      prefix: prefixBefore(view, from),
      before: ctx.before,
      after: ctx.after,
      mode: "selection",
    });
    return true;
  };

  /**
   * @GOV
   * codes:    BR-AG-STATE-003-EFFECT-AG-AG-ASSIST-003
   * type:     EFFECT
   * chain:    AG-ASSIST
   * rules:    BR-AG-STATE-003
   * term_ref: TERM-AG-004
   * boundary: in=EditorCursorTriggerAndAssistResponse
   *           | out=assistStore(threeCandidatesOrInternalFailureReason)
   *           | does_not=collapse_backend_failure_reason_into_reasonless_empty_state
   */
  const openAutocompleteAssist = (view: EditorView): boolean => {
    const { to } = view.state.selection;
    const docCtx = edStore.getSnapshot();
    if (!docCtx.openPath) return true;
    const cursor = getCursorRef();
    if (!cursor) return true;
    const anchor = assistAnchor(view, caretRect(view, to), "autocomplete");
    if (!anchor) return true;
    const prefix = prefixBefore(view, to);
    const ctx = anchorContext(cursor.blockId, cursor.offset, cursor.offset);
    if (!ctx) return true;
    if (!assistStore.claimTrigger("autocomplete")) return true;
    const current = assistStore.getSnapshot();
    if (current.kind !== "none" && current.requestId) finishAssistId(current.requestId, "dismiss");
    const target: AssistTarget = {
      docPath: docCtx.openPath,
      loadSeq: docCtx.loadSeq,
      blockId: cursor.blockId,
      start: cursor.offset,
      end: cursor.offset,
      text: "",
      prefix,
      before: ctx.before,
      after: ctx.after,
      mode: "cursor",
    };
    const requestId = assistStore.openAutocomplete(anchor, target);
    // 模型参数按 autocomplete 能力偏好解析（BR-AG-DATA-007：跟随全局或显式指定）。
    const autocompleteModel = modelStore.resolveFor("autocomplete");
    void requestAssist({
      requestId,
      kind: "autocomplete",
      docPath: docCtx.openPath,
      loadSeq: docCtx.loadSeq,
      blockId: target.blockId,
      start: target.start,
      end: target.end,
      prefix,
      before: target.before,
      after: target.after,
      provider: autocompleteModel.provider,
      model: autocompleteModel.model,
    }).then((result) => {
      const suggestions = result.candidates.length ? result.candidates : result.candidate ? [result.candidate] : [];
      if (!assistStore.isCurrent(requestId, "autocomplete")) {
        if (suggestions.length) finishAssistId(requestId, "dismiss");
        return;
      }
      const snap = assistStore.getSnapshot();
      if (!isAssistTargetCurrent(snap.target)) {
        finishAssistId(requestId, "anchor_lost");
        assistStore.close();
        return;
      }
      if (suggestions.length === 3) {
        assistStore.setSuggestions(requestId, suggestions);
      } else {
        const reason = result.failureReason ?? (suggestions.length > 0 ? "candidate_count_mismatch" : "unknown");
        assistStore.setAutocompleteFailure(requestId, reason);
      }
    });
    return true;
  };

  const editor = useEditor({
    extensions: buildEditorExtensions(),
    content: SEED_CONTENT,
    onUpdate: ({ editor: activeEditor }) => {
      const ed = edStore.getSnapshot();
      if (ed.openPath) {
        edStore.updateContent(ed.openPath, serialize(activeEditor, ed.editFormat, ed.contentFormat, ed.openPath));
      }
      edStore.markDirty();
      if (ed.openPath) scheduleEditorSessionSnapshotPersist(ed.openPath);
      if (assistStore.getSnapshot().kind !== "none") finishCurrentAssist("dismiss");
    },
    editorProps: {
      // 分页模式下，光标已完整可见时阻止冗余 scrollIntoView；真实越界仍由 ProseMirror 原生滚动。
      handleScrollToSelection: handlePaginatedScrollToSelection,
      scrollThreshold: 12,
      scrollMargin: 12,
      // 选区拖拽携带位置引用（PI-ED-10 #6，BR-ED-DATA-004/TERM-ED-005）：dragstart 时把 SelectionRef
      // （字符锚 + 文本 + 来源路径）写入自定义 MIME——拖入 AG 输入面成为位置引用；编辑器内拖动仍走
      // ProseMirror 原生移动（不阻止默认，返回 false）。
      handleDOMEvents: {
        pointerup: (view, event) => {
          schedulePointerActivatedInputScrollGuard(view, event);
          return false;
        },
        mouseup: (view, event) => {
          schedulePointerActivatedInputScrollGuard(view, event);
          return false;
        },
        blur: (view) => {
          clearPointerActivatedInputScrollGuard(view);
          // 选区失焦持久化（ED-SELECTION-PERSIST-01）：PM 的 handleDOMEvents 只取组件级 props，
          // 插件级被遮蔽，故在此桥接。注意：必须延迟到 PM handlers.blur（view.focused=false）之后
          // 再派发 persist 事务——否则 decorations 重算时 focused 仍 true，失焦分支不命中。
          setTimeout(() => persistSelectionOnBlur(view), 0);
          return false;
        },
        focus: (view) => {
          clearPersistedSelection(view);
          return false;
        },
        [PAGINATION_VIEWPORT_CORRECTION_EVENT]: (view, event) => {
          rebasePointerActivatedInputScrollGuard(view, event);
          return false;
        },
        dragstart: (view, _event) => {
          if (selectionContainsImage(view.state.doc, view.state.selection.from, view.state.selection.to)) {
            return false;
          }
          // 拖动锁定（ED-SELECTION-DRAG-01）：锁定当前非空选区为原选区，防原生拖选篡改 selection。
          lockDragSelection(view);
          // MIME 写入统一由 document 冒泡阶段后置回写（REF-REFERENCE-REPAIR-01 F1.3）：PM 内置
          // dragstart 会执行 clearData() 并只重写 text/html·text/plain，此处写入会被清除；此处仅
          // 暂存原选区引用（handleDOMEvents 先于 PM 内置 handler，selection 尚未被篡改）。
          dragStartSelectionRef = getSelectionRef();
          return false; // 不接管 dragstart 本身：编辑器内拖动仍由 PM 触发 drop，handleDrop 接管
        },
        dragend: (view) => {
          // 拖动结束释放锁（未 drop 的拖出场景同样在此解锁；lastNonEmpty 已保持锁定范围）。
          unlockDragSelection(view);
          dragStartSelectionRef = null;
          return false;
        },
        copy: (view, event) => {
          if (selectionContainsImage(view.state.doc, view.state.selection.from, view.state.selection.to)) {
            return false;
          }
          const ref = getSelectionRef();
          if (!ref || !event.clipboardData) return false;
          const path = edStore.getSnapshot().openPath ?? "";
          writeSelectionRefData(event.clipboardData, { ...ref, path });
          event.preventDefault();
          return true;
        },
        paste: (_view, event) => {
          const dataTransfer = event.clipboardData;
          if (!dataTransfer) return false;
          const imageFiles = clipboardImageFiles(dataTransfer);
          if (!imageFiles.length) {
            const html = dataTransfer.getData("text/html");
            if (isManagedImageClipboardHtml(html)) return false;
            const dataSources = clipboardImageDataSourcesFromHtml(html);
            if (dataSources.length) {
              event.preventDefault();
              const activeEditor = editorInstanceRef.current;
              const anchor = activeEditor ? imagePasteAnchorAtSelection(activeEditor) : null;
              if (!anchor) {
                setImageDropError("无法在当前光标位置建立图片插入锚点");
                return true;
              }
              void imageIngressRef.current(
                dataSources.map((source) => ({ kind: "data", ...source })),
                anchor,
              );
              return true;
            }
            if (/<img\b/i.test(html)) {
              event.preventDefault();
              setImageDropError("不能直接粘贴远程或未受控图片；请复制本地 PNG/JPEG 文件或图片像素");
              return true;
            }
            return false;
          }
          event.preventDefault();
          const activeEditor = editorInstanceRef.current;
          const anchor = activeEditor ? imagePasteAnchorAtSelection(activeEditor) : null;
          if (!anchor) {
            setImageDropError("无法在当前光标位置建立图片插入锚点");
            return true;
          }
          void Promise.all(imageFiles.map(async (file, index): Promise<ImageIngressSource> => ({
            kind: "data",
            sourceName: clipboardImageName(file, index),
            sourceDataUrl: (await fileAsDataUrl(file)).replace(
              /^data:[^,]*,/i,
              `data:${clipboardImageMediaType(file)};base64,`,
            ),
          }))).then(
            (sources) => imageIngressRef.current(sources, anchor),
            (error) => setImageDropError(error instanceof Error ? error.message : "读取剪贴板图片失败"),
          );
          return true;
        },
      },
      // 选区拖动 drop（ED-SELECTION-DRAG-01）：接管 move 型选区拖动，删除 dragstart 锁定的原选区。
      handleDrop: (view, event, slice, move) => handleSelectionDragDrop(view, event, slice, move),
      handleKeyDown: (view, event) => {
        const mod = event.metaKey || event.ctrlKey;
        if (!mod) return false;
        const key = event.key.toLowerCase();

        // ⌘S 保存：ws.save_doc（带加载基线，SM-WS-01）。
        if (key === "s") {
          event.preventDefault();
          void saveCurrent();
          return true;
        }
        // ⌘K InlineAssist：有选区时替换选区；无选区时按光标插入。
      if (key === "k") {
        event.preventDefault();
        if (assistStore.getSnapshot().kind === "inline") {
          finishCurrentAssist("dismiss");
          return true;
        }
          return openInlineAssist(view);
        }
        // ⌘J Autocomplete：光标处续写补全——经 ag_request_assist（门禁前置 + 网关，SM-AG-03）取候选。
        if (key === "j") {
          event.preventDefault();
          return openAutocompleteAssist(view);
        }
        return false;
      },
    },
  });
  editorInstanceRef.current = editor;
  imageIngressRef.current = async (sources, anchor) => {
    const expected = edStore.getSnapshot();
    const expectedPath = expected.openPath;
    const expectedLoadSeq = expected.loadSeq;
    const expectedSession = expectedPath ? expected.sessions[expectedPath] : undefined;
    const activeEditor = editorInstanceRef.current;
    if (
      !activeEditor
      || !expectedPath
      || expected.editFormat !== "html"
      || expectedSession?.baseline === null
      || expectedSession?.baseline === undefined
    ) {
      setImageDropError("只有已载入的 BDocx 编辑会话可以插入本地图片");
      return;
    }
    setImageDropError(null);
    try {
      const imported = [];
      for (const source of sources) {
        imported.push(source.kind === "path"
          ? await importProjectImage(expectedPath, source.sourcePath, expectedSession.baseline)
          : await importProjectImageData(
            expectedPath,
            source.sourceName,
            source.sourceDataUrl,
            expectedSession.baseline,
          ));
      }
      const current = edStore.getSnapshot();
      if (
        current.openPath !== expectedPath
        || current.loadSeq !== expectedLoadSeq
        || current.editFormat !== "html"
        || editorInstanceRef.current !== activeEditor
      ) {
        setImageDropError("文档已切换，图片未插入；已导入的项目资源可供后续使用");
        return;
      }
      const currentSession = current.sessions[expectedPath];
      const applied = applyImageOperations(activeEditor, {
        base: {
          sessionVersion: currentSession?.sessionVersion ?? 0,
          contentHash: currentSession?.contentHash ?? "",
        },
        operations: imported.map((asset) => ({
          op: "insert_image" as const,
          ...anchor,
          assetId: asset.assetId,
          sourcePath: asset.sourcePath,
          sourceHash: asset.sourceHash,
          mediaRef: asset.mediaRef,
          displaySrc: asset.displaySrc,
        })),
      });
      if (!applied) setImageDropError("图片已导入项目，但插入文档失败，请重试");
    } catch (error) {
      const detail = error as { code?: string; message?: string };
      setImageDropError(detail.message ?? detail.code ?? "图片导入失败");
    }
  };

  // 注册编辑器实例供按需查询（PI-ED-10：getSelectionRef/getCursorRef/locateBlock 精确计算）。
  useEffect(() => {
    registerEditor(editor);
    return () => {
      if (editorInstanceRef.current === editor) editorInstanceRef.current = null;
      registerEditor(null);
    };
  }, [editor]);

  // REF-REFERENCE-REPAIR-01 F1.3：document 冒泡阶段 dragstart 后置回写引用 MIME。PM 内置 dragstart
  // handler 注册在编辑器 DOM（冒泡阶段），document 冒泡监听在其之后执行；此时 PM 已完成
  // clearData() + 重写 text/html·text/plain，回写覆盖自定义 MIME 与 text/plain marker，使拖出编辑器
  // 到 AG 输入面仍能经 drag session store 形成 ReferenceToken。
  useEffect(() => {
    if (!editor) return;
    const onDocDragStart = (event: DragEvent) => {
      const ref = dragStartSelectionRef;
      if (!ref) return;
      const path = edStore.getSnapshot().openPath ?? "";
      if (!path || !event.dataTransfer) return;
      startReferenceDrag(event.dataTransfer, makeSelectionReference({ ...ref, path }));
      dragStartSelectionRef = null;
    };
    document.addEventListener("dragstart", onDocDragStart);
    return () => document.removeEventListener("dragstart", onDocDragStart);
  }, [editor]);

  // 光标激活周期只属于当前文档与当前分页几何。文档/格式/缩放切换必须显式清除；
  // 纯选区事务（键盘导航、程序化定位）结束旧周期，但保留同一 mouseup 尚待最终选区同步的待建周期。
  useEffect(() => {
    if (!editor) return;
    const view = editor.view;
    clearPointerActivatedInputScrollGuard(view);
    const releaseOnSelectionOnlyTransaction = ({ transaction }: { transaction: Transaction }) => {
      if (!transaction.docChanged && transaction.selectionSet) {
        dropPointerActivatedInputScrollGuard(view);
      }
    };
    editor.on("transaction", releaseOnSelectionOnlyTransaction);
    return () => {
      editor.off("transaction", releaseOnSelectionOnlyTransaction);
      clearPointerActivatedInputScrollGuard(view);
    };
  }, [
    editor,
    openPath,
    loadSeq,
    editFormat,
    contentFormat,
    zoom,
    paperPrefs?.paper,
    paperPrefs?.orientation,
  ]);

  // WebView/系统快捷键可能先于 ProseMirror 消费 ⌘J；在窗口捕获阶段补一层，仅限当前编辑器区域生效。
  useEffect(() => {
    if (!editor) return;
    const onAssistShortcut = (event: KeyboardEvent) => {
      if (!(event.metaKey || event.ctrlKey) || event.altKey) return;
      const key = event.key.toLowerCase();
      if (key !== "j" && key !== "k") return;
      const host = hostRef.current;
      if (!host) return;
      const target = event.target instanceof HTMLElement ? event.target : null;
      const active = document.activeElement;
      if (key === "k" && assistStore.getSnapshot().kind === "inline") {
        event.preventDefault();
        event.stopPropagation();
        finishCurrentAssist("dismiss");
        return;
      }
      if (target?.closest(".assist-pop")) return;
      if (!(target && host.contains(target)) && !(active && host.contains(active))) return;
      event.preventDefault();
      event.stopPropagation();
      if (key === "j") openAutocompleteAssist(editor.view);
      else openInlineAssist(editor.view);
    };
    window.addEventListener("keydown", onAssistShortcut, true);
    return () => window.removeEventListener("keydown", onAssistShortcut, true);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- 触发时从 edStore/ref 读取当前上下文，避免 useEditor 初始化闭包过期。
  }, [editor, openPath, loadSeq]);

  // 文档切换/重载会使 assist 锚失效；旧请求返回后也不应再展示或应用。右键菜单同步关闭。
  useEffect(() => {
    finishCurrentAssist("dismiss");
    setCtxMenu(null);
  }, [openPath, loadSeq]);

  // AG-ASSIST 交互锚生命周期：点击浮层外即视为用户放弃；旧请求后续返回由 requestId 失效拦截。
  useEffect(() => {
    if (!editor) return;
    const onPointerDown = (event: PointerEvent) => {
      if (assistStore.getSnapshot().kind === "none") return;
      if (isAssistFloatingTarget(event.target)) return;
      finishCurrentAssist("dismiss");
    };
    window.addEventListener("pointerdown", onPointerDown, true);
    return () => window.removeEventListener("pointerdown", onPointerDown, true);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- 监听器只需读取当前 assistStore；openPath/loadSeq 变化时由上方 effect 先关闭。
  }, [editor, openPath, loadSeq]);

  // AG-ASSIST 交互锚生命周期：键盘移动光标或改变选区，不再允许旧候选继续展示或应用。
  useEffect(() => {
    if (!editor) return;
    editor.on("selectionUpdate", finishAssistIfInteractionTargetChanged);
    return () => {
      editor.off("selectionUpdate", finishAssistIfInteractionTargetChanged);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- 事件触发时按当前 ED/assistStore 快照判定，不持有业务状态副本。
  }, [editor, openPath, loadSeq]);

  // AG-ASSIST 视觉锚重定位：滚动/窗口变化只更新浮层位置，不改变逻辑锚与候选。
  useEffect(() => {
    if (!editor) return;
    const paper = paperRef.current;
    let raf = 0;
    const scheduleReposition = () => {
      if (raf) cancelAnimationFrame(raf);
      raf = requestAnimationFrame(() => {
        raf = 0;
        repositionCurrentAssist(editor.view);
      });
    };
    paper?.addEventListener("scroll", scheduleReposition, { passive: true });
    window.addEventListener("resize", scheduleReposition);
    return () => {
      if (raf) cancelAnimationFrame(raf);
      paper?.removeEventListener("scroll", scheduleReposition);
      window.removeEventListener("resize", scheduleReposition);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- 重定位读取当前 assistStore；openPath/loadSeq 变化已先关闭旧锚。
  }, [editor, openPath, loadSeq]);

  // 编辑区缩放只改变视觉锚坐标；浮层尺寸保持应用 UI 尺寸并重算位置。
  useEffect(() => {
    if (!editor) return;
    const raf = requestAnimationFrame(() => repositionCurrentAssist(editor.view));
    return () => cancelAnimationFrame(raf);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- zoom 变化后按当前逻辑锚重算视觉位置。
  }, [editor, zoom, openPath, loadSeq]);

  // 大纲投影（PI-ED-12）：事务后提取标题树发布（store 内相等跳过，不引发无谓重渲染）。
  useEffect(() => {
    if (!editor) return;
    const publish = () => outlineStore.set(extractOutline(editor.state.doc));
    publish();
    editor.on("transaction", publish);
    return () => {
      editor.off("transaction", publish);
      outlineStore.reset();
    };
  }, [editor]);

  // BDocx 首轮分页 settled 门禁：只接受当前活动 loadSeq；迟到的旧测量不能揭示新文档。
  useEffect(() => {
    if (!editor) return;
    const onTransaction = ({ transaction }: { transaction: Transaction }) => {
      if (!isPaginationSettledTransaction(transaction)) return;
      const pendingLoadSeq = pendingPaginationLoadSeqRef.current;
      const current = edStore.getSnapshot();
      if (
        pendingLoadSeq > 0
        && current.loadSeq === pendingLoadSeq
        && current.openPath === lastPathRef.current
      ) {
        pendingPaginationLoadSeqRef.current = 0;
        setSettledLoadSeq(pendingLoadSeq);
        if (current.openPath) restorePendingScroll(current.openPath, pendingLoadSeq);
      }
    };
    editor.on("transaction", onTransaction);
    return () => {
      editor.off("transaction", onTransaction);
    };
  }, [editor]);

  // 载入活动会话：loadSeq 变更（loadDoc / 切换活动会话 setActivePath 投影）→ 先把上一文档的当前编辑序列化
  // 回写其会话（切标签不丢草稿），再 setContent 活动会话内容（不触发脏态）。
  // text/source-protected Markdown 按纯文本装载；结构化 Markdown、docx/html 按 HTML LogicalState 解析。
  useEffect(() => {
    if (!editor || loadSeq === 0) return;
    // 切换标签前：暂存上一活动文档的当前编辑内容到其会话（按其格式序列化）+ 滚动位置（N7）。
    const prev = lastPathRef.current;
    if (prev && prev !== openPath) {
      const prevSess = edStore.getSnapshot().sessions[prev];
      if (prevSess) edStore.updateContent(prev, serialize(editor, prevSess.editFormat, prevSess.contentFormat, prev));
      persistEditorSessionSnapshot(prev);
      if (paperRef.current) scrollMemo.set(prev, paperRef.current.scrollTop);
    }
    lastPathRef.current = openPath;
    // 选区持久化生命周期（ED-SELECTION-PERSIST-01）：持久化绑定文档——切走/加载其他文档即清除
    // 上文档的 lastNonEmpty 记录，避免残留位置错误渲染到新 doc（view 仍在旧 doc，清后 setContent）。
    clearPersistedSelection(editor.view);
    if (!openPath || loadedContent === null) {
      pendingPaginationLoadSeqRef.current = 0;
      pendingScrollRestoreRef.current = null;
      clearActiveEditorBinding();
      return;
    }
    const waitsForPagination = editFormat === "html";
    pendingPaginationLoadSeqRef.current = waitsForPagination ? loadSeq : 0;
    if (editFormat === "html" || editFormat === "html_file" || (editFormat === "markdown" && contentFormat === "html")) {
      // docx 编辑链 / .html 独立文件（PI-ED-13）：loadedContent 为 HTML，用 generateJSON 按 HTML 解析为
      // ProseMirror JSON；样式子集经扩展（TextAlign/Color/FontSize/Table 等）解析。
      // .html 源含 data-block-id 时经 BlockIdExtension parseHTML 恢复（blockId 跨保存稳定，REQ-ED-008）。
      editor.commands.setContent(generateJSON(loadedContent, buildEditorExtensions()), { emitUpdate: false });
    } else {
      // 纯文本或无法无损处理的 Markdown：源码保护，按行装载，不丢失原始语句。
      editor.commands.setContent(textToDoc(loadedContent), { emitUpdate: false });
    }
    bindActiveEditorDocument({ relPath: openPath, loadSeq, editFormat, contentFormat });
    // 分页呈现（BR-ED-DATA-003）：内容装载后只经一个命令同步开关/纸张并丢弃上一文档 plan。
    // 防止恢复链与视图偏好链分别重置几何；纯呈现层——不触发脏态（仅 setMeta，docChanged=false）。
    if (editFormat === "html" && openPath) {
      const p = edViewPrefsStore.getPrefs(openPath);
      editor.commands.syncPaginationLayout(true, getPageGeometry(p.paper, p.orientation));
    } else {
      editor.commands.syncPaginationLayout(false);
      setSettledLoadSeq(loadSeq);
    }
    // 搜索命中跳行：内容设入后定位到目标行（块），随后清除请求。
    const { jumpLine } = edStore.getSnapshot();
    if (jumpLine !== null) {
      pendingScrollRestoreRef.current = null;
      jumpToLine(editor, jumpLine);
      edStore.clearJump();
    } else {
      // N7：分页文档等首轮 settled 后恢复；连续流文档下一帧恢复。
      const memo = scrollMemo.get(openPath);
      if (memo !== undefined) {
        pendingScrollRestoreRef.current = { path: openPath, loadSeq, scrollTop: memo };
        if (!waitsForPagination) restorePendingScroll(openPath, loadSeq);
      } else {
        pendingScrollRestoreRef.current = null;
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps -- 仅 loadSeq 驱动载入；openPath/loadedContent/editFormat 在 loadSeq 变更时同步刷新，单独入依赖会引发重复 setContent
  }, [editor, loadSeq]);

  // 源码/富文本视图切换：进入源码模式时序列化当前编辑内容为 Markdown
  const prevSourceViewRef = useRef(sourceView);
  useEffect(() => {
    if (!editor || editFormat !== "markdown") return;
    if (sourceView && !prevSourceViewRef.current) {
      // 切换到源码视图：序列化当前 ProseMirror JSON 为 Markdown
      try {
        const result = serializeMarkdownDocument(editor.getJSON());
        const md = result.ok ? result.markdown : editor.getText();
        setSourceViewText(md);
        sourceViewTextRef.current = md;
      } catch {
        // Markdown 序列化失败 → 退回到纯文本
        setSourceViewText(editor.getText());
        sourceViewTextRef.current = editor.getText();
      }
    } else if (!sourceView && prevSourceViewRef.current && sourceViewTextRef.current) {
      // 从源码视图切回富文本：解析 Markdown 并重载
      try {
        const edited = sourceViewTextRef.current;
        const result = parseMarkdownForEditing(edited);
        if (result.ok && result.contentFormat === "html") {
          editor.commands.setContent(generateJSON(result.html!, buildEditorExtensions()), { emitUpdate: false });
          if (openPath) edStore.updateContent(openPath, result.html!);
        } else {
          // 源码含不支持语法 → 保留在源码模式，提示用户
          console.warn("[Editor] Markdown 源码含不支持语法，无法切回富文本视图");
        }
      } catch {
        console.warn("[Editor] 切回富文本失败");
      }
    }
    prevSourceViewRef.current = sourceView;
    // eslint-disable-next-line react-hooks/exhaustive-deps -- 仅 sourceView 变化时触发
  }, [sourceView]);

  const saveCurrent = async (): Promise<{ ok: boolean; message?: string; conflict?: boolean }> => {
    if (!editor) return { ok: false, message: "编辑器未就绪" };
    if (!isActiveEditorBoundTo(openPath, loadSeq, editFormat, contentFormat)) {
      return { ok: false, message: "编辑器未绑定到当前文档" };
    }
    if (tabState !== "dirty" && tabState !== "pending_diff" && tabState !== "saving") {
      return { ok: true }; // 无脏态，视为已是保存后状态（关闭前批量保存的空操作分支）
    }
    const saveSerialized = async (override = false) => {
      if (editFormat === "html") {
        return saveDocxDocument(serialize(editor, "html", "html", openPath), override);
      }
      if (editFormat === "html_file") {
        return saveDocument(serialize(editor, "html_file", "html", openPath), override);
      }
      if (editFormat === "markdown" && contentFormat === "html") {
        if (sourceView) {
          const source = sourceViewTextRef.current;
          const parsed = parseMarkdownForEditing(source);
          if (!parsed.ok || !parsed.html) {
            return { ok: false as const, message: "Markdown 源码包含当前富文本模式无法承载的语法，请保持源码模式后另存" };
          }
          return saveMarkdownDocument(source, parsed.html, "html", override);
        }
        const candidate = serializeMarkdownDocument(editor.getJSON());
        if (!candidate.ok) return { ok: false as const, message: candidate.message };
        return saveMarkdownDocument(candidate.markdown, editor.getHTML(), "html", override);
      }
      return saveDocument(serialize(editor, editFormat, contentFormat, openPath), override);
    };

    const result = await saveSerialized(false);
    if (!result.ok && result.conflict) {
      const confirmed = window.confirm("磁盘内容已在打开后变化。确定覆盖磁盘版本并保留当前编辑吗？");
      if (confirmed) return saveSerialized(true);
    }
    return result;
  };

  const exportCurrent = async (format: "pdf" | "docx" | "md") => {
    const saved = await saveCurrent();
    if (!saved.ok) return { ok: false, message: saved.message ?? "请先完成保存后再导出" };
    if (!openPath) return { ok: false, message: "没有可导出的文档" };
    try {
      const destination = await exportDoc(openPath, format);
      return { ok: true, message: `已导出：${destination}` };
    } catch (error) {
      const message = error as { message?: string };
      return { ok: false, message: message.message ?? "导出失败" };
    }
  };

  // 关闭前处理（APP-CLOSE-DIFF-LIFECYCLE-01）复用本编辑器的保存路由：注册当前 saveCurrent，
  // 卸载时解除注册（避免已卸载编辑器的处理器被误触发）。经 ref 保证始终调用最新闭包。
  const saveCurrentRef = useRef(saveCurrent);
  saveCurrentRef.current = saveCurrent;
  useEffect(() => {
    registerActiveSaveHandler(() => saveCurrentRef.current());
    return () => registerActiveSaveHandler(null);
  }, []);


  // 纸张偏好变化（工具栏切换）→ 实时应用分页几何（PI-ED-06，仅分页文档）。
  useEffect(() => {
    if (!editor || editFormat !== "html" || !paperPrefs) return;
    editor.commands.setPageGeometry(getPageGeometry(paperPrefs.paper, paperPrefs.orientation));
  }, [editor, editFormat, paperPrefs?.paper, paperPrefs?.orientation]); // eslint-disable-line react-hooks/exhaustive-deps -- paperPrefs 对象按字段依赖

  // #10 滚轮/触摸板缩放：ctrl+wheel（macOS 捏合手势合成 ctrlKey）→ 步进缩放；passive:false 以阻止页面缩放。
  useEffect(() => {
    const el = paperRef.current;
    if (!el) return;
    const onWheel = (e: WheelEvent) => {
      if (!e.ctrlKey && !e.metaKey) return;
      e.preventDefault();
      const path = edStore.getSnapshot().openPath;
      if (!path) return;
      const cur = edViewPrefsStore.getPrefs(path).zoom;
      edViewPrefsStore.setZoom(path, stepZoom(cur, e.deltaY < 0 ? ZOOM.step : -ZOOM.step));
    };
    el.addEventListener("wheel", onWheel, { passive: false });
    return () => el.removeEventListener("wheel", onWheel);
  }, [openPath === null]); // eslint-disable-line react-hooks/exhaustive-deps -- paper 容器随空态/编辑态切换重挂

  useEffect(() => setImageDropActive(false), [openPath, loadSeq]);

  const handlePaperImageDrag = (event: ReactDragEvent<HTMLDivElement>) => {
    if (!imageDropTransferKind(event.dataTransfer)) return;
    event.preventDefault();
    event.stopPropagation();
    event.dataTransfer.dropEffect = "copy";
    setImageDropActive(true);
  };

  const handlePaperImageDragLeave = (event: ReactDragEvent<HTMLDivElement>) => {
    const related = event.relatedTarget;
    if (related instanceof Node && event.currentTarget.contains(related)) return;
    setImageDropActive(false);
  };

  const handlePaperImageDrop = (event: ReactDragEvent<HTMLDivElement>) => {
    const transferKind = imageDropTransferKind(event.dataTransfer);
    if (!transferKind) return; // 文档内图片/富文本拖动继续交给 ProseMirror。
    event.preventDefault();
    event.stopPropagation();
    setImageDropActive(false);
    const activeEditor = editorInstanceRef.current;
    const anchor = activeEditor
      ? imageDropAnchorAtClientPoint(activeEditor, event.clientX, event.clientY)
      : null;
    if (!anchor) {
      setImageDropError("当前文档没有可用的图片插入锚点");
      return;
    }
    if (transferKind === "project_image") {
      const ref = readDocRefData(event.dataTransfer);
      if (!ref) {
        setImageDropError("项目图片拖放数据无效，请从文件树重新拖入");
        return;
      }
      if (!/\.(png|jpe?g)$/i.test(ref.relPath)) {
        setImageDropError("仅支持拖入 PNG、JPG 或 JPEG 图片");
        return;
      }
      void imageIngressRef.current([{ kind: "path", sourcePath: ref.relPath }], anchor);
      return;
    }
    const imageFiles = clipboardImageFiles(event.dataTransfer);
    const fileCount = dataTransferFileCount(event.dataTransfer);
    if (!imageFiles.length || imageFiles.length !== fileCount) {
      setImageDropError("仅支持拖入 PNG、JPG 或 JPEG 图片，且一次拖入不能混有其他文件");
      return;
    }
    void Promise.all(imageFiles.map(async (file, index): Promise<ImageIngressSource> => ({
      kind: "data",
      sourceName: clipboardImageName(file, index),
      sourceDataUrl: (await fileAsDataUrl(file)).replace(
        /^data:[^,]*,/i,
        `data:${clipboardImageMediaType(file)};base64,`,
      ),
    }))).then(
      (sources) => imageIngressRef.current(sources, anchor),
      (error) => setImageDropError(error instanceof Error ? error.message : "读取拖入图片失败"),
    );
  };

  /**
   * @GOV
   * codes:    BR-CORE-STATE-001-EFFECT-ED-ED-EDIT-001, BR-CORE-STATE-003-EFFECT-ED-ED-EDIT-001
   * type:     EFFECT
   * chain:    DE-DIFF, ED-EDIT, ED-DECOR
   * rules:    BR-CORE-STATE-001, BR-CORE-STATE-003
   * boundary: in=PendingDiffForOpenedLogicalState | out=ControlledLogicalStatePreviewAndGreenReview(noDiffCardLifecycleTransition)
   */
  // Pending diff 到达当前编辑器时先成为 ED 草稿内容；GreenReview 只标绿当前 after 内容。
  // 三态链路标注（A-CORE-C-T-01 §5.1）：
  //   deStore 变更（LogicalCandidate 投影层）→ 触发本 effect
  //   → applyEditOps（LogicalState 受控修改）
  //   → applyGreenReview（DisplayState 装饰重算）
  // 每一步都是"LogicalCandidate 变化 → 经受控接口修改 LogicalState → 显示态跟随刷新"，
  // 不是"DisplayState 变化触发 LogicalState"。
  useEffect(() => {
    if (!editor) return;
    const ed = edStore.getSnapshot();
    const open = ed.openPath;
    if (!isActiveEditorBoundTo(open, ed.loadSeq, ed.editFormat, ed.contentFormat)) return;
    for (const d of deStore.getSnapshot().diffs) {
      const hasStyleOps = Boolean(d.styleOpsPayload?.length);
      const hasBlockOps = Boolean(d.blockOpsPayload);
      const hasTableOps = Boolean(d.tableOpsPayload);
      const hasImageOps = Boolean(d.imageOpsPayload);
      let imageDiffFinalized = false;
      if (d.relPath !== open || d.state !== "pending" || d.draftApplied || d.draftSaved) continue;
      const baseContent = serialize(editor, editFormat, contentFormat, open);
      const syntheticAfter = isSyntheticAfter(d.after);
      const failDraftPreview = (reason: string) => {
        deStore.markDraftPreviewFailed(d.diffId, reason);
        if (d.previewContinuationRequired && d.patchKind === "replace_document") {
          void compensateCreateDocumentPreviewFailure(d.diffId);
        }
      };
      if (!d.blockId && !hasStyleOps && !hasBlockOps && !hasTableOps && !hasImageOps && syntheticAfter) {
        failDraftPreview("ED_DRAFT_CONTENT_MISSING");
        continue;
      }
      if (hasBlockOps && d.blockOpsPayload) {
        const session = open ? ed.sessions[open] : undefined;
        if (!session) {
          failDraftPreview("ED_BLOCK_OPERATION_SESSION_MISSING");
          continue;
        }
        const blockApplied = applyStructuredBlockOperations(
          editor,
          d.blockOpsPayload,
          "forward",
          {
            sessionVersion: session.sessionVersion,
            contentHash: session.contentHash,
          },
        );
        if (!blockApplied.ok) {
          failDraftPreview(blockApplied.reason);
          continue;
        }
      } else if (hasImageOps && d.imageOpsPayload) {
        const session = open ? ed.sessions[open] : undefined;
        const imageApplied = applyPendingImageDiffToEditor(
          editor,
          d,
          session,
          baseContent,
          tabState,
          () => serialize(editor, editFormat, contentFormat, open),
        );
        if (!imageApplied) {
          continue;
        }
        imageDiffFinalized = true;
      } else if (hasTableOps && d.tableOpsPayload) {
        const tableApplied = applyTableOperations(d.tableOpsPayload);
        if (!tableApplied.ok) {
          failDraftPreview(tableApplied.reason);
          continue;
        }
        deStore.setTableSnapshots(d.diffId, tableApplied.before, tableApplied.after);
      } else if (d.blockId && !syntheticAfter) {
        const applied = applyEditOps([{ op: "replace_block", blockId: d.blockId, text: d.after }]);
        if (!applied.ok) {
          failDraftPreview("ED_DRAFT_ANCHOR_LOST");
          continue;
        }
      } else if (!d.blockId && !hasStyleOps && !hasBlockOps && !hasTableOps && !hasImageOps && d.patchKind === "replace_document" && !syntheticAfter) {
        const applied = applyEditOps([{ op: "replace_document", content: d.after, format: editFormat, contentFormat }]);
        if (!applied.ok) {
          failDraftPreview("ED_DRAFT_REPLACE_FAILED");
          continue;
        }
      } else if (!d.blockId && !hasStyleOps && !hasBlockOps && !hasTableOps && !hasImageOps && !syntheticAfter) {
        failDraftPreview("ED_DRAFT_TARGET_MISSING");
        continue;
      }
      if (hasStyleOps && d.styleOpsPayload) {
        const styled = applyStyleOps(d.styleOpsPayload);
        if (!styled.ok) {
          failDraftPreview(styled.reason);
          continue;
        }
      }
      if (!imageDiffFinalized) {
        deStore.markDraftApplied(
          d.diffId,
          baseContent,
          tabState,
          serialize(editor, editFormat, contentFormat, open),
        );
      }
      // applyEditOps/结构化操作可能同步触发 onUpdate；显式写回同一序列化结果确保 Ack 所携带的
      // sessionVersion/contentHash 必定来自已完成的 LogicalState，而非 DiffProjection 的旧基线。
      const appliedContent = serialize(editor, editFormat, contentFormat, open);
      edStore.updateContent(open, appliedContent);
      const appliedSession = edStore.getSnapshot().sessions[open];
      const appliedDiff = deStore.getSnapshot().diffs.find((candidate) => candidate.diffId === d.diffId);
      if (
        appliedDiff?.previewContinuationRequired
        && appliedDiff.draftApplied
        && appliedSession?.content === appliedContent
      ) {
        void acknowledgePreviewAndContinue(d.diffId);
      }
    }

    const map = new Map<string, GreenReviewValue>();
    for (const d of deStore.getSnapshot().diffs) {
      const gr = toGreenReview(d.state);
      if (!gr || d.relPath !== open || d.draftSaved) continue;
      if (d.draftApplied && d.imageAffectedBlockIds?.length) {
        for (const blockId of d.imageAffectedBlockIds) map.set(blockId, gr);
        continue;
      }
      if (d.draftApplied && d.blockOpsPayload) {
        for (const operation of d.blockOpsPayload.operations) map.set(operation.blockId, gr);
        continue;
      }
      if (d.blockId && d.draftApplied) {
        if (d.styleOpsPayload?.length) {
          for (const op of d.styleOpsPayload) {
            if (op.blockId) map.set(op.blockId, gr);
          }
        }
        map.set(d.blockId, gr);
        continue;
      }
      if (d.draftApplied) {
        for (const blockId of currentBlockIds(editor)) map.set(blockId, gr);
      }
    }
    applyGreenReview(editor.view, map);
  }, [editor, diffs, editFormat, contentFormat, tabState, openPath, loadSeq]);

  // BR-CORE-STATE-003：background PendingDiff 在目标文档打开后立即迁移到 LogicalState 分流。
  // 迁移不裁决 DiffCard；紧随其后的预览 effect 把同一 payload 应用并显示绿色增量。
  useEffect(() => {
    if (!editor || !openPath || loadSeq === 0) return;
    for (const d of deStore.getSnapshot().diffs) {
      if (d.relPath !== openPath || d.draftMode !== "background" || d.state !== "pending") continue;
      const snapshot = typeof d.blockIndex === "number"
        ? getActiveBlockSnapshotByIndex(d.blockIndex)
        : null;
      if (d.patchKind === "replace_block" && !snapshot?.blockId && !d.blockId) continue;
      void promotePendingDiffOnOpen(d.diffId, snapshot?.blockId ?? d.blockId);
    }
  }, [editor, openPath, loadSeq]);

  /**
   * @GOV
   * codes:    BR-AG-STATE-003-RB-AG-AG-ASSIST-002
   * type:     RB
   * chain:    AG-ASSIST
   * rules:    BR-AG-STATE-003
   * term_ref: TERM-AG-003, TERM-AG-004
   * boundary: in=assistApplyIntent | out=applyEditOps(inlineReplaceOrCursorInsertAutocompleteInsertNoDiffCard)
   */
  /// AG-ASSIST 落地守则（BR-AG-STATE-003）：⌘K InlineAssist Apply 替换选区或在光标处插入、
  /// ⌘J Autocomplete 插入光标处，二者均经 ED 受控编辑接口（X-FORBID-005 不绕过），**不产 DiffCard、不进入 SM-DE-01**——
  /// 与 Chat Diff 链路（deStore）严格隔离。产出经 ag_request_assist（门禁前置 + 网关，SM-AG-03），
  /// 门禁不可用/失败时静默不提供（不打断编辑）。
  const applyInline = (text: string) => {
    const snap = assistStore.getSnapshot();
    if (!editor || snap.kind !== "inline" || !isAssistTargetCurrent(snap.target)) {
      finishCurrentAssist("anchor_lost");
      return;
    }
    const target = snap.target;
    const requestId = snap.requestId;
    assistStore.close();
    const applied = applyEditOps([{ op: "replace_text", blockId: target.blockId, start: target.start, end: target.end, text }]);
    if (requestId) finishAssistId(requestId, applied.ok ? "accept" : "anchor_lost");
  };
  const pickAutocomplete = (text: string) => {
    const snap = assistStore.getSnapshot();
    if (!editor || snap.kind !== "autocomplete" || !isAssistTargetCurrent(snap.target)) {
      finishCurrentAssist("anchor_lost");
      return;
    }
    const target = snap.target;
    const requestId = snap.requestId;
    assistStore.close();
    const applied = applyEditOps([{ op: "replace_text", blockId: target.blockId, start: target.start, end: target.end, text }]);
    if (requestId) finishAssistId(requestId, applied.ok ? "accept" : "anchor_lost");
  };

  // ⌘K 生成：经 ag_request_assist（门禁前置→网关单次产出，SM-AG-03）；失败/门禁不可用静默关闭（不打断编辑）。
  const generateInline = (instruction: string) => {
    const snap = assistStore.getSnapshot();
    if (snap.kind !== "inline" || !snap.target || !isAssistTargetCurrent(snap.target)) {
      finishCurrentAssist("anchor_lost");
      return;
    }
    const baseText = snap.inlinePreview ?? undefined;
    const requestId = assistStore.startRequest("inline");
    if (!requestId) return;
    const target = snap.target;
    // 模型参数按 inline-assist 能力偏好解析（BR-AG-DATA-007：跟随全局或显式指定）。
    const inlineModel = modelStore.resolveFor("inlineAssist");
    void requestAssist({
      requestId,
      kind: "inline",
      docPath: target.docPath,
      loadSeq: target.loadSeq,
      blockId: target.blockId,
      start: target.start,
      end: target.end,
      instruction,
      selection: target.mode === "selection" ? target.text : undefined,
      prefix: target.prefix,
      before: target.before,
      after: target.after,
      baseText,
      provider: inlineModel.provider,
      model: inlineModel.model,
    }).then((result) => {
      const candidate = result.candidate ?? result.candidates[0] ?? null;
      if (!assistStore.isCurrent(requestId, "inline")) {
        if (candidate) finishAssistId(requestId, "dismiss");
        return;
      } // 用户已切走或新请求覆盖，忽略
      if (!isAssistTargetCurrent(assistStore.getSnapshot().target)) {
        finishAssistId(requestId, "anchor_lost");
        assistStore.close();
        return;
      }
      if (candidate) assistStore.setInlinePreview(requestId, candidate);
      else assistStore.close();
    });
  };

  const dismissAssist = () => finishCurrentAssist("dismiss");

  // 未打开真实文档 → 诚实空态（不显示可编辑的假 untitled.md）。
  if (!openPath) {
    return (
      <div className="editor-host editor-empty">
        <p className="muted">从文件树打开一个文档开始编辑</p>
      </div>
    );
  }

  // 会话已建但内容未载入（大 docx 转换中）→ 加载骨架，立即反馈不阻塞交互（PI-ED-07 #7）。
  const loading = !isEditorContentReady(loadedContent, loadSeq, settledLoadSeq);

  // 编辑器右键（CTX-MENU-01）：富文本模式在正文区弹出上下文菜单；sourceView（textarea）保持无右键
  // （原生 ⌘C/⌘V 可用，避免选区读取与 TipTap 状态不一致）。选区态在触发瞬间读取——contextmenu 后
  // ProseMirror 选区可能被 DOM 重建，须先读后开。
  const onEditorContextMenu = (event: React.MouseEvent<HTMLDivElement>) => {
    if (!editor || sourceView) return;
    event.preventDefault();
    const { empty } = editor.state.selection;
    setCtxMenu({ x: event.clientX, y: event.clientY, hasSelection: !empty });
  };

  // 剪切/复制经 ProseMirror 剪贴板处理路径（execCommand 触发 PM 的 copy/cut 事件，X-FORBID-005
  // 不绕过编辑接口）；粘贴经 Web Clipboard API 读取后 insertContent 进编辑模型
  // （WKWebView 可用性为 G4 实测项，失败静默并保留 ⌘V 快捷键路径）。
  const doCut = () => {
    if (!editor) return;
    editor.view.focus();
    document.execCommand("cut");
  };
  const doCopy = () => {
    if (!editor) return;
    editor.view.focus();
    document.execCommand("copy");
  };
  const doPaste = () => {
    if (!editor) return;
    void navigator.clipboard
      .readText()
      .then((text) => {
        if (text) editor.chain().focus().insertContent(text).run();
      })
      .catch(() => {
        // 静默：读剪贴板失败不打断用户（保留 ⌘V 快捷键路径）。
      });
  };

  // 编辑器右键菜单项：有选区→剪切/复制/局部修改（TERM-AG-003）；无选区→粘贴/辅助续写（TERM-AG-004）。
  // AI 项复用 openInlineAssist/openAutocompleteAssist（门禁前置 + 静默语义，BR-AG-STATE-003），
  // 与 ⌘K/⌘J 快捷键行为一致。
  const ctxMenuItems: ContextMenuDef[] | null = (() => {
    if (!ctxMenu || !editor) return null;
    if (ctxMenu.hasSelection) {
      return [
        { type: "item", id: "cut", label: "剪切", onSelect: doCut },
        { type: "item", id: "copy", label: "复制", onSelect: doCopy },
        { type: "separator", id: "sep-ai" },
        { type: "item", id: "inline", label: "局部修改", onSelect: () => openInlineAssist(editor.view) },
      ];
    }
    return [
      { type: "item", id: "paste", label: "粘贴", onSelect: doPaste },
      { type: "separator", id: "sep-ai" },
      { type: "item", id: "autocomplete", label: "辅助续写", onSelect: () => openAutocompleteAssist(editor.view) },
    ];
  })();

  return (
    <div className="editor-host" ref={hostRef}>
      {/* 标签栏由 center TabStrip 统一承载（PI-FE-26）；冲突裁决移入底部状态栏（PI-ED-07 #4） */}
      <EditorToolbar
        editor={editor}
        zoom={zoom}
        onZoomIn={() => setZoom(stepZoom(zoom, ZOOM.step))}
        onZoomOut={() => setZoom(stepZoom(zoom, -ZOOM.step))}
        onZoomReset={() => setZoom(ZOOM.default)}
        paper={editFormat === "html" && paperPrefs ? paperPrefs.paper : undefined}
        orientation={editFormat === "html" && paperPrefs ? paperPrefs.orientation : undefined}
        onPaperChange={(p) => openPath && edViewPrefsStore.setPaper(openPath, p)}
        onOrientationChange={(o) => openPath && edViewPrefsStore.setOrientation(openPath, o)}
        localImageDropOnly={editFormat === "html"}
        editFormat={editFormat}
        sourceView={sourceView}
        onToggleSourceView={openPath ? () => edStore.setSourceView(openPath, !sourceView) : undefined}
        sourceText={sourceView ? sourceViewText : undefined}
        onSourceTextChange={(text) => {
          setSourceViewText(text);
          sourceViewTextRef.current = text;
          edStore.markDirty();
        }}
        onJumpToLine={(line) => {
          if (editor) jumpToLine(editor, line);
        }}
        exportTargets={exportTargetsForPath(openPath)}
        onExport={exportCurrent}
      />
      <div
        className={`editor-paper${imageDropActive ? " editor-paper-image-drop-active" : ""}`}
        ref={paperRef}
        aria-busy={loading}
        style={{ "--ed-zoom": String(zoom) } as CSSProperties}
        onContextMenu={onEditorContextMenu}
        onDragEnterCapture={handlePaperImageDrag}
        onDragOverCapture={handlePaperImageDrag}
        onDragLeaveCapture={handlePaperImageDragLeave}
        onDropCapture={handlePaperImageDrop}
      >
        {sourceView && editFormat === "markdown" ? (
          <textarea
            className="editor-source-view"
            value={sourceViewText}
            onChange={(e) => {
              setSourceViewText(e.target.value);
              sourceViewTextRef.current = e.target.value;
            }}
            spellCheck={false}
            aria-label="Markdown 源码编辑区"
          />
        ) : (
          <EditorContent editor={editor} className={loading ? "editor-content-loading" : undefined} />
        )}
        {loading && (
          <div className="editor-loading" role="status" aria-live="polite">
            <span className="editor-loading-spinner" aria-hidden="true" />
            <p className="muted">{converting ? "正在转换文档…" : "正在打开文档…"}</p>
            <span className="editor-loading-bar" aria-hidden="true" />
          </div>
        )}
        {imageDropError && (
          <div className="editor-image-drop-error" role="alert">
            {imageDropError}
            <button type="button" onClick={() => setImageDropError(null)}>关闭</button>
          </div>
        )}
      </div>
      <TableMoveControls
        editor={editor}
        enabled={!loading}
      />
      <ImageMoveControls
        editor={editor}
        enabled={!loading}
      />
      <EditorStatusBar
        editor={editor}
        tabState={tabState}
        converting={converting}
        styleNotices={styleNotices}
      />
      <InlineAssistPopover onGenerate={generateInline} onApply={applyInline} onDismiss={dismissAssist} />
      <AutocompleteOverlay
        onPick={pickAutocomplete}
        onDismiss={dismissAssist}
        isKeyboardScope={isAssistKeyboardScope}
      />
      {ctxMenu && ctxMenuItems && (
        <ContextMenu items={ctxMenuItems} x={ctxMenu.x} y={ctxMenu.y} onClose={() => setCtxMenu(null)} />
      )}
    </div>
  );
}
