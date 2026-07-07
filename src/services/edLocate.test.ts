import { describe, it, expect } from "vitest";
import { Schema } from "@tiptap/pm/model";
import { charOffsetInBlock, blockTextLength, getTopBlockIdByIndex, resolveSelectionAnchor } from "./edLocate";

// covers: BR-ED-DATA-004
// 字符锚纯逻辑：偏移定义在块内纯文本平面（textBetween 语义，叶子内联 1 字符占位对齐 PM pos）；
// 单块选区精确区间；跨块选区锚定首块（截首块文本尾）；折叠/无 blockId → null。
// 用 prosemirror-model 最小 schema 构造真实 doc（不依赖 TipTap 运行时/jsdom）。

const schema = new Schema({
  nodes: {
    doc: { content: "block+" },
    paragraph: {
      group: "block",
      content: "inline*",
      attrs: { blockId: { default: null } },
    },
    hardBreak: { group: "inline", inline: true }, // 叶子内联（nodeSize=1）
    text: { group: "inline" },
  },
});

/** 构造段落（可含 hardBreak 叶子）。 */
function p(blockId: string | null, ...parts: (string | "BR")[]) {
  const content = parts.map((x) => (x === "BR" ? schema.node("hardBreak") : schema.text(x)));
  return schema.node("paragraph", { blockId }, content);
}

describe("edLocate · 块内纯文本偏移（BR-ED-DATA-004 偏移平面）", () => {
  it("纯文本块：偏移 = 文本位置（含中文）", () => {
    const block = p("b1", "你好 world");
    expect(blockTextLength(block)).toBe(8); // 你好+空格+world
    expect(charOffsetInBlock(block, 0)).toBe(0);
    expect(charOffsetInBlock(block, 2)).toBe(2); // "你好" 后
    expect(charOffsetInBlock(block, 8)).toBe(8);
  });

  it("叶子内联节点（hardBreak）以 1 字符占位对齐 pos", () => {
    const block = p("b1", "ab", "BR", "cd"); // pos: a(0)b(1) BR(2) c(3)d(4)
    expect(blockTextLength(block)).toBe(5); // "ab￼cd"
    expect(charOffsetInBlock(block, 3)).toBe(3); // BR 之后 = 偏移 3
  });

  it("越界钳制（不抛错，安全上界）", () => {
    const block = p("b1", "abc");
    expect(charOffsetInBlock(block, 99)).toBe(3);
    expect(charOffsetInBlock(block, -5)).toBe(0);
  });
});

describe("edLocate · resolveSelectionAnchor", () => {
  // doc: [p1 "第一段落"(4字)] [p2 "第二段"(3字)]；p1 内容 pos 1..5，p2 内容 pos 7..10
  const doc = schema.node("doc", null, [p("b1", "第一段落"), p("b2", "第二段")]);

  it("单块选区 → 精确区间", () => {
    // 选 "一段"（p1 内偏移 1..3）：doc pos 2..4
    expect(resolveSelectionAnchor(doc, 2, 4)).toEqual({ blockId: "b1", start: 1, end: 3 });
  });

  it("整块选区 → 0..文本长", () => {
    expect(resolveSelectionAnchor(doc, 1, 5)).toEqual({ blockId: "b1", start: 0, end: 4 });
  });

  it("跨块选区 → 锚定首块并截到首块文本尾", () => {
    // 从 p1 偏移 2 跨到 p2 中部
    expect(resolveSelectionAnchor(doc, 3, 9)).toEqual({ blockId: "b1", start: 2, end: 4 });
  });

  it("折叠选区 → null；无 blockId 块 → null", () => {
    expect(resolveSelectionAnchor(doc, 3, 3)).toBeNull();
    const noId = schema.node("doc", null, [p(null, "无锚段")]);
    expect(resolveSelectionAnchor(noId, 1, 3)).toBeNull();
  });

  it("第二块内选区 → 正确归属 b2 且偏移按块内计", () => {
    // p2 内容 pos 从 7 开始；选 "二段"（p2 偏移 1..3）→ doc pos 8..10
    expect(resolveSelectionAnchor(doc, 8, 10)).toEqual({ blockId: "b2", start: 1, end: 3 });
  });
});

describe("edLocate · block_index → blockId", () => {
  const doc = schema.node("doc", null, [p("b1", "第一段落"), p("b2", "第二段"), p(null, "无锚段")]);

  it("按 0 起块号返回对应稳定 blockId", () => {
    expect(getTopBlockIdByIndex(doc, 0)).toBe("b1");
    expect(getTopBlockIdByIndex(doc, 1)).toBe("b2");
  });

  it("越界、负数、无 blockId 块不产生错位锚", () => {
    expect(getTopBlockIdByIndex(doc, -1)).toBeNull();
    expect(getTopBlockIdByIndex(doc, 3)).toBeNull();
    expect(getTopBlockIdByIndex(doc, 2)).toBeNull();
  });
});
