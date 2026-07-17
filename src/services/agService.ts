import { Channel, invoke } from "@tauri-apps/api/core";
import { agStore } from "../store/agStore";
import { edStore } from "../store/edStore";
import { deStore } from "../store/deStore";
import { getActiveBlockSnapshotByIndex, getActiveDocumentContent } from "./edLocate";
import { loadDocs } from "./wsService";
import type { MessageContentPart, ReferencePayload } from "./referencePayload";
import type { StyleOp } from "./edStyleOps";

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

type AgentEditPatchKind = "replace_block" | "replace_document" | "style_ops";

interface AgentDiffProjection {
  diff_id: string;
  rel_path: string;
  patch_kind: AgentEditPatchKind;
  block_id?: string | null;
  block_index?: number | null;
  before_text: string;
  after_text: string;
  location_label: string;
  draft_mode: "current_editor" | "background";
  style_ops?: unknown;
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
      (p.patch_kind !== "replace_block" && p.patch_kind !== "replace_document" && p.patch_kind !== "style_ops") ||
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
      draft_mode: p.draft_mode === "background" ? "background" : "current_editor",
      style_ops: p.style_ops,
    };
  } catch {
    return null;
  }
}

function parseStyleOps(value: unknown): StyleOp[] {
  if (!Array.isArray(value)) return [];
  const out: StyleOp[] = [];
  for (const item of value) {
    if (!isRecord(item) || typeof item.blockId !== "string" || !item.blockId.trim()) continue;
    if (item.op === "set_block_attr" && isRecord(item.attrs)) {
      out.push({ op: "set_block_attr", blockId: item.blockId, attrs: item.attrs });
      continue;
    }
    const start = typeof item.start === "number" && Number.isInteger(item.start) ? item.start : null;
    const end = typeof item.end === "number" && Number.isInteger(item.end) ? item.end : null;
    if (start === null || end === null) continue;
    if (item.op === "set_mark" && isRecord(item.mark) && typeof item.mark.type === "string") {
      const attrs = isRecord(item.mark.attrs) ? item.mark.attrs : undefined;
      out.push({ op: "set_mark", blockId: item.blockId, start, end, mark: { type: item.mark.type, attrs } });
      continue;
    }
    if (item.op === "unset_mark" && typeof item.markType === "string") {
      out.push({ op: "unset_mark", blockId: item.blockId, start, end, markType: item.markType });
    }
  }
  return out;
}

function summarizeStyleOp(op: StyleOp): string {
  if (op.op === "set_block_attr") return `${op.blockId} 设置块样式`;
  if (op.op === "unset_mark") return `${op.blockId} 移除 ${op.markType} [${op.start},${op.end}]`;
  return `${op.blockId} 设置 ${op.mark.type} [${op.start},${op.end}]`;
}

/** 文档块（供块级 @mention 选择非当前文档的块，PI-AG-09G）。 */
export interface DocBlock {
  block_id: string | null;
  block_index: number;
  text: string;
  /** 样式投影（PI-ED-11 · A-AG-I-P-01 §4.1）：mark 段（range 在块内纯文本平面，与字符锚同基准）。
   *  text 块为空数组；schema 单一来源 A-ED-M-T-01 §5.1。 */
  marks?: { type: string; range: [number, number]; attrs?: Record<string, unknown> }[];
  /** 块级呈现属性投影（textAlign 等）。 */
  block_attrs?: Record<string, unknown>;
}

let dseq = 0;
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
  const r = await dispatchCall<{ blocks: DocBlock[] }>("ag.list_blocks", { relPath });
  return r.blocks;
}

/**
 * BYOK：把 provider 密钥一次性上送 dev 网关代持（PI-BYOK-DEV-01）。
 * 客户端不持久化、不回显——密钥经后端 ent::byok 送网关后即清零（X-FORBID-001）。
 */
export async function uploadByok(provider: string, key: string): Promise<void> {
  await invoke("byok_upload", { provider, key });
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

  // 回合归属会话：发起时的活动会话（此后用户切标签不影响本回合投影目标）。
  const snap = agStore.getSnapshot();
  const sessionId = snap.activeSessionId;
  const session = snap.sessions.find((s) => s.id === sessionId);

  // 本轮之前的对话历史（PI-AG-09B）：传后端按预算保留最近（drop 最旧）。取本轮 user 入栈前的快照。
  const history = (session?.messages ?? [])
    .filter((m) => !m.error && m.content.trim())
    .slice(-30)
    .map((m) => ({ role: m.role, content: m.content }));

  agStore.pushMessage(sessionId, { id: uid(), role: "user", content: trimmed, parts: opts?.displayParts });
  agStore.clearToolCalls(sessionId); // 新回合清空旧工具卡（PI-AG-08D）
  agStore.setSending(true); // 乐观置 true 即时门控防双发；活动/释放由 SM-AG-01 广播投影（Task 2-E）

  const assistantId = uid();
  agStore.pushMessage(sessionId, { id: assistantId, role: "assistant", content: "", streaming: true });

  const turnId = `t-${Date.now()}-${(seq += 1)}`;
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

    onEvent.onmessage = (ev) => {
      switch (ev.type) {
        case "chunk":
          agStore.appendToMessage(sessionId, assistantId, ev.text);
          break;
        case "done":
          agStore.finishMessage(sessionId, assistantId);
          finish();
          break;
        case "error":
          agStore.finishMessage(sessionId, assistantId);
          agStore.pushMessage(sessionId, {
            id: uid(),
            role: "assistant",
            content: `[${ev.code}] ${ev.message}`,
            error: true,
          });
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
          } else if (projection.patch_kind === "replace_block" && !mappedBlockId) {
            title = `⚠ 缺少精准编辑目标，请复核 · ${ev.rel_path}`;
            state = "error";
            errorCode = "ED_DRAFT_TARGET_MISSING";
          }
          const styleOpsPayload = projection ? parseStyleOps(projection.style_ops) : [];
          deStore.addDiff({
            diffId: ev.diff_id,
            sessionId,
            turnId,
            anchorMessageId: assistantId,
            createdAt: Date.now(),
            blockId: mappedBlockId,
            relPath: ev.rel_path,
            locationLabel: projection?.location_label ?? "未知位置",
            patchKind: projection?.patch_kind,
            title,
            before: projection?.before_text ?? "",
            after: projection?.after_text ?? "",
            state,
            errorCode,
            styleOps: styleOpsPayload.length ? styleOpsPayload.map(summarizeStyleOp) : undefined,
            styleOpsPayload: styleOpsPayload.length ? styleOpsPayload : undefined,
          });
          break;
        }
        case "tool_request": {
          // 工具卡 executing（PI-AG-08D）。参数摘要作 detail。
          const p = typeof ev.params === "object" && ev.params ? JSON.stringify(ev.params) : "";
          agStore.addToolCall(sessionId, ev.tool_name, p && p !== "{}" ? p.slice(0, 80) : undefined);
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
          agStore.resolveToolCall(sessionId, ev.tool_name, ev.success, r ? r.slice(0, 80) : undefined);
          break;
        }
        default:
          break;
      }
    };

    // 当前编辑器打开的文档（P0 主事实，PI-AG-09A）：只读 edStore，不改其状态。
    const ed = edStore.getSnapshot();
    const activeDocContent = ed.openPath
      ? (getActiveDocumentContent(ed.editFormat) ?? ed.loadedContent ?? undefined)
      : undefined;
    invoke("ag_start_turn", {
      turnId,
      prompt: trimmed,
      provider: opts?.provider,
      model: opts?.model,
      activeDocPath: ed.openPath ?? undefined,
      activeDocContent,
      activeDocFormat: ed.editFormat, // text|html（docx 块编辑用，PI-AG-08F）
      history, // 对话历史（PI-AG-09B，后端按预算保留最近）
      mentions: references.length ? [] : (opts?.mentions ?? []), // 兼容旧路径；新路径走 references 防重复注入
      blockRefs: references.length ? [] : (opts?.blockRefs ?? []), // 兼容旧路径；新路径走 references 防重复注入
      references, // 用户显式 ReferencePayload（A-AG-I-P-01 v1.6）
      onEvent,
    }).catch((e: unknown) => {
      const err = e as { code?: string; message?: string };
      agStore.finishMessage(sessionId, assistantId);
      agStore.pushMessage(sessionId, {
        id: uid(),
        role: "assistant",
        content: err?.message ? `[${err.code ?? "?"}] ${err.message}` : String(e),
        error: true,
      });
      // 命令级 Err（IPC 传输失败等）可能不发广播：兜底释放 sending，避免输入永久禁用。
      agStore.setSending(false);
      finish();
    });
  });
}
