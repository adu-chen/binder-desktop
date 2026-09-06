import { invoke } from "@tauri-apps/api/core";
import { deStore, type DiffView, type ExpiredReason } from "../store/deStore";
import { edStore } from "../store/edStore";
import { closeStore } from "../store/closeStore";
import { tabsStore } from "../store/tabsStore";
import { openDocument, persistSessionSnapshot } from "./edService";
import { loadDocs } from "./wsService";
import { applyEditOps, getActiveDocumentContent, getActiveEditor } from "./edLocate";
import type { DocumentImageSnapshot } from "./edImageOps";
import {
  applyStructuredBlockOperations,
  validateStructuredBlockOperations,
} from "./edBlockOps";

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
// isDocxPath 已移除（2026-07-26 · D2）：BDocx background diff 不再阻断，直写盘
const errMsg = (e: unknown) => {
  const x = e as { code?: string; message?: string };
  return x?.code ? `[${x.code}] ${x.message ?? ""}` : String(e);
};

interface PendingDiffDto {
  diff_id: string;
  target: { rel_path: string };
  status: "pending" | "accepted" | "rejected" | "expired";
  effect_status?: "not_applicable" | "awaiting_save" | "saved" | "discarded_on_abnormal_exit";
  terminal_reason?: string | null;
  created_at: number;
  projection?: {
    proposal_batch_id?: string;
    group_index?: number;
    group_count?: number;
    patch_kind?: DiffView["patchKind"];
    block_id?: string | null;
    block_index?: number | null;
    before_text?: string;
    after_text?: string;
    location_label?: string;
    draft_mode?: DiffView["draftMode"];
    snapshot_source?: DiffView["snapshotSource"];
    base_session_version?: number | null;
    base_content_hash?: string | null;
  };
}

export interface DiffDetailDto extends PendingDiffDto {
  projection?: {
    proposal_batch_id?: string;
    group_index?: number;
    group_count?: number;
    patch_kind?: "replace_block" | "block_ops" | "replace_document" | "style_ops" | "table_ops" | "image_ops";
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
    block_ops?: unknown;
    table_ops?: unknown;
    image_ops?: unknown;
    image_before?: unknown;
    image_after?: unknown;
    image_affected_block_ids?: unknown;
  };
}

function replaceTextBlock(content: string, blockIndex: number | undefined, nextText: string): string | null {
  if (blockIndex === undefined || blockIndex < 0) return null;
  // 审计修复（2026-07-24）：统一换行符为 \n，防止 \r\n 导致 split 偏移
  const normalized = content.replace(/\r\n/g, "\n").replace(/\r/g, "\n");
  const blocks = normalized.split("\n");
  if (blockIndex >= blocks.length) return null;
  blocks[blockIndex] = nextText;
  return blocks.join("\n");
}

function applyOpenSessionPatch(diff: DiffView, content: string, editFormat: string, contentFormat: string): string | null {
  if (diff.patchKind === "block_ops" || diff.patchKind === "style_ops" || diff.patchKind === "table_ops" || diff.patchKind === "image_ops") return null;
  const isRich = contentFormat === "html" || editFormat === "html" || editFormat === "html_file";
  if (isRich) return null;
  if (diff.patchKind === "replace_document") return diff.after;
  if (diff.patchKind === "replace_block") {
    const index = typeof diff.blockIndex === "number" ? diff.blockIndex : undefined;
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

/**
 * @GOV
 * codes:    BR-SYS-STATE-008-EFFECT-DE-DE-DIFF-008
 * type:     EFFECT
 * chain:    DE-DIFF, ED-EDIT
 * rules:    BR-SYS-STATE-008
 * boundary: in=NewProposalBaseAdvanced | out=PriorStalePendingCardsExpired(documentRevisionAdvanced) | does_not=rollback_or_touch_disk
 */
/** 连续 Proposal 接力（BR-SYS-STATE-008 / REQ-DE-009）：后续 Proposal 基于最新 LogicalState 生成/预览
 * 成功后，使同文档所有基于旧基准的 pending 卡失效（document_revision_advanced）。旧卡保留历史、不可再
 * 裁决；不构成补丁栈、无任务级回滚、不触碰磁盘内容。并行同基准卡（同版本同哈希）不受影响；background
 * pending 卡（无 LogicalState 基准）不参与。 */
export async function expireStalePendingForPath(
  relPath: string,
  exceptDiffId: string,
  base: { sessionVersion?: number | null; contentHash?: string | null } | null,
): Promise<void> {
  const diffs = deStore.getSnapshot().diffs;
  for (const d of diffs) {
    if (d.diffId === exceptDiffId) continue;
    if (d.relPath !== relPath) continue;
    if (d.state !== "pending") continue;
    const stale =
      (typeof base?.sessionVersion === "number" &&
        typeof d.baseSessionVersion === "number" &&
        d.baseSessionVersion < base.sessionVersion) ||
      (base?.contentHash != null && d.baseContentHash != null && d.baseContentHash !== base.contentHash);
    if (stale) {
      await expireDiff(d.diffId, "document_revision_advanced", "DE_CONSECUTIVE_PROPOSAL_ADVANCED");
    }
  }
}

export async function loadDiffDetails(diffIds: string[]): Promise<DiffDetailDto[]> {
  const ids = Array.from(new Set(diffIds.filter(Boolean)));
  if (!ids.length) return [];
  return callCommand<DiffDetailDto[]>("de.list_diffs", { diffIds: ids });
}

/**
 * @GOV
 * codes:    BR-DE-DATA-002-EFFECT-DE-DE-DIFF-009
 * type:     EFFECT
 * chain:    DE-DIFF
 * rules:    BR-DE-DATA-002
 * term_ref: TERM-DE-004, TERM-ED-008
 * boundary: in=ImageOperationDiffIdAndBeforeAfterImageSnapshots | out=PersistedImageReviewSnapshotAcknowledgement | effect=IPC-de.update_image_diff_snapshots | does_not=write_document_content_or_change_diff_terminal
 */
export async function persistImageDiffSnapshots(
  diffId: string,
  before: DocumentImageSnapshot[],
  after: DocumentImageSnapshot[],
  affectedBlockIds: string[],
): Promise<void> {
  await callCommand<{ persisted: boolean }>("de.update_image_diff_snapshots", {
    diffId,
    before,
    after,
    affectedBlockIds,
  });
}

export async function loadPersistedDiffs(): Promise<void> {
  try {
    const list = await callCommand<PendingDiffDto[]>("de.list_diffs", { status: "pending" });
    const existing = new Set(deStore.getSnapshot().diffs.map((d) => d.diffId));
    for (const d of list) {
      if (existing.has(d.diff_id)) continue;
      existing.add(d.diff_id);
      const projection = d.projection;
      const batchId = projection?.proposal_batch_id;
      const groupIndex = projection?.group_index;
      const groupCount = projection?.group_count;
      const validBatch = typeof batchId === "string"
        && Boolean(batchId.trim())
        && Number.isSafeInteger(groupIndex)
        && Number.isSafeInteger(groupCount)
        && (groupIndex ?? 0) >= 1
        && (groupCount ?? 0) >= 1
        && (groupIndex ?? 0) <= (groupCount ?? 0);
      deStore.addDiff({
        diffId: d.diff_id,
        blockId: projection?.block_id ?? "",
        blockIndex: projection?.block_index ?? null,
        relPath: d.target.rel_path,
        locationLabel: projection?.location_label ?? "历史修改建议",
        patchKind: projection?.patch_kind,
        draftMode: projection?.draft_mode,
        snapshotSource: projection?.snapshot_source,
        baseSessionVersion: projection?.base_session_version ?? undefined,
        baseContentHash: projection?.base_content_hash ?? undefined,
        title: validBatch
          ? `待裁决修改 ${d.target.rel_path}`
          : `待裁决修改批次信息不可用 ${d.target.rel_path}`,
        before: projection?.before_text ?? "",
        after: projection?.after_text ?? "",
        state: validBatch ? "pending" : "error",
        errorCode: validBatch ? undefined : "DE_PROPOSAL_BATCH_INVALID",
        readonly: !validBatch,
        proposalBatch: validBatch ? {
          id: batchId!,
          groupIndex: groupIndex!,
          groupCount: groupCount!,
        } : undefined,
        createdAt: d.created_at,
      });
    }
  } catch (e) {
    console.error(`de.list_diffs failed ${errMsg(e)}`);
  }
}

/**
 * @GOV
 * codes:    BR-CORE-STATE-001-EFFECT-DE-ED-EDIT-010
 * type:     EFFECT
 * chain:    DE-DIFF, ED-EDIT
 * rules:    BR-CORE-STATE-001
 * boundary: in=PendingDiffForExistingInactiveLogicalState | out=ControlledLogicalStatePreviewWithoutAcceptanceOrDiskWrite
 */
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
  if (diff.patchKind === "block_ops" || diff.patchKind === "style_ops" || diff.patchKind === "table_ops" || diff.patchKind === "image_ops") {
    await expireDiff(diffId, "apply_replace_failed", "ED_OPEN_SESSION_STRUCTURED_OPS_UNSUPPORTED");
    return;
  }
  const nextContent = applyOpenSessionPatch(diff, session.content, session.editFormat, session.contentFormat);
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
  // 审计修复（2026-07-26 · D1）：open_session diff 不再 auto-accept。apply 只修改 session 内容，
  // 不调用 de.accept_draft_diff。accept 由后续用户操作驱动。
  await persistSessionSnapshot(applied.snapshot);
  deStore.markDraftApplied(diffId, session.content, session.tabState, nextContent);
  // 连续 Proposal 接力（BR-SYS-STATE-008 / REQ-DE-009）：本卡成功预览推进 session 基准，
  // 使同文档基于旧基准的 pending 卡失效（保留历史、不可裁决；不产生任务级回滚）。
  await expireStalePendingForPath(diff.relPath, diffId, {
    sessionVersion: applied.snapshot.sessionVersion,
    contentHash: applied.snapshot.contentHash,
  });
}

/**
 * @GOV
 * codes:    BR-CORE-STATE-003-EFFECT-DE-ED-EDIT-011
 * type:     EFFECT
 * chain:    DE-DIFF, ED-EDIT, ED-DECOR, WS-SAVE
 * rules:    BR-CORE-STATE-003
 * boundary: in=PendingBackgroundDiffAfterTargetOpenCreatesLogicalState | out=PendingDiffCardRoutedToLogicalStatePreviewAndSavePath
 */
/** 背景 PendingDiff 的目标在卡片待裁决期间被打开后，编辑器已先建立 LogicalState。
 * 此处只迁移 DiffCard 的分流归属，保持 pending；后续预览、接受/拒绝均按 LogicalState 路径执行。 */
export async function promotePendingDiffOnOpen(diffId: string, blockId = ""): Promise<void> {
  const diff = deStore.getSnapshot().diffs.find((item) => item.diffId === diffId);
  if (!diff || diff.draftMode !== "background" || diff.state !== "pending") return;
  try {
    await callCommand<{ pending: boolean }>("de.promote_pending_diff_on_open", { diffId });
    deStore.migrateDraftMode(diffId, blockId || diff.blockId, "current_editor");
  } catch (e) {
    deStore.setLocalView(diffId, "error", { errorCode: (e as { code?: string }).code ?? "DE_PROMOTE_ON_OPEN_FAILED" });
    console.error(`de.promote_pending_diff_on_open failed ${errMsg(e)}`);
  }
}

/**
 * 对当前打开文档生成示例修改（绕过桩模型）：确定性变换 → de.generate_diff 持久化 → deStore 投影。
 * 无打开文档则忽略。before/after 预览本地保留（DiffCard 展示），真实内容已持久化为 DiffPayload。
 */
export async function generateSampleDiff(): Promise<void> {
  // 关闭屏障（BR-SYS-STATE-004，APP-CLOSE-DIFF-LIFECYCLE-01）：关闭请求建立后拒绝新 diff 生成。
  if (closeStore.getSnapshot().phase !== "idle") {
    console.error("de.generate_diff blocked: application close in progress");
    return;
  }
  const { openPath, loadedContent, editFormat, contentFormat } = edStore.getSnapshot();
  if (!openPath) return;
  const baseContent = getActiveDocumentContent(editFormat, contentFormat) ?? loadedContent;
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
 * @GOV
 * codes:    BR-CORE-STATE-001-API-DE-DE-DIFF-001, BR-CORE-STATE-002-API-DE-DE-DIFF-001, BR-CORE-STATE-003-API-DE-DE-DIFF-001
 * type:     API
 * chain:    DE-DIFF, ED-EDIT, WS-SAVE, WS-DIFF-APPLY
 * rules:    BR-CORE-STATE-001, BR-CORE-STATE-002, BR-CORE-STATE-003
 * boundary: in=UserDiffCardAccept | out=LogicalStateLifecycleOnlyOrBackgroundAtomicDiskApply
 */
/** 接受单条：已应用到打开编辑器的草稿 diff 只确认裁决态，不再触发 WS apply；保存由普通 ⌘S/save 链落盘。
 * 无 LogicalState 的后台 diff 才保留 de.accept_diff→ws.apply_diff 路径；BDocx 必须先进入编辑器草稿。
 *
 * D5 路径选择表见 A-DE-M-S-01 §4（A-CORE-C-T-01 §5.5 决议落地）：
 *   draftApplied=true → 草稿路径（de.accept_draft_diff）
 *   isDocxPath → 拒绝（必须先进草稿）
 *   其余 → 后台路径（de.accept_diff → ws.apply_diff）
 */
export async function acceptDiff(diffId: string): Promise<void> {
  try {
    const diff = deStore.getSnapshot().diffs.find((d) => d.diffId === diffId);
    // 当前编辑器预览尚未完成时，不能把 pending proposal 误走成后台写盘接受。
    if (diff?.draftPreviewState === "queued") return;
    if (diff?.state === "error" && !diff.draftApplied) return;
    const { openPath } = edStore.getSnapshot();
    if (diff?.draftApplied && diff.blockOpsPayload) {
      const activeEditor = getActiveEditor();
      if (!activeEditor || diff.relPath !== openPath) {
        deStore.setLocalView(diffId, "conflict", { errorCode: "ED_BLOCK_OPERATION_EDITOR_MISSING" });
        return;
      }
      const preflight = validateStructuredBlockOperations(
        activeEditor,
        diff.blockOpsPayload,
        "reverse",
      );
      if (!preflight.ok) {
        deStore.setLocalView(diffId, "conflict", { errorCode: preflight.reason });
        return;
      }
    }
    if (diff?.draftMode === "open_session") {
      // open_session 在 diff 返回时已受控预览；此处仅由用户显式确认卡片。
      await applyOpenSessionDiff(diffId);
      if (deStore.getSnapshot().diffs.find((d) => d.diffId === diffId)?.draftApplied) {
        await callCommand<{ accepted: boolean }>("de.accept_draft_diff", {
          diffId,
          applyContext: "open_session",
          baseSessionVersion: diff.baseSessionVersion,
          baseContentHash: diff.baseContentHash,
        });
      }
      return;
    }
    if (diff?.draftApplied && diff.relPath === openPath) {
      const session = edStore.getSnapshot().sessions[diff.relPath];
      const result = await callCommand<{ accepted: boolean; expired?: boolean; reason?: string }>("de.accept_draft_diff", {
        diffId,
        applyContext: "current_editor",
        ...(session
          ? {
              logicalSessionVersion: session.sessionVersion,
              logicalContentHash: session.contentHash,
            }
          : {}),
      });
      if (result.expired) {
        deStore.expireLocal(diffId, {
          expiredReason: "document_revision_advanced",
          errorCode: result.reason ?? "WS_CREATE_BUNDLE_BASELINE_ADVANCED",
        });
      }
      return;
    }
    if (diff?.relPath && edStore.getSnapshot().sessions[diff.relPath]) {
      deStore.setLocalView(diffId, "error", { errorCode: "DE_OPEN_DOC_REQUIRES_LOGICAL_STATE" });
      return;
    }
    // 审计修复（2026-07-26 · D2）：无逻辑状态→直写盘（BDocx 也不例外）。session 不存在已验证。 |

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
    if (diff?.readonly) return;
    const inactiveDraft = diff?.draftApplied && diff.relPath && diff.relPath !== edStore.getSnapshot().openPath
      ? {
          relPath: diff.relPath,
          afterContent: diff.draftAfterContent,
          baseContent: diff.draftBaseContent,
          baseDirty: diff.draftBaseDirty,
          baseTabState: diff.draftBaseTabState,
        }
      : undefined;
    const inactiveSession = inactiveDraft ? edStore.getSnapshot().sessions[inactiveDraft.relPath] : undefined;
    if (inactiveSession && inactiveDraft?.afterContent !== undefined && inactiveSession.content !== inactiveDraft.afterContent) {
      deStore.setLocalView(diffId, "conflict", { errorCode: "ED_OPEN_SESSION_AFTER_MISMATCH" });
      return;
    }
    const activeEditor = diff?.draftApplied && diff.blockOpsPayload ? getActiveEditor() : null;
    if (diff?.draftApplied && diff.blockOpsPayload) {
      if (!activeEditor || diff.relPath !== edStore.getSnapshot().openPath) {
        deStore.setLocalView(diffId, "conflict", { errorCode: "ED_BLOCK_OPERATION_EDITOR_MISSING" });
        return;
      }
      const preflight = validateStructuredBlockOperations(
        activeEditor,
        diff.blockOpsPayload,
        "reverse",
      );
      if (!preflight.ok) {
        deStore.setLocalView(diffId, "conflict", { errorCode: preflight.reason });
        return;
      }
    }
    const bundleSession = diff?.relPath ? edStore.getSnapshot().sessions[diff.relPath] : undefined;
    const result = await callCommand<{
      status: "rejected" | "expired";
      resource_discarded?: boolean;
      reason?: string;
    }>("de.reject_diff", {
      diffId,
      ...(bundleSession
        ? {
            logicalSessionVersion: bundleSession.sessionVersion,
            logicalContentHash: bundleSession.contentHash,
          }
        : {}),
    });
    if (result.status === "expired") {
      deStore.expireLocal(diffId, {
        expiredReason: "document_revision_advanced",
        errorCode: result.reason ?? "WS_CREATE_BUNDLE_BASELINE_ADVANCED",
      });
      return;
    }
    if (result.resource_discarded && diff?.relPath) {
      const tab = tabsStore.getSnapshot().tabs.find((candidate) => (
        candidate.kind === "doc" && candidate.path === diff.relPath
      ));
      if (tab) tabsStore.close(tab.id, { force: true });
      await loadDocs();
      return;
    }
    const { openPath, editFormat, contentFormat } = edStore.getSnapshot();
    if (inactiveSession && inactiveDraft?.baseContent !== undefined) {
      edStore.updateContent(inactiveDraft.relPath, inactiveDraft.baseContent);
      edStore.setTabState(
        inactiveDraft.relPath,
        inactiveDraft.baseDirty ? "dirty" : (inactiveDraft.baseTabState ?? "normal"),
      );
      const restored = edStore.getSnapshot().sessions[inactiveDraft.relPath];
      if (restored?.content !== null) {
        await persistSessionSnapshot({
          relPath: inactiveDraft.relPath,
          content: restored.content,
          editFormat: restored.editFormat,
          contentFormat: restored.contentFormat,
          source: "open_editor_session",
          dirty: restored.tabState === "dirty",
          sessionVersion: restored.sessionVersion,
          contentHash: restored.contentHash,
          capturedAt: Date.now(),
        });
      }
      return;
    }
    if (
      diff?.draftApplied
      && diff.blockOpsPayload
      && diff.relPath === openPath
      && activeEditor
    ) {
      const reversed = applyStructuredBlockOperations(
        activeEditor,
        diff.blockOpsPayload,
        "reverse",
      );
      if (!reversed.ok) {
        deStore.setLocalView(diffId, "error", { errorCode: reversed.reason });
        return;
      }
      const currentContent = getActiveDocumentContent(editFormat, contentFormat);
      if (currentContent !== null) {
        edStore.updateContent(openPath, currentContent);
        if (currentContent === diff.draftBaseContent) {
          if (diff.draftBaseDirty) {
            edStore.setTabState(openPath, "dirty");
          } else if (diff.draftBaseTabState) {
            edStore.setTabState(openPath, diff.draftBaseTabState);
          }
        } else {
          edStore.setTabState(openPath, "dirty");
        }
      }
      return;
    }
    if (
      diff?.draftApplied &&
      diff.relPath === openPath &&
      (!diff.blockId || diff.styleOpsPayload?.length || diff.tableOpsPayload || diff.imageOpsPayload) &&
      diff.draftBaseContent !== undefined
    ) {
      // D4 后置编辑检测（A-CORE-C-T-01 §5.4）：比较当前内容与 draftAfterContent。
      // 相等 = diff 生效后无用户编辑，不等 = 用户又编辑过内容。
      const preRollbackContent = getActiveDocumentContent(editFormat, contentFormat);
      const hasPostDraftEdit = diff.draftAfterContent !== undefined
        && preRollbackContent !== null
        && preRollbackContent !== diff.draftAfterContent;
      const restored = applyEditOps([{ op: "replace_document", content: diff.draftBaseContent, format: editFormat, contentFormat }]);
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
      const currentContent = getActiveDocumentContent(editFormat, contentFormat);
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
