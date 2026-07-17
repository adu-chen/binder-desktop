import { useSyncExternalStore } from "react";

/**
 * WS 文件树状态投影层（X-FORBID-007）。
 * docs 是后端 ws.list_docs 的投影（DocRef 列表，BR-WS-DATA-001）；前端只建树呈现，不另立权威态。
 * status 是视图层请求态（loading/ready/error），非业务态。
 */

/** 文件树列举条目（FsEntry 投影）：文件或目录（is_dir 使空目录可见）。 */
export interface DocRefView {
  relPath: string;
  docType: string;
  isDir: boolean;
  /** 后端登记的 BDocx 身份；true 时 doc/docx 走 DOC 编辑链，false 原生 docx 只预览。 */
  isBDocx?: boolean;
}

export type DocsStatus = "idle" | "loading" | "ready" | "error";

interface WsState {
  docs: DocRefView[];
  status: DocsStatus;
  error: string | null;
}

let state: WsState = { docs: [], status: "idle", error: null };
const listeners = new Set<() => void>();

function emit() {
  for (const l of listeners) l();
}

export const wsStore = {
  subscribe(cb: () => void) {
    listeners.add(cb);
    return () => listeners.delete(cb);
  },
  getSnapshot(): WsState {
    return state;
  },
  _projectLoading() {
    state = { ...state, status: "loading", error: null };
    emit();
  },
  _projectDocs(docs: DocRefView[]) {
    state = { docs, status: "ready", error: null };
    emit();
  },
  _projectError(message: string) {
    state = { ...state, status: "error", error: message };
    emit();
  },
  /** 切换/关闭 workspace 时清空文件树（避免残留旧项目内容）。 */
  reset() {
    state = { docs: [], status: "idle", error: null };
    emit();
  },
};

export function useWsStore(): WsState {
  return useSyncExternalStore(wsStore.subscribe, wsStore.getSnapshot);
}
