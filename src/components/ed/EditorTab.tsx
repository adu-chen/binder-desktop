import type { TabState } from "../../store/edStore";

/**
 * EditorTab（参考稿 §6.3）· PI-FE-05。
 * 6 状态视觉互斥：normal / dirty（红点）/ saving（旋转器）/ saved（回退默认，不常驻对勾）/
 * pending_diff（红数字 badge）/ external_changed（金 !）。
 */

interface EditorTabProps {
  name: string;
  state: TabState;
  pendingDiffCount?: number;
  active?: boolean;
}

export function EditorTab({ name, state, pendingDiffCount = 0, active = true }: EditorTabProps) {
  return (
    <div className={`edtab${active ? " active" : ""} edtab-${state}`}>
      <span className="edtab-name">{name}</span>
      {state === "dirty" && <span className="edtab-dot" aria-label="未保存" />}
      {state === "saving" && <span className="edtab-spinner" aria-label="保存中" />}
      {state === "external_changed" && <span className="edtab-ext" title="文件被外部修改">!</span>}
      {state === "pending_diff" && pendingDiffCount > 0 && (
        <span className="edtab-badge">{pendingDiffCount}</span>
      )}
    </div>
  );
}
