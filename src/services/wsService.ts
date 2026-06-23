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

export interface DocReadDto {
  content: string;
  baseline: number;
}

/** 读文档内容 + 加载基线（ws.read_doc，守界后端执行）。 */
export async function readDoc(relPath: string): Promise<DocReadDto> {
  return callCommand<DocReadDto>("ws.read_doc", { relPath });
}

/**
 * 原子保存（ws.save_doc，带基线/override）。冲突时抛 WS_SAVE_CONFLICT（SM-WS-01）。
 * 返回新基线哈希。
 */
export async function saveDoc(
  relPath: string,
  content: string,
  baseline: number | null,
  override = false,
): Promise<number> {
  const res = await callCommand<{ baseline: number }>("ws.save_doc", {
    relPath,
    content,
    baseline,
    override,
  });
  return res.baseline;
}

export interface ExternalCheckDto {
  changed: boolean;
  deleted: boolean;
  current: number | null;
}

/** 外部变更检测（ws.check_external_change）：比对磁盘哈希与加载基线（BR-WS-OBS-001）。 */
export async function checkExternalChange(relPath: string, baseline: number): Promise<ExternalCheckDto> {
  return callCommand<ExternalCheckDto>("ws.check_external_change", { relPath, baseline });
}

/** 新建文档（ws.create_doc，守界，已存在拒绝 WS_EXISTS）。回新文件 rel_path。 */
export async function createDoc(relPath: string, content = ""): Promise<string> {
  const r = await callCommand<{ rel_path: string }>("ws.create_doc", { relPath, content });
  return r.rel_path;
}

/** 重命名/移动文档（ws.rename_doc，守界，目标已存在拒绝）。回新 rel_path。 */
export async function renameDoc(relPath: string, newRelPath: string): Promise<string> {
  const r = await callCommand<{ rel_path: string }>("ws.rename_doc", { relPath, newRelPath });
  return r.rel_path;
}

/** 删除文档（ws.delete_doc，守界，不存在拒绝）。 */
export async function deleteDoc(relPath: string): Promise<void> {
  await callCommand("ws.delete_doc", { relPath });
}

export interface TimelineEntryDto {
  entry_id: string;
  target: { rel_path: string; doc_type: string };
  version_seq: number;
  label: string | null;
  created_at: number;
}

/** 列文档时间轴节点（ws.list_timeline，version_seq 升序，BR-WS-DATA-002）。 */
export async function listTimeline(relPath: string): Promise<TimelineEntryDto[]> {
  return callCommand<TimelineEntryDto[]>("ws.list_timeline", { relPath });
}

/** 预览节点版本内容（ws.preview_node，SM-WS-03 Previewing）。 */
export async function previewNode(entryId: string): Promise<string> {
  const r = await callCommand<{ content: string }>("ws.preview_node", { entryId });
  return r.content;
}

/** 还原到节点版本（ws.restore_node，写回 + 前进式记新节点，BR-WS-STATE-002）。 */
export async function restoreNode(entryId: string): Promise<void> {
  await callCommand("ws.restore_node", { entryId });
}

/** 拉取当前活动 workspace 文件列表（裸调用，不投影）；供 loadDocs 与启动探测复用。 */
export async function fetchDocs(): Promise<DocRefView[]> {
  const docs = await callCommand<DocRefDto[]>("ws.list_docs", {});
  return docs.map((d) => ({ workspaceId: d.workspace_id, relPath: d.rel_path, docType: d.doc_type }));
}

/** 加载当前活动 workspace 的文件列表（投影 loading → ready/error）。 */
export async function loadDocs(): Promise<void> {
  wsStore._projectLoading();
  try {
    wsStore._projectDocs(await fetchDocs());
  } catch (e) {
    const err = e as { code?: string; message?: string };
    wsStore._projectError(err?.message ? `[${err.code ?? "?"}] ${err.message}` : String(e));
  }
}
