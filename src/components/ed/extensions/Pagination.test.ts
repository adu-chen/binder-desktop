import { describe, it, expect } from "vitest";
import { Schema } from "@tiptap/pm/model";
import { EditorState } from "@tiptap/pm/state";
import {
  computePageBreaks,
  gapHeightBeforePosition,
  isPaginationSettledTransaction,
  isNaturalLineReset,
  isParagraphEligibleForNaturalLineSplit,
  mapPageBreakPlan,
  normalizeRenderedPositionY,
  paginationKey,
  samePlan,
  sameBreaks,
  shouldRebuildPageGapDecorations,
  shouldMeasurePagination,
  paginationMeasureDelay,
  samePageGeometry,
  shouldRenderPageBreak,
  diffExtents,
  diffExtentsWithoutPageGaps,
  PAGE_CONTENT_HEIGHT,
  type PageBreakPlan,
} from "./Pagination";

// covers: BR-ED-DATA-003
// 切页判定纯逻辑（PI-ED-06 页高修复版）：块级不切分，累计超页高在块前换页；每页携带页尾补足 fill；
// 不变量：非溢出页 used + fill = 预算（每页视觉高度恒等所选纸张内容区，#8 修复）；末页 lastFill 撑满。
// 纯逻辑与 DOM 测量解耦（测量属视觉层，jsdom 无布局不可单测，真机验收）。
describe("Pagination · computePageBreaks (BR-ED-DATA-003 · PI-ED-06)", () => {
  it("空序列 → 无缝 + 末页满补", () => {
    expect(computePageBreaks([], 1000)).toEqual({ breaks: [], lastFill: 1000 });
  });

  it("全部块放得下一页 → 无缝，末页补足余量", () => {
    expect(computePageBreaks([300, 300, 300], 1000)).toEqual({ breaks: [], lastFill: 100 });
  });

  it("累计恰好等于页高不换页（用 > 而非 >=），fill=0", () => {
    // 500+500=1000 = 页高；第三块溢出 → 缝@2，页1 fill=0；末页 500 → lastFill=500
    expect(computePageBreaks([500, 500, 500], 1000)).toEqual({
      breaks: [{ id: "pg-2", pos: 2, kind: "block", fill: 0 }],
      lastFill: 500,
    });
  });

  it("页高不变量：非溢出页 used + fill = 预算（#8 核心修复断言）", () => {
    const heights = [600, 600, 600, 400, 300];
    const budget = 1000;
    const plan = computePageBreaks(heights, budget);
    // 重放：按缝切分各页，校验每页 used+fill=budget（末页 used+lastFill=budget）
    let page: number[] = [];
    let bi = 0;
    for (let i = 0; i < heights.length; i++) {
      if (bi < plan.breaks.length && plan.breaks[bi].pos === i) {
        const used = page.reduce((a, b) => a + b, 0);
        expect(used + plan.breaks[bi].fill).toBe(budget);
        page = [];
        bi += 1;
      }
      page.push(heights[i]);
    }
    const lastUsed = page.reduce((a, b) => a + b, 0);
    expect(lastUsed + plan.lastFill).toBe(budget);
  });

  it("首块永不换页（首块超页高也不在其前插缝）", () => {
    const plan = computePageBreaks([1500, 100], 1000);
    expect(plan.breaks).toEqual([{ id: "pg-2", pos: 1, kind: "block", fill: 0 }]); // 页1 超预算 → fill 钳 0
    expect(plan.lastFill).toBe(900);
    expect(computePageBreaks([1500], 1000)).toEqual({ breaks: [], lastFill: 0 }); // 超页末页不补负
  });

  it("单块高于整页 → 独占其页且 fill=0（溢出不补负）", () => {
    const plan = computePageBreaks([400, 2000, 300], 1000);
    expect(plan.breaks).toEqual([
      { id: "pg-2", pos: 1, kind: "block", fill: 600 }, // 页1: 400 → 补 600
      { id: "pg-3", pos: 2, kind: "block", fill: 0 }, // 页2: 2000 超预算 → 钳 0
    ]);
    expect(plan.lastFill).toBe(700); // 末页 300 → 补 700
  });

  it("fill 保留小数，避免逐页取整形成累计漂移", () => {
    const plan = computePageBreaks([600.4, 600], 1000);
    expect(plan.breaks[0].fill).toBeCloseTo(399.6);
    expect(plan.lastFill).toBe(400);
  });

  it("页高 ≤ 0（异常/未布局）→ 空方案，安全兜底", () => {
    expect(computePageBreaks([500, 500], 0)).toEqual({ breaks: [], lastFill: 0 });
    expect(computePageBreaks([500], -10)).toEqual({ breaks: [], lastFill: 0 });
  });

  it("A4 内容高度常量为正（几何自洽）", () => {
    expect(PAGE_CONTENT_HEIGHT).toBeGreaterThan(0);
  });
});

// covers: BR-ED-DATA-003
// 方案近似比较：0.5px 亚像素容差消测量抖动；Decoration 身份只由缝数/位置决定。
describe("Pagination · samePlan", () => {
  const base: PageBreakPlan = {
    breaks: [{ id: "pg-2", pos: 20, kind: "block", fill: 100 }],
    lastFill: 50,
  };

  it("完全相同 → true；0.5px 容差内 → true", () => {
    expect(samePlan(base, {
      breaks: [{ id: "changed-id", pos: 20, kind: "block", fill: 100 }],
      lastFill: 50,
    })).toBe(true);
    expect(samePlan(base, {
      breaks: [{ id: "pg-2", pos: 20, kind: "block", fill: 100.5 }],
      lastFill: 49.5,
    })).toBe(true);
  });

  it("fill 差超容差 / position 或 kind 不同 / 缝数不同 → false", () => {
    expect(samePlan(base, {
      breaks: [{ id: "pg-2", pos: 20, kind: "block", fill: 100.6 }],
      lastFill: 50,
    })).toBe(false);
    expect(samePlan(base, {
      breaks: [{ id: "pg-2", pos: 21, kind: "block", fill: 100 }],
      lastFill: 50,
    })).toBe(false);
    expect(samePlan(base, {
      breaks: [{ id: "pg-2", pos: 20, kind: "paragraph-line", fill: 100 }],
      lastFill: 50,
    })).toBe(false);
    expect(samePlan(base, { breaks: [], lastFill: 50 })).toBe(false);
  });

  it("仅末页补白变化：完整方案不同，但普通分页缝保持相同", () => {
    const next: PageBreakPlan = {
      breaks: [{ id: "pg-2", pos: 20, kind: "block", fill: 100 }],
      lastFill: 42,
    };
    expect(samePlan(base, next)).toBe(false);
    expect(sameBreaks(base, next)).toBe(true);
    expect(shouldRebuildPageGapDecorations(base, next)).toBe(false);
  });

  it("只有分页位置改变时重建普通分页缝，fill 改走稳定 DOM CSS 变量", () => {
    expect(shouldRebuildPageGapDecorations(base, {
      breaks: [{ id: "pg-2", pos: 21, kind: "block", fill: 100 }], lastFill: 50,
    })).toBe(true);
    expect(shouldRebuildPageGapDecorations(base, {
      breaks: [{ id: "pg-2", pos: 20, kind: "block", fill: 103 }], lastFill: 50,
    })).toBe(false);
    expect(sameBreaks(base, {
      breaks: [{ id: "pg-2", pos: 20, kind: "block", fill: 999 }], lastFill: 0,
    })).toBe(true);
  });
});

// covers: BR-ED-DATA-003
describe("Pagination · position plan transaction mapping", () => {
  const schema = new Schema({
    nodes: {
      doc: { content: "paragraph+" },
      paragraph: { content: "text*" },
      text: {},
    },
  });
  const doc = schema.node("doc", null, [
    schema.node("paragraph", null, schema.text("abcd")),
    schema.node("paragraph", null, schema.text("ef")),
  ]);
  const base: PageBreakPlan = {
    breaks: [
      { id: "pg-2", pos: 3, kind: "paragraph-line", fill: 20 },
      { id: "pg-3", pos: 6, kind: "block", fill: 30 },
    ],
    lastFill: 40,
  };

  it("断点前插入时 plan 与 side:-1 widget 同步后移", () => {
    const state = EditorState.create({ schema, doc });
    expect(mapPageBreakPlan(base, state.tr.insertText("X", 1))).toEqual({
      breaks: [
        { id: "pg-2", pos: 4, kind: "paragraph-line", fill: 20 },
        { id: "pg-3", pos: 7, kind: "block", fill: 30 },
      ],
      lastFill: 40,
    });
  });

  it("恰在断点插入时 assoc=-1 保持断点在新增内容之前", () => {
    const state = EditorState.create({ schema, doc });
    const mapped = mapPageBreakPlan(base, state.tr.insertText("X", 3));
    expect(mapped.breaks[0].pos).toBe(3);
  });

  it("删除覆盖行间断点时安全丢弃，后续 Block 断点继续映射", () => {
    const state = EditorState.create({ schema, doc });
    expect(mapPageBreakPlan(base, state.tr.delete(2, 4))).toEqual({
      breaks: [{ id: "pg-2", pos: 4, kind: "block", fill: 30 }],
      lastFill: 40,
    });
  });
});

// covers: BR-ED-DATA-003
// 分页输出本身不可回灌为测量输入，否则打开大文档会形成重排正反馈。
describe("Pagination · measurement invalidation", () => {
  it("真实行尾测量只撤下段内页缝，Block 物理分页始终保留", () => {
    expect(shouldRenderPageBreak({ kind: "block" }, true)).toBe(true);
    expect(shouldRenderPageBreak({ kind: "paragraph-line" }, true)).toBe(false);
    expect(shouldRenderPageBreak({ kind: "paragraph-line" }, false)).toBe(true);
  });

  it("仅分页方案提交不会触发下一轮测量", () => {
    expect(shouldMeasurePagination({
      wasEnabled: true,
      enabled: true,
      docChanged: false,
      geometryChanged: false,
    })).toBe(false);
  });

  it("仅内容、启用或纸张几何变化触发测量", () => {
    expect(shouldMeasurePagination({ wasEnabled: false, enabled: true, docChanged: false, geometryChanged: false })).toBe(true);
    expect(shouldMeasurePagination({ wasEnabled: true, enabled: true, docChanged: true, geometryChanged: false })).toBe(true);
    expect(shouldMeasurePagination({ wasEnabled: true, enabled: true, docChanged: false, geometryChanged: true })).toBe(true);
    expect(shouldMeasurePagination({ wasEnabled: true, enabled: true, docChanged: false, geometryChanged: false, layoutReset: true })).toBe(true);
  });

  it("普通内容输入等待 450ms 静默期，首次/几何/载入重置立即结算", () => {
    expect(paginationMeasureDelay({
      wasEnabled: true,
      enabled: true,
      docChanged: true,
      geometryChanged: false,
    })).toBe(450);
    expect(paginationMeasureDelay({
      wasEnabled: false,
      enabled: true,
      docChanged: false,
      geometryChanged: false,
    })).toBe(0);
    expect(paginationMeasureDelay({
      wasEnabled: true,
      enabled: true,
      docChanged: false,
      geometryChanged: true,
    })).toBe(0);
    expect(paginationMeasureDelay({
      wasEnabled: true,
      enabled: true,
      docChanged: false,
      geometryChanged: false,
      layoutReset: true,
    })).toBe(0);
    expect(paginationMeasureDelay({
      wasEnabled: true,
      enabled: true,
      docChanged: false,
      geometryChanged: false,
    })).toBeNull();
  });

  it("首轮测量只有显式 settled meta 才能揭示 BDocx", () => {
    const schema = new Schema({
      nodes: {
        doc: { content: "text*" },
        text: {},
      },
    });
    const state = EditorState.create({ schema });
    expect(isPaginationSettledTransaction(state.tr)).toBe(false);
    expect(isPaginationSettledTransaction(
      state.tr.setMeta(paginationKey, { settled: true }),
    )).toBe(true);
  });
});

describe("Pagination · layout configuration", () => {
  it("相同纸张几何不应触发重复重置", () => {
    expect(samePageGeometry(
      { width: 794, height: 1123, marginX: 76, marginY: 60, gap: 28 },
      { width: 794, height: 1123, marginX: 76, marginY: 60, gap: 28 },
    )).toBe(true);
  });

  it("任一纸张几何字段变化都必须重新分页", () => {
    expect(samePageGeometry(
      { width: 794, height: 1123, marginX: 76, marginY: 60, gap: 28 },
      { width: 1123, height: 794, marginX: 76, marginY: 60, gap: 28 },
    )).toBe(false);
  });
});

// covers: BR-ED-DATA-003
// 位置差分法（页高修复二轮）：块高=相邻 top 差（含塌陷后真实间距），Σ=精确渲染跨度；
// 旧求和法对塌陷 margin 重复计数导致页偏矮的回归防线。
describe("Pagination · diffExtents（位置差分）", () => {
  it("相邻 top 差分 + 末块 bottom+marginBottom", () => {
    // 三块：top=0/100/250，末块 bottom=300、mb=12 → [100,150,62]
    expect(diffExtents([0, 100, 250], 300, 12)).toEqual([100, 150, 62]);
  });
  it("Σ差分 = 总跨度 + 末 margin（塌陷不重复计数的不变量）", () => {
    const tops = [0, 80, 200, 330];
    const hs = diffExtents(tops, 400, 10);
    expect(hs.reduce((a, b) => a + b, 0)).toBe(400 - 0 + 10);
  });
  it("空/单块边界", () => {
    expect(diffExtents([], 0, 0)).toEqual([]);
    expect(diffExtents([50], 120, 8)).toEqual([78]);
  });
  it("乱序防御：负差钳 0", () => {
    expect(diffExtents([100, 50], 60, 0)).toEqual([0, 10]);
  });
});

// covers: BR-ED-DATA-003
// 分页重测量稳定性：从当前可见树读取 gap 高度并扣除，不能先隐藏/恢复可见分页缝。
describe("Pagination · diffExtentsWithoutPageGaps", () => {
  it("扣除一个分页缝后恢复连续内容坐标", () => {
    const blocks = [
      { pos: 0, endPos: 10, top: 0, bottom: 100 },
      { pos: 10, endPos: 20, top: 100, bottom: 200 },
      { pos: 20, endPos: 30, top: 350, bottom: 450 },
      { pos: 30, endPos: 40, top: 450, bottom: 530 },
    ];
    expect(diffExtentsWithoutPageGaps(blocks, [{ id: "pg-2", pos: 20, height: 150 }], 10))
      .toEqual([100, 100, 100, 90]);
  });

  it("多个分页缝按 position 累计扣除，输入数组顺序不影响结果", () => {
    const blocks = [
      { pos: 0, endPos: 10, top: 0, bottom: 50 },
      { pos: 10, endPos: 20, top: 150, bottom: 220 },
      { pos: 20, endPos: 30, top: 220, bottom: 280 },
      { pos: 30, endPos: 40, top: 500, bottom: 560 },
    ];
    const gaps = [
      { id: "pg-3", pos: 30, height: 220 },
      { id: "pg-2", pos: 10, height: 100 },
    ];
    expect(diffExtentsWithoutPageGaps(blocks, gaps, 0)).toEqual([50, 70, 60, 60]);
  });

  it("段落内部行间 gap 会从本 Block bottom 和后续 Block 坐标同时扣除", () => {
    const blocks = [
      { pos: 0, endPos: 100, top: 0, bottom: 250 },
      { pos: 100, endPos: 200, top: 250, bottom: 330 },
    ];
    const gaps = [{ id: "pg-2", pos: 50, height: 150 }];
    expect(diffExtentsWithoutPageGaps(blocks, gaps, 0)).toEqual([100, 80]);
  });

  it("同 position bias 与 side:-1 对齐：top/行首包含 gap，Block bottom 排除下一块 gap", () => {
    const gaps = [{ id: "pg-2", pos: 10, height: 150 }];
    expect(gapHeightBeforePosition(gaps, 10, "before")).toBe(0);
    expect(gapHeightBeforePosition(gaps, 10, "after")).toBe(150);
    expect(normalizeRenderedPositionY(250, 10, "after", gaps)).toBe(100);
    expect(normalizeRenderedPositionY(100, 10, "before", gaps)).toBe(100);
  });

  it("无分页缝时等价于普通位置差分", () => {
    const blocks = [
      { pos: 0, endPos: 10, top: 20, bottom: 80 },
      { pos: 10, endPos: 20, top: 100, bottom: 180 },
    ];
    expect(diffExtentsWithoutPageGaps(blocks, [], 8)).toEqual(diffExtents([20, 100], 180, 8));
  });
});

// covers: BR-ED-DATA-003
describe("Pagination · natural paragraph line boundary", () => {
  const rect = (
    top: number,
    bottom: number,
    left: number,
    right: number,
  ) => ({
    top,
    bottom,
    left,
    right,
    width: right - left,
    height: bottom - top,
  });

  it("LTR 只有纵向进入下一行且横向回到行首才接受断点", () => {
    const previous = rect(10, 26, 610, 620);
    expect(isNaturalLineReset(previous, rect(36, 52, 80, 90), "ltr")).toBe(true);
    // 字号/上下标可能改变 top，但横向仍在本行继续，不能误判为自然换行。
    expect(isNaturalLineReset(previous, rect(14, 32, 624, 636), "ltr")).toBe(false);
    // 只有横向回退、纵向仍在同一行也不是合法行尾。
    expect(isNaturalLineReset(previous, rect(10, 26, 80, 90), "ltr")).toBe(false);
  });

  it("RTL 使用相反的水平回行方向验证", () => {
    const previous = rect(10, 26, 80, 90);
    expect(isNaturalLineReset(previous, rect(36, 52, 610, 620), "rtl")).toBe(true);
    expect(isNaturalLineReset(previous, rect(36, 52, 68, 78), "rtl")).toBe(false);
  });

  it("含 hardBreak 的段落不参与段内分页，避免短行或旧 widget 污染被当成自然行尾", () => {
    const schema = new Schema({
      nodes: {
        doc: { content: "paragraph+" },
        paragraph: { content: "(text | hardBreak)*" },
        hardBreak: { group: "inline", inline: true },
        text: {},
      },
    });
    expect(isParagraphEligibleForNaturalLineSplit(
      schema.node("paragraph", null, schema.text("纯文本自然换行")),
    )).toBe(true);
    expect(isParagraphEligibleForNaturalLineSplit(
      schema.node("paragraph", null, [
        schema.text("前半"),
        schema.node("hardBreak"),
        schema.text("后半"),
      ]),
    )).toBe(false);
  });
});
