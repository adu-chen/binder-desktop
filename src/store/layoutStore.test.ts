import { describe, it, expect, beforeEach } from "vitest";
import { clampWidth, layoutStore, PANE_LIMITS } from "./layoutStore";

describe("layoutStore · clampWidth", () => {
  it("在区间内原样返回", () => {
    expect(clampWidth(300, 180, 520)).toBe(300);
  });
  it("低于 min 钳到 min，高于 max 钳到 max", () => {
    expect(clampWidth(100, 180, 520)).toBe(180);
    expect(clampWidth(900, 180, 520)).toBe(520);
  });
  it("边界值含入", () => {
    expect(clampWidth(180, 180, 520)).toBe(180);
    expect(clampWidth(520, 180, 520)).toBe(520);
  });
  it("NaN/非有限（含 Infinity）→ 回落 min（安全兜底）", () => {
    expect(clampWidth(NaN, 180, 520)).toBe(180);
    expect(clampWidth(Infinity, 180, 520)).toBe(180); // 非有限值一律回落 min
    expect(clampWidth(-Infinity, 180, 520)).toBe(180);
  });
});

describe("layoutStore · 设置与钳制", () => {
  beforeEach(() => {
    layoutStore.resetSide();
    layoutStore.resetChat();
  });

  it("setSideW 钳制到 side 约束", () => {
    layoutStore.setSideW(10_000);
    expect(layoutStore.getSnapshot().sideW).toBe(PANE_LIMITS.side.max);
    layoutStore.setSideW(0);
    expect(layoutStore.getSnapshot().sideW).toBe(PANE_LIMITS.side.min);
  });

  it("setChatW 钳制到 chat 约束", () => {
    layoutStore.setChatW(10_000);
    expect(layoutStore.getSnapshot().chatW).toBe(PANE_LIMITS.chat.max);
    layoutStore.setChatW(0);
    expect(layoutStore.getSnapshot().chatW).toBe(PANE_LIMITS.chat.min);
  });

  it("reset 恢复默认宽", () => {
    layoutStore.setSideW(200);
    layoutStore.resetSide();
    expect(layoutStore.getSnapshot().sideW).toBe(PANE_LIMITS.side.default);
  });

  it("subscribe 在变化时通知，无变化不通知", () => {
    layoutStore.setSideW(PANE_LIMITS.side.default); // 先归默认
    let n = 0;
    const unsub = layoutStore.subscribe(() => (n += 1));
    layoutStore.setSideW(300);
    expect(n).toBe(1);
    layoutStore.setSideW(300); // 同值不通知
    expect(n).toBe(1);
    unsub();
  });
});
