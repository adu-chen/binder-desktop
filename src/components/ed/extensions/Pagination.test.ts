import { describe, it, expect } from "vitest";
import { computePageBreaks, samePlan, diffExtents, PAGE_CONTENT_HEIGHT, type PageBreakPlan } from "./Pagination";

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
      breaks: [{ index: 2, fill: 0 }],
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
      if (bi < plan.breaks.length && plan.breaks[bi].index === i) {
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
    expect(plan.breaks).toEqual([{ index: 1, fill: 0 }]); // 页1 超预算 → fill 钳 0
    expect(plan.lastFill).toBe(900);
    expect(computePageBreaks([1500], 1000)).toEqual({ breaks: [], lastFill: 0 }); // 超页末页不补负
  });

  it("单块高于整页 → 独占其页且 fill=0（溢出不补负）", () => {
    const plan = computePageBreaks([400, 2000, 300], 1000);
    expect(plan.breaks).toEqual([
      { index: 1, fill: 600 }, // 页1: 400 → 补 600
      { index: 2, fill: 0 }, // 页2: 2000 超预算 → 钳 0
    ]);
    expect(plan.lastFill).toBe(700); // 末页 300 → 补 700
  });

  it("fill 取整（消浮点测量小数）", () => {
    const plan = computePageBreaks([600.4, 600], 1000);
    expect(plan.breaks[0].fill).toBe(400); // round(1000-600.4)
    expect(Number.isInteger(plan.lastFill)).toBe(true);
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
// 方案近似比较：±1px 容差消测量抖动（防 RAF 重算循环）；结构差异（缝数/位置）精确判不同。
describe("Pagination · samePlan", () => {
  const base: PageBreakPlan = { breaks: [{ index: 2, fill: 100 }], lastFill: 50 };

  it("完全相同 → true；±1px 容差内 → true", () => {
    expect(samePlan(base, { breaks: [{ index: 2, fill: 100 }], lastFill: 50 })).toBe(true);
    expect(samePlan(base, { breaks: [{ index: 2, fill: 101 }], lastFill: 49 })).toBe(true);
  });

  it("fill 差超容差 / index 不同 / 缝数不同 → false", () => {
    expect(samePlan(base, { breaks: [{ index: 2, fill: 103 }], lastFill: 50 })).toBe(false);
    expect(samePlan(base, { breaks: [{ index: 3, fill: 100 }], lastFill: 50 })).toBe(false);
    expect(samePlan(base, { breaks: [], lastFill: 50 })).toBe(false);
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
