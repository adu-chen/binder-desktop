import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { wsStore } from "../store/wsStore";

/**
 * DOC 编辑链 Service（X-SEQ-002 / A-DOC-I-P-01 §2）：docx 打开/保存经 DOC 编辑链 command。
 * 与预览链（prvService）完全分离（BR-DOC-DATA-001）：编辑链 Pandoc（docx↔HTML，保留样式子集），
 * 预览链 LibreOffice（docx→PDF 只读）。docx 编辑模型为 **HTML**（TipTap 原生），非 markdown——
 * markdown 丢失样式子集「部分支持」层（颜色/字体/字号），故编辑链产出 HTML。
 */

interface CommandResponse<T> {
  correlation_id: string;
  result?: T;
  error?: { code: string; message: string; retriable: boolean; reauth_required: boolean };
}

let seq = 0;
async function callCommand<T>(command: string, params: Record<string, unknown>): Promise<T> {
  const request = { command, correlation_id: `doc-${Date.now()}-${(seq += 1)}`, params };
  const response = await invoke<CommandResponse<T>>("dispatch", { request });
  if (response.error) throw response.error;
  if (response.result === undefined) {
    throw { code: "IPC_EMPTY_RESULT", message: `${command} 无返回`, retriable: false, reauth_required: false };
  }
  return response.result;
}

export interface DocxEditModel {
  /** 可编辑 HTML（含内联样式/data-custom-style，供 TipTap setContent(html)）。 */
  html: string;
  /** 加载基线（docx 字节内容哈希），保存时回传供外部变更冲突保护（SM-WS-01）。 */
  baseline: number | string;
  /** 与当前 HTML 同次转换得出的样式检测/风险报告。 */
  styleReport: DocxStyleReport;
}

export interface DocxStyleReport {
  detected_features: string[];
  verified: string[];
  required_pending: string[];
  best_effort: string[];
  template_inherited: string[];
  unsupported: string[];
  notices: string[];
  color_recovery: {
    detected_colored_runs: number;
    restored_runs: number;
    unrecovered_runs: number;
  };
  cell_shading_recovery: {
    detected_shaded_cells: number;
    restored_cells: number;
    unrecovered_cells: number;
  };
  image_recovery: {
    detected_images: number;
    restored_images: number;
    unrecovered_images: number;
    detected_resources: number;
    restored_resources: number;
    detected_sized_images: number;
    restored_sized_images: number;
    unrecovered_sized_images: number;
    missing_resources: string[];
  };
}

const MEDIA_REF_ATTRIBUTE = "data-binder-media-src";

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function tagAttribute(tag: string, name: string): string | null {
  const match = tag.match(new RegExp(`\\s${escapeRegExp(name)}\\s*=\\s*(["'])(.*?)\\1`, "i"));
  return match?.[2] ?? null;
}

function setTagAttribute(tag: string, name: string, value: string): string {
  const pattern = new RegExp(`(\\s${escapeRegExp(name)}\\s*=\\s*)(["'])(.*?)\\2`, "i");
  if (pattern.test(tag)) {
    return tag.replace(pattern, (_match, prefix: string, quote: string) => `${prefix}${quote}${value}${quote}`);
  }
  const close = tag.lastIndexOf(">");
  if (close < 0) return tag;
  const before = tag.slice(0, close);
  const insertAt = before.trimEnd().endsWith("/") ? before.lastIndexOf("/") : close;
  return `${tag.slice(0, insertAt)} ${name}="${value}"${tag.slice(insertAt)}`;
}

function stableRelativeMediaRef(src: string): string | null {
  const normalized = src.trim().replace(/\\/g, "/").replace(/^\.\/+/, "");
  if (
    !normalized ||
    normalized.startsWith("/") ||
    normalized.startsWith("//") ||
    /^(https?:|data:|asset:|blob:|file:)/i.test(normalized)
  ) {
    return null;
  }
  const parts = normalized.split("/");
  if (parts.some((part) => !part || part === ".." || part.includes(":"))) return null;
  return parts.filter((part) => part !== ".").join("/");
}

/**
 * @GOV
 * codes:    BR-ED-DATA-003-RB-ED-ED-EDIT-006
 * type:     RB
 * chain:    ED-EDIT
 * rules:    BR-ED-DATA-003
 * boundary: in=editChainHtmlWithRelativeMediaSrc | out=htmlWithAssetProtocolUrls(backendAuthorizedPerFileReadOnly)
 */
/// 图文呈现守则（BR-ED-DATA-003）：docx 编辑链 extract-media 把图片解到后端工作目录，HTML 内为相对
/// `media/…` 路径，webview 无法直接解析。将相对 <img src> 改写为 asset 协议 URL（convertFileSrc）供只读
/// 呈现——媒体文件由后端逐文件授权入 asset scope（X-FORBID-004 守界后端把关），前端只改呈现地址、
/// 不写入媒体文件、不改文档正文模型。绝对/内联(data:)/已处理(asset:/blob:) 的 src 原样保留。
/** 将编辑链 HTML 的相对媒体路径改写为 asset URL；mediaBase 为后端 extract-media 目录绝对路径。
 *  converter 依赖注入（默认 convertFileSrc），便于单测不依赖 tauri 运行时。
 *  审计修复（2026-07-26 · D12）：同时构建 assetUrl→相对路径映射表，供保存时反向映射。 */
export function rewriteMediaSrc(
  html: string,
  mediaBase: string,
  converter: (path: string) => string = convertFileSrc,
): string {
  const base = mediaBase.replace(/[/\\]+$/, "");
  return html.replace(/<img\b[^>]*>/gi, (tag) => {
    const src = tagAttribute(tag, "src");
    if (!src) return tag;
    const declared = tagAttribute(tag, MEDIA_REF_ATTRIBUTE);
    const relative = stableRelativeMediaRef(declared ?? src);
    if (!relative) return tag;
    const abs = `${base}/${relative}`;
    const assetUrl = converter(abs);
    assetUrlToRelative.set(assetUrl, relative);
    return setTagAttribute(
      setTagAttribute(tag, MEDIA_REF_ATTRIBUTE, relative),
      "src",
      assetUrl,
    );
  });
}

/** assetUrl → 相对路径（供保存时反向映射回原始 src）。
 *  审计修复（2026-07-26 · D12）：保存 DOCX 前将 asset:// 还原为相对路径，Pandoc 可据此找到媒体文件。 */
const assetUrlToRelative = new Map<string, string>();
const mediaBaseByRelPath = new Map<string, {
  mediaBase: string;
  baseline: string;
}>();

function baselineKey(baseline: number | string | null): string {
  return baseline === null ? "" : String(baseline);
}

function registerDocumentMediaBase(
  relPath: string,
  mediaBase: string,
  baseline: number | string | null,
): void {
  mediaBaseByRelPath.set(relPath, {
    mediaBase,
    baseline: baselineKey(baseline),
  });
}

/** 把后端已授权的 baseline 媒体引用投影为当前 WebView 显示 URL；稳定相对引用仍由
 * data-binder-media-src 持有，asset URL 不进入持久化真值。 */
export function projectMediaDisplaySrc(
  mediaBase: string,
  mediaRef: string,
  converter: (path: string) => string = convertFileSrc,
): string | null {
  const relative = stableRelativeMediaRef(mediaRef);
  if (!relative) return null;
  const base = mediaBase.replace(/[/\\]+$/, "");
  if (!base) return null;
  const assetUrl = converter(`${base}/${relative}`);
  assetUrlToRelative.set(assetUrl, relative);
  return assetUrl;
}

/**
 * @GOV
 * codes:    BR-ED-DATA-003-RB-ED-ED-EDIT-026
 * type:     RB
 * chain:    ED-EDIT
 * rules:    BR-ED-DATA-003
 * term_ref: TERM-ED-008
 * boundary: in=DocumentRelativePathAndStableMediaRef | out=AuthorizedWebViewDisplayUrl(localAbsoluteCachePathExcludedFromModelProjection)
 */
/** 当前进程内按已打开 BDocx 记录后端授权的 baseline 媒体目录。模型只看到稳定 mediaRef；
 * 本地绝对缓存路径不进入工具结果或模型上下文。 */
export function projectDocumentMediaDisplaySrc(
  relPath: string,
  mediaRef: string,
): string | null {
  const media = mediaBaseByRelPath.get(relPath);
  return media ? projectMediaDisplaySrc(media.mediaBase, mediaRef) : null;
}

/**
 * @GOV
 * codes:    BR-ED-DATA-003-EFFECT-ED-ED-EDIT-026
 * type:     EFFECT
 * chain:    ED-EDIT
 * rules:    BR-ED-DATA-003
 * term_ref: TERM-ED-008
 * boundary: in=ActiveBDocxLogicalStateAndBaseline | out=DocumentMediaDisplayBaseReady(idempotentBackendAuthorization) | effect=IPC-doc.restore_edit_media | does_not=expose_local_absolute_media_path_to_model_projection
 */
/** 活动 BDocx 从持久会话恢复且尚无图片时，既有恢复流程可能尚未登记媒体目录。首次
 * ImageOperation 插入前以当前 LogicalState/baseline 幂等建立本地显示映射。 */
export async function ensureDocumentMediaDisplayBase(
  relPath: string,
  currentHtml: string,
  baseline: number | string | null,
): Promise<void> {
  const media = mediaBaseByRelPath.get(relPath);
  if (media?.baseline === baselineKey(baseline)) return;
  await restoreDocxEditMedia(relPath, currentHtml, baseline);
}

export interface ImportedProjectImage {
  assetId: string;
  sourcePath: string;
  sourceHash: string;
  mediaType: "image/png" | "image/jpeg";
  byteSize: number;
  mediaRef: string;
  displaySrc: string;
}

interface ImportedProjectImageResponse {
  assetId: string;
  sourcePath: string;
  sourceHash: string;
  mediaType: "image/png" | "image/jpeg";
  byteSize: number;
  mediaRef: string;
  mediaBase: string;
  baseline: number | string;
}

function importedProjectImageFromResponse(
  relPath: string,
  res: ImportedProjectImageResponse,
): ImportedProjectImage {
  registerDocumentMediaBase(relPath, res.mediaBase, res.baseline);
  const displaySrc = projectMediaDisplaySrc(res.mediaBase, res.mediaRef);
  if (!displaySrc) {
    throw {
      code: "DOC_IMAGE_MEDIA_DISPLAY_FAILED",
      message: "导入图片已物化，但无法生成 WebView 显示地址",
      retriable: true,
      reauth_required: false,
    };
  }
  return {
    assetId: res.assetId,
    sourcePath: res.sourcePath,
    sourceHash: res.sourceHash,
    mediaType: res.mediaType,
    byteSize: res.byteSize,
    mediaRef: res.mediaRef,
    displaySrc,
  };
}

/**
 * 将 Binder 项目文件树给出的 workspace 相对 PNG/JPEG 纳入受控图片根，并物化到当前 BDocx
 * baseline。Finder 文件由 DOM File 读取字节后走 importProjectImageData；两者都只把稳定相对身份
 * 写入 LogicalState。
 */
export async function importProjectImage(
  relPath: string,
  sourcePath: string,
  baseline: number | string | null,
): Promise<ImportedProjectImage> {
  const res = await callCommand<ImportedProjectImageResponse>(
    "doc.import_project_image",
    { relPath, sourcePath, baseline },
  );
  return importedProjectImageFromResponse(relPath, res);
}

/** 将剪贴板提供的 PNG/JPEG data URL 纳入同一受控项目资产链；原始剪贴板数据不进入 LogicalState。 */
export async function importProjectImageData(
  relPath: string,
  sourceName: string,
  sourceDataUrl: string,
  baseline: number | string | null,
): Promise<ImportedProjectImage> {
  const res = await callCommand<ImportedProjectImageResponse>(
    "doc.import_project_image",
    { relPath, sourceName, sourceDataUrl, baseline },
  );
  return importedProjectImageFromResponse(relPath, res);
}

/** 将 HTML 中的 asset:// URL 还原为相对路径（保存前调用）。
 *  未在映射表中的 URL 保持原样（如外部图片、base64 等）。 */
export function restoreMediaSrc(html: string): string {
  let result = html.replace(/<img\b[^>]*>/gi, (tag) => {
    const declared = tagAttribute(tag, MEDIA_REF_ATTRIBUTE);
    const relative = declared ? stableRelativeMediaRef(declared) : null;
    return relative ? setTagAttribute(tag, "src", relative) : tag;
  });
  for (const [assetUrl, relative] of assetUrlToRelative) {
    // asset URL 可能出现在双引号或单引号中
    result = result.split(`"${assetUrl}"`).join(`"${relative}"`);
    result = result.split(`'${assetUrl}'`).join(`'${relative}'`);
  }
  return result;
}

/** 持久快照不保存进程/授权相关的 asset URL。baseline 作为快照字段传给后端；后端若需兼容现有
 * 数据库 schema，只能在存储层内部编码，IPC/TipTap LogicalState 始终保持纯净 HTML。 */
export function prepareDocxSnapshotHtml(
  html: string,
  _baseline: number | string | null,
): string {
  return restoreMediaSrc(html);
}

/** 应用重启后按持久快照 baseline 恢复后端媒体授权，再生成本进程的显示 URL。 */
export async function restoreDocxEditMedia(
  relPath: string,
  snapshotHtml: string,
  snapshotBaseline: number | string | null = null,
): Promise<{ html: string; baseline: number | string | null }> {
  const canonical = restoreMediaSrc(snapshotHtml);
  const res = await callCommand<{ baseline: number | string; media_base: string }>(
    "doc.restore_edit_media",
    { relPath, html: canonical, baseline: snapshotBaseline },
  );
  registerDocumentMediaBase(relPath, res.media_base, res.baseline);
  return {
    html: rewriteMediaSrc(canonical, res.media_base),
    baseline: res.baseline,
  };
}

/**
 * 打开 docx 进入编辑（doc.open_for_edit）：docx → 可编辑 HTML + 加载基线。
 * media_base 存在时将相对 <img src> 改写为 asset URL（图文呈现，BR-ED-DATA-003）。
 * 失败抛 IpcError（convert_failed/依赖缺失）——调用方据此不建立编辑会话（SM-DOC-01 OpenError）。
 */
export async function openDocxForEdit(relPath: string): Promise<DocxEditModel> {
  const res = await callCommand<{
    format: string;
    html: string;
    baseline: number | string;
    media_base?: string;
    style_report: DocxStyleReport;
  }>(
    "doc.open_for_edit",
    { relPath },
  );
  if (res.media_base) registerDocumentMediaBase(relPath, res.media_base, res.baseline);
  const html = res.media_base ? rewriteMediaSrc(res.html, res.media_base) : res.html;
  return { html, baseline: res.baseline, styleReport: res.style_report };
}

/** 读取 b-docx 的独立样式报告。通常编辑器直接使用 open_for_edit 同次结果；本接口供刷新/诊断页调用。 */
export async function getDocxStyleReport(relPath: string): Promise<DocxStyleReport> {
  const res = await callCommand<{ style_report: DocxStyleReport }>("doc.get_style_report", { relPath });
  return res.style_report;
}

/**
 * 保存编辑回 docx（doc.save_from_edit）：编辑 HTML → docx → WS 原子落盘。
 * 返回新内容哈希。
 */
export async function saveDocxFromEdit(
  relPath: string,
  html: string,
  baseline: number | string | null,
  override = false,
): Promise<number | string> {
  const res = await callCommand<{ baseline: number | string; media_base?: string }>("doc.save_from_edit", {
    relPath,
    html,
    baseline,
    override,
  });
  if (res.media_base) registerDocumentMediaBase(relPath, res.media_base, res.baseline);
  return res.baseline;
}

/**
 * 创建草稿（doc.create_draft，旧版成熟流程）：原生 docx → 同目录 `{stem}.draft.docx` 草稿副本。
 * 原生 docx 点击为只读预览，"创建草稿进行编辑"后草稿直接进编辑态。返回草稿 relPath。
 */
export async function createDocxDraft(relPath: string): Promise<string> {
  const res = await callCommand<{ rel_path: string }>("doc.create_draft", { relPath });
  return res.rel_path;
}

/** 是否 docx 文件（docx/doc）。 */
function isDocxFile(relPath: string): boolean {
  const ext = relPath.toLowerCase().split(".").pop() ?? "";
  return ext === "docx" || ext === "doc";
}

function isKnownBDocx(relPath: string): boolean {
  const normalized = relPath.replace(/\\/g, "/");
  return Boolean(wsStore.getSnapshot().docs.find((doc) => doc.relPath === normalized)?.isBDocx);
}

/** 是否 BDocx（草稿/文件树新建/Agent 创建）：直接进 DOC 编辑链；原生 docx 保持预览。 */
export function isDocxEditable(relPath: string): boolean {
  return isDocxFile(relPath) && (relPath.toLowerCase().includes(".draft.") || isKnownBDocx(relPath));
}

/** 是否原生 docx（docx/doc 但非草稿）：点击走只读预览，经"创建草稿"进编辑（旧版成熟流程）。 */
export function isNativeDocxPreview(relPath: string): boolean {
  return isDocxFile(relPath) && !isDocxEditable(relPath);
}
