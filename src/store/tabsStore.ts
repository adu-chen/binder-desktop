import { useSyncExternalStore } from "react";
import { edStore, isDirtyTabState } from "./edStore";

/**
 * center 标签栏状态（视图层导航态 · X-FORBID-007）。
 * tabs 同列承载文档(ED) + 预览(PRV)，activeTabId 决定 center 渲染（参考稿 §4 / A-SYS-C-T-02 §3 单窗多 tab）。
 * 标签是视图导航，非后端业务态——文档内容/脏态仍归 edStore。
 * 多文档标签（PI-FE-27 / PI-B）：每个文档一标签，激活文档标签 → 切 edStore 活动会话（各自保留未保存编辑）。
 */

export type TabKind = "doc" | "preview";

export interface OpenTab {
  id: string;
  path: string;
  kind: TabKind;
}

export interface PersistedOpenDocTab {
  relPath: string;
  kind: "doc";
  openIndex: number;
  isActive: boolean;
  updatedAt?: number;
}

interface TabsState {
  tabs: OpenTab[];
  activeTabId: string | null;
}

let state: TabsState = { tabs: [], activeTabId: null };
const listeners = new Set<() => void>();
function emit() {
  for (const l of listeners) l();
}
function setState(next: TabsState) {
  state = next;
  emit();
}

let seq = 0;
const genId = () => `tab-${Date.now()}-${(seq += 1)}`;

/** 关闭后选取相邻激活标签（优先后一个，否则前一个，否则 null）。 */
function neighborId(tabs: OpenTab[], removedIdx: number): string | null {
  if (tabs.length === 0) return null;
  return (tabs[removedIdx] ?? tabs[removedIdx - 1] ?? tabs[0]).id;
}

function activeDocPath(): string | null {
  const active = state.tabs.find((t) => t.id === state.activeTabId);
  if (active?.kind === "doc") return active.path;
  return edStore.getSnapshot().openPath;
}

export const tabsStore = {
  subscribe(cb: () => void) {
    listeners.add(cb);
    return () => listeners.delete(cb);
  },
  getSnapshot(): TabsState {
    return state;
  },

  /** 打开文档标签（多文档共存，按 path 去重）；激活并切 edStore 活动会话（复用已载入会话不丢草稿）。
   *  新文档标签的内容载入由调用方（edService.openDocument→loadDoc，会自设活动会话）负责。 */
  openDoc(path: string) {
    const existing = state.tabs.find((t) => t.kind === "doc" && t.path === path);
    if (existing) {
      setState({ tabs: state.tabs, activeTabId: existing.id });
      edStore.setActivePath(path); // 复用已载入会话 → 切活动会话（不重新读盘，保留未保存编辑）
    } else {
      const tab: OpenTab = { id: genId(), path, kind: "doc" };
      setState({ tabs: [...state.tabs, tab], activeTabId: tab.id });
      if (edStore.getSnapshot().sessions[path]) {
        edStore.setActivePath(path);
      }
    }
  },

  /** 打开预览标签（多预览共存，按 path 去重）；激活。 */
  openPreview(path: string) {
    const existing = state.tabs.find((t) => t.kind === "preview" && t.path === path);
    if (existing) {
      setState({ tabs: state.tabs, activeTabId: existing.id });
    } else {
      const tab: OpenTab = { id: genId(), path, kind: "preview" };
      setState({ tabs: [...state.tabs, tab], activeTabId: tab.id });
    }
  },

  /** 激活标签；切到文档标签 → 同步 edStore 活动会话（恢复该文档未保存编辑）。 */
  activate(id: string) {
    if (state.activeTabId === id) return;
    setState({ tabs: state.tabs, activeTabId: id });
    const tab = state.tabs.find((t) => t.id === id);
    if (tab?.kind === "doc") edStore.setActivePath(tab.path);
  },

  /** 关闭标签；激活相邻。关闭文档标签 → 销毁其编辑会话；若相邻激活的是文档标签则切其活动会话。 */
  close(id: string, options: { force?: boolean } = {}): boolean {
    const idx = state.tabs.findIndex((t) => t.id === id);
    if (idx === -1) return false;
    const closing = state.tabs[idx];
    const session = closing.kind === "doc" ? edStore.getSnapshot().sessions[closing.path] : undefined;
    if (!options.force && session && isDirtyTabState(session.tabState)) {
      const accepted = globalThis.confirm(`“${closing.path}”有未保存修改，关闭将丢失这些修改。仍要关闭吗？`);
      if (!accepted) return false;
    }
    const wasActive = state.activeTabId === id;
    const tabs = state.tabs.filter((t) => t.id !== id);
    const activeTabId = wasActive ? neighborId(tabs, idx) : state.activeTabId;
    setState({ tabs, activeTabId });
    if (closing.kind === "doc") {
      if (options.force || (session && isDirtyTabState(session.tabState))) {
        edStore.discardSession(closing.path);
      } else {
        edStore.closeSession(closing.path);
      }
    }
    if (wasActive && activeTabId) {
      const next = tabs.find((t) => t.id === activeTabId);
      if (next?.kind === "doc") edStore.setActivePath(next.path);
    }
    return true;
  },

  /** 拖动排序（PI-ED-07 #5）：把 fromId 标签移到 toId 标签之前（toId=null → 移到末尾）。
   *  纯数组重排，不改激活态/会话。无效 id / 同位 → 空操作。 */
  reorder(fromId: string, toId: string | null) {
    const fromIdx = state.tabs.findIndex((t) => t.id === fromId);
    if (fromIdx === -1 || fromId === toId) return;
    const tabs = state.tabs.filter((t) => t.id !== fromId);
    const insertIdx = toId === null ? tabs.length : tabs.findIndex((t) => t.id === toId);
    if (insertIdx === -1) return;
    tabs.splice(insertIdx, 0, state.tabs[fromIdx]);
    setState({ tabs, activeTabId: state.activeTabId });
  },

  getPersistedOpenDocTabs(): PersistedOpenDocTab[] {
    const activePath = activeDocPath();
    return state.tabs.flatMap((tab, index) => {
      if (tab.kind !== "doc") return [];
      return [{
        relPath: tab.path,
        kind: "doc",
        openIndex: index,
        isActive: tab.path === activePath,
      } satisfies PersistedOpenDocTab];
    });
  },

  restoreDocTabs(openTabs: PersistedOpenDocTab[]) {
    const sorted = [...openTabs]
      .filter((tab) => tab.kind === "doc")
      .sort((a, b) => a.openIndex - b.openIndex);
    const tabs = sorted.map((tab) => ({ id: genId(), path: tab.relPath, kind: "doc" as const }));
    const activeIdx = Math.max(0, sorted.findIndex((tab) => tab.isActive));
    const activeTabId = tabs[activeIdx]?.id ?? null;
    setState({ tabs, activeTabId });
    const active = tabs[activeIdx];
    if (active) edStore.setActivePath(active.path);
  },

  /** 切换/关闭 workspace 时清空全部标签。 */
  reset() {
    setState({ tabs: [], activeTabId: null });
  },
};

export function useTabsStore(): TabsState {
  return useSyncExternalStore(tabsStore.subscribe, tabsStore.getSnapshot);
}
