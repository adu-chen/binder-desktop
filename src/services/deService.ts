import { invoke } from "@tauri-apps/api/core";
import { deStore } from "../store/deStore";
import { edStore } from "../store/edStore";
import { openDocument } from "./edService";

/**
 * DE 模块 Service（X-SEQ-002）：diff 生成/裁决经 dispatch 信封触达后端（PI-BE-02 的 de.*）。
 * accepted 唯一经 de.accept_diff→ws.apply_diff 真落盘（X-INTER-001）；权威落盘态归后端（X-FORBID-007）。
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
const errMsg = (e: unknown) => {
  const x = e as { code?: string; message?: string };
  return x?.code ? `[${x.code}] ${x.message ?? ""}` : String(e);
};

/**
 * 对当前打开文档生成示例修改（绕过桩模型）：确定性变换 → de.generate_diff 持久化 → deStore 投影。
 * 无打开文档则忽略。before/after 预览本地保留（DiffCard 展示），真实内容已持久化为 DiffPayload。
 */
export async function generateSampleDiff(): Promise<void> {
  const { openPath, loadedContent } = edStore.getSnapshot();
  if (!openPath || loadedContent === null) return;
  const after = `${loadedContent}\n\n> （示例修改）AI 追加的一段说明文字。`;
  try {
    const { diff_id } = await callCommand<{ diff_id: string }>("de.generate_diff", {
      relPath: openPath,
      afterContent: after,
      originModule: "AG",
    });
    deStore.addDiff({
      diffId: diff_id,
      blockId: "", // 整文档变换，不绑定单块 blockId（块级 GreenReview 留待块级 diff）
      title: "对当前文档追加一段",
      before: snippet(loadedContent.slice(-48)),
      after: "…追加：AI 追加的一段说明文字。",
      state: "pending",
    });
  } catch (e) {
    console.error(`de.generate_diff failed ${errMsg(e)}`);
  }
}

/** 接受单条：validating→applying→de.accept_diff（真落盘）→accepted + 重载编辑器；失败→error。 */
export async function acceptDiff(diffId: string): Promise<void> {
  deStore.setDiffState(diffId, "validating");
  deStore.setDiffState(diffId, "applying");
  try {
    const entry = await callCommand<{ entry_id: string; version_seq: number }>("de.accept_diff", { diffId });
    deStore.setDiffState(diffId, "accepted", { timelineSeq: entry.version_seq });
    // 文件已被写入 → 重载编辑器显示落盘后内容（投影后端）。
    const { openPath } = edStore.getSnapshot();
    if (openPath) await openDocument(openPath);
  } catch (e) {
    const x = e as { code?: string };
    deStore.setDiffState(diffId, "error", { errorCode: x?.code ?? "DE_ACCEPT_FAILED" });
    console.error(`de.accept_diff failed ${errMsg(e)}`);
  }
}

/** 拒绝单条：de.reject_diff（不写文件）→rejected。 */
export async function rejectDiff(diffId: string): Promise<void> {
  try {
    await callCommand("de.reject_diff", { diffId });
  } catch (e) {
    console.error(`de.reject_diff failed ${errMsg(e)}`);
  }
  deStore.setDiffState(diffId, "rejected");
}

/**
 * 全部接受（倒序）：从文档靠后的 diff 先 de.accept_diff，避免先序落盘造成后序位置偏移（参考稿 §6.5）。
 * 过期/冲突/error 不计入可应用集，计入失效统计。
 */
export async function acceptAllDiffs(): Promise<void> {
  const { diffs } = deStore.getSnapshot();
  const appliable = diffs.filter((d) => d.state === "pending");
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
  for (const d of diffs.filter((x) => x.state === "pending")) {
    await rejectDiff(d.diffId);
  }
  deStore.setBatch("idle");
}
