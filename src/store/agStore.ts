import { useSyncExternalStore } from "react";

/**
 * AG 对话状态投影层（AG-TURN · SM-AG-01 · X-FORBID-007）。
 * messages 是 AgentTurn（TERM-AG-001）会话视图；toolCalls 是 ToolCall（TERM-AG-002）状态投影。
 * 回合权威态归后端 SM-AG-01，前端只呈现与转达意图，不另立可漂移的并行回合态。
 */

/** 代理编辑（AgentEdit）面板双模式：edit=编辑（AI 交互精确修改当前内容，diff 裁决）；build=构建（项目级蓝图与整体构建，接 CBT）。 */
export type AgMode = "edit" | "build";

export interface ChatMessage {
  id: string;
  role: "user" | "assistant";
  content: string;
  streaming?: boolean;
  error?: boolean;
}

/** ToolCall 6 状态（参考稿 §6.2，视觉互斥）。 */
export type ToolState =
  | "pending"
  | "executing"
  | "done"
  | "awaiting_confirmation"
  | "failed"
  | "skipped";

export interface ToolCallView {
  id: string;
  name: string;
  state: ToolState;
  detail?: string;
}

/** 后端 SM-AG-01 回合机运行态（Rust TurnState 的 Debug 名，A-AG-M-S-01 §2）。 */
export type RustTurnState =
  | "Idle"
  | "Assembling"
  | "AwaitingGate"
  | "Streaming"
  | "HandingOff"
  | "TurnError"
  | "Completed"
  | "Cancelled"
  | "Denied"
  | "HandedOver";

/** 回合“进行中”的活动态集合：sending（输入门控）= 处于这些态之一。其余（Idle/各终态/TurnError）→ 非进行中。 */
const ACTIVE_TURN_STATES: ReadonlySet<RustTurnState> = new Set([
  "Assembling",
  "AwaitingGate",
  "Streaming",
  "HandingOff",
]);

interface AgState {
  mode: AgMode;
  messages: ChatMessage[];
  toolCalls: ToolCallView[];
  sending: boolean;
}

// toolCalls 初始为空：真实工具调用经 SM-AG-01 tool 回合驱动（云端接通后）。ToolCallView 类型保留供其时投影。
let state: AgState = {
  mode: "edit",
  messages: [],
  toolCalls: [],
  sending: false,
};
const listeners = new Set<() => void>();

function emit() {
  for (const l of listeners) l();
}
function setState(patch: Partial<AgState>) {
  state = { ...state, ...patch };
  emit();
}

export const agStore = {
  subscribe(cb: () => void) {
    listeners.add(cb);
    return () => listeners.delete(cb);
  },
  getSnapshot(): AgState {
    return state;
  },
  setMode(mode: AgMode) {
    setState({ mode });
  },
  setSending(sending: boolean) {
    setState({ sending });
  },
  /**
   * 投影后端 SM-AG-01 回合机转移（A-CORE-X-L-04 Task 2-E · X-FORBID-007）：由 smListeners 监听
   * `sm:SM-AG-01:state_changed` 调用。sending（输入门控）= 回合处于活动态（Assembling/AwaitingGate/
   * Streaming/HandingOff）；到达任一终态或 TurnError 即释放。消息流/工具卡仍由 Channel 投影（互不替代）。
   */
  applyTurnState(_turnId: string, turnState: RustTurnState) {
    setState({ sending: ACTIVE_TURN_STATES.has(turnState) });
  },
  pushMessage(msg: ChatMessage) {
    setState({ messages: [...state.messages, msg] });
  },
  /** 追加流式分片到指定消息（模拟 turn_stream 逐字回显）。 */
  appendToMessage(id: string, chunk: string) {
    setState({
      messages: state.messages.map((m) =>
        m.id === id ? { ...m, content: m.content + chunk } : m,
      ),
    });
  },
  /** 标记消息流式结束。 */
  finishMessage(id: string) {
    setState({
      messages: state.messages.map((m) => (m.id === id ? { ...m, streaming: false } : m)),
    });
  },
  /** 新增一条工具调用卡（executing）；返回卡 id（PI-AG-08D，投影 ToolRequest）。 */
  addToolCall(name: string, detail?: string): string {
    const id = `tc-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
    setState({ toolCalls: [...state.toolCalls, { id, name, state: "executing", detail }] });
    return id;
  },
  /** 将该 name 最早的 executing 卡置 done/failed（事件无 call id，按 name+executing 先到先配）。 */
  resolveToolCall(name: string, ok: boolean, detail?: string) {
    let matched = false;
    setState({
      toolCalls: state.toolCalls.map((t) => {
        if (!matched && t.name === name && t.state === "executing") {
          matched = true;
          return { ...t, state: ok ? "done" : "failed", detail: detail ?? t.detail };
        }
        return t;
      }),
    });
  },
  /** 清空工具卡（新回合起）。 */
  clearToolCalls() {
    setState({ toolCalls: [] });
  },
};

export function useAgStore(): AgState {
  return useSyncExternalStore(agStore.subscribe, agStore.getSnapshot);
}
