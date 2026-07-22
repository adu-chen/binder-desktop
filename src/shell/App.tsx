import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useWorkspaceShell } from "../store/workspaceShell";
import { restoreActiveWorkspace } from "../services/workspaceService";
import { requestClose } from "../services/closeService";
import { closeStore } from "../store/closeStore";
import { CloseConfirmDialog } from "../components/shell/CloseConfirmDialog";
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
    let unlistenExitRequested: (() => void) | null = null;

    /**
     * @GOV
     * codes:    BR-SYS-STATE-004-GUARD-SYS-APP-CLOSE-003
     * type:     GUARD
     * chain:    APP-CLOSE
     * rules:    BR-SYS-STATE-004
     * boundary: in=windowCloseRequest | out=closeOrchestrationEntered(pendingDiffAndDirtyMustReachZeroBeforeRealClose) | does_not=bypass_close_confirm_dialog_when_residue_exists
     */
    // 统一关闭入口（APP-CLOSE-DIFF-LIFECYCLE-01）：窗口关闭按钮走 CloseRequested；Cmd+Q/Dock
    // 退出/系统关机由 Rust 的 RunEvent::ExitRequested 显式广播 sys:exit-requested。两者均只调用
    // requestClose()，不让 Rust 直接 close 窗口而绕开 WebView 监听器。
    // closeService.requestClose() 清点 pending/dirty；为零直接放行（内部会再次调用 appWindow.close()，
    // 此时 closeStore.phase 已是 "closing"，下方判断据此放行而不再拦截，避免死循环）；否则呈现
    // CloseConfirmDialog 由用户决策。
    void appWindow
      .onCloseRequested((event) => {
        if (closeStore.getSnapshot().phase === "closing") {
          return; // 关闭流程内部触发的再次关闭：放行，不拦截
        }
        event.preventDefault();
        void requestClose();
      })
      .then((fn) => {
        unlisten = fn;
      })
      .catch((e) => {
        console.error(`window close hook failed ${String(e)}`);
      });

    void listen("sys:exit-requested", () => {
      void requestClose();
    })
      .then((fn) => {
        unlistenExitRequested = fn;
      })
      .catch((e) => {
        console.error(`application exit hook failed ${String(e)}`);
      });

    return () => {
      unlisten?.();
      unlistenExitRequested?.();
    };
  }, []);

  return (
    <>
      {activeWorkspace ? <Workbench /> : <Welcome />}
      <CloseConfirmDialog />
    </>
  );
}
