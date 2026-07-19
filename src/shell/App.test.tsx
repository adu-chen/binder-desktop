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

vi.mock("../services/workspaceService", () => workspaceServiceMock);
vi.mock("./Welcome", () => ({ Welcome: () => null }));
vi.mock("./Workbench", () => ({ Workbench: () => null }));

import { App } from "./App";

beforeEach(() => {
  windowMock.handler = null;
  windowMock.unlisten.mockClear();
  windowMock.onCloseRequested.mockClear();
  windowMock.close.mockClear();
  windowMock.destroy.mockClear();
  workspaceServiceMock.restoreActiveWorkspace.mockReset();
  workspaceServiceMock.flushEditorStateBeforeExit.mockReset();
  workspaceServiceMock.restoreActiveWorkspace.mockResolvedValue(undefined);
  workspaceServiceMock.flushEditorStateBeforeExit.mockResolvedValue(undefined);
});

describe("App 退出前编辑器状态持久化", () => {
  // covers: BR-ED-STATE-001
  // 窗口关闭前必须 flush 编辑器打开标签与 session 镜像，避免启动恢复读取到旧打开集合。
  it("prevents first close request, flushes editor state, then unhooks and closes window", async () => {
    const { unmount } = render(<App />);
    await waitFor(() => expect(windowMock.onCloseRequested).toHaveBeenCalled());

    const event = { preventDefault: vi.fn() };
    await windowMock.handler?.(event);

    expect(event.preventDefault).toHaveBeenCalled();
    expect(workspaceServiceMock.flushEditorStateBeforeExit).toHaveBeenCalled();
    expect(windowMock.unlisten).toHaveBeenCalled();
    expect(windowMock.close).toHaveBeenCalled();
    expect(windowMock.destroy).not.toHaveBeenCalled();

    const secondEvent = { preventDefault: vi.fn() };
    await windowMock.handler?.(secondEvent);
    expect(secondEvent.preventDefault).toHaveBeenCalled();
    expect(workspaceServiceMock.flushEditorStateBeforeExit).toHaveBeenCalledTimes(1);
    expect(windowMock.close).toHaveBeenCalledTimes(1);

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
