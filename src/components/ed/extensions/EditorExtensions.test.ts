/** @vitest-environment jsdom */
import { describe, expect, it } from "vitest";
import { Editor } from "@tiptap/core";
import StarterKit from "@tiptap/starter-kit";
import { NodeSelection } from "@tiptap/pm/state";
import { Table } from "@tiptap/extension-table";
import { TableRow } from "@tiptap/extension-table-row";
import { TableHeader } from "@tiptap/extension-table-header";
import { TableCell } from "@tiptap/extension-table-cell";
import { reconcileBlockIds } from "./BlockIdExtension";
import {
  selectDecorations,
  selectPreviewWidgets,
  type BlockRef,
  type GreenReviewValue,
} from "./DiffDecorationExtension";
import type { GreenReviewState } from "../../../store/deStore";
import { parseFontSize, renderFontSize } from "./FontSize";
import { reconcileImageIds } from "./ImageIdExtension";
import { imageCssLengthToPx, ResizableImage } from "./ResizableImage";
import {
  TableCellShading,
  normalizeTableCellShading,
  parseTableCellShading,
  renderTableCellShading,
} from "./TableCellShading";
import { MARKDOWN_RICH_FIXTURE } from "../markdown/markdownFixtures";
import {
  inspectMarkdownSource,
  normalizeMarkdownSemantic,
  parseMarkdownForEditing,
  parseMarkdownToJSON,
  serializeMarkdownDocument,
} from "../markdown/markdownCodec";

// Migrated intact from src/components/ed/extensions/BlockIdExtension.test.ts
{
// 确定性 id 生成器（注入，避免时间戳/随机）。
function seqGen() {
  let n = 0;
  return () => `n${++n}`;
}

// covers: BR-ED-DATA-001
// 稳定 blockId 守则：既有唯一 id 稳定不变；缺失补新；复制/分裂的重复 id 令后者改取新 id；新 id 不复用。
describe("BlockIdExtension · reconcileBlockIds (BR-ED-DATA-001)", () => {
  it("既有且唯一的 id 全部保持稳定（返回 null=不改）", () => {
    expect(reconcileBlockIds(["a", "b", "c"], seqGen())).toEqual([null, null, null]);
  });

  it("缺失（null）的块补发新 id", () => {
    expect(reconcileBlockIds(["a", null, "c"], seqGen())).toEqual([null, "n1", null]);
  });

  it("重复 id（复制/分裂）：首个保持稳定，后出现者改取新 id", () => {
    // 第二个 "a" 为重复 → 改新；第一个 "a" 稳定
    expect(reconcileBlockIds(["a", "a"], seqGen())).toEqual([null, "n1"]);
  });

  it("新 id 不复用既有 id、全序列唯一", () => {
    const out = reconcileBlockIds([null, null, "x"], seqGen());
    const finalIds = ["a-placeholder"]; // 占位，下方按规则核验
    void finalIds;
    // 两个新 id 互不相同，且不等于既有 "x"
    expect(out[0]).not.toBe(out[1]);
    expect(out).not.toContain("x");
    expect(out[2]).toBeNull(); // 既有 x 稳定
  });

  it("生成 id 与既有冲突时跳过重取（不复用）", () => {
    // gen 先吐 "x"（与既有冲突）应被跳过，改取下一个
    let i = 0;
    const collidingGen = () => (i++ === 0 ? "x" : "y");
    const out = reconcileBlockIds(["x", null], collidingGen);
    expect(out[0]).toBeNull(); // 既有 x 稳定
    expect(out[1]).toBe("y"); // 跳过冲突的 "x"，取 "y"
  });
});
}

// Migrated intact from src/components/ed/extensions/DiffDecorationExtension.test.ts
{
function blocks(): BlockRef[] {
  return [
    { blockId: "b1", pos: 0, size: 5 },
    { blockId: "b2", pos: 5, size: 7 },
    { blockId: null, pos: 12, size: 3 }, // 无 blockId 块
  ];
}

// covers: BR-ED-STATE-002
// GreenReview 装饰守则：命中 blockId→对应 4 态 class；空映射/未命中/无 blockId→无装饰（复位）；不替换 DOM。
describe("DiffDecorationExtension · selectDecorations (BR-ED-STATE-002)", () => {
  it("空映射 → 无装饰（复位）", () => {
    expect(selectDecorations(blocks(), new Map())).toEqual([]);
  });

  it("命中 blockId → 装饰 class 含对应 GreenReview 态", () => {
    const map = new Map<string, GreenReviewState>([["b1", "pending"]]);
    const decos = selectDecorations(blocks(), map);
    expect(decos).toHaveLength(1);
    expect(decos[0]).toMatchObject({ pos: 0, size: 5, className: "green-review green-review-pending" });
  });

  it("4 态分别映射正确 class", () => {
    const states: GreenReviewState[] = ["pending", "applying", "accepted", "expired"];
    for (const st of states) {
      const decos = selectDecorations([{ blockId: "b1", pos: 0, size: 5 }], new Map([["b1", st]]));
      expect(decos[0].className).toBe(`green-review green-review-${st}`);
    }
  });

  it("未命中映射的块（含 rejected 不入映射）→ 无装饰（复位）；无 blockId 块跳过", () => {
    // 仅 b2 在映射；b1 未命中、第三块无 blockId
    const map = new Map<string, GreenReviewState>([["b2", "accepted"]]);
    const decos = selectDecorations(blocks(), map);
    expect(decos).toHaveLength(1);
    expect(decos[0].pos).toBe(5); // 仅 b2
  });

  it("对象映射仍按 state 生成块态装饰", () => {
    const map = new Map<string, GreenReviewValue>([
      ["b1", { state: "pending", before: "旧", after: "新" }],
    ]);
    const decos = selectDecorations(blocks(), map);
    expect(decos).toHaveLength(1);
    expect(decos[0]).toMatchObject({ pos: 0, size: 5, className: "green-review green-review-pending" });
  });
});

describe("DiffDecorationExtension · selectPreviewWidgets", () => {
  it("before/after 对象映射 → 在目标块之后生成只读预览 widget 规格", () => {
    const map = new Map<string, GreenReviewValue>([
      ["b1", { state: "pending", before: "旧文本", after: "新文本" }],
    ]);
    const widgets = selectPreviewWidgets(blocks(), map);
    expect(widgets).toHaveLength(1);
    expect(widgets[0]).toMatchObject({
      pos: 5,
      state: "pending",
      before: "旧文本",
      after: "新文本",
    });
    expect(widgets[0].key).toContain("green-review-preview-b1-pending-");
  });

  it("after-only 对象映射 → 只产生绿增预览数据，不强制红删行", () => {
    const map = new Map<string, GreenReviewValue>([
      ["b1", { state: "pending", after: "新文本" }],
    ]);
    const widgets = selectPreviewWidgets(blocks(), map);
    expect(widgets).toHaveLength(1);
    expect(widgets[0]).toMatchObject({
      pos: 5,
      state: "pending",
      after: "新文本",
    });
    expect(widgets[0].before).toBeUndefined();
  });

  it("状态-only 映射与 accepted 态不生成预览 widget", () => {
    expect(selectPreviewWidgets(blocks(), new Map<string, GreenReviewValue>([["b1", "pending"]]))).toEqual([]);
    expect(selectPreviewWidgets(blocks(), new Map<string, GreenReviewValue>([
      ["b1", { state: "accepted", before: "旧", after: "新" }],
    ]))).toEqual([]);
  });
});
}

// Migrated intact from src/components/ed/extensions/FontSize.test.ts
{
describe("FontSize", () => {
  it("preserves DOCX-derived pt units instead of rewriting them as px", () => {
    expect(parseFontSize({ style: { fontSize: "12pt" } } as HTMLElement)).toBe("12pt");
    expect(renderFontSize("12pt")).toEqual({ style: "font-size: 12pt" });
  });
});
}

// Migrated intact from src/components/ed/extensions/ImageIdExtension.test.ts
{
// covers: BR-ED-DATA-001
// covers: BR-ED-DATA-003
describe("ImageIdExtension", () => {
  it("保持唯一 ID，修复缺失和重复且不复用", () => {
    const generated = ["image_new_1", "image_kept", "image_new_2"];
    expect(reconcileImageIds(
      ["image_kept", null, "image_kept", "image_other"],
      () => generated.shift() ?? "image_fallback",
    )).toEqual([
      null,
      "image_new_1",
      "image_new_2",
      null,
    ]);
  });
});
}

// Migrated intact from src/components/ed/extensions/ResizableImage.test.ts
{
// covers: BR-ED-DATA-003
// covers: BR-ED-STATE-001
describe("ResizableImage · BDocx 图片属性恢复", () => {
  it("把 Pandoc 物理长度按 96dpi 规范为 px", () => {
    expect(imageCssLengthToPx("1.25in")).toBe("120");
    expect(imageCssLengthToPx("72pt")).toBe("96");
    expect(imageCssLengthToPx("2.54cm")).toBe("96");
    expect(imageCssLengthToPx("120px")).toBe("120");
    expect(imageCssLengthToPx("auto")).toBeNull();
  });

  it("真实 TipTap schema 保留稳定媒体引用并恢复 CSS 宽高", () => {
    const editor = new Editor({
      extensions: [
        StarterKit,
        ResizableImage.configure({ inline: true, allowBase64: true }),
      ],
      content:
        '<p><img src="asset://localhost/cache/media/image1.png" data-binder-media-src="media/image1.png" data-binder-project-src="images/cover.png" data-binder-project-hash="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" style="width:1.25in;height:0.75in"></p>',
    });
    const image = editor.state.doc.firstChild?.firstChild;
    expect(image?.type.name).toBe("image");
    expect(image?.attrs).toMatchObject({
      width: "120",
      height: "72",
      binderMediaSrc: "media/image1.png",
      binderProjectSrc: "images/cover.png",
      binderProjectHash: "a".repeat(64),
    });
    const html = editor.getHTML();
    expect(html).toContain('width="120"');
    expect(html).toContain('height="72"');
    expect(html).toContain('data-binder-media-src="media/image1.png"');
    expect(html).toContain('data-binder-project-src="images/cover.png"');
    expect(html).toContain(`data-binder-project-hash="${"a".repeat(64)}"`);
    editor.destroy();
  });

  it("document 捕获鼠标事件在缩放下实时预览，并于 mouseup 提交一次 width 事务", () => {
    const element = document.createElement("div");
    document.body.append(element);
    const editor = new Editor({
      element,
      extensions: [
        StarterKit,
        ResizableImage.configure({ inline: true, allowBase64: true }),
      ],
      content:
        '<p><img src="data:image/png;base64,AA==" width="100" height="50" data-binder-media-src="media/image1.png"></p>',
    });
    editor.view.dom.style.zoom = "1.25";
    let imagePos = -1;
    editor.state.doc.descendants((node, pos) => {
      if (node.type.name === "image") imagePos = pos;
    });
    editor.view.dispatch(editor.state.tr.setSelection(NodeSelection.create(editor.state.doc, imagePos)));
    const image = element.querySelector(".ed-img-view img") as HTMLImageElement;
    const imageView = element.querySelector(".ed-img-view") as HTMLElement;
    const handle = element.querySelector(".ed-img-handle") as HTMLElement;
    const parent = imageView.parentElement as HTMLElement;
    Object.defineProperty(image, "getBoundingClientRect", {
      value: () => ({ width: 125, height: 62.5, left: 0, right: 125, top: 0, bottom: 62.5, x: 0, y: 0, toJSON() {} }),
    });
    Object.defineProperty(parent, "getBoundingClientRect", {
      value: () => ({ width: 500, height: 100, left: 0, right: 500, top: 0, bottom: 100, x: 0, y: 0, toJSON() {} }),
    });
    let changedTransactions = 0;
    editor.on("transaction", ({ transaction }) => {
      if (transaction.docChanged) changedTransactions += 1;
    });

    handle.dispatchEvent(new MouseEvent("mousedown", {
      bubbles: true,
      cancelable: true,
      button: 0,
      clientX: 100,
    }));
    document.dispatchEvent(new MouseEvent("mousemove", {
      bubbles: true,
      cancelable: true,
      clientX: 150,
    }));
    expect(image.style.width).toBe("140px");
    expect(editor.getAttributes("image").width).toBe("100");

    document.dispatchEvent(new MouseEvent("mouseup", {
      bubbles: true,
      cancelable: true,
      clientX: 150,
    }));
    expect(editor.state.doc.nodeAt(imagePos)?.attrs).toMatchObject({ width: "140", height: null });
    expect(changedTransactions).toBe(1);
    expect(editor.getHTML()).toContain('width="140"');
    expect(editor.getHTML()).not.toContain('height="50"');
    editor.destroy();
    element.remove();
  });

  it("只按下并松开缩放手柄不会制造空事务或清除来源高度", () => {
    const element = document.createElement("div");
    document.body.append(element);
    const editor = new Editor({
      element,
      extensions: [
        StarterKit,
        ResizableImage.configure({ inline: true, allowBase64: true }),
      ],
      content: '<p><img src="data:image/png;base64,AA==" width="100" height="50"></p>',
    });
    const image = element.querySelector(".ed-img-view img") as HTMLImageElement;
    const handle = element.querySelector(".ed-img-handle") as HTMLElement;
    Object.defineProperty(image, "getBoundingClientRect", {
      value: () => ({ width: 100, height: 50, left: 0, right: 100, top: 0, bottom: 50, x: 0, y: 0, toJSON() {} }),
    });
    let changedTransactions = 0;
    editor.on("transaction", ({ transaction }) => {
      if (transaction.docChanged) changedTransactions += 1;
    });

    handle.dispatchEvent(new MouseEvent("mousedown", {
      bubbles: true,
      cancelable: true,
      button: 0,
      clientX: 100,
    }));
    document.dispatchEvent(new MouseEvent("mouseup", {
      bubbles: true,
      cancelable: true,
      clientX: 100,
    }));

    expect(changedTransactions).toBe(0);
    expect(editor.getHTML()).toContain('width="100"');
    expect(editor.getHTML()).toContain('height="50"');
    editor.destroy();
    element.remove();
  });
});
}

// Migrated intact from src/components/ed/extensions/TableCellShading.test.ts
{
// covers: BR-ED-DATA-007
describe("TableCellShading", () => {
  it("只接受不透明六位十六进制或等价 rgb，并统一为大写", () => {
    expect(normalizeTableCellShading("#ffe699")).toBe("#FFE699");
    expect(normalizeTableCellShading("rgb(255, 230, 153)")).toBe("#FFE699");
    expect(normalizeTableCellShading("rgba(255, 230, 153, 1)")).toBe("#FFE699");
    expect(normalizeTableCellShading("#abc")).toBeNull();
    expect(normalizeTableCellShading("rgba(255, 230, 153, 0.5)")).toBeNull();
    expect(normalizeTableCellShading("transparent")).toBeNull();
  });

  it("优先读取稳定 data 属性并同时渲染 data 与 CSS", () => {
    const cell = document.createElement("td");
    cell.setAttribute("data-binder-cell-shading", "#ff0000");
    cell.style.backgroundColor = "rgb(0, 0, 255)";
    expect(parseTableCellShading(cell)).toBe("#FF0000");
    expect(renderTableCellShading("#ffe699")).toEqual({
      "data-binder-cell-shading": "#FFE699",
      style: "background-color: #FFE699",
    });
    expect(renderTableCellShading(null)).toEqual({});
  });

  it("在真实 TipTap schema 中往返 td/th 属性", () => {
    const editor = new Editor({
      extensions: [StarterKit, Table, TableRow, TableHeader, TableCell, TableCellShading],
      content: '<table><tbody><tr><td data-binder-cell-shading="#ffe699"><p>内容</p></td></tr></tbody></table>',
    });
    const cell = editor.state.doc.firstChild?.firstChild?.firstChild;
    expect(cell?.type.name).toBe("tableCell");
    expect(cell?.attrs.backgroundColor).toBe("#FFE699");
    expect(editor.getHTML()).toContain('data-binder-cell-shading="#FFE699"');
    expect(editor.getHTML()).toContain("background-color:");
    editor.destroy();
  });
});
}

// Migrated intact from src/components/ed/markdown/markdownCodec.test.ts
{
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
}
