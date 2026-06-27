import { Channel, invoke } from "@tauri-apps/api/core";
import { agStore } from "../store/agStore";
import { edStore } from "../store/edStore";
import { deStore } from "../store/deStore";
import { locateBlock } from "./edLocate";

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

let seq = 0;
const uid = () => `m-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;

/** 文档块（供块级 @mention 选择非当前文档的块，PI-AG-09G）。 */
export interface DocBlock {
  block_id: string | null;
  text: string;
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

/**
 * 发起对话回合：`ag_start_turn`（async + Channel）→ 真实流式 `AgTurnEvent` 投影到 agStore。
 * 需先启动 cloud/dev-gateway（localhost:8787）并配置 provider 密钥；否则回 AG_GW_UNREACHABLE 错误终态。
 */
export async function startTurn(
  prompt: string,
  opts?: {
    provider?: string;
    model?: string;
    mentions?: string[];
    blockRefs?: { relPath: string; blockId: string; text: string }[];
  },
): Promise<void> {
  const trimmed = prompt.trim();
  if (!trimmed) return;

  // 本轮之前的对话历史（PI-AG-09B）：传后端按预算保留最近（drop 最旧）。取本轮 user 入栈前的快照。
  const history = agStore
    .getSnapshot()
    .messages.filter((m) => !m.error && m.content.trim())
    .slice(-30)
    .map((m) => ({ role: m.role, content: m.content }));

  agStore.pushMessage({ id: uid(), role: "user", content: trimmed });
  agStore.clearToolCalls(); // 新回合清空旧工具卡（PI-AG-08D）
  agStore.setSending(true);

  const assistantId = uid();
  agStore.pushMessage({ id: assistantId, role: "assistant", content: "", streaming: true });

  const turnId = `t-${Date.now()}-${(seq += 1)}`;
  const onEvent = new Channel<AgTurnEvent>();
  // 本回合最近一次 propose_edit 的目标块锚（PI-AG-08H）+ 提案内容（after 预览，PI-AG-09I），从 tool_request.params 捕获。
  let lastEditBlockId: string | undefined;
  let lastEditAfter: string | undefined;

  await new Promise<void>((resolve) => {
    let settled = false;
    const finish = () => {
      if (settled) return;
      settled = true;
      agStore.setSending(false);
      resolve();
    };

    onEvent.onmessage = (ev) => {
      switch (ev.type) {
        case "chunk":
          agStore.appendToMessage(assistantId, ev.text);
          break;
        case "done":
          agStore.finishMessage(assistantId);
          finish();
          break;
        case "error":
          agStore.finishMessage(assistantId);
          agStore.pushMessage({
            id: uid(),
            role: "assistant",
            content: `[${ev.code}] ${ev.message}`,
            error: true,
          });
          finish();
          break;
        case "diff_proposed": {
          // AI 经 propose_edit 生成了待裁决修改（PI-AG-08C）：投影为 diff 卡片；
          // 接受经既有 deService.acceptDiff→de.accept_diff→ws.apply_diff 真落盘（X-FORBID-006）。
          const before = edStore.getSnapshot().loadedContent ?? "";
          // ED 真实锚联动（PI-AG-08H）：对**实时编辑器**校验目标块锚；失效（用户已改动文档）则显式警示，不静默。
          let title = `AI 建议修改 ${ev.rel_path}`;
          if (lastEditBlockId && locateBlock(lastEditBlockId) === "anchor_lost") {
            title = `⚠ 文档已变动，请复核 · ${ev.rel_path}`;
          }
          deStore.addDiff({
            diffId: ev.diff_id,
            blockId: lastEditBlockId ?? "",
            relPath: ev.rel_path,
            title,
            before: before.slice(-60),
            after: lastEditAfter ? lastEditAfter.slice(0, 120) : "（AI 修改建议，接受后应用到文档）",
            state: "pending",
          });
          lastEditBlockId = undefined;
          lastEditAfter = undefined;
          break;
        }
        case "tool_request": {
          // 捕获 propose_edit 的目标块锚，供 diff_proposed 实时校验（PI-AG-08H）。
          if (ev.tool_name === "propose_edit" && ev.params && typeof ev.params === "object") {
            const p = ev.params as { block_id?: unknown; new_text?: unknown; content?: unknown };
            lastEditBlockId = typeof p.block_id === "string" && p.block_id ? p.block_id : undefined;
            const after = typeof p.new_text === "string" ? p.new_text : typeof p.content === "string" ? p.content : "";
            lastEditAfter = after.trim() ? after : undefined;
          }
          // 工具卡 executing（PI-AG-08D）。参数摘要作 detail。
          const p = typeof ev.params === "object" && ev.params ? JSON.stringify(ev.params) : "";
          agStore.addToolCall(ev.tool_name, p && p !== "{}" ? p.slice(0, 80) : undefined);
          break;
        }
        case "tool_result": {
          // 工具卡 done/failed（PI-AG-08D）。结果摘要作 detail。
          const r = (ev.result ?? "").toString();
          agStore.resolveToolCall(ev.tool_name, ev.success, r ? r.slice(0, 80) : undefined);
          break;
        }
        default:
          break;
      }
    };

    // 当前编辑器打开的文档（P0 主事实，PI-AG-09A）：只读 edStore，不改其状态。
    const ed = edStore.getSnapshot();
    invoke("ag_start_turn", {
      turnId,
      prompt: trimmed,
      provider: opts?.provider,
      model: opts?.model,
      activeDocPath: ed.openPath ?? undefined,
      activeDocContent: ed.loadedContent ?? undefined,
      activeDocFormat: ed.editFormat, // text|html（docx 块编辑用，PI-AG-08F）
      history, // 对话历史（PI-AG-09B，后端按预算保留最近）
      mentions: opts?.mentions ?? [], // 用户显式引用的文档（PI-AG-09E，P1 高优先注入）
      blockRefs: opts?.blockRefs ?? [], // 块级引用：选区精确锚（PI-AG-09F）
      onEvent,
    }).catch((e: unknown) => {
      const err = e as { code?: string; message?: string };
      agStore.finishMessage(assistantId);
      agStore.pushMessage({
        id: uid(),
        role: "assistant",
        content: err?.message ? `[${err.code ?? "?"}] ${err.message}` : String(e),
        error: true,
      });
      finish();
    });
  });
}
