import { useSyncExternalStore } from "react";
import { edStore } from "./edStore";

/**
 * center 标签栏状态（视图层导航态 · X-FORBID-007）。
 * tabs 同列承载文档(ED) + 预览(PRV)，activeTabId 决定 center 渲染（参考稿 §4 / A-SYS-C-T-02 §3 单窗多 tab）。
 * 标签是视图导航，非后端业务态——文档内容/脏态仍归 edStore（本 PI 单文档标签；多文档会话归 PI-B）。
 */

export type TabKind = "doc" | "preview";

export interface OpenTab {
  id: string;
  path: string;
  kind: TabKind;
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

export const tabsStore = {
  subscribe(cb: () => void) {
    listeners.add(cb);
    return () => listeners.delete(cb);
  },
  getSnapshot(): TabsState {
    return state;
  },

  /** 打开文档标签（本 PI 单文档：复用唯一 doc 标签、更新路径）；激活。 */
  openDoc(path: string) {
    const existing = state.tabs.find((t) => t.kind === "doc");
    if (existing) {
      const tabs = state.tabs.map((t) => (t.id === existing.id ? { ...t, path } : t));
      setState({ tabs, activeTabId: existing.id });
    } else {
      const tab: OpenTab = { id: genId(), path, kind: "doc" };
      setState({ tabs: [...state.tabs, tab], activeTabId: tab.id });
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

  activate(id: string) {
    if (state.activeTabId !== id) setState({ tabs: state.tabs, activeTabId: id });
  },

  /** 关闭标签；激活相邻。关闭文档标签 → 同步清空编辑器会话。 */
  close(id: string) {
    const idx = state.tabs.findIndex((t) => t.id === id);
    if (idx === -1) return;
    const closing = state.tabs[idx];
    const tabs = state.tabs.filter((t) => t.id !== id);
    const activeTabId = state.activeTabId === id ? neighborId(tabs, idx) : state.activeTabId;
    setState({ tabs, activeTabId });
    if (closing.kind === "doc") edStore.reset();
  },

  /** 切换/关闭 workspace 时清空全部标签。 */
  reset() {
    setState({ tabs: [], activeTabId: null });
  },
};

export function useTabsStore(): TabsState {
  return useSyncExternalStore(tabsStore.subscribe, tabsStore.getSnapshot);
}
