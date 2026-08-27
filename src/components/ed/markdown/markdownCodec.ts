import { generateHTML, type Extensions, type JSONContent } from "@tiptap/core";
import { MarkdownManager } from "@tiptap/markdown";
import { buildEditorExtensions } from "../extensions";

export type MarkdownSupportReason =
  | "front_matter"
  | "comment"
  | "footnote"
  | "math"
  | "admonition"
  | "embedded_html";

export type MarkdownParseResult =
  | { ok: true; json: JSONContent; html: string; contentFormat: "html"; sourceProtected: false }
  | { ok: false; source: string; contentFormat: "text"; sourceProtected: true; reasons: MarkdownSupportReason[] };

export type MarkdownSerializeResult =
  | { ok: true; markdown: string }
  | { ok: false; code: "ED_MARKDOWN_UNSUPPORTED_MODEL" | "ED_MARKDOWN_ROUNDTRIP_FAILED" | "ED_MARKDOWN_SERIALIZE_FAILED"; message: string };

const SUPPORTED_NODES = new Set([
  "doc",
  "paragraph",
  "text",
  "heading",
  "bulletList",
  "orderedList",
  "listItem",
  "taskList",
  "taskItem",
  "blockquote",
  "codeBlock",
  "hardBreak",
  "horizontalRule",
  "image",
  "table",
  "tableRow",
  "tableHeader",
  "tableCell",
]);

const SUPPORTED_MARKS = new Set(["bold", "italic", "strike", "code", "link"]);

let extensions: Extensions | null = null;
let manager: MarkdownManager | null = null;

function codecExtensions(): Extensions {
  if (!extensions) extensions = buildEditorExtensions();
  return extensions;
}

function markdownManager(): MarkdownManager {
  if (!manager) {
    manager = new MarkdownManager({
      extensions: codecExtensions(),
      indentation: { style: "space", size: 2 },
      markedOptions: { gfm: true },
    });
  }
  return manager;
}

export function inspectMarkdownSource(markdown: string): MarkdownSupportReason[] {
  const reasons = new Set<MarkdownSupportReason>();
  // Front matter 已支持：提取后只读保留，不拒绝
  if (/<!--[\s\S]*?-->/.test(markdown)) reasons.add("comment");
  if (/^\[\^[^\]]+\]:|\[\^[^\]]+\]/m.test(markdown)) reasons.add("footnote");
  // 数学公式已支持：`$$` 块公式和 `$` 行内公式作为文本保留，不拒绝
  if (/^\s*:::/m.test(markdown)) reasons.add("admonition");
  if (/<\/?[A-Za-z][A-Za-z0-9-]*(?:\s[^>]*)?>/.test(markdown)) reasons.add("embedded_html");
  return [...reasons];
}

/** 提取 YAML front matter（`---\n...\n---`），返回 [frontMatter, body]。无则返回 ["", markdown]。 */
function extractFrontMatter(markdown: string): [string, string] {
  const match = markdown.match(/^---\r?\n([\s\S]*?)\r?\n---(?:\r?\n|$)/);
  if (!match) return ["", markdown];
  return [match[1], markdown.slice(match[0].length)];
}

/**
 * @GOV
 * codes:    BR-ED-DATA-006-DATA-ED-ED-EDIT-014
 * type:     DATA
 * chain:    ED-EDIT
 * rules:    BR-ED-DATA-006
 * boundary: in=MarkdownDiskState | out=ProseMirrorLogicalStateOrSourceProtectedText(contentFormatExplicit)
 */
/// Markdown Codec 解析守则：受支持 CommonMark/GFM 进入结构化 LogicalState；非支持结构保留源码，
/// 不生成会在保存时丢内容的部分富文本模型。
export function parseMarkdownForEditing(markdown: string): MarkdownParseResult {
  const reasons = inspectMarkdownSource(markdown);
  if (reasons.length) {
    return { ok: false, source: markdown, contentFormat: "text", sourceProtected: true, reasons };
  }
  try {
    const [frontMatter, body] = extractFrontMatter(markdown);
    const json = markdownManager().parse(body);
    if (frontMatter) {
      json.attrs = { ...(json.attrs ?? {}), frontMatter };
    }
    const html = generateHTML(json, codecExtensions());
    return { ok: true, json, html, contentFormat: "html", sourceProtected: false };
  } catch {
    return { ok: false, source: markdown, contentFormat: "text", sourceProtected: true, reasons: [] };
  }
}

function unsupportedModelReason(node: JSONContent): string | null {
  const type = node.type ?? "";
  if (!SUPPORTED_NODES.has(type)) return `不支持的 Markdown 节点: ${type || "unknown"}`;
  for (const mark of node.marks ?? []) {
    const markType = mark.type ?? "";
    if (!SUPPORTED_MARKS.has(markType)) return `Markdown 无法无损表达样式: ${markType || "unknown"}`;
  }
  if ((type === "paragraph" || type === "heading") && node.attrs?.textAlign && node.attrs.textAlign !== "left") {
    return "Markdown 无法无损表达文本对齐";
  }
  if (type === "image" && (node.attrs?.width || (node.attrs?.display && node.attrs.display !== "inline"))) {
    return "Markdown 无法无损表达图片宽度或块级显示属性";
  }
  for (const child of node.content ?? []) {
    const reason = unsupportedModelReason(child);
    if (reason) return reason;
  }
  return null;
}

function semanticAttrs(node: JSONContent): Record<string, unknown> | undefined {
  switch (node.type) {
    case "heading":
      return { level: node.attrs?.level ?? 1 };
    case "orderedList":
      return { start: node.attrs?.start ?? 1 };
    case "taskItem":
      return { checked: Boolean(node.attrs?.checked) };
    case "codeBlock":
      return node.attrs?.language ? { language: node.attrs.language } : undefined;
    case "image":
      return { src: node.attrs?.src ?? "", alt: node.attrs?.alt ?? null, title: node.attrs?.title ?? null };
    default:
      return undefined;
  }
}

function normalizeMark(mark: JSONContent): { type: string; attrs?: Record<string, unknown> } {
  if (mark.type === "link") {
    return { type: "link", attrs: { href: mark.attrs?.href ?? "", title: mark.attrs?.title ?? null } };
  }
  return { type: mark.type ?? "unknown" };
}

export function normalizeMarkdownSemantic(node: JSONContent): JSONContent {
  const normalized: JSONContent = { type: node.type };
  if (node.text !== undefined) normalized.text = node.text;
  const attrs = semanticAttrs(node);
  if (attrs) normalized.attrs = attrs;
  // 保留 frontMatter 但不参与语义等价比较（它是只读保留字段，不随编辑内容变化）
  if (node.type === "doc" && node.attrs?.frontMatter) {
    normalized.attrs = { ...(normalized.attrs ?? {}), frontMatter: node.attrs.frontMatter };
  }
  if (node.marks?.length) {
    normalized.marks = node.marks.map(normalizeMark).sort((a, b) => String(a.type).localeCompare(String(b.type)));
  }
  if (node.content?.length) normalized.content = node.content.map((child) => normalizeMarkdownSemantic(child));
  return normalized;
}

/**
 * @GOV
 * codes:    BR-ED-VERIFY-001-GUARD-ED-ED-EDIT-015, BR-ED-DATA-005-GUARD-ED-ED-EDIT-015
 * type:     GUARD
 * chain:    ED-EDIT
 * rules:    BR-ED-VERIFY-001, BR-ED-DATA-005
 * boundary: in=ProseMirrorLogicalState | out=MarkdownDiskCandidateOrSerializeFailure(noSilentLoss)
 */
/// 保存前能力与语义守卫：拒绝目标方言不能表达的节点/样式，并用 parse→serialize→parse 语义等价
/// 校验阻断静默丢失；失败不返回可写盘候选。
export function serializeMarkdownDocument(json: JSONContent): MarkdownSerializeResult {
  const unsupported = unsupportedModelReason(json);
  if (unsupported) {
    return { ok: false, code: "ED_MARKDOWN_UNSUPPORTED_MODEL", message: unsupported };
  }
  // 序列化前提取 frontMatter 并临时移除，序列化后再前置写入
  const frontMatter = (json.attrs?.frontMatter as string | undefined) ?? "";
  const serializeJson = frontMatter ? { ...json, attrs: { ...(json.attrs as Record<string, unknown> ?? {}), frontMatter: undefined } } : json;
  try {
    const markdown = markdownManager().serialize(serializeJson);
    const reparsed = markdownManager().parse(markdown);
    const normalized = normalizeMarkdownSemantic(serializeJson);
    if (JSON.stringify(normalized) !== JSON.stringify(normalizeMarkdownSemantic(reparsed))) {
      return { ok: false, code: "ED_MARKDOWN_ROUNDTRIP_FAILED", message: "Markdown 语义往返校验失败，已阻止写盘" };
    }
    const output = frontMatter ? `---\n${frontMatter}\n---\n\n${markdown}` : markdown;
    return { ok: true, markdown: output };
  } catch (error) {
    return {
      ok: false,
      code: "ED_MARKDOWN_SERIALIZE_FAILED",
      message: error instanceof Error ? error.message : "Markdown 序列化失败",
    };
  }
}

export function parseMarkdownToJSON(markdown: string): JSONContent {
  return markdownManager().parse(markdown);
}
