import { useEffect } from "react";
import { useEditor, EditorContent } from "@tiptap/react";
import { buildEditorExtensions } from "./extensions";
import { applyGreenReview } from "./extensions/DiffDecorationExtension";
import { EditorTab } from "./EditorTab";
import { edStore, useEdStore } from "../../store/edStore";
import { deStore, toGreenReview, useDeStore, type GreenReviewState } from "../../store/deStore";

/**
 * ED 编辑器宿主（A-SYS-C-T-02 §3 编辑区 · PI-FE-05/07）。
 * TipTap 3 运行时 + blockId 扩展 + GreenReview 装饰层；编辑触发脏态（edStore，BR-ED-STATE-001 投影）。
 * GreenReview 与右侧 DiffCard 同源（deStore）→ 双向同步（X-INTER-001）。
 * 真实文档加载（ws.read_doc）与落盘（ws.save_doc）归后续 PI；本组件提供可编辑基础 md 表面。
 *
 * 占位 seed：显式 data-block-id 与 deStore 演示 diff 锚对齐，使 GreenReview 装饰可见
 * （实现期由 ws.read_doc 内容 + de.generate_diff 真实 blockId 替换）。
 */
const SEED_CONTENT = `
<p data-block-id="demo-b1">从前有座山。</p>
<p data-block-id="demo-b2">数据支持这一点。</p>
<p data-block-id="demo-b3">这个很重要。</p>
<p data-block-id="demo-b4">其实基本上大概可能。</p>
`;

export function Editor() {
  const { openPath, tabState, pendingDiffCount } = useEdStore();
  const { diffs } = useDeStore();

  const editor = useEditor({
    extensions: buildEditorExtensions(),
    content: SEED_CONTENT,
    onUpdate: () => {
      edStore.markDirty();
    },
  });

  useEffect(() => {
    edStore.openDoc("untitled.md");
  }, []);

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
