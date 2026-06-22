import { useSyncExternalStore } from "react";

/**
 * 前端外壳状态投影层（X-FORBID-007）。
 * 前端 Store 不另立权威业务状态——activeWorkspace 是后端 workspace 活动态（workspace.is_active，
 * WS 模块控制态）的投影。此处为脚手架占位：实现期（PI-WS-01 / PI-SYS-05）由 ws.* command 回填，
 * 经 Service 层（X-SEQ-002）订阅后端状态，不在前端维护与后端可漂移的并行状态。
 */

export interface WorkspaceRefView {
  workspaceId: string;
  rootPath: string;
}

interface ShellState {
  activeWorkspace: WorkspaceRefView | null;
}

let state: ShellState = { activeWorkspace: null };
const listeners = new Set<() => void>();

function emit() {
  for (const l of listeners) l();
}

// 占位：实现期由后端事件（ws.* 投影）驱动，不在前端自行决定权威态。
export const workspaceShell = {
  subscribe(cb: () => void) {
    listeners.add(cb);
    return () => listeners.delete(cb);
  },
  getSnapshot(): ShellState {
    return state;
  },
  // 脚手架内部用：投影后端活动态变更（实现期替换为后端订阅）。
  _projectActive(ws: WorkspaceRefView | null) {
    state = { activeWorkspace: ws };
    emit();
  },
};

export function useWorkspaceShell(): ShellState {
  return useSyncExternalStore(workspaceShell.subscribe, workspaceShell.getSnapshot);
}
