import { useSyncExternalStore } from "react";
import type { MessageContentPart } from "../services/referencePayload";

/**
 * AG 会话状态投影层（AG-TURN · SM-AG-01 · X-FORBID-007）。
 * messages 是 AgentTurn（TERM-AG-001）会话视图；toolCalls 是 ToolCall（TERM-AG-002）状态投影。
 * 回合权威态归后端 SM-AG-01（sending 门控投影），前端只呈现与转达意图，不另立可漂移的并行回合态。
 *
 * 多标签会话（PI-AG-UI-01）：会话分组是**呈现层组织**（运行期内存态，非后端业务态）；
 * 关闭标签=归档（本次运行内经「历史会话」可重开），跨重启持久化随 BR-AG-PERSIST-001
 * （agent_turns 接线批）落地，本结构按可持久化形状设计。
 */

/** 代理编辑（AgentEdit）面板双模式：edit=编辑（AI 交互精确修改当前内容，diff 裁决）；build=构建（项目级蓝图与整体构建，接 CBT）。 */
export type AgMode = "edit" | "build";

export interface ChatMessage {
  id: string;
  role: "user" | "assistant";
  content: string;
  parts?: MessageContentPart[];
  toolCalls?: ToolCallView[];
  streaming?: boolean;
  error?: boolean;
  turnId?: string;
  relPath?: string | null;
  turnStatus?: AgentTurnStatus;
}

export type AgentTurnStatus = "completed" | "error" | "cancelled" | "gate_rejected" | "draft";

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
  readonly?: boolean;
  turnId?: string;
  anchorMessageId?: string;
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

/** 一个标签会话（呈现层组织；持久化形状预留）。 */
export interface AgSession {
  id: string;
  /** 标签标题：新会话为「新对话」，首条用户消息后取其前缀。 */
  title: string;
  mode: AgMode;
  messages: ChatMessage[];
  toolCalls: ToolCallView[];
  createdAt: number;
  /** 标签已关闭（归档）：不显示为标签，可从历史会话重开。 */
  archived: boolean;
  /** 回合内容是否已经加载；历史恢复时仅活动会话 eager-load，其余点击懒加载。 */
  turnsLoaded: boolean;
}

interface AgState {
  sessions: AgSession[];
  activeSessionId: string;
  /** 回合级全局门控（后端 SM-AG-01 单回合投影；一次仅一个活动回合）。 */
  sending: boolean;
}

const sid = () => `s-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
function blankSession(): AgSession {
  return {
    id: sid(),
    title: "新对话",
    mode: "edit",
    messages: [],
    toolCalls: [],
    createdAt: Date.now(),
    archived: false,
    turnsLoaded: true,
  };
}

const initial = blankSession();
let state: AgState = { sessions: [initial], activeSessionId: initial.id, sending: false };
const listeners = new Set<() => void>();

function emit() {
  for (const l of listeners) l();
}
function setState(patch: Partial<AgState>) {
  state = { ...state, ...patch };
  emit();
}
function mapSession(id: string, fn: (s: AgSession) => AgSession) {
  setState({ sessions: state.sessions.map((s) => (s.id === id ? fn(s) : s)) });
}

/** 取活动会话（恒存在：关闭最后一个标签会自动新建）。 */
export function activeSession(s: AgState): AgSession {
  return s.sessions.find((x) => x.id === s.activeSessionId) ?? s.sessions[0];
}

export const agStore = {
  subscribe(cb: () => void) {
    listeners.add(cb);
    return () => listeners.delete(cb);
  },
  getSnapshot(): AgState {
    return state;
  },

  reset() {
    const fresh = blankSession();
    setState({ sessions: [fresh], activeSessionId: fresh.id, sending: false });
  },

  hydrateSessions(sessions: AgSession[], activeSessionId?: string) {
    const next = sessions.length ? sessions : [blankSession()];
    const active =
      (activeSessionId && next.some((s) => s.id === activeSessionId) && activeSessionId) ||
      next.find((s) => !s.archived)?.id ||
      next[0].id;
    setState({ sessions: next, activeSessionId: active, sending: false });
  },

  hydrateSessionTurns(sessionId: string, messages: ChatMessage[], toolCalls: ToolCallView[]) {
    mapSession(sessionId, (s) => ({ ...s, messages, toolCalls, turnsLoaded: true }));
  },

  // ── 会话（标签）操作 ──────────────────────────────────────────────
  /** 新建会话并激活；返回会话 id。 */
  newSession(): string {
    const s = blankSession();
    setState({ sessions: [...state.sessions, s], activeSessionId: s.id });
    return s.id;
  },
  /** 切换/重开会话（归档的自动取消归档回到标签栏）。 */
  switchSession(id: string) {
    if (!state.sessions.some((s) => s.id === id)) return;
    setState({
      sessions: state.sessions.map((s) => (s.id === id ? { ...s, archived: false } : s)),
      activeSessionId: id,
    });
  },
  /** 关闭标签=归档（历史可重开）；关闭活动标签切换到最近的打开标签，无剩余则新建。 */
  closeSession(id: string) {
    const sessions = state.sessions.map((s) => (s.id === id ? { ...s, archived: true } : s));
    let activeSessionId = state.activeSessionId;
    if (activeSessionId === id) {
      const open = sessions.filter((s) => !s.archived);
      if (open.length > 0) {
        activeSessionId = open[open.length - 1].id;
        setState({ sessions, activeSessionId });
      } else {
        const fresh = blankSession();
        setState({ sessions: [...sessions, fresh], activeSessionId: fresh.id });
      }
      return;
    }
    setState({ sessions, activeSessionId });
  },
  /** 删除会话=移除会话及其历史投影；用于用户确认删除已持久化会话记录。 */
  deleteSession(id: string) {
    const sessions = state.sessions.filter((s) => s.id !== id);
    if (!sessions.length) {
      const fresh = blankSession();
      setState({ sessions: [fresh], activeSessionId: fresh.id, sending: false });
      return;
    }
    const activeSessionId =
      state.activeSessionId === id
        ? sessions.find((s) => !s.archived)?.id ?? sessions[0].id
        : state.activeSessionId;
    setState({ sessions, activeSessionId });
  },
  /** 设定活动会话模式（编辑/构建，TERM-AG-006/007）。 */
  setMode(mode: AgMode) {
    mapSession(state.activeSessionId, (s) => ({ ...s, mode }));
  },

  // ── 回合门控（SM-AG-01 投影，全局） ───────────────────────────────
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

  // ── 消息 / 工具卡（按会话定位：流式期间切标签不串会话） ───────────
  pushMessage(sessionId: string, msg: ChatMessage) {
    mapSession(sessionId, (s) => ({
      ...s,
      messages: [...s.messages, msg],
      // 首条用户消息作为标签标题（截前 12 字）。
      title:
        s.title === "新对话" && msg.role === "user" && msg.content.trim()
          ? msg.content.trim().slice(0, 12)
          : s.title,
    }));
  },
  /** 追加流式分片到指定会话的指定消息。 */
  appendToMessage(sessionId: string, id: string, chunk: string) {
    mapSession(sessionId, (s) => ({
      ...s,
      messages: s.messages.map((m) => (m.id === id ? { ...m, content: m.content + chunk } : m)),
    }));
  },
  /** 标记消息流式结束。 */
  finishMessage(sessionId: string, id: string) {
    mapSession(sessionId, (s) => ({
      ...s,
      messages: s.messages.map((m) => (m.id === id ? { ...m, streaming: false } : m)),
    }));
  },
  /** 标记某个代理回合在当前会话内的持久化状态投影。 */
  setTurnStatus(sessionId: string, turnId: string, status: AgentTurnStatus) {
    mapSession(sessionId, (s) => ({
      ...s,
      messages: s.messages.map((m) => (m.turnId === turnId ? { ...m, turnStatus: status } : m)),
    }));
  },
  /** 新增一条工具调用卡（executing）；返回卡 id（PI-AG-08D，投影 ToolRequest）。 */
  addToolCall(sessionId: string, name: string, detail?: string, meta: Pick<ToolCallView, "turnId" | "anchorMessageId"> = {}): string {
    const id = `tc-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
    mapSession(sessionId, (s) => ({
      ...s,
      toolCalls: [...s.toolCalls, { id, name, state: "executing", detail, ...meta }],
    }));
    return id;
  },
  /** 将该 name 最早的 executing 卡置 done/failed（事件无 call id，按 name+executing 先到先配）。 */
  resolveToolCall(sessionId: string, name: string, ok: boolean, detail?: string, turnId?: string) {
    let matched = false;
    mapSession(sessionId, (s) => ({
      ...s,
      toolCalls: s.toolCalls.map((t) => {
        if (!matched && t.name === name && t.state === "executing" && (!turnId || t.turnId === turnId)) {
          matched = true;
          return { ...t, state: ok ? "done" : "failed", detail: detail ?? t.detail };
        }
        return t;
      }),
    }));
  },
  /** 清空该会话工具卡（新回合起）。 */
  clearToolCalls(sessionId: string) {
    mapSession(sessionId, (s) => ({ ...s, toolCalls: [] }));
  },
};

export function useAgStore(): AgState {
  return useSyncExternalStore(agStore.subscribe, agStore.getSnapshot);
}
