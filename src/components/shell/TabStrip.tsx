import { useEffect, useRef, useState } from "react";
import { tabsStore, useTabsStore, type OpenTab } from "../../store/tabsStore";
import { useEdStore } from "../../store/edStore";
import { useDeStore } from "../../store/deStore";

/**
 * center 标签栏（A-SYS-C-T-02 §3 单窗多 tab · 参考稿 §4 · PI-FE-26 · PI-ED-07）。
 * 文档(ED) + 预览(PRV) 同列共存；点切换、× 关闭。文档标签投影真实脏态/pending/external（X-FORBID-007）。
 * PI-ED-07：#5 指针拖动排序（tabsStore.reorder）；#12 滚动条隐藏 + 纵轮转横滚 + 激活标签自动滚入视区。
 */

const base = (p: string) => p.split("/").pop() ?? p;

export function TabStrip() {
  const { tabs, activeTabId } = useTabsStore();
  const { sessions } = useEdStore();
  const { diffs } = useDeStore();
  const stripRef = useRef<HTMLDivElement>(null);
  // 拖动排序态（视图瞬态）：拖动中的标签 id + 当前悬停目标（其前插入；null=末尾）。
  const [dragId, setDragId] = useState<string | null>(null);
  const [dropBefore, setDropBefore] = useState<string | null | undefined>(undefined);
  const pointerDown = useRef<{ id: string; x: number } | null>(null);

  // #12 纵向滚轮 → 横向滚动（passive:false 以 preventDefault 阻止页面滚动）。
  useEffect(() => {
    const el = stripRef.current;
    if (!el) return;
    const onWheel = (e: WheelEvent) => {
      if (Math.abs(e.deltaY) <= Math.abs(e.deltaX)) return; // 已是横向手势则原生处理
      e.preventDefault();
      el.scrollLeft += e.deltaY;
    };
    el.addEventListener("wheel", onWheel, { passive: false });
    return () => el.removeEventListener("wheel", onWheel);
  }, []);

  // #12 激活标签滚入视区。
  useEffect(() => {
    if (!activeTabId) return;
    stripRef.current
      ?.querySelector(`[data-tab-id="${CSS.escape(activeTabId)}"]`)
      ?.scrollIntoView({ block: "nearest", inline: "nearest" });
  }, [activeTabId]);

  if (tabs.length === 0) return null;

  // 各文档标签按其会话独立投影有效态（互斥：external > pending_diff > 保存态），投影 edStore/deStore。
  const marker = (tab: OpenTab) => {
    if (tab.kind !== "doc") return null;
    const sess = sessions[tab.path];
    if (!sess) return null;
    const pending = diffs.filter((d) => d.state === "pending" && d.relPath === tab.path).length;
    const st =
      sess.tabState === "external_changed" ? "external_changed" : pending > 0 ? "pending_diff" : sess.tabState;
    if (st === "dirty") return <span className="tab-dot" aria-label="未保存" />;
    if (st === "saving") return <span className="tab-spinner" aria-label="保存中" />;
    if (st === "external_changed") return <span className="tab-ext" title="外部修改">!</span>;
    if (st === "pending_diff" && pending > 0) return <span className="tab-badge">{pending}</span>;
    return null;
  };

  // #5 指针拖动排序：按下记录起点，横移超阈值进入拖动；悬停某标签中线前/后决定插入点；抬起提交 reorder。
  const DRAG_THRESHOLD = 6;
  const onPointerDown = (e: React.PointerEvent, id: string) => {
    if (e.button !== 0) return;
    pointerDown.current = { id, x: e.clientX };
  };
  const onPointerMove = (e: React.PointerEvent) => {
    const pd = pointerDown.current;
    if (!pd) return;
    if (dragId === null && Math.abs(e.clientX - pd.x) < DRAG_THRESHOLD) return;
    if (dragId === null) {
      setDragId(pd.id);
      (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    }
    // 命中测试：找到指针下的标签，按其中线决定插入其前或其后（后=下一标签前/末尾）。
    const strip = stripRef.current;
    if (!strip) return;
    const els = Array.from(strip.querySelectorAll<HTMLElement>("[data-tab-id]"));
    let target: string | null = null; // null = 末尾
    for (const el of els) {
      const r = el.getBoundingClientRect();
      if (e.clientX < r.left + r.width / 2) {
        target = el.dataset.tabId ?? null;
        break;
      }
    }
    setDropBefore(target);
  };
  const endDrag = () => {
    if (dragId !== null && dropBefore !== undefined && dropBefore !== dragId) {
      tabsStore.reorder(dragId, dropBefore);
    }
    pointerDown.current = null;
    setDragId(null);
    setDropBefore(undefined);
  };

  return (
    <div className="tabstrip" role="tablist" ref={stripRef}>
      {tabs.map((tab) => (
        <div
          key={tab.id}
          data-tab-id={tab.id}
          className={`tab${tab.id === activeTabId ? " active" : ""}${tab.id === dragId ? " dragging" : ""}${
            dropBefore === tab.id && dragId !== null ? " drop-before" : ""
          }`}
          role="tab"
          aria-selected={tab.id === activeTabId}
          onClick={() => dragId === null && tabsStore.activate(tab.id)}
          onPointerDown={(e) => onPointerDown(e, tab.id)}
          onPointerMove={onPointerMove}
          onPointerUp={endDrag}
          onPointerCancel={endDrag}
          title={tab.path}
        >
          <span className="tab-glyph">{tab.kind === "preview" ? "◳" : "M↓"}</span>
          <span className="tab-name">{base(tab.path)}</span>
          {marker(tab)}
          <button
            className="tab-close"
            title="关闭"
            onClick={(e) => {
              e.stopPropagation();
              tabsStore.close(tab.id);
            }}
          >
            ×
          </button>
        </div>
      ))}
      {/* 末尾落点指示（拖到最右） */}
      {dragId !== null && dropBefore === null && <span className="tab-drop-end" aria-hidden="true" />}
    </div>
  );
}
