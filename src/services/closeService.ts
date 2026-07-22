import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { closeStore, type CloseDecision } from "../store/closeStore";
import { edStore, isDirtyTabState } from "../store/edStore";
import { deStore } from "../store/deStore";
import { tabsStore } from "../store/tabsStore";
import { acceptAllDiffs, rejectAllDiffs } from "./deService";
import { triggerActiveSave, getActiveEditorBinding } from "./edLocate";
import { flushEditorStateBeforeExit } from "./workspaceService";

/**
 * 应用关闭编排（SM-SYS-02 · APP-CLOSE · APP-CLOSE-DIFF-LIFECYCLE-01）。
 * 窗口关闭按钮与 Cmd+Q/Dock 退出/系统关机（Rust 侧收敛为窗口 CloseRequested，见 src-tauri/src/lib.rs
 * RunEvent::ExitRequested 处理）均经同一入口 `requestClose()`，不设第二套关闭流程。
 * 关闭前真实计数以后端 `sys.inspect_close` 复核为准，本模块内部状态不作为最终放行依据
 * （BR-SYS-STATE-004 Verifying：不信任发起决策时刻的缓存计数）。
 */

interface IpcErrorLike {
  code?: string;
  message?: string;
}
interface CommandResponse<T> {
  correlation_id: string;
  result?: T;
  error?: IpcErrorLike;
}

let seq = 0;
async function callCommand<T>(command: string, params: Record<string, unknown> = {}): Promise<T> {
  const request = { command, correlation_id: `close-${Date.now()}-${(seq += 1)}`, params };
  const response = await invoke<CommandResponse<T>>("dispatch", { request });
  if (response.error) throw response.error;
  if (response.result === undefined) {
    throw { code: "IPC_EMPTY_RESULT", message: `${command} 无返回` } satisfies IpcErrorLike;
  }
  return response.result;
}

function ipcMessage(e: unknown): string {
  const err = e as IpcErrorLike;
  return err?.message ? `[${err.code ?? "?"}] ${err.message}` : String(e);
}

interface InspectCloseDto {
  pendingDiffCount: number;
  awaitingSaveCount: number;
  dirtyDocuments: { relPath: string; hasAcceptedAwaitingSave: boolean }[];
}

async function inspectClose(): Promise<InspectCloseDto> {
  return callCommand<InspectCloseDto>("sys.inspect_close");
}

/**
 * 破坏性操作前的聚合守卫（WorkspaceMenu 等复用）：待裁决 diff（workspace 级，deStore 已是全量投影）
 * 或任一已打开文档（含非激活）存在未落盘内容。修复原实现仅检查激活文档 tabState 的漏检
 * （src/components/shell/WorkspaceMenu.tsx 原 hasUnsaved()）。
 */
export function hasUnsavedOrPendingWork(): boolean {
  const pending = deStore.getSnapshot().diffs.some((d) => d.state === "pending");
  const dirty = Object.values(edStore.getSnapshot().sessions).some((s) => isDirtyTabState(s.tabState));
  return pending || dirty;
}

function waitForActiveBinding(relPath: string, timeoutMs: number): Promise<boolean> {
  return new Promise((resolve) => {
    const start = Date.now();
    const check = () => {
      const binding = getActiveEditorBinding();
      if (binding && binding.relPath === relPath) {
        resolve(true);
        return;
      }
      if (Date.now() - start > timeoutMs) {
        resolve(false);
        return;
      }
      globalThis.setTimeout(check, 50);
    };
    check();
  });
}

/** 切到目标文档标签并保存（复用 Editor.tsx 注册的保存处理器，不重复实现各格式保存分支）。 */
async function activateAndSave(relPath: string): Promise<{ ok: boolean; message?: string }> {
  const existing = tabsStore.getSnapshot().tabs.find((t) => t.kind === "doc" && t.path === relPath);
  if (!existing) {
    return { ok: false, message: `文档未在打开标签中: ${relPath}` };
  }
  tabsStore.activate(existing.id);
  const bound = await waitForActiveBinding(relPath, 2000);
  if (!bound) {
    return { ok: false, message: `编辑器未在超时内绑定到 ${relPath}` };
  }
  const result = await triggerActiveSave();
  return { ok: result.ok, message: result.message };
}

/**
 * 复核清零后（或强制退出）真正关闭窗口：flush 持久化 + 关窗，失败兜底 destroy。
 * 重入保护复用 closeStore.phase（而非另立模块级布尔位）：本函数第一步即置 phase="closing"，
 * 后续任何入口（含 App.tsx 收到窗口再次触发的 CloseRequested）据此判断已在关闭中，不重复执行。
 */
async function actuallyCloseWindow(): Promise<void> {
  if (closeStore.getSnapshot().phase === "closing") return;
  closeStore.setClosing();
  try {
    await flushEditorStateBeforeExit();
  } catch (e) {
    console.error(`flush before exit failed ${String(e)}`);
  }
  const appWindow = getCurrentWindow();
  try {
    await appWindow.close();
  } catch (closeError) {
    console.error(`window close failed ${String(closeError)}`);
    try {
      await appWindow.destroy();
    } catch (destroyError) {
      closeStore.reset();
      console.error(`window destroy failed ${String(destroyError)}`);
    }
  }
}

/**
 * 关闭请求入口（BR-SYS-STATE-004）：清点当前 workspace 真实 pending/awaiting_save/dirty 计数——
 * 为零直接放行关闭；
 * 否则呈现决策入口（AwaitingDecision），交由用户在 CloseConfirmDialog 中决策。
 * 屏障已由 agService.startTurn / deService.generateSampleDiff 在 closeStore.phase !== "idle" 时
 * 自行拒绝新回合/新 diff（检查该 phase 即生效，本函数不需要另行广播屏障事件）。
 */
export async function requestClose(): Promise<void> {
  if (closeStore.getSnapshot().phase !== "idle") return; // 已在处理中，不重复发起
  closeStore.setChecking();
  try {
    // 编辑器快照存在防抖持久化，必须先同步 flush，再以数据库真值检查；失败即拒绝放行。
    await flushEditorStateBeforeExit();
  } catch (e) {
    console.error(`flush before close inspection failed ${ipcMessage(e)}`);
    closeStore.setAwaitingDecision(0, [], 0);
    closeStore.setFailed("关闭前保存编辑状态失败，请重试或强制退出");
    return;
  }
  let report: InspectCloseDto;
  try {
    report = await inspectClose();
  } catch (e) {
    console.error(`sys.inspect_close failed ${ipcMessage(e)}`);
    closeStore.setAwaitingDecision(0, [], 0);
    closeStore.setFailed("关闭前检查失败，请重试或强制退出");
    return;
  }
  // 兼容升级期间旧后端响应未携带该字段；正式后端始终返回 0 或正整数。
  const awaitingSaveCount = report.awaitingSaveCount ?? 0;
  if (report.pendingDiffCount === 0 && awaitingSaveCount === 0 && report.dirtyDocuments.length === 0) {
    await actuallyCloseWindow();
    return;
  }
  closeStore.setAwaitingDecision(report.pendingDiffCount, report.dirtyDocuments, awaitingSaveCount);
}

/** 用户取消关闭：撤销屏障、恢复正常可用（不实际退出）。 */
export function cancelClose(): void {
  closeStore.reset();
}

/**
 * 用户在决策入口提交选择（BR-SYS-STATE-004）：批量裁决 + （保存|放弃）未保存内容，随后复核真值——
 * 未清零则回决策入口展示失败原因，不静默放行；清零才真正关闭窗口。
 */
export async function submitDecision(decision: CloseDecision): Promise<void> {
  const snap = closeStore.getSnapshot();
  if (snap.phase !== "awaitingDecision" || snap.busy) return;
  closeStore.setResolving();

  try {
    if (decision === "acceptAllAndSave") {
      await acceptAllDiffs();
    } else {
      await rejectAllDiffs();
    }

    if (decision === "acceptAllAndSave" || decision === "rejectAllAndSave") {
      const failures: string[] = [];
      for (const doc of snap.dirtyDocuments) {
        const result = await activateAndSave(doc.relPath);
        if (!result.ok) failures.push(`${doc.relPath}: ${result.message ?? "保存失败"}`);
      }
      if (failures.length > 0) {
        closeStore.setFailed(`保存失败：${failures.join("；")}`);
        return;
      }
    } else {
      // rejectAllAndDiscard：显式放弃未保存修改（terminal_reason=user_discarded_on_close）。
      await callCommand("ws.discard_unsaved_sessions");
      for (const doc of snap.dirtyDocuments) {
        edStore.setTabState(doc.relPath, "normal");
      }
    }
  } catch (e) {
    closeStore.setFailed(`处理失败：${ipcMessage(e)}`);
    return;
  }

  // 复核：重新查询真值，不信任发起决策时刻的计数（BR-SYS-STATE-004 Verifying）。
  let report: InspectCloseDto;
  try {
    report = await inspectClose();
  } catch (e) {
    closeStore.setFailed(`复核失败：${ipcMessage(e)}`);
    return;
  }
  const awaitingSaveCount = report.awaitingSaveCount ?? 0;
  if (report.pendingDiffCount > 0 || awaitingSaveCount > 0 || report.dirtyDocuments.length > 0) {
    closeStore.setAwaitingDecision(report.pendingDiffCount, report.dirtyDocuments, awaitingSaveCount);
    closeStore.setFailed("处理后仍有残留待处理项，请重试");
    return;
  }

  await actuallyCloseWindow();
}

/**
 * 强制退出（2026-07-20 人类决策 3，BR-SYS-STATE-006）：仅由用户在决策入口显式触发，不等待裁决/
 * 保存结果，允许残留未终态数据——下次启动由 BR-SYS-STATE-005 异常恢复护栏统一清算。
 */
/**
 * @GOV
 * codes:    BR-SYS-STATE-006-EFFECT-SYS-APP-CLOSE-001
 * type:     EFFECT
 * chain:    APP-CLOSE
 * rules:    BR-SYS-STATE-006
 * boundary: in=explicitForceQuitIntent | out=windowCloseStarted(residualHandledByNextStartupSweep) | does_not=autoForceQuit
 */
export async function forceQuit(): Promise<void> {
  await actuallyCloseWindow();
}
