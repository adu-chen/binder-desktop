import { invoke } from "@tauri-apps/api/core";
import { readDoc, saveDoc } from "./wsService";
import { openDocxForEdit, saveDocxFromEdit, isDocxEditable } from "./docService";
import { edStore, hashLogicalContent, type ContentFormat, type EditFormat, type OpenDocSnapshot } from "../store/edStore";
import type { PersistedOpenDocTab } from "../store/tabsStore";
import { deStore } from "../store/deStore";
import { parseMarkdownForEditing } from "../components/ed/markdown/markdownCodec";

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
let openRequestSeq = 0;
const latestOpenRequestByPath = new Map<string, number>();

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

export function isMarkdownFile(relPath: string): boolean {
  return relPath.toLowerCase().endsWith(".md") || relPath.toLowerCase().endsWith(".markdown");
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
  contentFormat?: ContentFormat,
): OpenDocSnapshot {
  const session = edStore.getSnapshot().sessions[relPath];
  return {
    relPath,
    content,
    editFormat,
    contentFormat: contentFormat ?? session?.contentFormat ?? (editFormat === "html" || editFormat === "html_file" ? "html" : "text"),
    source: "open_editor_session",
    dirty,
    sessionVersion: sessionVersion ?? edStore.getSnapshot().sessions[relPath]?.sessionVersion ?? 1,
    contentHash: hashLogicalContent(content),
    capturedAt: Date.now(),
  };
}

function beginOpenRequest(relPath: string): number {
  const requestId = ++openRequestSeq;
  latestOpenRequestByPath.set(relPath, requestId);
  return requestId;
}

function isLatestOpenRequest(relPath: string, requestId: number): boolean {
  return latestOpenRequestByPath.get(relPath) === requestId;
}

function finishOpenRequest(relPath: string, requestId: number): void {
  if (isLatestOpenRequest(relPath, requestId)) latestOpenRequestByPath.delete(relPath);
}

/**
 * 只提交当前路径的最新打开结果。用户在读取期间切换标签时，结果仍可补齐该会话，
 * 但不会改写活动编辑器或触发一次迟到的 setContent。
 */
function applyOpenResult(
  relPath: string,
  requestId: number,
  content: string,
  baseline: number | string,
  jumpLine: number | null,
  editFormat: EditFormat,
  contentFormat?: ContentFormat,
): boolean {
  if (!isLatestOpenRequest(relPath, requestId)) return false;
  const activate = edStore.getSnapshot().activePath === relPath;
  edStore.loadDoc(relPath, content, baseline, jumpLine, editFormat, contentFormat, { activate });
  return true;
}

/**
 * @GOV
 * codes:    BR-ED-DATA-002-RB-ED-ED-EDIT-012, BR-ED-DATA-006-DATA-ED-ED-EDIT-016
 * type:     RB
 * chain:    ED-EDIT
 * rules:    BR-ED-DATA-002, BR-ED-DATA-006
 * boundary: in=workspaceDocumentPath | out=editSessionLogicalState(editFormatAndContentFormatExplicit)
 */
/// Markdown/BDocx/html 统一编辑模型守则（BR-ED-DATA-002/006）。
/** 打开文档：docx 经 DOC 编辑链（doc.open_for_edit → HTML），其余经 ws.read_doc（文本）→ 载入编辑器
 *  （内容 + 加载基线）；jumpLine 时载入后跳转到该行。 */
export async function openDocument(relPath: string, jumpLine: number | null = null): Promise<void> {
  const requestId = beginOpenRequest(relPath);
  const hadSession = Boolean(edStore.getSnapshot().sessions[relPath]);
  if (!hadSession) edStore.openDoc(relPath);
  try {
    if (isDocxEditable(relPath)) {
      const { html, baseline } = await openDocxForEdit(relPath);
      if (!applyOpenResult(relPath, requestId, html, baseline, jumpLine, "html", "html")) return;
      const sess = edStore.getSnapshot().sessions[relPath];
      if (sess) {
        await persistSessionSnapshot(makePersistedSnapshot(relPath, html, "html", false, sess.sessionVersion, "html"));
      }
      return;
    }
    const { content, baseline } = await readDoc(relPath);
    let logicalContent = content;
    let editFormat: EditFormat = isHtmlFile(relPath) ? "html_file" : "text";
    let contentFormat: ContentFormat = isHtmlFile(relPath) ? "html" : "text";
    if (isMarkdownFile(relPath)) {
      editFormat = "markdown";
      const parsed = parseMarkdownForEditing(content);
      logicalContent = parsed.ok ? parsed.html : parsed.source;
      contentFormat = parsed.contentFormat;
    }
    if (!applyOpenResult(relPath, requestId, logicalContent, baseline, jumpLine, editFormat, contentFormat)) return;
    const sess = edStore.getSnapshot().sessions[relPath];
    if (sess) {
      await persistSessionSnapshot(makePersistedSnapshot(relPath, logicalContent, editFormat, false, sess.sessionVersion, contentFormat));
    }
  } catch (e) {
    const err = e as IpcLikeError;
    if (!hadSession && isLatestOpenRequest(relPath, requestId)) edStore.closeSession(relPath);
    console.error(`open document failed [${err.code ?? "?"}]: ${err.message ?? e}`);
  } finally {
    finishOpenRequest(relPath, requestId);
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
 * @GOV
 * codes:    BR-ED-VERIFY-001-GUARD-ED-ED-EDIT-016
 * type:     GUARD
 * chain:    ED-EDIT
 * rules:    BR-ED-VERIFY-001, BR-ED-DATA-006
 * boundary: in=validatedMarkdownDiskCandidateAndLogicalState | out=WorkspaceSaveAndPersistedLogicalSnapshot(noFormatConflation)
 */
/// Markdown 写盘只接收已经过 Codec 能力/语义校验的 DiskState 候选；保存成功后 session 仍持有
/// LogicalState（HTML 或源码保护 text），不得把 Markdown 源码误写回 HTML LogicalState 槽位。
export async function saveMarkdownDocument(
  markdown: string,
  logicalContent: string,
  contentFormat: ContentFormat,
  override = false,
): Promise<SaveOutcome> {
  const { openPath, baseline } = edStore.getSnapshot();
  if (!openPath) return { ok: false, message: "无打开的文档" };
  const path = openPath;
  try {
    const newBaseline = await saveDoc(path, markdown, baseline, override);
    edStore.updateContent(path, logicalContent);
    edStore.setBaseline(path, newBaseline);
    const sess = edStore.getSnapshot().sessions[path];
    if (sess) {
      await persistSessionSnapshot(
        makePersistedSnapshot(path, logicalContent, "markdown", false, sess.sessionVersion, contentFormat),
      );
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
