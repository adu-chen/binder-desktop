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

// toolCalls 为占位演示集（PI-FE-06）：真实工具调用经 SM-AG-01 tool 回合驱动（后续 PI）。
const DEMO_TOOLS: ToolCallView[] = [
  { id: "t1", name: "read_doc", state: "pending", detail: "排队中" },
  { id: "t2", name: "search", state: "executing", detail: "执行中 · 62%" },
  { id: "t3", name: "list_docs", state: "done", detail: "完成 · 12ms" },
  { id: "t4", name: "delete_file", state: "awaiting_confirmation", detail: "危险操作需授权" },
  { id: "t5", name: "fetch_url", state: "failed", detail: "超时，可重试" },
  { id: "t6", name: "rename", state: "skipped", detail: "多次失败后跳过" },
];

let state: AgState = {
  mode: "chat",
  messages: [],
  toolCalls: DEMO_TOOLS,
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
