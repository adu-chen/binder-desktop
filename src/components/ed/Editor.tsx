import { useEffect } from "react";
import { useEditor, EditorContent } from "@tiptap/react";
import { buildEditorExtensions } from "./extensions";
import { EditorTab } from "./EditorTab";
import { edStore, useEdStore } from "../../store/edStore";

/**
 * ED 编辑器宿主（A-SYS-C-T-02 §3 编辑区 · PI-FE-05）。
 * TipTap 3 运行时 + blockId 扩展；编辑触发脏态（edStore，BR-ED-STATE-001 投影）。
 * 真实文档加载（ws.read_doc）与落盘（ws.save_doc）归后续 PI；本 PI 提供可编辑基础 md 表面。
 */

const SEED_CONTENT = "<p>开始写作…</p>";

export function Editor() {
  const { openPath, tabState, pendingDiffCount } = useEdStore();

  const editor = useEditor({
    extensions: buildEditorExtensions(),
    content: SEED_CONTENT,
    onUpdate: () => {
      edStore.markDirty();
    },
  });

  useEffect(() => {
    // 占位文档基线（实现期由 ws.read_doc 内容 + 路径替换）。
    edStore.openDoc("untitled.md");
  }, []);

  return (
    <div className="editor-host">
      <div className="editor-tabs">
        <EditorTab name={openPath ?? "untitled.md"} state={tabState} pendingDiffCount={pendingDiffCount} />
      </div>
      <div className="editor-paper">
        <EditorContent editor={editor} />
      </div>
    </div>
  );
}
