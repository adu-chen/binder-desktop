import { describe, it, expect } from "vitest";
import { computePageBreaks, PAGE_CONTENT_HEIGHT } from "./Pagination";

// covers: BR-ED-DATA-003
// 切页判定纯逻辑：块级不切分，累计超页高在块前换页；首块不换；单块超整页独占其页；全放得下/空→无缝。
// 纯逻辑与 DOM 测量解耦（测量属视觉层，jsdom 无布局不可单测，真机验收）。
describe("Pagination · computePageBreaks (BR-ED-DATA-003)", () => {
  it("空序列 → 无分页缝", () => {
    expect(computePageBreaks([], 1000)).toEqual([]);
  });

  it("全部块放得下一页 → 无分页缝", () => {
    expect(computePageBreaks([300, 300, 300], 1000)).toEqual([]);
  });

  it("累计恰好等于页高不换页（用 > 而非 >=）", () => {
    // 500+500=1000 = 页高，不溢出；第三块 500 溢出 → 在下标 2 前换页
    expect(computePageBreaks([500, 500, 500], 1000)).toEqual([2]);
  });

  it("累计超过页高 → 在导致溢出的块前换页并从该块重新累计", () => {
    // 600(页1) | 600+600=1200>1000 → break@1；页2:600, +600=1200>1000 → break@2
    expect(computePageBreaks([600, 600, 600], 1000)).toEqual([1, 2]);
  });

  it("首块永不换页（即使首块高于页高也不在其前插缝）", () => {
    expect(computePageBreaks([1500, 100], 1000)).toEqual([1]); // 首块独占页1，第二块换到页2
    expect(computePageBreaks([1500], 1000)).toEqual([]); // 仅一超高块 → 无缝
  });

  it("单块高于整页 → 在其前换页独占其页（非首块），后续块继续正常换页", () => {
    // 400(页1) | 2000>页高→break@1 独占 | 400 之后 used=2000+... 实际：break@1后 used=0,+2000=2000；
    // 下一块 300: 2000+300>1000 → break@2
    expect(computePageBreaks([400, 2000, 300], 1000)).toEqual([1, 2]);
  });

  it("页高 ≤ 0（异常/未布局）→ 无缝，安全兜底", () => {
    expect(computePageBreaks([500, 500], 0)).toEqual([]);
    expect(computePageBreaks([500, 500], -10)).toEqual([]);
  });

  it("A4 内容高度常量为正（几何自洽）", () => {
    expect(PAGE_CONTENT_HEIGHT).toBeGreaterThan(0);
  });
});
