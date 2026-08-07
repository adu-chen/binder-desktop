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

interface FsEntryDto {
  rel_path: string;
  doc_type: string;
  is_dir: boolean;
  is_bdocx?: boolean;
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
  baseline: number | string;
}

/** 读文档内容 + 加载基线（ws.read_doc，守界后端执行）。 */
export async function readDoc(relPath: string): Promise<DocReadDto> {
  return callCommand<DocReadDto>("ws.read_doc", { relPath });
}

/**
 * 原子保存（ws.save_doc）。返回内容哈希。
 * baseline 是读取时点的磁盘内容哈希；override 必须来自用户显式覆盖确认。
 */
export async function saveDoc(
  relPath: string,
  content: string,
  baseline: number | string | null,
  override = false,
): Promise<number | string> {
  const res = await callCommand<{ baseline: number | string }>("ws.save_doc", {
    relPath,
    content,
    baseline,
    override,
  });
  return res.baseline;
}

/** 新建文档（ws.create_doc，守界，已存在拒绝 WS_EXISTS）。回新文件 rel_path。 */
export async function createDoc(relPath: string, content = ""): Promise<string> {
  const r = await callCommand<{ rel_path: string }>("ws.create_doc", { relPath, content });
  return r.rel_path;
}

/** 新建目录（ws.create_folder，守界；目标已为目录则幂等成功，目标为文件拒绝）。回 rel_path。 */
export async function createFolder(relPath: string): Promise<string> {
  const r = await callCommand<{ rel_path: string }>("ws.create_folder", { relPath });
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

/** 删除目录（ws.delete_folder，守界，两级语义）：mode=empty 仅删空目录（非空拒 WS_NOT_EMPTY），
 *  mode=recursive 递归删除；不存在/目标为文件拒绝。 */
export async function deleteFolder(relPath: string, mode: "empty" | "recursive"): Promise<void> {
  await callCommand("ws.delete_folder", { relPath, mode });
}

export interface SearchHitDto {
  rel_path: string;
  line: number;
  preview: string;
}

/** 全文搜索（ws.search，守界，命中封顶）；空查询回空。 */
export async function searchDocs(query: string): Promise<SearchHitDto[]> {
  if (!query.trim()) return [];
  return callCommand<SearchHitDto[]>("ws.search", { query });
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
  const entries = await callCommand<FsEntryDto[]>("ws.list_docs", {});
  return entries.map((e) => ({
    relPath: e.rel_path,
    docType: e.doc_type,
    isDir: e.is_dir,
    isBDocx: Boolean(e.is_bdocx),
  }));
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
