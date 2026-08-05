import { useEffect } from "react";
import { useWorkspaceShell } from "../store/workspaceShell";
import {
  openWorkspace,
  loadRecentWorkspaces,
  switchWorkspace,
} from "../services/workspaceService";
import { AppTitleBar } from "../components/shell/AppTitleBar";
import { BinderWordmark } from "../components/brand/BinderWordmark";

/**
 * 欢迎页（A-SYS-C-T-02 §4.1）。无活动 workspace 时的入口：
 * 打开/新建 workspace · 最近列表（ws.list_workspaces）· 登录入口（可选，AI 功能才需）。
 * 最近列表三态：loading / ready（含空态）/ error（统一错误形状 BR-SYS-OBS-002）。
 * 本地编辑离线可用（REQ-CORE-009），登录非进入工作台的前置。
 */
/**
 * @GOV
 * codes:    BR-WS-PERSIST-002-RB-WS-DATA-MIGRATE-001
 * type:     RB
 * chain:    DATA-MIGRATE
 * rules:    BR-WS-PERSIST-002
 * boundary: in=RegisteredWorkspaceListAndOpenSwitchIntent | out=WorkspaceEntryActions | delegate=openWorkspace,switchWorkspace,loadRecentWorkspaces | does_not=mutate_workspace_control_state_directly
 */
export function Welcome() {
  const { recent, recentStatus, recentError, notice } = useWorkspaceShell();

  useEffect(() => {
    void loadRecentWorkspaces();
  }, []);

  return (
    <div className="welcome-shell">
      <AppTitleBar title="Binder" />
      <main className="welcome">
        <BinderWordmark size={44} />
        <p className="muted">本地优先的 AI 原生写作工作台</p>

        {notice && <p className="welcome-notice">{notice}</p>}

        <div className="actions">
          <button onClick={() => void openWorkspace()}>打开 / 新建 workspace</button>
          <button className="ghost" disabled>
            登录（AI 功能）
          </button>
        </div>

        <section className="recent">
          <div className="label">最近的 workspace</div>
          {recentStatus === "loading" && <p className="muted recent-hint">加载中…</p>}
          {recentStatus === "error" && (
            <p className="recent-error">
              加载失败：{recentError}
              <button className="ghost recent-retry" onClick={() => void loadRecentWorkspaces()}>
                重试
              </button>
            </p>
          )}
          {recentStatus === "ready" && recent.length === 0 && (
            <p className="muted recent-hint">还没有打开过 workspace</p>
          )}
          {recentStatus === "ready" && recent.length > 0 && (
            <ul className="recent-list">
              {recent.map((ws) => (
                <li key={ws.workspaceId}>
                  <button className="recent-item" onClick={() => void switchWorkspace(ws.workspaceId)}>
                    {ws.rootPath}
                  </button>
                </li>
              ))}
            </ul>
          )}
        </section>

        <p className="hint">脚手架骨架 · 欢迎页（PI-FE-03）</p>
      </main>
    </div>
  );
}
