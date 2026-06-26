import { tabsStore, useTabsStore, type OpenTab } from "../../store/tabsStore";
import { useEdStore } from "../../store/edStore";
import { useDeStore } from "../../store/deStore";

/**
 * center 标签栏（A-SYS-C-T-02 §3 单窗多 tab · 参考稿 §4 · PI-FE-26）。
 * 文档(ED) + 预览(PRV) 同列共存；点切换、× 关闭。文档标签投影真实脏态/pending/external（X-FORBID-007）。
 */

const base = (p: string) => p.split("/").pop() ?? p;

export function TabStrip() {
  const { tabs, activeTabId } = useTabsStore();
  const { sessions } = useEdStore();
  const { diffs } = useDeStore();
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

  return (
    <div className="tabstrip" role="tablist">
      {tabs.map((tab) => (
        <div
          key={tab.id}
          className={`tab${tab.id === activeTabId ? " active" : ""}`}
          role="tab"
          aria-selected={tab.id === activeTabId}
          onClick={() => tabsStore.activate(tab.id)}
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
    </div>
  );
}
