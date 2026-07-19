import { beforeEach, describe, expect, it, vi } from "vitest";

const tauriMock = vi.hoisted(() => ({ invoke: vi.fn() }));
const edServiceMock = vi.hoisted(() => ({ openDocument: vi.fn(), persistSessionSnapshot: vi.fn() }));
const edLocateMock = vi.hoisted(() => ({
  applyEditOps: vi.fn(),
  getActiveDocumentContent: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: tauriMock.invoke }));
vi.mock("./edService", () => edServiceMock);
vi.mock("./edLocate", () => edLocateMock);

import { invoke } from "@tauri-apps/api/core";
import { deStore } from "../store/deStore";
import { edStore } from "../store/edStore";
import { acceptDiff, applyOpenSessionDiff, generateSampleDiff, rejectDiff } from "./deService";

const invokeMock = vi.mocked(invoke);

beforeEach(() => {
  invokeMock.mockReset();
  edServiceMock.openDocument.mockReset();
  edServiceMock.persistSessionSnapshot.mockReset();
  edServiceMock.persistSessionSnapshot.mockResolvedValue(undefined);
  edLocateMock.applyEditOps.mockReset();
  edLocateMock.applyEditOps.mockReturnValue({ ok: true });
  edLocateMock.getActiveDocumentContent.mockReset();
  edLocateMock.getActiveDocumentContent.mockReturnValue(null);
  deStore.reset();
  edStore.reset();
});

function lastCommand(): string {
  const calls = invokeMock.mock.calls;
  const [, arg] = calls[calls.length - 1] as [string, { request: { command: string } }];
  return arg.request.command;
}

describe("deService 草稿 diff 裁决", () => {
  // covers: BR-ED-STATE-001
  it("generateSampleDiff：以当前编辑器 LogicalState 为 before，不退回 stale 缓存", async () => {
    edStore.loadDoc("a.md", "缓存内容", 1);
    edLocateMock.getActiveDocumentContent.mockReturnValue("当前编辑器内容");
    invokeMock.mockResolvedValue({ result: { diff_id: "d-sample" } });

    await generateSampleDiff();

    const [, arg] = invokeMock.mock.calls[0] as [string, { request: { command: string; params: { afterContent: string } } }];
    expect(arg.request.command).toBe("de.generate_diff");
    expect(arg.request.params.afterContent).toContain("当前编辑器内容");
    expect(arg.request.params.afterContent).not.toContain("缓存内容");
  });

  // covers: BR-DE-STATE-001, BR-ED-STATE-001
  it("acceptDiff：当前编辑器草稿 diff 只确认 accepted，不调用旧落盘重开路径", async () => {
    edStore.loadDoc("a.md", "旧", 1);
    deStore.addDiff({
      diffId: "d1",
      blockId: "b1",
      relPath: "a.md",
      title: "改第二段",
      before: "旧",
      after: "新",
      state: "pending",
      draftApplied: true,
    });
    invokeMock.mockResolvedValue({ result: { accepted: true } });

    await acceptDiff("d1");

    expect(lastCommand()).toBe("de.accept_draft_diff");
    expect(edServiceMock.openDocument).not.toHaveBeenCalled();
  });

  it("acceptDiff：整篇草稿 diff 也只确认 accepted，不走旧落盘重开路径", async () => {
    edStore.loadDoc("a.docx", "<p>新</p>", 1, null, "html");
    deStore.addDiff({
      diffId: "d-whole",
      blockId: "",
      relPath: "a.docx",
      title: "整篇改写",
      before: "<p>旧</p>",
      after: "<p>新</p>",
      state: "pending",
      draftApplied: true,
      draftBaseContent: "<p>旧</p>",
      draftBaseTabState: "saved",
    });
    invokeMock.mockResolvedValue({ result: { accepted: true } });

    await acceptDiff("d-whole");

    expect(lastCommand()).toBe("de.accept_draft_diff");
    expect(edServiceMock.openDocument).not.toHaveBeenCalled();
  });

  it("acceptDiff：未打开文档保留 de.accept_diff 后台路径", async () => {
    deStore.addDiff({
      diffId: "d2",
      blockId: "",
      relPath: "a.md",
      title: "整篇",
      before: "旧",
      after: "新",
      state: "pending",
    });
    invokeMock.mockResolvedValue({ result: { entry_id: "e1", version_seq: 3 } });

    await acceptDiff("d2");

    expect(lastCommand()).toBe("de.accept_diff");
    expect(edServiceMock.openDocument).not.toHaveBeenCalled();
  });

  it("acceptDiff：已打开文档非草稿路径不走后台 accept_diff", async () => {
    edStore.loadDoc("a.docx", "<p>旧</p>", 1, null, "html");
    deStore.addDiff({
      diffId: "d-bdocx-bg",
      blockId: "b1",
      relPath: "a.docx",
      title: "后台 bdocx 修改",
      before: "旧",
      after: "新",
      state: "pending",
    });

    await acceptDiff("d-bdocx-bg");

    expect(invokeMock).not.toHaveBeenCalled();
    expect(deStore.getSnapshot().diffs[0]).toMatchObject({
      state: "error",
      errorCode: "DE_OPEN_DOC_REQUIRES_LOGICAL_STATE",
    });
  });

  it("applyOpenSessionDiff：已打开非激活文档直接应用 LogicalState 并确认 accepted", async () => {
    edStore.loadDoc("target.md", "一\n旧", 1);
    edStore.loadDoc("active.md", "当前", 2);
    const target = edStore.getSnapshot().sessions["target.md"];
    deStore.addDiff({
      diffId: "d-open",
      blockId: "",
      blockIndex: 1,
      relPath: "target.md",
      title: "修改非激活文档",
      before: "旧",
      after: "新",
      state: "pending",
      patchKind: "replace_block",
      draftMode: "open_session",
      snapshotSource: "open_editor_session",
      baseSessionVersion: target.sessionVersion,
      baseContentHash: target.contentHash,
      readonly: true,
    });
    invokeMock.mockResolvedValue({ result: { accepted: true } });

    await applyOpenSessionDiff("d-open");

    expect(edStore.getSnapshot().sessions["target.md"]).toMatchObject({
      content: "一\n新",
      tabState: "dirty",
      sessionVersion: target.sessionVersion + 1,
    });
    expect(edServiceMock.persistSessionSnapshot).toHaveBeenCalledWith(expect.objectContaining({
      relPath: "target.md",
      content: "一\n新",
      source: "open_editor_session",
      dirty: true,
    }));
    expect(lastCommand()).toBe("de.accept_draft_diff");
    expect(deStore.getSnapshot().diffs[0]).toMatchObject({
      state: "accepted",
      readonly: true,
      openSessionApplied: true,
    });
  });

  it("applyOpenSessionDiff：目标 LogicalState 已推进时过期，不覆盖新内容", async () => {
    edStore.loadDoc("target.md", "一\n旧", 1);
    edStore.loadDoc("active.md", "当前", 2);
    const target = edStore.getSnapshot().sessions["target.md"];
    edStore.updateContent("target.md", "一\n用户新改");
    deStore.addDiff({
      diffId: "d-open-stale",
      blockId: "",
      blockIndex: 1,
      relPath: "target.md",
      title: "修改非激活文档",
      before: "旧",
      after: "新",
      state: "pending",
      patchKind: "replace_block",
      draftMode: "open_session",
      snapshotSource: "open_editor_session",
      baseSessionVersion: target.sessionVersion,
      baseContentHash: target.contentHash,
      readonly: true,
    });
    invokeMock.mockResolvedValue({ result: { diff_id: "d-open-stale", status: "expired" } });

    await applyOpenSessionDiff("d-open-stale");

    expect(edStore.getSnapshot().sessions["target.md"]?.content).toBe("一\n用户新改");
    expect(edServiceMock.persistSessionSnapshot).not.toHaveBeenCalled();
    expect(lastCommand()).toBe("de.expire_diff");
    expect(deStore.getSnapshot().diffs[0]).toMatchObject({
      state: "expired",
      errorCode: "ED_OPEN_SESSION_BASE_MISMATCH",
    });
  });

  it("rejectDiff：当前编辑器草稿 diff 后端拒绝后回滚目标块 before", async () => {
    edStore.loadDoc("a.md", "新", 1);
    edStore.markDirty();
    deStore.addDiff({
      diffId: "d3",
      blockId: "b1",
      relPath: "a.md",
      title: "改第二段",
      before: "旧",
      after: "新",
      state: "pending",
      draftApplied: true,
      draftBaseContent: "旧",
      draftBaseTabState: "saved",
    });
    invokeMock.mockResolvedValue({ result: { rejected: true } });
    edLocateMock.getActiveDocumentContent.mockReturnValue("旧");

    await rejectDiff("d3");

    expect(lastCommand()).toBe("de.reject_diff");
    expect(edLocateMock.applyEditOps).toHaveBeenCalledWith([{ op: "replace_block", blockId: "b1", text: "旧" }]);
    expect(edStore.getSnapshot()).toMatchObject({ loadedContent: "旧", tabState: "saved" });
  });

  it("rejectDiff：回滚后若完整内容仍不同于草稿前快照，则保留 dirty", async () => {
    edStore.loadDoc("a.md", "新", 1);
    edStore.markDirty();
    deStore.addDiff({
      diffId: "d4",
      blockId: "b1",
      relPath: "a.md",
      title: "改第二段",
      before: "旧",
      after: "新",
      state: "pending",
      draftApplied: true,
      draftBaseContent: "旧",
      draftBaseTabState: "saved",
    });
    invokeMock.mockResolvedValue({ result: { rejected: true } });
    edLocateMock.getActiveDocumentContent.mockReturnValue("旧\n用户额外编辑");

    await rejectDiff("d4");

    expect(edStore.getSnapshot().loadedContent).toBe("旧\n用户额外编辑");
    expect(edStore.getSnapshot().tabState).toBe("dirty");
  });

  it("rejectDiff：样式草稿 diff 用草稿前全文恢复，而不是只回滚单块文本", async () => {
    edStore.loadDoc("a.docx", "<p><strong>新</strong></p>", 1, null, "html");
    edStore.markDirty();
    deStore.addDiff({
      diffId: "d5",
      blockId: "b1",
      relPath: "a.docx",
      title: "加粗",
      before: "旧",
      after: "（样式修改）",
      state: "pending",
      draftApplied: true,
      draftBaseContent: "<p>旧</p>",
      draftBaseTabState: "saved",
      styleOpsPayload: [{ op: "set_mark", blockId: "b1", start: 0, end: 1, mark: { type: "bold" } }],
    });
    invokeMock.mockResolvedValue({ result: { rejected: true } });

    await rejectDiff("d5");

    expect(edLocateMock.applyEditOps).toHaveBeenCalledWith([{ op: "replace_document", content: "<p>旧</p>", format: "html", contentFormat: "html" }]);
    expect(edStore.getSnapshot()).toMatchObject({ loadedContent: "<p>旧</p>", tabState: "saved" });
  });

  it("rejectDiff：整篇草稿 diff 用草稿前全文恢复", async () => {
    edStore.loadDoc("a.docx", "<p>新</p>", 1, null, "html");
    edStore.markDirty();
    deStore.addDiff({
      diffId: "d6",
      blockId: "",
      relPath: "a.docx",
      title: "整篇改写",
      before: "<p>旧</p>",
      after: "<p>新</p>",
      state: "pending",
      draftApplied: true,
      draftBaseContent: "<p>旧</p>",
      draftBaseTabState: "saved",
    });
    invokeMock.mockResolvedValue({ result: { rejected: true } });

    await rejectDiff("d6");

    expect(edLocateMock.applyEditOps).toHaveBeenCalledWith([{ op: "replace_document", content: "<p>旧</p>", format: "html", contentFormat: "html" }]);
    expect(edStore.getSnapshot()).toMatchObject({ loadedContent: "<p>旧</p>", tabState: "saved" });
  });

  // covers: A-CORE-C-T-01 §5.4（D4 后置编辑检测 — 无后置编辑不应产生虚假 dirty）
  it("rejectDiff：整篇草稿，diff 生效后无用户编辑 → 不产生 false dirty", async () => {
    edStore.loadDoc("a.md", "旧", 1);
    deStore.addDiff({
      diffId: "d7",
      blockId: "",
      relPath: "a.md",
      title: "整篇改写",
      before: "旧",
      after: "新内容",
      state: "pending",
      draftApplied: true,
      draftBaseContent: "旧",
      draftAfterContent: "新内容",
      draftBaseTabState: "saved",
    });
    invokeMock.mockResolvedValue({ result: { rejected: true } });
    // 当前内容 = diff 应用后的内容，无后续编辑
    edLocateMock.getActiveDocumentContent.mockReturnValue("新内容");

    await rejectDiff("d7");

    expect(lastCommand()).toBe("de.reject_diff");
    expect(edStore.getSnapshot()).toMatchObject({ loadedContent: "旧", tabState: "saved" });
  });

  // covers: A-CORE-C-T-01 §5.4（D4 后置编辑检测 — 有后置编辑应产生 dirty）
  it("rejectDiff：整篇草稿，diff 生效后用户又编辑 → 产生 dirty", async () => {
    edStore.loadDoc("a.md", "旧", 1);
    deStore.addDiff({
      diffId: "d8",
      blockId: "",
      relPath: "a.md",
      title: "整篇改写",
      before: "旧",
      after: "新内容",
      state: "pending",
      draftApplied: true,
      draftBaseContent: "旧",
      draftAfterContent: "新内容",
      draftBaseTabState: "saved",
    });
    invokeMock.mockResolvedValue({ result: { rejected: true } });
    // 当前内容 ≠ draftAfterContent，用户在后置编辑
    edLocateMock.getActiveDocumentContent.mockReturnValue("新内容 + 用户后置编辑");

    await rejectDiff("d8");

    expect(lastCommand()).toBe("de.reject_diff");
    expect(edStore.getSnapshot()).toMatchObject({ loadedContent: "旧", tabState: "dirty" });
  });
});
