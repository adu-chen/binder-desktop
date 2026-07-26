import { describe, expect, it } from "vitest";
import {
  computeClientViewportRect,
  computeClampedViewportScrollTop,
  computeViewportAnchorScrollDelta,
  estimatePointerActivatedCaretRect,
  isRenderedRectInsideViewport,
  normalizeViewportPixelsPerScrollPixel,
  shouldArmPointerActivatedInputScroll,
  shouldApplyViewportAnchorCorrection,
  shouldSuppressPaginatedSelectionScroll,
  shouldSuppressPointerActivatedInputScroll,
} from "./paginationViewportAnchor";

describe("pagination viewport anchor", () => {
  it("分页缝把锚点向下推时，正向滚动补偿保持屏幕位置", () => {
    expect(computeViewportAnchorScrollDelta({
      beforeTop: 240,
      afterTop: 388,
      viewportPixelsPerScrollPixel: 1,
    })).toBe(148);
  });

  it("容器存在屏幕/滚动比例时按比例换算，内容 zoom 不直接参与计算", () => {
    expect(computeViewportAnchorScrollDelta({
      beforeTop: 200,
      afterTop: 250,
      viewportPixelsPerScrollPixel: 0.5,
    })).toBe(100);
  });

  it("无效比例安全回落为 1；亚像素噪声不写滚动", () => {
    expect(normalizeViewportPixelsPerScrollPixel(0)).toBe(1);
    expect(normalizeViewportPixelsPerScrollPixel(Number.NaN)).toBe(1);
    expect(shouldApplyViewportAnchorCorrection(0.5)).toBe(false);
    expect(shouldApplyViewportAnchorCorrection(0.51)).toBe(true);
  });

  it("补偿目标钳制在滚动容器合法范围", () => {
    expect(computeClampedViewportScrollTop({
      currentScrollTop: 100,
      delta: 40,
      scrollHeight: 1000,
      clientHeight: 400,
    })).toBe(140);
    expect(computeClampedViewportScrollTop({
      currentScrollTop: 590,
      delta: 40,
      scrollHeight: 1000,
      clientHeight: 400,
    })).toBe(600);
    expect(computeClampedViewportScrollTop({
      currentScrollTop: 10,
      delta: -40,
      scrollHeight: 1000,
      clientHeight: 400,
    })).toBe(0);
  });

  it("从 border rect 计算排除边框和滚动条的 client viewport", () => {
    expect(computeClientViewportRect({
      borderRect: {
        left: 10,
        right: 414,
        top: 20,
        bottom: 324,
        width: 404,
        height: 304,
      },
      offsetWidth: 202,
      offsetHeight: 152,
      clientLeft: 1,
      clientTop: 1,
      clientWidth: 200,
      clientHeight: 150,
    })).toEqual({
      left: 12,
      right: 412,
      top: 22,
      bottom: 322,
    });
  });

  it("光标与 viewport 同处屏幕坐标时，缩放不改变可见性判断", () => {
    expect(isRenderedRectInsideViewport(
      { left: 200, right: 200, top: 80, bottom: 98 },
      { left: 0, right: 400, top: 0, bottom: 300 },
    )).toBe(true);
    expect(isRenderedRectInsideViewport(
      { left: 100, right: 100, top: 40, bottom: 49 },
      { left: 0, right: 200, top: 0, bottom: 150 },
    )).toBe(true);
  });

  it("仅容忍 1px 亚像素误差，真实越界交回原生滚动", () => {
    const viewport = { left: 0, right: 400, top: 0, bottom: 300 };
    expect(isRenderedRectInsideViewport(
      { left: 20, right: 20, top: -0.5, bottom: 18 },
      viewport,
    )).toBe(true);
    expect(isRenderedRectInsideViewport(
      { left: 20, right: 20, top: -1.1, bottom: 18 },
      viewport,
    )).toBe(false);
    expect(isRenderedRectInsideViewport(
      { left: 20, right: 20, top: 285, bottom: 302 },
      viewport,
    )).toBe(false);
    expect(isRenderedRectInsideViewport(
      { left: -2, right: 0, top: 80, bottom: 98 },
      viewport,
    )).toBe(false);
  });

  it("只接管分页、已揭示、非 composition 的折叠文本光标", () => {
    const visible = {
      renderedRect: { left: 200, right: 200, top: 80, bottom: 98 },
      viewportRect: { left: 0, right: 400, top: 0, bottom: 300 },
    };
    expect(shouldSuppressPaginatedSelectionScroll({
      paginated: true,
      loading: false,
      composing: false,
      collapsedTextSelection: true,
      ...visible,
    })).toBe(true);
    expect(shouldSuppressPaginatedSelectionScroll({
      paginated: false,
      loading: false,
      composing: false,
      collapsedTextSelection: true,
      ...visible,
    })).toBe(false);
    expect(shouldSuppressPaginatedSelectionScroll({
      paginated: true,
      loading: true,
      composing: false,
      collapsedTextSelection: true,
      ...visible,
    })).toBe(false);
    expect(shouldSuppressPaginatedSelectionScroll({
      paginated: true,
      loading: false,
      composing: true,
      collapsedTextSelection: true,
      ...visible,
    })).toBe(false);
    expect(shouldSuppressPaginatedSelectionScroll({
      paginated: true,
      loading: false,
      composing: false,
      collapsedTextSelection: false,
      ...visible,
    })).toBe(false);
    expect(shouldSuppressPaginatedSelectionScroll({
      paginated: true,
      loading: false,
      composing: false,
      collapsedTextSelection: true,
      renderedRect: null,
      viewportRect: visible.viewportRect,
    })).toBe(false);
  });

  it("只在最终选区为单击折叠光标时建立指针激活周期", () => {
    const base = {
      button: 0,
      clickCount: 1,
      paginated: true,
      loading: false,
      collapsedTextSelection: true,
      activationPoint: { x: 200, y: 120 },
      viewportRect: { left: 0, right: 400, top: 0, bottom: 300 },
    };
    expect(shouldArmPointerActivatedInputScroll(base)).toBe(true);
    expect(shouldArmPointerActivatedInputScroll({ ...base, clickCount: 2 })).toBe(false);
    expect(shouldArmPointerActivatedInputScroll({ ...base, collapsedTextSelection: false })).toBe(false);
    expect(shouldArmPointerActivatedInputScroll({
      ...base,
      activationPoint: { x: 200, y: 320 },
    })).toBe(false);
  });

  it("用指针屏幕锚与Range相对位移估算缩放后的真实光标位置", () => {
    expect(estimatePointerActivatedCaretRect({
      activationPoint: { x: 200, y: 120 },
      activationRenderedRect: { left: 300, right: 300, top: 400, bottom: 436 },
      currentRenderedRect: { left: 316, right: 316, top: 448, bottom: 484 },
      renderedDeltaScale: 0.5,
    })).toEqual({
      left: 207.5,
      right: 208.5,
      top: 135,
      bottom: 153,
    });
  });

  it("激活周期内连续文档变化保持抑制，真实越界或用户滚动时释放", () => {
    const base = {
      scrollTopAtPointer: 480,
      currentScrollTop: 480,
      activationPoint: { x: 200, y: 120 },
      activationRenderedRect: { left: 300, right: 300, top: 400, bottom: 436 },
      renderedDeltaScale: 0.5,
      viewportRect: { left: 0, right: 400, top: 0, bottom: 300 },
    };
    expect(shouldSuppressPointerActivatedInputScroll({
      ...base,
      documentChangedSinceLastDecision: true,
      currentRenderedRect: { left: 300, right: 300, top: 400, bottom: 436 },
    })).toBe(true);
    expect(shouldSuppressPointerActivatedInputScroll({
      ...base,
      documentChangedSinceLastDecision: true,
      currentRenderedRect: { left: 304, right: 304, top: 400, bottom: 436 },
    })).toBe(true);
    expect(shouldSuppressPointerActivatedInputScroll({
      ...base,
      documentChangedSinceLastDecision: false,
      currentRenderedRect: { left: 304, right: 304, top: 400, bottom: 436 },
    })).toBe(false);
    expect(shouldSuppressPointerActivatedInputScroll({
      ...base,
      documentChangedSinceLastDecision: true,
      currentScrollTop: 482,
      currentRenderedRect: { left: 304, right: 304, top: 400, bottom: 436 },
    })).toBe(false);
    // 分页层内部补偿会把激活周期基线同步到新 scrollTop；相同位置随后继续输入仍受保护。
    expect(shouldSuppressPointerActivatedInputScroll({
      ...base,
      documentChangedSinceLastDecision: true,
      scrollTopAtPointer: 512,
      currentScrollTop: 512,
      currentRenderedRect: { left: 304, right: 304, top: 400, bottom: 436 },
    })).toBe(true);
    expect(shouldSuppressPointerActivatedInputScroll({
      ...base,
      documentChangedSinceLastDecision: true,
      currentRenderedRect: { left: 304, right: 304, top: 780, bottom: 816 },
    })).toBe(false);
  });
});
