import { useSyncExternalStore } from "react";

/**
 * DE diff 裁决状态投影层（DE-DIFF · SM-DE-01 · X-FORBID-007）。
 *
 * 权威态归后端 SM-DE-01：pending / accepted / rejected / expired（status 单一枚举，A-SYS-I-P-01）。
 * DiffCard 的 8 个视觉态 = 后端 status 的投影 + 裁决往返的过渡/呈现态：
 *   validating/applying 是 accepted 落盘往返（ws.apply_diff）期间的过渡态；
 *   conflict/error 是呈现态（冲突重检 BR-DE-DATA-003 / 落盘失败）。
 * 三者均非前端另立的平行业务态——单源 deStore 同时驱动 DiffCard 与 GreenReview（X-INTER-001）。
 *
 * 本 PI 以投影占位数据演示全 8 态；真实 diff 来自后端 de.generate_diff 投影、裁决经 ws.apply_diff 落盘（实现期接线）。
 */

export type DiffCardState =
  | "pending"
  | "validating"
  | "applying"
  | "accepted"
  | "rejected"
  | "expired"
  | "conflict"
  | "error";

/** GreenReview 4 态（参考稿 §6.7）——由 DiffCardState 派生，叠加在编辑器正文。 */
export type GreenReviewState = "pending" | "applying" | "accepted" | "expired";

/** expired 5 种细分原因（参考稿 §6.1，每种引导"重新生成"而非技术报错）。 */
export type ExpiredReason =
  | "document_revision_advanced"
  | "block_resolve_failed"
  | "overlapping_range"
  | "external_file_changed"
  | "apply_replace_failed";

export const EXPIRED_REASON_TEXT: Record<ExpiredReason, string> = {
  document_revision_advanced: "原文已被手动编辑，建议重新生成",
  block_resolve_failed: "原始块已不可定位，建议重新生成",
  overlapping_range: "与已接受的修改范围重叠，建议重新生成",
  external_file_changed: "文件在磁盘上被外部修改，建议重新生成",
  apply_replace_failed: "多次重试后写入仍失败，建议重新生成",
};

export interface DiffView {
  diffId: string;
  blockId: string; // GreenReview 装饰锚（与 ED BlockIdExtension 对齐）
  /** 目标文档相对路径（pending 按文件归属投影到文件树/标签）。 */
  relPath?: string;
  title: string;
  before: string;
  after: string;
  state: DiffCardState;
  expiredReason?: ExpiredReason;
  conflictWith?: string; // conflict 态：重叠的另一条 diff id
  errorCode?: string; // error 态：错误码
  timelineSeq?: number; // accepted 后的时间轴号
}

interface DeState {
  diffs: DiffView[];
  batchState: "idle" | "applying" | "done";
  batchExpiredCount: number;
}

// PI-FE-11：diff 来自真实 de.generate_diff/list_diffs（不再有 demo 占位）。
let state: DeState = { diffs: [], batchState: "idle", batchExpiredCount: 0 };
const listeners = new Set<() => void>();

function emit() {
  for (const l of listeners) l();
}
function setDiffs(diffs: DiffView[], patch: Partial<DeState> = {}) {
  state = { ...state, diffs, ...patch };
  emit();
}
function mapDiff(id: string, fn: (d: DiffView) => DiffView) {
  setDiffs(state.diffs.map((d) => (d.diffId === id ? fn(d) : d)));
}

/** DiffCardState → GreenReview 4 态（rejected → 无装饰）。 */
export function toGreenReview(s: DiffCardState): GreenReviewState | null {
  switch (s) {
    case "pending":
    case "conflict":
    case "error":
      return "pending";
    case "validating":
    case "applying":
      return "applying";
    case "accepted":
      return "accepted";
    case "expired":
      return "expired";
    case "rejected":
      return null;
  }
}

/**
 * @GOV
 * codes:    BR-DE-STATE-001-RB-DE-DE-DIFF-002
 * type:     RB
 * chain:    DE-DIFF
 * rules:    BR-DE-STATE-001
 * term_ref: TERM-DE-001, TERM-DE-003
 * boundary: in=diffStateProjection | out=DiffViewState(terminalsIrreversibleProjectsBackendSMDE01)
 */
/// diff 状态投影（BR-DE-STATE-001）：DiffView 8 视觉态投影后端 SM-DE-01（pending/accepted/rejected/
/// expired 权威态 + validating/applying/conflict/error 过渡呈现态）。三终态互斥不可逆——accepted 唯一
/// 经 de.accept_diff 触发 ws.apply_diff 落盘（编排在 deService，权威落盘归 WS，X-FORBID-007）。
export const deStore = {
  subscribe(cb: () => void) {
    listeners.add(cb);
    return () => listeners.delete(cb);
  },
  getSnapshot(): DeState {
    return state;
  },

  /** 新增一条待裁决修改（de.generate_diff 成功后投影；before/after 为本地预览）。 */
  addDiff(view: DiffView) {
    setDiffs([...state.diffs, view]);
  },

  /** 设置某条 diff 的视觉态（裁决往返各阶段；deService 编排调用）。终态不回退。 */
  setDiffState(id: string, next: DiffCardState, patch: Partial<DiffView> = {}) {
    mapDiff(id, (d) => {
      // 三终态互斥不可逆：已 accepted/rejected/expired 不被覆盖。
      if (["accepted", "rejected", "expired"].includes(d.state)) return d;
      return { ...d, state: next, ...patch };
    });
  },

  setBatch(batchState: DeState["batchState"], expiredCount = 0) {
    setDiffs(state.diffs, { batchState, batchExpiredCount: expiredCount });
  },

  /** 切换/关闭 workspace 时清空 diff（避免残留旧项目的待裁决修改）。 */
  reset() {
    setDiffs([], { batchState: "idle", batchExpiredCount: 0 });
  },
};

export function useDeStore(): DeState {
  return useSyncExternalStore(deStore.subscribe, deStore.getSnapshot);
}
