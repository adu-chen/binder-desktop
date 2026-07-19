import { invoke } from "@tauri-apps/api/core";
import { deStore, type DiffView, type ExpiredReason } from "../store/deStore";
import { edStore } from "../store/edStore";
import { openDocument, persistSessionSnapshot } from "./edService";
import { applyEditOps, getActiveDocumentContent } from "./edLocate";

/**
 * DE 模块 Service（X-SEQ-002）：diff 生成/裁决经 dispatch 信封触达后端（PI-BE-02 的 de.*）。
 * 当前编辑器草稿 diff 的 accepted 只确认裁决态，落盘归 ED→WS save；后台 text/html diff 保留旧 apply 路径。
 * 示例修改入口用确定性本地变换绕过仍是桩的 AG 模型，使 DE→WS 落盘链端到端可演示。
 */

interface CommandResponse<T> {
  correlation_id: string;
  result?: T;
  error?: { code: string; message: string; retriable: boolean; reauth_required: boolean };
}

let seq = 0;
async function callCommand<T>(command: string, params: Record<string, unknown>): Promise<T> {
  const request = { command, correlation_id: `de-${Date.now()}-${(seq += 1)}`, params };
  const response = await invoke<CommandResponse<T>>("dispatch", { request });
  if (response.error) throw response.error;
  if (response.result === undefined) {
    throw { code: "IPC_EMPTY_RESULT", message: `${command} 无返回`, retriable: false, reauth_required: false };
  }
  return response.result;
}

const snippet = (s: string, n = 48) => (s.length > n ? `${s.slice(0, n)}…` : s);
const isDocxPath = (path?: string) => Boolean(path && /\.(docx|doc)$/i.test(path));
const errMsg = (e: unknown) => {
  const x = e as { code?: string; message?: string };
  return x?.code ? `[${x.code}] ${x.message ?? ""}` : String(e);
};

interface PendingDiffDto {
  diff_id: string;
  target: { rel_path: string };
  status: "pending" | "accepted" | "rejected" | "expired";
  created_at: number;
}

export interface DiffDetailDto extends PendingDiffDto {
  projection?: {
    patch_kind?: "replace_block" | "replace_document" | "style_ops";
    block_id?: string | null;
    block_index?: number | null;
    before_text?: string;
    after_text?: string;
    location_label?: string;
    draft_mode?: "current_editor" | "open_session" | "background";
    snapshot_source?: "active_editor" | "active_editor_session" | "open_editor_session" | "disk_snapshot";
    base_session_version?: number | null;
    base_content_hash?: string | null;
    style_ops?: unknown;
  };
}

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function replaceTextBlock(content: string, blockIndex: number | undefined, nextText: string): string | null {
  if (blockIndex === undefined || blockIndex < 0) return null;
  const blocks = content.split("\n");
  if (blockIndex >= blocks.length) return null;
  blocks[blockIndex] = nextText;
  return blocks.join("\n");
}

function findTagClose(content: string, tag: string, from: number): number {
  return content.toLowerCase().indexOf(`</${tag.toLowerCase()}>`, from);
}

function replaceHtmlBlock(content: string, blockId: string | undefined, blockIndex: number | undefined, nextText: string): string | null {
  const escaped = escapeHtml(nextText);
  const blocks: { id?: string; start: number; end: number }[] = [];
  const re = /<([a-zA-Z][\w:-]*)\b[^>]*>/g;
  let match: RegExpExecArray | null;
  while ((match = re.exec(content)) !== null) {
    const openTag = match[0];
    if (!/\bdata-block-id\s*=/.test(openTag) && !/^(p|h[1-6]|li|blockquote|pre|div|td|th)$/i.test(match[1])) {
      continue;
    }
    const innerStart = match.index + openTag.length;
    const closeStart = findTagClose(content, match[1], innerStart);
    if (closeStart === -1) continue;
    const idMatch = openTag.match(/\bdata-block-id\s*=\s*["']([^"']+)["']/i);
    blocks.push({ id: idMatch?.[1], start: innerStart, end: closeStart });
  }
  const target = blockId
    ? blocks.find((b) => b.id === blockId)
    : blockIndex === undefined ? undefined : blocks[blockIndex];
  if (!target) return null;
  return `${content.slice(0, target.start)}${escaped}${content.slice(target.end)}`;
}

function applyOpenSessionPatch(diff: DiffView, content: string, editFormat: string): string | null {
  if (diff.patchKind === "style_ops") return null;
  if (diff.patchKind === "replace_document") return diff.after;
  if (diff.patchKind === "replace_block") {
    const index = typeof diff.blockIndex === "number" ? diff.blockIndex : undefined;
    if (editFormat === "html" || editFormat === "html_file") {
      return replaceHtmlBlock(content, diff.blockId || undefined, index, diff.after);
    }
    return replaceTextBlock(content, index, diff.after);
  }
  return null;
}

async function expireDiff(diffId: string, reason: ExpiredReason, errorCode: string): Promise<void> {
  try {
    await callCommand("de.expire_diff", { diffId, reason: errorCode });
  } catch (e) {
    console.error(`de.expire_diff failed ${errMsg(e)}`);
  }
  deStore.expireLocal(diffId, { expiredReason: reason, errorCode });
}

export async function loadDiffDetails(diffIds: string[]): Promise<DiffDetailDto[]> {
  const ids = Array.from(new Set(diffIds.filter(Boolean)));
  if (!ids.length) return [];
  return callCommand<DiffDetailDto[]>("de.list_diffs", { diffIds: ids });
}

export async function loadPersistedDiffs(): Promise<void> {
  try {
    const list = await callCommand<PendingDiffDto[]>("de.list_diffs", { status: "pending" });
    const existing = new Set(deStore.getSnapshot().diffs.map((d) => d.diffId));
    for (const d of list) {
      if (existing.has(d.diff_id)) continue;
      existing.add(d.diff_id);
      deStore.addDiff({
        diffId: d.diff_id,
        blockId: "",
        relPath: d.target.rel_path,
        title: `待裁决修改 ${d.target.rel_path}`,
        before: "",
        after: "",
        state: "pending",
        createdAt: d.created_at,
      });
    }
  } catch (e) {
    console.error(`de.list_diffs failed ${errMsg(e)}`);
  }
}

export async function applyOpenSessionDiff(diffId: string): Promise<void> {
  const diff = deStore.getSnapshot().diffs.find((d) => d.diffId === diffId);
  if (!diff || diff.draftMode !== "open_session" || diff.state !== "pending") return;
  if (!diff.relPath) {
    await expireDiff(diffId, "block_resolve_failed", "DE_OPEN_SESSION_REL_PATH_MISSING");
    return;
  }
  if (diff.relPath === edStore.getSnapshot().openPath) {
    await expireDiff(diffId, "document_revision_advanced", "DE_OPEN_SESSION_ACTIVE_PATH");
    return;
  }
  if (typeof diff.baseSessionVersion !== "number" || !diff.baseContentHash) {
    await expireDiff(diffId, "document_revision_advanced", "DE_OPEN_SESSION_BASE_MISSING");
    return;
  }
  const session = edStore.getSnapshot().sessions[diff.relPath];
  if (!session || session.content === null) {
    await expireDiff(diffId, "block_resolve_failed", "ED_OPEN_SESSION_REQUIRED");
    return;
  }
  if (diff.patchKind === "style_ops") {
    await expireDiff(diffId, "apply_replace_failed", "ED_OPEN_SESSION_STYLE_OPS_UNSUPPORTED");
    return;
  }
  const nextContent = applyOpenSessionPatch(diff, session.content, session.editFormat);
  if (nextContent === null) {
    await expireDiff(diffId, "block_resolve_failed", "ED_OPEN_SESSION_APPLY_FAILED");
    return;
  }
  const applied = edStore.applySessionContent(
    diff.relPath,
    nextContent,
    diff.baseSessionVersion,
    diff.baseContentHash,
  );
  if (!applied.ok) {
    await expireDiff(diffId, "document_revision_advanced", applied.code);
    return;
  }
  try {
    await persistSessionSnapshot(applied.snapshot);
    await callCommand<{ accepted: boolean }>("de.accept_draft_diff", {
      diffId,
      applyContext: "open_session",
      baseSessionVersion: diff.baseSessionVersion,
      baseContentHash: diff.baseContentHash,
    });
    deStore.markOpenSessionApplied(diffId, session.content, nextContent);
  } catch (e) {
    deStore.setLocalView(diffId, "error", { errorCode: (e as { code?: string })?.code ?? "DE_OPEN_SESSION_CONFIRM_FAILED" });
    console.error(`open session diff confirm failed ${errMsg(e)}`);
  }
}

/**
 * 对当前打开文档生成示例修改（绕过桩模型）：确定性变换 → de.generate_diff 持久化 → deStore 投影。
 * 无打开文档则忽略。before/after 预览本地保留（DiffCard 展示），真实内容已持久化为 DiffPayload。
 */
export async function generateSampleDiff(): Promise<void> {
  const { openPath, loadedContent, editFormat } = edStore.getSnapshot();
  if (!openPath) return;
  const baseContent = getActiveDocumentContent(editFormat) ?? loadedContent;
  if (baseContent === null) return;
  const after = `${baseContent}\n\n> （示例修改）AI 追加的一段说明文字。`;
  try {
    const { diff_id } = await callCommand<{ diff_id: string }>("de.generate_diff", {
      relPath: openPath,
      afterContent: after,
      originModule: "AG",
    });
    deStore.addDiff({
      diffId: diff_id,
      blockId: "", // 整文档变换，不绑定单块 blockId（块级 GreenReview 留待块级 diff）
      relPath: openPath, // 按文件归属，供文件树/标签投影 pending
      title: "对当前文档追加一段",
      before: snippet(baseContent.slice(-48)),
      after: "…追加：AI 追加的一段说明文字。",
      state: "pending",
    });
  } catch (e) {
    console.error(`de.generate_diff failed ${errMsg(e)}`);
  }
}

/**
 * 接受单条：已应用到打开编辑器的草稿 diff 只确认裁决态，不再触发 WS apply；保存由普通 ⌘S/save 链落盘。
 * 非当前编辑器草稿的 text/html diff 保留旧 de.accept_diff→ws.apply_diff 路径；BDocx 必须先进入编辑器草稿。
 *
 * D5 路径选择表见 A-DE-M-S-01 §4（A-CORE-C-T-01 §5.5 决议落地）：
 *   draftApplied=true → 草稿路径（de.accept_draft_diff）
 *   isDocxPath → 拒绝（必须先进草稿）
 *   其余 → 后台路径（de.accept_diff → ws.apply_diff）
 */
export async function acceptDiff(diffId: string): Promise<void> {
  try {
    const diff = deStore.getSnapshot().diffs.find((d) => d.diffId === diffId);
    const { openPath } = edStore.getSnapshot();
    if (diff?.draftMode === "open_session") {
      await applyOpenSessionDiff(diffId);
      return;
    }
    if (diff?.draftApplied && diff.relPath === openPath) {
      await callCommand<{ accepted: boolean }>("de.accept_draft_diff", { diffId });
      return;
    }
    if (diff?.relPath && edStore.getSnapshot().sessions[diff.relPath]) {
      deStore.setLocalView(diffId, "error", { errorCode: "DE_OPEN_DOC_REQUIRES_LOGICAL_STATE" });
      return;
    }
    if (diff && isDocxPath(diff.relPath)) {
      deStore.setLocalView(diffId, "error", { errorCode: "DE_BDOCX_DRAFT_REQUIRED" });
      return;
    }

    const entry = await callCommand<{ entry_id: string; version_seq: number }>("de.accept_diff", { diffId });
    // applying/accepted 经 SM-WS-02 + SM-DE-01 广播到达；此处仅回填展示元数据 + 重载编辑器。
    deStore.attachMeta(diffId, { timelineSeq: entry.version_seq });
    if (openPath) await openDocument(openPath);
  } catch (e) {
    const x = e as { code?: string };
    deStore.setLocalView(diffId, "error", { errorCode: x?.code ?? "DE_ACCEPT_FAILED" });
    console.error(`de.accept_diff failed ${errMsg(e)}`);
  }
}

/** 拒绝单条：de.reject_diff（不写文件）；若 after 已进编辑器草稿，则按 before 回滚当前块。 */
export async function rejectDiff(diffId: string): Promise<void> {
  try {
    const diff = deStore.getSnapshot().diffs.find((d) => d.diffId === diffId);
    if (diff?.draftMode === "open_session" || diff?.readonly) return;
    await callCommand("de.reject_diff", { diffId });
    const { openPath, editFormat } = edStore.getSnapshot();
    if (
      diff?.draftApplied &&
      diff.relPath === openPath &&
      (!diff.blockId || diff.styleOpsPayload?.length) &&
      diff.draftBaseContent !== undefined
    ) {
      // D4 后置编辑检测（A-CORE-C-T-01 §5.4）：比较当前内容与 draftAfterContent。
      // 相等 = diff 生效后无用户编辑，不等 = 用户又编辑过内容。
      const preRollbackContent = getActiveDocumentContent(editFormat);
      const hasPostDraftEdit = diff.draftAfterContent !== undefined
        && preRollbackContent !== null
        && preRollbackContent !== diff.draftAfterContent;
      const restored = applyEditOps([{ op: "replace_document", content: diff.draftBaseContent, format: editFormat }]);
      if (!restored.ok) {
        deStore.setLocalView(diffId, "error", { errorCode: "ED_DRAFT_ROLLBACK_RESTORE_FAILED" });
        return;
      }
      edStore.updateContent(openPath, diff.draftBaseContent);
      // draftBaseDirty（前置脏态）与 hasPostDraftEdit（后置编辑）二者取或。
      if (diff.draftBaseDirty || hasPostDraftEdit) {
        edStore.setTabState(openPath, "dirty");
      } else if (diff.draftBaseTabState) {
        edStore.setTabState(openPath, diff.draftBaseTabState);
      }
      return;
    }
    if (diff?.draftApplied && diff.blockId && diff.relPath === openPath) {
      const located = applyEditOps([{ op: "replace_block", blockId: diff.blockId, text: diff.before }]);
      if (!located.ok) deStore.setLocalView(diffId, "error", { errorCode: "ED_DRAFT_ROLLBACK_ANCHOR_LOST" });
      const currentContent = getActiveDocumentContent(editFormat);
      if (located.ok && currentContent !== null) {
        edStore.updateContent(openPath, currentContent);
        // 块级路径：如果 currentContent !== draftBaseContent，其他块可能有未保存编辑，
        // tabState 已由 editor onUpdate→markDirty 维护，不需额外恢复。
        if (currentContent === diff.draftBaseContent) {
          if (diff.draftBaseDirty) {
            edStore.setTabState(openPath, "dirty");
          } else if (diff.draftBaseTabState) {
            edStore.setTabState(openPath, diff.draftBaseTabState);
          }
        }
      }
    }
  } catch (e) {
    console.error(`de.reject_diff failed ${errMsg(e)}`);
  }
}

/**
 * 全部接受（倒序）：从文档靠后的 diff 先 de.accept_diff，避免先序落盘造成后序位置偏移（参考稿 §6.5）。
 * 过期/冲突/error 不计入可应用集，计入失效统计。
 */
export async function acceptAllDiffs(): Promise<void> {
  const { diffs } = deStore.getSnapshot();
  const appliable = diffs.filter((d) => d.state === "pending" && !d.readonly);
  const expired = diffs.filter((d) => ["expired", "conflict", "error"].includes(d.state));
  if (appliable.length === 0) return;
  deStore.setBatch("applying", expired.length);
  for (const d of [...appliable].reverse()) {
    await acceptDiff(d.diffId);
  }
  deStore.setBatch("done", expired.length);
}

export async function rejectAllDiffs(): Promise<void> {
  const { diffs } = deStore.getSnapshot();
  for (const d of diffs.filter((x) => x.state === "pending" && !x.readonly)) {
    await rejectDiff(d.diffId);
  }
  deStore.setBatch("idle");
}
