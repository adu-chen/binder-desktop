import { useSyncExternalStore } from "react";

/**
 * 布局尺寸投影（视图层导航态，非 X-FORBID-007 限制的权威业务态——同 activePanel）。
 * 三栏（side 文件树 / center 编辑器 / chat AI）宽度可拖动自适应：side/chat 为像素宽（钳制在 min/max），
 * center 取剩余弹性空间（1fr）。宽度持久化到 localStorage，跨会话保留用户偏好。
 * 纯钳制逻辑 clampWidth 与 DOM/拖动解耦，可单测。
 */

/** 各栏尺寸约束（px）。center 无约束（弹性 1fr，最小由栅格 min-width:0 兜底）。 */
export const PANE_LIMITS = {
  side: { default: 272, min: 180, max: 520 },
  chat: { default: 372, min: 280, max: 620 },
} as const;

const LS_SIDE = "binder.layout.sideW";
const LS_CHAT = "binder.layout.chatW";

/** 纯逻辑：把宽度钳制到 [min, max]（拖动结果与恢复持久值共用；NaN → 回落 min）。 */
export function clampWidth(w: number, min: number, max: number): number {
  if (!Number.isFinite(w)) return min;
  return Math.max(min, Math.min(max, w));
}

export interface LayoutState {
  sideW: number;
  chatW: number;
}

/** 从 localStorage 读取并钳制持久宽度（无值/异常回落默认）。 */
function loadPersisted(): LayoutState {
  const read = (key: string, lim: { default: number; min: number; max: number }) => {
    try {
      const raw = localStorage.getItem(key);
      if (raw === null) return lim.default;
      return clampWidth(Number(raw), lim.min, lim.max);
    } catch {
      return lim.default;
    }
  };
  return {
    sideW: read(LS_SIDE, PANE_LIMITS.side),
    chatW: read(LS_CHAT, PANE_LIMITS.chat),
  };
}

let state: LayoutState = loadPersisted();
const listeners = new Set<() => void>();

function commit(next: LayoutState) {
  state = next;
  for (const l of listeners) l();
}
function persist(key: string, value: number) {
  try {
    localStorage.setItem(key, String(value));
  } catch {
    /* 隐私模式/配额不可写 → 仅内存态，不阻断拖动 */
  }
}

export const layoutStore = {
  subscribe(cb: () => void) {
    listeners.add(cb);
    return () => listeners.delete(cb);
  },
  getSnapshot(): LayoutState {
    return state;
  },
  /** 设 side 宽（钳制 + 持久化）。 */
  setSideW(w: number) {
    const v = clampWidth(w, PANE_LIMITS.side.min, PANE_LIMITS.side.max);
    if (v === state.sideW) return;
    persist(LS_SIDE, v);
    commit({ ...state, sideW: v });
  },
  /** 设 chat 宽（钳制 + 持久化）。 */
  setChatW(w: number) {
    const v = clampWidth(w, PANE_LIMITS.chat.min, PANE_LIMITS.chat.max);
    if (v === state.chatW) return;
    persist(LS_CHAT, v);
    commit({ ...state, chatW: v });
  },
  /** 恢复默认宽（拖柄双击复位）。 */
  resetSide() {
    this.setSideW(PANE_LIMITS.side.default);
  },
  resetChat() {
    this.setChatW(PANE_LIMITS.chat.default);
  },
};

export function useLayoutStore(): LayoutState {
  return useSyncExternalStore(layoutStore.subscribe, layoutStore.getSnapshot);
}
