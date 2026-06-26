import { Channel, invoke } from "@tauri-apps/api/core";
import { agStore } from "../store/agStore";
import { edStore } from "../store/edStore";
import { deStore } from "../store/deStore";

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

/**
 * 发起对话回合：`ag_start_turn`（async + Channel）→ 真实流式 `AgTurnEvent` 投影到 agStore。
 * 需先启动 cloud/dev-gateway（localhost:8787）并配置 provider 密钥；否则回 AG_GW_UNREACHABLE 错误终态。
 */
export async function startTurn(
  prompt: string,
  opts?: { provider?: string; model?: string },
): Promise<void> {
  const trimmed = prompt.trim();
  if (!trimmed) return;

  agStore.pushMessage({ id: uid(), role: "user", content: trimmed });
  agStore.clearToolCalls(); // 新回合清空旧工具卡（PI-AG-08D）
  agStore.setSending(true);

  const assistantId = uid();
  agStore.pushMessage({ id: assistantId, role: "assistant", content: "", streaming: true });

  const turnId = `t-${Date.now()}-${(seq += 1)}`;
  const onEvent = new Channel<AgTurnEvent>();

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
          deStore.addDiff({
            diffId: ev.diff_id,
            blockId: "",
            relPath: ev.rel_path,
            title: `AI 建议修改 ${ev.rel_path}`,
            before: before.slice(-60),
            after: "（AI 修改建议，接受后应用到文档）",
            state: "pending",
          });
          break;
        }
        case "tool_request": {
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
