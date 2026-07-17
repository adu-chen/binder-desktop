import { useState, type ClipboardEvent, type KeyboardEvent } from "react";
import { Node, mergeAttributes, type JSONContent } from "@tiptap/core";
import { EditorContent, useEditor, type Editor as TipTapEditor } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import { activeSession, agStore, useAgStore, type AgMode } from "../../store/agStore";
import { modelKey, modelStore, useModelStore } from "../../store/modelStore";
import { startTurn } from "../../services/agService";
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
  readReferencePayloadData,
  referenceFromPlainTextPaste,
  referenceKey,
  type MessageContentPart,
  type ReferencePayload,
  type ReferenceTokenAttrs,
} from "../../services/referencePayload";
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

const MODES: { id: AgMode; label: string; hint: string }[] = [
  { id: "edit", label: "edit", hint: "编辑 · 精确修改当前内容（diff 裁决）" },
  { id: "build", label: "build", hint: "构建 · 项目蓝图与整体构建" },
];
const REFERENCE_DRAG_TYPES = [REFERENCE_MIME, SELECTION_REF_MIME, DOC_REF_MIME, "text/uri-list", "text/plain"] as const;

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

  return { parts, references, tokens, prompt: promptTextFromParts(parts) };
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

function payloadFromTransfer(dataTransfer: DataTransfer): ReferencePayload | null {
  const generic = readReferencePayloadData(dataTransfer);
  if (generic) return generic;
  const selection = readSelectionRefData(dataTransfer);
  if (selection) return makeSelectionReference(selection);
  const doc = readDocRefData(dataTransfer);
  if (doc) return makeDocReference(doc.relPath);
  const uri = dataTransfer.getData("text/uri-list");
  const plain = dataTransfer.getData("text/plain");
  return referenceFromPlainTextPaste(uri || plain);
}

interface DraftState {
  tokens: ReferenceTokenAttrs[];
  prompt: string;
}

/**
 * Composer（A-SYS-C-T-02 §4.6 · PI-AG-REF-TOKEN-01）。
 * 输入区使用 TipTap inline atom 承载 ReferenceToken：拖拽/粘贴/@mention 都进入正文流；草稿 chip/tray
 * 只是引用摘要，发送后清空，已发送消息用同一 MessageContentPart 渲染。
 */
export function Composer() {
  const agState = useAgStore();
  const { sending } = agState;
  const mode = activeSession(agState).mode;
  const { models, selectedKey } = useModelStore();
  const currentModel = modelStore.current();
  const { docs } = useWsStore();

  const [draft, setDraft] = useState<DraftState>({ tokens: [], prompt: "" });
  const [atQuery, setAtQuery] = useState<MentionQuery | null>(null);
  const [modePop, setModePop] = useState(false);
  const [modelPop, setModelPop] = useState(false);
  const [configOpen, setConfigOpen] = useState(false);

  const refreshDraft = (editor: TipTapEditor) => {
    const next = serializeEditor(editor);
    setDraft({ tokens: next.tokens, prompt: next.prompt });
    setAtQuery(activeMentionQuery(editor));
  };

  const editor = useEditor({
    extensions: [StarterKit, ReferenceTokenNode],
    content: { type: "doc", content: [{ type: "paragraph" }] },
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

  const fileDocs = docs.filter((d) => !d.isDir);
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
    const snapshot = serializeEditor(editor);
    if (!snapshot.prompt.trim() && snapshot.references.length === 0) return;
    void startTurn(snapshot.prompt, {
      references: snapshot.references,
      displayParts: snapshot.parts,
      provider: currentModel.provider,
      model: currentModel.model,
    });
    editor.commands.clearContent(true);
    setDraft({ tokens: [], prompt: "" });
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
    const payload = payloadFromTransfer(e.clipboardData);
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
    const payload = payloadFromTransfer(e.dataTransfer);
    if (!payload) return;
    e.preventDefault();
    e.stopPropagation();
    let pos: number | undefined;
    if (typeof document.elementFromPoint === "function") {
      try {
        pos = editor.view.posAtCoords({ left: e.clientX, top: e.clientY })?.pos;
      } catch {
        pos = undefined;
      }
    }
    insertReferencePayload(payload, undefined, pos);
  };

  const visibleTokens = draft.tokens.slice(0, 5);
  const hiddenTokenCount = Math.max(0, draft.tokens.length - visibleTokens.length);
  const canSend = Boolean(draft.prompt.trim()) && !sending;
  const sendCls = sending ? "is-busy" : canSend ? "is-ready" : "is-disabled";

  return (
    <div className="composer" onDragOverCapture={onDragOver} onDropCapture={onDrop}>
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
            <div className="cbar-wrap">
              <button
                className="cbar-btn"
                title="模式：edit 编辑 / build 构建"
                aria-haspopup="menu"
                onClick={() => {
                  setModePop((v) => !v);
                  setModelPop(false);
                }}
              >
                {mode} <span className="cbar-caret">▾</span>
              </button>
              {modePop && (
                <div className="cbar-pop" role="menu">
                  {MODES.map((m) => (
                    <button
                      key={m.id}
                      className={`cbar-pop-item${mode === m.id ? " selected" : ""}`}
                      onClick={() => {
                        agStore.setMode(m.id);
                        setModePop(false);
                      }}
                    >
                      {mode === m.id ? "✓ " : ""}
                      {m.label}
                      <span className="label cbar-pop-hint">{m.hint}</span>
                    </button>
                  ))}
                </div>
              )}
            </div>

            <div className="cbar-wrap">
              <button
                className="cbar-btn"
                title="模型选择"
                aria-haspopup="menu"
                onClick={() => {
                  setModelPop((v) => !v);
                  setModePop(false);
                }}
              >
                {currentModel.label} <span className="cbar-caret">▾</span>
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
            onClick={send}
            disabled={!canSend}
            title={sending ? "AI 回复中…" : canSend ? "发送（Enter）" : "输入内容或引用后可发送"}
          >
            {sending ? "生成中" : "发送"}
          </button>
        </div>
      </div>

      {configOpen && <ModelConfig onClose={() => setConfigOpen(false)} />}
    </div>
  );
}
