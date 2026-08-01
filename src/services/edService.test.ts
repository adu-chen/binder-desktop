// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: (path: string) => `asset://localhost/${path}`,
}));
import { invoke } from "@tauri-apps/api/core";
import {
  openDocument,
  restoreSessionSnapshots,
  saveDocxDocument,
  saveDocument,
  saveMarkdownDocument,
} from "./edService";
import { edStore } from "../store/edStore";
import { deStore } from "../store/deStore";
import { wsStore } from "../store/wsStore";

const invokeMock = vi.mocked(invoke);
const ok = <T>(result: T) => ({ correlation_id: "c", result });
const fail = (code: string) => ({ correlation_id: "c", error: { code, message: "失败", retriable: false, reauth_required: false } });

beforeEach(() => {
  invokeMock.mockReset();
  edStore.reset();
  deStore.reset();
  wsStore.reset();
  edStore.loadDoc("a.md", "content", 100);
});

describe("edService 保存契约", () => {
  function addAppliedPendingDiff(relPath = "a.md") {
    deStore.addDiff({
      diffId: "d-save",
      blockId: "b1",
      relPath,
      title: "待保存修改",
      before: "before",
      after: "after",
      state: "pending",
    });
    deStore.markDraftApplied("d-save", "before", "dirty", "after");
  }

  function commandNames() {
    return invokeMock.mock.calls.map(([, args]) => (args as { request: { command: string } }).request.command);
  }

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

  it("保存即接受：先持久化 accepted，再写盘，成功后才显示已保存", async () => {
    addAppliedPendingDiff();
    invokeMock.mockImplementation((_command, args) => {
      const command = (args as { request: { command: string } }).request.command;
      if (command === "de.accept_draft_diff") return Promise.resolve(ok({ accepted: true }));
      if (command === "ws.save_doc") return Promise.resolve(ok({ baseline: 200 }));
      return Promise.resolve(ok({}));
    });

    await expect(saveDocument("new content")).resolves.toEqual({ ok: true });
    expect(commandNames().slice(0, 2)).toEqual(["de.accept_draft_diff", "ws.save_doc"]);
    expect(deStore.getSnapshot().diffs[0]).toMatchObject({ state: "accepted", draftSaved: true });
  });

  it("保存失败后仍保留已接受、待保存，不伪造 draftSaved", async () => {
    addAppliedPendingDiff();
    invokeMock.mockImplementation((_command, args) => {
      const command = (args as { request: { command: string } }).request.command;
      if (command === "de.accept_draft_diff") return Promise.resolve(ok({ accepted: true }));
      if (command === "ws.save_doc") return Promise.resolve(fail("WS_SAVE_CONFLICT"));
      return Promise.resolve(ok({}));
    });

    await expect(saveDocument("new content")).resolves.toMatchObject({ ok: false, conflict: true });
    expect(deStore.getSnapshot().diffs[0]).toMatchObject({ state: "accepted", draftSaved: false });
  });

  it("Markdown 与 DOCX 保存同样先接受已应用 pending Diff", async () => {
    const reply = (_command: unknown, args: unknown) => {
      const command = (args as { request: { command: string } }).request.command;
      if (command === "de.accept_draft_diff") return Promise.resolve(ok({ accepted: true }));
      if (command === "ws.save_doc" || command === "doc.save_from_edit") return Promise.resolve(ok({ baseline: 201 }));
      return Promise.resolve(ok({}));
    };

    addAppliedPendingDiff();
    invokeMock.mockImplementation(reply);
    await expect(saveMarkdownDocument("# disk", "<h1>logical</h1>", "html")).resolves.toEqual({ ok: true });
    expect(commandNames().slice(0, 2)).toEqual(["de.accept_draft_diff", "ws.save_doc"]);

    invokeMock.mockReset();
    edStore.reset();
    deStore.reset();
    edStore.loadDoc("a.draft.docx", "<p>content</p>", 101, null, "html", "html");
    addAppliedPendingDiff("a.draft.docx");
    deStore.attachMeta("d-save", {
      patchKind: "image_ops",
      imageOpsPayload: {
        base: { sessionVersion: 1, contentHash: "h1" },
        operations: [{
          op: "resize_image",
          imageId: "image-stable",
          fingerprint: "fp-current",
          width: 240,
        }],
      },
    });
    invokeMock.mockImplementation(reply);
    await expect(saveDocxDocument("<p>saved</p>")).resolves.toEqual({ ok: true });
    expect(commandNames().slice(0, 2)).toEqual(["de.accept_draft_diff", "doc.save_from_edit"]);
    expect(deStore.getSnapshot().diffs[0]).toMatchObject({
      patchKind: "image_ops",
      state: "accepted",
      draftSaved: true,
    });
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
    invokeMock.mockResolvedValue(ok({
      format: "docx",
      html: "<p>h</p>",
      baseline: 22,
      style_report: { notices: ["字体样式需要复核"] },
    }));
    await openDocument("notes/y.draft.docx");
    const s = edStore.getSnapshot();
    expect(s).toMatchObject({ openPath: "notes/y.draft.docx", editFormat: "html", loadedContent: "<p>h</p>", baseline: 22 });
    expect(s.styleNotices).toEqual(["字体样式需要复核"]);
    expect(vi.mocked(invoke).mock.calls[0][1]).toMatchObject({ request: { command: "doc.open_for_edit" } });
    expect(vi.mocked(invoke).mock.calls[1][1]).toMatchObject({
      request: {
        command: "ed.persist_session_snapshot",
        params: {
          snapshot: {
            relPath: "notes/y.draft.docx",
            content: "<p>h</p>",
            baseline: 22,
            editFormat: "html",
            dirty: false,
          },
        },
      },
    });
  });

  it("后端登记的 BDocx 普通 .docx → doc.open_for_edit HTML 模型", async () => {
    edStore.reset();
    wsStore._projectDocs([{ relPath: "notes/generated.docx", docType: "docx", isDir: false, isBDocx: true }]);
    invokeMock.mockResolvedValue(ok({ format: "docx", html: "<p>h</p>", baseline: 23, style_report: { notices: [] } }));
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
        params: {
          snapshot: {
            relPath: "notes/generated.docx",
            content: "<p>h</p>",
            baseline: 23,
            editFormat: "html",
            dirty: false,
          },
        },
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
    await expect(openDocument("gone.md")).rejects.toMatchObject({ code: "WS_NOT_FOUND" });
    expect(edStore.getSnapshot().sessions["gone.md"]).toBeUndefined();
    expect(edStore.getSnapshot().openPath).toBeNull();
  });

  it("迟到的打开结果只补全原会话，不能抢回当前编辑器", async () => {
    edStore.reset();
    const pending = new Map<string, (value: unknown) => void>();
    invokeMock.mockImplementation((_command, args) => {
      const request = (args as { request: { command: string; params: { relPath?: string } } }).request;
      if (request.command === "ed.persist_session_snapshot") return Promise.resolve(ok({}));
      if (request.command === "ws.read_doc") {
        return new Promise((resolve) => {
          pending.set(request.params.relPath ?? "", resolve);
        });
      }
      throw new Error(`unexpected command ${request.command}`);
    });

    const first = openDocument("first.md");
    const second = openDocument("second.md");
    pending.get("second.md")?.(ok({ content: "第二份", baseline: 2 }));
    await second;
    pending.get("first.md")?.(ok({ content: "第一份", baseline: 1 }));
    await first;

    const state = edStore.getSnapshot();
    expect(state).toMatchObject({ openPath: "second.md", loadedContent: "<p>第二份</p>" });
    expect(state.sessions["first.md"]).toMatchObject({ content: "<p>第一份</p>", baseline: 1 });
  });
});

describe("edService BDocx 持久会话媒体恢复", () => {
  it("先读取规范快照，再按原 baseline 恢复媒体显示地址", async () => {
    edStore.reset();
    invokeMock.mockImplementation((_command, args) => {
      const request = (args as { request: { command: string } }).request;
      if (request.command === "ed.list_session_snapshots") {
        return Promise.resolve(ok({
          snapshots: [{
            relPath: "notes/image.draft.docx",
            content:
              '<p><img src="media/image1.png" data-binder-media-src="media/image1.png"></p>',
            baseline: "81",
            editFormat: "html",
            contentFormat: "html",
            source: "open_editor_session",
            dirty: true,
            sessionVersion: 3,
            contentHash: "h",
            capturedAt: 1,
          }],
        }));
      }
      if (request.command === "doc.restore_edit_media") {
        return Promise.resolve(ok({ baseline: "81", media_base: "/cache/81" }));
      }
      throw new Error(`unexpected command ${request.command}`);
    });

    const snapshots = await restoreSessionSnapshots();
    expect(snapshots[0]).toMatchObject({ baseline: "81", dirty: true });
    expect(snapshots[0].content).toContain("asset://localhost//cache/81/media/image1.png");
    expect(snapshots[0].content).not.toContain("binder-docx-media-baseline");
  });

  it("无图片 BDocx 同样恢复持久 baseline，且不做空媒体授权调用", async () => {
    invokeMock.mockResolvedValue(ok({
      snapshots: [{
        relPath: "notes/text-only.draft.docx",
        content: "<p>正文</p>",
        baseline: "91",
        editFormat: "html",
        contentFormat: "html",
        source: "open_editor_session",
        dirty: true,
        sessionVersion: 2,
        contentHash: "h2",
        capturedAt: 1,
      }],
    }));
    const snapshots = await restoreSessionSnapshots();
    expect(snapshots[0]).toMatchObject({
      baseline: "91",
      content: "<p>正文</p>",
    });
    expect(invokeMock).toHaveBeenCalledTimes(1);
  });

  it("旧 dirty 快照缺 baseline 时不冒用当前磁盘基线恢复图片", async () => {
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => undefined);
    invokeMock.mockResolvedValue(ok({
      snapshots: [{
        relPath: "notes/legacy.draft.docx",
        content: '<p><img src="media/image1.png"></p>',
        editFormat: "html",
        contentFormat: "html",
        source: "open_editor_session",
        dirty: true,
        sessionVersion: 2,
        contentHash: "legacy",
        capturedAt: 1,
      }],
    }));
    const snapshots = await restoreSessionSnapshots();
    expect(snapshots[0].baseline).toBeNull();
    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(consoleError).toHaveBeenCalled();
    consoleError.mockRestore();
  });
});
