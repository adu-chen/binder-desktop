import { useSyncExternalStore } from "react";

/**
 * ED 编辑会话状态投影层（ED-EDIT · SM-ED-01 · X-FORBID-007）。
 * tabState 是编辑会话/脏态的视图投影：编辑产生脏态（前端编辑器感知），落盘/外部变更/待裁决由
 * 保存链（WS）、外部变更事件、DE 投影驱动。前端不另立与后端可漂移的权威落盘态。
 *
 * 多文档会话（PI-FE-27 / PI-B）：sessions 按路径保存每个已打开文档的会话（内容/基线/态/格式），
 * activePath 指向当前编辑的文档。多个文档标签各自保留未保存编辑——切换标签不丢草稿。
 * openPath/tabState/loadedContent/baseline/editFormat 等扁平字段为「活动会话」派生投影，
 * 保既有消费方（edService/Editor/WorkspaceMenu/Composer/deService）零改动兼容。
 */

/** 编辑标签 5 状态（视觉互斥；由 center TabStrip 渲染）。 */
export type TabState =
  | "normal"
  | "dirty"
  | "saving"
  | "saved"
  | "pending_diff";

/** 后端 SM-WS-01 保存机运行态（Rust SaveState 的 Debug 名，A-WS-M-S-01 §2）。 */
export type RustSaveState = "Idle" | "Dirty" | "Saving" | "Saved" | "SaveError";

/** Rust SM-WS-01 SaveState → TabState（A-CORE-X-L-04 Task 2-C，人类决策 2026-06-27）。
 *  仅保存**动作**态经广播投影：Saving→saving、Saved→saved。Idle/Dirty 脏态留前端本地（高频不走 IPC）；
 *  SaveError 不投影——失败(dirty) 由前端据错误码细分（edService catch）。 */
export function mapSaveState(s: RustSaveState): Extract<TabState, "saving" | "saved"> | null {
  switch (s) {
    case "Saving":
      return "saving";
    case "Saved":
      return "saved";
    default:
      return null;
  }
}

/** 编辑模型格式：text=md/纯文本；html=docx 编辑链（doc.save_from_edit 回写）；
 *  html_file=.html 独立文件（PI-ED-13 / REQ-ED-008：富文本编辑 + HTML 序列化，经 ws.save_doc 文本落盘）。 */
export type EditFormat = "text" | "html" | "html_file";

/** 单文档编辑会话（按路径独立保存，切换标签不互相覆盖）。
 *  三态归属（A-CORE-C-T-01 §1.2/§4）：content 是 LogicalState 的派生缓存（序列化副本），
 *  非权威源。权威逻辑态是 TipTap ProseMirror doc（editor.state.doc）。
 *  serialize() 是只读导出，setContent() 是外部数据加载人逻辑态。
 *  需要真值时必须从 editor.state.doc 取。 */
export interface EditSession {
  /** 文档内容；text=纯文本/md，html=docx 编辑链 HTML。null=已建会话但未载入内容（占位）。
   *  归属：LogicalState 派生缓存（非权威源）。 */
  content: string | null;
  /** 加载基线哈希（后端 ws.read_doc / doc.open_for_edit 回；供保存校验）。 */
  baseline: number | string | null;
  tabState: TabState;
  /** 编辑模型格式（EditFormat）。 */
  editFormat: EditFormat;
  /** docx 打开转换进行中（SM-DOC-01 OpenConverting 投影，PI-ED-07 #7 加载反馈）。 */
  converting?: boolean;
}

interface EdState {
  /** 已打开文档会话表（按相对路径键控）。 */
  sessions: Record<string, EditSession>;
  /** 当前活动文档路径（= 编辑器正在编辑的文档）；null=无活动文档。 */
  activePath: string | null;
  /** 载入序号——loadDoc/切换活动会话时递增，驱动编辑器 setContent（区分同路径重载）。 */
  loadSeq: number;
  /** 本次载入后请求跳转的目标行（搜索命中跳行）；跳后清除。null = 不跳。 */
  jumpLine: number | null;

  // —— 以下为「活动会话」派生投影，保既有消费方兼容（X-FORBID-007 单一权威：sessions[activePath]） ——
  openPath: string | null;
  tabState: TabState;
  pendingDiffCount: number;
  /** 活动会话内容（编辑器据此 setContent）；null = 未打开真实文档。 */
  loadedContent: string | null;
  baseline: number | string | null;
  editFormat: EditFormat;
  /** 活动会话 docx 转换进行中（SM-DOC-01 投影）。 */
  converting: boolean;
}

type Core = Pick<EdState, "sessions" | "activePath" | "loadSeq" | "jumpLine">;

/** 由活动会话派生扁平投影字段（活动会话不存在时回落默认）。 */
function derive(core: Core): Omit<EdState, keyof Core> {
  const sess = core.activePath ? core.sessions[core.activePath] : undefined;
  return {
    openPath: core.activePath,
    tabState: sess?.tabState ?? "normal",
    pendingDiffCount: 0,
    loadedContent: sess?.content ?? null,
    baseline: sess?.baseline ?? null,
    editFormat: sess?.editFormat ?? "text",
    converting: sess?.converting ?? false,
  };
}

let state: EdState = {
  sessions: {},
  activePath: null,
  loadSeq: 0,
  jumpLine: null,
  ...derive({ sessions: {}, activePath: null, loadSeq: 0, jumpLine: null }),
};
const listeners = new Set<() => void>();

function emit() {
  for (const l of listeners) l();
}
/** 提交核心态 → 重算派生投影 → 通知。 */
function commit(core: Core) {
  state = { ...core, ...derive(core) };
  emit();
}
/** 修改指定路径会话的部分字段（会话不存在时为空操作）。异步保存/外部变更轮询按 path 锁定的基础。 */
function patchSession(path: string, patch: Partial<EditSession>) {
  const cur = state.sessions[path];
  if (!cur) return;
  commit({
    sessions: { ...state.sessions, [path]: { ...cur, ...patch } },
    activePath: state.activePath,
    loadSeq: state.loadSeq,
    jumpLine: state.jumpLine,
  });
}
/** 仅修改活动会话的部分字段（无活动会话时为空操作）。 */
function patchActive(patch: Partial<EditSession>) {
  if (state.activePath) patchSession(state.activePath, patch);
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
/// 落盘转 saving→saved；待裁决转 pending_diff。状态互斥，权威落盘归 WS。
/// 多文档会话按路径独立保存（PI-FE-27）：脏态/格式/基线均属各自会话，切换活动会话不互相覆盖。
export const edStore = {
  subscribe(cb: () => void) {
    listeners.add(cb);
    return () => listeners.delete(cb);
  },
  getSnapshot(): EdState {
    return state;
  },
  /** 打开文档（初始 saved 基线态，内容待 loadDoc 载入）。 */
  openDoc(path: string) {
    commit({
      sessions: {
        ...state.sessions,
        [path]: { content: null, baseline: null, tabState: "saved", editFormat: "text" },
      },
      activePath: path,
      loadSeq: state.loadSeq,
      jumpLine: state.jumpLine,
    });
  },
  /** 载入真实文档内容 + 加载基线（ws.read_doc / doc.open_for_edit 投影）；建/刷新该路径会话并置为活动；
   *  编辑器据 loadSeq 变更 setContent；jumpLine 跳行；editFormat=html 时为 docx 编辑链（内容为 HTML）。 */
  loadDoc(
    path: string,
    content: string,
    baseline: number | string,
    jumpLine: number | null = null,
    editFormat: EditFormat = "text",
  ) {
    commit({
      sessions: {
        ...state.sessions,
        [path]: { content, baseline, tabState: "saved", editFormat },
      },
      activePath: path,
      loadSeq: state.loadSeq + 1,
      jumpLine,
    });
  },
  /** 切换活动会话到已打开文档（不重新读盘 → 保留该会话未保存编辑）；loadSeq 递增触发编辑器载入会话内容。 */
  setActivePath(path: string) {
    if (path === state.activePath || !state.sessions[path]) return;
    commit({
      sessions: state.sessions,
      activePath: path,
      loadSeq: state.loadSeq + 1,
      jumpLine: null,
    });
  },
  /** 暂存某会话的当前编辑内容（切换标签前由编辑器序列化回写 → 不丢草稿）；不触发 loadSeq/不改脏态。 */
  updateContent(path: string, content: string) {
    const cur = state.sessions[path];
    if (!cur) return;
    commit({
      sessions: { ...state.sessions, [path]: { ...cur, content } },
      activePath: state.activePath,
      loadSeq: state.loadSeq,
      jumpLine: state.jumpLine,
    });
  },
  /** 跳转完成后清除目标行（避免后续重渲染重复跳）。 */
  clearJump() {
    if (state.jumpLine !== null) {
      commit({ sessions: state.sessions, activePath: state.activePath, loadSeq: state.loadSeq, jumpLine: null });
    }
  },
  /** 保存成功：刷新指定会话基线，回 saved 态。按 path 锁定 → 异步保存期间切标签不误改他会话（PI-FE-29）。
   *  会话已被关闭则为空操作（文件仍已落盘，无会话可更新）。 */
  markSaved(path: string, baseline: number | string) {
    patchSession(path, { baseline, tabState: "saved" });
  },
  /** 仅刷新会话加载基线（A-CORE-X-L-04 Task 2-C）：保存成功后由后端返回值回填；saved 态由 SM-WS-01 广播驱动。 */
  setBaseline(path: string, baseline: number | string) {
    patchSession(path, { baseline });
  },
  /**
   * 投影后端 SM-WS-01 保存机转移（A-CORE-X-L-04 Task 2-C · X-FORBID-007）：由 smListeners 监听
   * `sm:SM-WS-01:state_changed` 调用。仅保存动作态：Saving→saving、Saved→saved（按 path 锁定）。
   * pending_diff 为更高优先呈现态，不被保存动作广播覆盖。
   */
  applyWsSaveState(relPath: string, saveState: RustSaveState) {
    const tab = mapSaveState(saveState);
    if (!tab) return;
    const cur = state.sessions[relPath];
    if (!cur || cur.tabState === "pending_diff") return;
    patchSession(relPath, { tabState: tab });
  },
  /**
   * 投影后端 SM-DOC-01 docx 开/存机转移（PI-ED-07 #7 · X-FORBID-007）：OpenConverting → 会话
   * converting（加载反馈呈现）；Ready/OpenError → 清除。会话不存在时空操作（会话由打开流程建立）。
   */
  applyDocConvertState(relPath: string, docState: string) {
    if (!state.sessions[relPath]) return;
    patchSession(relPath, { converting: docState === "OpenConverting" });
  },
  /** 编辑产生脏态（编辑器 onUpdate 驱动，作用于活动会话）。 */
  markDirty() {
    const sess = state.activePath ? state.sessions[state.activePath] : undefined;
    if (sess && sess.tabState !== "pending_diff") {
      patchActive({ tabState: "dirty" });
    }
  },
  /** 设置指定会话标签态（保存链/外部变更轮询/DE 投影注入）。按 path 锁定（PI-FE-29）：异步保存/轮询
   *  在 await 前捕获目标 path，完成后据此回写，避免期间切标签污染他会话。 */
  setTabState(path: string, tabState: TabState) {
    patchSession(path, { tabState });
  },
  /** 关闭某文档会话（关闭标签时调用）；若为活动会话则置空活动（相邻激活由 tabsStore 决定）。 */
  closeSession(path: string) {
    if (!state.sessions[path]) return;
    const sessions = { ...state.sessions };
    delete sessions[path];
    commit({
      sessions,
      activePath: state.activePath === path ? null : state.activePath,
      loadSeq: state.loadSeq,
      jumpLine: state.jumpLine,
    });
  },
  /** 切换/关闭 workspace 时清空全部编辑会话（避免残留旧文档内容/基线）。 */
  reset() {
    commit({ sessions: {}, activePath: null, loadSeq: 0, jumpLine: null });
  },
};

export function useEdStore(): EdState {
  return useSyncExternalStore(edStore.subscribe, edStore.getSnapshot);
}
