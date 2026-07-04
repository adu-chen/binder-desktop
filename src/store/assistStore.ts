import { useSyncExternalStore } from "react";

/**
 * AI 辅助入口状态（AG-ASSIST · PI-FE-08）。
 * 承载 ⌘K InlineAssist（TERM-AG-003）与 ⌘J Autocomplete（TERM-AG-004）两入口的视图交互态。
 * 两入口互斥（同一时刻至多一个开启），且**都不产出 DiffCard、不进入 SM-DE-01 Diff 链路**
 * （A-SYS-C-T-02 §6.6）——与 Chat Diff 严格区分。辅助 UI 态为视图层交互态，非 X-FORBID-007 业务态。
 */

export type AssistKind = "none" | "inline" | "autocomplete";

export interface AssistAnchor {
  x: number;
  y: number;
}

export interface AssistState {
  kind: AssistKind;
  anchor: AssistAnchor | null;
  /** ⌘K：选区文本与范围（Apply 时替换）。 */
  selectionText: string;
  selectionFrom: number;
  selectionTo: number;
  /** ⌘K：内联改写预览（ag_request_assist 候选）。 */
  inlinePreview: string | null;
  generating: boolean;
  /** ⌘J：续写候选（ag_request_assist，门禁/失败时为空）。 */
  suggestions: string[];
}

const EMPTY: AssistState = {
  kind: "none",
  anchor: null,
  selectionText: "",
  selectionFrom: 0,
  selectionTo: 0,
  inlinePreview: null,
  generating: false,
  suggestions: [],
};

let state: AssistState = EMPTY;
const listeners = new Set<() => void>();

function emit() {
  for (const l of listeners) l();
}
function setState(patch: Partial<AssistState>) {
  state = { ...state, ...patch };
  emit();
}

export const assistStore = {
  subscribe(cb: () => void) {
    listeners.add(cb);
    return () => listeners.delete(cb);
  },
  getSnapshot(): AssistState {
    return state;
  },
  /** ⌘K：在选区上方开局部修改（互斥：覆盖任何已开浮层）。 */
  openInline(anchor: AssistAnchor, selectionText: string, from: number, to: number) {
    state = { ...EMPTY, kind: "inline", anchor, selectionText, selectionFrom: from, selectionTo: to };
    emit();
  },
  /** ⌘J：在光标处开辅助续写（互斥）。 */
  openAutocomplete(anchor: AssistAnchor, suggestions: string[]) {
    state = { ...EMPTY, kind: "autocomplete", anchor, suggestions };
    emit();
  },
  setGenerating(generating: boolean) {
    setState({ generating });
  },
  setInlinePreview(inlinePreview: string) {
    setState({ inlinePreview, generating: false });
  },
  /** ⌘J：异步填入续写候选（ag_request_assist 返回后）。 */
  setSuggestions(suggestions: string[]) {
    setState({ suggestions, generating: false });
  },
  close() {
    state = EMPTY;
    emit();
  },
};

export function useAssistStore(): AssistState {
  return useSyncExternalStore(assistStore.subscribe, assistStore.getSnapshot);
}
