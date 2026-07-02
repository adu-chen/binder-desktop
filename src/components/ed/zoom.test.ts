import { describe, it, expect } from "vitest";
import { clampZoom, stepZoom, formatZoom, ZOOM } from "./zoom";

describe("ed/zoom · clampZoom", () => {
  it("区间内保留（规整 2 位小数）", () => {
    expect(clampZoom(1)).toBe(1);
    expect(clampZoom(1.2)).toBe(1.2);
  });
  it("越界钳到 min/max", () => {
    expect(clampZoom(0.1)).toBe(ZOOM.min);
    expect(clampZoom(5)).toBe(ZOOM.max);
  });
  it("消 FP 漂移", () => {
    expect(clampZoom(0.7999999999)).toBe(0.8);
  });
  it("非有限 → 默认 100%", () => {
    expect(clampZoom(NaN)).toBe(ZOOM.default);
    expect(clampZoom(Infinity)).toBe(ZOOM.default);
  });
});

describe("ed/zoom · stepZoom", () => {
  it("放大/缩小一步且钳制", () => {
    expect(stepZoom(1, ZOOM.step)).toBe(1.1);
    expect(stepZoom(1, -ZOOM.step)).toBe(0.9);
  });
  it("连续步进无 FP 漂移", () => {
    let z = 1;
    for (let i = 0; i < 3; i++) z = stepZoom(z, -ZOOM.step);
    expect(z).toBe(0.7);
  });
  it("到达边界不越界", () => {
    expect(stepZoom(ZOOM.max, ZOOM.step)).toBe(ZOOM.max);
    expect(stepZoom(ZOOM.min, -ZOOM.step)).toBe(ZOOM.min);
  });
});

describe("ed/zoom · formatZoom", () => {
  it("格式化为百分比整数", () => {
    expect(formatZoom(1)).toBe("100%");
    expect(formatZoom(1.2)).toBe("120%");
    expect(formatZoom(0.5)).toBe("50%");
  });
});
