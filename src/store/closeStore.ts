import { useSyncExternalStore } from "react";

/**
 * 应用关闭协调机前端投影（SM-SYS-02 · APP-CLOSE · X-FORBID-007）。
 * 权威状态机在前端编排（closeService 驱动），本 store 只是供 CloseConfirmDialog 渲染的视图投影；
 * 关闭前的真实计数以后端 `sys.inspect_close` 复核为准，不信任本 store 缓存值做最终放行判断
 * （BR-SYS-STATE-004：Verifying 态必须重新查询）。
 */

export type ClosePhase =
  | "idle"
  | "checking"
  | "awaitingDecision"
  | "resolving"
  | "closing";

export type CloseDecision =
  | "acceptAllAndSave"
  | "rejectAllAndDiscard"
  | "rejectAllAndSave";

export interface DirtyDocumentInfo {
  relPath: string;
  hasAcceptedAwaitingSave: boolean;
}

export interface CloseState {
  phase: ClosePhase;
  pendingDiffCount: number;
  /** 已接受但尚未确认写盘的 effect；同样阻断普通关闭。 */
  awaitingSaveCount: number;
  dirtyDocuments: DirtyDocumentInfo[];
  /** 处理中标记（Resolving/强制退出执行期间禁用重复提交）。 */
  busy: boolean;
  /** 上一次执行失败的原因（resolveFail/verifyRetry），展示于弹窗并保留决策入口。 */
  error: string | null;
}

const initial: CloseState = {
  phase: "idle",
  pendingDiffCount: 0,
  awaitingSaveCount: 0,
  dirtyDocuments: [],
  busy: false,
  error: null,
};

let state: CloseState = initial;
const listeners = new Set<() => void>();
function emit() {
  for (const l of listeners) l();
}
function setState(patch: Partial<CloseState>) {
  state = { ...state, ...patch };
  emit();
}

export const closeStore = {
  subscribe(cb: () => void) {
    listeners.add(cb);
    return () => listeners.delete(cb);
  },
  getSnapshot(): CloseState {
    return state;
  },

  reset() {
    setState(initial);
  },
  setChecking() {
    setState({ phase: "checking", busy: true, error: null });
  },
  /** Inspecting/Verifying 复核结果为清零 → 由调用方直接进入 closing，不停留本态。 */
  setAwaitingDecision(pendingDiffCount: number, dirtyDocuments: DirtyDocumentInfo[], awaitingSaveCount = 0) {
    setState({
      phase: "awaitingDecision",
      pendingDiffCount,
      awaitingSaveCount,
      dirtyDocuments,
      busy: false,
      error: null,
    });
  },
  setResolving() {
    setState({ phase: "resolving", busy: true, error: null });
  },
  /** resolveFail/verifyRetry：回决策入口并展示失败原因，不静默放行、不静默重试。 */
  setFailed(error: string) {
    setState({ phase: "awaitingDecision", busy: false, error });
  },
  setClosing() {
    setState({ phase: "closing", busy: true, error: null });
  },
};

export function useCloseStore(): CloseState {
  return useSyncExternalStore(closeStore.subscribe, closeStore.getSnapshot);
}
