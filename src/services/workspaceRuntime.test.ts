import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { edStore } from "../store/edStore";
import { tabsStore } from "../store/tabsStore";
import { workspaceShell } from "../store/workspaceShell";
import { wsStore } from "../store/wsStore";
import { restoreActiveWorkspace } from "./workspaceService";
import { createDoc, createFolder, renameDoc, deleteDoc, fetchDocs, saveDoc } from "./wsService";

// Migrated intact from src/services/workspaceService.test.ts.
{
const tauriMock = vi.hoisted(() => ({ invoke: vi.fn() }));
const agServiceMock = vi.hoisted(() => ({ hydrateAgentHistory: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke: tauriMock.invoke }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
vi.mock("./agService", () => agServiceMock);


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
}

// Migrated intact from src/services/wsService.test.ts.
{
// mock 跨进程入口：服务层不真触后端，只验信封构造与 result/error 互斥。

const invokeMock = vi.mocked(invoke);
const ok = <T>(result: T) => ({ correlation_id: "c", result });
const fail = (code: string) => ({ correlation_id: "c", error: { code, message: "x", retriable: false, reauth_required: false } });

beforeEach(() => invokeMock.mockReset());

describe("wsService dispatch 信封契约（X-SEQ-002）", () => {
  it("createDoc 构造 ws.create_doc 信封并回 rel_path", async () => {
    invokeMock.mockResolvedValue(ok({ rel_path: "notes/a.md" }));
    const rel = await createDoc("notes/a.md", "hi");
    expect(rel).toBe("notes/a.md");
    const [cmd, arg] = invokeMock.mock.calls[0];
    expect(cmd).toBe("dispatch");
    expect((arg as { request: { command: string; params: unknown } }).request).toMatchObject({
      command: "ws.create_doc",
      params: { relPath: "notes/a.md", content: "hi" },
    });
  });

  // covers: BR-WS-DATA-001
  it("createFolder 构造 ws.create_folder 信封并回 rel_path", async () => {
    invokeMock.mockResolvedValue(ok({ rel_path: "notes", created: true, existed: false }));
    const rel = await createFolder("notes");
    expect(rel).toBe("notes");
    expect(vi.mocked(invoke).mock.calls[0][1]).toMatchObject({
      request: { command: "ws.create_folder", params: { relPath: "notes" } },
    });
  });

  it("renameDoc / deleteDoc 命令名与参数正确", async () => {
    invokeMock.mockResolvedValue(ok({ rel_path: "b.md" }));
    await renameDoc("a.md", "b.md");
    expect(vi.mocked(invoke).mock.calls[0][1]).toMatchObject({ request: { command: "ws.rename_doc", params: { relPath: "a.md", newRelPath: "b.md" } } });
    invokeMock.mockResolvedValue(ok({ deleted: true }));
    await deleteDoc("b.md");
    expect(vi.mocked(invoke).mock.calls[1][1]).toMatchObject({ request: { command: "ws.delete_doc", params: { relPath: "b.md" } } });
  });

  it("error 信封 → 抛出 IpcError（result/error 互斥）", async () => {
    invokeMock.mockResolvedValue(fail("WS_EXISTS"));
    await expect(createDoc("a.md")).rejects.toMatchObject({ code: "WS_EXISTS" });
  });

  // covers: BR-WS-OBS-001
  it("saveDoc 传递加载基线与显式覆盖标志，供后端保存前冲突保护", async () => {
    invokeMock.mockResolvedValue(ok({ baseline: "456" }));
    await expect(saveDoc("a.md", "new", "123", true)).resolves.toBe("456");
    expect(vi.mocked(invoke).mock.calls[0][1]).toMatchObject({
      request: {
        command: "ws.save_doc",
        params: { relPath: "a.md", content: "new", baseline: "123", override: true },
      },
    });
  });

  it("fetchDocs 映射后端 is_bdocx 身份", async () => {
    invokeMock.mockResolvedValue(ok([{ rel_path: "generated.docx", doc_type: "docx", is_dir: false, is_bdocx: true }]));
    await expect(fetchDocs()).resolves.toEqual([
      { relPath: "generated.docx", docType: "docx", isDir: false, isBDocx: true },
    ]);
  });
});
}
