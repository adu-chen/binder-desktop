import { invoke, convertFileSrc } from "@tauri-apps/api/core";

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

/** 预览文档 → PDF asset URL（后端缓存落盘 + asset 授权，前端 convertFileSrc 生成 URL 交 iframe 加载）。 */
export async function previewDoc(relPath: string): Promise<string> {
  const res = await callCommand<{ format: string; abs_path: string }>("prv.preview", { relPath });
  return convertFileSrc(res.abs_path);
}

/** 本地打开：以系统默认应用打开 workspace 内文件（只读查看，X-FORBID-004 守界）。 */
export async function openExternal(relPath: string): Promise<void> {
  await callCommand<{ opened: boolean }>("prv.open_external", { relPath });
}

/** 导出文档为目标格式（落 workspace 内，守界）；回导出文件 rel_path。 */
export async function exportDoc(relPath: string, format: "pdf" | "docx" | "md"): Promise<string> {
  const res = await callCommand<{ rel_path: string }>("prv.export", { relPath, format });
  return res.rel_path;
}

/**
 * 音视频源 URL（asset 协议流式）：prv.media_src 守界 + 授权该文件 → convertFileSrc → asset URL。
 * 大文件不进内存（webview 从磁盘流式解码）；只读。
 */
export async function mediaSrc(relPath: string): Promise<string> {
  const res = await callCommand<{ abs_path: string }>("prv.media_src", { relPath });
  return convertFileSrc(res.abs_path);
}
