import { invoke } from "@tauri-apps/api/core";

/**
 * DOC 编辑链 Service（X-SEQ-002 / A-DOC-I-P-01 §2）：docx 打开/保存经 DOC 编辑链 command。
 * 与预览链（prvService）完全分离（BR-DOC-DATA-001）：编辑链 Pandoc（docx↔HTML，保留样式子集），
 * 预览链 LibreOffice（docx→PDF 只读）。docx 编辑模型为 **HTML**（TipTap 原生），非 markdown——
 * markdown 丢失样式子集「部分支持」层（颜色/字体/字号），故编辑链产出 HTML。
 */

interface CommandResponse<T> {
  correlation_id: string;
  result?: T;
  error?: { code: string; message: string; retriable: boolean; reauth_required: boolean };
}

let seq = 0;
async function callCommand<T>(command: string, params: Record<string, unknown>): Promise<T> {
  const request = { command, correlation_id: `doc-${Date.now()}-${(seq += 1)}`, params };
  const response = await invoke<CommandResponse<T>>("dispatch", { request });
  if (response.error) throw response.error;
  if (response.result === undefined) {
    throw { code: "IPC_EMPTY_RESULT", message: `${command} 无返回`, retriable: false, reauth_required: false };
  }
  return response.result;
}

export interface DocxEditModel {
  /** 可编辑 HTML（含内联样式/data-custom-style，供 TipTap setContent(html)）。 */
  html: string;
  /** 加载基线（docx 字节内容哈希），保存时回传供外部变更冲突保护（SM-WS-01）。 */
  baseline: number;
}

/**
 * 打开 docx 进入编辑（doc.open_for_edit）：docx → 可编辑 HTML + 加载基线。
 * 失败抛 IpcError（convert_failed/依赖缺失）——调用方据此不建立编辑会话（SM-DOC-01 OpenError）。
 */
export async function openDocxForEdit(relPath: string): Promise<DocxEditModel> {
  const res = await callCommand<{ format: string; html: string; baseline: number }>(
    "doc.open_for_edit",
    { relPath },
  );
  return { html: res.html, baseline: res.baseline };
}

/**
 * 保存编辑回 docx（doc.save_from_edit）：编辑 HTML → docx（--reference-doc 统一样式）→ WS 原子落盘。
 * baseline = 打开时的加载基线；磁盘≠基线且未 override → WS_SAVE_CONFLICT（SM-WS-01，原文件不变）。
 * 返回新基线（保存后 docx 字节哈希）。
 */
export async function saveDocxFromEdit(
  relPath: string,
  html: string,
  baseline: number | null,
  override = false,
): Promise<number> {
  const res = await callCommand<{ baseline: number }>("doc.save_from_edit", {
    relPath,
    html,
    baseline,
    override,
  });
  return res.baseline;
}

/** 是否 docx 编辑链文档（经 DOC 编辑链打开/保存，区别于 md 原生与 Excel/PPT 只读预览）。 */
export function isDocxEditable(relPath: string): boolean {
  const ext = relPath.toLowerCase().split(".").pop() ?? "";
  return ext === "docx" || ext === "doc";
}
