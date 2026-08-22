/** @vitest-environment jsdom */
import { beforeEach, describe, expect, it, vi } from "vitest";
import { Schema } from "@tiptap/pm/model";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: (path: string) => `asset://localhost/${path}`,
}));

import { invoke } from "@tauri-apps/api/core";
import {
  DOC_REF_MIME,
  SELECTION_REF_MIME,
  bindActiveEditorDocument,
  blockTextLength,
  charOffsetInBlock,
  getActiveEditorBinding,
  findTopBlockSnapshotById,
  getTopBlockIdByIndex,
  getTopBlockSnapshotByIndex,
  isActiveEditorBoundTo,
  readDocRefData,
  readSelectionRefData,
  registerEditor,
  resolveSelectionAnchor,
  writeDocRefData,
  writeSelectionRefData,
} from "./edLocate";
import { REFERENCE_MIME } from "./referencePayload";
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

// Migrated intact from src/services/edLocate.test.ts
{
// covers: BR-ED-DATA-004
// 字符锚纯逻辑：偏移定义在块内纯文本平面（textBetween 语义，叶子内联 1 字符占位对齐 PM pos）；
// 单块选区精确区间；跨块选区锚定首块（截首块文本尾）；折叠/无 blockId → null。
// 用 prosemirror-model 最小 schema 构造真实 doc（不依赖 TipTap 运行时/jsdom）。

const schema = new Schema({
  nodes: {
    doc: { content: "block+" },
    paragraph: {
      group: "block",
      content: "inline*",
      attrs: { blockId: { default: null } },
    },
    hardBreak: { group: "inline", inline: true }, // 叶子内联（nodeSize=1）
    text: { group: "inline" },
  },
});

beforeEach(() => registerEditor(null));

/** 构造段落（可含 hardBreak 叶子）。 */
function p(blockId: string | null, ...parts: (string | "BR")[]) {
  const content = parts.map((x) => (x === "BR" ? schema.node("hardBreak") : schema.text(x)));
  return schema.node("paragraph", { blockId }, content);
}

function transfer() {
  const values = new Map<string, string>();
  return {
    values,
    getData(type: string) {
      return values.get(type) ?? "";
    },
    setData(type: string, data: string) {
      values.set(type, data);
    },
  };
}

describe("edLocate · 块内纯文本偏移（BR-ED-DATA-004 偏移平面）", () => {
  it("纯文本块：偏移 = 文本位置（含中文）", () => {
    const block = p("b1", "你好 world");
    expect(blockTextLength(block)).toBe(8); // 你好+空格+world
    expect(charOffsetInBlock(block, 0)).toBe(0);
    expect(charOffsetInBlock(block, 2)).toBe(2); // "你好" 后
    expect(charOffsetInBlock(block, 8)).toBe(8);
  });

  it("叶子内联节点（hardBreak）以 1 字符占位对齐 pos", () => {
    const block = p("b1", "ab", "BR", "cd"); // pos: a(0)b(1) BR(2) c(3)d(4)
    expect(blockTextLength(block)).toBe(5); // "ab￼cd"
    expect(charOffsetInBlock(block, 3)).toBe(3); // BR 之后 = 偏移 3
  });

  it("越界钳制（不抛错，安全上界）", () => {
    const block = p("b1", "abc");
    expect(charOffsetInBlock(block, 99)).toBe(3);
    expect(charOffsetInBlock(block, -5)).toBe(0);
  });
});

describe("edLocate · resolveSelectionAnchor", () => {
  // doc: [p1 "第一段落"(4字)] [p2 "第二段"(3字)]；p1 内容 pos 1..5，p2 内容 pos 7..10
  const doc = schema.node("doc", null, [p("b1", "第一段落"), p("b2", "第二段")]);

  it("单块选区 → 精确区间", () => {
    // 选 "一段"（p1 内偏移 1..3）：doc pos 2..4
    expect(resolveSelectionAnchor(doc, 2, 4)).toEqual({ blockId: "b1", start: 1, end: 3 });
  });

  it("整块选区 → 0..文本长", () => {
    expect(resolveSelectionAnchor(doc, 1, 5)).toEqual({ blockId: "b1", start: 0, end: 4 });
  });

  it("跨块选区 → 锚定首块并截到首块文本尾", () => {
    // 从 p1 偏移 2 跨到 p2 中部
    expect(resolveSelectionAnchor(doc, 3, 9)).toEqual({ blockId: "b1", start: 2, end: 4 });
  });

  it("折叠选区 → null；无 blockId 块 → null", () => {
    expect(resolveSelectionAnchor(doc, 3, 3)).toBeNull();
    const noId = schema.node("doc", null, [p(null, "无锚段")]);
    expect(resolveSelectionAnchor(noId, 1, 3)).toBeNull();
  });

  it("第二块内选区 → 正确归属 b2 且偏移按块内计", () => {
    // p2 内容 pos 从 7 开始；选 "二段"（p2 偏移 1..3）→ doc pos 8..10
    expect(resolveSelectionAnchor(doc, 8, 10)).toEqual({ blockId: "b2", start: 1, end: 3 });
  });
});

describe("edLocate · block_index → blockId", () => {
  const doc = schema.node("doc", null, [p("b1", "第一段落"), p("b2", "第二段"), p(null, "无锚段")]);

  it("按 0 起块号返回对应稳定 blockId", () => {
    expect(getTopBlockIdByIndex(doc, 0)).toBe("b1");
    expect(getTopBlockIdByIndex(doc, 1)).toBe("b2");
  });

  it("越界、负数、无 blockId 块不产生错位锚", () => {
    expect(getTopBlockIdByIndex(doc, -1)).toBeNull();
    expect(getTopBlockIdByIndex(doc, 3)).toBeNull();
    expect(getTopBlockIdByIndex(doc, 2)).toBeNull();
  });

  it("按 block_index / blockId 返回同一目标块快照，before 文本来自该块", () => {
    const byIndex = getTopBlockSnapshotByIndex(doc, 1);
    expect(byIndex).toMatchObject({ blockId: "b2", index: 1, text: "第二段" });
    expect(byIndex?.contentFrom).toBeGreaterThan(byIndex?.pos ?? -1);
    expect(byIndex?.contentTo).toBeGreaterThan(byIndex?.contentFrom ?? -1);

    expect(findTopBlockSnapshotById(doc, "b2")).toEqual(byIndex);
    expect(getTopBlockSnapshotByIndex(doc, 3)).toBeNull();
    expect(findTopBlockSnapshotById(doc, "missing")).toBeNull();
  });
});

describe("edLocate · active editor binding", () => {
  it("记录并校验 ProseMirror 当前承载的文档身份", () => {
    registerEditor(null);
    bindActiveEditorDocument({ relPath: "notes/a.md", loadSeq: 3, editFormat: "text" });

    expect(getActiveEditorBinding()).toEqual({ relPath: "notes/a.md", loadSeq: 3, editFormat: "text" });
    expect(isActiveEditorBoundTo("notes/a.md", 3, "text")).toBe(true);
    expect(isActiveEditorBoundTo("notes/b.md", 3, "text")).toBe(false);
    expect(isActiveEditorBoundTo("notes/a.md", 4, "text")).toBe(false);
    expect(isActiveEditorBoundTo("notes/a.md", 3, "html")).toBe(false);
  });

  it("registerEditor(null) 清空旧绑定，避免卸载后继续信任旧正文", () => {
    bindActiveEditorDocument({ relPath: "notes/a.md", loadSeq: 1, editFormat: "text" });
    registerEditor(null);

    expect(getActiveEditorBinding()).toBeNull();
    expect(isActiveEditorBoundTo("notes/a.md", 1, "text")).toBe(false);
  });
});

describe("edLocate · 引用拖拽/粘贴数据契约", () => {
  it("写入并读取文档引用 MIME，同时提供 text/plain 兜底文本", () => {
    const dt = transfer();

    writeDocRefData(dt, "notes/a.md");

    expect(dt.values.get(DOC_REF_MIME)).toBe(JSON.stringify({ relPath: "notes/a.md" }));
    expect(JSON.parse(dt.values.get(REFERENCE_MIME) ?? "{}")).toEqual({ kind: "doc", relPath: "notes/a.md" });
    expect(dt.values.get("text/plain")).toBe("notes/a.md");
    expect(readDocRefData(dt)).toEqual({ relPath: "notes/a.md" });
  });

  it("忽略空路径或损坏的文档引用 payload", () => {
    const dt = transfer();
    dt.setData(DOC_REF_MIME, JSON.stringify({ relPath: "" }));
    expect(readDocRefData(dt)).toBeNull();

    dt.setData(DOC_REF_MIME, "{");
    expect(readDocRefData(dt)).toBeNull();
  });

  it("写入并读取 SelectionRef MIME，同时提供选区文本兜底", () => {
    const dt = transfer();
    const ref = {
      path: "notes/a.md",
      blockId: "block-1",
      start: 2,
      end: 5,
      text: "选中的内容",
    };

    writeSelectionRefData(dt, ref);

    expect(dt.values.get(SELECTION_REF_MIME)).toBe(JSON.stringify(ref));
    expect(JSON.parse(dt.values.get(REFERENCE_MIME) ?? "{}")).toEqual({
      kind: "selection",
      relPath: "notes/a.md",
      blockId: "block-1",
      start: 2,
      end: 5,
      text: "选中的内容",
    });
    expect(dt.values.get("text/plain")).toBe("选中的内容");
    expect(readSelectionRefData(dt)).toEqual(ref);
  });

  it("忽略字段不完整或损坏的 SelectionRef payload", () => {
    const dt = transfer();
    dt.setData(SELECTION_REF_MIME, JSON.stringify({ path: "notes/a.md", blockId: "b1" }));
    expect(readSelectionRefData(dt)).toBeNull();

    dt.setData(SELECTION_REF_MIME, "{");
    expect(readSelectionRefData(dt)).toBeNull();
  });
});
}
// Migrated intact from src/services/edService.test.ts
{
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
  function addAppliedPendingDiff(relPath = "a.md", after = "after") {
    deStore.addDiff({
      diffId: "d-save",
      blockId: "b1",
      relPath,
      title: "待保存修改",
      before: "before",
      after,
      state: "pending",
    });
    deStore.markDraftApplied("d-save", "before", "dirty", after);
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

  // covers: BR-CORE-STATE-001
  it("保存即接受：先持久化 accepted，再写盘，成功后才显示已保存", async () => {
    addAppliedPendingDiff();
    invokeMock.mockImplementation((_command, args) => {
      const command = (args as { request: { command: string } }).request.command;
      if (command === "de.accept_draft_diff") return Promise.resolve(ok({ accepted: true }));
      if (command === "ws.save_doc") return Promise.resolve(ok({ baseline: 200 }));
      return Promise.resolve(ok({}));
    });

    await expect(saveDocument("after")).resolves.toEqual({ ok: true });
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

    await expect(saveDocument("after")).resolves.toMatchObject({ ok: false, conflict: true });
    expect(commandNames().slice(0, 2)).toEqual(["de.accept_draft_diff", "ws.save_doc"]);
    expect(deStore.getSnapshot().diffs[0]).toMatchObject({ state: "accepted", draftSaved: false });
  });

  // covers: BR-CORE-STATE-001
  it("保存当前手动编辑时使旧 pending Diff 失效，而非把旧卡误裁决为接受", async () => {
    addAppliedPendingDiff();
    invokeMock.mockImplementation((_command, args) => {
      const command = (args as { request: { command: string } }).request.command;
      if (command === "de.expire_diff") return Promise.resolve(ok({ status: "expired" }));
      if (command === "ws.save_doc") return Promise.resolve(ok({ baseline: 200 }));
      return Promise.resolve(ok({}));
    });

    await expect(saveDocument("用户在预览后继续修改")).resolves.toEqual({ ok: true });

    expect(commandNames().slice(0, 2)).toEqual(["de.expire_diff", "ws.save_doc"]);
    expect(commandNames()).not.toContain("de.accept_draft_diff");
    expect(deStore.getSnapshot().diffs[0]).toMatchObject({
      state: "expired",
      expiredReason: "document_revision_advanced",
    });
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
    await expect(saveMarkdownDocument("# disk", "after", "html")).resolves.toEqual({ ok: true });
    expect(commandNames().slice(0, 2)).toEqual(["de.accept_draft_diff", "ws.save_doc"]);

    invokeMock.mockReset();
    edStore.reset();
    deStore.reset();
    edStore.loadDoc("a.draft.docx", "<p>content</p>", 101, null, "html", "html");
    addAppliedPendingDiff("a.draft.docx", "<p>saved</p>");
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
    const errorSpy = vi.spyOn(console, "error").mockImplementation(() => undefined);
    invokeMock.mockResolvedValue(fail("WS_NOT_FOUND"));
    await expect(openDocument("gone.md")).rejects.toMatchObject({ code: "WS_NOT_FOUND" });
    expect(edStore.getSnapshot().sessions["gone.md"]).toBeUndefined();
    expect(edStore.getSnapshot().openPath).toBeNull();
    expect(errorSpy).toHaveBeenCalledWith("[ed-document-open]", expect.objectContaining({
      event: "document_open_failed",
      rel_path: "gone.md",
      error_code: "WS_NOT_FOUND",
    }));
    errorSpy.mockRestore();
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
}
