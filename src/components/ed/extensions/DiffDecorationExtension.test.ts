import { describe, it, expect } from "vitest";
import { selectDecorations, selectPreviewWidgets, type BlockRef, type GreenReviewValue } from "./DiffDecorationExtension";
import type { GreenReviewState } from "../../../store/deStore";

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
