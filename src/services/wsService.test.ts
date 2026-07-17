import { beforeEach, describe, expect, it, vi } from "vitest";

// mock 跨进程入口：服务层不真触后端，只验信封构造与 result/error 互斥。
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
import { invoke } from "@tauri-apps/api/core";
import { createDoc, renameDoc, deleteDoc, fetchDocs, saveDoc } from "./wsService";

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
