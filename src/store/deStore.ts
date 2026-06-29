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

/** 后端 SM-DE-01 运行态（Rust DiffState 的 Debug 名，A-DE-M-S-01 §2）；经广播投影到 8 视觉态。 */
export type RustDiffState =
  | "Generating"
  | "Pending"
  | "Accepted"
  | "Rejected"
  | "Expired"
  | "GenFailed";

/** Rust SM-DE-01 DiffState → DiffCard 视觉态（A-CORE-X-L-04 §6.2）。applying 落盘往返过渡态
 *  由 SM-WS-02 广播补充（Task 2-D），不在本映射。 */
export function mapRustDiffState(s: RustDiffState): DiffCardState {
  switch (s) {
    case "Generating":
      return "pending"; // 生成中暂显 pending
    case "Pending":
      return "pending";
    case "Accepted":
      return "accepted";
    case "Rejected":
      return "rejected";
    case "Expired":
      return "expired";
    case "GenFailed":
      return "error";
  }
}

/** 后端 SM-WS-02 落盘机运行态（Rust DiffApplyState 的 Debug 名，A-WS-M-S-01 §3）。 */
export type RustDiffApplyState =
  | "AwaitingDecision"
  | "Applying"
  | "Applied"
  | "Discarded"
  | "ApplyError";

/** Rust SM-WS-02 DiffApplyState → DiffCard 视觉态（A-CORE-X-L-04 Task 2-D）。
 *  Applied→accepted（落盘完成）；Discarded/AwaitingDecision 由 SM-DE-01 驱动终态，本机不投影（null）。 */
export function mapWsApplyState(s: RustDiffApplyState): DiffCardState | null {
  switch (s) {
    case "Applying":
      return "applying";
    case "Applied":
      return "accepted";
    case "ApplyError":
      return "error";
    case "Discarded":
    case "AwaitingDecision":
      return null; // 终态归 SM-DE-01（rejected/expired）；待裁决态不投影
  }
}

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

  /**
   * 投影后端 SM-DE-01 转移（A-CORE-X-L-04 Task 2-B/3-A · X-FORBID-007）：由 smListeners 监听
   * `sm:SM-DE-01:state_changed` 调用，是 diff **权威业务态**的唯一写入口。Rust DiffState → 8 视觉态；
   * 三终态互斥——已 accepted/rejected/expired 不被覆盖（与后端 transition 守护一致）。
   */
  applyBackendState(diffId: string, rustState: RustDiffState, _terminal: boolean) {
    mapDiff(diffId, (d) => {
      if (["accepted", "rejected", "expired"].includes(d.state)) return d;
      return { ...d, state: mapRustDiffState(rustState) };
    });
  },

  /**
   * 本地呈现态（非后端权威业务态）：裁决往返期间的 validating/applying 过渡态、及 conflict/error
   * 呈现态，由 deService 在 await 期间设置。已是后端终态者不被覆盖（不与广播相争）。
   * 注：SM-WS-02 集成后（Task 2-D）validating/applying 改由其落盘机广播驱动。
   */
  setLocalView(id: string, next: Extract<DiffCardState, "validating" | "applying" | "conflict" | "error">, patch: Partial<DiffView> = {}) {
    mapDiff(id, (d) => {
      if (["accepted", "rejected", "expired"].includes(d.state)) return d;
      return { ...d, state: next, ...patch };
    });
  },

  /**
   * 投影后端 SM-WS-02 落盘机转移（A-CORE-X-L-04 Task 2-D · X-FORBID-007）：由 smListeners 监听
   * `sm:SM-WS-02:state_changed` 调用。Applying→applying 过渡呈现、Applied→accepted、ApplyError→error；
   * Discarded/AwaitingDecision 不投影（终态归 SM-DE-01）。已是终态者不被覆盖。
   */
  applyWsApplyState(diffId: string, wsState: RustDiffApplyState) {
    const visual = mapWsApplyState(wsState);
    if (!visual) return;
    mapDiff(diffId, (d) => {
      if (["accepted", "rejected", "expired"].includes(d.state)) return d;
      return { ...d, state: visual };
    });
  },

  /** 合并展示元数据（如 accepted 后的 timelineSeq），不改变状态——供 deService 回填后端调用结果。 */
  attachMeta(id: string, patch: Partial<DiffView>) {
    mapDiff(id, (d) => ({ ...d, ...patch }));
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
