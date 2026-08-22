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

/** 磁盘编辑格式：text=纯文本；markdown=.md Codec；html=docx 编辑链（doc.save_from_edit 回写）；
 *  html_file=.html 独立文件（PI-ED-13 / REQ-ED-008：富文本编辑 + HTML 序列化，经 ws.save_doc 文本落盘）。 */
export type EditFormat = "text" | "markdown" | "html" | "html_file";

/** EditSession/OpenDocSnapshot.content 的编码；与磁盘 editFormat 分离。 */
export type ContentFormat = "text" | "html";

export type OpenDocSnapshotSource = "active_editor" | "active_editor_session" | "open_editor_session";

export type AgentEditableBlockRole = "content" | "container" | "atom";

export interface AgentEditableBlockSnapshot {
  blockId: string;
  blockIndex: number;
  /** HTML 语义标签投影；nodeType 保留 ProseMirror schema 名。 */
  blockType: string;
  nodeType: string;
  role: AgentEditableBlockRole;
  parentBlockId: string | null;
  depth: number;
  ordinal: number;
  text: string;
  attrs: Record<string, unknown>;
  marks: { type: string; range: [number, number]; attrs?: Record<string, unknown> }[];
  nodeJson: Record<string, unknown>;
  fingerprint: string;
  capabilities: string[];
}

export interface StructuredBlockOperation {
  op: "replace_node" | "insert_before" | "insert_after" | "append" | "delete_node";
  blockId: string;
  fingerprint: string;
  expectedNodeType: string;
  beforeNode: Record<string, unknown>;
  // AG-TASK-01 F2：delete_node 时 afterNode 为 null（删除标记）。
  afterNode: Record<string, unknown> | null;
}

export interface StructuredBlockEditPayload {
  payloadVersion: 1;
  base: { sessionVersion: number; contentHash: string };
  operations: StructuredBlockOperation[];
}

export interface OpenDocSnapshot {
  relPath: string;
  content: string;
  /** BDocx 持久恢复使用的磁盘加载基线；旧快照可缺省。 */
  baseline?: number | string | null;
  editFormat: EditFormat;
  contentFormat: ContentFormat;
  source: OpenDocSnapshotSource;
  dirty: boolean;
  sessionVersion: number;
  contentHash: string;
  capturedAt: number;
  /** 仅活动富文档编辑器提供，直接从同次 ProseMirror LogicalState 投影。 */
  blockSnapshots?: AgentEditableBlockSnapshot[];
}

export type SessionApplyResult =
  | { ok: true; snapshot: OpenDocSnapshot }
  | { ok: false; code: string; message: string };

/**
 * 异步读取完成后写回会话的控制项。
 *
 * 正常打开必须激活该路径；但当用户已切换到另一文档时，旧请求的结果只能补全原会话，
 * 不得夺回 activePath 或推进 loadSeq。这使编辑器只响应当前活动路径的受控载入。
 */
export interface LoadDocOptions {
  activate?: boolean;
  /** DOCX 转换时实际检测到的样式风险，随会话切换而切换。 */
  styleNotices?: string[];
}

export function hashLogicalContent(content: string): string {
  let hash = 0x811c9dc5;
  for (let i = 0; i < content.length; i += 1) {
    hash ^= content.charCodeAt(i);
    hash = Math.imul(hash, 0x01000193);
  }
  return (hash >>> 0).toString(16).padStart(8, "0");
}

/** 该 tabState 是否代表内容偏离磁盘态、尚未安全落盘（供 tab-dot 视觉与关闭前聚合脏态复用同一判断）。 */
export function isDirtyTabState(tabState: TabState): boolean {
  return tabState === "dirty" || tabState === "pending_diff" || tabState === "saving";
}

function makeSession(
  content: string | null,
  baseline: number | string | null,
  tabState: TabState,
  editFormat: EditFormat,
  contentFormat: ContentFormat = editFormat === "html" || editFormat === "html_file" ? "html" : "text",
  version = content === null ? 0 : 1,
  converting = false,
  styleNotices: string[] = [],
): EditSession {
  return {
    content,
    baseline,
    tabState,
    editFormat,
    contentFormat,
    converting,
    styleNotices,
    sessionVersion: version,
    contentHash: content === null ? "" : hashLogicalContent(content),
  };
}

/** 单文档编辑会话（按路径独立保存，切换标签不互相覆盖）。
 *  三态归属（A-CORE-C-T-01 §1.2/§4）：激活文档的权威 LogicalState 是 TipTap ProseMirror doc；
 *  非激活打开文档的 content 是该 session 当前 LogicalState 的序列化持有者。
 *  sessionVersion/contentHash 用于 AG/DE 基线校验，禁止已打开文档退回 DiskState。 */
export interface EditSession {
  /** 文档 LogicalState 序列化内容；编码由 contentFormat 声明。null=已建会话但未载入内容（占位）。
   *  归属：激活文档为 LogicalState 派生缓存；非激活打开文档为序列化 LogicalState。 */
  content: string | null;
  /** 加载基线哈希（后端 ws.read_doc / doc.open_for_edit 回；供保存校验）。 */
  baseline: number | string | null;
  tabState: TabState;
  /** 编辑模型格式（EditFormat）。 */
  editFormat: EditFormat;
  /** content 编码；Markdown 终态模型为 html，源码保护路径为 text。 */
  contentFormat: ContentFormat;
  /** ED session 版本；每次受控内容回写递增，用于 open session diff 防基线偏移。 */
  sessionVersion: number;
  /** content 的稳定哈希；与 sessionVersion 共同构成 DiffProjection 基线。 */
  contentHash: string;
  /** docx 打开转换进行中（SM-DOC-01 OpenConverting 投影，PI-ED-07 #7 加载反馈）。 */
  converting?: boolean;
  /** 当前文档的样式风险告知；仅来自 DOCX 转换报告。 */
  styleNotices?: string[];
  /** Markdown 文档的源码视图开关（仅 editFormat=markdown 有效）。true=显示 Markdown 源码，false=WYSIWYG 富文本。 */
  sourceView?: boolean;
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
  contentFormat: ContentFormat;
  /** 活动会话 docx 转换进行中（SM-DOC-01 投影）。 */
  converting: boolean;
  /** 活动会话的 DOCX 样式风险告知。 */
  styleNotices: string[];
  /** Markdown 文档当前是否在源码视图（仅 editFormat=markdown 时有效）。 */
  sourceView: boolean;
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
    contentFormat: sess?.contentFormat ?? "text",
    converting: sess?.converting ?? false,
    styleNotices: sess?.styleNotices ?? [],
    sourceView: sess?.sourceView ?? false,
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
        [path]: makeSession(null, null, "saved", "text"),
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
    contentFormat: ContentFormat = editFormat === "html" || editFormat === "html_file" ? "html" : "text",
    options: LoadDocOptions = {},
  ) {
    const activate = options.activate ?? true;
    commit({
      sessions: {
        ...state.sessions,
        [path]: makeSession(content, baseline, "saved", editFormat, contentFormat, 1, false, options.styleNotices),
      },
      activePath: activate ? path : state.activePath,
      loadSeq: activate ? state.loadSeq + 1 : state.loadSeq,
      jumpLine: activate ? jumpLine : state.jumpLine,
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
  /** Markdown 文档切换源码/富文本视图（仅 editFormat=markdown 时有效）。 */
  setSourceView(path: string, sourceView: boolean) {
    const cur = state.sessions[path];
    if (!cur || cur.editFormat !== "markdown") return;
    patchSession(path, { sourceView });
  },
  /** 新建 Markdown 的初始内容首次成功解析后，把占位 text 会话提升为结构化 HTML LogicalState。 */
  setContentFormat(path: string, contentFormat: ContentFormat) {
    const cur = state.sessions[path];
    if (!cur || cur.contentFormat === contentFormat) return;
    patchSession(path, { contentFormat });
  },
  /** 暂存某会话的当前编辑内容（切换标签前由编辑器序列化回写 → 不丢草稿）；不触发 loadSeq/不改脏态。 */
  updateContent(path: string, content: string) {
    const cur = state.sessions[path];
    if (!cur) return;
    if (cur.content === content) return;
    commit({
      sessions: {
        ...state.sessions,
        [path]: {
          ...cur,
          content,
          sessionVersion: cur.sessionVersion + 1,
          contentHash: hashLogicalContent(content),
        },
      },
      activePath: state.activePath,
      loadSeq: state.loadSeq,
      jumpLine: state.jumpLine,
    });
  },
  getOpenSnapshots(activeSource: Extract<OpenDocSnapshotSource, "active_editor" | "active_editor_session"> = "active_editor"): OpenDocSnapshot[] {
    const capturedAt = Date.now();
    return Object.entries(state.sessions)
      .flatMap(([path, sess]) => {
        if (sess.content === null) return [];
        return [{
          relPath: path,
          content: sess.content,
          baseline: sess.baseline,
          editFormat: sess.editFormat,
          contentFormat: sess.contentFormat,
          source: path === state.activePath ? activeSource : "open_editor_session",
          dirty: isDirtyTabState(sess.tabState),
          sessionVersion: sess.sessionVersion,
          contentHash: sess.contentHash,
          capturedAt,
        } satisfies OpenDocSnapshot];
      });
  },
  restoreSnapshots(snapshots: OpenDocSnapshot[], activePath: string | null = null) {
    if (!snapshots.length) {
      commit({
        sessions: state.sessions,
        activePath: null,
        loadSeq: state.loadSeq,
        jumpLine: state.jumpLine,
      });
      return;
    }
    const sessions = { ...state.sessions };
    for (const snap of snapshots) {
      sessions[snap.relPath] = {
        content: snap.content,
        baseline: snap.baseline ?? sessions[snap.relPath]?.baseline ?? null,
        tabState: snap.dirty ? "dirty" : "saved",
        editFormat: snap.editFormat,
        contentFormat: snap.contentFormat ?? (snap.editFormat === "html" || snap.editFormat === "html_file" ? "html" : "text"),
        sessionVersion: snap.sessionVersion,
        contentHash: snap.contentHash || hashLogicalContent(snap.content),
      };
    }
    const nextActivePath = activePath && sessions[activePath]?.content !== null
      ? activePath
      : null;
    commit({
      sessions,
      activePath: nextActivePath,
      loadSeq: nextActivePath ? state.loadSeq + 1 : state.loadSeq,
      jumpLine: state.jumpLine,
    });
  },
  applySessionContent(
    path: string,
    nextContent: string,
    baseSessionVersion: number,
    baseContentHash: string,
  ): SessionApplyResult {
    const cur = state.sessions[path];
    if (!cur || cur.content === null) {
      return { ok: false, code: "ED_OPEN_SESSION_REQUIRED", message: "目标文档未打开，不能应用 open session diff" };
    }
    if (path === state.activePath) {
      return { ok: false, code: "ED_OPEN_SESSION_ACTIVE", message: "激活文档必须走 current editor 草稿路径" };
    }
    if (cur.sessionVersion !== baseSessionVersion || cur.contentHash !== baseContentHash) {
      return { ok: false, code: "ED_OPEN_SESSION_BASE_MISMATCH", message: "目标文档 LogicalState 已变化，diff 已失效" };
    }
    const nextVersion = cur.sessionVersion + 1;
    const nextHash = hashLogicalContent(nextContent);
    const next: EditSession = {
      ...cur,
      content: nextContent,
      tabState: "dirty",
      sessionVersion: nextVersion,
      contentHash: nextHash,
    };
    commit({
      sessions: { ...state.sessions, [path]: next },
      activePath: state.activePath,
      loadSeq: state.loadSeq,
      jumpLine: state.jumpLine,
    });
    return {
      ok: true,
      snapshot: {
        relPath: path,
        content: nextContent,
        editFormat: next.editFormat,
        contentFormat: next.contentFormat,
        source: "open_editor_session",
        dirty: true,
        sessionVersion: nextVersion,
        contentHash: nextHash,
        capturedAt: Date.now(),
      },
    };
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
    if (sess) {
      patchActive({ tabState: "dirty" });
    }
  },
  /** 设置指定会话标签态（保存链/外部变更轮询/DE 投影注入）。按 path 锁定（PI-FE-29）：异步保存/轮询
   *  在 await 前捕获目标 path，完成后据此回写，避免期间切标签污染他会话。 */
  setTabState(path: string, tabState: TabState) {
    patchSession(path, { tabState });
  },
  /** 关闭某文档会话（关闭标签时调用）；若为活动会话则置空活动（相邻激活由 tabsStore 决定）。
   *  审计修复（2026-07-26 · D11）：不 delete session，保留缓存逻辑状态供重开时 setActivePath 复用。
   *  workspace 切换时 reset() 全清，不会跨 workspace 污染。 */
  closeSession(path: string) {
    if (!state.sessions[path]) return;
    const sessions = { ...state.sessions };
    commit({
      sessions,
      activePath: state.activePath === path ? null : state.activePath,
      loadSeq: state.loadSeq,
      jumpLine: state.jumpLine,
    });
  },
  /** 用户明确丢弃未保存内容或资源操作已改变磁盘身份时，彻底移除旧 LogicalState。 */
  discardSession(path: string) {
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
  /** 丢弃尚未形成逻辑内容的打开占位会话。
   * 与 closeSession 不同：后者保留已加载会话以维护 BDocx 的稳定 blockId；本方法只供打开失败路径
   * 清理空占位，避免文件不存在/转换失败时留下可被重开的假文档。 */
  discardOpenPlaceholder(path: string) {
    const session = state.sessions[path];
    if (!session || session.content !== null) return;
    const sessions = { ...state.sessions };
    delete sessions[path];
    commit({
      sessions,
      activePath: state.activePath === path ? null : state.activePath,
      loadSeq: state.loadSeq,
      jumpLine: state.jumpLine,
    });
  },
  /** 关闭 relPath 以 dirPath/ 为前缀的所有编辑器会话（目录删除后清理，WS-DIR-DELETE-01）；
   *  活动会话在被删目录内则置空并重置载入序号（对齐 reset）；其余会话/活动保留（不误伤未保存编辑）。 */
  closeDocsUnder(dirPath: string) {
    const prefix = `${dirPath}/`;
    const sessions = Object.fromEntries(
      Object.entries(state.sessions).filter(([path]) => !path.startsWith(prefix)),
    );
    const activeInDir = state.activePath != null && state.activePath.startsWith(prefix);
    commit({
      sessions,
      activePath: activeInDir ? null : state.activePath,
      loadSeq: activeInDir ? 0 : state.loadSeq,
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
