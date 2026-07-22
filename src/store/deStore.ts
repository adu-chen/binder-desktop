import { useSyncExternalStore } from "react";
import type { TabState } from "./edStore";
import type { StyleOp } from "../services/edStyleOps";

/**
 * DE diff 裁决状态投影层（DE-DIFF · SM-DE-01 · X-FORBID-007）。
 *
 * 权威态归后端 SM-DE-01：pending / accepted / rejected / expired（status 单一枚举，A-SYS-I-P-01）。
 * DiffCard 的 8 个视觉态 = 后端 status 的投影 + 裁决往返的过渡/呈现态：
 *   validating/applying 是后台 accepted apply 往返（ws.apply_diff）期间的过渡态；
 *   conflict/error 是呈现态（冲突重检 BR-DE-DATA-003 / 落盘失败）。
 * 三者均非前端另立的平行业务态——单源 deStore 同时驱动 DiffCard 与 GreenReview（X-INTER-001）。
 *
 * 真实 diff 来自后端 de.generate_diff 投影；当前编辑器草稿路径经 save 落盘，后台路径可经 ws.apply_diff 落盘。
 *
 * 三态归属（A-CORE-C-T-01 §4 映射表）：deStore 是 LogicalCandidate（逻辑态候选）的前端投影层，
 * 不是 DisplayState。其所持 DiffView[].state 变化通过 ED 受控接口驱动 ProseMirror doc 修改，
 * 这是"逻辑候选态变化 → 经受控接口修改逻辑态"的合法链路，不是"显示态变化触发逻辑态"。
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

/** 裁决状态之外的内容效果态：供历史只读卡区分已保存与已接受但内容已丢失。 */
export type DiffEffectStatus = "not_applicable" | "awaiting_save" | "saved" | "discarded_on_abnormal_exit";

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

export type DiffDraftMode = "current_editor" | "open_session" | "background";
export type DiffSnapshotSource = "active_editor" | "active_editor_session" | "open_editor_session" | "disk_snapshot";

export const EXPIRED_REASON_TEXT: Record<ExpiredReason, string> = {
  document_revision_advanced: "原文已被手动编辑，建议重新生成",
  block_resolve_failed: "原始块已不可定位，建议重新生成",
  overlapping_range: "与已接受的修改范围重叠，建议重新生成",
  external_file_changed: "文件在磁盘上被外部修改，建议重新生成",
  apply_replace_failed: "多次重试后写入仍失败，建议重新生成",
};

export interface DiffView {
  diffId: string;
  /** 归属的 AG 会话；用于把 DiffCard 插回对应会话的信息流。 */
  sessionId?: string;
  /** 归属的 AgentTurn；用于审计与同回合批量组织。 */
  turnId?: string;
  /** DiffCard 在消息流中的锚点消息 id，通常为本回合 assistant 消息。 */
  anchorMessageId?: string;
  /** 前端生成顺序；同一消息下多张卡按此稳定排序。 */
  createdAt?: number;
  blockId: string; // GreenReview 装饰锚（与 ED BlockIdExtension 对齐）
  /** 目标块 0 起序号；text 文档/open session 无 blockId 时使用。 */
  blockIndex?: number | null;
  /** 目标文档相对路径（pending 按文件归属投影到文件树/标签）。 */
  relPath?: string;
  /** 人读位置摘要（如“Block 3”或“整篇”），与 blockId 一起用于 DiffCard 定位呈现。 */
  locationLabel?: string;
  /** AgentEdit 修改类型；Editor 只允许显式 replace_document 执行整篇草稿替换。 */
  patchKind?: "replace_block" | "replace_document" | "style_ops";
  /** DiffProjection 分流路径：active 草稿 / open session 自动应用 / 未打开后台路径。 */
  draftMode?: DiffDraftMode;
  /** DiffProjection 生成基线来源。 */
  snapshotSource?: DiffSnapshotSource;
  /** 生成时 ED session 版本；open session 必填。 */
  baseSessionVersion?: number;
  /** 生成时内容哈希；open session 必填。 */
  baseContentHash?: string;
  title: string;
  before: string;
  after: string;
  state: DiffCardState;
  /** 样式变更摘要（PI-ED-11 · D-3 方案A：回合产出样式修改经 DiffCard 裁决）。人读描述行，
   *  由 StyleOp 序列（载荷 style_ops，A-DE-M-T-01 v1.1）投影；空/缺省 = 纯文本 diff。 */
  styleOps?: string[];
  /** 可执行 StyleOp 载荷；由编辑器草稿路径消费，DiffCard 仍显示 styleOps 摘要。 */
  styleOpsPayload?: StyleOp[];
  /** 已将 after 应用到当前编辑器草稿；accept 只确认，save 才落盘。 */
  draftApplied?: boolean;
  /** 已自动应用到已打开非激活文档 LogicalState；DiffCard 禁用裁决。 */
  openSessionApplied?: boolean;
  /** 草稿应用前的完整编辑器内容，用于 reject 后判断是否可恢复原标签态。 */
  draftBaseContent?: string;
  /** 草稿应用后（刚应用完那一刻）的编辑器内容快照，用于 D4 后置编辑检测：
   *  reject 时比较当前内容与该字段：相等 = 无后置编辑，不等 = 用户又编辑过。 */
  draftAfterContent?: string;
  /** 草稿应用前的标签态；reject 完全回到 base content 时恢复。 */
  draftBaseTabState?: TabState;
  /** 草稿应用前 edStore 对应路径的 tabState 是否为 dirty（布尔值）；
   *  用于 D4 修复（A-CORE-C-T-01 §5.4）：避免 pending_diff 覆盖后真实脏态丢失。
   *  true = 用户在 diff 到来前已编辑过文档，裁决后（accepted/rejected）应恢复为 dirty。 */
  draftBaseDirty?: boolean;
  /** 草稿已随普通保存链落盘，编辑器 GreenReview 应复位，但 DiffCard 记录保留。 */
  draftSaved?: boolean;
  expiredReason?: ExpiredReason;
  conflictWith?: string; // conflict 态：重叠的另一条 diff id
  errorCode?: string; // error 态：错误码
  timelineSeq?: number; // accepted 后的时间轴号
  /** 历史恢复卡或不可用卡：只展示，不允许从历史重新触发裁决/重试。 */
  readonly?: boolean;
  /** 后端 pending_diffs.effect_status 的历史投影；实时新卡尚未持久化时可为空。 */
  effectStatus?: DiffEffectStatus;
  /** 后端 terminal_reason 的历史投影，如 abnormal_exit。 */
  terminalReason?: string | null;
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

/** DiffCardState → GreenReview 4 态（rejected/error → 无装饰）。 */
export function toGreenReview(s: DiffCardState): GreenReviewState | null {
  switch (s) {
    case "pending":
    case "conflict":
      return "pending";
    case "error":
      return null;
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
/// expired 权威态 + validating/applying/conflict/error 过渡呈现态）。三终态互斥不可逆——当前编辑器草稿
/// 路径 accepted 只确认并等待 save，后台路径 accepted 可触发 ws.apply_diff（编排在 deService）。
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

  expireLocal(id: string, patch: Partial<DiffView> = {}) {
    mapDiff(id, (d) => {
      if (["accepted", "rejected", "expired"].includes(d.state)) return d;
      return { ...d, state: "expired", readonly: true, ...patch };
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

  /** 标记草稿已应用到编辑器。记录 baseContent/baseTabState/baseDirty 及 afterContent（D4 修复，
   *  A-CORE-C-T-01 §5.4），使 reject 后可通过比较当前内容与 afterContent 准确判断是否有后置编辑。
   *  baseContent/baseTabState 从 edStore 活动会话取；baseDirty 取该时的 tabState === "dirty"；
   *  afterContent 为 diff 应用后的编辑器内容快照。 */
  markDraftApplied(id: string, baseContent?: string, baseTabState?: TabState, afterContent?: string) {
    const baseDirty = baseTabState === "dirty";
    mapDiff(id, (d) => ({
      ...d,
      draftApplied: true,
      draftBaseContent: baseContent,
      draftBaseTabState: baseTabState,
      draftBaseDirty: baseDirty,
      draftAfterContent: afterContent,
      draftSaved: false,
    }));
  },

  markOpenSessionApplied(id: string, baseContent?: string, afterContent?: string) {
    mapDiff(id, (d) => ({
      ...d,
      state: "accepted",
      readonly: true,
      draftApplied: true,
      openSessionApplied: true,
      draftBaseContent: baseContent,
      draftAfterContent: afterContent,
      draftSaved: false,
    }));
  },

  markDraftSaved(relPath: string) {
    setDiffs(
      state.diffs.map((d) =>
        d.relPath === relPath && d.draftApplied && d.state === "accepted" ? { ...d, draftSaved: true } : d,
      ),
    );
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
