import { useEffect, useReducer, useState } from "react";
import type { Editor } from "@tiptap/react";
import type { TabState } from "../../store/edStore";

/**
 * 编辑器底部状态栏（PI-ED-07 #4 · 视图层）。
 * 左：字数（中文按字符计，CharacterCount 扩展）/ 行数（顶层块）/ 光标位（块:offset）。
 * 右：保存态（脏/保存中/已保存）+ 外部修改项（点击弹出显式裁决——BR-ED-STATE-003 语义不弱化：
 * 「保留我的编辑（覆盖）」与「放弃并重载」双向内容损失均须用户显式选择，仅呈现位置从横幅移入状态栏）。
 */

const STATE_TEXT: Record<TabState, string> = {
  normal: "就绪",
  dirty: "● 未保存",
  saving: "保存中…",
  saved: "已保存",
  pending_diff: "待裁决修改",
  external_changed: "⚠ 外部修改",
};

export function EditorStatusBar({
  editor,
  tabState,
  converting,
  onKeepMine,
  onReload,
}: {
  editor: Editor | null;
  tabState: TabState;
  /** docx 打开转换中（SM-DOC-01 OpenConverting 投影）。 */
  converting: boolean;
  onKeepMine: () => void;
  onReload: () => void;
}) {
  const [, tick] = useReducer((n: number) => n + 1, 0);
  const [conflictOpen, setConflictOpen] = useState(false);

  // 订阅编辑器事务 → 刷新字数/光标（选区/内容变化即时反映）。
  useEffect(() => {
    if (!editor) return;
    const cb = () => tick();
    editor.on("transaction", cb);
    return () => {
      editor.off("transaction", cb);
    };
  }, [editor]);

  // 冲突解除后自动收起弹出。
  useEffect(() => {
    if (tabState !== "external_changed") setConflictOpen(false);
  }, [tabState]);

  if (!editor) return null;

  // 展示项对齐旧项目 EditorStatusBar（2026-07-03 #5 修正）：字数（选中时 选中/总）/ 行数 / 光标 行:列
  // ——用户无"块"概念，行列基于文本行（blockSeparator="\n"，纯文本精确、富文本=段落行直觉一致）。
  const chars = (editor.storage as { characterCount?: { characters: () => number; words: () => number } })
    .characterCount;
  const charCount = chars?.characters() ?? editor.state.doc.textContent.length;
  const { from, to, head } = editor.state.selection;
  const selectedCount = from === to ? 0 : editor.state.doc.textBetween(from, to, "\n").replace(/\s/g, "").length;
  const fullText = editor.state.doc.textBetween(0, editor.state.doc.content.size, "\n");
  const lineCount = fullText === "" ? 1 : fullText.split("\n").length;
  // 光标 行:列（光标前文本的换行数 + 1 / 末行长度 + 1）。
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
      {tabState === "external_changed" ? (
        <span className="ed-sb-conflict-wrap">
          <button className="ed-sb-item ed-sb-conflict" onClick={() => setConflictOpen((v) => !v)}>
            {STATE_TEXT.external_changed}
          </button>
          {conflictOpen && (
            <div className="ed-sb-conflict-pop" role="alertdialog" aria-label="外部修改裁决">
              <p>文件被外部修改或保存冲突。</p>
              <div className="ed-sb-conflict-actions">
                <button onClick={onKeepMine}>保留我的编辑（覆盖）</button>
                <button className="ghost" onClick={onReload}>放弃并重载</button>
              </div>
            </div>
          )}
        </span>
      ) : (
        <span className={`ed-sb-item ed-sb-state ed-sb-${tabState}`}>{STATE_TEXT[tabState]}</span>
      )}
    </div>
  );
}
