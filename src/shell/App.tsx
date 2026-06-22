import { useEffect } from "react";
import { useWorkspaceShell } from "../store/workspaceShell";
import { restoreActiveWorkspace } from "../services/workspaceService";
import { Welcome } from "./Welcome";
import { Workbench } from "./Workbench";

/**
 * 应用外壳（承接 A-SYS-C-T-02 §3/§4.1 / REQ-SYS-006 / PI-SYS-05）。
 * 欢迎页 ↔ 主工作台两阶段，依「活动 workspace 是否存在」切换。
 * 启动时投影后端活动态（ws.active_workspace 恢复）；X-FORBID-007：切换是活动态投影，非前端独立权威态。
 */
export function App() {
  const { activeWorkspace } = useWorkspaceShell();

  useEffect(() => {
    void restoreActiveWorkspace();
  }, []);

  return activeWorkspace ? <Workbench /> : <Welcome />;
}
