import { Channel, invoke } from "@tauri-apps/api/core";
import { agStore, type AgentTurnStatus, type AgMode, type AgSession, type ChatMessage, type ToolCallView } from "../store/agStore";
import { edStore, type ContentFormat, type EditFormat, type OpenDocSnapshot } from "../store/edStore";
import { deStore, type DiffCardState, type DiffView } from "../store/deStore";
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
import { loadDiffDetails, loadPersistedDiffs, type DiffDetailDto } from "./deService";
import type { MessageContentPart, ReferencePayload } from "./referencePayload";
import { parseStyleOps, type StyleOp } from "./edStyleOps";
import {
  parseStructuredBlockEditPayload,
  projectAgentEditableBlocks,
} from "./edBlockOps";

/**
 * AG 模块 Service 层。
 * PI-AG-WIRE-01：经独立 async 命令 `ag_start_turn` + Tauri `Channel<AgTurnEvent>` 接**真实流式**
 * （替换桩 dispatch 回复 + 前端逐字模拟）。门禁前置（X-SEQ-001）由后端 SM-AG-01 ensure_gate_passed
 * 保证；模型经本地 dev GW（cloud/dev-gateway），客户端不持 provider 密钥（X-FORBID-001）。
 */

/** 与后端 `ipc::AgTurnEvent`（serde tag="type" snake_case）逐字段对齐（A-AG-I-P-01 §2.1）。 */
export type AgTurnEvent =
  | { type: "chunk"; turn_id: string; text: string }
  | { type: "tool_request"; turn_id: string; tool_name: string; params: unknown }
  | { type: "tool_result"; turn_id: string; tool_name: string; result: string; success: boolean }
  | { type: "diff_proposed"; turn_id: string; diff_id: string; rel_path: string }
  | { type: "done"; turn_id: string; stop_reason: string }
  | { type: "error"; turn_id: string; code: string; message: string; retriable: boolean };

type AgentEditPatchKind = "replace_block" | "block_ops" | "replace_document" | "style_ops" | "table_ops" | "image_ops";

interface AgentDiffProjection {
  diff_id: string;
  rel_path: string;
  patch_kind: AgentEditPatchKind;
  block_id?: string | null;
  block_index?: number | null;
  before_text: string;
  after_text: string;
  location_label: string;
  draft_mode: "current_editor" | "open_session" | "background";
  snapshot_source: "active_editor" | "active_editor_session" | "open_editor_session" | "disk_snapshot";
  base_session_version?: number | null;
  base_content_hash?: string | null;
  style_ops?: unknown;
  block_ops?: unknown;
  table_ops?: unknown;
  image_ops?: unknown;
}

let seq = 0;
const uid = () => `m-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value && typeof value === "object" && !Array.isArray(value));
}

function parseDiffProjectionResult(result: string): AgentDiffProjection | null {
  try {
    const parsed = JSON.parse(result);
    if (!isRecord(parsed) || !isRecord(parsed.projection)) return null;
    const p = parsed.projection;
    if (
      typeof p.diff_id !== "string" ||
      typeof p.rel_path !== "string" ||
      (p.patch_kind !== "replace_block" && p.patch_kind !== "block_ops" && p.patch_kind !== "replace_document" && p.patch_kind !== "style_ops" && p.patch_kind !== "table_ops" && p.patch_kind !== "image_ops") ||
      typeof p.before_text !== "string" ||
      typeof p.after_text !== "string" ||
      typeof p.location_label !== "string"
    ) {
      return null;
    }
    return {
      diff_id: p.diff_id,
      rel_path: p.rel_path,
      patch_kind: p.patch_kind,
      block_id: typeof p.block_id === "string" ? p.block_id : null,
      block_index: typeof p.block_index === "number" ? p.block_index : null,
      before_text: p.before_text,
      after_text: p.after_text,
      location_label: p.location_label,
      draft_mode: p.draft_mode === "open_session" ? "open_session" : p.draft_mode === "background" ? "background" : "current_editor",
      snapshot_source: p.snapshot_source === "active_editor_session"
        ? "active_editor_session"
        : p.snapshot_source === "open_editor_session"
          ? "open_editor_session"
          : p.snapshot_source === "disk_snapshot"
            ? "disk_snapshot"
            : "active_editor",
      base_session_version: typeof p.base_session_version === "number" ? p.base_session_version : null,
      base_content_hash: typeof p.base_content_hash === "string" ? p.base_content_hash : null,
      style_ops: p.style_ops,
      block_ops: p.block_ops,
      table_ops: p.table_ops,
      image_ops: p.image_ops,
    };
  } catch {
    return null;
  }
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
}

interface AgentTurnPayloadV1 {
  version: 1;
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
  payload: AgentTurnPayloadV1;
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
    if (turn.payload.assistantMessage) {
      const assistant = messageFromPersisted(turn.payload.assistantMessage, turn);
      assistantAnchor = assistant.id;
      messages.push(assistant);
    } else {
      const fallback = turnFailureMessage(turn);
      if (fallback) {
        assistantAnchor = fallback.id;
        messages.push(fallback);
      }
    }
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
      if (!anchorMessageId || !appendToolToMessage(messages, anchorMessageId, tool)) {
        toolCalls.push(tool);
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

export async function ensureAgentSessionTurns(sessionId: string): Promise<void> {
  const session = agStore.getSnapshot().sessions.find((s) => s.id === sessionId);
  if (!session || session.turnsLoaded) return;
  const turns = await dispatchCall<AgentTurnDto[]>("ag.list_turns", { sessionId });
  const projected = projectTurns(turns);
  await hydrateDiffRefsForTurns(turns);
  agStore.hydrateSessionTurns(sessionId, projected.messages, projected.toolCalls);
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

export function setAgentMode(mode: AgMode): void {
  agStore.setMode(mode);
  void persistAgentSessionsMetadata();
}

const turnSaveChain = new Map<string, Promise<void>>();

export async function deleteAgentSession(sessionId: string): Promise<void> {
  if (agStore.getSnapshot().sending) {
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
      const payload: AgentTurnPayloadV1 = {
        version: 1,
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
      await dispatchCall("ag.save_turn", { sessionId, turnId, status, relPath, payload });
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
 * rules:    BR-AG-STATE-003, BR-AG-VERIFY-001
 * term_ref: TERM-AG-003, TERM-AG-004
 * boundary: in=AssistReq
 *           | out=AssistResponse(candidatesOrInternalFailureReasonPreserved)
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
  },
): Promise<void> {
  const trimmed = prompt.trim();
  const references = opts?.references ?? [];
  if (!trimmed && references.length === 0) return;

  // 关闭屏障（BR-SYS-STATE-004，APP-CLOSE-DIFF-LIFECYCLE-01）：关闭请求建立后拒绝新回合，
  // 避免清点后又产生新的待裁决 diff。
  if (closeStore.getSnapshot().phase !== "idle") {
    console.error("ag_start_turn blocked: application close in progress");
    return;
  }

  // 回合归属会话：发起时的活动会话（此后用户切标签不影响本回合投影目标）。
  const snap = agStore.getSnapshot();
  const sessionId = snap.activeSessionId;
  const session = snap.sessions.find((s) => s.id === sessionId);
  const turnId = `t-${Date.now()}-${(seq += 1)}`;
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
    .filter((m) => !m.error && m.turnStatus !== "draft" && m.content.trim())
    .slice(-30)
    .map((m) => ({
      role: m.role,
      content: m.content,
      relPath: m.relPath ?? null,
      turnId: m.turnId ?? null,
    }));

  agStore.pushMessage(sessionId, {
    id: uid(),
    role: "user",
    content: trimmed,
    parts: opts?.displayParts,
    turnId,
    relPath: turnActiveDocPath ?? null,
    turnStatus: "draft",
  });
  agStore.clearToolCalls(sessionId); // 新回合清空旧工具卡（PI-AG-08D）
  agStore.setSending(true); // 乐观置 true 即时门控防双发；活动/释放由 SM-AG-01 广播投影（Task 2-E）

  const assistantId = uid();
  agStore.pushMessage(sessionId, {
    id: assistantId,
    role: "assistant",
    content: "",
    streaming: true,
    turnId,
    relPath: turnActiveDocPath ?? null,
    turnStatus: "draft",
  });
  void persistAgentSessionsMetadata();
  void enqueueTurnSave(sessionId, turnId, "draft", turnActiveDocPath);

  const onEvent = new Channel<AgTurnEvent>();
  // propose_edit 的权威投影来自后端 tool_result.projection；diff_proposed 只负责通知展示。
  const pendingDiffProjections = new Map<string, AgentDiffProjection>();

  await new Promise<void>((resolve) => {
    let settled = false;
    const finish = () => {
      if (settled) return;
      settled = true;
      // sending 的释放由 SM-AG-01 终态广播驱动（Completed/TurnError，Task 2-E）；此处仅结束等待。
      resolve();
    };
    const failStart = (e: unknown) => {
      const err = e as { code?: string; message?: string };
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
      void enqueueTurnSave(
        sessionId,
        turnId,
        err?.code?.includes("GATE") ? "gate_rejected" : "error",
        turnActiveDocPath,
        {
          code: err?.code ?? "AG_INVOKE_ERROR",
          message: err?.message ?? String(e),
          retriable: true,
        },
      );
      agStore.setSending(false);
      finish();
    };

    let eventWork = Promise.resolve();
    onEvent.onmessage = (ev) => {
      eventWork = eventWork.then(async () => {
        switch (ev.type) {
        case "chunk":
          agStore.appendToMessage(sessionId, assistantId, ev.text);
          break;
        case "done":
          agStore.finishMessage(sessionId, assistantId);
          agStore.setTurnStatus(sessionId, turnId, ev.stop_reason === "cancelled" ? "cancelled" : "completed");
          void enqueueTurnSave(
            sessionId,
            turnId,
            ev.stop_reason === "cancelled" ? "cancelled" : "completed",
            turnActiveDocPath,
          );
          finish();
          break;
        case "error":
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
          finish();
          break;
        case "diff_proposed": {
          // AI 经 propose_edit 生成了待裁决修改（PI-AG-08C）：投影为 diff 卡片；
          // 当前编辑器命中的块级 diff 会先进入 ED 草稿，accept 只确认，save 再落盘。
          const projection = pendingDiffProjections.get(ev.diff_id);
          pendingDiffProjections.delete(ev.diff_id);
          let title = `AI 建议修改 ${ev.rel_path}`;
          let state: "pending" | "error" = "pending";
          let errorCode: string | undefined;
          const mappedBlockId =
            projection?.block_id ??
            (projection &&
            projection.patch_kind === "replace_block" &&
            typeof projection.block_index === "number" &&
            edStore.getSnapshot().openPath === ev.rel_path
              ? (getActiveBlockSnapshotByIndex(projection.block_index)?.blockId ?? "")
              : "");
          if (!projection) {
            title = `⚠ 缺少修改载荷，请重试 · ${ev.rel_path}`;
            state = "error";
            errorCode = "DE_DIFF_PROJECTION_MISSING";
          } else if (
            projection.patch_kind === "replace_block" &&
            !mappedBlockId &&
            typeof projection.block_index !== "number"
          ) {
            title = `⚠ 缺少精准编辑目标，请复核 · ${ev.rel_path}`;
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
            title = `⚠ 样式修改载荷无效，请重试 · ${ev.rel_path}`;
            state = "error";
            errorCode = "AG_STYLE_OPERATION_INVALID";
          }
          if (projection?.patch_kind === "block_ops" && !blockOpsPayload) {
            title = `⚠ 结构化块修改载荷无效，请重试 · ${ev.rel_path}`;
            state = "error";
            errorCode = "AG_BLOCK_OPERATION_INVALID";
          }
          if (projection?.patch_kind === "image_ops" && !imageOpsPayload) {
            title = `⚠ 图片修改载荷无效，请重试 · ${ev.rel_path}`;
            state = "error";
            errorCode = "AG_IMAGE_OPERATION_INVALID";
          }
          deStore.addDiff({
            diffId: ev.diff_id,
            sessionId,
            turnId,
            anchorMessageId: assistantId,
            createdAt: Date.now(),
            blockId: mappedBlockId,
            blockIndex: projection?.block_index ?? null,
            relPath: ev.rel_path,
            locationLabel: projection?.location_label ?? "未知位置",
            patchKind: projection?.patch_kind,
            draftMode: projection?.draft_mode,
            snapshotSource: projection?.snapshot_source,
            baseSessionVersion: projection?.base_session_version ?? undefined,
            baseContentHash: projection?.base_content_hash ?? undefined,
            title,
            before: projection?.before_text ?? "",
            after: sanitizeInlineHtml(projection?.after_text ?? ""),
            state,
            errorCode,
            readonly: projection?.draft_mode === "open_session",
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
          });
          // 审计修复（2026-07-26 · D1）：open_session diff 不再 auto-accept，等用户裁决
          void enqueueTurnSave(sessionId, turnId, "draft", turnActiveDocPath);
          break;
        }
        case "tool_request": {
          // 工具卡 executing（PI-AG-08D）。参数摘要作 detail。
          const p = typeof ev.params === "object" && ev.params ? JSON.stringify(ev.params) : "";
          agStore.addToolCall(sessionId, ev.tool_name, p && p !== "{}" ? p.slice(0, 80) : undefined, {
            turnId,
            anchorMessageId: assistantId,
          });
          void enqueueTurnSave(sessionId, turnId, "draft", turnActiveDocPath);
          break;
        }
        case "tool_result": {
          // 工具卡 done/failed（PI-AG-08D）。结果摘要作 detail。
          const r = (ev.result ?? "").toString();
          if (ev.tool_name === "propose_edit" && ev.success) {
            const projection = parseDiffProjectionResult(r);
            if (projection) pendingDiffProjections.set(projection.diff_id, projection);
          }
          if ((ev.tool_name === "move_doc" || ev.tool_name === "create_folder") && ev.success) {
            void loadDocs();
          }
          agStore.resolveToolCall(sessionId, ev.tool_name, ev.success, r ? r.slice(0, 80) : undefined, turnId);
          void enqueueTurnSave(sessionId, turnId, "draft", turnActiveDocPath);
          break;
        }
          default:
            break;
        }
      }).catch(failStart);
    };

    void (async () => {
      let openDocSnapshots: OpenDocSnapshot[];
      try {
        openDocSnapshots = await collectOpenDocSnapshotsReady(turnActiveDocTarget);
      } catch (e) {
        failStart(e);
        return;
      }
      const activeSnapshot = turnActiveDocPath
        ? openDocSnapshots.find((s) => s.relPath === turnActiveDocPath)
        : undefined;
      const activeDocContent = activeSnapshot?.content;
      invoke("ag_start_turn", {
        turnId,
        sessionId,
        prompt: trimmed,
        provider: opts?.provider,
        model: opts?.model,
        activeDocPath: turnActiveDocPath,
        activeDocContent,
        activeDocFormat: activeSnapshot?.editFormat ?? turnActiveDocTarget.editFormat, // text|html（docx 块编辑用，PI-AG-08F）
        openDocSnapshots,
        history, // 对话历史（PI-AG-09B，后端按预算保留最近）
        mentions: references.length ? [] : (opts?.mentions ?? []), // 兼容旧路径；新路径走 references 防重复注入
        blockRefs: references.length ? [] : (opts?.blockRefs ?? []), // 兼容旧路径；新路径走 references 防重复注入
        references, // 用户显式 ReferencePayload（A-AG-I-P-01 v1.6）
        onEvent,
      }).catch(failStart);
    })();
  });
}
