import { useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useWorkspaceShell } from "../store/workspaceShell";
import { flushEditorStateBeforeExit, restoreActiveWorkspace } from "../services/workspaceService";
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

  useEffect(() => {
    const appWindow = getCurrentWindow();
    let unlisten: (() => void) | null = null;
    let closing = false;

    /**
     * @GOV
     * codes:    BR-ED-STATE-001-GUARD-ED-ED-EDIT-013
     * type:     GUARD
     * chain:    ED-EDIT
     * rules:    BR-ED-STATE-001
     * boundary: in=windowCloseRequest | out=editorOpenTabsAndSessionSnapshotsFlushedBeforeWindowClose | does_not=leave_close_request_trapped_or_reenter_close_loop
     */
    void appWindow.onCloseRequested(async (event) => {
      if (closing) {
        event.preventDefault();
        return;
      }
      event.preventDefault();
      closing = true;
      try {
        await flushEditorStateBeforeExit();
      } finally {
        const stopListening = unlisten;
        unlisten = null;
        stopListening?.();
        try {
          await appWindow.close();
        } catch (closeError) {
          console.error(`window close after persistence failed ${String(closeError)}`);
          try {
            await appWindow.destroy();
          } catch (destroyError) {
            closing = false;
            console.error(`window destroy after close failure failed ${String(destroyError)}`);
          }
        }
      }
    }).then((fn) => {
      unlisten = fn;
    }).catch((e) => {
      console.error(`window close persistence hook failed ${String(e)}`);
    });

    return () => {
      unlisten?.();
    };
  }, []);

  return activeWorkspace ? <Workbench /> : <Welcome />;
}
