import { beforeEach, describe, expect, it } from "vitest";
import { computeAssistPlacement, type AssistRect } from "../components/ed/assistPlacement";
import { assistStore, type AssistTarget } from "./assistStore";

const target: AssistTarget = {
  docPath: "notes/a.md",
  loadSeq: 1,
  blockId: "b1",
  start: 0,
  end: 0,
  text: "",
  prefix: "前文",
  before: "前",
  after: "后",
  mode: "cursor",
};

beforeEach(() => {
  assistStore.resetForTest();
});

describe("assistStore latest-wins", () => {
  // covers: BR-AG-STATE-003
  // 默认 450ms 内重复触发应被防抖丢弃，不进入模型调用。
  it("debounces repeated assist triggers", () => {
    expect(assistStore.claimTrigger("autocomplete", 1000)).toBe(true);
    expect(assistStore.claimTrigger("autocomplete", 1200)).toBe(false);
    expect(assistStore.claimTrigger("autocomplete", 1450)).toBe(true);
  });

  // covers: BR-AG-STATE-003
  // 新请求覆盖旧请求：旧 requestId 返回后不能写入候选。
  it("ignores stale autocomplete suggestions after a newer request starts", () => {
    const first = assistStore.openAutocomplete({ x: 1, y: 2 }, target);
    const second = assistStore.startRequest("autocomplete");

    expect(second).toBeTruthy();
    expect(assistStore.isCurrent(first, "autocomplete")).toBe(false);
    assistStore.setSuggestions(first, ["旧候选"]);
    expect(assistStore.getSnapshot().suggestions).toEqual([]);

    assistStore.setSuggestions(second!, ["新候选 1", "新候选 2", "新候选 3", "新候选 4"]);
    expect(assistStore.getSnapshot().suggestions).toEqual(["新候选 1", "新候选 2", "新候选 3"]);
    expect(assistStore.getSnapshot().autocompleteFailureReason).toBeNull();
  });

  // covers: BR-AG-STATE-003
  // 空态必须保留内部 failureReason，不能被统一“暂无可用续写”吞成无原因状态。
  it("keeps autocomplete failure reason for diagnostics", () => {
    const requestId = assistStore.openAutocomplete({ x: 1, y: 2 }, target);

    assistStore.setAutocompleteFailure(requestId, "parse_underflow");

    expect(assistStore.getSnapshot().generating).toBe(false);
    expect(assistStore.getSnapshot().suggestions).toEqual([]);
    expect(assistStore.getSnapshot().autocompleteFailureReason).toBe("parse_underflow");

    assistStore.setSuggestions(requestId, ["候选一", "候选二", "候选三"]);
    expect(assistStore.getSnapshot().autocompleteFailureReason).toBeNull();
  });

  // covers: BR-AG-STATE-003
  // 局部修改 refine 生成新的 requestId，旧预览响应不能覆盖新请求。
  it("replaces inline request id for refine and ignores stale preview", () => {
    const first = assistStore.openInline({ x: 1, y: 2 }, "", target);
    const second = assistStore.startRequest("inline");

    assistStore.setInlinePreview(first, "旧预览");
    expect(assistStore.getSnapshot().inlinePreview).toBeNull();

    assistStore.setInlinePreview(second!, "新预览");
    expect(assistStore.getSnapshot().inlinePreview).toBe("新预览");
  });

  // covers: BR-AG-STATE-003
  // 交互锚失效关闭后，进行中的异步响应不能恢复浮层或写入候选。
  it("drops returned content after the active assist session closes", () => {
    const autocompleteRequest = assistStore.openAutocomplete({ x: 1, y: 2 }, target);
    assistStore.close();
    assistStore.setSuggestions(autocompleteRequest, ["候选一", "候选二", "候选三"]);
    expect(assistStore.getSnapshot().kind).toBe("none");
    expect(assistStore.getSnapshot().suggestions).toEqual([]);

    const inlineRequest = assistStore.openInline({ x: 1, y: 2 }, "", target);
    assistStore.close();
    assistStore.setInlinePreview(inlineRequest, "旧预览");
    expect(assistStore.getSnapshot().kind).toBe("none");
    expect(assistStore.getSnapshot().inlinePreview).toBeNull();
  });

  // covers: BR-AG-STATE-003
  // 滚动/缩放/resize 后只允许当前 requestId 更新视觉锚，不改变逻辑锚或恢复旧请求。
  it("updates only the current visual anchor during reposition", () => {
    const first = assistStore.openAutocomplete({ x: 1, y: 2 }, target);
    const second = assistStore.startRequest("autocomplete");

    assistStore.setAnchor(first, { x: 10, y: 20 });
    expect(assistStore.getSnapshot().anchor).toEqual({ x: 1, y: 2 });

    assistStore.setAnchor(second!, { x: 30, y: 40, width: 420, height: 220 });
    expect(assistStore.getSnapshot().requestId).toBe(second);
    expect(assistStore.getSnapshot().target).toEqual(target);
    expect(assistStore.getSnapshot().anchor).toEqual({ x: 30, y: 40, width: 420, height: 220 });
  });
});

// Migrated intact from assistPlacement.test.ts.
describe("assistPlacement migrated suite", () => {
  const host: AssistRect = { left: 0, top: 0, right: 900, bottom: 700, width: 900, height: 700 };
  const boundary: AssistRect = { left: 100, top: 80, right: 820, bottom: 640, width: 720, height: 560 };

  describe("computeAssistPlacement", () => {
    // covers: BR-AG-STATE-003
    // 辅助续写优先贴近光标下方，不再只把浮层钳制到编辑器边缘。
    it("places autocomplete below the cursor by default", () => {
      const placed = computeAssistPlacement({
        kind: "autocomplete",
        anchorRect: rect(300, 260, 304, 280),
        boundaryRect: boundary,
        hostRect: host,
      });
      expect(placed).toEqual({ x: 300, y: 286, placement: "bottom-start", width: 420, height: 156 });
    });

    // covers: BR-AG-STATE-003
    // 光标靠近底部时辅助续写翻到上方，保持靠近插入点。
    it("flips autocomplete above the cursor near the lower boundary", () => {
      const placed = computeAssistPlacement({
        kind: "autocomplete",
        anchorRect: rect(300, 600, 304, 620),
        boundaryRect: boundary,
        hostRect: host,
      });
      expect(placed.placement).toBe("top-start");
      expect(placed.y).toBe(438);
    });

    // covers: BR-AG-STATE-003
    // 局部修改有选区时优先在选区上方居中。
    it("places inline assist above and centered on the selection", () => {
      const placed = computeAssistPlacement({
        kind: "inline",
        anchorRect: rect(300, 360, 520, 390),
        boundaryRect: boundary,
        hostRect: host,
      });
      expect(placed).toEqual({ x: 230, y: 240, placement: "top-center", width: 360, height: 112 });
    });

    // covers: BR-AG-STATE-003
    // 局部修改初始态使用贴近真实输入态的稳定高度，避免按过大预估值把弹窗推远。
    it("uses a compact inline height to keep the popover adjacent to the selection", () => {
      const placed = computeAssistPlacement({
        kind: "inline",
        anchorRect: rect(300, 360, 520, 390),
        boundaryRect: boundary,
        hostRect: host,
      });
      expect(placed.placement).toBe("top-center");
      expect(placed.x).toBe(230);
      expect(placed.y + placed.height).toBe(352);
    });

    // covers: BR-AG-STATE-003
    // 定位纯函数仍支持显式尺寸输入；如未来恢复测量，不需要新增第二套定位规则。
    it("can keep the top placement adjacent with an explicit inline height", () => {
      const placed = computeAssistPlacement({
        kind: "inline",
        anchorRect: rect(300, 360, 520, 390),
        boundaryRect: boundary,
        hostRect: host,
        floatingSize: { width: 360, height: 180, minWidth: 300, minHeight: 180 },
      });
      expect(placed.placement).toBe("top-center");
      expect(placed.y + placed.height).toBe(352);
    });

    // covers: BR-AG-STATE-003
    // 横向空间不足时以目标边缘做最小偏移，而不是脱离目标贴到编辑器窗口边缘。
    it("uses end alignment when autocomplete would overflow to the right", () => {
      const placed = computeAssistPlacement({
        kind: "autocomplete",
        anchorRect: rect(760, 260, 780, 280),
        boundaryRect: boundary,
        hostRect: host,
      });
      expect(placed.placement).toBe("bottom-end");
      expect(placed.x).toBe(360);
      expect(placed.x + placed.width).toBeLessThanOrEqual(boundary.right - 12);
    });

    // covers: BR-AG-STATE-003
    // 靠近右侧代理编辑面板时，辅助续写应提前改为 end 对齐，避免右下展开后被裁切。
    it("prefers end alignment for autocomplete when close to the right boundary", () => {
      const placed = computeAssistPlacement({
        kind: "autocomplete",
        anchorRect: rect(350, 260, 354, 280),
        boundaryRect: boundary,
        hostRect: host,
      });
      expect(placed.placement).toBe("bottom-end");
      expect(placed.x + placed.width).toBeLessThanOrEqual(boundary.right - 12);
    });

    // covers: BR-AG-STATE-003
    // center/editor-host 可见区比内容区窄时，以传入 boundary 的右边界为硬约束。
    it("keeps the overlay inside a narrow visible boundary", () => {
      const narrowBoundary = rect(100, 80, 560, 640);
      const placed = computeAssistPlacement({
        kind: "inline",
        anchorRect: rect(500, 360, 540, 390),
        boundaryRect: narrowBoundary,
        hostRect: host,
      });
      expect(placed.x + placed.width).toBeLessThanOrEqual(narrowBoundary.right - 12);
    });

    // covers: BR-AG-STATE-003
    // 编辑器缩放只改变锚点视觉坐标；边界来自未缩放 viewport 时，浮层不应随 50% 正文宽度被压扁。
    it("keeps the preferred control width when the document anchor is visually zoomed", () => {
      const placed = computeAssistPlacement({
        kind: "autocomplete",
        anchorRect: rect(260, 220, 262, 230),
        boundaryRect: boundary,
        hostRect: host,
      });
      expect(placed.width).toBe(420);
      expect(placed.placement).toBe("bottom-start");
    });

    // covers: BR-AG-STATE-003
    // 只有宿主真实可用宽度不足时才压缩；仍尽量保持最小可用宽度。
    it("compresses only when the visible host boundary is genuinely narrow", () => {
      const narrowBoundary = rect(100, 80, 450, 640);
      const placed = computeAssistPlacement({
        kind: "autocomplete",
        anchorRect: rect(260, 220, 262, 230),
        boundaryRect: narrowBoundary,
        hostRect: host,
      });
      expect(placed.width).toBe(326);
      expect(placed.width).toBeGreaterThanOrEqual(320);
      expect(placed.x + placed.width).toBeLessThanOrEqual(narrowBoundary.right - 12);
    });
  });

  function rect(left: number, top: number, right: number, bottom: number): AssistRect {
    return { left, top, right, bottom, width: right - left, height: bottom - top };
  }
});
