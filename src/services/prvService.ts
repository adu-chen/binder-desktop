import { invoke } from "@tauri-apps/api/core";

/**
 * PRV Service（X-SEQ-002）：只读预览经 prv.preview 触达 DOC 预览链（LibreOffice → PDF 字节）。
 * 字节回传 → 前端组 blob URL 呈现；只读无编辑（PRV 硬边界，A-PRV-M-T-01）。
 */

interface CommandResponse<T> {
  correlation_id: string;
  result?: T;
  error?: { code: string; message: string; retriable: boolean; reauth_required: boolean };
}

let seq = 0;
async function callCommand<T>(command: string, params: Record<string, unknown>): Promise<T> {
  const request = { command, correlation_id: `prv-${Date.now()}-${(seq += 1)}`, params };
  const response = await invoke<CommandResponse<T>>("dispatch", { request });
  if (response.error) throw response.error;
  if (response.result === undefined) {
    throw { code: "IPC_EMPTY_RESULT", message: `${command} 无返回`, retriable: false, reauth_required: false };
  }
  return response.result;
}

/** 预览文档 → PDF blob URL（字节回传组装）。调用方负责 revokeObjectURL。 */
export async function previewDoc(relPath: string): Promise<string> {
  const res = await callCommand<{ format: string; bytes: number[] }>("prv.preview", { relPath });
  const blob = new Blob([new Uint8Array(res.bytes)], { type: "application/pdf" });
  return URL.createObjectURL(blob);
}
