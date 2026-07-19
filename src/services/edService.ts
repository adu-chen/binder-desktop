import { invoke } from "@tauri-apps/api/core";
import { readDoc, saveDoc } from "./wsService";
import { openDocxForEdit, saveDocxFromEdit, isDocxEditable } from "./docService";
import { edStore, hashLogicalContent, type EditFormat, type OpenDocSnapshot } from "../store/edStore";
import type { PersistedOpenDocTab } from "../store/tabsStore";
import { deStore } from "../store/deStore";

/**
 * ED 编排 Service（X-SEQ-002）：打开/保存真实文档。
 */

interface IpcLikeError {
  code?: string;
  message?: string;
}

interface CommandResponse<T> {
  correlation_id: string;
  result?: T;
  error?: { code: string; message: string; retriable: boolean; reauth_required: boolean };
}

const SAVE_CONFLICT_CODES = new Set(["WS_SAVE_CONFLICT", "save_conflict"]);
let seq = 0;

async function callCommand<T>(command: string, params: Record<string, unknown>): Promise<T> {
  const request = { command, correlation_id: `ed-${Date.now()}-${(seq += 1)}`, params };
  const response = await invoke<CommandResponse<T>>("dispatch", { request });
  if (response.error) throw response.error;
  if (response.result === undefined) {
    throw { code: "IPC_EMPTY_RESULT", message: `${command} 无返回`, retriable: false, reauth_required: false };
  }
  return response.result;
}

function isSaveConflict(e: IpcLikeError): boolean {
  return Boolean(e.code && SAVE_CONFLICT_CODES.has(e.code));
}

/** 是否 .html/.htm 独立文件（PI-ED-13 / REQ-ED-008：富文本编辑 + HTML 文本往返，不经 DOC）。 */
export function isHtmlFile(relPath: string): boolean {
  const ext = relPath.toLowerCase().split(".").pop() ?? "";
  return ext === "html" || ext === "htm";
}

export async function persistSessionSnapshot(snapshot: OpenDocSnapshot): Promise<void> {
  await callCommand("ed.persist_session_snapshot", { snapshot });
}

export async function restoreSessionSnapshots(): Promise<OpenDocSnapshot[]> {
  const res = await callCommand<{ snapshots: OpenDocSnapshot[] }>("ed.list_session_snapshots", {});
  return res.snapshots;
}

export async function persistOpenTabs(tabs: PersistedOpenDocTab[]): Promise<void> {
  await callCommand("ed.persist_open_tabs", { tabs });
}

export async function restoreOpenTabs(): Promise<PersistedOpenDocTab[]> {
  const res = await callCommand<{ tabs: PersistedOpenDocTab[] }>("ed.list_open_tabs", {});
  return res.tabs;
}

export function makePersistedSnapshot(
  relPath: string,
  content: string,
  editFormat: EditFormat,
  dirty: boolean,
  sessionVersion?: number,
): OpenDocSnapshot {
  return {
    relPath,
    content,
    editFormat,
    source: "open_editor_session",
    dirty,
    sessionVersion: sessionVersion ?? edStore.getSnapshot().sessions[relPath]?.sessionVersion ?? 1,
    contentHash: hashLogicalContent(content),
    capturedAt: Date.now(),
  };
}

/**
 * @GOV
 * codes:    BR-ED-DATA-002-RB-ED-ED-EDIT-012
 * type:     RB
 * chain:    ED-EDIT
 * rules:    BR-ED-DATA-002
 * boundary: in=workspaceDocumentPath | out=editSessionContentModel(textOrHtmlViaCorrectPipeline)
 */
/// md/BDocx/html 统一编辑模型守则（BR-ED-DATA-002）。
/** 打开文档：docx 经 DOC 编辑链（doc.open_for_edit → HTML），其余经 ws.read_doc（文本）→ 载入编辑器
 *  （内容 + 加载基线）；jumpLine 时载入后跳转到该行。 */
export async function openDocument(relPath: string, jumpLine: number | null = null): Promise<void> {
  const hadSession = Boolean(edStore.getSnapshot().sessions[relPath]);
  if (!hadSession) edStore.openDoc(relPath);
  try {
    if (isDocxEditable(relPath)) {
      const { html, baseline } = await openDocxForEdit(relPath);
      edStore.loadDoc(relPath, html, baseline, jumpLine, "html");
      const sess = edStore.getSnapshot().sessions[relPath];
      if (sess) {
        await persistSessionSnapshot(makePersistedSnapshot(relPath, html, "html", false, sess.sessionVersion));
      }
      return;
    }
    const { content, baseline } = await readDoc(relPath);
    const editFormat = isHtmlFile(relPath) ? "html_file" : "text";
    edStore.loadDoc(relPath, content, baseline, jumpLine, editFormat);
    const sess = edStore.getSnapshot().sessions[relPath];
    if (sess) {
      await persistSessionSnapshot(makePersistedSnapshot(relPath, content, editFormat, false, sess.sessionVersion));
    }
  } catch (e) {
    const err = e as IpcLikeError;
    if (!hadSession) edStore.closeSession(relPath);
    console.error(`open document failed [${err.code ?? "?"}]: ${err.message ?? e}`);
  }
}

/** 保存结果。 */
export type SaveOutcome = { ok: true } | { ok: false; message: string; conflict?: boolean };

/**
 * @GOV
 * codes:    BR-ED-STATE-003-EFFECT-ED-ED-EDIT-013
 * type:     EFFECT
 * chain:    ED-EDIT
 * rules:    BR-ED-STATE-003
 * boundary: in=saveConflictFromWorkspace | out=SaveOutcome(conflictDirtyNoSilentOverwrite)
 */
/// 保存冲突守则（BR-ED-STATE-003）：WS 报保存冲突时保持当前编辑为 dirty，返回 conflict 信号给 UI；
/// 只有用户显式确认覆盖时才以 override=true 重试，不静默覆盖磁盘、不静默重载丢弃本地编辑。
/**
 * 保存当前文档（⌘S）：ws.save_doc。失败回脏态。
 */
export async function saveDocument(content: string, override = false): Promise<SaveOutcome> {
  const { openPath, baseline } = edStore.getSnapshot();
  if (!openPath) return { ok: false, message: "无打开的文档" };
  const path = openPath;
  try {
    const newBaseline = await saveDoc(path, content, baseline, override);
    edStore.updateContent(path, content);
    edStore.setBaseline(path, newBaseline);
    const sess = edStore.getSnapshot().sessions[path];
    if (sess) {
      await persistSessionSnapshot(makePersistedSnapshot(path, content, sess.editFormat, false, sess.sessionVersion));
    }
    deStore.markDraftSaved(path);
    return { ok: true };
  } catch (e) {
    const err = e as IpcLikeError;
    edStore.setTabState(path, "dirty");
    return { ok: false, message: err.message ?? String(e), conflict: isSaveConflict(err) };
  }
}

/**
 * 保存 docx 文档（⌘S，编辑链）：编辑 HTML → doc.save_from_edit。
 */
export async function saveDocxDocument(html: string, override = false): Promise<SaveOutcome> {
  const { openPath, baseline } = edStore.getSnapshot();
  if (!openPath) return { ok: false, message: "无打开的文档" };
  const path = openPath;
  try {
    const newBaseline = await saveDocxFromEdit(path, html, baseline, override);
    edStore.updateContent(path, html);
    edStore.setBaseline(path, newBaseline);
    const sess = edStore.getSnapshot().sessions[path];
    if (sess) {
      await persistSessionSnapshot(makePersistedSnapshot(path, html, sess.editFormat, false, sess.sessionVersion));
    }
    deStore.markDraftSaved(path);
    return { ok: true };
  } catch (e) {
    const err = e as IpcLikeError;
    edStore.setTabState(path, "dirty");
    return { ok: false, message: err.message ?? String(e), conflict: isSaveConflict(err) };
  }
}

/** 重载：重新从磁盘载入。 */
export async function reloadDocument(): Promise<void> {
  const { openPath } = edStore.getSnapshot();
  if (openPath) await openDocument(openPath);
}
