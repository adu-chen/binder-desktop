import { openWorkspace } from "../services/workspaceService";

/**
 * 欢迎页（A-SYS-C-T-02 §3/§4.1）。无活动 workspace 时的入口：
 * 打开/新建 workspace · 最近列表 · 登录入口（可选，AI 功能才需，REQ-CORE-009 离线本地可用）。
 * 脚手架占位 UI——实现期贴 PI-SYS-05 / 路线图 §9 原料细化。
 */
export function Welcome() {
  return (
    <main className="welcome">
      <h1>Binder</h1>
      <p className="muted">本地优先的 AI 原生写作工作台</p>
      <div className="actions">
        <button onClick={() => void openWorkspace()}>打开 / 新建 workspace</button>
        <button className="ghost" disabled>
          最近的 workspace
        </button>
        <button className="ghost" disabled>
          登录（AI 功能）
        </button>
      </div>
      <p className="hint">脚手架骨架 · 欢迎页（PI-SYS-05）</p>
    </main>
  );
}
