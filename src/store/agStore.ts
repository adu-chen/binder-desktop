import { useSyncExternalStore } from "react";
import type { JSONContent } from "@tiptap/core";
import type { MessageContentPart, ReferenceTokenAttrs } from "../services/referencePayload";

/**
 * AG 会话状态投影层（AG-TURN · SM-AG-01 · X-FORBID-007）。
 * messages 是 AgentTurn（TERM-AG-001）会话视图；toolCalls 是 ToolCall（TERM-AG-002）状态投影。
 * 回合权威态归后端 SM-AG-01（sending 门控投影），前端只呈现与转达意图，不另立可漂移的并行回合态。
 *
 * 多标签会话（PI-AG-UI-01）：会话分组是**呈现层组织**（运行期内存态，非后端业务态）；
 * 关闭标签=归档（本次运行内经「历史会话」可重开），跨重启持久化随 BR-AG-PERSIST-001
 * （agent_turns 接线批）落地，本结构按可持久化形状设计。
 */

/** 会话持久化兼容字段：Agent 1.0 运行时只开放 edit；build 仅保留后续 Profile 的加法式数据形状，不授予入口。 */
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
  /** 回合内消息片段（TERM-AG-011 · BR-AG-UI-001）：assistant 消息按事件到达顺序交错排列
   *  text/tool/diff 片段；缺省（legacy 历史/降级）时渲染层沉底布局，不丢失数据。 */
  segments?: TurnSegment[];
}

/** 回合内消息片段（TERM-AG-011）：text=流式累积正文 / tool=引用 ToolCall 卡 / diff=引用 DiffCard。 */
export type TurnSegment =
  | { kind: "text"; text: string }
  | { kind: "tool"; toolCallId: string }
  | { kind: "diff"; diffId: string };

export type AgentTurnStatus = "completed" | "error" | "cancelled" | "gate_rejected" | "draft" | "clarifying";

export interface AgentComposerDraft {
  doc: JSONContent;
  tokens: ReferenceTokenAttrs[];
  prompt: string;
}

export interface WorkspaceReferenceMutation {
  kind: "rename_resource" | "move_resource" | "delete_resource" | "discard_created_resource" | string;
  source_rel_path?: string | null;
  target_rel_path?: string | null;
  resource_kind: "file" | "folder";
}

function resourcePathMatches(candidate: string, source: string, resourceKind: string): boolean {
  return candidate === source || (resourceKind === "folder" && candidate.startsWith(`${source}/`));
}

function rewriteReferenceToken(
  token: ReferenceTokenAttrs,
  mutations: WorkspaceReferenceMutation[],
): ReferenceTokenAttrs | null {
  if (token.payload.kind !== "doc" && token.payload.kind !== "selection") return token;
  let next = token;
  for (const mutation of mutations) {
    const source = mutation.source_rel_path ?? "";
    const payload = next.payload;
    if (payload.kind !== "doc" && payload.kind !== "selection") return next;
    if (!source || !resourcePathMatches(payload.relPath, source, mutation.resource_kind)) continue;
    if (mutation.kind === "delete_resource" || mutation.kind === "discard_created_resource") return null;
    if ((mutation.kind === "rename_resource" || mutation.kind === "move_resource") && mutation.target_rel_path) {
      const relPath = mutation.resource_kind === "folder"
        ? `${mutation.target_rel_path}${payload.relPath.slice(source.length)}`
        : mutation.target_rel_path;
      const label = relPath.split("/").filter(Boolean).pop() ?? relPath;
      next = {
        ...next,
        label,
        payload: { ...payload, relPath, label },
      } as ReferenceTokenAttrs;
    }
  }
  return next;
}

function rewriteReferenceDocument(
  node: JSONContent,
  tokenById: Map<string, ReferenceTokenAttrs>,
): JSONContent | null {
  if (node.type === "referenceToken") {
    const refId = typeof node.attrs?.refId === "string" ? node.attrs.refId : "";
    const token = tokenById.get(refId);
    return token ? { ...node, attrs: token } : null;
  }
  if (!node.content) return node;
  return {
    ...node,
    content: node.content
      .map((child) => rewriteReferenceDocument(child, tokenById))
      .filter((child): child is JSONContent => Boolean(child)),
  };
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
  | "AwaitingClarification"
  | "TurnError"
  | "Completed"
  | "Cancelled"
  | "Denied";

/** 回合“进行中”的活动态集合：sending（输入门控）= 处于这些态之一。其余（Idle/各终态/TurnError）→ 非进行中。 */
const ACTIVE_TURN_STATES: ReadonlySet<RustTurnState> = new Set([
  "Assembling",
  "AwaitingGate",
  "Streaming",
  "HandingOff",
  // AwaitingClarification 不在其中：澄清是等待用户输入的非终态（BR-SYS-STATE-007），
  // 不阻塞输入门控——用户可回复并受控重入同一 AgentTurn，或直接取消。
]);

/** 任务步骤态（TERM-AG-016 · SM-AG-04 steps）。 */
export type TaskStepViewStatus =
  | "pending"
  | "running"
  | "waiting_confirm"
  | "ok"
  | "failed"
  | "cancelled";

/** 受限任务实例步骤（TERM-AG-016）。 */
export interface TaskStepView {
  stepId: string;
  kind: string; // tool / propose / read / confirm
  label: string;
  status: TaskStepViewStatus;
}

/** 受限任务实例状态（TERM-AG-015 · SM-AG-04）。 */
export type TaskRunViewStatus =
  | "planned"
  | "executing"
  | "waiting_confirm"
  | "waiting_clarification"
  | "completed"
  | "failed"
  | "cancelled"
  | "interrupted";

/** 受限任务实例（TERM-AG-015 · SM-AG-04）：会话内任务进度/步骤/确认点（只读呈现）。 */
export interface TaskRunView {
  taskRunId: string;
  status: TaskRunViewStatus;
  intentSummary?: string;
  steps: TaskStepView[];
  createdAt: number;
  updatedAt: number;
}

/** 回合实时当前动作（AG-1.0-TASKRUN-REWORK-IMP-UI-01 · step_progress，BR-SYS-STATE-008）。 */
export interface TurnCurrentAction {
  action: "reading" | "tool" | "generating" | "proposing" | "deciding" | "previewing";
  label: string;
  detail?: string;
}

/** 任务级裁决请求（TERM-AG-017）：待用户裁决（confirm/cancel → ag.resolve_decision）。 */
export interface PendingDecisionView {
  requestId: string;
  kind: string;
  payload?: unknown;
}

/** 任务计划步骤（TERM-AG-018 · TaskPlan）。 */
export interface TaskPlanStepView {
  stepId: string;
  seq: number;
  kind: string;
  label: string;
  deps: string[];
  requiredGrant?: string;
  confirmPoint: boolean;
  status: string;
}

/** 任务计划（TERM-AG-018 · TaskPlan）：可展示操作计划（步骤/依赖/游标/确认点）。 */
export interface TaskPlanView {
  planId: string;
  taskRunId: string;
  steps: TaskPlanStepView[];
  cursor: number;
}

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
  /** 输入框草稿是标签级运行时状态，不随历史元数据持久化。 */
  composerDraft?: AgentComposerDraft;
  /** 会话受限任务实例（TERM-AG-015）：最近 TaskRun 倒序，供会话摘要卡呈现（只读）。 */
  taskRuns?: TaskRunView[];
  /** 任务计划（TERM-AG-018）：会话最近 run 的可展示操作计划（步骤/依赖/确认点）。 */
  taskPlans?: TaskPlanView[];
}

interface AgState {
  sessions: AgSession[];
  activeSessionId: string;
  /** 回合级全局门控（后端 SM-AG-01 单回合投影；一次仅一个活动回合）。 */
  sending: boolean;
  activeTurn: { sessionId: string; turnId: string } | null;
  /** 回合实时当前动作（step_progress；null=无活动回合动作）。 */
  currentAction: TurnCurrentAction | null;
  /** 待用户裁决的任务级请求（DecisionRequest；null=无待裁决项）。 */
  pendingDecision: PendingDecisionView | null;
}

const sid = () => `s-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;

/** 把流式分片追加到消息片段序列的最后一个 text 段；最后一个段非 text 或无序列时新建 text 段。 */
function appendTextSegment(segments: TurnSegment[] | undefined, chunk: string): TurnSegment[] {
  const list = segments ?? [];
  const last = list[list.length - 1];
  if (last?.kind === "text") {
    return [...list.slice(0, -1), { ...last, text: last.text + chunk }];
  }
  return [...list, { kind: "text", text: chunk }];
}

export function emptyComposerDraft(): AgentComposerDraft {
  return {
    doc: { type: "doc", content: [{ type: "paragraph" }] },
    tokens: [],
    prompt: "",
  };
}

function withComposerDraft(s: AgSession): AgSession {
  return { ...s, composerDraft: s.composerDraft ?? emptyComposerDraft() };
}

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
    composerDraft: emptyComposerDraft(),
  };
}

const initial = blankSession();
let state: AgState = {
  sessions: [initial],
  activeSessionId: initial.id,
  sending: false,
  activeTurn: null,
  currentAction: null,
  pendingDecision: null,
};
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
    setState({
      sessions: [fresh],
      activeSessionId: fresh.id,
      sending: false,
      activeTurn: null,
      currentAction: null,
      pendingDecision: null,
    });
  },

  hydrateSessions(sessions: AgSession[], activeSessionId?: string) {
    const next = sessions.length ? sessions.map(withComposerDraft) : [blankSession()];
    const active =
      (activeSessionId && next.some((s) => s.id === activeSessionId) && activeSessionId) ||
      next.find((s) => !s.archived)?.id ||
      next[0].id;
    setState({
      sessions: next,
      activeSessionId: active,
      sending: false,
      activeTurn: null,
      currentAction: null,
      pendingDecision: null,
    });
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
  /** AG-1.0-IMP-TASKRUN-UI-01：投影会话受限任务实例（只读，不伪造进度，BR-SYS-STATE-008）。 */
  hydrateSessionTaskRuns(sessionId: string, taskRuns: TaskRunView[]) {
    mapSession(sessionId, (s) => ({ ...s, taskRuns }));
  },
  /** AG-1.0-TASKRUN-REWORK-CONTINUE-01：投影会话任务计划（TERM-AG-018，可展示操作计划）。 */
  hydrateSessionPlans(sessionId: string, taskPlans: TaskPlanView[]) {
    mapSession(sessionId, (s) => ({ ...s, taskPlans }));
  },
  /** AG-1.0-TASKRUN-REWORK-IMP-UI-01：回合实时当前动作（step_progress 驱动）。 */
  setCurrentAction(action: TurnCurrentAction | null) {
    setState({ currentAction: action });
  },
  /** AG-1.0-TASKRUN-REWORK-IMP-UI-01：待裁决任务级请求（DecisionRequest proposed）。 */
  setPendingDecision(decision: PendingDecisionView | null) {
    setState({ pendingDecision: decision });
  },
  /** 设定活动会话模式（编辑/代理构建，TERM-AG-006/007）。 */
  setMode(mode: AgMode) {
    mapSession(state.activeSessionId, (s) => ({ ...s, mode }));
  },
  /** 保存指定会话的输入框草稿；仅运行时使用，不触发历史持久化。 */
  setComposerDraft(sessionId: string, draft: AgentComposerDraft) {
    mapSession(sessionId, (s) => ({ ...s, composerDraft: draft }));
  },
  clearComposerDraft(sessionId: string) {
    mapSession(sessionId, (s) => ({ ...s, composerDraft: emptyComposerDraft() }));
  },
  /** WorkspaceOperation 成功回执驱动引用同步：重命名/移动重绑路径，删除移除失效引用。 */
  reconcileResourceReferences(mutations: WorkspaceReferenceMutation[]) {
    if (!mutations.length) return;
    setState({
      sessions: state.sessions.map((session) => {
        const draft = session.composerDraft;
        if (!draft?.tokens.length) return session;
        const tokens = draft.tokens
          .map((token) => rewriteReferenceToken(token, mutations))
          .filter((token): token is ReferenceTokenAttrs => Boolean(token));
        const tokenById = new Map(tokens.map((token) => [token.refId, token]));
        const doc = rewriteReferenceDocument(draft.doc, tokenById) ?? emptyComposerDraft().doc;
        return { ...session, composerDraft: { ...draft, doc, tokens } };
      }),
    });
  },

  // ── 回合门控（SM-AG-01 投影，全局） ───────────────────────────────
  setSending(sending: boolean) {
    setState({ sending });
  },
  setActiveTurn(sessionId: string, turnId: string) {
    setState({ activeTurn: { sessionId, turnId }, sending: true });
  },
  clearActiveTurn(turnId?: string) {
    if (turnId && state.activeTurn?.turnId !== turnId) return;
    setState({ activeTurn: null, sending: false });
  },
  /**
   * 投影后端 SM-AG-01 回合机转移（A-CORE-X-L-04 Task 2-E · X-FORBID-007）：由 smListeners 监听
   * `sm:SM-AG-01:state_changed` 调用。sending（输入门控）= 回合处于活动态（Assembling/AwaitingGate/
   * Streaming/HandingOff）；到达任一终态或 TurnError 即释放。消息流/工具卡仍由 Channel 投影（互不替代）。
   */
  applyTurnState(_turnId: string, turnState: RustTurnState) {
    const sending = ACTIVE_TURN_STATES.has(turnState);
    setState({
      sending,
      activeTurn: sending || state.activeTurn?.turnId !== _turnId ? state.activeTurn : null,
    });
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
  /**
   * 将用户补充说明并入原 AgentTurn 的 user message，并恢复原 assistant message 的流式投影。
   * 只更新同 turnId 现有消息，不创建第二个 AgentTurn 或伪造新的 assistant 身份。
   */
  resumeClarifyingTurn(
    sessionId: string,
    turnId: string,
    clarification: string,
    clarificationParts?: MessageContentPart[],
  ) {
    const prefix = "\n\n补充说明：";
    mapSession(sessionId, (session) => ({
      ...session,
      messages: session.messages.map((message) => {
        if (message.turnId !== turnId) return message;
        if (message.role === "assistant") {
          return { ...message, streaming: true, turnStatus: "draft" };
        }
        const content = `${message.content}${prefix}${clarification}`;
        let parts = message.parts;
        if (clarificationParts?.length) {
          parts = message.parts?.length
            ? [...message.parts, { type: "text", text: prefix }, ...clarificationParts]
            : [{ type: "text", text: `${message.content}${prefix}` }, ...clarificationParts];
        } else if (message.parts?.length) {
          parts = [...message.parts, { type: "text", text: `${prefix}${clarification}` }];
        }
        return { ...message, content, parts, turnStatus: "draft" };
      }),
    }));
  },
  /** 追加流式分片到指定会话的指定消息；同步追加到消息片段序列的最后一个 text 段（无则创建）。 */
  appendToMessage(sessionId: string, id: string, chunk: string) {
    mapSession(sessionId, (s) => ({
      ...s,
      messages: s.messages.map((m) =>
        m.id === id
          ? {
              ...m,
              content: m.content + chunk,
              segments: appendTextSegment(m.segments, chunk),
            }
          : m,
      ),
    }));
  },
  /** 关闭当前 text 段并追加 tool 段（TERM-AG-011）：工具调用在正文时序位置内嵌。 */
  addToolSegment(sessionId: string, id: string, toolCallId: string) {
    mapSession(sessionId, (s) => ({
      ...s,
      messages: s.messages.map((m) =>
        m.id === id ? { ...m, segments: [...(m.segments ?? []), { kind: "tool", toolCallId }] } : m,
      ),
    }));
  },
  /** 追加 diff 段（TERM-AG-011）：DiffCard 紧跟其产出的工具调用/正文时序位置。 */
  addDiffSegment(sessionId: string, id: string, diffId: string) {
    mapSession(sessionId, (s) => ({
      ...s,
      messages: s.messages.map((m) =>
        m.id === id ? { ...m, segments: [...(m.segments ?? []), { kind: "diff", diffId }] } : m,
      ),
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
  /** 按 toolCallId 精确置终态（AG-MSG-RENDER-01 · BR-AG-UI-001）：修复同轮多个同名工具的配对错乱。 */
  resolveToolCallById(sessionId: string, toolCallId: string, ok: boolean, detail?: string) {
    mapSession(sessionId, (s) => ({
      ...s,
      toolCalls: s.toolCalls.map((t) =>
        t.id === toolCallId ? { ...t, state: ok ? "done" : "failed", detail: detail ?? t.detail } : t,
      ),
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
