import { useSyncExternalStore } from "react";

/**
 * ED 编辑会话状态投影层（ED-EDIT · SM-ED-01 · X-FORBID-007）。
 * tabState 是编辑会话/脏态的视图投影：编辑产生脏态（前端编辑器感知），落盘/外部变更/待裁决由
 * 保存链（WS）、外部变更事件、DE 投影驱动。前端不另立与后端可漂移的权威落盘态。
 */

/** EditorTab 6 状态（参考稿 §6.3，视觉互斥）。 */
export type TabState =
  | "normal"
  | "dirty"
  | "saving"
  | "saved"
  | "pending_diff"
  | "external_changed";

interface EdState {
  openPath: string | null;
  tabState: TabState;
  pendingDiffCount: number;
  /** 已载入文档内容（纯文本）；编辑器据此 setContent。null = 未打开真实文档（用 seed）。 */
  loadedContent: string | null;
  /** 加载基线哈希（后端 ws.read_doc 回；供 ws.save_doc 校验）。 */
  baseline: number | null;
  /** 载入序号——内容变更触发编辑器 setContent（区分同路径重载）。 */
  loadSeq: number;
  /** 本次载入后请求跳转的目标行（搜索命中跳行）；跳后清除。null = 不跳。 */
  jumpLine: number | null;
  /** 编辑模型格式：text=md/纯文本（ws.read_doc/save_doc）；html=docx 编辑链（doc.open_for_edit/save_from_edit）。 */
  editFormat: "text" | "html";
}

let state: EdState = {
  openPath: null,
  tabState: "normal",
  pendingDiffCount: 0,
  loadedContent: null,
  baseline: null,
  loadSeq: 0,
  jumpLine: null,
  editFormat: "text",
};
const listeners = new Set<() => void>();

function emit() {
  for (const l of listeners) l();
}
function setState(patch: Partial<EdState>) {
  state = { ...state, ...patch };
  emit();
}

/**
 * @GOV
 * codes:    BR-ED-STATE-001-RB-ED-ED-EDIT-002
 * type:     RB
 * chain:    ED-EDIT
 * rules:    BR-ED-STATE-001
 * boundary: in=editSessionEvent | out=TabState(dirtyOnEditSavedOnPersistExternalOnConflict)
 */
/// 编辑会话脏态守则（BR-ED-STATE-001 投影）：打开文档为 normal/saved；编辑即转 dirty；
/// 落盘转 saving→saved；外部变更转 external_changed；待裁决转 pending_diff。状态互斥，权威落盘归 WS。
export const edStore = {
  subscribe(cb: () => void) {
    listeners.add(cb);
    return () => listeners.delete(cb);
  },
  getSnapshot(): EdState {
    return state;
  },
  /** 打开文档（初始 saved 基线态）。 */
  openDoc(path: string) {
    setState({ openPath: path, tabState: "saved", pendingDiffCount: 0 });
  },
  /** 载入真实文档内容 + 加载基线（ws.read_doc / doc.open_for_edit 投影）；编辑器据 loadSeq 变更 setContent；
   *  jumpLine 跳行；editFormat=html 时为 docx 编辑链（内容为 HTML）。 */
  loadDoc(
    path: string,
    content: string,
    baseline: number,
    jumpLine: number | null = null,
    editFormat: "text" | "html" = "text",
  ) {
    setState({
      openPath: path,
      loadedContent: content,
      baseline,
      tabState: "saved",
      pendingDiffCount: 0,
      loadSeq: state.loadSeq + 1,
      jumpLine,
      editFormat,
    });
  },
  /** 跳转完成后清除目标行（避免后续重渲染重复跳）。 */
  clearJump() {
    if (state.jumpLine !== null) setState({ jumpLine: null });
  },
  /** 保存成功：刷新基线，回 saved 态。 */
  markSaved(baseline: number) {
    setState({ baseline, tabState: "saved" });
  },
  /** 编辑产生脏态（编辑器 onUpdate 驱动）。 */
  markDirty() {
    if (state.tabState !== "external_changed" && state.tabState !== "pending_diff") {
      setState({ tabState: "dirty" });
    }
  },
  /** 设置标签态（保存链/外部变更/DE 投影注入）。 */
  setTabState(tabState: TabState) {
    setState({ tabState });
  },
  /** 切换/关闭 workspace 时清空编辑会话（避免残留旧文档内容/基线）。 */
  reset() {
    setState({
      openPath: null,
      tabState: "normal",
      pendingDiffCount: 0,
      loadedContent: null,
      baseline: null,
      loadSeq: 0,
      jumpLine: null,
      editFormat: "text",
    });
  },
};

export function useEdStore(): EdState {
  return useSyncExternalStore(edStore.subscribe, edStore.getSnapshot);
}
