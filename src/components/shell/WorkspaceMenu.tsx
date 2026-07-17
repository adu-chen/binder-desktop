import { useState } from "react";
import type { ReactNode } from "react";
import { useWorkspaceShell } from "../../store/workspaceShell";
import {
  openWorkspace,
  switchWorkspace,
  closeWorkspace,
  loadRecentWorkspaces,
} from "../../services/workspaceService";
import { edStore } from "../../store/edStore";
import { deStore } from "../../store/deStore";

/**
 * workspace 菜单（A-SYS-C-T-02 §3 WS 切换 / §4.1 WB→关闭/切换→C · PI-FE-10）。
 * 切换最近 / 打开其他文件夹 / 返回欢迎页；破坏性操作前脏态守卫（编辑器 dirty 或有 pending diff）。
 * 活动态以后端为准（X-FORBID-007）；服务层负责重置下游 store + 投影。
 */

/** 是否有未保存改动或待裁决修改（守卫依据，参考稿链路 G）。 */
function hasUnsaved(): boolean {
  const dirty = edStore.getSnapshot().tabState === "dirty";
  const pending = deStore.getSnapshot().diffs.some((d) => d.state === "pending");
  return dirty || pending;
}

interface WorkspaceMenuProps {
  trigger?: ReactNode;
  triggerClassName?: string;
  triggerTitle?: string;
  placement?: "titlebar" | "rail";
}

export function WorkspaceMenu({
  trigger,
  triggerClassName = "",
  triggerTitle = "workspace 菜单",
  placement = "titlebar",
}: WorkspaceMenuProps) {
  const { activeWorkspace, recent, recentStatus } = useWorkspaceShell();
  const [open, setOpen] = useState(false);
  const [confirm, setConfirm] = useState<null | (() => void)>(null);

  const toggle = () => {
    const next = !open;
    setOpen(next);
    setConfirm(null);
    if (next) void loadRecentWorkspaces();
  };

  // 破坏性操作经守卫：有未保存改动则先确认，否则直接执行。
  const guarded = (action: () => void) => {
    if (hasUnsaved()) {
      setConfirm(() => action);
    } else {
      action();
      setOpen(false);
    }
  };

  const doSwitch = (id: string) => guarded(() => void switchWorkspace(id));
  const doOpen = () => guarded(() => void openWorkspace());
  const doClose = () => guarded(() => void closeWorkspace());

  const others = recent.filter((w) => w.workspaceId !== activeWorkspace?.workspaceId);
  const triggerContent = trigger ?? (
    <>
      <span className="wsmenu-path">{activeWorkspace?.rootPath ?? "—"}</span>
      <span className="wsmenu-caret">▾</span>
    </>
  );

  return (
    <div className={`wsmenu wsmenu-${placement}`}>
      <button
        type="button"
        className={`wsmenu-trigger ${triggerClassName}`.trim()}
        onClick={toggle}
        title={triggerTitle}
        aria-label={triggerTitle}
        aria-expanded={open}
      >
        {triggerContent}
      </button>

      {open && (
        <div className="wsmenu-pop" role="menu">
          {confirm ? (
            <div className="wsmenu-confirm">
              <div>有未保存改动或待裁决修改，仍要继续？</div>
              <div className="wsmenu-confirm-actions">
                <button
                  onClick={() => {
                    const fn = confirm;
                    setConfirm(null);
                    setOpen(false);
                    fn();
                  }}
                >
                  继续（放弃改动）
                </button>
                <button className="ghost" onClick={() => setConfirm(null)}>
                  取消
                </button>
              </div>
            </div>
          ) : (
            <>
              <div className="wsmenu-section label">切换最近</div>
              {recentStatus === "loading" && <div className="wsmenu-item muted">加载中…</div>}
              {recentStatus === "ready" && others.length === 0 && (
                <div className="wsmenu-item muted">无其他历史 workspace</div>
              )}
              {others.map((w) => (
                <button key={w.workspaceId} className="wsmenu-item" onClick={() => doSwitch(w.workspaceId)}>
                  {w.rootPath}
                </button>
              ))}
              <div className="wsmenu-divider" />
              <button className="wsmenu-item" onClick={doOpen}>
                打开其他文件夹…
              </button>
              <button className="wsmenu-item wsmenu-close" onClick={doClose}>
                返回欢迎页（关闭 workspace）
              </button>
            </>
          )}
        </div>
      )}
    </div>
  );
}
