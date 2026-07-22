/** @vitest-environment jsdom */
import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, waitFor } from "@testing-library/react";
import defaultCapability from "../../src-tauri/capabilities/default.json";

const windowMock = vi.hoisted(() => {
  const state: {
    handler: ((event: { preventDefault: () => void }) => void | Promise<void>) | null;
    unlisten: ReturnType<typeof vi.fn>;
    onCloseRequested: ReturnType<typeof vi.fn>;
    close: ReturnType<typeof vi.fn>;
    destroy: ReturnType<typeof vi.fn>;
  } = {
    handler: null,
    unlisten: vi.fn(),
    onCloseRequested: vi.fn(),
    close: vi.fn(),
    destroy: vi.fn(),
  };
  state.onCloseRequested.mockImplementation(async (handler: typeof state.handler) => {
    state.handler = handler;
    return state.unlisten;
  });
  state.close.mockResolvedValue(undefined);
  state.destroy.mockResolvedValue(undefined);
  return state;
});

const tauriMock = vi.hoisted(() => ({ invoke: vi.fn() }));

const eventMock = vi.hoisted(() => {
  const state: {
    handler: (() => void) | null;
    unlisten: ReturnType<typeof vi.fn>;
    listen: ReturnType<typeof vi.fn>;
  } = {
    handler: null,
    unlisten: vi.fn(),
    listen: vi.fn(),
  };
  state.listen.mockImplementation(async (_eventName: string, handler: typeof state.handler) => {
    state.handler = handler;
    return state.unlisten;
  });
  return state;
});

const workspaceServiceMock = vi.hoisted(() => ({
  restoreActiveWorkspace: vi.fn(),
  flushEditorStateBeforeExit: vi.fn(),
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    onCloseRequested: windowMock.onCloseRequested,
    close: windowMock.close,
    destroy: windowMock.destroy,
  }),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke: tauriMock.invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: eventMock.listen }));

vi.mock("../services/workspaceService", () => workspaceServiceMock);
vi.mock("./Welcome", () => ({ Welcome: () => null }));
vi.mock("./Workbench", () => ({ Workbench: () => null }));

import { App } from "./App";
import { closeStore } from "../store/closeStore";

const ok = <T,>(result: T) => ({ correlation_id: "app-test", result });

function mockInspectClose(pendingDiffCount: number, dirtyDocuments: { relPath: string; hasAcceptedAwaitingSave: boolean }[]) {
  tauriMock.invoke.mockImplementation(async (command: string, arg: unknown) => {
    if (command !== "dispatch") return undefined;
    const request = (arg as { request: { command: string } }).request;
    if (request.command === "sys.inspect_close") {
      return ok({ pendingDiffCount, dirtyDocuments });
    }
    return ok({});
  });
}

beforeEach(() => {
  windowMock.handler = null;
  windowMock.unlisten.mockClear();
  windowMock.onCloseRequested.mockClear();
  windowMock.close.mockClear();
  windowMock.destroy.mockClear();
  eventMock.handler = null;
  eventMock.unlisten.mockClear();
  eventMock.listen.mockClear();
  tauriMock.invoke.mockReset();
  workspaceServiceMock.restoreActiveWorkspace.mockReset();
  workspaceServiceMock.flushEditorStateBeforeExit.mockReset();
  workspaceServiceMock.restoreActiveWorkspace.mockResolvedValue(undefined);
  workspaceServiceMock.flushEditorStateBeforeExit.mockResolvedValue(undefined);
  closeStore.reset();
  mockInspectClose(0, []);
});

describe("App 关闭编排入口（APP-CLOSE-DIFF-LIFECYCLE-01）", () => {
  // covers: BR-SYS-STATE-004
  // 无残留 pending/dirty 时：先 flush 编辑器状态再清点，清点后再次 flush 并关闭窗口；窗口再次触发的
  // CloseRequested（closeService 内部调用 appWindow.close() 所致）不得再次拦截，否则应用永远关不掉。
  it("clean close: inspects via sys.inspect_close, flushes, and lets the internal re-close proceed unblocked", async () => {
    const { unmount } = render(<App />);
    await waitFor(() => expect(windowMock.onCloseRequested).toHaveBeenCalled());

    const event = { preventDefault: vi.fn() };
    await windowMock.handler?.(event);
    await waitFor(() => expect(windowMock.close).toHaveBeenCalled());

    expect(event.preventDefault).toHaveBeenCalled();
    expect(workspaceServiceMock.flushEditorStateBeforeExit).toHaveBeenCalled();
    expect(windowMock.close).toHaveBeenCalledTimes(1);
    expect(closeStore.getSnapshot().phase).toBe("closing");

    // closeService 内部调用 appWindow.close() 会再次触发 CloseRequested；此时 phase 已是
    // "closing"，App 的处理器必须直接放行（不 preventDefault），否则窗口永远无法真正关闭。
    const secondEvent = { preventDefault: vi.fn() };
    await windowMock.handler?.(secondEvent);
    expect(secondEvent.preventDefault).not.toHaveBeenCalled();
    // 首次用于让 inspect_close 看到最新数据库快照，第二次是实际关窗前的最终 flush。
    expect(workspaceServiceMock.flushEditorStateBeforeExit).toHaveBeenCalledTimes(2);
    expect(windowMock.close).toHaveBeenCalledTimes(1); // 未重复触发关闭编排

    unmount();
  });

  // covers: BR-SYS-STATE-004
  // 存在残留 pending diff 时：不得直接关闭，必须停在 awaitingDecision 呈现决策入口。
  it("residual pending diff: does not close the window, enters awaitingDecision instead", async () => {
    mockInspectClose(2, []);
    const { unmount } = render(<App />);
    await waitFor(() => expect(windowMock.onCloseRequested).toHaveBeenCalled());

    const event = { preventDefault: vi.fn() };
    await windowMock.handler?.(event);
    await waitFor(() => expect(closeStore.getSnapshot().phase).toBe("awaitingDecision"));

    expect(event.preventDefault).toHaveBeenCalled();
    expect(windowMock.close).not.toHaveBeenCalled();
    expect(closeStore.getSnapshot().pendingDiffCount).toBe(2);

    unmount();
  });

  // covers: BR-SYS-STATE-004
  it("Cmd+Q/Dock 的后端退出事件也进入同一决策入口，不直接关闭窗口", async () => {
    mockInspectClose(1, []);
    const { unmount } = render(<App />);
    await waitFor(() => expect(eventMock.listen).toHaveBeenCalledWith("sys:exit-requested", expect.any(Function)));

    eventMock.handler?.();
    await waitFor(() => expect(closeStore.getSnapshot().phase).toBe("awaitingDecision"));

    expect(windowMock.close).not.toHaveBeenCalled();
    expect(closeStore.getSnapshot().pendingDiffCount).toBe(1);
    unmount();
  });

  // covers: BR-ED-STATE-001
  // Tauri v2 capability 必须授权 close/destroy，否则真实窗口关闭会被拦截后卡住。
  it("declares Tauri permissions required by the close persistence hook", () => {
    const capability = defaultCapability as { permissions: string[] };

    expect(capability.permissions).toContain("core:window:allow-close");
    expect(capability.permissions).toContain("core:window:allow-destroy");
  });
});
