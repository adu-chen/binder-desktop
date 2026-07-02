import { useSyncExternalStore } from "react";

/**
 * ED 视图偏好 store（PI-ED-06/07 · 视图层，非 X-FORBID-007 限制的权威业务态）。
 * 按文档路径保存呈现偏好：纸张（分页几何）与方向；zoom 迁移入本仓（PI-ED-07）。
 * 偏好持久化 localStorage（跨会话保留），**不进文档模型、不落盘文档**（BR-ED-DATA-003 呈现层纯化）。
 * 几何计算为纯函数（横排=交换宽高），与 DOM/store 解耦可单测。
 */

export type PaperKind = "A4" | "A3" | "A5" | "Letter";
export type PaperOrientation = "portrait" | "landscape";

/** 分页几何（px @96dpi）。 */
export interface PageGeometry {
  width: number;
  height: number;
  marginX: number;
  marginY: number;
  gap: number;
}

/** 纸张预设（竖排基准尺寸，px @96dpi；ISO 216 / US Letter）。 */
export const PAPERS: Record<PaperKind, { width: number; height: number }> = {
  A4: { width: 794, height: 1123 },
  A3: { width: 1123, height: 1587 },
  A5: { width: 559, height: 794 },
  Letter: { width: 816, height: 1056 },
};

const MARGIN_X = 76;
const MARGIN_Y = 60;
const GAP = 28;

/** 纸张 + 方向 → 分页几何（纯函数）：横排交换宽高；页边距/缝隙恒定。 */
export function getPageGeometry(paper: PaperKind, orientation: PaperOrientation): PageGeometry {
  const base = PAPERS[paper];
  const [width, height] = orientation === "landscape" ? [base.height, base.width] : [base.width, base.height];
  return { width, height, marginX: MARGIN_X, marginY: MARGIN_Y, gap: GAP };
}

/** 每页可用内容高度（切页预算，纯函数）。 */
export function pageContentHeight(geo: PageGeometry): number {
  return geo.height - geo.marginY * 2;
}

export interface EdViewPrefs {
  paper: PaperKind;
  orientation: PaperOrientation;
  /** 编辑预览区缩放（1 = 100%）；PI-ED-07 迁移（#10 per-doc 持久）。 */
  zoom: number;
}

const DEFAULT_PREFS: EdViewPrefs = { paper: "A4", orientation: "portrait", zoom: 1 };
const LS_PREFIX = "binder.ed.viewprefs.";

function isPaperKind(v: unknown): v is PaperKind {
  return v === "A4" || v === "A3" || v === "A5" || v === "Letter";
}

/** 读取某文档的持久偏好（无/损坏回默认；字段级校验防脏数据）。 */
function loadPrefs(path: string): EdViewPrefs {
  try {
    const raw = localStorage.getItem(LS_PREFIX + path);
    if (!raw) return { ...DEFAULT_PREFS };
    const p = JSON.parse(raw) as Partial<EdViewPrefs>;
    return {
      paper: isPaperKind(p.paper) ? p.paper : DEFAULT_PREFS.paper,
      orientation: p.orientation === "landscape" ? "landscape" : "portrait",
      zoom: typeof p.zoom === "number" && Number.isFinite(p.zoom) ? p.zoom : DEFAULT_PREFS.zoom,
    };
  } catch {
    return { ...DEFAULT_PREFS };
  }
}

// 内存缓存（按路径）；写穿 localStorage。
let cache: Record<string, EdViewPrefs> = {};
const listeners = new Set<() => void>();

function emit() {
  for (const l of listeners) l();
}
function persist(path: string, prefs: EdViewPrefs) {
  try {
    localStorage.setItem(LS_PREFIX + path, JSON.stringify(prefs));
  } catch {
    /* 配额/隐私模式不可写 → 仅内存态 */
  }
}
function update(path: string, patch: Partial<EdViewPrefs>) {
  const next = { ...edViewPrefsStore.getPrefs(path), ...patch };
  cache = { ...cache, [path]: next };
  persist(path, next);
  emit();
}

export const edViewPrefsStore = {
  subscribe(cb: () => void) {
    listeners.add(cb);
    return () => listeners.delete(cb);
  },
  getSnapshot(): Record<string, EdViewPrefs> {
    return cache;
  },
  /** 取某文档偏好（缓存未命中时从 localStorage 载入并入缓存）。 */
  getPrefs(path: string): EdViewPrefs {
    const hit = cache[path];
    if (hit) return hit;
    const loaded = loadPrefs(path);
    cache = { ...cache, [path]: loaded };
    return loaded;
  },
  setPaper(path: string, paper: PaperKind) {
    update(path, { paper });
  },
  setOrientation(path: string, orientation: PaperOrientation) {
    update(path, { orientation });
  },
  setZoom(path: string, zoom: number) {
    update(path, { zoom });
  },
};

/** React hook：订阅偏好表（组件内以 getPrefs(path) 取具体文档）。 */
export function useEdViewPrefs(): Record<string, EdViewPrefs> {
  return useSyncExternalStore(edViewPrefsStore.subscribe, edViewPrefsStore.getSnapshot);
}
