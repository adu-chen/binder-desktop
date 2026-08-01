import { useSyncExternalStore } from "react";
import type { Node as PMNode } from "@tiptap/pm/model";
import { isContentBlock } from "../components/ed/extensions/BlockIdExtension";

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

/** 大纲提取纯逻辑：遍历所有内容块（含嵌套在 blockquote/列表内的 heading）；
 *  审计修复（2026-07-25 · A+D）：从 doc.forEach 改为 doc.descendants，支持嵌套 heading。 */
export function extractOutline(doc: PMNode): OutlineItem[] {
  const items: OutlineItem[] = [];
  doc.descendants((node, _pos) => {
    if (!isContentBlock(node as unknown as { type: { name: string } })) return true;
    if (node.type.name !== "heading") return true;
    const blockId = (node.attrs.blockId as string | null) ?? "";
    if (!blockId) return true;
    items.push({ blockId, level: Number(node.attrs.level) || 1, text: node.textContent });
    return false;
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
