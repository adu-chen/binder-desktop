import { useEffect, useReducer } from "react";
import type { Editor } from "@tiptap/react";
import type { TabState } from "../../store/edStore";

const STATE_TEXT: Record<TabState, string> = {
  normal: "就绪",
  dirty: "● 未保存",
  saving: "保存中…",
  saved: "已保存",
  pending_diff: "待裁决修改",
};

export function EditorStatusBar({
  editor,
  tabState,
  converting,
  styleNotices = [],
}: {
  editor: Editor | null;
  tabState: TabState;
  converting: boolean;
  /** DOCX 转换报告的风险告知；无告知时不增加状态栏噪声。 */
  styleNotices?: string[];
}) {
  const [, tick] = useReducer((n: number) => n + 1, 0);

  useEffect(() => {
    if (!editor) return;
    const cb = () => tick();
    editor.on("transaction", cb);
    return () => {
      editor.off("transaction", cb);
    };
  }, [editor]);

  if (!editor) return null;

  const chars = (editor.storage as { characterCount?: { characters: () => number; words: () => number } })
    .characterCount;
  const charCount = chars?.characters() ?? editor.state.doc.textContent.length;
  const { from, to, head } = editor.state.selection;
  const selectedCount = from === to ? 0 : editor.state.doc.textBetween(from, to, "\n").replace(/\s/g, "").length;
  const fullText = editor.state.doc.textBetween(0, editor.state.doc.content.size, "\n");
  const lineCount = fullText === "" ? 1 : fullText.split("\n").length;
  const beforeCursor = editor.state.doc.textBetween(0, head, "\n");
  const cursorLine = (beforeCursor.match(/\n/g)?.length ?? 0) + 1;
  const cursorCol = beforeCursor.length - (beforeCursor.lastIndexOf("\n") + 1) + 1;

  return (
    <div className="ed-statusbar" role="status">
      <span className="ed-sb-item">
        字数：{selectedCount > 0 ? `${selectedCount}/${charCount}` : charCount}
      </span>
      <span className="ed-sb-item">行数：{lineCount}</span>
      <span className="ed-sb-item">{cursorLine}:{cursorCol}</span>
      <span className="ed-sb-spacer" aria-hidden="true" />
      {converting && <span className="ed-sb-item ed-sb-loading">正在转换文档…</span>}
      {styleNotices.length > 0 && (
        <span className="ed-sb-item" title={styleNotices.join("\n")} aria-label={`样式提示：${styleNotices.join("；")}`}>
          样式提示：{styleNotices.length}
        </span>
      )}
      <span className={`ed-sb-item ed-sb-state ed-sb-${tabState}`}>{STATE_TEXT[tabState]}</span>
    </div>
  );
}
