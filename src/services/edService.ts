import { readDoc, saveDoc, checkExternalChange } from "./wsService";
import { openDocxForEdit, saveDocxFromEdit, isDocxEditable } from "./docService";
import { edStore } from "../store/edStore";

/**
 * ED 编排 Service（X-SEQ-002）：打开/保存真实文档，连接 PI-BE-01 落盘 command 与编辑器投影。
 * 落盘往返以后端基线为准（BR-WS-OBS-001）；前端不另立可漂移落盘态（X-FORBID-007）。
 */

interface IpcLikeError {
  code?: string;
  message?: string;
}

/** 是否 .html/.htm 独立文件（PI-ED-13 / REQ-ED-008：富文本编辑 + HTML 文本往返，不经 DOC）。 */
export function isHtmlFile(relPath: string): boolean {
  const ext = relPath.toLowerCase().split(".").pop() ?? "";
  return ext === "html" || ext === "htm";
}

/**
 * @GOV
 * codes:    BR-ED-DATA-002-RB-ED-ED-EDIT-004
 * type:     RB
 * chain:    ED-EDIT
 * rules:    BR-ED-DATA-002
 * boundary: in=DocReadDtoOrDocxEditModel | out=edSessionContent(unifiedModelDocxViaDocHtmlFileAsTextMdToText)
 */
/// md/docx/html 统一编辑模型守则（BR-ED-DATA-002，2026-07-03 扩展至三格式）：可编辑 docx
/// （*.draft.docx）经 **DOC 编辑链**（doc.open_for_edit）转为统一编辑模型 HTML 后进编辑器
/// （editFormat=html）；**.html 独立文件**为文本格式经 ws.read_doc 直接进富文本模型
/// （editFormat=html_file，HTML 序列化往返，**不经 DOC 二进制管线**）；md/纯文本经 ws.read_doc
/// 进文本模型（editFormat=text）。docx 二进制解析/生成全部归 DOC（docService 仅薄客户端），
/// ED/前端不含 docx 二进制处理路径——转换归 DOC、编辑归 ED 的分工在打开入口落地。
/** 打开文档：docx 经 DOC 编辑链（doc.open_for_edit → HTML），其余经 ws.read_doc（文本）→ 载入编辑器
 *  （内容 + 加载基线）；jumpLine 时载入后跳转到该行。 */
export async function openDocument(relPath: string, jumpLine: number | null = null): Promise<void> {
  // 立即建占位会话并置活动（PI-ED-07 #7）：大 docx 转换耗时秒级，先切标签显示加载骨架、
  // 不阻塞交互；SM-DOC-01 OpenConverting 广播叠加"正在转换"反馈。已有会话（切回标签）不重建。
  const hadSession = Boolean(edStore.getSnapshot().sessions[relPath]);
  if (!hadSession) edStore.openDoc(relPath);
  try {
    if (isDocxEditable(relPath)) {
      // docx 编辑链：docx → 可编辑 HTML（保留样式子集），editFormat=html。
      const { html, baseline } = await openDocxForEdit(relPath);
      edStore.loadDoc(relPath, html, baseline, jumpLine, "html");
      return;
    }
    const { content, baseline } = await readDoc(relPath);
    // .html 独立文件 → 富文本模型（html_file，HTML 往返）；其余 → 文本模型。
    edStore.loadDoc(relPath, content, baseline, jumpLine, isHtmlFile(relPath) ? "html_file" : "text");
  } catch (e) {
    const err = e as IpcLikeError;
    // 打开失败：撤销本次新建的占位会话（不留假空文档）；既有会话保留（重载失败不丢草稿）。
    if (!hadSession) edStore.closeSession(relPath);
    console.error(`open document failed [${err.code ?? "?"}]: ${err.message ?? e}`);
  }
}

/** 保存结果：成功 ok；冲突需用户裁决（覆盖/重载）。 */
export type SaveOutcome = { ok: true } | { ok: false; conflict: boolean; message: string };

/**
 * 保存当前文档（⌘S）：ws.save_doc 带加载基线。磁盘≠基线且未 override → WS_SAVE_CONFLICT（SM-WS-01）。
 * override=true 表示用户显式确认覆盖外部修改。
 */
export async function saveDocument(content: string, override = false): Promise<SaveOutcome> {
  const { openPath, baseline } = edStore.getSnapshot();
  if (!openPath) return { ok: false, conflict: false, message: "无打开的文档" };
  const path = openPath; // 锁定本次保存目标：await 期间用户切标签也只回写该会话（PI-FE-29）。
  // saving/saved 态由后端 SM-WS-01 广播投影（X-FORBID-007）；此处仅回填基线（后端返回值）+ 错误细分。
  try {
    const newBaseline = await saveDoc(path, content, baseline, override);
    edStore.setBaseline(path, newBaseline);
    return { ok: true };
  } catch (e) {
    const err = e as IpcLikeError;
    const conflict = err.code === "WS_SAVE_CONFLICT";
    // 冲突回 external_changed 等待裁决；其余失败回脏态（不谎称已存）。SaveError 广播不细分，由此处据错误码定。
    edStore.setTabState(path, conflict ? "external_changed" : "dirty");
    return { ok: false, conflict, message: err.message ?? String(e) };
  }
}

/**
 * 保存 docx 文档（⌘S，编辑链）：编辑 HTML → doc.save_from_edit（HTML→docx→WS 原子落盘）带加载基线。
 * 磁盘≠基线且未 override → WS_SAVE_CONFLICT（SM-WS-01，原 docx 不变）。状态投影同 saveDocument。
 */
export async function saveDocxDocument(html: string, override = false): Promise<SaveOutcome> {
  const { openPath, baseline } = edStore.getSnapshot();
  if (!openPath) return { ok: false, conflict: false, message: "无打开的文档" };
  const path = openPath; // 锁定本次保存目标（PI-FE-29）。
  // saving/saved 态由后端 SM-WS-01 广播投影（X-FORBID-007）；此处仅回填基线 + 错误细分。
  try {
    const newBaseline = await saveDocxFromEdit(path, html, baseline, override);
    edStore.setBaseline(path, newBaseline);
    return { ok: true };
  } catch (e) {
    const err = e as IpcLikeError;
    const conflict = err.code === "WS_SAVE_CONFLICT";
    edStore.setTabState(path, conflict ? "external_changed" : "dirty");
    return { ok: false, conflict, message: err.message ?? String(e) };
  }
}

/** 冲突后用户选择"重载"：丢弃本地编辑，重新从磁盘载入。 */
export async function reloadDocument(): Promise<void> {
  const { openPath } = edStore.getSnapshot();
  if (openPath) await openDocument(openPath);
}

/**
 * @GOV
 * codes:    BR-ED-STATE-003-RB-ED-ED-EDIT-003
 * type:     RB
 * chain:    ED-EDIT
 * rules:    BR-ED-STATE-003, BR-WS-OBS-001
 * boundary: in=diskBaselineComparison | out=TabState(externalChangedOnDriftAwaitingExplicitOverrideOrReload)
 */
/// 编辑会话外部变更处理守则（BR-ED-STATE-003）：订阅 WS 外部变更（比对磁盘哈希 vs 加载基线，BR-WS-OBS-001）
/// → 漂移置 external_changed（SM-ED-01 Conflicted）→ 编辑器冲突条由用户**显式**选「保留我的编辑（覆盖，
/// saveDocument override）」或「放弃并重载（reloadDocument）」；两个方向的内容损失都不静默发生（不静默覆盖
/// 外部修改、不静默重载丢脏态，SM-WS-01）。物理事件订阅延 SY-02，当前以 5s 轮询实现逻辑契约。
/**
 * 外部变更轮询（实现 external_change 逻辑契约，物理事件延 SY-02）：
 * 比对磁盘哈希与加载基线（磁盘状态 vs 逻辑状态的加载快照）。BR-ED-STATE-003 双分支（2026-07-03
 * 三态审计修复）：**无脏态**（逻辑状态无独立修改）→ **自动重载**磁盘内容刷新基线（非阻断，不弹
 * 裁决——外部修改无内容损失方）；**有脏态** → external_changed 显式裁决（双向内容损失须用户选择）。
 * saving 态跳过（磁盘已写、基线未回填的竞态窗口会误报）；已是 external_changed 不重复置。
 */
export async function pollExternalChange(): Promise<void> {
  const { openPath, baseline, tabState } = edStore.getSnapshot();
  if (!openPath || baseline === null) return;
  if (tabState === "external_changed" || tabState === "saving") return;
  const path = openPath; // 锁定被检测文档：await 期间切标签也只标记该会话（PI-FE-29）。
  const wasDirty = tabState === "dirty";
  try {
    const r = await checkExternalChange(path, baseline);
    if (!r.changed) return;
    if (wasDirty) {
      // 有脏态：进冲突裁决（保留我的编辑（覆盖）/ 放弃重载——状态栏弹出，双显式按钮）。
      edStore.setTabState(path, "external_changed");
    } else {
      // 无脏态：自动重载外部修改（逻辑状态无独立修改，无内容损失方；重载后基线刷新）。
      // 仅当该文档仍是活动文档才重载（切走后由重新激活时的会话内容兜底，避免后台抢 setContent）。
      if (edStore.getSnapshot().openPath === path) await openDocument(path);
    }
  } catch {
    /* 检测失败不打断编辑（下次轮询重试） */
  }
}
