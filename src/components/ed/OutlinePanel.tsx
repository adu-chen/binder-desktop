import { useOutline } from "../../store/outlineStore";
import { useEdStore } from "../../store/edStore";
import { getActiveEditor } from "../../services/edLocate";

/**
 * 大纲面板（PI-ED-12 · N1 · 视图层）。活动文档标题树（outlineStore 投影），点击经 blockId
 * 精确跳转（PI-ED-10 定位底座：blockId → PM pos → setTextSelection + scrollIntoView）。
 * 旧项目 OutlinePanel 参考重写（适配 blockId 锚定 + store 订阅）。
 */

/** 按 blockId 跳转到块（光标置块首 + 滚入视区）；块不存在（已删除）静默不动（大纲随即会刷新）。 */
function jumpToBlock(blockId: string): void {
  const ed = getActiveEditor();
  if (!ed) return;
  let pos = -1;
  let p = 0;
  ed.state.doc.forEach((node) => {
    if (pos === -1 && node.attrs.blockId === blockId) pos = p;
    p += node.nodeSize;
  });
  if (pos === -1) return;
  ed.chain().focus().setTextSelection(pos + 1).scrollIntoView().run();
}

export function OutlinePanel() {
  const items = useOutline();
  const { openPath } = useEdStore();

  if (!openPath) {
    return <p className="muted outline-empty">打开文档后显示大纲</p>;
  }
  if (items.length === 0) {
    return <p className="muted outline-empty">当前文档没有标题</p>;
  }
  return (
    <nav className="outline" aria-label="文档大纲">
      {items.map((it) => (
        <button
          key={it.blockId}
          className={`outline-item outline-l${Math.min(it.level, 4)}`}
          title={it.text}
          onClick={() => jumpToBlock(it.blockId)}
        >
          {it.text || "（无标题文本）"}
        </button>
      ))}
    </nav>
  );
}
