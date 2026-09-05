import { Channel, invoke } from "@tauri-apps/api/core";
import { activeSession, agStore, type AgentTurnStatus, type AgMode, type AgSession, type ChatMessage, type ProjectReportView, type TaskPlanView, type TaskRunView, type TaskRunViewStatus, type TaskStepViewStatus, type ToolCallView, type TurnSegment } from "../store/agStore";
import { edStore, type ContentFormat, type EditFormat, type OpenDocSnapshot } from "../store/edStore";
import { deStore, type DiffCardState, type DiffEffectStatus, type DiffView } from "../store/deStore";
import { listActiveTables, parseTableEditPayload, summarizeTableOperation, type TableEditPayload } from "./edTableOps";
import {
  parseImageDiffSnapshotProjection,
  parseImageEditPayload,
  projectDocumentImages,
  type ImageEditPayload,
  type ImageOperation,
} from "./edImageOps";
import {
  ensureDocumentMediaDisplayBase,
  projectDocumentMediaDisplaySrc,
} from "./docService";
import { closeStore } from "../store/closeStore";
import { tabsStore } from "../store/tabsStore";
import { flushOpenEditorSessionSnapshots, openDocument } from "./edService";
import { parseMarkdownForEditing } from "../components/ed/markdown/markdownCodec";
import { workspaceShell } from "../store/workspaceShell";

/** 审计修复（2026-07-26 · D10）：HTML sanitize — 保留内联安全标签，剥离 script/iframe/on* 等。
 *  用于 replace_block 的 new_html 在进入后端前清洗。 */
const SAFE_INLINE_TAGS = /<\/?(strong|em|u|s|sub|sup|code|a|span|br)\b[^>]*\/?>/i;
const DANGEROUS_ATTR = /\s+on\w+\s*=\s*["'][^"']*["']/gi;
function sanitizeInlineHtml(html: string): string {
  // 1. 剥离所有标签
  let result = html.replace(/<[^>]*>/g, (match) => {
    // 保留安全内联标签
    const cleaned = match.replace(DANGEROUS_ATTR, "");
    return SAFE_INLINE_TAGS.test(match) ? cleaned : "";
  });
  // 2. 还原 HTML 实体（允许模型输出已转义的内容）
  return result;
}
import { getActiveBlockSnapshotByIndex, getActiveDocumentContent, getActiveEditor, getActiveEditorBinding } from "./edLocate";
import { loadDocs } from "./wsService";
import { applyOpenSessionDiff, expireStalePendingForPath, loadDiffDetails, loadPersistedDiffs, type DiffDetailDto } from "./deService";
import type { MessageContentPart, ReferencePayload } from "./referencePayload";
import { parseStyleOps, type StyleOp } from "./edStyleOps";
import {
  parseStructuredBlockEditPayload,
  projectAgentEditableBlocks,
} from "./edBlockOps";
import type { DiffProjectionDto } from "../types/diffProjection";

/**
 * AG 模块 Service 层。
 * PI-AG-WIRE-01：经独立 async 命令 `ag_start_turn` + Tauri `Channel<AgTurnEvent>` 接**真实流式**
 * （替换桩 dispatch 回复 + 前端逐字模拟）。门禁前置（X-SEQ-001）由后端 SM-AG-01 ensure_gate_passed
 * 保证；模型经本地 dev GW（cloud/dev-gateway），客户端不持 provider 密钥（X-FORBID-001）。
 */

/** 与后端 `ipc::AgTurnEvent` envelope + payload 逐字段对齐（A-AG-I-P-01 §2.1）。 */
interface AgTurnEventEnvelope {
  eventId: string;
  turnId: string;
  seq: number;
  occurredAt: number;
  payloadVersion: number;
}

export type AgTurnEvent = AgTurnEventEnvelope & (
  | { type: "chunk"; text: string }
  | { type: "intent_updated"; revision: number; projection: unknown }
  | { type: "route_updated"; revision: number; summary: string; steps: unknown[] }
  | { type: "tool_request"; toolCallId: string; toolName: string; params: unknown }
  | { type: "tool_result"; toolCallId: string; toolName: string; result: string; success: boolean; evidenceIds: string[] }
  | { type: "project_report"; report: ProjectReportView }
  | { type: "diff_proposed"; diffId: string; proposalBatchId: string; groupIndex: number; groupCount: number; relPath: string; taskRunId?: string }
  | { type: "step_progress"; action: string; label: string; detail?: string }
  | { type: "decision_request_proposed"; requestId: string; kind: string; payload: unknown }
  | { type: "decision_request_resolved"; requestId: string; outcome: string }
  | { type: "clarification_required"; text: string }
  | { type: "done"; stopReason: string }
  | { type: "error"; code: string; message: string; retriable: boolean }
);

export const AGENT_EDIT_RUNTIME_PROFILE = "agent-edit-1.0" as const;
export const AGENT_CONTEXT_FACTS_VERSION = "agent-context-facts-v1" as const;

export interface AgentContextDocumentFact {
  relPath: string;
  source: OpenDocSnapshot["source"];
  editFormat: OpenDocSnapshot["editFormat"];
  contentFormat: OpenDocSnapshot["contentFormat"];
  dirty: boolean;
  baseline: string | null;
  sessionVersion: number;
  contentHash: string;
}

export interface AgentContextFacts {
  version: typeof AGENT_CONTEXT_FACTS_VERSION;
  workspaceId: string;
  turnId: string;
  activeDoc: AgentContextDocumentFact | null;
  openDocs: AgentContextDocumentFact[];
  selections: { relPath: string; blockId: string; start?: number; end?: number }[];
  proposalFacts: {
    diffId: string;
    sessionId: string;
    relPath: string;
    state: DiffCardState;
    effectStatus?: DiffEffectStatus;
    saved: boolean;
  }[];
}

/**
 * @GOV
 * codes:    BR-AG-DATA-008-DATA-AG-AG-TURN-041, BR-ED-DATA-008-DATA-AG-AG-TURN-041, BR-SYS-DATA-008-DATA-AG-AG-TURN-041
 * type:     DATA
 * chain:    AG-TURN
 * rules:    BR-AG-DATA-008, BR-ED-DATA-008, BR-SYS-DATA-008
 * term_ref: TERM-AG-020
 * boundary: in=ActiveWorkspaceOpenDocSnapshotsExplicitSelectionRefsAndProposalProjection | out=AgentContextFactsV1(noDocumentBodyNoAuthorization) | consumer=ag_start_turn | does_not=copyDocumentBodyOrExpandCapabilityGrantOrExplicitEditTargetSet
 */
export function buildAgentContextFacts(
  turnId: string,
  sessionId: string,
  openDocs: OpenDocSnapshot[],
  references: ReferencePayload[],
  blockRefs: ClarifyingTurnIdentity["blockRefs"] = [],
): AgentContextFacts {
  const workspaceId = workspaceShell.getSnapshot().activeWorkspace?.workspaceId;
  if (!workspaceId) throw new Error("AG_CONTEXT_FACTS_WORKSPACE_REQUIRED");
  const projectDocument = (snapshot: OpenDocSnapshot): AgentContextDocumentFact => ({
    relPath: snapshot.relPath,
    source: snapshot.source,
    editFormat: snapshot.editFormat,
    contentFormat: snapshot.contentFormat,
    dirty: snapshot.dirty,
    baseline: snapshot.baseline == null ? null : String(snapshot.baseline),
    sessionVersion: snapshot.sessionVersion,
    contentHash: snapshot.contentHash,
  });
  const projectedOpenDocs = openDocs.map(projectDocument);
  const activeDoc = projectedOpenDocs.find((doc) =>
    doc.source === "active_editor" || doc.source === "active_editor_session"
  ) ?? null;
  const referenceSelections = references.flatMap((reference) => reference.kind === "selection"
    ? [{
        relPath: reference.relPath,
        blockId: reference.blockId,
        ...(reference.start !== undefined ? { start: reference.start } : {}),
        ...(reference.end !== undefined ? { end: reference.end } : {}),
      }]
    : []);
  const selections = mergeLegacyBlockRefs(referenceSelections.map((selection) => ({
    ...selection,
    text: "",
  })), blockRefs).map(({ text: _text, ...selection }) => selection);
  const proposalFacts = deStore.getSnapshot().diffs.flatMap((diff) =>
    diff.relPath && diff.sessionId === sessionId
      ? [{
          diffId: diff.diffId,
          sessionId,
          relPath: diff.relPath,
          state: diff.state,
          ...(diff.effectStatus ? { effectStatus: diff.effectStatus } : {}),
          saved: diff.draftSaved === true || diff.effectStatus === "saved",
        }]
      : []
  );
  return {
    version: AGENT_CONTEXT_FACTS_VERSION,
    workspaceId,
    turnId,
    activeDoc,
    openDocs: projectedOpenDocs,
    selections,
    proposalFacts,
  };
}

let seq = 0;
interface ClarifyingTurnIdentity {
  sessionId: string;
  turnId: string;
  assistantMessageId: string;
  prompt: string;
  references: ReferencePayload[];
  mentions: string[];
  blockRefs: { relPath: string; blockId: string; text: string; start?: number; end?: number }[];
  /** 已并入 UI 但尚未成功越过后端入口的补充说明；只在本地前置失败时累积。 */
  pendingInput?: string;
}
/** 按 session 保留可受控重入的非终态 AgentTurn；不得按“最近回合”跨 session 猜测。 */
const clarifyingTurns = new Map<string, ClarifyingTurnIdentity>();
interface DecisionTurnIdentity extends ClarifyingTurnIdentity {
  requestId: string;
}
/** 按 DecisionRequest 保留 AwaitingDecision 的原回合身份；裁决只恢复该回合，不创建替代回合。 */
const decisionTurns = new Map<string, DecisionTurnIdentity>();
const uid = () => `m-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;

function mergeReferencePayloads(...groups: ReferencePayload[][]): ReferencePayload[] {
  const seen = new Set<string>();
  const merged: ReferencePayload[] = [];
  for (const reference of groups.flat()) {
    const key = JSON.stringify(reference);
    if (seen.has(key)) continue;
    seen.add(key);
    merged.push(reference);
  }
  return merged;
}

function mergeLegacyBlockRefs(
  ...groups: ClarifyingTurnIdentity["blockRefs"][]
): ClarifyingTurnIdentity["blockRefs"] {
  const seen = new Set<string>();
  return groups.flat().filter((reference) => {
    const key = JSON.stringify(reference);
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}

function projectionFromDetail(
  diffId: string,
  relPath: string,
  detail: DiffDetailDto | undefined,
): DiffProjectionDto | null {
  const p = detail?.projection as Partial<DiffProjectionDto> | undefined;
  if (
    !p ||
    (p.patch_kind !== "replace_block" &&
      p.patch_kind !== "block_ops" &&
      p.patch_kind !== "replace_document" &&
      p.patch_kind !== "style_ops" &&
      p.patch_kind !== "table_ops" &&
      p.patch_kind !== "image_ops")
  ) {
    return null;
  }
  return {
    ...p,
    diff_id: diffId,
    rel_path: detail?.target.rel_path ?? relPath,
    patch_kind: p.patch_kind,
    before_text: p.before_text ?? "",
    after_text: p.after_text ?? "",
    location_label: p.location_label ?? "未知位置",
    draft_mode: p.draft_mode ?? "current_editor",
    snapshot_source: p.snapshot_source ?? "active_editor",
  };
}

function summarizeStyleOp(op: StyleOp): string {
  if (op.op === "set_block_attr") return `${op.blockId} 设置块样式`;
  if (op.op === "unset_mark") return `${op.blockId} 移除 ${op.markType} [${op.start},${op.end}]`;
  return `${op.blockId} 设置 ${op.mark.type} [${op.start},${op.end}]`;
}

function parseTableOps(value: unknown): TableEditPayload | undefined {
  return parseTableEditPayload(value);
}

function parseImageOps(value: unknown, relPath?: string | null): ImageEditPayload | undefined {
  const payload = parseImageEditPayload(value);
  if (!payload) return undefined;
  if (!relPath) return payload;
  const operations: ImageOperation[] = [];
  for (const operation of payload.operations) {
    if (operation.op !== "insert_image") {
      operations.push(operation);
      continue;
    }
    const displaySrc = projectDocumentMediaDisplaySrc(relPath, operation.mediaRef);
    if (!displaySrc) return undefined;
    operations.push({ ...operation, displaySrc });
  }
  return { ...payload, operations };
}

function summarizeImageOperation(operation: ImageOperation): string {
  if (operation.op === "insert_image") {
    return `插入 ${operation.sourcePath} · ${operation.position === "before" ? "锚点前" : "锚点后"}`;
  }
  if (operation.op === "delete_image") return `删除图片 ${operation.imageId}`;
  if (operation.op === "resize_image") return `缩放图片 ${operation.imageId} · ${operation.width}px`;
  if (operation.op === "move_image") {
    return `移动图片 ${operation.imageId} · ${operation.direction === "up" ? "向上" : "向下"}`;
  }
  return `图片对齐 ${operation.imageId} · ${operation.alignment}`;
}

/** 文档块（供块级 @mention 选择非当前文档的块，PI-AG-09G）。 */
export interface DocBlock {
  block_id: string | null;
  block_index: number;
  block_type?: string | null;
  text: string;
  /** 样式投影（PI-ED-11 · A-AG-I-P-01 §4.1）：mark 段（range 在块内纯文本平面，与字符锚同基准）。
   *  text 块为空数组；schema 单一来源 A-ED-M-T-01 §5.1。 */
  marks?: { type: string; range: [number, number]; attrs?: Record<string, unknown> }[];
  /** 块级呈现属性投影（textAlign 等）。 */
  block_attrs?: Record<string, unknown>;
}

let dseq = 0;
const ACTIVE_SNAPSHOT_WAIT_MS = 800;
const ACTIVE_SNAPSHOT_POLL_MS = 25;

/** 经统一 dispatch 信封调命令（非流式），PI-AG-09G。 */
async function dispatchCall<T>(command: string, params: Record<string, unknown>): Promise<T> {
  const request = { command, correlation_id: `agd-${Date.now()}-${(dseq += 1)}`, params };
  const resp = await invoke<{ result?: T; error?: { code: string; message: string } }>("dispatch", { request });
  if (resp.error) throw resp.error;
  if (resp.result === undefined) throw { code: "IPC_EMPTY", message: `${command} 无返回` };
  return resp.result;
}

/** 列指定文档的块（ag.list_blocks），供块级引用选择。 */
export async function listDocBlocks(relPath: string): Promise<DocBlock[]> {
  const openDocSnapshots = await collectOpenDocSnapshotsReady();
  const r = await dispatchCall<{ blocks: DocBlock[] }>("ag.list_blocks", { relPath, openDocSnapshots });
  return r.blocks;
}

function snapshotError(code: string, message: string) {
  return { code, message, retriable: true };
}

function isActiveSessionLoading(e: unknown): boolean {
  const code = (e as { code?: string })?.code;
  return code === "AG_ACTIVE_SESSION_LOADING";
}

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => globalThis.setTimeout(resolve, ms));
}

function bindingMatches(
  binding: ReturnType<typeof getActiveEditorBinding>,
  relPath: string | null,
  loadSeq: number,
  editFormat: EditFormat,
  contentFormat: ContentFormat,
): boolean {
  return Boolean(
    relPath &&
    binding &&
    binding.relPath === relPath &&
    binding.loadSeq === loadSeq &&
    binding.editFormat === editFormat &&
    (binding.contentFormat ?? "text") === contentFormat,
  );
}

function flushBoundActiveEditor(ed = edStore.getSnapshot()): void {
  const binding = getActiveEditorBinding();
  if (!binding || bindingMatches(binding, ed.openPath, ed.loadSeq, ed.editFormat, ed.contentFormat)) return;
  const boundContent = getActiveDocumentContent(binding.editFormat, binding.contentFormat);
  if (boundContent !== null) edStore.updateContent(binding.relPath, boundContent);
}

interface ActiveDocTarget {
  relPath: string | null;
  loadSeq: number;
  editFormat: EditFormat;
  contentFormat: ContentFormat;
}

export type AgentOpenDocSnapshot = OpenDocSnapshot & {
  tableSnapshots?: ReturnType<typeof listActiveTables>;
  imageSnapshots?: ReturnType<typeof projectDocumentImages>;
};

function attachActiveStructuredSnapshots(snapshots: OpenDocSnapshot[], activePath: string | null): AgentOpenDocSnapshot[] {
  const tables = listActiveTables();
  const images = projectDocumentImages(getActiveEditor());
  const blocks = projectAgentEditableBlocks(getActiveEditor());
  return snapshots.map((snapshot) => (
    snapshot.relPath === activePath && snapshot.source === "active_editor" && snapshot.contentFormat === "html"
      ? {
          ...snapshot,
          blockSnapshots: blocks,
          tableSnapshots: tables,
          imageSnapshots: images,
        }
      : snapshot
  ));
}

/**
 * @GOV
 * codes:    BR-AG-DATA-001-EFFECT-AG-AG-TURN-005
 * type:     EFFECT
 * chain:    AG-TURN
 * rules:    BR-AG-DATA-001
 * term_ref: TERM-DOC-003, TERM-ED-008
 * boundary: in=CurrentOpenDocumentLogicalStateAndEditorBinding | out=AgentOpenDocSnapshotList(withSameVersionBlockTableImageSnapshots) | effect=memory | delegate=projectAgentEditableBlocks,projectTables,projectDocumentImages | caller_context=agentTurnSnapshotAssembly | does_not=fall_back_to_DiskState_or_reparse_HTML_for_open_document
 */
export function collectOpenDocSnapshots(target?: ActiveDocTarget): AgentOpenDocSnapshot[] {
  const ed = edStore.getSnapshot();
  const activeTarget = target ?? {
    relPath: ed.openPath,
    loadSeq: ed.loadSeq,
    editFormat: ed.editFormat,
    contentFormat: ed.contentFormat,
  };
  if (!activeTarget.relPath) return edStore.getOpenSnapshots("active_editor_session");

  flushBoundActiveEditor(ed);

  let latest = edStore.getSnapshot();
  const binding = getActiveEditorBinding();
  if (bindingMatches(binding, activeTarget.relPath, activeTarget.loadSeq, activeTarget.editFormat, activeTarget.contentFormat)) {
    const activeContent = getActiveDocumentContent(activeTarget.editFormat, activeTarget.contentFormat);
    if (activeContent === null) {
      throw snapshotError("AG_ACTIVE_SNAPSHOT_REQUIRED", "当前打开文档无法序列化，已阻断代理回合");
    }
    edStore.updateContent(activeTarget.relPath, activeContent);
    return attachActiveStructuredSnapshots(edStore.getOpenSnapshots(), activeTarget.relPath);
  }

  latest = edStore.getSnapshot();
  const activeSession = latest.sessions[activeTarget.relPath];
  if (!activeSession || activeSession.content === null) {
    throw snapshotError("AG_ACTIVE_SESSION_LOADING", "当前文档 LogicalState 正在载入，请稍候");
  }
  if (activeTarget.contentFormat === "html") {
    throw snapshotError(
      "AG_ACTIVE_SESSION_LOADING",
      "当前富文档正在建立 ProseMirror 结构化快照，请稍候",
    );
  }

  // 切换/启动恢复期间，目标文档的 LogicalState 可能已经在 EditSession 中就绪，
  // 但 TipTap 显示绑定尚未完成。AG 读取逻辑态，不以显示态渲染完成作为前提。
  return edStore.getOpenSnapshots("active_editor_session");
}

async function collectOpenDocSnapshotsReady(target?: ActiveDocTarget): Promise<OpenDocSnapshot[]> {
  const startedAt = Date.now();
  for (;;) {
    try {
      return collectOpenDocSnapshots(target);
    } catch (e) {
      if (!isActiveSessionLoading(e) || Date.now() - startedAt >= ACTIVE_SNAPSHOT_WAIT_MS) throw e;
      await delay(ACTIVE_SNAPSHOT_POLL_MS);
    }
  }
}

interface AgentSessionDto {
  session_id: string;
  title: string;
  mode: AgMode;
  archived: boolean;
  open_index: number;
  is_active: boolean;
  created_at: number;
  updated_at: number;
  last_active_at?: number | null;
}

interface AgentSessionsResponse {
  sessions: AgentSessionDto[];
  active_session_id?: string | null;
}

interface PersistedChatMessage {
  id: string;
  role: "user" | "assistant";
  content: string;
  parts?: MessageContentPart[];
  streaming?: boolean;
  error?: boolean;
  /** 回合内消息片段（TERM-AG-011 · BR-AG-UI-001）：additive，恢复时重建时序交错。 */
  segments?: TurnSegment[];
}

interface AgentTurnPayloadV1 {
  version: 1;
  lastEventSeq?: number;
  userMessage: PersistedChatMessage;
  assistantMessage?: PersistedChatMessage;
  toolSummaries: { id: string; name: string; inputSummary?: string; resultSummary?: string; state: string; anchorMessageId?: string }[];
  diffRefs: { diffId: string; relPath: string; anchorMessageId?: string }[];
  failure?: { code: string; message: string; retriable: boolean };
  incompleteReason?: "stream_interrupted" | "app_closed";
}

interface AgentTurnPayloadV2 {
  version: 2;
  lastEventSeq: number;
  userMessage: PersistedChatMessage;
  assistantMessage?: PersistedChatMessage;
  toolSummaries: { id: string; name: string; inputSummary?: string; resultSummary?: string; state: string; anchorMessageId?: string }[];
  diffRefs: { diffId: string; relPath: string; anchorMessageId?: string }[];
  failure?: { code: string; message: string; retriable: boolean };
  incompleteReason?: "stream_interrupted" | "app_closed";
}

interface AgentTurnDto {
  turn_id: string;
  session_id: string;
  rel_path?: string | null;
  status: AgentTurnStatus;
  payload: AgentTurnPayloadV1 | AgentTurnPayloadV2;
  created_at: number;
  updated_at: number;
}

function toSession(dto: AgentSessionDto): AgSession {
  return {
    id: dto.session_id,
    title: dto.title || "新对话",
    mode: dto.mode === "build" ? "build" : "edit",
    messages: [],
    toolCalls: [],
    createdAt: dto.created_at,
    archived: dto.archived,
    turnsLoaded: false,
  };
}

function serializeMessage(message: ChatMessage | PersistedChatMessage): PersistedChatMessage {
  return {
    id: message.id,
    role: message.role,
    content: message.content,
    parts: message.parts,
    streaming: false,
    error: message.error,
    segments: message.segments,
  };
}

function messageFromPersisted(message: PersistedChatMessage, turn: AgentTurnDto): ChatMessage {
  return {
    id: message.id,
    role: message.role,
    content: message.content,
    parts: message.parts,
    streaming: false,
    error: message.error,
    turnId: turn.turn_id,
    relPath: turn.rel_path ?? null,
    turnStatus: turn.status,
    segments: message.segments,
  };
}

function turnFailureMessage(turn: AgentTurnDto): ChatMessage | null {
  const failure = turn.payload.failure;
  if (!failure && turn.status === "draft") return null;
  const content =
    failure?.message ??
    (turn.status === "gate_rejected"
      ? "门禁拒绝：本次调用未执行。"
      : turn.status === "cancelled"
        ? "本回合已取消。"
        : "本回合未正常完成。");
  return {
    id: `hist-${turn.turn_id}-${turn.status}`,
    role: "assistant",
    content,
    error: turn.status === "error" || turn.status === "gate_rejected",
    turnId: turn.turn_id,
    relPath: turn.rel_path ?? null,
    turnStatus: turn.status,
  };
}

function normalizeToolState(state: string): ToolCallView["state"] {
  if (
    state === "pending" ||
    state === "executing" ||
    state === "done" ||
    state === "awaiting_confirmation" ||
    state === "failed" ||
    state === "skipped"
  ) {
    return state;
  }
  return "skipped";
}

function appendToolToMessage(messages: ChatMessage[], anchorMessageId: string, tool: ToolCallView): boolean {
  const index = messages.findIndex((m) => m.id === anchorMessageId);
  if (index < 0) return false;
  const message = messages[index];
  messages[index] = { ...message, toolCalls: [...(message.toolCalls ?? []), tool] };
  return true;
}

function projectTurns(turns: AgentTurnDto[]): { messages: ChatMessage[]; toolCalls: ToolCallView[] } {
  const messages: ChatMessage[] = [];
  const toolCalls: ToolCallView[] = [];
  for (const turn of turns) {
    messages.push(messageFromPersisted(turn.payload.userMessage, turn));
    let assistantAnchor: string | undefined;
    let assistant: ChatMessage | null = null;
    if (turn.payload.assistantMessage) {
      assistant = messageFromPersisted(turn.payload.assistantMessage, turn);
      assistantAnchor = assistant.id;
      messages.push(assistant);
    } else {
      const fallback = turnFailureMessage(turn);
      if (fallback) {
        assistantAnchor = fallback.id;
        messages.push(fallback);
      }
    }
    // 工具摘要 id → 恢复后 ToolCallView id 映射（AG-MSG-RENDER-01 · TERM-AG-011）：
    // payload 的 tool 段引用 ToolSummary.id，恢复后卡 id 带 turn 前缀，需重写段引用以匹配。
    const toolIdMap = new Map<string, string>();
    for (const t of turn.payload.toolSummaries ?? []) {
      const anchorMessageId = t.anchorMessageId ?? assistantAnchor;
      const tool: ToolCallView = {
        id: `${turn.turn_id}-${t.id}`,
        name: t.name,
        state: normalizeToolState(t.state),
        detail: t.resultSummary ?? t.inputSummary,
        readonly: true,
        turnId: turn.turn_id,
        anchorMessageId,
      };
      toolIdMap.set(t.id, tool.id);
      if (!anchorMessageId || !appendToolToMessage(messages, anchorMessageId, tool)) {
        toolCalls.push(tool);
      }
    }
    if (assistant?.segments?.length) {
      const index = messages.findIndex((m) => m.id === assistant!.id);
      if (index >= 0) {
        messages[index] = {
          ...messages[index],
          segments: assistant.segments.map((segment) =>
            segment.kind === "tool"
              ? { ...segment, toolCallId: toolIdMap.get(segment.toolCallId) ?? segment.toolCallId }
              : segment,
          ),
        };
      }
    }
  }
  return { messages, toolCalls };
}

interface TurnDiffRef {
  diffId: string;
  relPath: string;
  sessionId: string;
  turnId: string;
  anchorMessageId?: string;
  createdAt: number;
}

function collectTurnDiffRefs(turns: AgentTurnDto[]): TurnDiffRef[] {
  const refs: TurnDiffRef[] = [];
  for (const turn of turns) {
    const fallbackAnchor = turn.payload.assistantMessage?.id;
    for (const d of turn.payload.diffRefs ?? []) {
      refs.push({
        diffId: d.diffId,
        relPath: d.relPath,
        sessionId: turn.session_id,
        turnId: turn.turn_id,
        anchorMessageId: d.anchorMessageId ?? fallbackAnchor,
        createdAt: turn.created_at,
      });
    }
  }
  return refs;
}

function diffStateFromStatus(status: DiffDetailDto["status"]): DiffCardState {
  switch (status) {
    case "accepted":
      return "accepted";
    case "rejected":
      return "rejected";
    case "expired":
      return "expired";
    case "pending":
      return "pending";
  }
}

function hydratedDiffPatch(ref: TurnDiffRef, detail: DiffDetailDto): Partial<DiffView> {
  const projection = detail.projection;
  const styleOpsPayload = parseStyleOps(projection?.style_ops);
  const blockOpsPayload = parseStructuredBlockEditPayload(projection?.block_ops);
  const tableOpsPayload = parseTableOps(projection?.table_ops);
  const imageOpsPayload = parseImageOps(projection?.image_ops);
  const imageSnapshots = parseImageDiffSnapshotProjection(
    projection?.image_before,
    projection?.image_after,
    projection?.image_affected_block_ids,
  );
  return {
    sessionId: ref.sessionId,
    turnId: ref.turnId,
    anchorMessageId: ref.anchorMessageId,
    createdAt: detail.created_at ?? ref.createdAt,
    blockId: projection?.block_id ?? "",
    blockIndex: projection?.block_index ?? null,
    relPath: detail.target?.rel_path ?? ref.relPath,
    locationLabel: projection?.location_label ?? "历史修改建议",
    patchKind: projection?.patch_kind,
    draftMode: projection?.draft_mode,
    snapshotSource: projection?.snapshot_source,
    baseSessionVersion: projection?.base_session_version ?? undefined,
    baseContentHash: projection?.base_content_hash ?? undefined,
    title: `历史修改建议 ${detail.target?.rel_path ?? ref.relPath}`,
    before: projection?.before_text ?? "",
    after: projection?.after_text ?? "",
    state: diffStateFromStatus(detail.status),
    effectStatus: detail.effect_status,
    terminalReason: detail.terminal_reason,
    // 历史回合恢复的 diff 一律只读（BR-SYS-STATE-004/005，APP-CLOSE-DIFF-LIFECYCLE-01）：正常关闭
    // 保证退出前 pending=0，异常退出后启动清算把残留 pending 转 expired；本路径只服务于持久化历史
    // 回合恢复（ensureAgentSessionTurns），不服务于本次会话内实时生成的 diff（走 deStore.addDiff
    // 直接投影），因此不依赖 detail.status 判断——即使后端因故仍返回 pending 也不得当作可裁决态复活。
    readonly: true,
    styleOps: styleOpsPayload.length ? styleOpsPayload.map(summarizeStyleOp) : undefined,
    styleOpsPayload: styleOpsPayload.length ? styleOpsPayload : undefined,
    blockOpsPayload,
    blockOps: blockOpsPayload?.operations.map((operation) => (
      `${operation.blockId} · ${operation.expectedNodeType}`
    )),
    tableOpsPayload,
    tableOps: tableOpsPayload?.operations.map(summarizeTableOperation),
    imageOpsPayload,
    imageOps: imageOpsPayload?.operations.map(summarizeImageOperation),
    imageBefore: imageSnapshots?.before,
    imageAfter: imageSnapshots?.after,
    imageAffectedBlockIds: imageSnapshots?.affectedBlockIds,
  };
}

function unavailableDiff(ref: TurnDiffRef): DiffView {
  return {
    diffId: ref.diffId,
    sessionId: ref.sessionId,
    turnId: ref.turnId,
    anchorMessageId: ref.anchorMessageId,
    createdAt: ref.createdAt,
    blockId: "",
    relPath: ref.relPath,
    title: `历史修改建议不可用 ${ref.relPath}`,
    before: "",
    after: "",
    state: "error",
    errorCode: "DE_DIFF_UNAVAILABLE",
    readonly: true,
  };
}

async function hydrateDiffRefsForTurns(turns: AgentTurnDto[]): Promise<void> {
  const refs = collectTurnDiffRefs(turns);
  if (!refs.length) return;
  const details = await loadDiffDetails(refs.map((r) => r.diffId)).catch((e) => {
    console.error(`de.list_diffs by ids failed ${JSON.stringify(e)}`);
    return [] as DiffDetailDto[];
  });
  const detailById = new Map(details.map((d) => [d.diff_id, d]));
  const existing = new Set(deStore.getSnapshot().diffs.map((d) => d.diffId));
  for (const ref of refs) {
    const detail = detailById.get(ref.diffId);
    if (!detail) {
      if (existing.has(ref.diffId)) {
        deStore.attachMeta(ref.diffId, {
          sessionId: ref.sessionId,
          turnId: ref.turnId,
          anchorMessageId: ref.anchorMessageId,
          createdAt: ref.createdAt,
        });
      } else {
        existing.add(ref.diffId);
        deStore.addDiff(unavailableDiff(ref));
      }
      continue;
    }
    const patch = hydratedDiffPatch(ref, detail);
    if (existing.has(ref.diffId)) {
      deStore.attachMeta(ref.diffId, patch);
    } else {
      existing.add(ref.diffId);
      deStore.addDiff({ diffId: ref.diffId, ...patch } as DiffView);
    }
  }
}

export async function persistAgentSessionsMetadata(): Promise<void> {
  const { sessions, activeSessionId } = agStore.getSnapshot();
  await Promise.all(
    sessions.map((s, index) =>
      dispatchCall("ag.save_session", {
        sessionId: s.id,
        title: s.title,
        mode: s.mode,
        archived: s.archived,
        openIndex: index,
        isActive: s.id === activeSessionId,
      }).catch((e) => console.error(`ag.save_session failed ${JSON.stringify(e)}`)),
    ),
  );
}

/** AG-1.0-IMP-TASKRUN-UI-01 · 后端 `ag.list_task_runs` DTO（snake_case，与 TaskRunV1 对齐）。 */
export interface TaskStepDto {
  step_id: string;
  kind: string;
  label: string;
  status: TaskStepViewStatus;
}

export interface TaskRunDto {
  task_run_id: string;
  status: TaskRunViewStatus;
  intent_summary?: string | null;
  steps: TaskStepDto[];
  created_at: number;
  updated_at: number;
}

/** 查询会话受限任务实例（SM-AG-04 状态/步骤，倒序）；映射为 TaskRunView 投影到 agStore。 */
export async function listSessionTaskRuns(sessionId: string): Promise<TaskRunView[]> {
  const runs = await dispatchCall<TaskRunDto[]>("ag.list_task_runs", { sessionId });
  return runs.map((r) => ({
    taskRunId: r.task_run_id,
    status: r.status,
    intentSummary: r.intent_summary ?? undefined,
    steps: r.steps.map((s) => ({ stepId: s.step_id, kind: s.kind, label: s.label, status: s.status })),
    createdAt: r.created_at,
    updatedAt: r.updated_at,
  }));
}

/** 刷新会话 TaskRun 摘要（回合 done/error 后 + 历史恢复时；查询失败不阻塞，best-effort）。 */
export async function refreshSessionTaskRuns(sessionId: string): Promise<void> {
  try {
    const runs = await listSessionTaskRuns(sessionId);
    agStore.hydrateSessionTaskRuns(sessionId, runs);
  } catch {
    // 保持上次状态或空；不伪造进度（BR-SYS-STATE-008）。
  }
}

function resolvedDecisionPostProcessingError(cause: unknown, taskRunId: string) {
  const value = cause && typeof cause === "object"
    ? cause as { code?: unknown; message?: unknown }
    : null;
  return {
    code: typeof value?.code === "string" ? value.code : "AG_DECISION_POST_PROCESSING_FAILED",
    message: typeof value?.message === "string" ? value.message : String(cause),
    cause,
    decisionResolved: true as const,
    taskRunId,
  };
}

/** AG-1.0-TASKRUN-REWORK-IMP-UI-01：任务级裁决（TERM-AG-017，三层分离）——
 * confirm → TaskRun Executing（可自动下一片段）；cancel → Cancelled。返回已裁决的 run 身份。 */
export async function resolveDecision(
  requestId: string,
  outcome: "confirm" | "cancel",
): Promise<{
  taskRunId: string;
  status: string;
  workspaceOperationReceipt?: WorkspaceOperationReceiptDto;
  createDocumentDiffId?: string;
}> {
  const pendingDecision = agStore.getSnapshot().pendingDecision;
  const pendingPayload = pendingDecision?.requestId === requestId
    && pendingDecision.payload
    && typeof pendingDecision.payload === "object"
    ? pendingDecision.payload as Record<string, unknown>
    : null;
  if (outcome === "confirm" && typeof pendingPayload?.workspaceOperationPlanId === "string") {
    await flushOpenEditorSessionSnapshots();
  }
  const result = await dispatchCall<{
    task_run_id: string;
    status: string;
    workspace_operation_receipt?: WorkspaceOperationReceiptDto | null;
    workspace_operation_items?: WorkspaceOperationItemDto[] | null;
    create_document_bundle?: CreateDocumentBundleDto | null;
    already_resolved?: boolean;
  }>("ag.resolve_decision", {
    requestId,
    outcome,
  });
  try {
    let createDocumentDiffId: string | undefined;
    if (result.workspace_operation_receipt) await loadDocs();
    for (const diffId of result.workspace_operation_receipt?.invalidated_diff_ids ?? []) {
      deStore.expireLocal(diffId, {
        expiredReason: "document_revision_advanced",
        errorCode: "resource_baseline_changed",
      });
    }
    if (result.workspace_operation_items) {
      const completed = new Set(
        result.workspace_operation_receipt?.item_receipts
          .filter((receipt) => receipt.status === "completed")
          .map((receipt) => receipt.item_id) ?? [],
      );
      for (const receipt of result.workspace_operation_receipt?.compensation_receipts ?? []) {
        if (receipt.status === "compensated") completed.delete(receipt.item_id);
      }
      const effectiveItems = result.workspace_operation_items.filter((item) => completed.has(item.item_id));
      agStore.reconcileResourceReferences(effectiveItems);
      await reconcileWorkspaceOperationTabs(
        effectiveItems,
        Boolean(result.create_document_bundle),
      );
    }
    if (result.create_document_bundle) {
      createDocumentDiffId = await projectCreateDocumentBundle(result.create_document_bundle);
    }
    return {
      taskRunId: result.task_run_id,
      status: result.status,
      workspaceOperationReceipt: result.workspace_operation_receipt ?? undefined,
      createDocumentDiffId,
    };
  } catch (cause) {
    throw resolvedDecisionPostProcessingError(cause, result.task_run_id);
  }
}

interface CreateDocumentBundleDto {
  plan_id: string;
  target: { rel_path: string };
  status: string;
}

interface WorkspaceOperationItemDto {
  item_id: string;
  seq: number;
  kind: "create_document" | "create_folder" | "rename_resource" | "move_resource" | "delete_resource" | "restore_resource" | "discard_created_resource";
  source_rel_path?: string | null;
  target_rel_path?: string | null;
  resource_kind: "file" | "folder";
}

async function reconcileWorkspaceOperationTabs(
  items: WorkspaceOperationItemDto[],
  hasCreateBundle: boolean,
): Promise<void> {
  for (const item of [...items].sort((left, right) => left.seq - right.seq)) {
    const source = item.source_rel_path ?? "";
    const target = item.target_rel_path ?? "";
    const matchingTabs = tabsStore.getSnapshot().tabs.filter((tab) => (
      tab.kind === "doc"
      && source
      && (tab.path === source || (item.resource_kind === "folder" && tab.path.startsWith(`${source}/`)))
    ));
    if (item.kind === "delete_resource" || item.kind === "discard_created_resource") {
      for (const tab of matchingTabs) tabsStore.close(tab.id, { force: true });
      continue;
    }
    if (item.kind === "rename_resource" || item.kind === "move_resource") {
      const reopenTargets = matchingTabs.map((tab) => (
        item.resource_kind === "folder"
          ? `${target}${tab.path.slice(source.length)}`
          : target
      ));
      for (const tab of matchingTabs) tabsStore.close(tab.id, { force: true });
      for (const relPath of reopenTargets) {
        tabsStore.openDoc(relPath);
        await openDocument(relPath);
      }
      continue;
    }
    if (item.kind === "create_document" && target && !hasCreateBundle) {
      tabsStore.openDoc(target);
      await openDocument(target);
    }
  }
}

interface CreateDocumentProjectionResult {
  diff_id?: string;
  task_run_id?: string;
  source_turn_id?: string;
  already_projected?: boolean;
  projection_failed_compensated?: boolean;
  error?: unknown;
  projection?: DiffProjectionDto;
}

async function projectCreateDocumentBundle(bundle: CreateDocumentBundleDto): Promise<string | undefined> {
  const relPath = bundle.target.rel_path;
  tabsStore.openDoc(relPath);
  try {
    await openDocument(relPath);
  } catch (openError) {
    console.error("[create-document-trace]", {
      phase: "open_document",
      planId: bundle.plan_id,
      relPath,
      error: openError,
    });
    try {
      await dispatchCall("ws.compensate_create_document_open_failure", { planId: bundle.plan_id });
      const tab = tabsStore.getSnapshot().tabs.find((candidate) => candidate.kind === "doc" && candidate.path === relPath);
      if (tab) tabsStore.close(tab.id, { force: true });
      await loadDocs();
    } catch (compensationError) {
      throw {
        code: "WS_CREATE_BUNDLE_PARTIAL_FAILED",
        message: `新文档已创建但编辑器打开失败，自动补偿也失败：${String(compensationError)}`,
        cause: openError,
      };
    }
    throw { code: "ED_CREATE_BUNDLE_OPEN_FAILED_COMPENSATED", message: "新文档打开失败，已安全撤销创建", cause: openError };
  }
  const session = edStore.getSnapshot().sessions[relPath];
  if (!session || session.content === null) {
    throw { code: "ED_CREATE_BUNDLE_SESSION_MISSING", message: "新文档已创建，但编辑器未建立 LogicalState" };
  }
  let projected: CreateDocumentProjectionResult;
  try {
    projected = await dispatchCall<CreateDocumentProjectionResult>("ws.project_create_document_bundle", {
      planId: bundle.plan_id,
      sessionId: activeSession(agStore.getSnapshot()).id,
      relPath,
      beforeSnapshot: session.content,
      baseSessionVersion: session.sessionVersion,
      baseContentHash: session.contentHash,
    });
  } catch (projectionError) {
    console.error("[create-document-trace]", {
      phase: "project_initial_content",
      planId: bundle.plan_id,
      relPath,
      error: projectionError,
    });
    try {
      await dispatchCall("ws.compensate_create_document_projection_preflight_failure", {
        planId: bundle.plan_id,
      });
      const tab = tabsStore.getSnapshot().tabs.find((candidate) => candidate.kind === "doc" && candidate.path === relPath);
      if (tab) tabsStore.close(tab.id, { force: true });
      await loadDocs();
    } catch (compensationError) {
      throw {
        code: "WS_CREATE_BUNDLE_PARTIAL_FAILED",
        message: `初始内容投影失败，自动补偿也失败：${String(compensationError)}`,
        cause: projectionError,
      };
    }
    throw { code: "WS_CREATE_BUNDLE_PROJECTION_FAILED_COMPENSATED", message: "初始内容投影失败，已安全撤销创建", cause: projectionError };
  }
  if (projected.projection_failed_compensated) {
    const tab = tabsStore.getSnapshot().tabs.find((candidate) => (
      candidate.kind === "doc" && candidate.path === relPath
    ));
    if (tab) tabsStore.close(tab.id, { force: true });
    await loadDocs();
    throw {
      code: "WS_CREATE_BUNDLE_PROJECTION_FAILED_COMPENSATED",
      message: "初始内容投影失败，已安全撤销创建",
      cause: projected.error,
    };
  }
  if (!projected.diff_id) {
    throw { code: "WS_CREATE_BUNDLE_DIFF_MISSING", message: "新文档初始内容未生成可审阅 Diff" };
  }
  if (projected.already_projected || !projected.projection) return projected.diff_id;
  const projection = projected.projection;
  const logicalAfter = session.editFormat === "markdown"
    ? (() => {
        const parsed = parseMarkdownForEditing(projection.after_text);
        if (!parsed.ok) return projection.after_text;
        edStore.setContentFormat(relPath, "html");
        return parsed.html;
      })()
    : projection.after_text;
  deStore.addDiff({
    diffId: projected.diff_id,
    sessionId: activeSession(agStore.getSnapshot()).id,
    turnId: projected.source_turn_id,
    taskRunId: projected.task_run_id,
    createdAt: Date.now(),
    blockId: "",
    blockIndex: null,
    relPath,
    locationLabel: projection.location_label,
    patchKind: "replace_document",
    draftMode: "current_editor",
    snapshotSource: projection.snapshot_source,
    baseSessionVersion: projection.base_session_version ?? session.sessionVersion,
    baseContentHash: projection.base_content_hash ?? session.contentHash,
    previewContinuationRequired: true,
    title: `AI 建议新建 ${relPath}`,
    before: projection.before_text,
    // 初始内容是完整文档载荷，不能按块内文本清洗，否则标题、段落和列表结构会丢失。
    after: logicalAfter,
    state: "pending",
    draftPreviewState: "queued",
    readonly: false,
  });
  return projected.diff_id;
}

export async function compensateCreateDocumentPreviewFailure(diffId: string): Promise<boolean> {
  const diff = deStore.getSnapshot().diffs.find((candidate) => candidate.diffId === diffId);
  const session = diff?.relPath ? edStore.getSnapshot().sessions[diff.relPath] : undefined;
  if (!diff || !session) return false;
  try {
    const result = await dispatchCall<{ compensated?: boolean; not_create_bundle?: boolean }>(
      "ws.compensate_create_document_preview_failure",
      {
        diffId,
        logicalSessionVersion: session.sessionVersion,
        logicalContentHash: session.contentHash,
      },
    );
    if (!result.compensated) return false;
    const tab = tabsStore.getSnapshot().tabs.find((candidate) => candidate.kind === "doc" && candidate.path === diff.relPath);
    if (tab) tabsStore.close(tab.id, { force: true });
    await loadDocs();
    deStore.expireLocal(diffId, {
      expiredReason: "apply_replace_failed",
      errorCode: "WS_CREATE_BUNDLE_PREVIEW_FAILED_COMPENSATED",
    });
    return true;
  } catch (error) {
    console.error("[workspace-operation] create document preview compensation failed", error);
    return false;
  }
}

export interface WorkspaceOperationItemReceiptDto {
  item_id: string;
  seq: number;
  status: "attempted" | "completed" | "compensated" | "failed";
  error_code?: string | null;
  error_message?: string | null;
}

export interface WorkspaceOperationReceiptDto {
  operation_id: string;
  plan_id: string;
  final_status:
    | "completed"
    | "failed_compensated"
    | "partial_failed"
    | "expired"
    | "interrupted";
  item_receipts: WorkspaceOperationItemReceiptDto[];
  compensation_receipts: WorkspaceOperationItemReceiptDto[];
  entry_id?: string | null;
  invalidated_diff_ids?: string[];
}

const previewAckInFlight = new Set<string>();

/**
 * @GOV
 * codes:    BR-SYS-STATE-008-EFFECT-AG-AG-TURN-006, BR-AG-DATA-001-EFFECT-AG-AG-TURN-006
 * type:     EFFECT
 * chain:    AG-TURN, DE-DIFF, ED-EDIT
 * rules:    BR-SYS-STATE-008, BR-AG-DATA-001
 * term_ref: TERM-AG-015, TERM-ED-008
 * boundary: in=AppliedDiffCardAndCurrentLogicalStateSnapshot | out=PreviewAckThenFreshSnapshotContinuation(noDiffAcceptNoDiskWriteNoBackgroundCall)
 */
/** 只有 ED/DE 已成功标记 draftApplied 后才回报 PreviewAck，并由前端明确启动下一执行片段。
 * 此处不从 Diff payload 推断内容，也不会对 background Diff 或无 checkpoint 的卡片续编。 */
export async function acknowledgePreviewAndContinue(diffId: string): Promise<void> {
  if (previewAckInFlight.has(diffId)) return;
  const diff = deStore.getSnapshot().diffs.find((candidate) => candidate.diffId === diffId);
  if (
    !diff?.previewContinuationRequired
    || !diff.draftApplied
    || diff.state !== "pending"
    || !diff.taskRunId
    || !diff.turnId
    || !diff.sessionId
    || !diff.relPath
  ) return;
  const session = edStore.getSnapshot().sessions[diff.relPath];
  if (!session || session.content === null) return;
  if (diff.draftAfterContent !== undefined && session.content !== diff.draftAfterContent) {
    await dispatchCall("de.expire_diff", {
      diffId,
      reason: "ED_DRAFT_BASE_ADVANCED",
    });
    deStore.expireLocal(diffId, {
      expiredReason: "document_revision_advanced",
      errorCode: "ED_DRAFT_BASE_ADVANCED",
    });
    return;
  }

  previewAckInFlight.add(diffId);
  try {
    agStore.setCurrentAction({ action: "previewing", label: "已应用预览，正在校验最新文档状态" });
    await invoke<{ task_run_id: string; ready: boolean }>("ag_ack_preview", {
      taskRunId: diff.taskRunId,
      sessionId: diff.sessionId,
      turnId: diff.turnId,
      diffId,
      relPath: diff.relPath,
      sessionVersion: session.sessionVersion,
      contentHash: session.contentHash,
    });
    await startTurn("（编辑器预览已完成）请基于最新文档状态继续当前任务；若目标已完成，请直接给出简短结果。", {
      autoContinue: true,
      mentions: [diff.relPath],
      continuation: { taskRunId: diff.taskRunId, previewDiffId: diffId },
    });
  } catch (error) {
    // Ack 被拒绝意味着基线已变化/卡片终态/身份不匹配；不重试、更不以旧快照续编。
    console.error("[ag-preview] continuation acknowledgement rejected", error);
    agStore.setCurrentAction(null);
  } finally {
    previewAckInFlight.delete(diffId);
  }
}

/** 任务级裁决后恢复原 AgentTurn。confirm/cancel 都是模型可见的合法用户事件；
 * 只有本进程仍持有 AwaitingDecision 身份时才恢复，历史任务裁决不会猜测或伪造回合。 */
export async function resolveDecisionAndContinue(
  requestId: string,
  outcome: "confirm" | "cancel",
): Promise<Awaited<ReturnType<typeof resolveDecision>>> {
  const decisionTurn = decisionTurns.get(requestId);
  const resolution = await resolveDecision(requestId, outcome);
  if (
    decisionTurn
    && (resolution.status === "executing" || resolution.status === "cancelled")
  ) {
    if (closeStore.getSnapshot().phase !== "idle") {
      await invoke("ag_cancel_turn", {
        sessionId: decisionTurn.sessionId,
        turnId: decisionTurn.turnId,
      });
      await projectParkedCancellation(decisionTurn);
      return resolution;
    }
    await startTurn(decisionTurn.prompt, {
      autoContinue: true,
      continuation: { taskRunId: resolution.taskRunId, decisionRequestId: requestId },
    });
  }
  return resolution;
}

interface TaskPlanStepDto {
  step_id: string;
  seq: number;
  kind: string;
  label: string;
  deps: string[];
  required_grant?: string | null;
  confirm_point: boolean;
  status: string;
}

interface TaskPlanDto {
  plan_id: string;
  task_run_id: string;
  steps: TaskPlanStepDto[];
  cursor: number;
}

/** AG-1.0-TASKRUN-REWORK-CONTINUE-01：按 task_run_id 列出任务计划（TERM-AG-018；snake→camel）。 */
export async function listPlansByRun(taskRunId: string): Promise<TaskPlanView[]> {
  const plans = await dispatchCall<TaskPlanDto[]>("ag.list_plans", { taskRunId });
  return plans.map((p) => ({
    planId: p.plan_id,
    taskRunId: p.task_run_id,
    cursor: p.cursor,
    steps: p.steps.map((s) => ({
      stepId: s.step_id,
      seq: s.seq,
      kind: s.kind,
      label: s.label,
      deps: s.deps,
      requiredGrant: s.required_grant ?? undefined,
      confirmPoint: s.confirm_point,
      status: s.status,
    })),
  }));
}

/** AG-1.0-TASKRUN-REWORK-INTENT-01：刷新会话任务计划（回合 done/error 后；查询失败不阻塞，best-effort）。 */
export async function refreshSessionPlans(sessionId: string): Promise<void> {
  try {
    const runs = await listSessionTaskRuns(sessionId);
    const latest = runs[0];
    if (latest) {
      const plans = await listPlansByRun(latest.taskRunId);
      agStore.hydrateSessionPlans(sessionId, plans);
    }
  } catch {
    // 保持上次状态；不伪造计划（BR-SYS-STATE-008）。
  }
}

export async function ensureAgentSessionTurns(sessionId: string): Promise<void> {
  const session = agStore.getSnapshot().sessions.find((s) => s.id === sessionId);
  if (!session || session.turnsLoaded) return;
  const turns = await dispatchCall<AgentTurnDto[]>("ag.list_turns", { sessionId });
  const projected = projectTurns(turns);
  await hydrateDiffRefsForTurns(turns);
  agStore.hydrateSessionTurns(sessionId, projected.messages, projected.toolCalls);
  for (const turn of turns) {
    agStore.seedTurnEventProjection(
      turn.turn_id,
      turn.payload.lastEventSeq ?? 0,
      turn.status !== "draft" && turn.status !== "clarifying",
    );
  }
  await refreshSessionTaskRuns(sessionId);
}

/** Channel 中断后的安全补流入口；事件仍由相同 seq reducer 去重与终态冻结。 */
export async function listTurnEvents(turnId: string, afterSeq = 0): Promise<AgTurnEvent[]> {
  return dispatchCall<AgTurnEvent[]>("ag.list_turn_events", { turnId, afterSeq });
}

export async function hydrateAgentHistory(): Promise<void> {
  try {
    const res = await dispatchCall<AgentSessionsResponse>("ag.list_sessions", {});
    if (!res.sessions.length) {
      agStore.reset();
      await persistAgentSessionsMetadata();
      return;
    }
    await loadPersistedDiffs();
    agStore.hydrateSessions(res.sessions.map(toSession), res.active_session_id ?? undefined);
    await ensureAgentSessionTurns(agStore.getSnapshot().activeSessionId);
  } catch (e) {
    console.error(`ag.list_sessions failed ${JSON.stringify(e)}`);
    agStore.reset();
  }
}

export function newAgentSession(): string {
  const id = agStore.newSession();
  void persistAgentSessionsMetadata();
  return id;
}

export function switchAgentSession(id: string): void {
  agStore.switchSession(id);
  void ensureAgentSessionTurns(id);
  void persistAgentSessionsMetadata();
}

export function closeAgentSession(id: string): void {
  agStore.closeSession(id);
  void persistAgentSessionsMetadata();
}

const turnSaveChain = new Map<string, Promise<void>>();

export async function deleteAgentSession(sessionId: string): Promise<void> {
  const hasParkedDecision = [...decisionTurns.values()]
    .some((decision) => decision.sessionId === sessionId);
  if (agStore.getSnapshot().sending || clarifyingTurns.has(sessionId) || hasParkedDecision) {
    throw { code: "AG_TURN_ACTIVE", message: "代理回合进行中，暂不能删除会话" };
  }
  await Promise.allSettled([...turnSaveChain.values()]);
  await dispatchCall("ag.delete_session", { sessionId });
  turnSaveChain.clear();
  agStore.deleteSession(sessionId);
  await persistAgentSessionsMetadata();
}

function enqueueTurnSave(
  sessionId: string,
  turnId: string,
  status: AgentTurnStatus,
  relPath?: string | null,
  failure?: { code: string; message: string; retriable: boolean },
): Promise<void> {
  const prev = turnSaveChain.get(turnId) ?? Promise.resolve();
  const next = prev
    .catch(() => undefined)
    .then(async () => {
      const session = agStore.getSnapshot().sessions.find((s) => s.id === sessionId);
      if (!session) return;
      const turnMessages = session.messages.filter((m) => m.turnId === turnId);
      const user = turnMessages.find((m) => m.role === "user");
      if (!user) return;
      const assistants = turnMessages.filter((m) => m.role === "assistant");
      const assistant =
        [...assistants].reverse().find((m) => m.content.trim() || m.error) ?? assistants[0];
      const diffRefs = deStore
        .getSnapshot()
        .diffs.filter((d) => d.turnId === turnId)
        .map((d) => ({
          diffId: d.diffId,
          relPath: d.relPath ?? "",
          anchorMessageId: d.anchorMessageId ?? assistant?.id,
        }))
        .filter((d) => d.relPath);
      const payload: AgentTurnPayloadV2 = {
        version: 2,
        lastEventSeq: agStore.lastTurnEventSeq(turnId),
        userMessage: serializeMessage(user),
        assistantMessage: assistant ? serializeMessage(assistant) : undefined,
        toolSummaries: session.toolCalls.filter((t) => t.turnId === turnId).map((t) => ({
          id: t.id,
          name: t.name,
          inputSummary: t.detail,
          resultSummary: t.state === "done" || t.state === "failed" ? t.detail : undefined,
          state: t.state,
          anchorMessageId: t.anchorMessageId ?? assistant?.id,
        })),
        diffRefs,
        failure,
        incompleteReason: status === "draft" ? "stream_interrupted" : undefined,
      };
      await dispatchCall("ag.save_turn", { sessionId, turnId, relPath, payload });
    })
    .catch((e) => console.error(`ag.save_turn failed ${JSON.stringify(e)}`));
  turnSaveChain.set(turnId, next);
  return next;
}

/**
 * BYOK：把 provider 密钥一次性上送 dev 网关代持（PI-BYOK-DEV-01）。
 * 客户端不持久化、不回显——密钥经后端 ent::byok 送网关后即清零（X-FORBID-001）。
 */
export async function uploadByok(provider: string, key: string): Promise<void> {
  await invoke("byok_upload", { provider, key });
}

/** dev GW `/models` 预设模型条目（TERM-AG-010，桌面端仅消费标识符）。 */
export interface PresetModelDto {
  provider: string;
  model: string;
  label: string;
  source: string;
}

/** 拉取预设模型清单（AG-MODEL-CATALOG / TERM-AG-010）：经后端 ent::gateway fetch_models。 */
export async function listPresetModels(): Promise<PresetModelDto[]> {
  const models = await invoke<PresetModelDto[]>("list_models");
  return Array.isArray(models) ? models : [];
}

/** dev GW 已配置 BYOK provider（脱敏 key_preview，不回显明文）。 */
export interface ByokProviderStatus {
  provider: string;
  key_preview: string;
}

/** 查询已配置 BYOK provider 状态（AG-MODEL-CATALOG / PI-GW-02）。 */
export async function byokListProviders(): Promise<ByokProviderStatus[]> {
  const providers = await invoke<ByokProviderStatus[]>("byok_list_providers");
  return Array.isArray(providers) ? providers : [];
}

/** 删除指定 provider 的 BYOK 密钥（立即生效）。 */
export async function byokDeleteKey(provider: string): Promise<void> {
  await invoke("byok_delete_key", { provider });
}

/** 辅助请求（⌘K 内联改写 / ⌘J 续写补全）。 */
export interface AssistReq {
  requestId: string;
  kind: "inline" | "autocomplete";
  docPath: string;
  loadSeq: number;
  blockId: string;
  start: number;
  end: number;
  instruction?: string; // inline：改写指令
  selection?: string; // inline：选区原文
  prefix?: string; // autocomplete：光标前文本
  before?: string; // 触发锚点前文指纹
  after?: string; // 触发锚点后文指纹
  baseText?: string; // inline refine：上一轮候选
  /** 调用模型（provider/model 标识符，不含密钥；调用点按能力偏好解析传入，BR-AG-DATA-007）。缺省由后端按 provider 默认模型。 */
  provider?: string;
  model?: string;
}

export interface AssistResponse {
  requestId: string;
  candidate: string | null;
  candidates: string[];
  failureReason: string | null;
}

/**
 * 经 `ag_request_assist`（独立命令；后端门禁前置 + 网关单次产出 + SM-AG-03 驱动/广播，A-AG-I-P-01）取候选。
 * 前端在 SM-AG-03 节流后调用；门禁不可用/失败/空产出 → 空候选 + failureReason（静默不提供，不打断编辑，SM-AG-03）。
 */
/**
 * @GOV
 * codes:    BR-AG-STATE-003-EFFECT-AG-AG-ASSIST-002
 * type:     EFFECT
 * chain:    AG-ASSIST
 * rules:    BR-AG-STATE-003
 * term_ref: TERM-AG-003, TERM-AG-004
 * boundary: in=AssistReq
 *           | out=AssistResponse(candidatesOrInternalFailureReasonPreserved)
 *           | delegate=ag_request_assist
 *           | does_not=swallow_invoke_error_as_reasonless_empty_state
 */
export async function requestAssist(req: AssistReq): Promise<AssistResponse> {
  try {
    const r = await invoke<{
      request_id?: string;
      candidate?: string | null;
      candidates?: string[];
      failure_reason?: string | null;
    }>("ag_request_assist", {
      assistId: req.requestId,
      kind: req.kind,
      docPath: req.docPath,
      loadSeq: req.loadSeq,
      blockId: req.blockId,
      start: req.start,
      end: req.end,
      instruction: req.instruction,
      selection: req.selection,
      prefix: req.prefix,
      before: req.before,
      after: req.after,
      baseText: req.baseText,
      provider: req.provider,
      model: req.model,
    });
    return {
      requestId: r.request_id ?? req.requestId,
      candidate: r.candidate ?? null,
      candidates: Array.isArray(r.candidates) ? r.candidates.filter((item) => typeof item === "string") : [],
      failureReason: typeof r.failure_reason === "string" ? r.failure_reason : null,
    };
  } catch {
    return {
      requestId: req.requestId,
      candidate: null,
      candidates: [],
      failureReason: "invoke_error",
    };
  }
}

export type AssistFinishEvent = "accept" | "dismiss" | "anchor_lost";

/**
 * 回写 SM-AG-03 终态。该命令只广播辅助回合生命周期，不修改编辑器内容。
 */
export async function finishAssist(requestId: string, event: AssistFinishEvent): Promise<void> {
  try {
    await invoke("ag_finish_assist", { assistId: requestId, event });
  } catch {
    // 终态回写失败不得打断编辑；状态机观测缺口由治理/日志追踪。
  }
}

/**
 * @GOV
 * codes:    BR-AG-OBS-001-EFFECT-AG-AG-TURN-035, BR-SYS-STATE-007-EFFECT-AG-AG-TURN-035, BR-AG-STATE-001-EFFECT-AG-AG-TURN-035
 * type:     EFFECT
 * chain:    AG-TURN
 * rules:    BR-AG-OBS-001, BR-SYS-STATE-007, BR-AG-STATE-001
 * term_ref: TERM-AG-001, TERM-AG-021
 * boundary: in=UserPromptOrClarificationForCurrentSessionAndOrderedSafeTurnEvents | out=NewOrSameIdentityAgentTurnInvocationMessageProjectionAndReadOnlyProjectReport | effect=memory,IPC-ag_start_turn | consumer=Composer | does_not=cancelClarifyingTurnCreateNewTurnIdForClarificationCrossSessionResumeOrTreatProjectReportAsAuthorization
 */
/**
 * 发起对话回合：`ag_start_turn`（async + Channel）→ 真实流式 `AgTurnEvent` 投影到 agStore。
 * 多标签会话（PI-AG-UI-01）：回合归属发起时的活动会话（sessionId 闭包定位），流式期间切换标签
 * 不串会话。需先启动 cloud/dev-gateway（localhost:8787）并配置 provider 密钥；否则回
 * AG_GW_UNREACHABLE 错误终态。
 */
export async function startTurn(
  prompt: string,
  opts?: {
    provider?: string;
    model?: string;
    mentions?: string[];
    blockRefs?: { relPath: string; blockId: string; text: string; start?: number; end?: number }[];
    references?: ReferencePayload[];
    displayParts?: MessageContentPart[];
    /** 受控恢复入口；只供已确认的裁决或编辑器预览回执使用。 */
    autoContinue?: boolean;
    /** 已确认的 DecisionRequest 或预览回执所属任务。 */
    continuation?: { taskRunId: string; decisionRequestId?: string; previewDiffId?: string };
  },
): Promise<void> {
  const trimmed = prompt.trim();
  const submittedReferences = opts?.references ?? [];
  if (!trimmed && submittedReferences.length === 0) return;
  if (opts?.continuation && !opts.autoContinue) {
    throw new Error("任务续片段只能由已确认的任务级裁决自动启动");
  }
  if (
    opts?.continuation
    && Boolean(opts.continuation.decisionRequestId) === Boolean(opts.continuation.previewDiffId)
  ) {
    throw new Error("任务续片段必须且只能关联已确认裁决或已确认预览回执之一");
  }
  // AG-1.0-DECISION-CARD-01：存在待裁决任务级请求时，所有普通发送均被门控；
  // 必须先完成裁决（DecisionBar），避免"确认"等文本被当作新编辑请求触发 AG_EDIT_TOOL_REQUIRED。
  if (!opts?.autoContinue && agStore.getSnapshot().pendingDecision) {
    return;
  }

  // 关闭屏障（BR-SYS-STATE-004，APP-CLOSE-DIFF-LIFECYCLE-01）：关闭请求建立后拒绝新回合，
  // 避免清点后又产生新的待裁决 diff。
  if (closeStore.getSnapshot().phase !== "idle") {
    console.error("ag_start_turn blocked: application close in progress");
    return;
  }

  // 回合归属会话：发起时的活动会话（此后用户切标签不影响本回合投影目标）。
  const snap = agStore.getSnapshot();
  const decision = opts?.autoContinue && opts.continuation?.decisionRequestId
    ? decisionTurns.get(opts.continuation.decisionRequestId)
    : undefined;
  const sessionId = decision?.sessionId ?? snap.activeSessionId;
  const session = snap.sessions.find((s) => s.id === sessionId);
  const workspaceId = workspaceShell.getSnapshot().activeWorkspace?.workspaceId;
  if (!workspaceId) throw new Error("AG_CONTEXT_FACTS_WORKSPACE_REQUIRED");
  const clarification = !opts?.autoContinue ? clarifyingTurns.get(sessionId) : undefined;
  if (clarification && opts?.continuation) {
    throw new Error("澄清恢复不能与任务裁决或预览续片段混用");
  }
  const turnId = clarification?.turnId ?? decision?.turnId ?? `t-${Date.now()}-${(seq += 1)}`;
  const effectivePrompt = decision?.prompt ?? (clarification?.pendingInput
    ? `${clarification.pendingInput}${trimmed ? `\n\n${trimmed}` : ""}`
    : trimmed);
  const references = mergeReferencePayloads(
    clarification?.references ?? decision?.references ?? [],
    submittedReferences,
  );
  const mentions = [...new Set([
    ...(clarification?.mentions ?? decision?.mentions ?? []),
    ...(opts?.mentions ?? []),
  ])];
  const blockRefs = mergeLegacyBlockRefs(
    clarification?.blockRefs ?? decision?.blockRefs ?? [],
    opts?.blockRefs ?? [],
  );
  const turnEd = edStore.getSnapshot();
  const turnActiveDocTarget: ActiveDocTarget = {
    relPath: turnEd.openPath,
    loadSeq: turnEd.loadSeq,
    editFormat: turnEd.editFormat,
    contentFormat: turnEd.contentFormat,
  };
  const turnActiveDocPath = turnActiveDocTarget.relPath ?? undefined;

  // 本轮之前的对话历史（PI-AG-09B）：传后端按预算保留最近（drop 最旧）。取本轮 user 入栈前的快照。
  const history = (session?.messages ?? [])
    .filter((m) => m.turnId !== turnId && !m.error && m.turnStatus !== "draft" && m.content.trim())
    .slice(-30)
    .map((m) => ({
      workspaceId,
      sessionId,
      role: m.role,
      content: m.content,
      relPath: m.relPath ?? null,
      turnId: m.turnId ?? null,
    }));

  let assistantId: string;
  if (clarification) {
    // 先合并补充上下文但保留等待身份；只有编辑器快照就绪、即将调用后端时才认领。
    // 这样调用前失败仍能回到同一 AwaitingClarification，不会丢失原 AgentTurn。
    clarifyingTurns.set(sessionId, {
      ...clarification,
      pendingInput: effectivePrompt,
      references,
      mentions,
      blockRefs,
    });
    agStore.resumeClarifyingTurn(sessionId, turnId, trimmed, opts?.displayParts);
    assistantId = clarification.assistantMessageId;
  } else if (decision) {
    assistantId = decision.assistantMessageId;
  } else {
    agStore.pushMessage(sessionId, {
      id: uid(),
      role: "user",
      content: trimmed,
      parts: opts?.displayParts,
      turnId,
      relPath: turnActiveDocPath ?? null,
      turnStatus: "draft",
    });
    assistantId = uid();
    agStore.pushMessage(sessionId, {
      id: assistantId,
      role: "assistant",
      content: "",
      streaming: true,
      turnId,
      relPath: turnActiveDocPath ?? null,
      turnStatus: "draft",
    });
  }
  agStore.clearToolCalls(sessionId); // 新回合清空旧工具卡（PI-AG-08D）
  agStore.setActiveTurn(sessionId, turnId); // 后端同 session 单活仍是权威守卫；前端只做即时交互门控。
  void persistAgentSessionsMetadata();

  const onEvent = new Channel<AgTurnEvent>();

  await new Promise<void>((resolve) => {
    let settled = false;
    let terminalSeen = false;
    let pausedForClarification = false;
    let pausedForDecision = false;
    let backendInvocationStarted = false;
    const finish = () => {
      if (settled) return;
      settled = true;
      resolve();
    };
    const failStart = (e: unknown) => {
      if (terminalSeen || pausedForClarification || pausedForDecision) return;
      if (clarification && !backendInvocationStarted) {
        // 快照/装配尚未越过后端入口：保持原回合非终态，允许用户修复编辑器状态后继续补充。
        pausedForClarification = true;
        agStore.setTurnStatus(sessionId, turnId, "clarifying");
        agStore.clearActiveTurn(turnId);
        finish();
        return;
      }
      terminalSeen = true;
      const err = e as { code?: string; message?: string };
      if (clarification) {
        clarifyingTurns.delete(sessionId);
        // 后端入口一旦开始，调用失败时 best-effort 清除仍可能停放的同一回合，避免孤儿等待身份。
        void invoke("ag_cancel_turn", { sessionId, turnId }).catch(() => undefined);
      }
      if (decision) {
        decisionTurns.delete(decision.requestId);
        void invoke("ag_cancel_turn", { sessionId, turnId }).catch(() => undefined);
      }
      agStore.finishMessage(sessionId, assistantId);
      agStore.setTurnStatus(sessionId, turnId, err?.code?.includes("GATE") ? "gate_rejected" : "error");
      agStore.pushMessage(sessionId, {
        id: uid(),
        role: "assistant",
        content: err?.message ? `[${err.code ?? "?"}] ${err.message}` : String(e),
        error: true,
        turnId,
        relPath: turnActiveDocPath ?? null,
        turnStatus: err?.code?.includes("GATE") ? "gate_rejected" : "error",
      });
      agStore.clearActiveTurn(turnId);
      finish();
    };

    let eventWork = Promise.resolve();
    const consumeEvent = (ev: AgTurnEvent) => {
      eventWork = eventWork.then(async () => {
        if (terminalSeen || pausedForClarification || pausedForDecision) return;
        if (ev.turnId !== turnId || !agStore.acceptTurnEvent(turnId, ev.seq, ev.type)) return;
        switch (ev.type) {
        case "chunk":
          agStore.appendToMessage(sessionId, assistantId, ev.text);
          break;
        case "clarification_required":
          // 澄清是等待用户输入的非终态（BR-SYS-STATE-007）：不终结回合、不锁存终态。
          // 保存同 session/turn 的恢复身份；当前 Channel 在事件后关闭，补充输入经新 command 受控重入。
          agStore.appendToMessage(sessionId, assistantId, ev.text);
          agStore.setTurnStatus(sessionId, turnId, "clarifying");
          clarifyingTurns.set(sessionId, {
            sessionId,
            turnId,
            assistantMessageId: assistantId,
            prompt: effectivePrompt,
            references,
            mentions,
            blockRefs,
          });
          void enqueueTurnSave(sessionId, turnId, "draft", turnActiveDocPath);
          pausedForClarification = true;
          finish();
          break;
        case "done":
          terminalSeen = true;
          if (clarifyingTurns.get(sessionId)?.turnId === turnId) clarifyingTurns.delete(sessionId);
          if (decision && decisionTurns.get(decision.requestId)?.turnId === turnId) {
            decisionTurns.delete(decision.requestId);
          }
          agStore.finishMessage(sessionId, assistantId);
          agStore.setTurnStatus(sessionId, turnId, ev.stopReason === "cancelled" ? "cancelled" : "completed");
          void enqueueTurnSave(
            sessionId,
            turnId,
            ev.stopReason === "cancelled" ? "cancelled" : "completed",
            turnActiveDocPath,
          );
          // AG-1.0-IMP-TASKRUN-UI-01：回合终态后刷新 TaskRun 摘要（SM-AG-04 settle 已落库）。
          void refreshSessionTaskRuns(sessionId);
          // AG-1.0-TASKRUN-REWORK-INTENT-01：回合终态后刷新任务计划（复杂目标已建 plan）。
          void refreshSessionPlans(sessionId);
          if (ev.stopReason === "waiting_preview") {
            agStore.setCurrentAction({ action: "previewing", label: "等待编辑器应用修改建议" });
          } else {
            agStore.setCurrentAction(null);
          }
          finish();
          break;
        case "error":
          terminalSeen = true;
          if (clarifyingTurns.get(sessionId)?.turnId === turnId) clarifyingTurns.delete(sessionId);
          if (decision && decisionTurns.get(decision.requestId)?.turnId === turnId) {
            decisionTurns.delete(decision.requestId);
          }
          agStore.finishMessage(sessionId, assistantId);
          agStore.setTurnStatus(sessionId, turnId, ev.code.includes("GATE") || ev.code.includes("401") ? "gate_rejected" : "error");
          agStore.pushMessage(sessionId, {
            id: uid(),
            role: "assistant",
            content: `[${ev.code}] ${ev.message}`,
            error: true,
            turnId,
            relPath: turnActiveDocPath ?? null,
            turnStatus: ev.code.includes("GATE") || ev.code.includes("401") ? "gate_rejected" : "error",
          });
          void enqueueTurnSave(
            sessionId,
            turnId,
            ev.code.includes("GATE") || ev.code.includes("401") ? "gate_rejected" : "error",
            turnActiveDocPath,
            { code: ev.code, message: ev.message, retriable: ev.retriable },
          );
          // AG-1.0-IMP-TASKRUN-UI-01：回合失败后刷新 TaskRun 摘要（Failed 终态已落库）。
          void refreshSessionTaskRuns(sessionId);
          // AG-1.0-TASKRUN-REWORK-INTENT-01：回合失败后刷新任务计划。
          void refreshSessionPlans(sessionId);
          agStore.setCurrentAction(null);
          finish();
          break;
        case "step_progress":
          // AG-1.0-TASKRUN-REWORK-IMP-UI-01：实时当前动作（信息流底部状态条，BR-SYS-STATE-008 不伪造）。
          agStore.setCurrentAction({
            action: (["reading", "tool", "generating", "proposing", "deciding"].includes(ev.action)
              ? ev.action
              : "tool") as "reading" | "tool" | "generating" | "proposing" | "deciding",
            label: ev.label,
            detail: ev.detail,
          });
          break;
        case "decision_request_proposed":
          // AwaitingDecision 是原 AgentTurn 的非终态等待；保存稳定身份并释放输入门控。
          agStore.setPendingDecision({
            requestId: ev.requestId,
            kind: ev.kind,
            payload: ev.payload,
          });
          agStore.setCurrentAction({ action: "deciding", label: "等待你的决策" });
          decisionTurns.set(ev.requestId, {
            requestId: ev.requestId,
            sessionId,
            turnId,
            assistantMessageId: assistantId,
            prompt: effectivePrompt,
            references,
            mentions,
            blockRefs,
          });
          void enqueueTurnSave(sessionId, turnId, "draft", turnActiveDocPath);
          pausedForDecision = true;
          finish();
          break;
        case "decision_request_resolved":
          agStore.setPendingDecision(null);
          agStore.setCurrentAction(null);
          break;
        case "diff_proposed": {
          // AI 经 propose_edit 生成了待裁决修改（PI-AG-08C）：投影为 diff 卡片；
          // 当前编辑器命中的块级 diff 会先进入 ED 草稿，accept 只确认，save 再落盘。
          const detail = (await loadDiffDetails([ev.diffId]).catch(() => []))
            .find((candidate) => candidate.diff_id === ev.diffId);
          const projection = projectionFromDetail(ev.diffId, ev.relPath, detail);
          let title = `AI 建议修改 ${ev.relPath}`;
          let state: "pending" | "error" = "pending";
          let errorCode: string | undefined;
          const mappedBlockId =
            projection?.block_id ??
            (projection &&
            projection.patch_kind === "replace_block" &&
            typeof projection.block_index === "number" &&
            edStore.getSnapshot().openPath === ev.relPath
              ? (getActiveBlockSnapshotByIndex(projection.block_index)?.blockId ?? "")
              : "");
          if (!projection) {
            title = `⚠ 缺少修改载荷，请重试 · ${ev.relPath}`;
            state = "error";
            errorCode = "DE_DIFF_PROJECTION_MISSING";
          } else if (
            projection.patch_kind === "replace_block" &&
            !mappedBlockId &&
            typeof projection.block_index !== "number"
          ) {
            title = `⚠ 缺少精准编辑目标，请复核 · ${ev.relPath}`;
            state = "error";
            errorCode = "ED_DRAFT_TARGET_MISSING";
          }
          const styleOpsPayload = projection ? parseStyleOps(projection.style_ops) : [];
          const blockOpsPayload = projection
            ? parseStructuredBlockEditPayload(projection.block_ops)
            : undefined;
          const tableOpsPayload = projection ? parseTableOps(projection.table_ops) : undefined;
          const rawImageOpsPayload = projection
            ? parseImageEditPayload(projection.image_ops)
            : null;
          if (
            projection?.patch_kind === "image_ops"
            && rawImageOpsPayload?.operations.some((operation) => operation.op === "insert_image")
          ) {
            const ed = edStore.getSnapshot();
            const activeHtml = projection.rel_path === ed.openPath
              ? getActiveDocumentContent(ed.editFormat, ed.contentFormat)
              : null;
            if (activeHtml !== null) {
              try {
                await ensureDocumentMediaDisplayBase(
                  projection.rel_path,
                  activeHtml,
                  ed.baseline,
                );
              } catch {
                // 下方统一投影为不可裁决图片载荷错误；不把本地绝对媒体路径回注模型。
              }
            }
          }
          const imageOpsPayload = projection
            ? parseImageOps(projection.image_ops, projection.rel_path)
            : undefined;
          if (projection?.patch_kind === "style_ops" && styleOpsPayload.length === 0) {
            title = `⚠ 样式修改载荷无效，请重试 · ${ev.relPath}`;
            state = "error";
            errorCode = "AG_STYLE_OPERATION_INVALID";
          }
          if (projection?.patch_kind === "block_ops" && !blockOpsPayload) {
            title = `⚠ 结构化块修改载荷无效，请重试 · ${ev.relPath}`;
            state = "error";
            errorCode = "AG_BLOCK_OPERATION_INVALID";
          }
          if (projection?.patch_kind === "image_ops" && !imageOpsPayload) {
            title = `⚠ 图片修改载荷无效，请重试 · ${ev.relPath}`;
            state = "error";
            errorCode = "AG_IMAGE_OPERATION_INVALID";
          }
          deStore.addDiff({
            diffId: ev.diffId,
            sessionId,
            turnId,
            taskRunId: ev.taskRunId,
            anchorMessageId: assistantId,
            createdAt: Date.now(),
            blockId: mappedBlockId,
            blockIndex: projection?.block_index ?? null,
            relPath: ev.relPath,
            locationLabel: projection?.location_label ?? "未知位置",
            patchKind: projection?.patch_kind,
            draftMode: projection?.draft_mode,
            snapshotSource: projection?.snapshot_source,
            baseSessionVersion: projection?.base_session_version ?? undefined,
            baseContentHash: projection?.base_content_hash ?? undefined,
            previewContinuationRequired: projection?.preview_checkpoint_required === true,
            title,
            before: projection?.before_text ?? "",
            after: sanitizeInlineHtml(projection?.after_text ?? ""),
            state,
            errorCode,
            // propose_edit 只创建 pending proposal；已有 LogicalState 的预览成功后才会记录已投影。
            draftPreviewState: projection?.draft_mode === "current_editor"
              ? (state === "pending" ? "queued" : "failed")
              : undefined,
            // 已打开非激活会话的预览同样保留用户接受/拒绝，不能以 readonly 代替卡片裁决。
            readonly: false,
            styleOps: styleOpsPayload.length ? styleOpsPayload.map(summarizeStyleOp) : undefined,
            styleOpsPayload: styleOpsPayload.length ? styleOpsPayload : undefined,
            blockOpsPayload,
            blockOps: blockOpsPayload?.operations.map((operation) => (
              `${operation.blockId} · ${operation.expectedNodeType}`
            )),
            // AG-TASK-01 F1b：批次元数据透传（DiffCard 批次徽标）。
            batch: {
              id: ev.proposalBatchId,
              index: ev.groupIndex,
              total: ev.groupCount,
            },
            tableOpsPayload,
            tableOps: tableOpsPayload?.operations.map(summarizeTableOperation),
            imageOpsPayload,
            imageOps: imageOpsPayload?.operations.map(summarizeImageOperation),
          });
          // 连续 Proposal 接力（BR-SYS-STATE-008 / REQ-DE-009）：新 Proposal 基于最新 LogicalState，
          // 使同文档基于旧基准的 pending 卡失效（保留历史、不可裁决；不构成补丁栈/任务级回滚）。
          if (projection?.base_session_version != null || projection?.base_content_hash != null) {
            void expireStalePendingForPath(ev.relPath, ev.diffId, {
              sessionVersion: projection.base_session_version,
              contentHash: projection.base_content_hash,
            });
          }
          // 已打开非激活文档在 diff 返回时即受控预览到其 LogicalState；预览不自动接受。
          if (projection?.draft_mode === "open_session" && state === "pending") {
            void applyOpenSessionDiff(ev.diffId).then(() => void acknowledgePreviewAndContinue(ev.diffId));
          }
          // 消息片段（TERM-AG-011）：DiffCard 内嵌于回合内时序位置（紧随其产出工具/正文）。
          agStore.addDiffSegment(sessionId, assistantId, ev.diffId);
          void enqueueTurnSave(sessionId, turnId, "draft", turnActiveDocPath);
          break;
        }
        case "tool_request": {
          // 工具卡 executing（PI-AG-08D）。参数摘要作 detail。
          const p = typeof ev.params === "object" && ev.params ? JSON.stringify(ev.params) : "";
          const toolCallId = agStore.addToolCall(sessionId, ev.toolName, p && p !== "{}" ? p.slice(0, 80) : undefined, {
            id: ev.toolCallId,
            turnId,
            anchorMessageId: assistantId,
          });
          // 消息片段（TERM-AG-011）：工具调用内嵌于正文时序位置。
          agStore.addToolSegment(sessionId, assistantId, toolCallId);
          void enqueueTurnSave(sessionId, turnId, "draft", turnActiveDocPath);
          break;
        }
        case "tool_result": {
          // 工具卡 done/failed（PI-AG-08D）。结果摘要作 detail。
          const r = (ev.result ?? "").toString();
          agStore.resolveToolCallById(sessionId, ev.toolCallId, ev.success, r ? r.slice(0, 80) : undefined);
          void enqueueTurnSave(sessionId, turnId, "draft", turnActiveDocPath);
          break;
        }
        case "project_report": {
          agStore.projectProjectReport(turnId, ev.report);
          break;
        }
          default:
            break;
        }
      }).catch(failStart);
    };
    onEvent.onmessage = consumeEvent;

    void (async () => {
      let openDocSnapshots: OpenDocSnapshot[];
      try {
        openDocSnapshots = await collectOpenDocSnapshotsReady(turnActiveDocTarget);
      } catch (e) {
        if (decision) {
          let projected = false;
          try {
            await invoke("ag_cancel_turn", { sessionId, turnId });
            projected = await projectParkedCancellation(decision);
          } catch {
            decisionTurns.delete(decision.requestId);
          }
          if (projected) {
            finish();
            return;
          }
        }
        failStart(e);
        return;
      }
      const activeSnapshot = turnActiveDocPath
        ? openDocSnapshots.find((s) => s.relPath === turnActiveDocPath)
        : undefined;
      const activeDocContent = activeSnapshot?.content;
      let agentContextFacts: AgentContextFacts;
      try {
        const factBlockRefs = references.length ? [] : blockRefs;
        agentContextFacts = buildAgentContextFacts(
          turnId,
          sessionId,
          openDocSnapshots,
          references,
          factBlockRefs,
        );
      } catch (e) {
        failStart(e);
        return;
      }
      backendInvocationStarted = true;
      if (clarification) clarifyingTurns.delete(sessionId);
      if (decision) decisionTurns.delete(decision.requestId);
      try {
        await invoke("ag_start_turn", {
          turnId,
          sessionId,
          runtimeProfile: AGENT_EDIT_RUNTIME_PROFILE,
          prompt: effectivePrompt,
          provider: opts?.provider,
          model: opts?.model,
          activeDocPath: turnActiveDocPath,
          activeDocContent,
          activeDocFormat: activeSnapshot?.editFormat ?? turnActiveDocTarget.editFormat, // text|html（docx 块编辑用，PI-AG-08F）
          openDocSnapshots,
          agentContextFacts,
          history, // 对话历史（PI-AG-09B，后端按预算保留最近）
          mentions: references.length ? [] : mentions, // 兼容旧路径；新路径走 references 防重复注入
          blockRefs: references.length ? [] : blockRefs, // 兼容旧路径；新路径走 references 防重复注入
          references, // 用户显式 ReferencePayload（A-AG-I-P-01 v1.6）
          continuationTaskRunId: opts?.continuation?.taskRunId,
          continuationDecisionRequestId: opts?.continuation?.decisionRequestId,
          continuationPreviewDiffId: opts?.continuation?.previewDiffId,
          clarificationResume: Boolean(clarification),
          decisionResume: Boolean(decision),
          onEvent,
        });
        await eventWork;
        if (!terminalSeen && !pausedForClarification && !pausedForDecision) {
          const recovered = await listTurnEvents(turnId, agStore.lastTurnEventSeq(turnId));
          for (const event of recovered) consumeEvent(event);
          await eventWork;
        }
        if (!terminalSeen && !pausedForClarification && !pausedForDecision) {
          failStart({
            code: "AG_TURN_TERMINAL_MISSING",
            message: "Agent 回合结束但没有可恢复的终态事件",
          });
        }
      } catch (error) {
        failStart(error);
      }
    })();
  });
}

/** 取消后端真实活动回合；成功返回只表示取消信号已送达，最终状态以 Channel 终态为准。 */
export async function cancelActiveTurn(): Promise<void> {
  const snapshot = agStore.getSnapshot();
  const clarification = clarifyingTurns.get(snapshot.activeSessionId);
  const decision = [...decisionTurns.values()]
    .find((candidate) => candidate.sessionId === snapshot.activeSessionId);
  const parked = clarification ?? decision;
  const active = snapshot.activeTurn ?? parked;
  if (!active) return;
  try {
    await invoke("ag_cancel_turn", {
      sessionId: active.sessionId,
      turnId: active.turnId,
    });
    if (parked?.turnId === active.turnId) {
      const projected = await projectParkedCancellation(parked);
      if (!projected) {
        throw {
          code: "AG_TURN_TERMINAL_MISSING",
          message: "取消已返回，但没有可恢复的回合终态事件",
        };
      }
    }
  } catch (error) {
    const code = (error as { code?: string })?.code;
    if (code !== "AG_TURN_NOT_ACTIVE") throw error;
    if (parked?.turnId === active.turnId) {
      const projected = await projectParkedCancellation(parked);
      if (!projected) throw error;
    }
  }
}

async function projectParkedCancellation(parked: ClarifyingTurnIdentity | DecisionTurnIdentity): Promise<boolean> {
  const events = await listTurnEvents(
    parked.turnId,
    agStore.lastTurnEventSeq(parked.turnId),
  );
  for (const event of events) {
    if (
      event.turnId !== parked.turnId ||
      !agStore.acceptTurnEvent(event.turnId, event.seq, event.type)
    ) {
      continue;
    }
    if (event.type === "done" && event.stopReason === "cancelled") {
      if (clarifyingTurns.get(parked.sessionId)?.turnId === parked.turnId) {
        clarifyingTurns.delete(parked.sessionId);
      }
      if ("requestId" in parked) {
        decisionTurns.delete(parked.requestId);
        if (agStore.getSnapshot().pendingDecision?.requestId === parked.requestId) {
          agStore.setPendingDecision(null);
        }
      }
      agStore.finishMessage(parked.sessionId, parked.assistantMessageId);
      agStore.setTurnStatus(parked.sessionId, parked.turnId, "cancelled");
      agStore.setCurrentAction(null);
      void enqueueTurnSave(parked.sessionId, parked.turnId, "cancelled");
      return true;
    }
  }
  return false;
}
