import { describe, it, expect } from "vitest";
import { Schema } from "@tiptap/pm/model";
import { extractOutline, sameOutline, outlineStore } from "./outlineStore";

// PI-ED-12 大纲纯逻辑：顶层 heading → {blockId, level, text}；无 blockId 标题跳过（无锚不可跳转）；
// 相等比较避免无谓发布。

const schema = new Schema({
  nodes: {
    doc: { content: "block+" },
    heading: {
      group: "block",
      content: "inline*",
      attrs: { blockId: { default: null }, level: { default: 1 } },
    },
    paragraph: { group: "block", content: "inline*", attrs: { blockId: { default: null } } },
    text: { group: "inline" },
  },
});

const h = (id: string | null, level: number, text: string) =>
  schema.node("heading", { blockId: id, level }, text ? [schema.text(text)] : []);
const p = (text: string) => schema.node("paragraph", { blockId: "pb" }, [schema.text(text)]);

describe("outlineStore · extractOutline", () => {
  it("提取标题树（级别 + 文本 + 锚），段落不入大纲", () => {
    const doc = schema.node("doc", null, [h("h1", 1, "第一章"), p("正文"), h("h2", 2, "第一节")]);
    expect(extractOutline(doc)).toEqual([
      { blockId: "h1", level: 1, text: "第一章" },
      { blockId: "h2", level: 2, text: "第一节" },
    ]);
  });

  it("无 blockId 的标题跳过（不产死项）", () => {
    const doc = schema.node("doc", null, [h(null, 1, "无锚标题"), h("h2", 2, "有锚")]);
    expect(extractOutline(doc)).toEqual([{ blockId: "h2", level: 2, text: "有锚" }]);
  });
});

describe("outlineStore · sameOutline / set", () => {
  it("相等不发布，变化才通知", () => {
    outlineStore.reset();
    let n = 0;
    const unsub = outlineStore.subscribe(() => (n += 1));
    const items = [{ blockId: "a", level: 1, text: "T" }];
    outlineStore.set(items);
    expect(n).toBe(1);
    outlineStore.set([{ blockId: "a", level: 1, text: "T" }]); // 内容相等 → 跳过
    expect(n).toBe(1);
    outlineStore.set([{ blockId: "a", level: 2, text: "T" }]); // level 变 → 通知
    expect(n).toBe(2);
    expect(sameOutline([], [])).toBe(true);
    unsub();
    outlineStore.reset();
  });
});
