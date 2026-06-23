import { invoke } from "@tauri-apps/api/core";
import { wsStore, type DocRefView } from "../store/wsStore";

/**
 * WS 模块 Service 层（X-SEQ-002）：文件树等 WS command 经此触达后端。
 * 经统一 dispatch 信封（A-SYS-I-P-02）调用，result/error 互斥（BR-SYS-OBS-002）。
 * 注：dispatch 信封逻辑与 workspaceService 同构；后续可抽公共 ipc 客户端（范围外，记为债务）。
 */

interface CommandResponse<T> {
  correlation_id: string;
  result?: T;
  error?: { code: string; message: string; retriable: boolean; reauth_required: boolean };
}

interface DocRefDto {
  workspace_id: string;
  rel_path: string;
  doc_type: string;
}

let seq = 0;
async function callCommand<T>(command: string, params: Record<string, unknown>): Promise<T> {
  const request = { command, correlation_id: `ws-${Date.now()}-${(seq += 1)}`, params };
  const response = await invoke<CommandResponse<T>>("dispatch", { request });
  if (response.error) throw response.error;
  if (response.result === undefined) {
    throw { code: "IPC_EMPTY_RESULT", message: `${command} 无返回`, retriable: false, reauth_required: false };
  }
  return response.result;
}

/** 加载当前活动 workspace 的文件列表（投影 loading → ready/error）。 */
export async function loadDocs(): Promise<void> {
  wsStore._projectLoading();
  try {
    const docs = await callCommand<DocRefDto[]>("ws.list_docs", {});
    const view: DocRefView[] = docs.map((d) => ({
      workspaceId: d.workspace_id,
      relPath: d.rel_path,
      docType: d.doc_type,
    }));
    wsStore._projectDocs(view);
  } catch (e) {
    const err = e as { code?: string; message?: string };
    wsStore._projectError(err?.message ? `[${err.code ?? "?"}] ${err.message}` : String(e));
  }
}
