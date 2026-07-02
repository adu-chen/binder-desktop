import { useSyncExternalStore } from "react";
import type { Node as PMNode } from "@tiptap/pm/model";

/**
 * 大纲投影 store（PI-ED-12 · 视图层）。活动文档的标题树投影：Editor 在事务后提取（相等则不发布），
 * OutlinePanel 订阅呈现、点击经 blockId 跳转（PI-ED-10 定位底座）。
 * 纯视图投影（来源=编辑器内存模型），非 X-FORBID-007 限制的权威业务态。
 */

export interface OutlineItem {
  blockId: string;
  /** 标题级别 1-6；0 = 非标题（不出现在大纲）。 */
  level: number;
  text: string;
}

/** 大纲提取纯逻辑（供 Editor 与单测共用）：顶层 heading 块 → {blockId, level, text}；
 *  无 blockId 的标题跳过（无锚不可跳转，不产生死项）。 */
export function extractOutline(doc: PMNode): OutlineItem[] {
  const items: OutlineItem[] = [];
  doc.forEach((node) => {
    if (node.type.name !== "heading") return;
    const blockId = (node.attrs.blockId as string | null) ?? "";
    if (!blockId) return;
    items.push({ blockId, level: Number(node.attrs.level) || 1, text: node.textContent });
  });
  return items;
}

/** 两个大纲相等（避免每次事务都发布引发重渲染）。 */
export function sameOutline(a: OutlineItem[], b: OutlineItem[]): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) {
    if (a[i].blockId !== b[i].blockId || a[i].level !== b[i].level || a[i].text !== b[i].text) return false;
  }
  return true;
}

let items: OutlineItem[] = [];
const listeners = new Set<() => void>();

export const outlineStore = {
  subscribe(cb: () => void) {
    listeners.add(cb);
    return () => listeners.delete(cb);
  },
  getSnapshot(): OutlineItem[] {
    return items;
  },
  /** 发布新大纲（相等则跳过，不通知）。 */
  set(next: OutlineItem[]) {
    if (sameOutline(items, next)) return;
    items = next;
    for (const l of listeners) l();
  },
  reset() {
    this.set([]);
  },
};

export function useOutline(): OutlineItem[] {
  return useSyncExternalStore(outlineStore.subscribe, outlineStore.getSnapshot);
}
