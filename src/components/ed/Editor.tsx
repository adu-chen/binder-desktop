import { useEffect, useRef } from "react";
import { useEditor, EditorContent } from "@tiptap/react";
import { buildEditorExtensions } from "./extensions";
import { applyGreenReview } from "./extensions/DiffDecorationExtension";
import { edStore, useEdStore } from "../../store/edStore";
import { deStore, toGreenReview, useDeStore, type GreenReviewState } from "../../store/deStore";
import { assistStore } from "../../store/assistStore";
import { saveDocument, saveDocxDocument, reloadDocument, pollExternalChange } from "../../services/edService";
import { requestAssist } from "../../services/agService";
import { InlineAssistPopover } from "../ag/InlineAssistPopover";
import { AutocompleteOverlay } from "../ag/AutocompleteOverlay";

/**
 * ED 编辑器宿主（A-SYS-C-T-02 §3 编辑区 · PI-FE-05/07/08）。
 * TipTap 3 运行时 + blockId 扩展 + GreenReview 装饰层；编辑触发脏态（edStore，BR-ED-STATE-001 投影）。
 * GreenReview 与右侧 DiffCard 同源（deStore）→ 双向同步（X-INTER-001）。
 * ⌘K InlineAssist / ⌘J Autocomplete 辅助入口（PI-FE-08）：均经 ED 受控接口编辑，不产 DiffCard。
 */
const SEED_CONTENT = "开始写作…";

/** 是否按 Markdown 处理（.md/.markdown）；其余按纯文本忠实往返。 */
function isMarkdownPath(path: string | null): boolean {
  if (!path) return false;
  const ext = path.split(".").pop()?.toLowerCase();
  return ext === "md" || ext === "markdown";
}

/**
 * 跳转到目标行对应块（搜索命中跳行）：行号 → 顶层块索引（钳制），定位光标 + 滚动到视区。
 * 纯文本精确（行=段落）；md 近似（块索引≈行号，A 类债务）。
 */
function jumpToLine(editor: NonNullable<ReturnType<typeof useEditor>>, line: number) {
  const doc = editor.state.doc;
  const idx = Math.min(Math.max(line - 1, 0), Math.max(doc.childCount - 1, 0));
  let pos = 1; // doc 起始内偏移
  for (let j = 0; j < idx; j++) pos += doc.child(j).nodeSize;
  editor.chain().focus().setTextSelection(pos).scrollIntoView().run();
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

/** 序列化编辑器当前内容为会话格式：html=docx 编辑链 HTML；md=getMarkdown；其余=纯文本忠实往返。 */
function serialize(
  editor: NonNullable<ReturnType<typeof useEditor>>,
  fmt: "text" | "html",
  path: string | null,
): string {
  if (fmt === "html") return editor.getHTML();
  if (isMarkdownPath(path)) {
    const md = (editor.storage as unknown as Record<string, { getMarkdown: () => string }>).markdown;
    return md.getMarkdown();
  }
  return editor.getText({ blockSeparator: "\n" });
}

export function Editor() {
  const { openPath, tabState, loadSeq, loadedContent, editFormat } = useEdStore();
  const { diffs } = useDeStore(); // GreenReview 同步用（标签状态投影已移至 center TabStrip）
  const hostRef = useRef<HTMLDivElement>(null);
  // 当前辅助操作的目标编辑范围（⌘K=选区，⌘J=光标）。
  const assistRange = useRef<{ from: number; to: number }>({ from: 0, to: 0 });
  // 上一活动文档路径（切换标签前据此把当前编辑内容序列化回写其会话 → 不丢草稿）。
  const lastPathRef = useRef<string | null>(null);

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
        // ⌘J Autocomplete：光标处续写补全——经 ag_request_assist（门禁前置 + 网关，SM-AG-03）取候选。
        if (event.key === "j") {
          const coords = view.coordsAtPos(to);
          const rect = host.getBoundingClientRect();
          assistRange.current = { from: to, to };
          assistStore.openAutocomplete({ x: coords.left - rect.left, y: coords.bottom - rect.top }, []);
          assistStore.setGenerating(true);
          const prefix = view.state.doc.textBetween(Math.max(0, to - 400), to, " ");
          void requestAssist({ kind: "autocomplete", prefix }).then((candidate) => {
            // 仍停在本次自动补全（用户未切走）才填入；否则忽略（锚已变）。
            if (assistStore.getSnapshot().kind !== "autocomplete") return;
            if (candidate) assistStore.setSuggestions([candidate]);
            else assistStore.close(); // 门禁不可用/失败 → 静默不提供（不打断编辑）
          });
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

  // 载入活动会话：loadSeq 变更（loadDoc / 切换活动会话 setActivePath 投影）→ 先把上一文档的当前编辑序列化
  // 回写其会话（切标签不丢草稿），再 setContent 活动会话内容（不触发脏态）。
  // md/markdown 以 Markdown 解析为富文本（字符串经 tiptap-markdown）；其余按纯文本（doc 对象，绕过 md 解析）。
  useEffect(() => {
    if (!editor || loadSeq === 0) return;
    // 切换标签前：暂存上一活动文档的当前编辑内容到其会话（按其格式序列化）。
    const prev = lastPathRef.current;
    if (prev && prev !== openPath) {
      const prevSess = edStore.getSnapshot().sessions[prev];
      if (prevSess) edStore.updateContent(prev, serialize(editor, prevSess.editFormat, prev));
    }
    lastPathRef.current = openPath;
    if (!openPath || loadedContent === null) return;
    if (editFormat === "html") {
      // docx 编辑链：loadedContent 为 HTML（保留样式子集），TipTap 原生解析。
      editor.commands.setContent(loadedContent, { emitUpdate: false });
    } else if (isMarkdownPath(openPath)) {
      editor.commands.setContent(loadedContent, { emitUpdate: false });
    } else {
      editor.commands.setContent(textToDoc(loadedContent), { emitUpdate: false });
    }
    // 搜索命中跳行：内容设入后定位到目标行（块），随后清除请求。
    const { jumpLine } = edStore.getSnapshot();
    if (jumpLine !== null) {
      jumpToLine(editor, jumpLine);
      edStore.clearJump();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps -- 仅 loadSeq 驱动载入；openPath/loadedContent/editFormat 在 loadSeq 变更时同步刷新，单独入依赖会引发重复 setContent
  }, [editor, loadSeq]);

  // ⌘S 保存：docx 以 getHTML → doc.save_from_edit（编辑链回写）；md 以 getMarkdown、其余纯文本 → ws.save_doc。
  const saveCurrent = async (override: boolean) => {
    if (!editor) return;
    if (editFormat === "html") {
      // docx 编辑链回写：HTML → docx（--reference-doc）→ WS 原子落盘。
      await saveDocxDocument(serialize(editor, "html", openPath), override);
      return;
    }
    await saveDocument(serialize(editor, "text", openPath), override);
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
  /// 与 Chat Diff 链路（deStore）严格隔离。产出经 ag_request_assist（门禁前置 + 网关，SM-AG-03），
  /// 门禁不可用/失败时静默不提供（不打断编辑）。
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

  // ⌘K 生成：经 ag_request_assist（门禁前置→网关单次产出，SM-AG-03）；失败/门禁不可用静默关闭（不打断编辑）。
  const generateInline = (instruction: string) => {
    assistStore.setGenerating(true);
    const sel = assistStore.getSnapshot().selectionText;
    void requestAssist({ kind: "inline", instruction, selection: sel }).then((candidate) => {
      if (assistStore.getSnapshot().kind !== "inline") return; // 用户已切走，忽略
      if (candidate) assistStore.setInlinePreview(candidate);
      else assistStore.close();
    });
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
      {/* 标签栏由 center TabStrip 统一承载（PI-FE-26）；此处仅编辑纸张 + 冲突条 + 辅助浮层 */}
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
