// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
import { invoke } from "@tauri-apps/api/core";
import { saveDocument, openDocument } from "./edService";
import { edStore } from "../store/edStore";
import { wsStore } from "../store/wsStore";

const invokeMock = vi.mocked(invoke);
const ok = <T>(result: T) => ({ correlation_id: "c", result });
const fail = (code: string) => ({ correlation_id: "c", error: { code, message: "失败", retriable: false, reauth_required: false } });

beforeEach(() => {
  invokeMock.mockReset();
  edStore.reset();
  wsStore.reset();
  edStore.loadDoc("a.md", "content", 100);
});

describe("edService 保存契约", () => {
  // covers: BR-WS-OBS-001
  it("保存成功 → setBaseline 刷新基线；saved 态由 SM-WS-01 广播投影", async () => {
    invokeMock.mockResolvedValue(ok({ baseline: 200 }));
    edStore.markDirty();
    expect(edStore.getSnapshot().tabState).toBe("dirty");
    const r = await saveDocument("new content");
    expect(r).toEqual({ ok: true });
    expect(edStore.getSnapshot()).toMatchObject({ baseline: 200, tabState: "dirty" });
    edStore.applyWsSaveState("a.md", "Saved");
    expect(edStore.getSnapshot().tabState).toBe("saved");
    expect(vi.mocked(invoke).mock.calls[0][1]).toMatchObject({
      request: {
        command: "ws.save_doc",
        params: { relPath: "a.md", content: "new content", baseline: 100, override: false },
      },
    });
    expect(vi.mocked(invoke).mock.calls[1][1]).toMatchObject({
      request: {
        command: "ed.persist_session_snapshot",
        params: {
          snapshot: {
            relPath: "a.md",
            content: "new content",
            editFormat: "text",
            dirty: false,
          },
        },
      },
    });
  });

  // covers: BR-ED-STATE-003
  it("保存冲突 → 保持 dirty 并返回 conflict，等待用户显式覆盖或重载", async () => {
    invokeMock.mockResolvedValue(fail("WS_SAVE_CONFLICT"));
    const r = await saveDocument("content");
    expect(r).toMatchObject({ ok: false, conflict: true });
    expect(edStore.getSnapshot().tabState).toBe("dirty");
  });

  it("保存失败 → dirty 态（不谎称已存）", async () => {
    invokeMock.mockResolvedValue(fail("WS_IO_ERROR"));
    const r = await saveDocument("content");
    expect(r).toMatchObject({ ok: false });
    expect(edStore.getSnapshot().tabState).toBe("dirty");
  });

  it("无打开文档 → 不保存", async () => {
    edStore.reset();
    const r = await saveDocument("x");
    expect(r).toMatchObject({ ok: false });
    expect(invokeMock).not.toHaveBeenCalled();
  });
});

describe("edService openDocument 路由（BR-ED-DATA-002 md/docx 统一编辑模型）", () => {
  // covers: BR-ED-DATA-002
  it("md → Markdown Codec 终态模型（editFormat=markdown/contentFormat=html）", async () => {
    edStore.reset();
    invokeMock.mockResolvedValue(ok({ content: "# md", baseline: 11 }));
    await openDocument("notes/x.md");
    const s = edStore.getSnapshot();
    expect(s).toMatchObject({ openPath: "notes/x.md", editFormat: "markdown", contentFormat: "html", baseline: 11 });
    expect(vi.mocked(invoke).mock.calls[0][1]).toMatchObject({ request: { command: "ws.read_doc" } });
    expect(vi.mocked(invoke).mock.calls[1][1]).toMatchObject({
      request: {
        command: "ed.persist_session_snapshot",
        params: { snapshot: { relPath: "notes/x.md", editFormat: "markdown", contentFormat: "html", dirty: false } },
      },
    });
  });

  it("*.draft.docx → doc.open_for_edit HTML 模型（editFormat=html）", async () => {
    edStore.reset();
    invokeMock.mockResolvedValue(ok({ format: "docx", html: "<p>h</p>", baseline: 22 }));
    await openDocument("notes/y.draft.docx");
    const s = edStore.getSnapshot();
    expect(s).toMatchObject({ openPath: "notes/y.draft.docx", editFormat: "html", loadedContent: "<p>h</p>", baseline: 22 });
    expect(vi.mocked(invoke).mock.calls[0][1]).toMatchObject({ request: { command: "doc.open_for_edit" } });
    expect(vi.mocked(invoke).mock.calls[1][1]).toMatchObject({
      request: {
        command: "ed.persist_session_snapshot",
        params: { snapshot: { relPath: "notes/y.draft.docx", content: "<p>h</p>", editFormat: "html", dirty: false } },
      },
    });
  });

  it("后端登记的 BDocx 普通 .docx → doc.open_for_edit HTML 模型", async () => {
    edStore.reset();
    wsStore._projectDocs([{ relPath: "notes/generated.docx", docType: "docx", isDir: false, isBDocx: true }]);
    invokeMock.mockResolvedValue(ok({ format: "docx", html: "<p>h</p>", baseline: 23 }));
    await openDocument("notes/generated.docx");
    expect(edStore.getSnapshot()).toMatchObject({
      openPath: "notes/generated.docx",
      editFormat: "html",
      loadedContent: "<p>h</p>",
      baseline: 23,
    });
    expect(vi.mocked(invoke).mock.calls[0][1]).toMatchObject({ request: { command: "doc.open_for_edit" } });
    expect(vi.mocked(invoke).mock.calls[1][1]).toMatchObject({
      request: {
        command: "ed.persist_session_snapshot",
        params: { snapshot: { relPath: "notes/generated.docx", content: "<p>h</p>", editFormat: "html", dirty: false } },
      },
    });
  });

  it(".html 独立文件 → ws.read_doc 富文本模型（editFormat=html_file）", async () => {
    edStore.reset();
    invokeMock.mockResolvedValue(ok({ content: '<p data-block-id="b1">页面</p>', baseline: 33 }));
    await openDocument("site/index.html");
    const s = edStore.getSnapshot();
    expect(s).toMatchObject({ openPath: "site/index.html", editFormat: "html_file", baseline: 33 });
    expect(vi.mocked(invoke).mock.calls[0][1]).toMatchObject({ request: { command: "ws.read_doc" } });
    expect(vi.mocked(invoke).mock.calls[1][1]).toMatchObject({
      request: {
        command: "ed.persist_session_snapshot",
        params: {
          snapshot: {
            relPath: "site/index.html",
            content: '<p data-block-id="b1">页面</p>',
            editFormat: "html_file",
            dirty: false,
          },
        },
      },
    });
  });

  it("打开失败 → 撤销占位会话（不留假空文档）", async () => {
    edStore.reset();
    invokeMock.mockResolvedValue(fail("WS_NOT_FOUND"));
    await openDocument("gone.md");
    expect(edStore.getSnapshot().sessions["gone.md"]).toBeUndefined();
    expect(edStore.getSnapshot().openPath).toBeNull();
  });
});
