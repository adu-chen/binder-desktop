import { useState } from "react";
import { useWorkspaceShell } from "../store/workspaceShell";
import { LeftRail, type RailPanelId } from "../components/shell/LeftRail";
import { BinderWordmark } from "../components/brand/BinderWordmark";

/**
 * 主工作台（A-SYS-C-T-02 §3/§4 界面分区与全局布局）。有活动 workspace 后的主界面。
 * 四栏 CSS Grid（PI-FE-01）：titlebar / work(rail·side·center·chat) / diffstrip / statusbar。
 * Left Rail（PI-FE-02）已接入；side/center/chat 内容由后续 PI 填充。
 * activePanel 是视图层导航态（本地持有，非 X-FORBID-007 限制的权威业务态）。
 */
export function Workbench() {
  const { activeWorkspace } = useWorkspaceShell();
  const [activePanel, setActivePanel] = useState<RailPanelId>("files");
  const pendingDiffCount = 0; // 占位：PI-FE-07 由 deStore 投影后端 pending_diffs 计数

  return (
    <div className="workbench">
      <header className="titlebar">
        <BinderWordmark size={16} />
        <span className="label">{activeWorkspace?.rootPath ?? "—"}</span>
      </header>

      <div className="work">
        <LeftRail active={activePanel} onChange={setActivePanel} />
        <aside className="side">
          {/* PI-FE-04 WS 文件树（及其余面板按 activePanel 切换） */}
          <span className="label">侧边栏 · {activePanel}</span>
        </aside>
        <main className="center">
          {/* PI-FE-05 ED 编辑器 */}
          <span className="label">编辑区 · ED</span>
        </main>
        <aside className="chat">
          {/* PI-FE-06 AG 对话区 */}
          <span className="label">对话区 · AG</span>
        </aside>
      </div>

      <div className="diffstrip" hidden={pendingDiffCount === 0}>
        {/* PI-FE-07 DE 批量裁决条 */}
        <span className="label">{pendingDiffCount} 待裁决</span>
      </div>

      <footer className="statusbar">
        <span>就绪</span>
        <span>{activeWorkspace?.rootPath ?? "无 workspace"}</span>
        <span>账户/订阅 · ACCT/ENT</span>
      </footer>
    </div>
  );
}
