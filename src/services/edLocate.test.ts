import { beforeEach, describe, it, expect } from "vitest";
import { Schema } from "@tiptap/pm/model";
import {
  DOC_REF_MIME,
  SELECTION_REF_MIME,
  bindActiveEditorDocument,
  blockTextLength,
  charOffsetInBlock,
  getActiveEditorBinding,
  findTopBlockSnapshotById,
  getTopBlockIdByIndex,
  getTopBlockSnapshotByIndex,
  isActiveEditorBoundTo,
  readDocRefData,
  readSelectionRefData,
  registerEditor,
  resolveSelectionAnchor,
  writeDocRefData,
  writeSelectionRefData,
} from "./edLocate";
import { REFERENCE_MIME } from "./referencePayload";

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

beforeEach(() => registerEditor(null));

/** 构造段落（可含 hardBreak 叶子）。 */
function p(blockId: string | null, ...parts: (string | "BR")[]) {
  const content = parts.map((x) => (x === "BR" ? schema.node("hardBreak") : schema.text(x)));
  return schema.node("paragraph", { blockId }, content);
}

function transfer() {
  const values = new Map<string, string>();
  return {
    values,
    getData(type: string) {
      return values.get(type) ?? "";
    },
    setData(type: string, data: string) {
      values.set(type, data);
    },
  };
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

  it("按 block_index / blockId 返回同一目标块快照，before 文本来自该块", () => {
    const byIndex = getTopBlockSnapshotByIndex(doc, 1);
    expect(byIndex).toMatchObject({ blockId: "b2", index: 1, text: "第二段" });
    expect(byIndex?.contentFrom).toBeGreaterThan(byIndex?.pos ?? -1);
    expect(byIndex?.contentTo).toBeGreaterThan(byIndex?.contentFrom ?? -1);

    expect(findTopBlockSnapshotById(doc, "b2")).toEqual(byIndex);
    expect(getTopBlockSnapshotByIndex(doc, 3)).toBeNull();
    expect(findTopBlockSnapshotById(doc, "missing")).toBeNull();
  });
});

describe("edLocate · active editor binding", () => {
  it("记录并校验 ProseMirror 当前承载的文档身份", () => {
    registerEditor(null);
    bindActiveEditorDocument({ relPath: "notes/a.md", loadSeq: 3, editFormat: "text" });

    expect(getActiveEditorBinding()).toEqual({ relPath: "notes/a.md", loadSeq: 3, editFormat: "text" });
    expect(isActiveEditorBoundTo("notes/a.md", 3, "text")).toBe(true);
    expect(isActiveEditorBoundTo("notes/b.md", 3, "text")).toBe(false);
    expect(isActiveEditorBoundTo("notes/a.md", 4, "text")).toBe(false);
    expect(isActiveEditorBoundTo("notes/a.md", 3, "html")).toBe(false);
  });

  it("registerEditor(null) 清空旧绑定，避免卸载后继续信任旧正文", () => {
    bindActiveEditorDocument({ relPath: "notes/a.md", loadSeq: 1, editFormat: "text" });
    registerEditor(null);

    expect(getActiveEditorBinding()).toBeNull();
    expect(isActiveEditorBoundTo("notes/a.md", 1, "text")).toBe(false);
  });
});

describe("edLocate · 引用拖拽/粘贴数据契约", () => {
  it("写入并读取文档引用 MIME，同时提供 text/plain 兜底文本", () => {
    const dt = transfer();

    writeDocRefData(dt, "notes/a.md");

    expect(dt.values.get(DOC_REF_MIME)).toBe(JSON.stringify({ relPath: "notes/a.md" }));
    expect(JSON.parse(dt.values.get(REFERENCE_MIME) ?? "{}")).toEqual({ kind: "doc", relPath: "notes/a.md" });
    expect(dt.values.get("text/plain")).toBe("notes/a.md");
    expect(readDocRefData(dt)).toEqual({ relPath: "notes/a.md" });
  });

  it("忽略空路径或损坏的文档引用 payload", () => {
    const dt = transfer();
    dt.setData(DOC_REF_MIME, JSON.stringify({ relPath: "" }));
    expect(readDocRefData(dt)).toBeNull();

    dt.setData(DOC_REF_MIME, "{");
    expect(readDocRefData(dt)).toBeNull();
  });

  it("写入并读取 SelectionRef MIME，同时提供选区文本兜底", () => {
    const dt = transfer();
    const ref = {
      path: "notes/a.md",
      blockId: "block-1",
      start: 2,
      end: 5,
      text: "选中的内容",
    };

    writeSelectionRefData(dt, ref);

    expect(dt.values.get(SELECTION_REF_MIME)).toBe(JSON.stringify(ref));
    expect(JSON.parse(dt.values.get(REFERENCE_MIME) ?? "{}")).toEqual({
      kind: "selection",
      relPath: "notes/a.md",
      blockId: "block-1",
      start: 2,
      end: 5,
      text: "选中的内容",
    });
    expect(dt.values.get("text/plain")).toBe("选中的内容");
    expect(readSelectionRefData(dt)).toEqual(ref);
  });

  it("忽略字段不完整或损坏的 SelectionRef payload", () => {
    const dt = transfer();
    dt.setData(SELECTION_REF_MIME, JSON.stringify({ path: "notes/a.md", blockId: "b1" }));
    expect(readSelectionRefData(dt)).toBeNull();

    dt.setData(SELECTION_REF_MIME, "{");
    expect(readSelectionRefData(dt)).toBeNull();
  });
});
