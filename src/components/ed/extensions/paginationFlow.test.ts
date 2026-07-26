import { describe, expect, it } from "vitest";
import {
  createPaginationFlowPlanner,
  type RenderedFlowBlock,
} from "./paginationFlow";

function atomic(pos: number, height: number): RenderedFlowBlock {
  return {
    blockPos: pos,
    kind: "atomic",
    fragments: [{
      pos,
      height,
      kind: "block",
      blockPos: pos,
      lineIndex: 0,
      lineCount: 1,
    }],
  };
}

function paragraph(pos: number, heights: number[]): RenderedFlowBlock {
  return {
    blockPos: pos,
    kind: "paragraph",
    fragments: heights.map((height, lineIndex) => ({
      pos: pos + lineIndex,
      height,
      kind: lineIndex === 0 ? "block" : "paragraph-line",
      blockPos: pos,
      lineIndex,
      lineCount: heights.length,
    })),
  };
}

function plan(budget: number, blocks: RenderedFlowBlock[]) {
  const planner = createPaginationFlowPlanner(budget);
  blocks.forEach((block) => planner.append(block));
  return planner.finish();
}

// covers: BR-ED-DATA-003
describe("paginationFlow · position 分页规划", () => {
  it("原子 Block 与旧规则等价，并保留精确满页语义", () => {
    expect(plan(100, [atomic(0, 60), atomic(10, 60)])).toEqual({
      breaks: [{ id: "pg-2", pos: 10, kind: "block", fill: 40 }],
      lastFill: 40,
    });
    expect(plan(100, [atomic(0, 50), atomic(10, 50)])).toEqual({
      breaks: [],
      lastFill: 0,
    });
  });

  it("段落在当前页保留所有能完整容纳的视觉行，断点使用下一行行首 position", () => {
    expect(plan(100, [atomic(0, 80), paragraph(100, [10, 10, 10, 10])])).toEqual({
      breaks: [{ id: "pg-2", pos: 102, kind: "paragraph-line", fill: 0 }],
      lastFill: 80,
    });
  });

  it("当前页只能再放一行时仍保留该行；一行都放不下才在段落前换页", () => {
    expect(plan(100, [atomic(0, 90), paragraph(100, [10, 10, 10, 10])])).toEqual({
      breaks: [{ id: "pg-2", pos: 101, kind: "paragraph-line", fill: 0 }],
      lastFill: 70,
    });
    expect(plan(100, [atomic(0, 95), paragraph(100, [10, 10])])).toEqual({
      breaks: [{ id: "pg-2", pos: 100, kind: "block", fill: 5 }],
      lastFill: 80,
    });
  });

  it("下一页只剩一行时不回退本可容纳的行，优先填满当前页", () => {
    expect(plan(100, [atomic(0, 40), paragraph(100, [20, 20, 20, 20])])).toEqual({
      breaks: [{ id: "pg-2", pos: 103, kind: "paragraph-line", fill: 0 }],
      lastFill: 80,
    });
  });

  it("2–3 行段落同样按剩余空间贪心拆分，不整段移页", () => {
    expect(plan(100, [atomic(0, 60), paragraph(100, [20, 20, 20])])).toEqual({
      breaks: [{ id: "pg-2", pos: 102, kind: "paragraph-line", fill: 0 }],
      lastFill: 80,
    });
  });

  it("长段落跨多页时每页均使用最大可容纳行数", () => {
    expect(plan(100, [paragraph(0, Array.from({ length: 10 }, () => 30))])).toEqual({
      breaks: [
        { id: "pg-2", pos: 3, kind: "paragraph-line", fill: 10 },
        { id: "pg-3", pos: 6, kind: "paragraph-line", fill: 10 },
        { id: "pg-4", pos: 9, kind: "paragraph-line", fill: 10 },
      ],
      lastFill: 70,
    });
  });

  it("单行高于整页时仍保证前进，溢出页 fill 钳为 0", () => {
    expect(plan(100, [paragraph(0, [150, 20, 20])])).toEqual({
      breaks: [{ id: "pg-2", pos: 1, kind: "paragraph-line", fill: 0 }],
      lastFill: 60,
    });
  });

  it("fill 保留小数且页面 id 按页序确定", () => {
    expect(plan(100, [atomic(0, 60.4), atomic(10, 60), atomic(20, 60)])).toEqual({
      breaks: [
        { id: "pg-2", pos: 10, kind: "block", fill: 39.6 },
        { id: "pg-3", pos: 20, kind: "block", fill: 40 },
      ],
      lastFill: 40,
    });
  });

  it("无效预算安全返回空方案；wouldOverflow 反映当前页余量", () => {
    expect(plan(0, [atomic(0, 50)])).toEqual({ breaks: [], lastFill: 0 });
    const planner = createPaginationFlowPlanner(100);
    expect(planner.wouldOverflow(101)).toBe(true);
    planner.append(atomic(0, 60));
    expect(planner.wouldOverflow(41)).toBe(true);
    expect(planner.wouldOverflow(40)).toBe(false);
  });
});
