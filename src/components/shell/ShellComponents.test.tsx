/** @vitest-environment jsdom */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import defaultCapability from "../../../src-tauri/capabilities/default.json";
import { App } from "../../shell/App";
import { CloseConfirmDialog } from "./CloseConfirmDialog";
import { TabStrip } from "./TabStrip";
import { DOC_REF_MIME } from "../../services/edLocate";
import { REFERENCE_MIME } from "../../services/referencePayload";
import { closeStore } from "../../store/closeStore";
import { deStore } from "../../store/deStore";
import { edStore } from "../../store/edStore";
import { tabsStore } from "../../store/tabsStore";

// Migrated intact from src/shell/App.test.tsx.
{
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

vi.mock("../../services/workspaceService", () => workspaceServiceMock);
vi.mock("../../shell/Welcome", () => ({ Welcome: () => null }));
vi.mock("../../shell/Workbench", () => ({ Workbench: () => null }));


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
}

// Migrated intact from src/components/shell/CloseConfirmDialog.test.tsx.
{
const closeServiceMock = vi.hoisted(() => ({
  cancelClose: vi.fn(),
  submitDecision: vi.fn(),
  forceQuit: vi.fn(),
}));
vi.mock("../../services/closeService", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../../services/closeService")>();
  return {
    ...actual,
    cancelClose: closeServiceMock.cancelClose,
    submitDecision: closeServiceMock.submitDecision,
    forceQuit: closeServiceMock.forceQuit,
  };
});

beforeEach(() => {
  closeStore.reset();
  closeServiceMock.cancelClose.mockReset();
  closeServiceMock.submitDecision.mockReset();
  closeServiceMock.forceQuit.mockReset();
});

afterEach(() => {
  cleanup();
  closeStore.reset();
});

describe("CloseConfirmDialog（BR-SYS-STATE-004 关闭前处理弹窗）", () => {
  // covers: BR-SYS-STATE-004
  it("idle 态不渲染", () => {
    const { container } = render(<CloseConfirmDialog />);
    expect(container.firstChild).toBeNull();
  });

  // covers: BR-SYS-STATE-004
  it("closing 态不渲染（已进入实际关闭流程，不应再打断）", () => {
    closeStore.setClosing();
    const { container } = render(<CloseConfirmDialog />);
    expect(container.firstChild).toBeNull();
  });

  // covers: BR-SYS-STATE-004
  it("awaitingDecision：未选择前确认按钮禁用", () => {
    closeStore.setAwaitingDecision(2, [{ relPath: "a.md", hasAcceptedAwaitingSave: false }]);
    render(<CloseConfirmDialog />);

    expect((screen.getByRole("button", { name: "关闭应用" }) as HTMLButtonElement).disabled).toBe(true);
  });

  // covers: BR-SYS-STATE-004（2026-07-20 人类决策 2：接受全部强制耦合保存）
  it("选择接受全部后，未保存文件的放弃选项被禁用且保存被强制选中", () => {
    closeStore.setAwaitingDecision(2, [{ relPath: "a.md", hasAcceptedAwaitingSave: false }]);
    render(<CloseConfirmDialog />);

    fireEvent.click(screen.getByRole("button", { name: /接受全部/ }));

    const saveRadio = screen.getByLabelText(/保存修改/) as HTMLInputElement;
    const discardRadio = screen.getByLabelText(/放弃修改/) as HTMLInputElement;
    expect(saveRadio.checked).toBe(true);
    expect(discardRadio.disabled).toBe(true);
    expect((screen.getByRole("button", { name: "关闭应用" }) as HTMLButtonElement).disabled).toBe(false);
  });

  // covers: BR-SYS-STATE-004
  it("拒绝全部 + 放弃修改 → 提交 rejectAllAndDiscard", () => {
    closeStore.setAwaitingDecision(1, [{ relPath: "a.md", hasAcceptedAwaitingSave: false }]);
    render(<CloseConfirmDialog />);

    fireEvent.click(screen.getByRole("button", { name: /拒绝全部/ }));
    fireEvent.click(screen.getByLabelText(/放弃修改/));
    fireEvent.click(screen.getByRole("button", { name: "关闭应用" }));

    expect(closeServiceMock.submitDecision).toHaveBeenCalledWith("rejectAllAndDiscard");
  });

  // covers: BR-SYS-STATE-004
  it("只有未保存文件、无 diff 时不展示 AI 修改分区", () => {
    closeStore.setAwaitingDecision(0, [{ relPath: "a.md", hasAcceptedAwaitingSave: false }]);
    render(<CloseConfirmDialog />);

    expect(screen.queryByText("AI 修改")).toBeNull();
    expect(screen.getByText("未保存文件")).toBeTruthy();
  });

  // covers: BR-SYS-STATE-006（2026-07-20 人类决策 3：强制退出仅在失败后作为显式兜底出现）
  it("无失败时不展示强制退出；失败后展示且可触发", () => {
    closeStore.setAwaitingDecision(1, []);
    const { rerender } = render(<CloseConfirmDialog />);
    expect(screen.queryByText(/强制退出/)).toBeNull();

    closeStore.setFailed("保存失败：磁盘冲突");
    rerender(<CloseConfirmDialog />);
    const forceBtn = screen.getByRole("button", { name: /强制退出/ });
    fireEvent.click(forceBtn);
    expect(closeServiceMock.forceQuit).toHaveBeenCalled();
  });

  // covers: BR-SYS-STATE-004
  it("取消按钮调用 cancelClose", () => {
    closeStore.setAwaitingDecision(1, []);
    render(<CloseConfirmDialog />);
    fireEvent.click(screen.getByRole("button", { name: "取消" }));
    expect(closeServiceMock.cancelClose).toHaveBeenCalled();
  });
});
}

// Migrated intact from src/components/shell/TabStrip.test.tsx.
{
function dragData() {
  const values = new Map<string, string>();
  return {
    values,
    dataTransfer: {
      getData: vi.fn((type: string) => values.get(type) ?? ""),
      setData: vi.fn((type: string, value: string) => values.set(type, value)),
    },
  };
}

beforeEach(() => {
  Object.defineProperty(globalThis, "CSS", {
    value: { escape: (value: string) => value },
    configurable: true,
  });
  Object.defineProperty(window.HTMLElement.prototype, "scrollIntoView", {
    value: vi.fn(),
    configurable: true,
  });
  tabsStore.reset();
  edStore.reset();
  deStore.reset();
});

afterEach(cleanup);

describe("TabStrip · 文档引用拖出", () => {
  it("标签拖出时写入旧文档引用 MIME 与统一 ReferencePayload MIME", () => {
    tabsStore.openDoc("notes/a.md");
    const { container } = render(<TabStrip />);
    const tab = container.querySelector("[data-tab-id]") as HTMLElement;
    const { values, dataTransfer } = dragData();

    fireEvent.dragStart(tab, { dataTransfer });

    expect(JSON.parse(values.get(DOC_REF_MIME) ?? "{}")).toEqual({ relPath: "notes/a.md" });
    expect(JSON.parse(values.get(REFERENCE_MIME) ?? "{}")).toEqual({ kind: "doc", relPath: "notes/a.md" });
    expect(values.get("text/plain")).toBe("notes/a.md");
  });
});
}
