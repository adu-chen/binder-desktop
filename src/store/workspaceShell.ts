import { useSyncExternalStore } from "react";

/**
 * 前端外壳状态投影层（X-FORBID-007）。
 * 前端 Store 不另立权威业务状态——activeWorkspace 与 recent 是后端 workspace 控制态
 * （workspace.is_active / workspaces 表，WS 模块控制态）的投影，经 Service 层（X-SEQ-002）回填。
 * recentStatus 是 UI 请求态（视图层），非业务态——仅描述最近列表的拉取过程。
 */

export interface WorkspaceRefView {
  workspaceId: string;
  rootPath: string;
}

export type RecentStatus = "idle" | "loading" | "ready" | "error";

interface ShellState {
  activeWorkspace: WorkspaceRefView | null;
  recent: WorkspaceRefView[];
  recentStatus: RecentStatus;
  recentError: string | null;
  /** 外壳级提示（如启动恢复发现历史项目已不可用）；欢迎页展示。 */
  notice: string | null;
}

let state: ShellState = {
  activeWorkspace: null,
  recent: [],
  recentStatus: "idle",
  recentError: null,
  notice: null,
};
const listeners = new Set<() => void>();

function emit() {
  for (const l of listeners) l();
}

function setState(patch: Partial<ShellState>) {
  state = { ...state, ...patch };
  emit();
}

// 投影后端状态：所有写入均代表后端控制态/请求态变更，前端不自行决定权威业务态。
export const workspaceShell = {
  subscribe(cb: () => void) {
    listeners.add(cb);
    return () => listeners.delete(cb);
  },
  getSnapshot(): ShellState {
    return state;
  },
  // 投影后端活动态变更。
  _projectActive(ws: WorkspaceRefView | null) {
    setState({ activeWorkspace: ws });
  },
  // 投影最近列表拉取过程（loading → ready/error）。
  _projectRecentLoading() {
    setState({ recentStatus: "loading", recentError: null });
  },
  _projectRecent(list: WorkspaceRefView[]) {
    setState({ recent: list, recentStatus: "ready", recentError: null });
  },
  _projectRecentError(message: string) {
    setState({ recentStatus: "error", recentError: message });
  },
  setNotice(notice: string | null) {
    setState({ notice });
  },
};

export function useWorkspaceShell(): ShellState {
  return useSyncExternalStore(workspaceShell.subscribe, workspaceShell.getSnapshot);
}
