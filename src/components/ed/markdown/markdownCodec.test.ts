// @vitest-environment jsdom
import { describe, expect, it } from "vitest";
import { MARKDOWN_RICH_FIXTURE } from "./markdownFixtures";
import {
  inspectMarkdownSource,
  normalizeMarkdownSemantic,
  parseMarkdownForEditing,
  parseMarkdownToJSON,
  serializeMarkdownDocument,
} from "./markdownCodec";

// covers: BR-ED-DATA-006
// covers: BR-ED-VERIFY-001
describe("Markdown Codec", () => {
  it("把 CommonMark/GFM 结构解析为终态 ProseMirror 节点", () => {
    const parsed = parseMarkdownForEditing(MARKDOWN_RICH_FIXTURE);
    expect(parsed.ok).toBe(true);
    if (!parsed.ok) return;
    expect(parsed.contentFormat).toBe("html");
    expect(parsed.html).toContain("<h1");
    expect(parsed.html).toContain("<h2");
    expect(parsed.html).toContain("<blockquote");
    expect(parsed.html).toContain("<table");
    expect(JSON.stringify(parsed.json)).toContain('"type":"taskList"');
  });

  it("支持子集序列化后语义等价", () => {
    const original = parseMarkdownToJSON(MARKDOWN_RICH_FIXTURE);
    const serialized = serializeMarkdownDocument(original);
    expect(serialized.ok).toBe(true);
    if (!serialized.ok) return;
    const reparsed = parseMarkdownToJSON(serialized.markdown);
    expect(normalizeMarkdownSemantic(reparsed)).toEqual(normalizeMarkdownSemantic(original));
  });

  it("保留 YAML front matter 并支持往返", () => {
    const source = "---\ntitle: x\n---\n\n正文";
    expect(inspectMarkdownSource(source)).toEqual([]);

    const parsed = parseMarkdownForEditing(source);
    expect(parsed).toMatchObject({
      ok: true,
      sourceProtected: false,
      json: { attrs: { frontMatter: "title: x" } },
    });
    if (!parsed.ok) return;

    expect(serializeMarkdownDocument(parsed.json)).toMatchObject({
      ok: true,
      markdown: expect.stringContaining("---\ntitle: x\n---"),
    });
  });

  it("HTML 注释进入源码保护，不构造有损富文本", () => {
    const source = "---\ntitle: x\n---\n\n正文\n\n<!-- keep -->";
    expect(inspectMarkdownSource(source)).toEqual(["comment"]);
    expect(parseMarkdownForEditing(source)).toMatchObject({ ok: false, sourceProtected: true, contentFormat: "text", source });
  });

  it("拒绝 Markdown 不能表达的颜色样式", () => {
    const result = serializeMarkdownDocument({
      type: "doc",
      content: [{
        type: "paragraph",
        content: [{ type: "text", text: "彩色", marks: [{ type: "textStyle", attrs: { color: "#f00" } }] }],
      }],
    });
    expect(result).toMatchObject({ ok: false, code: "ED_MARKDOWN_UNSUPPORTED_MODEL" });
  });
});
