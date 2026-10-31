import { useSyncExternalStore } from "react";

/**
 * AI 辅助入口状态（AG-ASSIST / BR-AG-STATE-003）。
 * 承载 ⌘K InlineAssist（TERM-AG-003）与 ⌘J Autocomplete（TERM-AG-004）两入口的视图交互态。
 * 两入口互斥（同一时刻至多一个开启），且**都不产出 DiffCard、不进入 SM-DE-01 Diff 链路**
 * （A-SYS-C-T-02 §6.6）——与 Chat Diff 严格区分。辅助 UI 态为视图层交互态，非 X-FORBID-007 业务态。
 */

export type AssistKind = "none" | "inline" | "autocomplete";
export type AssistFailureReason =
  | "gate_denied"
  | "gate_offline"
  | "gate_unavailable"
  | "gateway_error"
  | "empty_output"
  | "parse_underflow"
  | "stale_request"
  | "invoke_error"
  | "candidate_count_mismatch"
  | "anchor_lost"
  | "unknown"
  | (string & {});

export const ASSIST_TRIGGER_DEBOUNCE_MS = 450;

export interface AssistAnchor {
  x: number;
  y: number;
  placement?: string;
  width?: number;
  height?: number;
}

export interface AssistTarget {
  docPath: string;
  loadSeq: number;
  blockId: string;
  start: number;
  end: number;
  text: string;
  prefix: string;
  before: string;
  after: string;
  mode: "selection" | "cursor";
}

export interface AssistState {
  kind: AssistKind;
  requestId: string | null;
  anchor: AssistAnchor | null;
  target: AssistTarget | null;
  /** ⌘K：选区文本；光标插入态为空串。 */
  selectionText: string;
  /** ⌘K：局部修改预览（ag_request_assist 候选）。 */
  inlinePreview: string | null;
  generating: boolean;
  /** ⌘J：3 条续写候选（ag_request_assist，门禁/失败时为空）。 */
  suggestions: string[];
  /** ⌘J：内部诊断原因；普通 UI 不弹错，但测试/诊断不得丢失失败分型。 */
  autocompleteFailureReason: AssistFailureReason | null;
}

const EMPTY: AssistState = {
  kind: "none",
  requestId: null,
  anchor: null,
  target: null,
  selectionText: "",
  inlinePreview: null,
  generating: false,
  suggestions: [],
  autocompleteFailureReason: null,
};

let state: AssistState = EMPTY;
let seq = 0;
let lastTriggerAt: Record<Exclude<AssistKind, "none">, number> = { inline: 0, autocomplete: 0 };
const listeners = new Set<() => void>();

function emit() {
  for (const l of listeners) l();
}
function setState(patch: Partial<AssistState>) {
  state = { ...state, ...patch };
  emit();
}
function nextRequestId(): string {
  seq += 1;
  return `as-${Date.now()}-${seq}`;
}

/**
 * @GOV
 * codes:    BR-AG-STATE-003-RB-AG-AG-ASSIST-008
 * type:     RB
 * chain:    AG-ASSIST
 * rules:    BR-AG-STATE-003
 * term_ref: TERM-AG-003, TERM-AG-004
 * boundary: in=AssistViewEventAndAssistBackendResult | consumer=BR-AG-STATE-003
 *           | out=AssistViewState(latestWinsCandidatesOrInternalFailureReason)
 *           | does_not=drop_assist_failure_reason_or_apply_stale_candidate
 */
export const assistStore = {
  subscribe(cb: () => void) {
    listeners.add(cb);
    return () => listeners.delete(cb);
  },
  /** 快捷键入口防抖：默认窗口内重复触发静默忽略，不发起新请求。 */
  claimTrigger(kind: Exclude<AssistKind, "none">, now = Date.now()): boolean {
    const last = lastTriggerAt[kind];
    if (last > 0 && now >= last && now - last < ASSIST_TRIGGER_DEBOUNCE_MS) return false;
    lastTriggerAt[kind] = now;
    return true;
  },
  getSnapshot(): AssistState {
    return state;
  },
  /** ⌘K：在选区或光标上方开局部修改（互斥：覆盖任何已开浮层）。 */
  openInline(anchor: AssistAnchor, selectionText: string, target: AssistTarget): string {
    const requestId = nextRequestId();
    state = { ...EMPTY, kind: "inline", requestId, anchor, selectionText, target };
    emit();
    return requestId;
  },
  /** ⌘J：在光标处开辅助续写（互斥）。 */
  openAutocomplete(anchor: AssistAnchor, target: AssistTarget): string {
    const requestId = nextRequestId();
    state = { ...EMPTY, kind: "autocomplete", requestId, anchor, target, generating: true };
    emit();
    return requestId;
  },
  /** 当前回合内发起新请求：latest-wins，旧 requestId 返回后必须被忽略。 */
  startRequest(kind: Exclude<AssistKind, "none">): string | null {
    if (state.kind !== kind || !state.target || !state.anchor) return null;
    const requestId = nextRequestId();
    state = {
      ...state,
      requestId,
      generating: true,
      inlinePreview: kind === "inline" ? null : state.inlinePreview,
      suggestions: kind === "autocomplete" ? [] : state.suggestions,
      autocompleteFailureReason: kind === "autocomplete" ? null : state.autocompleteFailureReason,
    };
    emit();
    return requestId;
  },
  isCurrent(requestId: string, kind?: Exclude<AssistKind, "none">): boolean {
    return state.requestId === requestId && state.kind !== "none" && (!kind || state.kind === kind);
  },
  setGenerating(requestId: string, generating: boolean) {
    if (state.requestId !== requestId) return;
    setState({ generating, autocompleteFailureReason: generating ? null : state.autocompleteFailureReason });
  },
  /** 视觉锚点重算：滚动/缩放/resize 只更新浮层位置，不改变逻辑锚与候选生命周期。 */
  setAnchor(requestId: string, anchor: AssistAnchor) {
    if (state.requestId !== requestId || state.kind === "none") return;
    setState({ anchor });
  },
  setInlinePreview(requestId: string, inlinePreview: string) {
    if (state.requestId !== requestId || state.kind !== "inline") return;
    setState({ inlinePreview, generating: false });
  },
  /** ⌘J：异步填入 3 条续写候选（ag_request_assist 返回后）。 */
  setSuggestions(requestId: string, suggestions: string[]) {
    if (state.requestId !== requestId || state.kind !== "autocomplete") return;
    setState({ suggestions: suggestions.slice(0, 3), generating: false, autocompleteFailureReason: null });
  },
  /** ⌘J：失败分型进入内部状态；普通 UI 保持轻量空态，但原因不可被 service/editor 层吞掉。 */
  setAutocompleteFailure(requestId: string, reason: AssistFailureReason) {
    if (state.requestId !== requestId || state.kind !== "autocomplete") return;
    setState({ suggestions: [], generating: false, autocompleteFailureReason: reason });
  },
  close() {
    state = EMPTY;
    emit();
  },
  resetForTest() {
    state = EMPTY;
    seq = 0;
    lastTriggerAt = { inline: 0, autocomplete: 0 };
    emit();
  },
};

export function useAssistStore(): AssistState {
  return useSyncExternalStore(assistStore.subscribe, assistStore.getSnapshot);
}
