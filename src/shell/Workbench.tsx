import { useEffect, useState, type CSSProperties } from "react";
import { useWorkspaceShell } from "../store/workspaceShell";
import { LeftRail, type RailPanelId } from "../components/shell/LeftRail";
import { Splitter } from "../components/shell/Splitter";
import { layoutStore, useLayoutStore } from "../store/layoutStore";
import { AppTitleBar } from "../components/shell/AppTitleBar";
import { FileTree } from "../components/ws/FileTree";
import { TimelinePanel } from "../components/ws/TimelinePanel";
import { PrvPanel } from "../components/prv/PrvPanel";
import { TabStrip } from "../components/shell/TabStrip";
import { useTabsStore } from "../store/tabsStore";
import { Editor } from "../components/ed/Editor";
import { OutlinePanel } from "../components/ed/OutlinePanel";
import { AgentPanel } from "../components/ag/AgentPanel";
import { ModelConfig } from "../components/ag/ModelConfig";
import { startSmListeners, stopSmListeners } from "../services/smListeners";

/**
 * 主工作台（A-SYS-C-T-02 §3/§4 界面分区与全局布局）。有活动 workspace 后的主界面。
 * 布局 CSS Grid（PI-FE-01）：titlebar / work(rail·side·center·chat) / statusbar；
 * 批量裁决条已移入代理编辑面板输入框上方（PI-AG-UI-01，原全局 diffstrip 移除）。
 * Left Rail（PI-FE-02）已接入；side/center/chat 内容由后续 PI 填充。
 * activePanel 是视图层导航态（本地持有，非 X-FORBID-007 限制的权威业务态）。
 */
const basename = (path: string) => path.split("/").filter(Boolean).pop() ?? path;

/**
 * @GOV
 * codes:    BR-WS-PERSIST-002-RB-WS-DATA-MIGRATE-002
 * type:     RB
 * chain:    DATA-MIGRATE
 * rules:    BR-WS-PERSIST-002
 * boundary: in=ActiveWorkspaceProjectionAndViewNavigationState | out=WorkspaceBoundWorkbenchPanels | delegate=startSmListeners,stopSmListeners | does_not=store_workspace_control_state_locally
 */
export function Workbench() {
  const { activeWorkspace } = useWorkspaceShell();
  const [activePanel, setActivePanel] = useState<RailPanelId>("files");
  const [settingsOpen, setSettingsOpen] = useState(false);
  // 三栏宽度（可拖动自适应，视图态）：side/chat 像素宽由 CSS 变量驱动，center 取剩余 1fr。
  const { sideW, chatW } = useLayoutStore();
  const { tabs, activeTabId } = useTabsStore();
  const activeTab = tabs.find((t) => t.id === activeTabId);
  const title = activeTab?.path
    ? basename(activeTab.path)
    : activeWorkspace?.rootPath
      ? basename(activeWorkspace.rootPath)
      : "Binder";
  // 面板重挂键：workspace 变更即重挂 FileTree/Editor/AgentPanel，载入新项目内容。
  const wsKey = activeWorkspace?.workspaceId ?? "none";
  // 启动状态机广播监听（A-CORE-X-L-04 §5.5 · X-FORBID-007）：后端 SM-* 转移经 IPC 投影到 store。
  useEffect(() => {
    void startSmListeners();
    return () => stopSmListeners();
  }, []);

  return (
    <div className="workbench">
      <AppTitleBar title={title} />

      <div
        className="work"
        style={{ "--side-w": `${sideW}px`, "--chat-w": `${chatW}px` } as CSSProperties}
      >
        <LeftRail active={activePanel} onChange={setActivePanel} onOpenSettings={() => setSettingsOpen(true)} />
        <aside className="side">
          {activePanel === "files" ? (
            // 按 workspaceId 重挂：切换项目即刷新文件树为新项目内容（PI-FE-10）。
            <FileTree key={wsKey} />
          ) : activePanel === "outline" ? (
            <OutlinePanel />
          ) : activePanel === "timeline" ? (
            <TimelinePanel key={wsKey} />
          ) : (
            // 其余面板（记忆/知识/模板）由后续 PI 填充
            <span className="label">{activePanel}</span>
          )}
        </aside>
        <Splitter
          className="gut-left"
          ariaLabel="调整侧栏宽度"
          onResize={(dx) => layoutStore.setSideW(layoutStore.getSnapshot().sideW + dx)}
          onReset={() => layoutStore.resetSide()}
        />
        <main className="center">
          <TabStrip />
          {activeTab?.kind === "preview" ? (
            <PrvPanel key={activeTab.path} path={activeTab.path} />
          ) : (
            <Editor key={wsKey} />
          )}
        </main>
        <Splitter
          className="gut-right"
          ariaLabel="调整代理编辑面板宽度"
          onResize={(dx) => layoutStore.setChatW(layoutStore.getSnapshot().chatW - dx)}
          onReset={() => layoutStore.resetChat()}
        />
        <aside className="chat">
          <AgentPanel key={wsKey} />
        </aside>
      </div>

      <footer className="statusbar">
        <span>就绪</span>
        <span>{activeWorkspace?.rootPath ?? "无 workspace"}</span>
      </footer>

      {settingsOpen && <ModelConfig onClose={() => setSettingsOpen(false)} />}
    </div>
  );
}
