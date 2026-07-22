import { beforeEach, describe, expect, it, vi } from "vitest";

const tauriMock = vi.hoisted(() => ({ invoke: vi.fn() }));
const windowMock = vi.hoisted(() => ({ close: vi.fn(), destroy: vi.fn() }));
const workspaceServiceMock = vi.hoisted(() => ({ flushEditorStateBeforeExit: vi.fn() }));
const edLocateMock = vi.hoisted(() => ({
  triggerActiveSave: vi.fn(),
  getActiveEditorBinding: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: tauriMock.invoke }));
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => windowMock }));
vi.mock("./workspaceService", () => workspaceServiceMock);
vi.mock("./edLocate", () => edLocateMock);

import { invoke } from "@tauri-apps/api/core";
import { closeStore } from "../store/closeStore";
import { deStore } from "../store/deStore";
import { edStore } from "../store/edStore";
import { tabsStore } from "../store/tabsStore";
import {
  requestClose,
  cancelClose,
  submitDecision,
  forceQuit,
  hasUnsavedOrPendingWork,
} from "./closeService";

const invokeMock = vi.mocked(invoke);
const ok = <T,>(result: T) => ({ correlation_id: "close-test", result });

function commandOf(call: unknown): string {
  const [, arg] = call as [string, { request: { command: string } }];
  return arg.request.command;
}

beforeEach(() => {
  invokeMock.mockReset();
  windowMock.close.mockReset();
  windowMock.destroy.mockReset();
  windowMock.close.mockResolvedValue(undefined);
  windowMock.destroy.mockResolvedValue(undefined);
  workspaceServiceMock.flushEditorStateBeforeExit.mockReset();
  workspaceServiceMock.flushEditorStateBeforeExit.mockResolvedValue(undefined);
  edLocateMock.triggerActiveSave.mockReset();
  edLocateMock.getActiveEditorBinding.mockReset();
  closeStore.reset();
  deStore.reset();
  edStore.reset();
  tabsStore.reset();
});

describe("closeService.hasUnsavedOrPendingWork（聚合脏态，修复原仅查激活文档的漏检）", () => {
  it("非激活文档 dirty 也应被判定为有未保存修改", () => {
    edStore.loadDoc("a.md", "内容A", 1);
    tabsStore.openDoc("a.md");
    edStore.loadDoc("b.md", "内容B", 1); // 切换为激活文档
    tabsStore.openDoc("b.md");
    edStore.setTabState("a.md", "dirty"); // 非激活文档变脏
    expect(edStore.getSnapshot().activePath).toBe("b.md");
    expect(hasUnsavedOrPendingWork()).toBe(true);
  });

  it("有 pending diff 时应判定为有未处理工作", () => {
    deStore.addDiff({ diffId: "d1", blockId: "", relPath: "a.md", title: "t", before: "", after: "", state: "pending" });
    expect(hasUnsavedOrPendingWork()).toBe(true);
  });

  it("无脏态无 pending 时为 false", () => {
    expect(hasUnsavedOrPendingWork()).toBe(false);
  });
});

describe("closeService.requestClose（BR-SYS-STATE-004 清点）", () => {
  it("pending=0 且 dirty=0：直接放行，flush 并关窗", async () => {
    invokeMock.mockImplementation(async (command, arg) => {
      if (command !== "dispatch") return undefined;
      const req = (arg as { request: { command: string } }).request;
      if (req.command === "sys.inspect_close") return ok({ pendingDiffCount: 0, dirtyDocuments: [] });
      return ok({});
    });

    await requestClose();

    expect(workspaceServiceMock.flushEditorStateBeforeExit).toHaveBeenCalled();
    expect(windowMock.close).toHaveBeenCalled();
    expect(closeStore.getSnapshot().phase).toBe("closing");
  });

  it("存在残留：进入 awaitingDecision，不关窗", async () => {
    invokeMock.mockImplementation(async (command, arg) => {
      if (command !== "dispatch") return undefined;
      const req = (arg as { request: { command: string } }).request;
      if (req.command === "sys.inspect_close") {
        return ok({ pendingDiffCount: 1, dirtyDocuments: [{ relPath: "a.md", hasAcceptedAwaitingSave: false }] });
      }
      return ok({});
    });

    await requestClose();

    expect(windowMock.close).not.toHaveBeenCalled();
    const snap = closeStore.getSnapshot();
    expect(snap.phase).toBe("awaitingDecision");
    expect(snap.pendingDiffCount).toBe(1);
    expect(snap.dirtyDocuments).toEqual([{ relPath: "a.md", hasAcceptedAwaitingSave: false }]);
  });

  it("仅有 awaiting_save 效果也必须阻断关闭", async () => {
    invokeMock.mockImplementation(async (command, arg) => {
      if (command !== "dispatch") return undefined;
      const req = (arg as { request: { command: string } }).request;
      if (req.command === "sys.inspect_close") {
        return ok({ pendingDiffCount: 0, awaitingSaveCount: 1, dirtyDocuments: [] });
      }
      return ok({});
    });

    await requestClose();

    expect(windowMock.close).not.toHaveBeenCalled();
    expect(closeStore.getSnapshot().phase).toBe("awaitingDecision");
    expect(closeStore.getSnapshot().awaitingSaveCount).toBe(1);
  });

  it("检查前 flush 失败时拒绝关闭，且不使用陈旧数据库快照", async () => {
    workspaceServiceMock.flushEditorStateBeforeExit.mockRejectedValueOnce({ code: "ED_PERSIST", message: "磁盘错误" });

    await requestClose();

    expect(invokeMock).not.toHaveBeenCalled();
    expect(windowMock.close).not.toHaveBeenCalled();
    expect(closeStore.getSnapshot().phase).toBe("awaitingDecision");
    expect(closeStore.getSnapshot().error).toContain("保存编辑状态失败");
  });

  it("查询失败：不放行关闭，进入 awaitingDecision 并展示错误", async () => {
    invokeMock.mockImplementation(async (command, arg) => {
      if (command !== "dispatch") return undefined;
      const req = (arg as { request: { command: string } }).request;
      if (req.command === "sys.inspect_close") return { correlation_id: "x", error: { code: "WS_DB_ERROR", message: "boom" } };
      return ok({});
    });

    await requestClose();

    expect(windowMock.close).not.toHaveBeenCalled();
    expect(closeStore.getSnapshot().phase).toBe("awaitingDecision");
    expect(closeStore.getSnapshot().error).toBeTruthy();
  });
});

describe("closeService.cancelClose", () => {
  it("撤销屏障回到 idle", () => {
    closeStore.setAwaitingDecision(1, []);
    cancelClose();
    expect(closeStore.getSnapshot().phase).toBe("idle");
  });
});

describe("closeService.submitDecision（BR-SYS-STATE-004 决策 + 复核）", () => {
  it("rejectAllAndDiscard：拒绝全部 diff + 显式放弃未保存内容，复核清零后关闭", async () => {
    deStore.addDiff({ diffId: "d1", blockId: "", relPath: "a.md", title: "t", before: "", after: "", state: "pending" });
    closeStore.setAwaitingDecision(1, [{ relPath: "a.md", hasAcceptedAwaitingSave: false }]);

    let inspectCalls = 0;
    invokeMock.mockImplementation(async (command, arg) => {
      if (command !== "dispatch") return undefined;
      const req = (arg as { request: { command: string } }).request;
      if (req.command === "de.reject_diff") return ok({});
      if (req.command === "ws.discard_unsaved_sessions") return ok({ discardedDirtySnapshots: 1, discardedAwaitingSaveDiffs: 0 });
      if (req.command === "sys.inspect_close") {
        inspectCalls += 1;
        return ok({ pendingDiffCount: 0, dirtyDocuments: [] }); // 复核：真实清零
      }
      return ok({});
    });

    await submitDecision("rejectAllAndDiscard");

    const commands = invokeMock.mock.calls.map(commandOf);
    expect(commands).toContain("de.reject_diff");
    expect(commands).toContain("ws.discard_unsaved_sessions");
    expect(inspectCalls).toBe(1); // 复核确实重新查询了一次
    expect(windowMock.close).toHaveBeenCalled();
    expect(closeStore.getSnapshot().phase).toBe("closing");
  });

  it("复核发现仍有残留：回 awaitingDecision 展示失败，不放行关闭（不信任发起决策时刻的计数）", async () => {
    deStore.addDiff({ diffId: "d1", blockId: "", relPath: "a.md", title: "t", before: "", after: "", state: "pending" });
    closeStore.setAwaitingDecision(1, []);

    invokeMock.mockImplementation(async (command, arg) => {
      if (command !== "dispatch") return undefined;
      const req = (arg as { request: { command: string } }).request;
      if (req.command === "de.reject_diff") return ok({});
      if (req.command === "sys.inspect_close") {
        // 复核仍发现残留（例如屏障期间竞态产生了新 diff）。
        return ok({ pendingDiffCount: 1, dirtyDocuments: [] });
      }
      return ok({});
    });

    await submitDecision("rejectAllAndDiscard");

    expect(windowMock.close).not.toHaveBeenCalled();
    const snap = closeStore.getSnapshot();
    expect(snap.phase).toBe("awaitingDecision");
    expect(snap.error).toBeTruthy();
    expect(snap.pendingDiffCount).toBe(1);
  });

  it("保存失败：展示失败原因，不放行关闭", async () => {
    edStore.loadDoc("a.md", "内容", 1);
    tabsStore.openDoc("a.md");
    edStore.setTabState("a.md", "dirty");
    closeStore.setAwaitingDecision(0, [{ relPath: "a.md", hasAcceptedAwaitingSave: false }]);

    edLocateMock.getActiveEditorBinding.mockReturnValue({ relPath: "a.md", loadSeq: 1, editFormat: "text" });
    edLocateMock.triggerActiveSave.mockResolvedValue({ ok: false, message: "磁盘冲突" });

    await submitDecision("rejectAllAndSave");

    expect(windowMock.close).not.toHaveBeenCalled();
    const snap = closeStore.getSnapshot();
    expect(snap.phase).toBe("awaitingDecision");
    expect(snap.error).toContain("磁盘冲突");
  });

  it("忽略非 awaitingDecision 态或忙碌态的重复提交", async () => {
    expect(closeStore.getSnapshot().phase).toBe("idle");
    await submitDecision("rejectAllAndDiscard");
    expect(invokeMock).not.toHaveBeenCalled();
  });
});

describe("closeService.forceQuit（2026-07-20 人类决策 3，BR-SYS-STATE-006）", () => {
  // covers: BR-SYS-STATE-006
  it("不查询清点、不等待裁决，直接 flush 并关窗", async () => {
    await forceQuit();

    expect(invokeMock).not.toHaveBeenCalled(); // 未调用 sys.inspect_close
    expect(workspaceServiceMock.flushEditorStateBeforeExit).toHaveBeenCalled();
    expect(windowMock.close).toHaveBeenCalled();
  });
});
