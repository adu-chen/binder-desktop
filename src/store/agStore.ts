import { useSyncExternalStore } from "react";

/**
 * AG 对话状态投影层（AG-TURN · SM-AG-01 · X-FORBID-007）。
 * messages 是 AgentTurn（TERM-AG-001）会话视图；toolCalls 是 ToolCall（TERM-AG-002）状态投影。
 * 回合权威态归后端 SM-AG-01，前端只呈现与转达意图，不另立可漂移的并行回合态。
 */

export type ChatMode = "chat" | "agent";

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

interface AgState {
  mode: ChatMode;
  messages: ChatMessage[];
  toolCalls: ToolCallView[];
  sending: boolean;
}

// toolCalls 初始为空：真实工具调用经 SM-AG-01 tool 回合驱动（云端接通后）。ToolCallView 类型保留供其时投影。
let state: AgState = {
  mode: "chat",
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
  setMode(mode: ChatMode) {
    setState({ mode });
  },
  setSending(sending: boolean) {
    setState({ sending });
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
};

export function useAgStore(): AgState {
  return useSyncExternalStore(agStore.subscribe, agStore.getSnapshot);
}
