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

// 投影占位演示集（PI-FE-07）：真实 diff 经 de.generate_diff 投影（实现期）。
const DEMO_DIFFS: DiffView[] = [
  { diffId: "d1", blockId: "demo-b1", title: "改写开头段", before: "从前有座山。", after: "很久很久以前，有一座云雾缭绕的山。", state: "pending" },
  { diffId: "d2", blockId: "demo-b2", title: "补充论据", before: "数据支持这一点。", after: "近三年的销售数据持续支持这一结论。", state: "conflict", conflictWith: "d5" },
  { diffId: "d3", blockId: "demo-b3", title: "修正措辞", before: "这个很重要。", after: "这一点至关重要。", state: "expired", expiredReason: "external_file_changed" },
  { diffId: "d4", blockId: "demo-b4", title: "精简冗余", before: "其实基本上大概可能。", after: "大概。", state: "error", errorCode: "WS_APPLY_REPLACE_FAILED" },
];

let state: DeState = { diffs: DEMO_DIFFS, batchState: "idle", batchExpiredCount: 0 };
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

let timelineCounter = 100;

/**
 * @GOV
 * codes:    BR-DE-STATE-001-RB-DE-DE-DIFF-002
 * type:     RB
 * chain:    DE-DIFF
 * rules:    BR-DE-STATE-001
 * term_ref: TERM-DE-001, TERM-DE-003
 * boundary: in=diffDecisionIntent | out=DiffViewState(terminalsIrreversibleAcceptedTriggersApply)
 */
/// diff 裁决意图投影（BR-DE-STATE-001）：accept 经 validating→applying→accepted（唯一触发落盘，
/// 不可逆）；reject→rejected（不落盘、装饰复位）；regenerate 令 expired 起新生成。三终态互斥不可逆——
/// 与后端 SM-DE-01 status 一一对应；过渡态为落盘往返呈现，权威落盘归 WS（ws.apply_diff）。
export const deStore = {
  subscribe(cb: () => void) {
    listeners.add(cb);
    return () => listeners.delete(cb);
  },
  getSnapshot(): DeState {
    return state;
  },

  /** 接受单条：pending → validating → applying → accepted（模拟落盘往返；实现期调 ws.apply_diff）。 */
  accept(id: string) {
    const target = state.diffs.find((d) => d.diffId === id);
    if (!target || target.state !== "pending") return; // conflict/终态不可直接接受
    mapDiff(id, (d) => ({ ...d, state: "validating" }));
    setTimeout(() => {
      mapDiff(id, (d) => ({ ...d, state: "applying" }));
      setTimeout(() => {
        timelineCounter += 1;
        mapDiff(id, (d) => ({ ...d, state: "accepted", timelineSeq: timelineCounter }));
      }, 240);
    }, 200);
  },

  /** 拒绝单条：→ rejected（不落盘，装饰复位）。conflict/error 亦可单独拒绝。 */
  reject(id: string) {
    mapDiff(id, (d) =>
      d.state === "accepted" || d.state === "expired" ? d : { ...d, state: "rejected" },
    );
  },

  /** 过期重生成：expired → pending（起新一条的占位；实现期由 de.generate_diff 产新 diff）。 */
  regenerate(id: string) {
    mapDiff(id, (d) =>
      d.state === "expired" ? { ...d, state: "pending", expiredReason: undefined } : d,
    );
  },

  /** error 重试：error → validating（重走落盘）。 */
  retry(id: string) {
    mapDiff(id, (d) => (d.state === "error" ? { ...d, state: "validating", errorCode: undefined } : d));
    setTimeout(() => {
      timelineCounter += 1;
      mapDiff(id, (d) => (d.state === "validating" ? { ...d, state: "accepted", timelineSeq: timelineCounter } : d));
    }, 300);
  },

  /**
   * 全部接受（倒序应用）：从文档靠后的 diff 先应用，避免先序落盘造成后序行号偏移（参考稿 §6.5）。
   * 过期/冲突/error 不计入可应用集，计入失效统计。
   */
  acceptAll() {
    const appliable = state.diffs.filter((d) => d.state === "pending");
    const expired = state.diffs.filter((d) => ["expired", "conflict", "error"].includes(d.state));
    if (appliable.length === 0) return;
    setDiffs(state.diffs, { batchState: "applying", batchExpiredCount: expired.length });
    // 倒序：按 blockId 锚的文档顺序假定与数组序一致，从后往前应用。
    const ordered = [...appliable].reverse();
    ordered.forEach((d, i) => {
      setTimeout(() => {
        timelineCounter += 1;
        mapDiff(d.diffId, (x) => ({ ...x, state: "accepted", timelineSeq: timelineCounter }));
        if (i === ordered.length - 1) {
          setDiffs(state.diffs, { batchState: "done" });
        }
      }, 180 * (i + 1));
    });
  },

  rejectAll() {
    setDiffs(
      state.diffs.map((d) =>
        d.state === "pending" ? { ...d, state: "rejected" } : d,
      ),
      { batchState: "idle" },
    );
  },
};

export function useDeStore(): DeState {
  return useSyncExternalStore(deStore.subscribe, deStore.getSnapshot);
}
