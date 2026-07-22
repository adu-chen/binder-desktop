import { useEffect, useRef, type CSSProperties } from "react";
import { useEditor, EditorContent } from "@tiptap/react";
import type { EditorView } from "@tiptap/pm/view";
import type { Node as PMNode } from "@tiptap/pm/model";
import { generateJSON } from "@tiptap/core";
import { buildEditorExtensions } from "./extensions";
import { applyGreenReview, type GreenReviewValue } from "./extensions/DiffDecorationExtension";
import { edStore, useEdStore, type ContentFormat, type EditFormat } from "../../store/edStore";
import { deStore, toGreenReview, useDeStore } from "../../store/deStore";
import { assistStore, type AssistTarget } from "../../store/assistStore";
import {
  makePersistedSnapshot,
  persistSessionSnapshot,
  saveDocument,
  saveDocxDocument,
  saveMarkdownDocument,
} from "../../services/edService";
import { serializeMarkdownDocument } from "./markdown/markdownCodec";
import {
  registerEditor,
  bindActiveEditorDocument,
  clearActiveEditorBinding,
  getSelectionRef,
  getCursorRef,
  getActiveBlockSnapshotById,
  locateBlock,
  applyEditOps,
  isActiveEditorBoundTo,
  writeSelectionRefData,
  registerActiveSaveHandler,
} from "../../services/edLocate";
import { applyStyleOps } from "../../services/edStyleOps";
import { outlineStore, extractOutline } from "../../store/outlineStore";
import { finishAssist, requestAssist, type AssistFinishEvent } from "../../services/agService";
import { InlineAssistPopover } from "../ag/InlineAssistPopover";
import { AutocompleteOverlay } from "../ag/AutocompleteOverlay";
import { EditorToolbar } from "./EditorToolbar";
import { EditorStatusBar } from "./EditorStatusBar";
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

let sessionSnapshotPersistTimer: ReturnType<typeof globalThis.setTimeout> | null = null;

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
  if (sessionSnapshotPersistTimer) globalThis.clearTimeout(sessionSnapshotPersistTimer);
  sessionSnapshotPersistTimer = globalThis.setTimeout(() => {
    sessionSnapshotPersistTimer = null;
    persistEditorSessionSnapshot(path);
  }, 500);
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
  const { openPath, tabState, loadSeq, loadedContent, editFormat, contentFormat, converting } = useEdStore();
  const { diffs } = useDeStore(); // GreenReview 同步用（标签状态投影已移至 center TabStrip）
  const hostRef = useRef<HTMLDivElement>(null);
  const paperRef = useRef<HTMLDivElement>(null);
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
      // 选区拖拽携带位置引用（PI-ED-10 #6，BR-ED-DATA-004/TERM-ED-005）：dragstart 时把 SelectionRef
      // （字符锚 + 文本 + 来源路径）写入自定义 MIME——拖入 AG 输入面成为位置引用；编辑器内拖动仍走
      // ProseMirror 原生移动（不阻止默认，返回 false）。
      handleDOMEvents: {
        dragstart: (_view, event) => {
          const ref = getSelectionRef();
          if (ref && event.dataTransfer) {
            const path = edStore.getSnapshot().openPath ?? "";
            writeSelectionRefData(event.dataTransfer, { ...ref, path });
          }
          return false; // 不接管：器内拖动保持原生行为
        },
        copy: (_view, event) => {
          const ref = getSelectionRef();
          if (!ref || !event.clipboardData) return false;
          const path = edStore.getSnapshot().openPath ?? "";
          writeSelectionRefData(event.clipboardData, { ...ref, path });
          event.preventDefault();
          return true;
        },
      },
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

  // 注册编辑器实例供按需查询（PI-ED-10：getSelectionRef/getCursorRef/locateBlock 精确计算）。
  useEffect(() => {
    registerEditor(editor);
    return () => registerEditor(null);
  }, [editor]);

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

  // 文档切换/重载会使 assist 锚失效；旧请求返回后也不应再展示或应用。
  useEffect(() => {
    finishCurrentAssist("dismiss");
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
    if (!openPath || loadedContent === null) {
      clearActiveEditorBinding();
      return;
    }
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
    // 分页呈现（BR-ED-DATA-003）：docx 编辑（html）默认开分页，md/纯文本关（人类决策 2026-07-02）。
    // 纸张几何取该文档视图偏好（PI-ED-06）。纯呈现层——不触发脏态（仅 setMeta，docChanged=false）。
    if (editFormat === "html" && openPath) {
      const p = edViewPrefsStore.getPrefs(openPath);
      editor.commands.setPageGeometry(getPageGeometry(p.paper, p.orientation));
    }
    editor.commands.setPagination(editFormat === "html");
    // 搜索命中跳行：内容设入后定位到目标行（块），随后清除请求。
    const { jumpLine } = edStore.getSnapshot();
    if (jumpLine !== null) {
      jumpToLine(editor, jumpLine);
      edStore.clearJump();
    } else {
      // N7：无跳行请求时恢复该标签上次滚动位置（下一帧，待内容布局）。
      const memo = scrollMemo.get(openPath);
      if (memo !== undefined) {
        requestAnimationFrame(() => {
          if (paperRef.current && lastPathRef.current === openPath) paperRef.current.scrollTop = memo;
        });
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps -- 仅 loadSeq 驱动载入；openPath/loadedContent/editFormat 在 loadSeq 变更时同步刷新，单独入依赖会引发重复 setContent
  }, [editor, loadSeq]);

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
      if (d.relPath !== open || d.state !== "pending" || d.draftApplied || d.draftSaved) continue;
      const baseContent = serialize(editor, editFormat, contentFormat, open);
      const syntheticAfter = isSyntheticAfter(d.after);
      if (!d.blockId && !hasStyleOps && syntheticAfter) {
        deStore.setLocalView(d.diffId, "error", { errorCode: "ED_DRAFT_CONTENT_MISSING" });
        continue;
      }
      if (d.blockId && !syntheticAfter) {
        const applied = applyEditOps([{ op: "replace_block", blockId: d.blockId, text: d.after }]);
        if (!applied.ok) {
          deStore.setLocalView(d.diffId, "error", { errorCode: "ED_DRAFT_ANCHOR_LOST" });
          continue;
        }
      } else if (!d.blockId && !hasStyleOps && d.patchKind === "replace_document" && !syntheticAfter) {
        const applied = applyEditOps([{ op: "replace_document", content: d.after, format: editFormat, contentFormat }]);
        if (!applied.ok) {
          deStore.setLocalView(d.diffId, "error", { errorCode: "ED_DRAFT_REPLACE_FAILED" });
          continue;
        }
      } else if (!d.blockId && !hasStyleOps && !syntheticAfter) {
        deStore.setLocalView(d.diffId, "error", { errorCode: "ED_DRAFT_TARGET_MISSING" });
        continue;
      }
      if (hasStyleOps && d.styleOpsPayload) {
        const styled = applyStyleOps(d.styleOpsPayload);
        if (!styled.ok) {
          deStore.setLocalView(d.diffId, "error", { errorCode: styled.reason });
          continue;
        }
      }
      deStore.markDraftApplied(d.diffId, baseContent, tabState, serialize(editor, editFormat, contentFormat, open));
    }

    const map = new Map<string, GreenReviewValue>();
    for (const d of deStore.getSnapshot().diffs) {
      const gr = toGreenReview(d.state);
      if (!gr || d.relPath !== open || d.draftSaved) continue;
      if (d.blockId && d.draftApplied) {
        map.set(d.blockId, gr);
        continue;
      }
      if (d.draftApplied) {
        for (const blockId of currentBlockIds(editor)) map.set(blockId, gr);
      }
    }
    applyGreenReview(editor.view, map);
  }, [editor, diffs, editFormat, contentFormat, tabState, openPath, loadSeq]);

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
  const loading = loadedContent === null;

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
      />
      <div className="editor-paper" ref={paperRef} style={{ "--ed-zoom": String(zoom) } as CSSProperties}>
        {loading ? (
          <div className="editor-loading" role="status" aria-live="polite">
            <span className="editor-loading-spinner" aria-hidden="true" />
            <p className="muted">{converting ? "正在转换文档…" : "正在打开文档…"}</p>
            <span className="editor-loading-bar" aria-hidden="true" />
          </div>
        ) : (
          <EditorContent editor={editor} />
        )}
      </div>
      <EditorStatusBar
        editor={editor}
        tabState={tabState}
        converting={converting}
      />
      <InlineAssistPopover onGenerate={generateInline} onApply={applyInline} onDismiss={dismissAssist} />
      <AutocompleteOverlay
        onPick={pickAutocomplete}
        onDismiss={dismissAssist}
        isKeyboardScope={isAssistKeyboardScope}
      />
    </div>
  );
}
