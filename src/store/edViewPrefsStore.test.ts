/** @vitest-environment jsdom */
import { describe, it, expect, beforeEach } from "vitest";
import {
  getPageGeometry,
  pageContentHeight,
  edViewPrefsStore,
  PAPERS,
} from "./edViewPrefsStore";

describe("edViewPrefsStore · getPageGeometry（纯函数）", () => {
  it("A4 竖排基准尺寸", () => {
    const g = getPageGeometry("A4", "portrait");
    expect(g.width).toBe(PAPERS.A4.width);
    expect(g.height).toBe(PAPERS.A4.height);
  });

  it("横排交换宽高（各纸张）", () => {
    for (const k of ["A4", "A3", "A5", "Letter"] as const) {
      const p = getPageGeometry(k, "portrait");
      const l = getPageGeometry(k, "landscape");
      expect(l.width).toBe(p.height);
      expect(l.height).toBe(p.width);
      expect(l.marginX).toBe(p.marginX); // 边距/缝隙恒定
      expect(l.gap).toBe(p.gap);
    }
  });

  it("内容高度预算 = 纸高 − 2×上下边距，恒为正", () => {
    for (const k of ["A4", "A3", "A5", "Letter"] as const) {
      for (const o of ["portrait", "landscape"] as const) {
        const g = getPageGeometry(k, o);
        expect(pageContentHeight(g)).toBe(g.height - g.marginY * 2);
        expect(pageContentHeight(g)).toBeGreaterThan(0);
      }
    }
  });
});

describe("edViewPrefsStore · 持久化与订阅", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("未存偏好回默认（A4 竖排 100%）", () => {
    const p = edViewPrefsStore.getPrefs("fresh-" + Math.random());
    expect(p).toEqual({ paper: "A4", orientation: "portrait", zoom: 1 });
  });

  it("setPaper/setOrientation 写穿 localStorage 并可回读", () => {
    const path = "docs/t1.docx";
    edViewPrefsStore.setPaper(path, "A3");
    edViewPrefsStore.setOrientation(path, "landscape");
    const raw = JSON.parse(localStorage.getItem("binder.ed.viewprefs." + path)!);
    expect(raw.paper).toBe("A3");
    expect(raw.orientation).toBe("landscape");
    expect(edViewPrefsStore.getPrefs(path).paper).toBe("A3");
  });

  it("损坏/脏数据回落默认（字段级校验）", () => {
    const path = "docs/t2.docx";
    localStorage.setItem("binder.ed.viewprefs." + path, '{"paper":"B5","orientation":"diagonal","zoom":"x"}');
    const p = edViewPrefsStore.getPrefs(path);
    expect(p).toEqual({ paper: "A4", orientation: "portrait", zoom: 1 });
  });

  it("subscribe 在变更时通知", () => {
    const path = "docs/t3-" + Math.random();
    let n = 0;
    const unsub = edViewPrefsStore.subscribe(() => (n += 1));
    edViewPrefsStore.setPaper(path, "A5");
    expect(n).toBe(1);
    unsub();
  });
});
