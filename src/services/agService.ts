import { invoke } from "@tauri-apps/api/core";
import { agStore } from "../store/agStore";

/**
 * AG 模块 Service 层（X-SEQ-002）：对话回合经统一 dispatch 信封触达后端。
 * 模型输出当前桩化（云网关实现期接通）；门禁前置（X-SEQ-001）由后端 ensure_gate_passed 结构保证。
 * 流式回显此处为前端模拟（逐字），真实 turn_stream 分片（BR-SYS-STATE-002）实现期替换。
 */

interface CommandResponse<T> {
  correlation_id: string;
  result?: T;
  error?: { code: string; message: string; retriable: boolean; reauth_required: boolean };
}

interface StartTurnResult {
  reply: string;
  turn_state: string;
}

let seq = 0;
async function callCommand<T>(command: string, params: Record<string, unknown>): Promise<T> {
  const request = { command, correlation_id: `ag-${Date.now()}-${(seq += 1)}`, params };
  const response = await invoke<CommandResponse<T>>("dispatch", { request });
  if (response.error) throw response.error;
  if (response.result === undefined) {
    throw { code: "IPC_EMPTY_RESULT", message: `${command} 无返回`, retriable: false, reauth_required: false };
  }
  return response.result;
}

const uid = () => `m-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;

/** 模拟逐字流式回显（实现期由后端 turn_stream 分片驱动）。 */
function streamInto(messageId: string, text: string): Promise<void> {
  return new Promise((resolve) => {
    let i = 0;
    const step = () => {
      if (i >= text.length) {
        agStore.finishMessage(messageId);
        resolve();
        return;
      }
      const next = text.slice(i, i + 2);
      agStore.appendToMessage(messageId, next);
      i += 2;
      setTimeout(step, 16);
    };
    step();
  });
}

/** 发起对话回合：门禁前置（后端）→ 流式回显（前端模拟）。 */
export async function startTurn(prompt: string): Promise<void> {
  const trimmed = prompt.trim();
  if (!trimmed) return;
  agStore.pushMessage({ id: uid(), role: "user", content: trimmed });
  agStore.setSending(true);

  const assistantId = uid();
  try {
    const res = await callCommand<StartTurnResult>("ag.start_turn", { prompt: trimmed });
    agStore.pushMessage({ id: assistantId, role: "assistant", content: "", streaming: true });
    await streamInto(assistantId, res.reply);
  } catch (e) {
    const err = e as { code?: string; message?: string };
    agStore.pushMessage({
      id: assistantId,
      role: "assistant",
      content: err?.message ? `[${err.code ?? "?"}] ${err.message}` : String(e),
      error: true,
    });
  } finally {
    agStore.setSending(false);
  }
}
