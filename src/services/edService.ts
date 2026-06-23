import { readDoc, saveDoc, checkExternalChange } from "./wsService";
import { edStore } from "../store/edStore";

/**
 * ED 编排 Service（X-SEQ-002）：打开/保存真实文档，连接 PI-BE-01 落盘 command 与编辑器投影。
 * 落盘往返以后端基线为准（BR-WS-OBS-001）；前端不另立可漂移落盘态（X-FORBID-007）。
 */

interface IpcLikeError {
  code?: string;
  message?: string;
}

/** 打开文档：ws.read_doc → 载入编辑器（内容 + 加载基线）。 */
export async function openDocument(relPath: string): Promise<void> {
  try {
    const { content, baseline } = await readDoc(relPath);
    edStore.loadDoc(relPath, content, baseline);
  } catch (e) {
    const err = e as IpcLikeError;
    console.error(`ws.read_doc failed [${err.code ?? "?"}]: ${err.message ?? e}`);
  }
}

/** 保存结果：成功 ok；冲突需用户裁决（覆盖/重载）。 */
export type SaveOutcome = { ok: true } | { ok: false; conflict: boolean; message: string };

/**
 * 保存当前文档（⌘S）：ws.save_doc 带加载基线。磁盘≠基线且未 override → WS_SAVE_CONFLICT（SM-WS-01）。
 * override=true 表示用户显式确认覆盖外部修改。
 */
export async function saveDocument(content: string, override = false): Promise<SaveOutcome> {
  const { openPath, baseline } = edStore.getSnapshot();
  if (!openPath) return { ok: false, conflict: false, message: "无打开的文档" };
  edStore.setTabState("saving");
  try {
    const newBaseline = await saveDoc(openPath, content, baseline, override);
    edStore.markSaved(newBaseline);
    return { ok: true };
  } catch (e) {
    const err = e as IpcLikeError;
    const conflict = err.code === "WS_SAVE_CONFLICT";
    // 冲突回脏态等待裁决；其余失败也回脏态（不谎称已存）。
    edStore.setTabState(conflict ? "external_changed" : "dirty");
    return { ok: false, conflict, message: err.message ?? String(e) };
  }
}

/** 冲突后用户选择"重载"：丢弃本地编辑，重新从磁盘载入。 */
export async function reloadDocument(): Promise<void> {
  const { openPath } = edStore.getSnapshot();
  if (openPath) await openDocument(openPath);
}

/**
 * 外部变更轮询（实现 external_change 逻辑契约，物理事件延 SY-02）：
 * 比对磁盘哈希与加载基线，磁盘已被外部改动/删除 → 置 external_changed（触发冲突条裁决）。
 * 已是 external_changed 则不重复置；无打开文档/无基线则跳过。不静默覆盖外部修改（SM-WS-01）。
 */
export async function pollExternalChange(): Promise<void> {
  const { openPath, baseline, tabState } = edStore.getSnapshot();
  if (!openPath || baseline === null || tabState === "external_changed") return;
  try {
    const r = await checkExternalChange(openPath, baseline);
    if (r.changed) edStore.setTabState("external_changed");
  } catch {
    /* 检测失败不打断编辑（下次轮询重试） */
  }
}
