import { useEffect, useRef } from "react";
import { useEditor, EditorContent } from "@tiptap/react";
import { buildEditorExtensions } from "./extensions";
import { applyGreenReview } from "./extensions/DiffDecorationExtension";
import { EditorTab } from "./EditorTab";
import { edStore, useEdStore } from "../../store/edStore";
import { deStore, toGreenReview, useDeStore, type GreenReviewState } from "../../store/deStore";
import { assistStore } from "../../store/assistStore";
import { saveDocument, reloadDocument, pollExternalChange } from "../../services/edService";
import { InlineAssistPopover } from "../ag/InlineAssistPopover";
import { AutocompleteOverlay } from "../ag/AutocompleteOverlay";

/**
 * ED 编辑器宿主（A-SYS-C-T-02 §3 编辑区 · PI-FE-05/07/08）。
 * TipTap 3 运行时 + blockId 扩展 + GreenReview 装饰层；编辑触发脏态（edStore，BR-ED-STATE-001 投影）。
 * GreenReview 与右侧 DiffCard 同源（deStore）→ 双向同步（X-INTER-001）。
 * ⌘K InlineAssist / ⌘J Autocomplete 辅助入口（PI-FE-08）：均经 ED 受控接口编辑，不产 DiffCard。
 */
const SEED_CONTENT = "开始写作…";

const AUTOCOMPLETE_STUB = ["，山上有座庙。", "，云雾常年缭绕。", "，住着一位老者。"];

/** 是否按 Markdown 处理（.md/.markdown）；其余按纯文本忠实往返。 */
function isMarkdownPath(path: string | null): boolean {
  if (!path) return false;
  const ext = path.split(".").pop()?.toLowerCase();
  return ext === "md" || ext === "markdown";
}

/** 纯文本 → ProseMirror doc（每行一段落，忠实往返；富文本 md 解析序列化为债务）。 */
function textToDoc(text: string) {
  const lines = text.split("\n");
  return {
    type: "doc",
    content: lines.map((line) =>
      line ? { type: "paragraph", content: [{ type: "text", text: line }] } : { type: "paragraph" },
    ),
  };
}

export function Editor() {
  const { openPath, tabState, pendingDiffCount, loadSeq, loadedContent } = useEdStore();
  const { diffs } = useDeStore();
  const hostRef = useRef<HTMLDivElement>(null);
  // 当前辅助操作的目标编辑范围（⌘K=选区，⌘J=光标）。
  const assistRange = useRef<{ from: number; to: number }>({ from: 0, to: 0 });

  const editor = useEditor({
    extensions: buildEditorExtensions(),
    content: SEED_CONTENT,
    onUpdate: () => {
      edStore.markDirty();
    },
    editorProps: {
      handleKeyDown: (view, event) => {
        const mod = event.metaKey || event.ctrlKey;
        if (!mod) return false;
        const host = hostRef.current;
        if (!host) return false;
        const { from, to, empty } = view.state.selection;

        // ⌘S 保存：ws.save_doc（带加载基线，SM-WS-01）。
        if (event.key === "s") {
          event.preventDefault();
          void saveCurrent(false);
          return true;
        }
        // ⌘K InlineAssist：有选区时开内联辅助（替换选区）。
        if (event.key === "k") {
          if (empty) return false;
          const coords = view.coordsAtPos(from);
          const rect = host.getBoundingClientRect();
          assistRange.current = { from, to };
          assistStore.openInline(
            { x: coords.left - rect.left, y: coords.top - rect.top },
            view.state.doc.textBetween(from, to, " "),
            from,
            to,
          );
          return true;
        }
        // ⌘J Autocomplete：光标处续写补全。
        if (event.key === "j") {
          const coords = view.coordsAtPos(to);
          const rect = host.getBoundingClientRect();
          assistRange.current = { from: to, to };
          assistStore.openAutocomplete(
            { x: coords.left - rect.left, y: coords.bottom - rect.top },
            AUTOCOMPLETE_STUB,
          );
          return true;
        }
        return false;
      },
    },
  });

  // 外部变更监视：打开真实文档时每 5s 轮询磁盘 vs 加载基线（external_change 逻辑契约，物理事件延 SY-02）。
  useEffect(() => {
    if (!openPath) return;
    const id = window.setInterval(() => void pollExternalChange(), 5000);
    return () => window.clearInterval(id);
  }, [openPath]);

  // 载入真实文档：loadSeq 变更（ws.read_doc 投影）→ setContent（不触发脏态）。
  // md/markdown 以 Markdown 解析为富文本（字符串经 tiptap-markdown）；其余按纯文本（doc 对象，绕过 md 解析）。
  useEffect(() => {
    if (!editor || loadSeq === 0 || loadedContent === null) return;
    if (isMarkdownPath(openPath)) {
      editor.commands.setContent(loadedContent, { emitUpdate: false });
    } else {
      editor.commands.setContent(textToDoc(loadedContent), { emitUpdate: false });
    }
  }, [editor, loadSeq, loadedContent, openPath]);

  // ⌘S 保存：md 以 getMarkdown 序列化、其余取纯文本（行→\n）→ ws.save_doc（带加载基线）。
  const saveCurrent = async (override: boolean) => {
    if (!editor) return;
    const mdStorage = (editor.storage as unknown as Record<string, { getMarkdown: () => string }>).markdown;
    const content = isMarkdownPath(openPath)
      ? mdStorage.getMarkdown()
      : editor.getText({ blockSeparator: "\n" });
    await saveDocument(content, override);
  };

  // GreenReview 同步：deStore diff 态变更 → 推送 blockId→GreenReviewState 映射到装饰层。
  useEffect(() => {
    if (!editor) return;
    const map = new Map<string, GreenReviewState>();
    for (const d of deStore.getSnapshot().diffs) {
      const gr = toGreenReview(d.state);
      if (gr) map.set(d.blockId, gr);
    }
    applyGreenReview(editor.view, map);
  }, [editor, diffs]);

  /**
   * @GOV
   * codes:    BR-AG-STATE-003-RB-AG-AG-ASSIST-002
   * type:     RB
   * chain:    AG-ASSIST
   * rules:    BR-AG-STATE-003
   * term_ref: TERM-AG-003, TERM-AG-004
   * boundary: in=assistApplyIntent | out=editorMutation(inlineReplacesSelectionAutocompleteInsertsNoDiffCard)
   */
  /// AG-ASSIST 落地守则（BR-AG-STATE-003）：⌘K InlineAssist Apply 直接替换选区、⌘J Autocomplete 插入
  /// 光标处，二者均经 ED 受控编辑接口（X-FORBID-005 不绕过），**不产 DiffCard、不进入 SM-DE-01**——
  /// 与 Chat Diff 链路（deStore）严格隔离。模型输出桩化，门禁拒绝时辅助静默不提供（SM-AG-03）。
  const applyInline = (text: string) => {
    if (!editor) return;
    const { from, to } = assistRange.current;
    editor.chain().focus().insertContentAt({ from, to }, text).run();
    assistStore.close();
  };
  const pickAutocomplete = (text: string) => {
    if (!editor) return;
    const { from } = assistRange.current;
    editor.chain().focus().insertContentAt(from, text).run();
    assistStore.close();
  };

  // ⌘K 生成（桩）：实现期经门禁→网关产出；此处本地桩化预览。
  const generateInline = (instruction: string) => {
    assistStore.setGenerating(true);
    const sel = assistStore.getSnapshot().selectionText;
    setTimeout(() => {
      assistStore.setInlinePreview(`（桩）按「${instruction}」改写：${sel}`);
    }, 220);
  };

  // 未打开真实文档 → 诚实空态（不显示可编辑的假 untitled.md）。
  if (!openPath) {
    return (
      <div className="editor-host editor-empty">
        <p className="muted">从文件树打开一个文档开始编辑</p>
      </div>
    );
  }

  return (
    <div className="editor-host" ref={hostRef}>
      <div className="editor-tabs">
        <EditorTab name={openPath} state={tabState} pendingDiffCount={pendingDiffCount} />
      </div>
      {tabState === "external_changed" && (
        <div
          role="alert"
          style={{
            display: "flex",
            alignItems: "center",
            gap: "var(--sp-3)",
            padding: "var(--sp-2) var(--sp-4)",
            background: "var(--gold-bg)",
            borderBottom: "2px solid var(--gold)",
            fontSize: 13,
            color: "var(--ink-2)",
          }}
        >
          <span>文件被外部修改或保存冲突。</span>
          <button style={{ padding: "var(--sp-1) var(--sp-3)", fontSize: 12 }} onClick={() => void saveCurrent(true)}>
            保留我的编辑（覆盖）
          </button>
          <button className="ghost" style={{ padding: "var(--sp-1) var(--sp-3)", fontSize: 12 }} onClick={() => void reloadDocument()}>
            放弃并重载
          </button>
        </div>
      )}
      <div className="editor-paper">
        <EditorContent editor={editor} />
      </div>
      <InlineAssistPopover onGenerate={generateInline} onApply={applyInline} />
      <AutocompleteOverlay onPick={pickAutocomplete} />
    </div>
  );
}
