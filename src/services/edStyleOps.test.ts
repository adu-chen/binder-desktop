import { describe, it, expect } from "vitest";
import { Schema } from "@tiptap/pm/model";
import {
  extractBlockStyles,
  projectDocStyles,
  parseStyleOps,
  posFromCharOffset,
  planStyleOps,
  type StyleOp,
} from "./edStyleOps";

// covers: BR-ED-DATA-005
// 样式协议纯逻辑：读侧投影（marks range 在块内纯文本平面、相邻同型合并、无 blockId 块跳过）；
// 写侧规划（锚校验批内原子——任一失效整批拒绝；字符锚 → PM 位置换算精确）。
// 最小 prosemirror schema（含 marks），不依赖 TipTap 运行时。

const schema = new Schema({
  nodes: {
    doc: { content: "block+" },
    paragraph: {
      group: "block",
      content: "inline*",
      attrs: { blockId: { default: null }, textAlign: { default: null } },
    },
    text: { group: "inline" },
  },
  marks: {
    bold: {},
    textStyle: { attrs: { color: { default: null } } },
  },
});

const bold = schema.marks.bold.create();
const red = schema.marks.textStyle.create({ color: "#ff0000" });

function para(blockId: string | null, parts: { t: string; marks?: import("@tiptap/pm/model").Mark[] }[]) {
  return schema.node(
    "paragraph",
    { blockId },
    parts.map((p) => schema.text(p.t, p.marks)),
  );
}

describe("edStyleOps · 读侧投影（BR-ED-DATA-005）", () => {
  it("mark 段 range 在块内纯文本平面 + attrs 透出", () => {
    const block = para("b1", [{ t: "普通" }, { t: "加粗", marks: [bold] }, { t: "红字", marks: [red] }]);
    const { text, marks } = extractBlockStyles(block);
    expect(text).toBe("普通加粗红字");
    expect(marks).toEqual([
      { type: "bold", range: [2, 4], attrs: undefined },
      { type: "textStyle", range: [4, 6], attrs: { color: "#ff0000" } },
    ]);
  });

  it("相邻同型同 attrs 段合并", () => {
    const block = para("b1", [{ t: "ab", marks: [bold] }, { t: "cd", marks: [bold] }]);
    expect(extractBlockStyles(block).marks).toEqual([{ type: "bold", range: [0, 4], attrs: undefined }]);
  });

  it("文档投影：无 blockId 块跳过；blockAttrs 剔除 blockId 内部属性", () => {
    const doc = schema.node("doc", null, [
      schema.node("paragraph", { blockId: "b1", textAlign: "center" }, [schema.text("居中")]),
      para(null, [{ t: "无锚" }]),
    ]);
    const views = projectDocStyles(doc);
    expect(views).toHaveLength(1);
    expect(views[0]).toMatchObject({ blockId: "b1", type: "paragraph", text: "居中", blockAttrs: { textAlign: "center" } });
    expect(views[0].blockAttrs.blockId).toBeUndefined();
  });
});

describe("edStyleOps · posFromCharOffset", () => {
  it("多段文本中偏移 → 精确内容 pos", () => {
    const block = para("b1", [{ t: "ab" }, { t: "cde", marks: [bold] }]);
    expect(posFromCharOffset(block, 0)).toBe(0);
    expect(posFromCharOffset(block, 2)).toBe(2);
    expect(posFromCharOffset(block, 4)).toBe(4); // "ab"+"cd" 之后
    expect(posFromCharOffset(block, 5)).toBe(5);
  });
});

describe("edStyleOps · planStyleOps（批内原子）", () => {
  const doc = schema.node("doc", null, [para("b1", [{ t: "第一段落" }]), para("b2", [{ t: "第二段" }])]);

  it("合法批 → 事务步骤（字符锚换算 PM 位置）", () => {
    const ops: StyleOp[] = [
      { op: "set_mark", blockId: "b1", start: 1, end: 3, mark: { type: "bold" } },
      { op: "set_block_attr", blockId: "b2", attrs: { textAlign: "right" } },
    ];
    const r = planStyleOps(doc, ops);
    expect(r.ok).toBe(true);
    if (r.ok) {
      // b1 内容起点 pos=1 → from=1+1=2, to=1+3=4
      expect(r.steps[0]).toEqual({ kind: "addMark", from: 2, to: 4, markType: "bold", attrs: undefined });
      expect(r.steps[1]).toMatchObject({ kind: "setBlockAttrs", blockPos: 6 });
    }
  });

  it("任一锚失效 → 整批拒绝（块不存在 / 偏移越界 / 空区间）", () => {
    const good: StyleOp = { op: "set_mark", blockId: "b1", start: 0, end: 2, mark: { type: "bold" } };
    expect(planStyleOps(doc, [good, { op: "set_mark", blockId: "不存在", start: 0, end: 1, mark: { type: "bold" } }]).ok).toBe(false);
    expect(planStyleOps(doc, [good, { op: "unset_mark", blockId: "b2", start: 0, end: 99, markType: "bold" }]).ok).toBe(false);
    expect(planStyleOps(doc, [good, { op: "set_mark", blockId: "b2", start: 1, end: 1, mark: { type: "bold" } }]).ok).toBe(false);
  });

  it("空批 → ok 空步骤", () => {
    expect(planStyleOps(doc, [])).toEqual({ ok: true, steps: [] });
  });

  it("未知 mark、非法 attrs 与不支持块属性在规划前整批拒绝", () => {
    expect(planStyleOps(doc, [
      { op: "set_mark", blockId: "b1", start: 0, end: 1, mark: { type: "sub" } },
    ]).ok).toBe(false);
    expect(planStyleOps(doc, [
      { op: "set_mark", blockId: "b1", start: 0, end: 1, mark: { type: "bold", attrs: { color: "#FF0000" } } },
    ]).ok).toBe(false);
  });
});

describe("edStyleOps · 不可信载荷严格解析", () => {
  it("接受真实 mark 名与受控 attrs", () => {
    expect(parseStyleOps([
      { op: "set_mark", blockId: "b1", start: 0, end: 2, mark: { type: "subscript" } },
      { op: "set_mark", blockId: "b1", start: 0, end: 2, mark: { type: "textStyle", attrs: { color: "#FF0000", fontSize: "12pt" } } },
      { op: "set_mark", blockId: "b1", start: 0, end: 2, mark: { type: "link", attrs: { href: "https://example.com" } } },
      { op: "set_block_attr", blockId: "b1", attrs: { textAlign: "center" } },
    ])).toHaveLength(4);
  });

  it("错误别名、危险链接、未知字段或批内任一非法项使整批拒绝", () => {
    for (const value of [
      [{ op: "set_mark", blockId: "b1", start: 0, end: 2, mark: { type: "sub" } }],
      [{ op: "set_mark", blockId: "b1", start: 0, end: 2, mark: { type: "bold", attrs: {} } }],
      [{ op: "set_mark", blockId: "b1", start: 0, end: 2, mark: { type: "textStyle", attrs: { fontSize: "01pt" } } }],
      [{ op: "set_mark", blockId: "b1", start: 0, end: 2, mark: { type: "link", attrs: { href: "javascript:alert(1)" } } }],
      [{ op: "set_block_attr", blockId: "b1", attrs: { textAlign: "middle" } }],
      [
        { op: "set_mark", blockId: "b1", start: 0, end: 2, mark: { type: "bold" } },
        { op: "set_mark", blockId: "b1", start: 0, end: 2, mark: { type: "bold" }, extra: true },
      ],
    ]) {
      expect(parseStyleOps(value)).toEqual([]);
    }
  });
});
