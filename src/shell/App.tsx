import { useWorkspaceShell } from "../store/workspaceShell";
import { Welcome } from "./Welcome";
import { Workbench } from "./Workbench";

/**
 * 应用外壳（承接 A-SYS-C-T-02 §3 / REQ-SYS-006 / PI-SYS-05）。
 * 欢迎页 ↔ 主工作台两阶段，依「活动 workspace 是否存在」切换。
 * X-FORBID-007：该切换是 workspace 活动态的投影，非前端独立权威状态。
 */
export function App() {
  const { activeWorkspace } = useWorkspaceShell();
  return activeWorkspace ? <Workbench /> : <Welcome />;
}
