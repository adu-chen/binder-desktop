import { Channel, invoke } from "@tauri-apps/api/core";
import type { ToolCallReceiptV1 } from "../store/agStore";
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
import { modelStore } from "../store/modelStore";

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
import type { AgentFailureDiagnostic } from "../store/agStore";

// Accept only the safe versioned projection; never spread opaque error objects into history.
function failureDiagnostic(value: unknown): AgentFailureDiagnostic | undefined {
  if (!value || typeof value !== "object") return undefined;
  const d = value as Record<string, unknown>;
  const stages = ["context_assembly", "runtime", "gate", "finalizing", "tool_validation", "tool_execution", "operation_post_processing", "frontend_start", "frontend_invoke", "event_recovery"];
  const categories = ["core_context", "model_protocol", "authorization", "professional_capability", "ui_projection", "unknown"];
  if (d.version !== 1 || d.rootCauseStatus !== "unknown" || typeof d.observedAt !== "string" || typeof d.category !== "string" || !stages.includes(d.observedAt) || !categories.includes(d.category)) return undefined;
  return { version: 1, observedAt: d.observedAt as AgentFailureDiagnostic["observedAt"], category: d.category as AgentFailureDiagnostic["category"], rootCauseStatus: "unknown" };
}

function diagnosticSummary(d: AgentFailureDiagnostic | undefined): string {
  return d ? `（诊断阶段：${d.observedAt}；分类：${d.category}；根因未确认）` : "";
}

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
  | { type: "tool_result"; toolCallId: string; toolName: string; result: string; success: boolean; evidenceIds: string[]; diagnostic?: AgentFailureDiagnostic; executionReceipt?: unknown }
  | { type: "project_report"; report: ProjectReportView }
  | { type: "diff_proposed"; diffId: string; proposalBatchId: string; groupIndex: number; groupCount: number; relPath: string; taskRunId?: string }
  | { type: "step_progress"; action: string; label: string; detail?: string }
  | { type: "decision_request_proposed"; requestId: string; kind: string; payload: unknown }
  | { type: "decision_request_resolved"; requestId: string; outcome: string }
  | { type: "clarification_required"; text: string }
  | { type: "done"; stopReason: string }
  | { type: "error"; code: string; message: string; retriable: boolean; diagnostic?: AgentFailureDiagnostic }
);

export const AGENT_EDIT_RUNTIME_PROFILE = "agent-edit-1.0" as const;

/** Fail closed on a present but invalid receipt; absence alone is legacy-compatible. */
export function validateToolCallReceipt(raw: unknown, identity: {
  workspaceId: string; sessionId: string; turnId: string; toolCallId: string; toolName: string;
}, success: boolean, evidenceIds: string[]): ToolCallReceiptV1 {
  const invalid = () => { throw new Error("AG_TOOL_RECEIPT_INVALID"); };
  if (!raw || typeof raw !== "object" || Array.isArray(raw)) return invalid();
  const r = raw as Record<string, unknown>;
  if (r.version !== "tool-call-receipt-v1" || r.attempt !== 1) return invalid();
  for (const [key, value] of Object.entries(identity)) if (!value || r[key] !== value) return invalid();
  for (const key of ["capabilityId", "grantId"]) if (typeof r[key] !== "string" || !r[key]) return invalid();
  if (!["succeeded", "rejected", "failed"].includes(String(r.executionStatus))
    || !["completed", "waiting", "blocked"].includes(String(r.completionStatus))
    || !["not_applicable", "not_started", "proposal_created", "resource_prepared", "resource_executed", "unknown"].includes(String(r.effectStatus))
    || !["none", "await_user", "respond_blocked", "retry_explicitly", "inspect_effect"].includes(String(r.nextAction))) return invalid();
  if (success !== (r.executionStatus === "succeeded")) return invalid();
  if (!Array.isArray(r.evidenceIds) || r.evidenceIds.some(id => typeof id !== "string")
    || JSON.stringify(r.evidenceIds) !== JSON.stringify(evidenceIds)) return invalid();
  if (!Array.isArray(r.references) || r.references.some(ref => !ref || typeof ref !== "object"
    || !["decision_request", "pending_diff", "workspace_operation_plan"].includes(ref.kind) || typeof ref.id !== "string" || !ref.id)) return invalid();
  const failure = r.failure as Record<string, unknown> | null;
  if (success ? failure !== null : !failure || typeof failure.code !== "string" || !failure.code
    || typeof failure.retriable !== "boolean" || typeof failure.reauthRequired !== "boolean") return invalid();
  if (r.completionStatus === "waiting" && (r.nextAction !== "await_user"
    || !r.references.length || !["not_started", "proposal_created"].includes(String(r.effectStatus)))) return invalid();
  if (r.completionStatus === "completed" && (r.nextAction !== "none"
    || !["not_applicable", "resource_prepared", "resource_executed"].includes(String(r.effectStatus)) || !success)) return invalid();
  if (r.effectStatus === "resource_executed" && (r.toolName !== "execute_workspace_operation"
    || r.executionStatus !== "succeeded" || r.completionStatus !== "completed"
    || !r.references.some(ref => ref.kind === "workspace_operation_plan"))) return invalid();
  // Project only declared safe fields, not arbitrary document/error payload additions.
  return {
    ...identity, version: "tool-call-receipt-v1", attempt: 1,
    capabilityId: r.capabilityId as string, grantId: r.grantId as string,
    executionStatus: r.executionStatus as ToolCallReceiptV1["executionStatus"],
    completionStatus: r.completionStatus as ToolCallReceiptV1["completionStatus"],
    effectStatus: r.effectStatus as ToolCallReceiptV1["effectStatus"],
    nextAction: r.nextAction as ToolCallReceiptV1["nextAction"],
    references: r.references.map(ref => ({ kind: ref.kind, id: ref.id })),
    evidenceIds: [...evidenceIds],
    failure: failure ? { code: failure.code as string, retriable: failure.retriable as boolean, reauthRequired: failure.reauthRequired as boolean } : null,
  };
}

function receiptSummary(receipt: ToolCallReceiptV1): string {
  if (receipt.effectStatus === "unknown") return "操作效果待核对；不会自动重试。";
  if (receipt.effectStatus === "proposal_created") return "已生成修改建议；不代表预览、接受或保存。";
  if (receipt.completionStatus === "waiting") return "等待用户裁决；专业修改尚未执行。";
  if (receipt.completionStatus === "blocked") return `请求受阻${receipt.failure ? ` [${receipt.failure.code}]` : ""}；不代表业务成功。`;
  if (receipt.effectStatus === "resource_prepared") return "资源已准备；不代表文档已保存。";
  if (receipt.effectStatus === "resource_executed") return "资源操作已执行；不代表文档修改已接受或保存。";
  return "只读调用完成。";
}
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
  activeTarget?: ActiveDocTarget;
  contextRevision?: number;
  modelBinding?: { provider: string; model: string };
  /** Original request cache for an internal fragment; never a new UI user message. */
  requestSnapshot?: PersistedChatMessage;
}
/** 按 session 保留可受控重入的非终态 AgentTurn；不得按“最近回合”跨 session 猜测。 */
const clarifyingTurns = new Map<string, ClarifyingTurnIdentity>();
interface DecisionTurnIdentity extends ClarifyingTurnIdentity {
  requestId: string;
  workspaceId: string;
}
/** 按 DecisionRequest 保留 AwaitingDecision 的原回合身份；裁决只恢复该回合，不创建替代回合。 */
const decisionTurns = new Map<string, DecisionTurnIdentity>();
const savedGroupWaits = new Map<string, { path: string; workspaceId: string; sessionId: string; turnId: string }>();
export async function refreshSettledResourceGroup(diffId: string | undefined): Promise<void> {
  const diff = deStore.getSnapshot().diffs.find(diff => diff.diffId === diffId);
  for (const [requestId,wait] of savedGroupWaits) {
    const owner = decisionTurns.get(requestId);
    if (owner && wait.turnId === diff?.turnId && wait.path === diff.relPath) {
      await projectParkedCancellation(owner);
    }
  }
}
export async function resumeSavedResourceGroups(path: string | undefined): Promise<void> {
  const workspaceId = workspaceShell.getSnapshot().activeWorkspace?.workspaceId;
  for (const [requestId, wait] of savedGroupWaits) {
    if (wait.path !== path || wait.workspaceId !== workspaceId
      || !decisionTurns.has(requestId) || decisionsInFlight.has(requestId)) continue;
    // Saving is an explicit user action. Backend still checks the original decision,
    // saved bundle, turn cancellation, all source baselines and a single effect lease.
    await resolveDecisionAndContinue(requestId, "confirm");
  }
}
export async function continueSavedResourceGroup(requestId: string): Promise<string> {
  if (!savedGroupWaits.has(requestId) || !decisionTurns.has(requestId)) {
    throw new Error("原等待已结束或未在本进程恢复，不会重放操作；请先核对执行效果。");
  }
  const resolution=await resolveDecisionAndContinue(requestId,"confirm");
  return resolution.workspaceOperationReceipt?.final_status === "awaiting_content"
    ? "初始内容尚未保存；后续操作未执行。请先接受并保存新文件。"
    : "已核对原操作组；后续状态以原回合及执行回执为准。";
}
if (typeof window !== "undefined") {
  window.addEventListener("binder:diff-settled", event => { void refreshSettledResourceGroup((event as CustomEvent<{diffId?: string}>).detail?.diffId).catch(error => console.error("[ag-result] parked result query failed",error)); });
  window.addEventListener("binder:doc-saved", event => { void resumeSavedResourceGroups((event as CustomEvent<{path?: string}>).detail?.path).catch(error => console.error("[ag-result] saved group continuation failed",error)); });
}
const decisionsInFlight = new Set<string>();
const decisionEffectUnknown = new Set<string>();
async function boundFailureDelivery<T>(work: Promise<T>): Promise<T> {
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    return await Promise.race([work,new Promise<never>((_,reject)=>{
      timer=setTimeout(()=>reject(new Error("原回合结果收束/补流等待超时；后端可能已提交，请核对历史，不会重发操作。")),2000);
    })]);
  } finally { if(timer!==undefined) clearTimeout(timer); }
}
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
  expectedBatch?: { id: string; groupIndex: number; groupCount: number },
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
  const proposalBatch = proposalBatchFromProjection(p, expectedBatch);
  if (!proposalBatch) return null;
  return {
    ...p,
    diff_id: diffId,
    proposal_batch_id: proposalBatch.id,
    group_index: proposalBatch.groupIndex,
    group_count: proposalBatch.groupCount,
    rel_path: detail?.target.rel_path ?? relPath,
    patch_kind: p.patch_kind,
    before_text: p.before_text ?? "",
    after_text: p.after_text ?? "",
    location_label: p.location_label ?? "未知位置",
    draft_mode: p.draft_mode ?? "current_editor",
    snapshot_source: p.snapshot_source ?? "active_editor",
  };
}

function proposalBatchFromProjection(
  projection: Pick<Partial<DiffProjectionDto>, "proposal_batch_id" | "group_index" | "group_count">,
  expected?: { id: string; groupIndex: number; groupCount: number },
): { id: string; groupIndex: number; groupCount: number } | null {
  const id = projection.proposal_batch_id;
  const groupIndex = projection.group_index;
  const groupCount = projection.group_count;
  if (
    typeof id !== "string"
    || !id.trim()
    || !Number.isSafeInteger(groupIndex)
    || !Number.isSafeInteger(groupCount)
    || (groupIndex ?? 0) < 1
    || (groupCount ?? 0) < 1
    || (groupIndex ?? 0) > (groupCount ?? 0)
  ) return null;
  if (
    expected
    && (id !== expected.id || groupIndex !== expected.groupIndex || groupCount !== expected.groupCount)
  ) return null;
  return { id, groupIndex: groupIndex!, groupCount: groupCount! };
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

interface AgentContextFactsV2 {
  version: "agent-context-facts-v2";
  workspaceId: string;
  sessionId: string;
  turnId: string;
  contextRevision: number;
  activeTarget: { relPath: string; loadSeq: number } | null;
  openResources: {
    relPath: string;
    availability: "not_requested" | "loading" | "ready" | "unavailable";
    reasonCode: string | null;
    snapshot: Omit<AgentContextDocumentFact, "relPath"> | null;
  }[];
  selections: (AgentContextFacts["selections"][number] & { validation: "unverified" | "valid" | "stale"; contextRevision: number })[];
  proposalFacts: AgentContextFacts["proposalFacts"];
}

interface AgentContextRequest {
  requestId: string;
  workspaceId: string;
  sessionId: string;
  turnId: string;
  contextRevision: number;
  paths: string[];
  allDocuments: boolean;
}

/**
 * @GOV
 * codes:    BR-AG-DATA-008-DATA-AG-AG-TURN-054, BR-SYS-DATA-008-DATA-AG-AG-TURN-054
 * type:     DATA
 * chain:    AG-TURN, AG-TOOL
 * rules:    BR-AG-DATA-008, BR-SYS-DATA-008
 * term_ref: TERM-AG-020
 * boundary: in=FrozenAgentTurnTargetCurrentHostResourcesAndRequestedPaths | out=AgentContextFactsV2AndRequestedLogicalStateSnapshots | does_not=waitForUnrelatedEditorOrReadDiskOrChangeFrozenTarget
 */
function collectAgentContextV2(
  turnId: string, sessionId: string, target: ActiveDocTarget, revision: number,
  references: ReferencePayload[], blockRefs: ClarifyingTurnIdentity["blockRefs"],
  previous?: AgentContextFactsV2, requested: string[] = [], allDocuments = false,
): { facts: AgentContextFactsV2; snapshots: AgentOpenDocSnapshot[] } {
  const workspaceId = workspaceShell.getSnapshot().activeWorkspace?.workspaceId;
  if (!workspaceId || (previous && previous.workspaceId !== workspaceId)) throw snapshotError("AG_CONTEXT_FACTS_IDENTITY_MISMATCH", "工作区已切换，请重新提交");
  const initial = edStore.getSnapshot();
  // ED keeps closed sessions as caches. Only tabs and the live active path prove an open resource.
  const openPaths = new Set([...tabsStore.getSnapshot().tabs.filter((t) => t.kind === "doc").map((t) => t.path), ...(initial.openPath ? [initial.openPath] : [])]);
  if (!previous && target.relPath) openPaths.add(target.relPath);
  const allPaths = new Set([...openPaths, ...(previous?.openResources.map((r) => r.relPath) ?? [])]);
  const wanted = new Set(allDocuments ? [...openPaths] : requested);
  const pendingBinding = getActiveEditorBinding();
  if (pendingBinding && wanted.has(pendingBinding.relPath)) flushBoundActiveEditor(initial);
  const snapshots: AgentOpenDocSnapshot[] = [];
  const resources: AgentContextFactsV2["openResources"] = [];
  for (const relPath of allPaths) {
    let snapshot: AgentOpenDocSnapshot | undefined;
    let reasonCode: string | null = null;
    let availability: AgentContextFactsV2["openResources"][number]["availability"] = "not_requested";
    if (!openPaths.has(relPath)) {
      availability = "unavailable"; reasonCode = "AG_CONTEXT_RESOURCE_CLOSED";
    } else if (wanted.has(relPath)) {
      const ed = edStore.getSnapshot();
      const session = ed.sessions[relPath];
      if (relPath === target.relPath && relPath === ed.openPath && ed.loadSeq !== target.loadSeq) {
        availability = "unavailable"; reasonCode = "AG_CONTEXT_TARGET_STALE";
      } else if (!session || session.content === null || session.converting) {
        availability = "loading"; reasonCode = "AG_ACTIVE_SESSION_LOADING";
      } else if (relPath === ed.openPath) {
        const binding = getActiveEditorBinding();
        if (bindingMatches(binding, relPath, ed.loadSeq, session.editFormat, session.contentFormat)) {
          const content = getActiveDocumentContent(session.editFormat, session.contentFormat);
          if (content !== null) {
            edStore.updateContent(relPath, content);
            snapshot = attachActiveStructuredSnapshots(edStore.getOpenSnapshots().filter((s) => s.relPath === relPath), relPath)[0];
          } else { availability = "unavailable"; reasonCode = "AG_ACTIVE_SNAPSHOT_REQUIRED"; }
        } else if (session.contentFormat === "html") {
          availability = "loading"; reasonCode = "AG_ACTIVE_SESSION_LOADING";
        } else {
          snapshot = edStore.getOpenSnapshots("active_editor_session").find((s) => s.relPath === relPath);
        }
      } else {
        snapshot = edStore.getOpenSnapshots("active_editor_session").find((s) => s.relPath === relPath);
      }
      if (snapshot) availability = "ready";
    }
    if (snapshot) snapshots.push(snapshot);
    resources.push({ relPath, availability, reasonCode, snapshot: snapshot ? {
      source: snapshot.source, editFormat: snapshot.editFormat, contentFormat: snapshot.contentFormat,
      dirty: snapshot.dirty, baseline: snapshot.baseline == null ? null : String(snapshot.baseline),
      sessionVersion: snapshot.sessionVersion, contentHash: snapshot.contentHash,
    } : null });
  }
  const projection = buildAgentContextFacts(turnId, sessionId, [], references, blockRefs);
  return { snapshots, facts: {
    version: "agent-context-facts-v2", workspaceId, sessionId, turnId, contextRevision: revision,
    activeTarget: target.relPath ? { relPath: target.relPath, loadSeq: target.loadSeq } : null,
    openResources: resources,
    selections: projection.selections.map((s) => {
      const snapshot = snapshots.find((d) => d.relPath === s.relPath);
      const block = snapshot?.blockSnapshots?.find((b) => b.blockId === s.blockId);
      const ref = references.find((r) => r.kind === "selection" && r.relPath === s.relPath && r.blockId === s.blockId);
      const originalText = ref?.kind === "selection" ? ref.text : blockRefs.find((r) => r.relPath === s.relPath && r.blockId === s.blockId)?.text;
      const selectedText = block && s.start !== undefined && s.end !== undefined ? block.text.slice(s.start, s.end) : block?.text;
      const validation = !snapshot ? "unverified" : block && originalText === selectedText ? "valid" : "stale";
      return { ...s, validation, contextRevision: revision };
    }),
    proposalFacts: projection.proposalFacts,
  } };
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
  failure?: ChatMessage["turnFailure"];
  incompleteReason?: "stream_interrupted" | "app_closed";
}

interface AgentTurnPayloadV2 {
  version: 2;
  lastEventSeq: number;
  userMessage: PersistedChatMessage;
  assistantMessage?: PersistedChatMessage;
  toolSummaries: { id: string; name: string; inputSummary?: string; resultSummary?: string; state: string; anchorMessageId?: string }[];
  diffRefs: { diffId: string; relPath: string; anchorMessageId?: string }[];
  failure?: ChatMessage["turnFailure"];
  incompleteReason?: "stream_interrupted" | "app_closed";
}

interface AgentTurnDto {
  /** Backend runtime provenance; display only, never continuation permission. */
  internal_continuation?: boolean;
  turn_id: string;
  session_id: string;
  workspace_id?: string;
  execution_facts?: unknown[];
  rel_path?: string | null;
  status: AgentTurnStatus;
  payload: AgentTurnPayloadV1 | AgentTurnPayloadV2;
  created_at: number;
  updated_at: number;
}

/** V1 仅在读取边界转换为当前 V2 展示投影；转换结果只存在内存，不回写或延续旧字段。 */
function toCurrentTurnProjection(turn: AgentTurnDto): AgentTurnDto & { payload: AgentTurnPayloadV2 } {
  const payload = turn.payload;
  return {
    ...turn,
    payload: {
      version: 2,
      lastEventSeq: Number.isSafeInteger(payload.lastEventSeq) && (payload.lastEventSeq ?? 0) >= 0
        ? payload.lastEventSeq ?? 0
        : 0,
      userMessage: payload.userMessage,
      assistantMessage: payload.assistantMessage,
      toolSummaries: payload.toolSummaries ?? [],
      diffRefs: payload.diffRefs ?? [],
      failure: payload.failure,
      incompleteReason: payload.incompleteReason,
    },
  };
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
    turnFailure: turn.status === "draft" && turn.payload.failure ? { ...turn.payload.failure, diagnostic: failureDiagnostic(turn.payload.failure.diagnostic) } : undefined,
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
    content: content + diagnosticSummary(failureDiagnostic(failure?.diagnostic)),
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
    if (!turn.internal_continuation) messages.push(messageFromPersisted(turn.payload.userMessage, turn));
    let assistantAnchor: string | undefined;
    let assistant: ChatMessage | null = null;
    if (turn.payload.assistantMessage) {
      assistant = messageFromPersisted(turn.payload.assistantMessage, turn);
      assistantAnchor = assistant.id;
      messages.push(assistant);
      // A diagnostic must not replace the independently persisted visible answer.
      if (turn.payload.failure && !assistant.error && (turn.status === "error" || turn.status === "gate_rejected")) {
        const diagnostic = turnFailureMessage(turn);
        if (diagnostic) messages.push(diagnostic);
      }
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
  const proposalBatch = projection ? proposalBatchFromProjection(projection) : null;
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
    title: proposalBatch
      ? `历史修改建议 ${detail.target?.rel_path ?? ref.relPath}`
      : `历史修改建议批次信息不可用 ${detail.target?.rel_path ?? ref.relPath}`,
    before: projection?.before_text ?? "",
    after: projection?.after_text ?? "",
    state: proposalBatch ? diffStateFromStatus(detail.status) : "error",
    errorCode: proposalBatch ? undefined : "DE_PROPOSAL_BATCH_INVALID",
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
    proposalBatch: proposalBatch ?? undefined,
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

/** 后端 TurnEvent 派生 TaskRunProjectionV2 DTO。 */
export interface TaskStepDto {
  step_id: string;
  kind: string;
  label: string;
  status: TaskStepViewStatus;
  depends_on: string[];
  source_event_id: string;
  source_event_seq: number;
  result?: string | null;
}

export interface TaskRunDto {
  task_run_id: string;
  source_turn_ids: string[];
  through_seq: number;
  goal: string;
  status: TaskRunViewStatus;
  observed_steps: TaskStepDto[];
  waiting_on: string[];
  current_route_revision?: number | null;
  candidate_steps: TaskStepDto[];
  created_at: number;
  updated_at: number;
}

function projectTaskStep(step: TaskStepDto) {
  return {
    stepId: step.step_id,
    kind: step.kind,
    label: step.label,
    status: step.status,
    dependsOn: step.depends_on,
    sourceEventId: step.source_event_id,
    sourceEventSeq: step.source_event_seq,
    result: step.result ?? undefined,
  };
}

/** 查询会话受限任务实例（SM-AG-04 状态/步骤，倒序）；映射为 TaskRunView 投影到 agStore。 */
export async function listSessionTaskRuns(sessionId: string): Promise<TaskRunView[]> {
  const runs = await dispatchCall<TaskRunDto[]>("ag.list_task_runs", { sessionId });
  return runs.map((r) => ({
    taskRunId: r.task_run_id,
    sourceTurnIds: r.source_turn_ids,
    throughSeq: r.through_seq,
    goal: r.goal,
    status: r.status,
    observedSteps: r.observed_steps.map(projectTaskStep),
    waitingOn: r.waiting_on,
    currentRouteRevision: r.current_route_revision ?? undefined,
    candidateSteps: r.candidate_steps.map(projectTaskStep),
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

/** 任务级裁决（TERM-AG-017，三层分离）：confirm/cancel 后仍由原 AgentTurn 重新过 Gate，
 * TaskRun 只投影已发生的裁决与后续活动，不因裁决预先进入执行态。 */
export async function resolveDecision(
  requestId: string,
  outcome: "confirm" | "cancel",
): Promise<{
  taskRunId: string;
  status: string;
  workspaceOperationReceipt?: WorkspaceOperationReceiptDto;
  createDocumentDiffId?: string;
}> {
  const workspaceId = workspaceShell.getSnapshot().activeWorkspace?.workspaceId;
  const key = `${workspaceId ?? ""}:${requestId}`;
  if (decisionEffectUnknown.has(key)) {
    throw { code: "AG_DECISION_EFFECT_UNKNOWN", message: "原裁决效果待核对，禁止重复提交。请核对原回合执行效果。", retriable: false };
  }
  const deadlineAt = Date.now() + 30_000;
  const ownerSessionId = decisionTurns.get(requestId)?.sessionId ?? activeSession(agStore.getSnapshot()).id;
  const assertDecisionContext = () => {
    if (decisionEffectUnknown.has(key) || Date.now() >= deadlineAt) {
      decisionEffectUnknown.add(key);
      throw {code: "AG_DECISION_TIMEOUT_EFFECT_UNKNOWN", message: "原裁决等待已结束，禁止迟到结果继续后续操作。", retriable: false};
    }
    if (workspaceShell.getSnapshot().activeWorkspace?.workspaceId !== workspaceId) {
      throw {code: "AG_DECISION_CONTEXT_CHANGED", message: "工作区已切换，原裁决不得在新工作区继续。"};
    }
  };
  const bounded = async <T>(work: Promise<T>, submitted: boolean): Promise<T> => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    try {
      return await Promise.race([work, new Promise<never>((_, reject) => {
        timer = setTimeout(() => {
          if (submitted) decisionEffectUnknown.add(key);
          reject({ code: submitted ? "AG_DECISION_TIMEOUT_EFFECT_UNKNOWN" : "AG_DECISION_PREPARATION_TIMEOUT",
            message: submitted ? "裁决等待超时；服务端可能仍在执行。请核对执行效果，不会自动继续或重发。" : "裁决前置准备超时；尚未提交裁决，请就绪后重新操作。", retriable: !submitted });
        }, Math.max(0, deadlineAt - Date.now()));
      })]);
    } finally { if (timer !== undefined) clearTimeout(timer); }
  };
  const pendingDecision = agStore.getSnapshot().pendingDecision;
  const pendingPayload = pendingDecision?.requestId === requestId
    && pendingDecision.payload
    && typeof pendingDecision.payload === "object"
    ? pendingDecision.payload as Record<string, unknown>
    : null;
  if (outcome === "confirm" && typeof pendingPayload?.workspaceOperationPlanId === "string") {
    await bounded(flushOpenEditorSessionSnapshots(), false);
  }
  if (workspaceShell.getSnapshot().activeWorkspace?.workspaceId !== workspaceId) {
    throw { code: "AG_DECISION_CONTEXT_CHANGED", message: "工作区已切换，未提交原决策。" };
  }
  const result = await bounded(dispatchCall<{
    task_run_id: string;
    status: string;
    workspace_operation_receipt?: WorkspaceOperationReceiptDto | null;
    workspace_operation_items?: WorkspaceOperationItemDto[] | null;
    create_document_bundle?: CreateDocumentBundleDto | null;
    already_resolved?: boolean;
  }>("ag.resolve_decision", {
    requestId,
    outcome,
  }), true);
  try {
    if (workspaceShell.getSnapshot().activeWorkspace?.workspaceId !== workspaceId) {
      throw { code: "AG_DECISION_CONTEXT_CHANGED", message: "原决策已提交，但工作区已切换；未在新工作区继续或投影回执，请返回原工作区核查。" };
    }
    let createDocumentDiffId: string | undefined;
    if (result.workspace_operation_receipt) await bounded(loadDocs(), true);
    assertDecisionContext();
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
      await bounded(reconcileWorkspaceOperationTabs(
        effectiveItems,
        Boolean(result.create_document_bundle),
        assertDecisionContext,
      ), true);
    }
    if (result.create_document_bundle) {
      createDocumentDiffId = await bounded(projectCreateDocumentBundle(result.create_document_bundle, ownerSessionId, assertDecisionContext), true);
    }
    assertDecisionContext();
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
  assertContext: () => void,
): Promise<void> {
  for (const item of [...items].sort((left, right) => left.seq - right.seq)) {
    assertContext();
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
        assertContext();
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

async function projectCreateDocumentBundle(bundle: CreateDocumentBundleDto, ownerSessionId: string, assertContext: () => void): Promise<string | undefined> {
  assertContext();
  const relPath = bundle.target.rel_path;
  tabsStore.openDoc(relPath);
  try {
    await openDocument(relPath);
  } catch (openError) {
    assertContext();
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
  assertContext();
  const session = edStore.getSnapshot().sessions[relPath];
  if (!session || session.content === null) {
    throw { code: "ED_CREATE_BUNDLE_SESSION_MISSING", message: "新文档已创建，但编辑器未建立 LogicalState" };
  }
  let projected: CreateDocumentProjectionResult;
  try {
    projected = await dispatchCall<CreateDocumentProjectionResult>("ws.project_create_document_bundle", {
      planId: bundle.plan_id,
      sessionId: ownerSessionId,
      relPath,
      beforeSnapshot: session.content,
      baseSessionVersion: session.sessionVersion,
      baseContentHash: session.contentHash,
    });
  } catch (projectionError) {
    assertContext();
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
  assertContext();
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
    sessionId: ownerSessionId,
    turnId: projected.source_turn_id,
    taskRunId: projected.task_run_id,
    anchorMessageId: agStore.getSnapshot().sessions.find(candidate => candidate.id === ownerSessionId)
      ?.messages.find(message => message.role === "assistant" && message.turnId === projected.source_turn_id)?.id,
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
  const anchor = deStore.getSnapshot().diffs.find(diff => diff.diffId === projected.diff_id)?.anchorMessageId;
  if (anchor) agStore.addDiffSegment(ownerSessionId, anchor, projected.diff_id);
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
    window.dispatchEvent(new CustomEvent("binder:diff-settled", { detail: { diffId } }));
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
    | "awaiting_content"
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
const previewContinuations = new Map<string, ClarifyingTurnIdentity & { workspaceId: string }>();
const previewAckEffectUnknown = new Set<string>();
const PREVIEW_ACK_DEADLINE_MS = 15_000;

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
  if (previewAckInFlight.has(diffId) || previewAckEffectUnknown.has(diffId)) return;
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
  const workspaceId = workspaceShell.getSnapshot().activeWorkspace?.workspaceId;
  if (!workspaceId) return;
  const action = {
    sessionId: diff.sessionId,
    turnId: diff.turnId,
    action: "previewing" as const,
    label: "已应用预览，正在校验最新文档状态",
  };
  previewAckInFlight.add(diffId);
  try {
    agStore.setCurrentAction(action);
    if (diff.draftAfterContent !== undefined && session.content !== diff.draftAfterContent) {
      await dispatchCall("de.expire_diff", { diffId, reason: "ED_DRAFT_BASE_ADVANCED" });
      if (workspaceShell.getSnapshot().activeWorkspace?.workspaceId === workspaceId) {
        deStore.expireLocal(diffId, { expiredReason: "document_revision_advanced", errorCode: "ED_DRAFT_BASE_ADVANCED" });
      }
      throw new Error("AG_PREVIEW_CONTINUATION_STALE：预览基线已变化，未继续执行。");
    }
    let deadline: ReturnType<typeof setTimeout> | undefined;
    const receipt = await Promise.race([
      invoke<{ task_run_id: string; ready: boolean; awaiting_save?: boolean }>("ag_ack_preview", {
        taskRunId: diff.taskRunId,
        sessionId: diff.sessionId,
        turnId: diff.turnId,
        diffId,
        relPath: diff.relPath,
        sessionVersion: session.sessionVersion,
        contentHash: session.contentHash,
      }),
      new Promise<never>((_, reject) => {
        deadline = setTimeout(() => reject({
          code: "AG_PREVIEW_ACK_TIMEOUT_EFFECT_UNKNOWN",
          message: "预览回执在 15 秒内未返回；服务端可能已提交。已停止自动继续及本进程内重复确认，请核对任务状态后再决定下一步。",
        }), PREVIEW_ACK_DEADLINE_MS);
      }),
    ]).finally(() => { if (deadline !== undefined) clearTimeout(deadline); });
    const current = edStore.getSnapshot().sessions[diff.relPath];
    const currentDiff = deStore.getSnapshot().diffs.find((candidate) => candidate.diffId === diffId);
    if (workspaceShell.getSnapshot().activeWorkspace?.workspaceId !== workspaceId) {
      throw new Error("AG_PREVIEW_CONTEXT_CHANGED：工作区已切换，未继续执行。");
    }
    if (!receipt?.ready || receipt.task_run_id !== diff.taskRunId) {
      throw new Error("AG_PREVIEW_ACK_INVALID：预览回执不匹配，未继续执行。");
    }
    if (receipt.awaiting_save || [...savedGroupWaits.values()].some(wait => wait.turnId === diff.turnId && wait.path === diff.relPath && wait.workspaceId === workspaceId)) {
      agStore.setCurrentAction({ ...action, action: "deciding", label: "初始内容已预览；等待你接受并保存新文件，任务尚未完成" });
      return;
    }
    if (currentDiff?.state !== "pending" || !currentDiff.draftApplied
      || currentDiff.sessionId !== diff.sessionId || currentDiff.taskRunId !== diff.taskRunId
      || currentDiff.turnId !== diff.turnId || currentDiff.relPath !== diff.relPath
      || current?.sessionVersion !== session.sessionVersion || current?.contentHash !== session.contentHash) {
      throw new Error("AG_PREVIEW_CONTINUATION_STALE：文档或修改状态已变化，未继续执行。");
    }
    if (closeStore.getSnapshot().phase !== "idle") {
      throw new Error("AG_PREVIEW_CLOSE_PENDING：应用正在关闭，未继续执行。");
    }
    const origin = previewContinuations.get(diffId);
    if (!origin || origin.turnId !== diff.turnId || origin.sessionId !== diff.sessionId
      || origin.workspaceId !== workspaceId || !origin.modelBinding?.provider || !origin.modelBinding.model) {
      throw new Error("AG_PREVIEW_ORIGIN_UNAVAILABLE：原请求或模型绑定不可用，未创建替代请求；请核对任务状态。");
    }
    await startTurn(origin.prompt, {
      autoContinue: true,
      mentions: [diff.relPath],
      continuation: { taskRunId: diff.taskRunId, previewDiffId: diffId },
    });
  } catch (error) {
    // 不自动重试：传输失败不能证明服务端未提交。错误仅投影到仍存在的原工作区会话。
    if ((error as { code?: string })?.code === "AG_PREVIEW_ACK_TIMEOUT_EFFECT_UNKNOWN") {
      previewAckEffectUnknown.add(diffId);
    }
    console.error("[ag-preview] continuation acknowledgement rejected", error);
    if (workspaceShell.getSnapshot().activeWorkspace?.workspaceId === workspaceId
      && agStore.getSnapshot().sessions.some((candidate) => candidate.id === diff.sessionId)) {
      const failure = error as { code?: string; message?: string };
      agStore.pushMessage(diff.sessionId, {
        id: `preview-error-${diffId}-${Date.now()}`,
        role: "assistant",
        content: `预览继续未完成；不代表修改已接受或文件已保存。${failure?.code ? `[${failure.code}] ` : ""}${failure?.message ?? String(error)}`,
        error: true,
      });
    }
  } finally {
    agStore.clearCurrentAction(action);
    previewContinuations.delete(diffId);
    previewAckInFlight.delete(diffId);
  }
}

/**
 * @GOV
 * codes: BR-SYS-STATE-007-EFFECT-AG-AG-TURN-060, BR-SYS-STATE-008-EFFECT-AG-AG-TURN-060
 * type: EFFECT
 * chain: AG-TURN, IPC-CALL
 * rules: BR-SYS-STATE-007, BR-SYS-STATE-008
 * term_ref: TERM-AG-017, TERM-AG-001
 * boundary: in=DecisionRequestIdAndUserOutcome | out=SingleInFlightResolutionAndIdentityBoundAgentTurnContinuation | does_not=grantPermissionOrFabricateHistoricalTurn
 */
export async function resolveDecisionAndContinue(
  requestId: string,
  outcome: "confirm" | "cancel",
): Promise<Awaited<ReturnType<typeof resolveDecision>>> {
  if (decisionsInFlight.has(requestId)) {
    throw { code: "AG_DECISION_IN_FLIGHT", message: "该决策正在提交，请等待结果。" };
  }
  const workspaceId = workspaceShell.getSnapshot().activeWorkspace?.workspaceId;
  const pending = agStore.getSnapshot().pendingDecision;
  const decisionTurn = decisionTurns.get(requestId);
  if (decisionTurn && decisionTurn.workspaceId !== workspaceId) {
    throw { code: "AG_DECISION_CONTEXT_CHANGED", message: "请返回该决策所属工作区后操作。" };
  }
  decisionsInFlight.add(requestId);
  let resolvedTaskRunId: string | undefined;
  try {
    const resolution = await resolveDecision(requestId, outcome);
    resolvedTaskRunId = resolution.taskRunId;
    if (pending?.requestId === requestId) agStore.clearPendingDecision(pending);
    if (resolution.workspaceOperationReceipt?.final_status === "awaiting_content") {
      const diff = deStore.getSnapshot().diffs.find(diff => diff.diffId === resolution.createDocumentDiffId);
      if (decisionTurn && diff?.relPath && workspaceId) {
        const firstWait = !savedGroupWaits.has(requestId);
        savedGroupWaits.set(requestId, { path: diff.relPath, workspaceId, sessionId: decisionTurn.sessionId, turnId: decisionTurn.turnId });
        agStore.setResourceSaveWait(decisionTurn.sessionId,decisionTurn.assistantMessageId,{requestId,path:diff.relPath});
        if (firstWait) agStore.appendToMessage(decisionTurn.sessionId, decisionTurn.assistantMessageId,
          "\n\n新文件资源已创建，初始内容仍需审阅与保存。等待你保存后才会重验原任务及剩余操作；不会自动接受或保存，也不会提前执行同组后续操作。接受不等于保存。");
        agStore.setCurrentAction({ sessionId: decisionTurn.sessionId, turnId: decisionTurn.turnId, action: "deciding", label: "等待保存新文件；原任务尚未完成" });
      }
      return resolution;
    }
    savedGroupWaits.delete(requestId);
    if (decisionTurn) agStore.setResourceSaveWait(decisionTurn.sessionId,decisionTurn.assistantMessageId);
    if (decisionTurn && resolution.workspaceOperationReceipt && !["completed","awaiting_content"].includes(resolution.workspaceOperationReceipt.final_status)) {
      await projectParkedCancellation(decisionTurn);
      return resolution;
    }
    if (
      decisionTurn
      && decisionTurns.get(requestId) === decisionTurn
      && agStore.getSnapshot().sessions.some((session) => session.id === decisionTurn.sessionId)
      && (resolution.status === "waiting" || resolution.status === "cancelled")
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
  } catch (cause) {
    const error = resolvedTaskRunId ? resolvedDecisionPostProcessingError(cause, resolvedTaskRunId) : cause;
    const failureCode=(error as {code?:string})?.code;
    let originalInteractionSettled=false;
    if (decisionTurn && ((error as {decisionResolved?:boolean})?.decisionResolved || failureCode === "AG_DECISION_TIMEOUT_EFFECT_UNKNOWN")) {
      try {
        const notice=await boundFailureDelivery(dispatchCall<{settled?:boolean;already_terminal?:boolean}>("ag.finish_operation_interaction",{
          workspaceId:decisionTurn.workspaceId,sessionId:decisionTurn.sessionId,turnId:decisionTurn.turnId,requestId,
          code:failureCode === "AG_DECISION_CONTEXT_CHANGED" || failureCode === "AG_DECISION_TIMEOUT_EFFECT_UNKNOWN" ? failureCode : "AG_DECISION_POSTPROCESSING_FAILED",
        }));
        originalInteractionSettled=notice?.settled===true || notice?.already_terminal===true;
      } catch (settlementError) { console.error("[ag-result] original interaction settlement unavailable",settlementError); }
    }
    if (decisionTurn && ((error as { decisionResolved?: boolean })?.decisionResolved || originalInteractionSettled)) {
      await boundFailureDelivery(projectParkedCancellation(decisionTurn)).catch(settlementError => { console.error("[ag-result] original interaction replay unavailable",settlementError); return false; });
    }
    if ((error as { decisionResolved?: boolean })?.decisionResolved && pending?.requestId === requestId) {
      agStore.clearPendingDecision(pending);
    }
    throw error;
  } finally {
    decisionsInFlight.delete(requestId);
  }
}

interface TaskPlanStepDto {
  step_id: string;
  kind: string;
  label: string;
  status: TaskStepViewStatus;
  depends_on: string[];
  source_event_id: string;
  source_event_seq: number;
  result?: string | null;
}

interface TaskPlanDto {
  plan_id: string;
  task_run_id: string;
  through_seq: number;
  route_revision: number;
  summary: string;
  candidate_steps: TaskPlanStepDto[];
}

/** 按 task_run_id 查询当前 route revision 的 TaskPlan 只读投影。 */
export async function listPlansByRun(taskRunId: string): Promise<TaskPlanView[]> {
  const plans = await dispatchCall<TaskPlanDto[]>("ag.list_plans", { taskRunId });
  return plans.map((p) => ({
    planId: p.plan_id,
    taskRunId: p.task_run_id,
    throughSeq: p.through_seq,
    routeRevision: p.route_revision,
    summary: p.summary,
    candidateSteps: p.candidate_steps.map(projectTaskStep),
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
  const persistedTurns = await dispatchCall<AgentTurnDto[]>("ag.list_turns", { sessionId });
  const turns = persistedTurns.map(toCurrentTurnProjection);
  const recoveredByTurn = new Map<string, AgTurnEvent[]>();
  for (const turn of turns) {
    const recovered = await listTurnEvents(turn.turn_id, 0).catch((error) => {
      console.error("ag.list_turn_events history recovery failed", turn.turn_id, error);
      return [];
    });
    const safe: AgTurnEvent[] = [];
    let seq = 0;
    let terminal = false;
    for (const event of [...(Array.isArray(recovered) ? recovered : [])].sort((a, b) => a.seq - b.seq)) {
      if (terminal || event.turnId !== turn.turn_id || !Number.isSafeInteger(event.seq) || event.seq <= seq) continue;
      seq = event.seq;
      safe.push(event);
      terminal = event.type === "done" || event.type === "error";
    }
    recoveredByTurn.set(turn.turn_id, safe);
    const previous = turn.payload.assistantMessage;
    if ((turn.status === "error" || turn.status === "gate_rejected") && (!previous?.content || previous.error)) {
      const body = safe.filter((event): event is Extract<AgTurnEvent, {type: "chunk"}> => event.type === "chunk")
        .map(event => event.text).join("");
      if (body) turn.payload.assistantMessage = {
        id: `hist-${turn.turn_id}-body`, role: "assistant", content: body, streaming: false, error: false,
      };
    }
  }
  const projected = projectTurns(turns);
  await hydrateDiffRefsForTurns(turns);
  agStore.hydrateSessionTurns(sessionId, projected.messages, projected.toolCalls);
  for (const turn of turns) {
    const events = recoveredByTurn.get(turn.turn_id) ?? [];
    let lastSafeSeq = 0;
    let eventTerminal = false;
    for (const event of [...events].sort((left, right) => left.seq - right.seq)) {
      if (
        event.turnId !== turn.turn_id
        || !Number.isSafeInteger(event.seq)
        || event.seq <= lastSafeSeq
        || eventTerminal
      ) continue;
      lastSafeSeq = event.seq;
      if (event.type === "project_report") {
        agStore.projectProjectReport(turn.turn_id, event.report);
      }
      if (event.type === "done") agStore.setTurnCompletionReason(sessionId, turn.turn_id, event.stopReason);
      if (event.type === "done" || event.type === "error") eventTerminal = true;
    }
    agStore.seedTurnEventProjection(turn.turn_id, Math.max(turn.payload.lastEventSeq, lastSafeSeq),
      eventTerminal || (turn.status !== "draft" && turn.status !== "clarifying"));
  }
  await refreshSessionTaskRuns(sessionId);
}

/** Channel 中断后的安全补流入口；事件仍由相同 seq reducer 去重与终态冻结。 */
export async function listTurnEvents(turnId: string, afterSeq = 0): Promise<AgTurnEvent[]> {
  return dispatchCall<AgTurnEvent[]>("ag.list_turn_events", { turnId, afterSeq });
}

/**
 * @GOV
 * codes: BR-AG-PERSIST-001-QUERY-AG-AG-TURN-067, BR-SYS-STATE-007-QUERY-AG-AG-TURN-067
 * type: QUERY
 * chain: AG-TURN
 * rules: BR-AG-PERSIST-001, BR-SYS-STATE-007
 * boundary: in=AgentTurnWorkspaceSessionIdentity | out=ValidatedTurnEventsToolCallReceiptsAndUnresolvedToolCallIds | does_not=RestoreAuthorizationExecuteToolsOrClaimAcceptanceOrSave
 */
/** 只查询持久执行事实；不复活授权、不恢复执行、不把专业对象引用推导为接受/保存。 */
export async function queryTurnExecutionEvidence(identity: {
  workspaceId: string; sessionId: string; turnId: string;
}): Promise<{ events: AgTurnEvent[]; receipts: ToolCallReceiptV1[]; unresolvedToolCallIds: string[] }> {
  const events = await listTurnEvents(identity.turnId);
  if (!Array.isArray(events)) throw new Error("AG_RETRY_EVIDENCE_UNAVAILABLE");
  let seq = 0;
  let terminal = false;
  const requests = new Map<string, string>();
  const seen = new Set<string>();
  const receipts: ToolCallReceiptV1[] = [];
  for (const event of events) {
    if (event.turnId !== identity.turnId || event.payloadVersion !== 1 || !Number.isSafeInteger(event.seq)
      || event.seq <= seq || terminal) throw new Error("AG_RETRY_EVIDENCE_UNAVAILABLE");
    seq = event.seq;
    if (event.type === "tool_request") {
      if (!event.toolCallId || seen.has(event.toolCallId)) throw new Error("AG_TOOL_RECEIPT_INVALID");
      seen.add(event.toolCallId);
      requests.set(event.toolCallId, event.toolName);
    }
    if (event.type === "tool_result" && event.executionReceipt !== undefined) {
      if (requests.get(event.toolCallId) !== event.toolName) throw new Error("AG_TOOL_RECEIPT_INVALID");
      receipts.push(validateToolCallReceipt(event.executionReceipt, { ...identity, toolCallId: event.toolCallId, toolName: event.toolName }, event.success, event.evidenceIds));
      requests.delete(event.toolCallId);
    }
    terminal = event.type === "done" || event.type === "error";
  }
  return { events, receipts, unresolvedToolCallIds: [...requests.keys()] };
}

/** Queries late effect settlement separately; never resumes or retries a capability. */
/**
 * @GOV
 * codes: BR-AG-PERSIST-001-QUERY-AG-AG-TURN-068, BR-SYS-STATE-007-QUERY-AG-AG-TURN-068
 * type: QUERY
 * chain: AG-TURN, IPC-CALL
 * rules: BR-AG-PERSIST-001, BR-SYS-STATE-007
 * boundary: in=WorkspaceScopedAgentSessionAndAgentTurnIdentity | out=ValidatedLateCapabilityEffectSummaryAndProfessionalObjectReferences | does_not=ExecuteCapabilityRestoreAuthorizationOrInferAcceptanceOrSave
 */
export async function inspectTurnEffects(sessionId: string, turnId: string): Promise<string> {
  const workspaceId = workspaceShell.getSnapshot().activeWorkspace?.workspaceId;
  if (!workspaceId) throw new Error("AG_CONTEXT_FACTS_WORKSPACE_REQUIRED");
  const turns = await dispatchCall<AgentTurnDto[]>("ag.list_turns", { sessionId });
  if (workspaceShell.getSnapshot().activeWorkspace?.workspaceId !== workspaceId) throw new Error("AG_RETRY_CONTEXT_CHANGED");
  if (!Array.isArray(turns)) throw new Error("AG_EFFECT_FACT_UNAVAILABLE");
  const turn = turns.find(turn => turn.turn_id === turnId && turn.session_id === sessionId && turn.workspace_id === workspaceId);
  if (!turn) throw new Error("AG_EFFECT_FACT_UNAVAILABLE");
  const rawFacts = turn.execution_facts ?? [];
  if (!Array.isArray(rawFacts)) throw new Error("AG_TOOL_RECEIPT_INVALID");
  const starts = new Map<string, Record<string, unknown>>();
  const settled = new Map<string, ToolCallReceiptV1>();
  for (const raw of rawFacts) {
    if (!raw || typeof raw !== "object" || Array.isArray(raw)) throw new Error("AG_TOOL_RECEIPT_INVALID");
    const fact = raw as Record<string, unknown>; const id = fact.identity as Record<string, unknown>;
    if (!id || id.workspaceId !== workspaceId || id.sessionId !== sessionId || id.turnId !== turnId
      || typeof id.toolCallId !== "string" || !id.toolCallId || typeof id.toolName !== "string" || !id.toolName
      || typeof id.capabilityId !== "string" || !id.capabilityId || typeof id.grantId !== "string" || !id.grantId || id.attempt !== 1)
      throw new Error("AG_TOOL_RECEIPT_INVALID");
    if (fact.phase === "started" && fact.receipt === null && !starts.has(id.toolCallId)) starts.set(id.toolCallId, id);
    else if (fact.phase === "settled" && !settled.has(id.toolCallId)) {
      const receipt = fact.receipt as Record<string, unknown>;
      const checked = validateToolCallReceipt(receipt, { workspaceId, sessionId, turnId, toolCallId: id.toolCallId, toolName: id.toolName },
        receipt?.executionStatus === "succeeded", []);
      if (checked.capabilityId !== id.capabilityId || checked.grantId !== id.grantId) throw new Error("AG_TOOL_RECEIPT_INVALID");
      settled.set(id.toolCallId, checked);
    } else throw new Error("AG_TOOL_RECEIPT_INVALID");
  }
  for (const [call, receipt] of settled) {
    const start = starts.get(call);
    if (!start || start.toolName !== receipt.toolName || start.grantId !== receipt.grantId || start.capabilityId !== receipt.capabilityId)
      throw new Error("AG_TOOL_RECEIPT_INVALID");
  }
  if (!starts.size) return "没有独立写能力执行记录；这不证明所有旧操作均无效果。不会重新执行。";
  return [...starts.keys()].map(call => {
    const receipt = settled.get(call);
    if (!receipt) return `${call}：未结算或中断，效果待核对；禁止重放。`;
    return `${call}：${receiptSummary(receipt)}${receipt.references.map(ref => ` ${ref.kind}=${ref.id}`).join("")}`;
  }).join("\n");
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
  failure?: ChatMessage["turnFailure"],
  requestSnapshot?: PersistedChatMessage,
): Promise<void> {
  const prev = turnSaveChain.get(turnId) ?? Promise.resolve();
  const next = prev
    .catch(() => undefined)
    .then(async () => {
      const session = agStore.getSnapshot().sessions.find((s) => s.id === sessionId);
      if (!session) return;
      const turnMessages = session.messages.filter((m) => m.turnId === turnId);
      const user = turnMessages.find((m) => m.role === "user") ?? requestSnapshot;
      if (!user) return;
      const assistants = turnMessages.filter((m) => m.role === "assistant");
      const newest = [...assistants].reverse();
      const assistant = newest.find(m => !m.error && m.content.trim())
        ?? newest.find(m => !m.error) ?? assistants[0];
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

const retryRequests = new Set<string>();

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
  opts?: Parameters<typeof startTurnWithOptions>[1],
): Promise<void> {
  const retryKey = opts?.retryOf ? JSON.stringify([opts.retryOf.sessionId, opts.retryOf.turnId]) : undefined;
  if (retryKey && retryRequests.has(retryKey)) {
    throw new Error("AG_RETRY_ALREADY_PENDING：该回合的重试核验或执行仍在进行，未重复提交。");
  }
  if (retryKey) retryRequests.add(retryKey);
  try {
    await startTurnWithOptions(prompt, opts);
  } finally {
    if (retryKey) retryRequests.delete(retryKey);
  }
}

async function startTurnWithOptions(
  prompt: string,
  opts?: {
    provider?: string;
    model?: string;
    mentions?: string[];
    blockRefs?: { relPath: string; blockId: string; text: string; start?: number; end?: number }[];
    references?: ReferencePayload[];
    displayParts?: MessageContentPart[];
    /** 显式重试必须先核实原回合；不复用其执行身份或裁决。 */
    retryOf?: { sessionId: string; turnId: string; continuationOnly?: boolean };
    /** 受控恢复入口；只供已确认的裁决或编辑器预览回执使用。 */
    autoContinue?: boolean;
    /** 已确认的 DecisionRequest 或预览回执所属任务。 */
    continuation?: { taskRunId: string; decisionRequestId?: string; previewDiffId?: string };
  },
): Promise<void> {
  const trimmed = prompt.trim();
  const submittedReferences = opts?.references ?? [];
  if (!trimmed && submittedReferences.length === 0) return;
  if (opts?.retryOf) {
    const origin = opts.retryOf;
    const workspaceId = workspaceShell.getSnapshot().activeWorkspace?.workspaceId;
    const assertOrigin = () => {
      const current = agStore.getSnapshot();
      const source = current.sessions.find((s) => s.id === origin.sessionId)?.messages
        .find((m) => m.role === "user" && m.turnId === origin.turnId);
      if (!workspaceId || workspaceShell.getSnapshot().activeWorkspace?.workspaceId !== workspaceId
        || current.activeSessionId !== origin.sessionId || current.activeTurn || current.sending
        || current.pendingDecision || clarifyingTurns.has(origin.sessionId)
        || opts.autoContinue || opts.continuation || !source
        || (origin.continuationOnly
          ? source.turnStatus !== "completed" || source.completionReason !== "budget_exhausted"
          : source.turnStatus !== "draft")
        || (source.relPath ?? null) !== edStore.getSnapshot().openPath
        || source.content !== prompt) {
        throw new Error("AG_RETRY_CONTEXT_CHANGED：原回合、会话或工作区状态已变化，未重新执行。");
      }
    };
    assertOrigin();
    let events: AgTurnEvent[];
    try {
      events = (await queryTurnExecutionEvidence({ workspaceId: workspaceId!, sessionId: origin.sessionId, turnId: origin.turnId })).events;
    } catch (error) {
      if (error instanceof Error && ["AG_RETRY_EVIDENCE_UNAVAILABLE", "AG_TOOL_RECEIPT_INVALID"].includes(error.message)) {
        throw new Error("AG_RETRY_EFFECT_UNVERIFIED：原回合证据不完整或身份不一致，未重新执行。");
      }
      throw new Error("AG_RETRY_EVIDENCE_UNAVAILABLE：无法读取原回合执行证据，未重新执行。");
    }
    if (!Array.isArray(events)) {
      throw new Error("AG_RETRY_EVIDENCE_UNAVAILABLE：原回合证据格式无效，未重新执行。");
    }
    assertOrigin();
    const readonlyTools = new Set(["read_doc", "list_docs", "workspace_glob", "search_text", "list_blocks", "list_tables", "list_images"]);
    let previousSeq = 0;
    const valid = events.length > 0 && events.every((event) => {
      if (event.turnId !== origin.turnId || event.payloadVersion !== 1
        || !Number.isSafeInteger(event.seq) || event.seq <= previousSeq) return false;
      previousSeq = event.seq;
      return true;
    });
    const terminal = events.filter((event) => event.type === "done" || event.type === "error");
    const last = events[events.length - 1];
    const safeTerminal = terminal.length === 1 && last === terminal[0]
      && (origin.continuationOnly
        ? last?.type === "done" && last.stopReason === "budget_exhausted"
        : (last?.type === "error" && last.retriable) || (last?.type === "done" && last.stopReason === "cancelled"));
    const possibleEffect = events.some((event) =>
      ((event.type === "tool_request" || event.type === "tool_result") && !readonlyTools.has(event.toolName))
      || event.type === "diff_proposed" || event.type === "decision_request_proposed"
      || event.type === "decision_request_resolved");
    if (!valid || !safeTerminal || previousSeq < agStore.lastTurnEventSeq(origin.turnId) || possibleEffect) {
      throw new Error("AG_RETRY_EFFECT_UNVERIFIED：原回合未确认结束或可能已产生操作；请先核对已有结果，未重新执行。");
    }
  }
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
  const preview = opts?.autoContinue && opts.continuation?.previewDiffId
    ? previewContinuations.get(opts.continuation.previewDiffId)
    : undefined;
  if (opts?.continuation?.previewDiffId && (!preview
    || preview.workspaceId !== workspaceShell.getSnapshot().activeWorkspace?.workspaceId
    || !snap.sessions.some((candidate) => candidate.id === preview.sessionId))) {
    throw new Error("AG_PREVIEW_CONTEXT_CHANGED：预览所属会话已失效，未继续执行。");
  }
  const sessionId = decision?.sessionId ?? preview?.sessionId ?? snap.activeSessionId;
  const session = snap.sessions.find((s) => s.id === sessionId);
  const workspaceId = workspaceShell.getSnapshot().activeWorkspace?.workspaceId;
  if (!workspaceId) throw new Error("AG_CONTEXT_FACTS_WORKSPACE_REQUIRED");
  const clarification = !opts?.autoContinue ? clarifyingTurns.get(sessionId) : undefined;
  if (clarification && opts?.continuation) {
    throw new Error("澄清恢复不能与任务裁决或预览续片段混用");
  }
  const turnId = clarification?.turnId ?? decision?.turnId ?? `t-${Date.now()}-${(seq += 1)}`;
  const selectedModel = clarification?.modelBinding ?? decision?.modelBinding ?? preview?.modelBinding
    ?? (clarification || decision || preview ? undefined : (opts?.provider && opts?.model
      ? { provider: opts.provider, model: opts.model }
      : modelStore.resolveFor("agentEdit")));
  if (!clarification && !decision && !preview && !selectedModel) throw new Error("AG_MODEL_SELECTION_INVALID：模型未选定，未发起请求。");
  if (!clarification && !decision && !preview && Boolean(opts?.provider) !== Boolean(opts?.model)) {
    throw new Error("AG_MODEL_SELECTION_INVALID：provider 与 model 必须同时指定。");
  }
  const effectivePrompt = decision?.prompt ?? preview?.prompt ?? (clarification?.pendingInput
    ? `${clarification.pendingInput}${trimmed ? `\n\n${trimmed}` : ""}`
    : opts?.retryOf?.continuationOnly
      ? `继续以下原请求尚未完成的部分。已有结果只作历史参考，当前对象须重新核验；不得重放既有副作用，不确定的效果须先核对。\n原请求：${trimmed}`
      : trimmed);
  const references = mergeReferencePayloads(
    clarification?.references ?? decision?.references ?? preview?.references ?? [],
    submittedReferences,
  );
  const mentions = [...new Set([
    ...(clarification?.mentions ?? decision?.mentions ?? preview?.mentions ?? []),
    ...(opts?.mentions ?? []),
  ])];
  const blockRefs = mergeLegacyBlockRefs(
    clarification?.blockRefs ?? decision?.blockRefs ?? preview?.blockRefs ?? [],
    opts?.blockRefs ?? [],
  );
  const turnEd = edStore.getSnapshot();
  const turnActiveDocTarget: ActiveDocTarget = decision?.activeTarget ?? clarification?.activeTarget ?? preview?.activeTarget ?? {
    relPath: turnEd.openPath,
    loadSeq: turnEd.loadSeq,
    editFormat: turnEd.editFormat,
    contentFormat: turnEd.contentFormat,
  };
  const turnActiveDocPath = turnActiveDocTarget.relPath ?? undefined;
  let contextRevision = decision?.contextRevision ?? clarification?.contextRevision ?? 0;

  // 本轮之前的对话历史（PI-AG-09B）：传后端按预算保留最近（drop 最旧）。取本轮 user 入栈前的快照。
  const history = (session?.messages ?? [])
    .filter((m) => m.turnId !== turnId && !m.error && m.turnStatus !== "draft" && m.content.trim()
      && !(m.role === "assistant" && (m.turnStatus === "error" || m.turnStatus === "gate_rejected")))
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
    // A preview is an internal continuation of the original request, not user input.
    // Keep its new execution identity without fabricating a role=user message.
    if (!preview) agStore.pushMessage(sessionId, {
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
  const requestSnapshot = clarification?.requestSnapshot ?? decision?.requestSnapshot
    ?? (preview ? { id: `u-${turnId}`, role: "user" as const, content: effectivePrompt } : undefined);
  const saveCurrentTurn = (status: AgentTurnStatus, relPath?: string | null, failure?: ChatMessage["turnFailure"]) =>
    enqueueTurnSave(sessionId, turnId, status, relPath, failure, requestSnapshot);

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
    const interruptStart = (cause: unknown) => {
      if (terminalSeen || pausedForClarification || pausedForDecision) return;
      // 安全透传真实原因（可诊断性修复）：后端命令错误不再被伪装成笼统“流被中断”。
      const causeObj = cause && typeof cause === "object"
        ? (cause as Record<string, unknown>)
        : null;
      const causeCode = typeof causeObj?.code === "string" && causeObj.code ? causeObj.code : undefined;
      const causeMessage = typeof causeObj?.message === "string" && causeObj.message ? causeObj.message : undefined;
      const failure = {
            code: causeCode ?? "AG_TURN_INTERRUPTED",
            message: causeMessage ?? "连接中断，本回合未完成，请重试。",
            retriable: causeObj?.retriable === true,
            diagnostic: {
              version: 1,
              observedAt: !backendInvocationStarted ? "frontend_start" : causeCode === "AG_TURN_TERMINAL_MISSING" ? "event_recovery" : "frontend_invoke",
              category: !backendInvocationStarted ? "core_context" : "unknown",
              rootCauseStatus: "unknown",
            } as AgentFailureDiagnostic,
          };
      console.error(`[ag-interrupt] turn_id=${turnId} code=${failure.code}`);
      if (clarification && !backendInvocationStarted) {
        // 快照/装配尚未越过后端入口：保持原回合非终态，允许用户修复编辑器状态后继续补充。
        pausedForClarification = true;
        agStore.setTurnStatus(sessionId, turnId, "clarifying");
        agStore.clearActiveTurn(turnId);
        finish();
        return;
      }
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
      // Promise/Channel 中断不是 AgentTurn 终态事实。保持 draft，允许用户显式以新 turn 重试；
      // 只有安全 TurnEvent 的 Done/Error 才能投影 completed/error/cancelled/gate_rejected。
      agStore.setTurnStatus(sessionId, turnId, "draft", failure);
      if (backendInvocationStarted) {
        // 诊断与模型正文分开；重启从 payload.failure 恢复到同一未完成提示。
        void saveCurrentTurn("draft", turnActiveDocPath, failure);
      }
      agStore.clearTurnAction(sessionId, turnId);
      agStore.clearActiveTurn(turnId);
      finish();
    };

    let eventWork = Promise.resolve();
    const consumeEvent = (ev: AgTurnEvent) => {
      eventWork = eventWork.then(async () => {
        if (terminalSeen || pausedForClarification || pausedForDecision) return;
        if (ev.turnId !== turnId || !agStore.acceptTurnEvent(turnId, ev.seq, ev.type)) return;
        // Receipt of a versioned event is not proof that a component rendered or a file was saved.
        // Never spread ev: it can contain prompt text, tool arguments, paths and document content.
        if (import.meta.env.DEV && ev.type !== "chunk") {
          console.debug("[AG-AG-TURN] frontend_accepted:", JSON.stringify({
            sessionId, turnId, eventId: ev.eventId, seq: ev.seq, eventType: ev.type,
          }));
        }
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
            activeTarget: turnActiveDocTarget,
            contextRevision,
            assistantMessageId: assistantId,
            prompt: effectivePrompt,
            references,
            mentions,
            blockRefs,
            modelBinding: selectedModel,
            requestSnapshot,
          });
          void saveCurrentTurn("draft", turnActiveDocPath);
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
          agStore.setTurnCompletionReason(sessionId, turnId, ev.stopReason);
          void saveCurrentTurn(
            ev.stopReason === "cancelled" ? "cancelled" : "completed",
            turnActiveDocPath,
          );
          // AG-1.0-IMP-TASKRUN-UI-01：回合终态后刷新 TaskRun 摘要（SM-AG-04 settle 已落库）。
          void refreshSessionTaskRuns(sessionId);
          // AG-1.0-TASKRUN-REWORK-INTENT-01：回合终态后刷新任务计划（复杂目标已建 plan）。
          void refreshSessionPlans(sessionId);
          if (ev.stopReason === "waiting_preview") {
            agStore.setCurrentAction({
              sessionId,
              turnId,
              action: "previewing",
              label: "等待编辑器应用修改建议",
            });
          } else {
            agStore.clearTurnAction(sessionId, turnId);
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
          agStore.setTurnStatus(sessionId, turnId, ev.code === "AG_GATE_DENIED" || ev.code === "AG_GATE_OFFLINE" ? "gate_rejected" : "error");
          agStore.pushMessage(sessionId, {
            id: uid(),
            role: "assistant",
            content: `[${ev.code}] ${ev.message}${diagnosticSummary(failureDiagnostic(ev.diagnostic))}`,
            error: true,
            turnId,
            relPath: turnActiveDocPath ?? null,
            turnStatus: ev.code === "AG_GATE_DENIED" || ev.code === "AG_GATE_OFFLINE" ? "gate_rejected" : "error",
          });
          void saveCurrentTurn(
            ev.code === "AG_GATE_DENIED" || ev.code === "AG_GATE_OFFLINE" ? "gate_rejected" : "error",
            turnActiveDocPath,
            { code: ev.code, message: ev.message, retriable: ev.retriable, diagnostic: failureDiagnostic(ev.diagnostic) },
          );
          // AG-1.0-IMP-TASKRUN-UI-01：回合失败后刷新 TaskRun 摘要（Failed 终态已落库）。
          void refreshSessionTaskRuns(sessionId);
          // AG-1.0-TASKRUN-REWORK-INTENT-01：回合失败后刷新任务计划。
          void refreshSessionPlans(sessionId);
          agStore.clearTurnAction(sessionId, turnId);
          finish();
          break;
        case "step_progress":
          // AG-1.0-TASKRUN-REWORK-IMP-UI-01：实时当前动作（信息流底部状态条，BR-SYS-STATE-008 不伪造）。
          agStore.setCurrentAction({
            sessionId,
            turnId,
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
            sessionId,
            turnId,
            requestId: ev.requestId,
            kind: ev.kind,
            payload: ev.payload,
          });
          agStore.setCurrentAction({ sessionId, turnId, action: "deciding", label: "等待你的决策" });
          decisionTurns.set(ev.requestId, {
            requestId: ev.requestId,
            workspaceId,
            sessionId,
            turnId,
            activeTarget: turnActiveDocTarget,
            contextRevision,
            assistantMessageId: assistantId,
            prompt: effectivePrompt,
            references,
            mentions,
            blockRefs,
            modelBinding: selectedModel,
            requestSnapshot,
          });
          void saveCurrentTurn("draft", turnActiveDocPath);
          pausedForDecision = true;
          finish();
          break;
        case "decision_request_resolved":
          agStore.clearPendingDecision({ requestId: ev.requestId, sessionId, turnId });
          break;
        case "diff_proposed": {
          // AI 经 propose_edit 生成了待裁决修改（PI-AG-08C）：投影为 diff 卡片；
          // 当前编辑器命中的块级 diff 会先进入 ED 草稿，accept 只确认，save 再落盘。
          const detail = (await loadDiffDetails([ev.diffId]).catch(() => []))
            .find((candidate) => candidate.diff_id === ev.diffId);
          const eventBatch = {
            id: ev.proposalBatchId,
            groupIndex: ev.groupIndex,
            groupCount: ev.groupCount,
          };
          const projection = projectionFromDetail(ev.diffId, ev.relPath, detail, eventBatch);
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
            errorCode = detail?.projection
              ? "DE_PROPOSAL_BATCH_INVALID"
              : "DE_DIFF_PROJECTION_MISSING";
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
          if (projection?.preview_checkpoint_required === true) {
            previewContinuations.set(ev.diffId, {
              sessionId, turnId, assistantMessageId: assistantId, workspaceId,
              prompt: effectivePrompt, references, mentions, blockRefs,
              activeTarget: turnActiveDocTarget, contextRevision,
              modelBinding: selectedModel ? { ...selectedModel } : undefined,
            });
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
            proposalBatch: projection ? {
              id: projection.proposal_batch_id,
              groupIndex: projection.group_index,
              groupCount: projection.group_count,
            } : undefined,
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
          void saveCurrentTurn("draft", turnActiveDocPath);
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
          void saveCurrentTurn("draft", turnActiveDocPath);
          break;
        }
        case "tool_result": {
          // 工具卡 done/failed（PI-AG-08D）。结果摘要作 detail。
          const r = (ev.result ?? "").toString();
          const summary = r.slice(0, 80) + (!ev.success ? diagnosticSummary(failureDiagnostic(ev.diagnostic)) : "");
          let receipt: ToolCallReceiptV1 | undefined;
          try {
            if (ev.executionReceipt !== undefined) {
              const requested = agStore.getSnapshot().sessions.find(s => s.id === sessionId)?.toolCalls.find(t => t.id === ev.toolCallId);
              if (!requested || requested.name !== ev.toolName || requested.turnId !== turnId) throw new Error("AG_TOOL_RECEIPT_INVALID");
              receipt = validateToolCallReceipt(ev.executionReceipt,
                { workspaceId, sessionId, turnId, toolCallId: ev.toolCallId, toolName: requested.name }, ev.success, ev.evidenceIds);
            }
            agStore.resolveToolCallById(sessionId, ev.toolCallId, ev.success,
              receipt ? `${summary} · ${receiptSummary(receipt)}` : summary || undefined, receipt);
          } catch {
            agStore.resolveToolCallById(sessionId, ev.toolCallId, false,
              "[AG_TOOL_RECEIPT_INVALID] 执行回执无法确认；请核对实际操作，不会自动重试。");
          }
          void saveCurrentTurn("draft", turnActiveDocPath);
          break;
        }
        case "project_report": {
          agStore.projectProjectReport(turnId, ev.report);
          break;
        }
          default:
            break;
        }
      }).catch(interruptStart);
    };
    onEvent.onmessage = consumeEvent;

    void (async () => {
      let openDocSnapshots: OpenDocSnapshot[];
      let agentContextFacts: AgentContextFactsV2;
      const factBlockRefs = references.length ? [] : blockRefs;
      const selectionPaths = [...new Set([
        ...references.flatMap((r) => r.kind === "selection" ? [r.relPath] : []),
        ...factBlockRefs.map((r) => r.relPath),
      ])];
      try {
        // Preview admission consumes an exact versioned checkpoint before the model
        // can request context. Supply only that fresh snapshot, not a later callback.
        const previewPath = opts?.continuation?.previewDiffId
          ? deStore.getSnapshot().diffs.find(diff => diff.diffId === opts.continuation?.previewDiffId)?.relPath
          : undefined;
        const captured = collectAgentContextV2(turnId, sessionId, turnActiveDocTarget, ++contextRevision, references, factBlockRefs, undefined,
          [...new Set([...selectionPaths, ...(previewPath ? [previewPath] : [])])]);
        openDocSnapshots = captured.snapshots;
        agentContextFacts = captured.facts;
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
        interruptStart(e);
        return;
      }
      const activeSnapshot = turnActiveDocPath
        ? openDocSnapshots.find((s) => s.relPath === turnActiveDocPath)
        : undefined;
      const activeDocContent = activeSnapshot?.content;
      const onContextRequest = new Channel<AgentContextRequest>();
      const admissionRevision = agentContextFacts.contextRevision;
      let contextInventory = agentContextFacts;
      onContextRequest.onmessage = (request) => {
        if (settled || terminalSeen || request.workspaceId !== workspaceId || request.sessionId !== sessionId || request.turnId !== turnId) return;
        if (!Number.isSafeInteger(request.contextRevision) || request.contextRevision <= admissionRevision) return;
        try {
          const captured = collectAgentContextV2(turnId, sessionId, turnActiveDocTarget, request.contextRevision, references, factBlockRefs, contextInventory, [...request.paths, ...selectionPaths], request.allDocuments);
          // Keep the admission inventory: backend owns accepted revision and repeated acquisition after a failed response.
          contextRevision = Math.max(contextRevision, request.contextRevision);
          contextInventory = captured.facts;
          void invoke("ag_supply_context", { response: { requestId: request.requestId, facts: captured.facts, snapshots: captured.snapshots } }).catch((error) => {
            console.warn("Agent context response rejected", error);
          });
        } catch (error) {
          // Identity loss cannot be repaired with old facts; cancel the owning turn rather than retarget it.
          console.warn("Agent context collection failed", error);
          void invoke("ag_cancel_turn", { sessionId, turnId }).catch((cancelError) => console.warn("Agent context cancellation failed", cancelError));
        }
      };
      backendInvocationStarted = true;
      if (clarification) clarifyingTurns.delete(sessionId);
      if (decision) decisionTurns.delete(decision.requestId);
      try {
        await invoke("ag_start_turn", {
          turnId,
          sessionId,
          runtimeProfile: AGENT_EDIT_RUNTIME_PROFILE,
          prompt: effectivePrompt,
          provider: selectedModel?.provider,
          model: selectedModel?.model,
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
          onContextRequest,
        });
        await eventWork;
        if (!terminalSeen && !pausedForClarification && !pausedForDecision) {
          const recovered = await listTurnEvents(turnId, agStore.lastTurnEventSeq(turnId));
          for (const event of recovered) consumeEvent(event);
          await eventWork;
        }
        if (!terminalSeen && !pausedForClarification && !pausedForDecision) {
          interruptStart({
            code: "AG_TURN_TERMINAL_MISSING",
            message: "Agent 回合结束但没有可恢复的终态事件",
          });
        }
      } catch (error) {
        interruptStart(error);
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
    if (event.type === "chunk") agStore.appendToMessage(parked.sessionId, parked.assistantMessageId, event.text);
    if (event.type === "error" || (event.type === "done" && event.stopReason === "cancelled")) {
      if (import.meta.env.DEV) console.debug("[AG-AG-TURN] frontend_accepted:", JSON.stringify({
        sessionId: parked.sessionId, turnId: parked.turnId, eventId: event.eventId,
        seq: event.seq, eventType: event.type, source: "parked_cancellation_replay",
      }));
      if (clarifyingTurns.get(parked.sessionId)?.turnId === parked.turnId) {
        clarifyingTurns.delete(parked.sessionId);
      }
      if ("requestId" in parked) {
        decisionTurns.delete(parked.requestId);
        savedGroupWaits.delete(parked.requestId);
        agStore.setResourceSaveWait(parked.sessionId, parked.assistantMessageId);
        agStore.clearPendingDecision({ requestId: parked.requestId, sessionId: parked.sessionId, turnId: parked.turnId });
      }
      agStore.finishMessage(parked.sessionId, parked.assistantMessageId);
      const status = event.type === "error" ? "error" : "cancelled";
      agStore.setTurnStatus(parked.sessionId, parked.turnId, status);
      agStore.clearTurnAction(parked.sessionId, parked.turnId);
      void enqueueTurnSave(parked.sessionId, parked.turnId, status, undefined, undefined, parked.requestSnapshot);
      return true;
    }
  }
  return false;
}
