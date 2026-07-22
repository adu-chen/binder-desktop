import { beforeEach, describe, expect, it } from "vitest";
import { closeStore } from "./closeStore";

/** covers: BR-SYS-STATE-004 — SM-SYS-02 前端投影的基础转移语义（供 CloseConfirmDialog 渲染）。 */
describe("closeStore（SM-SYS-02 前端投影）", () => {
  beforeEach(() => {
    closeStore.reset();
  });

  it("初始态为 idle，无残留计数", () => {
    const s = closeStore.getSnapshot();
    expect(s.phase).toBe("idle");
    expect(s.pendingDiffCount).toBe(0);
    expect(s.dirtyDocuments).toEqual([]);
    expect(s.busy).toBe(false);
    expect(s.error).toBeNull();
  });

  it("setChecking → checking 且 busy，清除旧错误", () => {
    closeStore.setFailed("旧错误");
    closeStore.setChecking();
    const s = closeStore.getSnapshot();
    expect(s.phase).toBe("checking");
    expect(s.busy).toBe(true);
    expect(s.error).toBeNull();
  });

  it("setAwaitingDecision 携带计数且 busy=false", () => {
    closeStore.setAwaitingDecision(2, [{ relPath: "a.md", hasAcceptedAwaitingSave: true }]);
    const s = closeStore.getSnapshot();
    expect(s.phase).toBe("awaitingDecision");
    expect(s.pendingDiffCount).toBe(2);
    expect(s.dirtyDocuments).toEqual([{ relPath: "a.md", hasAcceptedAwaitingSave: true }]);
    expect(s.busy).toBe(false);
  });

  it("setResolving → resolving 且 busy", () => {
    closeStore.setResolving();
    const s = closeStore.getSnapshot();
    expect(s.phase).toBe("resolving");
    expect(s.busy).toBe(true);
  });

  it("setFailed 回 awaitingDecision 且 busy=false，不静默丢失失败原因", () => {
    closeStore.setResolving();
    closeStore.setFailed("保存冲突");
    const s = closeStore.getSnapshot();
    expect(s.phase).toBe("awaitingDecision");
    expect(s.busy).toBe(false);
    expect(s.error).toBe("保存冲突");
  });

  it("setClosing → closing 且 busy", () => {
    closeStore.setClosing();
    const s = closeStore.getSnapshot();
    expect(s.phase).toBe("closing");
    expect(s.busy).toBe(true);
  });

  it("reset 恢复初始态（取消关闭场景）", () => {
    closeStore.setAwaitingDecision(3, []);
    closeStore.reset();
    expect(closeStore.getSnapshot().phase).toBe("idle");
  });

  it("订阅者在状态变化时收到通知", () => {
    let notified = 0;
    const unsubscribe = closeStore.subscribe(() => {
      notified += 1;
    });
    closeStore.setChecking();
    closeStore.setAwaitingDecision(1, []);
    unsubscribe();
    closeStore.setResolving();
    expect(notified).toBe(2); // 取消订阅后的变化不再计数
  });
});
