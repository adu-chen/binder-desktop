import { beforeEach, describe, expect, it, vi } from "vitest";

const tauriMock = vi.hoisted(() => ({ invoke: vi.fn() }));
const agServiceMock = vi.hoisted(() => ({ hydrateAgentHistory: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke: tauriMock.invoke }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
vi.mock("./agService", () => agServiceMock);

import { invoke } from "@tauri-apps/api/core";
import { edStore } from "../store/edStore";
import { tabsStore } from "../store/tabsStore";
import { workspaceShell } from "../store/workspaceShell";
import { wsStore } from "../store/wsStore";
import { restoreActiveWorkspace } from "./workspaceService";

const invokeMock = vi.mocked(invoke);

const ok = <T>(result: T) => ({ correlation_id: "ws-test", result });

beforeEach(() => {
  invokeMock.mockReset();
  agServiceMock.hydrateAgentHistory.mockReset();
  workspaceShell._projectActive(null);
  wsStore.reset();
  edStore.reset();
  tabsStore.reset();
});

describe("workspaceService 编辑器启动恢复", () => {
  // covers: BR-ED-STATE-001
  // ed_open_tabs 是打开标签真源；孤立 ed_session_snapshots 不得被恢复为打开文档。
  it("restoreActiveWorkspace restores only persisted open tabs and aligns current document", async () => {
    invokeMock.mockImplementation(async (command, arg) => {
      if (command !== "dispatch") return undefined;
      const request = (arg as { request: { command: string } }).request;
      switch (request.command) {
        case "ws.active_workspace":
          return ok({ workspace_id: "ws1", root_path: "/tmp/ws", is_active: true });
        case "ws.list_docs":
          return ok([]);
        case "ed.list_open_tabs":
          return ok({
            tabs: [
              { relPath: "docs/a.md", kind: "doc", openIndex: 0, isActive: false, updatedAt: 1 },
              { relPath: "docs/b.md", kind: "doc", openIndex: 1, isActive: true, updatedAt: 2 },
            ],
          });
        case "ed.list_session_snapshots":
          return ok({
            snapshots: [
              {
                relPath: "docs/a.md",
                content: "A",
                editFormat: "text",
                source: "open_editor_session",
                dirty: false,
                sessionVersion: 1,
                contentHash: "ha",
                capturedAt: 1,
              },
              {
                relPath: "docs/b.md",
                content: "B",
                editFormat: "text",
                source: "open_editor_session",
                dirty: true,
                sessionVersion: 2,
                contentHash: "hb",
                capturedAt: 2,
              },
              {
                relPath: "docs/stale.md",
                content: "STALE",
                editFormat: "text",
                source: "open_editor_session",
                dirty: false,
                sessionVersion: 1,
                contentHash: "hs",
                capturedAt: 3,
              },
            ],
          });
        default:
          return ok({ ok: true });
      }
    });

    await restoreActiveWorkspace();

    const tabs = tabsStore.getSnapshot();
    expect(tabs.tabs.map((tab) => tab.path)).toEqual(["docs/a.md", "docs/b.md"]);
    expect(tabs.tabs.find((tab) => tab.id === tabs.activeTabId)?.path).toBe("docs/b.md");
    expect(edStore.getSnapshot()).toMatchObject({
      openPath: "docs/b.md",
      loadedContent: "B",
      tabState: "dirty",
    });
    expect(edStore.getSnapshot().sessions["docs/stale.md"]).toBeUndefined();
    expect(agServiceMock.hydrateAgentHistory).toHaveBeenCalled();
  });

  it("并发启动恢复复用同一轮后端读取，避免重复恢复编辑会话", async () => {
    let resolveActive: ((value: unknown) => void) | undefined;
    invokeMock.mockImplementation((command, arg) => {
      if (command !== "dispatch") return Promise.resolve(undefined);
      const request = (arg as { request: { command: string } }).request;
      if (request.command === "ws.active_workspace") {
        return new Promise((resolve) => { resolveActive = resolve; });
      }
      if (request.command === "ws.list_docs") return Promise.resolve(ok([]));
      if (request.command === "ed.list_open_tabs") return Promise.resolve(ok({ tabs: [] }));
      if (request.command === "ed.list_session_snapshots") return Promise.resolve(ok({ snapshots: [] }));
      return Promise.resolve(ok({ ok: true }));
    });

    const first = restoreActiveWorkspace();
    const second = restoreActiveWorkspace();
    expect(invokeMock.mock.calls.filter(([, arg]) => (
      (arg as { request: { command: string } }).request.command === "ws.active_workspace"
    ))).toHaveLength(1);

    resolveActive?.(ok({ workspace_id: "ws1", root_path: "/tmp/ws", is_active: true }));
    await Promise.all([first, second]);

    expect(agServiceMock.hydrateAgentHistory).toHaveBeenCalledTimes(1);
  });
});
