import { useEffect, useState, type CSSProperties } from "react";
import { useWorkspaceShell } from "../store/workspaceShell";
import { LeftRail, type RailPanelId } from "../components/shell/LeftRail";
import { Splitter } from "../components/shell/Splitter";
import { layoutStore, useLayoutStore } from "../store/layoutStore";
import { BinderWordmark } from "../components/brand/BinderWordmark";
import { WorkspaceMenu } from "../components/shell/WorkspaceMenu";
import { FileTree } from "../components/ws/FileTree";
import { TimelinePanel } from "../components/ws/TimelinePanel";
import { PrvPanel } from "../components/prv/PrvPanel";
import { TabStrip } from "../components/shell/TabStrip";
import { tabsStore, useTabsStore } from "../store/tabsStore";
import { Editor } from "../components/ed/Editor";
import { OutlinePanel } from "../components/ed/OutlinePanel";
import { ChatPanel } from "../components/ag/ChatPanel";
import { DiffAllActionsBar } from "../components/de/DiffAllActionsBar";
import { useDeStore } from "../store/deStore";
import { startSmListeners, stopSmListeners } from "../services/smListeners";

/**
 * 主工作台（A-SYS-C-T-02 §3/§4 界面分区与全局布局）。有活动 workspace 后的主界面。
 * 四栏 CSS Grid（PI-FE-01）：titlebar / work(rail·side·center·chat) / diffstrip / statusbar。
 * Left Rail（PI-FE-02）已接入；side/center/chat 内容由后续 PI 填充。
 * activePanel 是视图层导航态（本地持有，非 X-FORBID-007 限制的权威业务态）。
 */
export function Workbench() {
  const { activeWorkspace } = useWorkspaceShell();
  const [activePanel, setActivePanel] = useState<RailPanelId>("files");
  // 三栏宽度（可拖动自适应，视图态）：side/chat 像素宽由 CSS 变量驱动，center 取剩余 1fr。
  const { sideW, chatW } = useLayoutStore();
  const { diffs, batchState } = useDeStore();
  const { tabs, activeTabId } = useTabsStore();
  const activeTab = tabs.find((t) => t.id === activeTabId);
  // DiffStrip 显隐投影待裁决修改（PendingDiff）计数 + 批量应用过程。
  const pendingDiffCount = diffs.filter((d) => d.state === "pending").length;
  const showDiffStrip = pendingDiffCount > 0 || batchState !== "idle";
  // 面板重挂键：workspace 变更即重挂 FileTree/Editor/ChatPanel，载入新项目内容。
  const wsKey = activeWorkspace?.workspaceId ?? "none";
  // 切换/打开 workspace 时清空标签（避免残留旧项目标签；tabsStore 是单例，需显式重置）。
  useEffect(() => {
    tabsStore.reset();
  }, [wsKey]);

  // 启动状态机广播监听（A-CORE-X-L-04 §5.5 · X-FORBID-007）：后端 SM-* 转移经 IPC 投影到 store。
  useEffect(() => {
    void startSmListeners();
    return () => stopSmListeners();
  }, []);

  return (
    <div className="workbench">
      <header className="titlebar">
        <BinderWordmark size={16} />
        <WorkspaceMenu />
      </header>

      <div
        className="work"
        style={{ "--side-w": `${sideW}px`, "--chat-w": `${chatW}px` } as CSSProperties}
      >
        <LeftRail active={activePanel} onChange={setActivePanel} />
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
          ariaLabel="调整对话栏宽度"
          onResize={(dx) => layoutStore.setChatW(layoutStore.getSnapshot().chatW - dx)}
          onReset={() => layoutStore.resetChat()}
        />
        <aside className="chat">
          <ChatPanel key={wsKey} />
        </aside>
      </div>

      <div className="diffstrip" hidden={!showDiffStrip}>
        <DiffAllActionsBar />
      </div>

      <footer className="statusbar">
        <span>就绪</span>
        <span>{activeWorkspace?.rootPath ?? "无 workspace"}</span>
        <span>账户/订阅 · ACCT/ENT</span>
      </footer>
    </div>
  );
}
