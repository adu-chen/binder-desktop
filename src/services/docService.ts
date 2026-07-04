import { invoke, convertFileSrc } from "@tauri-apps/api/core";

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
  baseline: number;
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
 *  converter 依赖注入（默认 convertFileSrc），便于单测不依赖 tauri 运行时。 */
export function rewriteMediaSrc(
  html: string,
  mediaBase: string,
  converter: (path: string) => string = convertFileSrc,
): string {
  const base = mediaBase.replace(/[/\\]+$/, "");
  return html.replace(
    /(<img\b[^>]*?\bsrc=)(["'])([^"']+)\2/gi,
    (match, pre: string, quote: string, src: string) => {
      // 绝对 URL / 内联 data / 已处理 asset·blob·file / 协议相对 → 不改。
      if (/^(https?:|data:|asset:|blob:|file:|\/\/)/i.test(src)) return match;
      const abs = `${base}/${src.replace(/^\.?[/\\]/, "")}`;
      return `${pre}${quote}${converter(abs)}${quote}`;
    },
  );
}

/**
 * 打开 docx 进入编辑（doc.open_for_edit）：docx → 可编辑 HTML + 加载基线。
 * media_base 存在时将相对 <img src> 改写为 asset URL（图文呈现，BR-ED-DATA-003）。
 * 失败抛 IpcError（convert_failed/依赖缺失）——调用方据此不建立编辑会话（SM-DOC-01 OpenError）。
 */
export async function openDocxForEdit(relPath: string): Promise<DocxEditModel> {
  const res = await callCommand<{ format: string; html: string; baseline: number; media_base?: string }>(
    "doc.open_for_edit",
    { relPath },
  );
  const html = res.media_base ? rewriteMediaSrc(res.html, res.media_base) : res.html;
  return { html, baseline: res.baseline };
}

/**
 * 保存编辑回 docx（doc.save_from_edit）：编辑 HTML → docx（--reference-doc 统一样式）→ WS 原子落盘。
 * baseline = 打开时的加载基线；磁盘≠基线且未 override → WS_SAVE_CONFLICT（SM-WS-01，原文件不变）。
 * 返回新基线（保存后 docx 字节哈希）。
 */
export async function saveDocxFromEdit(
  relPath: string,
  html: string,
  baseline: number | null,
  override = false,
): Promise<number> {
  const res = await callCommand<{ baseline: number }>("doc.save_from_edit", {
    relPath,
    html,
    baseline,
    override,
  });
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

/** 是否草稿/可直接编辑的 docx（`*.draft.docx`）：草稿与项目生成的 docx 直接进编辑态。 */
export function isDocxEditable(relPath: string): boolean {
  return isDocxFile(relPath) && relPath.toLowerCase().includes(".draft.");
}

/** 是否原生 docx（docx/doc 但非草稿）：点击走只读预览，经"创建草稿"进编辑（旧版成熟流程）。 */
export function isNativeDocxPreview(relPath: string): boolean {
  return isDocxFile(relPath) && !relPath.toLowerCase().includes(".draft.");
}
