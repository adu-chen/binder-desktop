import { BinderMark } from "../brand/BinderMark";
import { WorkspaceMenu } from "./WorkspaceMenu";

/**
 * Left Rail（44px）· A-SYS-C-T-02 §3 界面分区（PI-FE-02）。
 * 6 大模块导航 + 顶部 Logo workspace 菜单 + 底部设置。
 *
 * 激活面板（activePanel）是**视图层导航态**，由上层 Workbench 本地持有——
 * 不属 X-FORBID-007 限制的"权威业务状态"（导航选择不是后端可漂移的业务态），故不投影后端。
 * 各面板的业务内容（文件树/记忆/知识等）才经各自 Store 投影后端（后续 PI）。
 */

export type RailPanelId = "files" | "outline" | "memory" | "knowledge" | "templates" | "timeline";

interface RailItem {
  id: RailPanelId;
  glyph: string;
  num: string;
  label: string;
}

const RAIL_ITEMS: RailItem[] = [
  { id: "files", glyph: "WS", num: "I·A", label: "工作区" },
  { id: "outline", glyph: "OL", num: "I·O", label: "大纲" },
  { id: "memory", glyph: "ME", num: "I·B", label: "记忆库" },
  { id: "knowledge", glyph: "KN", num: "I·C", label: "知识库" },
  { id: "templates", glyph: "TM", num: "I·D", label: "模板库" },
  { id: "timeline", glyph: "TL", num: "I·E", label: "时间轴" },
];

interface LeftRailProps {
  active: RailPanelId;
  onChange: (id: RailPanelId) => void;
  /** 底部设置按钮：打开全局设置（当前承载模型配置，MODEL-CONFIG-01）。 */
  onOpenSettings?: () => void;
}

export function LeftRail({ active, onChange, onOpenSettings }: LeftRailProps) {
  return (
    <nav className="rail" aria-label="模块导航">
      <div className="rail-logo">
        <WorkspaceMenu
          placement="rail"
          trigger={<BinderMark size={28} variant="accented" />}
          triggerClassName="rail-logo-trigger"
          triggerTitle="workspace 菜单"
        />
      </div>
      {RAIL_ITEMS.map((it) => (
        <button
          key={it.id}
          className={`rail-btn${active === it.id ? " active" : ""}`}
          onClick={() => onChange(it.id)}
          title={it.label}
          aria-pressed={active === it.id}
        >
          <span className="rail-btn-glyph">{it.glyph}</span>
          <span className="rail-btn-num">{it.num}</span>
        </button>
      ))}
      <div className="rail-spacer" />
      <button className="rail-btn" title="设置" onClick={onOpenSettings}>
        <span className="rail-btn-glyph">·</span>
        <span className="rail-btn-num">SET</span>
      </button>
    </nav>
  );
}
