import { useWorkspaceShell } from "../store/workspaceShell";

/**
 * 主工作台（A-SYS-C-T-02 §3/§4 界面分区与全局布局）。有活动 workspace 后的主界面。
 * 四栏 CSS Grid 骨架（PI-FE-01）：titlebar / work(rail·side·center·chat) / diffstrip / statusbar。
 * 各分区→模块映射（脚手架占位，后续 PI 填充实现）：
 *   rail(导航) · side(WS 文件树 PI-FE-04) · center(ED 编辑器 PI-FE-05) ·
 *   chat(AG 对话 PI-FE-06) · diffstrip(DE 批量裁决 PI-FE-07) · statusbar(ACCT/ENT)
 * DiffStrip 依待裁决修改（PendingDiff）计数显隐——计数投影后端，当前恒为 0（隐藏）。
 */
export function Workbench() {
  const { activeWorkspace } = useWorkspaceShell();
  const pendingDiffCount = 0; // 占位：PI-FE-07 由 deStore 投影后端 pending_diffs 计数

  return (
    <div className="workbench">
      <header className="titlebar">
        <span style={{ fontWeight: 600, letterSpacing: "-0.04em" }}>binder</span>
        <span className="label">{activeWorkspace?.rootPath ?? "—"}</span>
      </header>

      <div className="work">
        <nav className="rail" aria-label="模块导航">
          {/* PI-FE-02 填充 Logo + 5 模块切换（工作区/记忆/知识/模板/时间轴） */}
          <span className="label">I·A</span>
        </nav>
        <aside className="side">
          {/* PI-FE-04 WS 文件树 */}
          <span className="label">侧边栏 · WS</span>
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
