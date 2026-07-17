import { beforeEach, describe, expect, it } from "vitest";
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
