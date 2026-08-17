import { useEffect, useRef, useState, type ClipboardEvent, type KeyboardEvent } from "react";
import { Node, mergeAttributes, type JSONContent } from "@tiptap/core";
import { EditorContent, useEditor, type Editor as TipTapEditor } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import {
  activeSession,
  agStore,
  emptyComposerDraft,
  useAgStore,
  type AgentComposerDraft,
} from "../../store/agStore";
import { modelKey, modelStore, useModelStore } from "../../store/modelStore";
import { cancelActiveTurn, startTurn } from "../../services/agService";
import { useWsStore } from "../../store/wsStore";
import {
  DOC_REF_MIME,
  SELECTION_REF_MIME,
  readDocRefData,
  readSelectionRefData,
} from "../../services/edLocate";
import {
  REFERENCE_MIME,
  coerceReferencePayload,
  dataTransferHasAnyType,
  makeDocReference,
  makeReferenceToken,
  makeSelectionReference,
  promptTextFromParts,
  referenceFromPlainTextPaste,
  referenceKey,
  registerReferenceInsertHandler,
  type MessageContentPart,
  type ReferencePayload,
  type ReferenceTokenAttrs,
} from "../../services/referencePayload";
import { DRAG_SESSION_MIME, resolveReferenceDrag } from "../../services/referenceDrag";
import { ModelConfig } from "./ModelConfig";

/**
 * @GOV
 * codes:    BR-REF-DATA-004-AG-REF-INJECT-001
 * type:     DATA
 * chain:    REF-INJECT
 * rules:    BR-REF-DATA-004
 * term_ref: TERM-REF-004, TERM-REF-003
 * boundary: in=ReferencePayload | out=ReferenceTokenInlineAtom
 */
const ReferenceTokenNode = Node.create({
  name: "referenceToken",
  group: "inline",
  inline: true,
  atom: true,
  selectable: false,
  draggable: true,

  addAttributes() {
    return {
      refId: {
        default: null,
        parseHTML: (element: HTMLElement) => element.getAttribute("data-ref-id"),
        renderHTML: (attrs: Record<string, unknown>) => ({ "data-ref-id": String(attrs.refId ?? "") }),
      },
      kind: {
        default: null,
        parseHTML: (element: HTMLElement) => element.getAttribute("data-ref-kind"),
        renderHTML: (attrs: Record<string, unknown>) => ({ "data-ref-kind": String(attrs.kind ?? "") }),
      },
      label: {
        default: "引用",
        parseHTML: (element: HTMLElement) => element.getAttribute("data-ref-label") ?? "引用",
        renderHTML: (attrs: Record<string, unknown>) => ({ "data-ref-label": String(attrs.label ?? "引用") }),
      },
      payload: {
        default: null,
        parseHTML: (element: HTMLElement) => {
          const raw = element.getAttribute("data-ref-payload");
          if (!raw) return null;
          try {
            return JSON.parse(decodeURIComponent(raw));
          } catch {
            return null;
          }
        },
        renderHTML: (attrs: Record<string, unknown>) => ({
          "data-ref-payload": encodeURIComponent(JSON.stringify(attrs.payload ?? null)),
        }),
      },
    };
  },

  parseHTML() {
    return [{ tag: "span[data-reference-token]" }];
  },

  renderHTML({ HTMLAttributes, node }) {
    const label = String(node.attrs.label ?? "引用");
    const kind = String(node.attrs.kind ?? "unknown");
    return [
      "span",
      mergeAttributes(HTMLAttributes, {
        "data-reference-token": "true",
        class: `ref-token ref-token-${kind}`,
      }),
      label,
    ];
  },
});

interface MentionQuery {
  query: string;
  from: number;
  to: number;
}

const REFERENCE_DRAG_TYPES = [
  REFERENCE_MIME,
  DRAG_SESSION_MIME,
  SELECTION_REF_MIME,
  DOC_REF_MIME,
  "text/uri-list",
  "text/plain",
] as const;

function activeMentionQuery(editor: TipTapEditor): MentionQuery | null {
  const { $from } = editor.state.selection;
  const before = $from.parent.textBetween(0, $from.parentOffset, undefined, " ");
  const at = before.lastIndexOf("@");
  if (at < 0) return null;
  const prev = at === 0 ? "" : before[at - 1];
  if (prev && !/\s/.test(prev)) return null;
  const query = before.slice(at + 1);
  if (/\s/.test(query)) return null;
  return { query, from: $from.start() + at, to: $from.pos };
}

function attrsFromNode(node: JSONContent): ReferenceTokenAttrs | null {
  const attrs = node.attrs as Partial<ReferenceTokenAttrs> | undefined;
  if (!attrs || typeof attrs.refId !== "string" || typeof attrs.label !== "string") return null;
  const payload = coerceReferencePayload(attrs.payload);
  if (!payload || attrs.kind !== payload.kind) return null;
  return { refId: attrs.refId, kind: payload.kind, label: attrs.label, payload };
}

function appendText(parts: MessageContentPart[], text: string) {
  if (!text) return;
  const last = parts[parts.length - 1];
  if (last?.type === "text") last.text += text;
  else parts.push({ type: "text", text });
}

function serializeEditor(editor: TipTapEditor): {
  doc: JSONContent;
  parts: MessageContentPart[];
  references: ReferencePayload[];
  tokens: ReferenceTokenAttrs[];
  prompt: string;
} {
  const parts: MessageContentPart[] = [];
  const tokens: ReferenceTokenAttrs[] = [];
  const references: ReferencePayload[] = [];
  const seen = new Set<string>();
  const doc = editor.getJSON();

  const visitInline = (node: JSONContent) => {
    if (node.type === "text") {
      appendText(parts, node.text ?? "");
      return;
    }
    if (node.type === "referenceToken") {
      const token = attrsFromNode(node);
      if (!token) return;
      parts.push({ type: "reference", ...token });
      tokens.push(token);
      const key = referenceKey(token.payload);
      if (!seen.has(key)) {
        references.push(token.payload);
        seen.add(key);
      }
      return;
    }
    node.content?.forEach(visitInline);
  };

  doc.content?.forEach((block, index) => {
    if (index > 0) appendText(parts, "\n");
    block.content?.forEach(visitInline);
  });

  return { doc, parts, references, tokens, prompt: promptTextFromParts(parts) };
}

function tokenNode(token: ReferenceTokenAttrs): JSONContent {
  return { type: "referenceToken", attrs: token };
}

function safeFocusEditor(editor: TipTapEditor) {
  try {
    editor.view.focus();
  } catch {
    // Focus is best-effort for drag/drop and paste; insertion still uses the current selection.
  }
}

// F7：@ 候选仅可读文档（md/markdown/docx/doc/html/txt/rtf），资源文件不进入引用候选。
const READABLE_DOC_EXT = new Set(["md", "markdown", "docx", "doc", "html", "htm", "txt", "rtf"]);
// ExternalContentRef（TERM-REF-005 / BR-SYS-DATA-007）：有可用只读解析路径（Web File API 纯文本）的扩展名。
const EXTERNAL_READABLE_EXT = new Set(["md", "markdown", "txt", "html", "htm", "rtf"]);
/** 外部文件内容注入上限（防御性截断；仅当前回合 data-only，不落盘/不转存）。 */
const EXTERNAL_CONTENT_CAP = 20000;

function isReadableExternalFileName(name: string): boolean {
  const dot = name.lastIndexOf(".");
  if (dot < 0) return false;
  return EXTERNAL_READABLE_EXT.has(name.slice(dot + 1).toLowerCase());
}

function fileReferenceFromTransfer(dataTransfer: DataTransfer): Promise<ReferencePayload | null> {
  const files = Array.from(dataTransfer.files ?? []);
  if (files.length === 0) return Promise.resolve(null);
  const file = files[0];
  const base: ReferencePayload = { kind: "file", name: file.name, size: file.size };
  // ExternalContentRef（BR-SYS-DATA-007 / TERM-REF-005）：可读纯文本扩展名经 Web File API 只读解析内容
  // 进当前回合（data-only、不扫描目录/不写盘/不转存）；不可读扩展名仅取元数据，不伪称已读。
  if (!isReadableExternalFileName(file.name)) return Promise.resolve(base);
  return file
    .text()
    .then((text) =>
      text.length > EXTERNAL_CONTENT_CAP
        ? { ...base, readContent: text.slice(0, EXTERNAL_CONTENT_CAP) }
        : { ...base, readContent: text },
    )
    .catch(() => base); // 读取失败：保留元数据，不伪称已读
}

function payloadFromTransfer(dataTransfer: DataTransfer): ReferencePayload | null {
  // REF-REFERENCE-REPAIR-01 F1：优先解析统一 drag session 通道（REFERENCE_MIME → DRAG_SESSION_MIME →
  // text/plain marker，命中即消费清理），再回落旧 selection/doc MIME 与 uri/plain 分类。
  // 外部文件（含内容读取）由 onPaste/onDrop 的独立异步路径处理，不在此同步链。
  const sessionResolved = resolveReferenceDrag(dataTransfer);
  if (sessionResolved) return sessionResolved;
  const selection = readSelectionRefData(dataTransfer);
  if (selection) return makeSelectionReference(selection);
  const doc = readDocRefData(dataTransfer);
  if (doc) return makeDocReference(doc.relPath);
  const uri = dataTransfer.getData("text/uri-list");
  const plain = dataTransfer.getData("text/plain");
  return referenceFromPlainTextPaste(uri || plain);
}

/**
 * Composer（A-SYS-C-T-02 §4.6 · PI-AG-REF-TOKEN-01）。
 * 输入区使用 TipTap inline atom 承载 ReferenceToken：拖拽/粘贴/@mention 都进入正文流；草稿 chip/tray
 * 只是引用摘要，发送后清空，已发送消息用同一 MessageContentPart 渲染。
 */
export function Composer() {
  const agState = useAgStore();
  const { sending } = agState;
  const session = activeSession(agState);
  const { models, selectedKey } = useModelStore();
  const currentModel = modelStore.current();
  const { docs } = useWsStore();

  const [draft, setDraft] = useState<AgentComposerDraft>(() => session.composerDraft ?? emptyComposerDraft());
  const [atQuery, setAtQuery] = useState<MentionQuery | null>(null);
  const [modelPop, setModelPop] = useState(false);
  const [configOpen, setConfigOpen] = useState(false);
  const [gateNotice, setGateNotice] = useState(false);
  const draftSessionIdRef = useRef(session.id);
  const applyingSessionDraftRef = useRef(false);

  const refreshDraft = (editor: TipTapEditor) => {
    if (applyingSessionDraftRef.current) return;
    const next = serializeEditor(editor);
    const nextDraft: AgentComposerDraft = { doc: next.doc, tokens: next.tokens, prompt: next.prompt };
    setDraft(nextDraft);
    agStore.setComposerDraft(draftSessionIdRef.current, nextDraft);
    setAtQuery(activeMentionQuery(editor));
  };

  const editor = useEditor({
    extensions: [StarterKit, ReferenceTokenNode],
    content: draft.doc,
    editorProps: {
      attributes: {
        class: "composer-prosemirror",
        role: "textbox",
        "aria-label": "Agent 输入",
        "data-placeholder": "描述你想要的修改…（@ 引用文档 · Enter 发送 · Shift+Enter 换行）",
      },
    },
    onCreate: ({ editor }) => refreshDraft(editor),
    onUpdate: ({ editor }) => refreshDraft(editor),
    onSelectionUpdate: ({ editor }) => setAtQuery(activeMentionQuery(editor)),
  });

  useEffect(() => {
    if (!editor || draftSessionIdRef.current === session.id) return;
    const latestSession = agStore.getSnapshot().sessions.find((s) => s.id === session.id);
    const nextDraft = latestSession?.composerDraft ?? emptyComposerDraft();
    applyingSessionDraftRef.current = true;
    editor.commands.setContent(nextDraft.doc, { emitUpdate: false });
    applyingSessionDraftRef.current = false;
    draftSessionIdRef.current = session.id;
    setDraft(nextDraft);
    setAtQuery(activeMentionQuery(editor));
    setModelPop(false);
  }, [editor, session.id]);

  useEffect(() => {
    if (session.mode === "build") agStore.setMode("edit");
  }, [session.id, session.mode]);

  // F7：@ 候选仅可读文档（见模块级 READABLE_DOC_EXT），资源文件不进入引用候选。
  const isReadableReferenceDoc = (relPath: string): boolean => {
    const dot = relPath.lastIndexOf(".");
    if (dot < 0) return false;
    return READABLE_DOC_EXT.has(relPath.slice(dot + 1).toLowerCase());
  };
  const fileDocs = docs.filter((d) => !d.isDir && isReadableReferenceDoc(d.relPath));
  const atMatches = atQuery
    ? fileDocs.filter((d) => d.relPath.toLowerCase().includes(atQuery.query.toLowerCase())).slice(0, 8)
    : [];

  const insertReferencePayload = (payload: ReferencePayload, range?: { from: number; to: number }, pos?: number) => {
    if (!editor) return;
    const token = makeReferenceToken(payload);
    safeFocusEditor(editor);
    const chain = editor.chain();
    if (pos !== undefined) chain.setTextSelection(pos);
    if (range) chain.deleteRange(range);
    chain.insertContent([tokenNode(token), { type: "text", text: " " }]).run();
    setAtQuery(null);
  };

  // 侧栏面板（记忆库等）经引用注入总线注入引用（BR-SYS-DATA-007 / TERM-AST-002，data-only）。
  useEffect(() => {
    if (!editor) return;
    registerReferenceInsertHandler(insertReferencePayload);
    return () => registerReferenceInsertHandler(() => {});
  }, [editor, insertReferencePayload]);

  const selectAtMention = (relPath: string) => {
    if (!atQuery) return;
    insertReferencePayload(makeDocReference(relPath), { from: atQuery.from, to: atQuery.to });
  };

  const removeReferenceToken = (refId: string) => {
    if (!editor) return;
    let range: { from: number; to: number } | null = null;
    editor.state.doc.descendants((node, pos) => {
      if (node.type.name === "referenceToken" && node.attrs.refId === refId) {
        range = { from: pos, to: pos + node.nodeSize };
        return false;
      }
      return true;
    });
    if (range) {
      safeFocusEditor(editor);
      editor.chain().deleteRange(range).run();
    }
  };

  const send = () => {
    if (!editor || sending) return;
    // AG-1.0-DECISION-CARD-01：存在待裁决的任务级请求时，输入门控——不发起新回合，
    // 必须先完成裁决（DecisionBar 按钮），避免"确认"等文本被当作新编辑请求（AG_EDIT_TOOL_REQUIRED 死循环）。
    if (agStore.getSnapshot().pendingDecision) {
      setGateNotice(true);
      return;
    }
    const snapshot = serializeEditor(editor);
    if (!snapshot.prompt.trim() && snapshot.references.length === 0) return;
    const sendModel = modelStore.resolveFor("agentEdit");
    void startTurn(snapshot.prompt, {
      references: snapshot.references,
      displayParts: snapshot.parts,
      provider: sendModel.provider,
      model: sendModel.model,
    });
    editor.commands.clearContent(true);
    const emptyDraft = emptyComposerDraft();
    setDraft(emptyDraft);
    agStore.setComposerDraft(draftSessionIdRef.current, emptyDraft);
    setAtQuery(null);
  };

  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    if (!editor) return;
    if (e.key === "Escape" && atQuery) {
      setAtQuery(null);
      return;
    }
    if (e.key !== "Enter") return;
    if (e.nativeEvent.isComposing || e.keyCode === 229) return;
    if (atQuery && atMatches.length && !e.shiftKey) {
      e.preventDefault();
      selectAtMention(atMatches[0].relPath);
      return;
    }
    if (e.shiftKey) return;
    e.preventDefault();
    send();
  };

  const onPaste = (e: ClipboardEvent<HTMLDivElement>) => {
    const data = e.clipboardData;
    if (!data) return;
    // 含外部文件：先阻止默认粘贴，再异步只读解析（ExternalContentRef，BR-SYS-DATA-007）。
    if ((data.files?.length ?? 0) > 0) {
      e.preventDefault();
      e.stopPropagation();
      void fileReferenceFromTransfer(data).then((payload) => {
        if (payload) insertReferencePayload(payload);
      });
      return;
    }
    const payload = payloadFromTransfer(data);
    if (!payload) return;
    e.preventDefault();
    e.stopPropagation();
    insertReferencePayload(payload);
  };

  const onDragOver = (e: React.DragEvent) => {
    if (dataTransferHasAnyType(e.dataTransfer, REFERENCE_DRAG_TYPES)) {
      e.preventDefault();
      e.stopPropagation();
      e.dataTransfer.dropEffect = "copy";
    }
  };

  const onDrop = (e: React.DragEvent) => {
    if (!editor) return;
    let pos: number | undefined;
    if (typeof document.elementFromPoint === "function") {
      try {
        pos = editor.view.posAtCoords({ left: e.clientX, top: e.clientY })?.pos;
      } catch {
        pos = undefined;
      }
    }
    const data = e.dataTransfer;
    // 含外部文件：先阻止默认拖放，再异步只读解析（ExternalContentRef，BR-SYS-DATA-007）。
    if ((data?.files?.length ?? 0) > 0) {
      e.preventDefault();
      e.stopPropagation();
      void fileReferenceFromTransfer(data).then((payload) => {
        if (payload) insertReferencePayload(payload, undefined, pos);
      });
      return;
    }
    const payload = payloadFromTransfer(data);
    if (!payload) return;
    e.preventDefault();
    e.stopPropagation();
    insertReferencePayload(payload, undefined, pos);
  };

  const visibleTokens = draft.tokens.slice(0, 5);
  const hiddenTokenCount = Math.max(0, draft.tokens.length - visibleTokens.length);
  // REF-REFERENCE-REPAIR-01 F2：纯引用（无正文文字）可发送（REQ-REF-006），canSend 与 send() guard 对齐。
  const canSend = (Boolean(draft.prompt.trim()) || draft.tokens.length > 0) && !sending;
  const sendCls = sending ? "is-busy" : canSend ? "is-ready" : "is-disabled";

  return (
    <div className="composer" onDragOverCapture={onDragOver} onDropCapture={onDrop}>
      {gateNotice && (
        <div className="composer-gate">
          存在待你决策的任务项：请先在上方裁决区完成决策（确认并继续 / 取消），完成后才能继续对话。
        </div>
      )}
      {draft.tokens.length > 0 && (
        <div className="composer-mentions" aria-label="草稿引用">
          {visibleTokens.map((token) => (
            <span key={token.refId} className={`mention-chip mention-chip-${token.kind}`} title={token.label}>
              {token.label}
              <button
                className="mention-chip-x"
                onClick={() => removeReferenceToken(token.refId)}
                aria-label="移除引用"
              >
                ×
              </button>
            </span>
          ))}
          {hiddenTokenCount > 0 && (
            <span className="mention-chip mention-chip-more" title={draft.tokens.slice(5).map((t) => t.label).join("\n")}>
              +{hiddenTokenCount} 引用
            </span>
          )}
        </div>
      )}

      {atQuery && atMatches.length > 0 && (
        <div className="mention-inline">
          {atMatches.map((d) => (
            <button key={d.relPath} className="mention-block-option" onClick={() => selectAtMention(d.relPath)}>
              @{d.relPath}
            </button>
          ))}
        </div>
      )}

      <div className="composer-box" onKeyDownCapture={onKeyDown} onPasteCapture={onPaste}>
        <div className={`composer-editor${editor?.isEmpty ? " is-empty" : ""}`}>
          <EditorContent editor={editor} />
        </div>
        <div className="composer-bar">
          <div className="composer-bar-left">
            <span className="cbar-btn" title="agent-edit-1.0 · 文档编辑模式">edit</span>

            <div className="cbar-wrap">
              <button
                className="cbar-btn"
                title="模型选择"
                aria-haspopup="menu"
                onClick={() => {
                  setModelPop((v) => !v);
                }}
              >
                {currentModel.label}{" "}
                <span className="label cbar-pop-src">{currentModel.source === "byok" ? "BYOK" : "预设"}</span>{" "}
                <span className="cbar-caret">▾</span>
              </button>
              {modelPop && (
                <div className="cbar-pop" role="menu">
                  {models.map((m) => {
                    const key = modelKey(m);
                    return (
                      <button
                        key={key}
                        className={`cbar-pop-item${selectedKey === key ? " selected" : ""}`}
                        onClick={() => {
                          modelStore.select(key);
                          setModelPop(false);
                        }}
                      >
                        {selectedKey === key ? "✓ " : ""}
                        {m.label}
                        <span className="label cbar-pop-src">{m.source === "byok" ? "BYOK" : "预设"}</span>
                        <span className="label cbar-pop-hint">{key}</span>
                      </button>
                    );
                  })}
                  <button
                    className="cbar-pop-item cbar-pop-add"
                    onClick={() => {
                      setModelPop(false);
                      setConfigOpen(true);
                    }}
                  >
                    ＋ 添加模型
                  </button>
                </div>
              )}
            </div>
          </div>

          <button
            className={`composer-send ${sendCls}`}
            onClick={sending ? () => void cancelActiveTurn() : send}
            disabled={!sending && !canSend}
            title={sending ? "停止当前 Agent 回合" : canSend ? "发送（Enter）" : "输入内容或引用后可发送"}
          >
            {sending ? "停止" : "发送"}
          </button>
        </div>
      </div>

      {configOpen && <ModelConfig onClose={() => setConfigOpen(false)} />}
    </div>
  );
}
