import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { deStore, resolveProposalBatchCohort, toGreenReview, EXPIRED_REASON_TEXT, type DiffView } from "../store/deStore";
import { edStore } from "../store/edStore";
import {
  acceptDiff,
  applyOpenSessionDiff,
  expireStalePendingForPath,
  generateSampleDiff,
  persistImageDiffSnapshots,
  promotePendingDiffOnOpen,
  rejectDiff,
} from "./deService";
import type { DocumentImageSnapshot } from "./edImageOps";

// covers: BR-DE-DATA-001, BR-DE-STATE-001, BR-ED-STATE-001
describe("ProposalBatch cohort 校验", () => {
  const grouped = (id: string, groupIndex: number, groupCount = 2): DiffView => ({
    diffId: id,
    blockId: `p-${groupIndex}`,
    relPath: "notes/a.docx",
    title: id,
    before: `before-${groupIndex}`,
    after: `after-${groupIndex}`,
    state: "pending",
    patchKind: "block_ops",
    draftMode: "current_editor",
    baseSessionVersion: 7,
    baseContentHash: "base-hash",
    proposalBatch: { id: "proposal-1", groupIndex, groupCount },
    blockOpsPayload: {
      payloadVersion: 1,
      base: { sessionVersion: 7, contentHash: "base-hash" },
      operations: [{
        op: "replace_node",
        blockId: `p-${groupIndex}`,
        fingerprint: `fp-${groupIndex}`,
        expectedNodeType: "paragraph",
        beforeNode: { type: "paragraph", attrs: { blockId: `p-${groupIndex}` } },
        afterNode: { type: "paragraph", attrs: { blockId: `p-${groupIndex}` } },
      }],
    },
  });

  it("缺组时等待，乱序到达后按 group_index 形成完整 cohort", () => {
    const first = grouped("d-1", 1);
    const second = grouped("d-2", 2);
    expect(resolveProposalBatchCohort([second], second).status).toBe("waiting");
    const result = resolveProposalBatchCohort([second, first], second);
    expect(result.status).toBe("ready");
    expect(result.diffs.map((diff) => diff.diffId)).toEqual(["d-1", "d-2"]);
  });

  it("重复组序、基线漂移或目标重叠均失败关闭", () => {
    const first = grouped("d-1", 1);
    const duplicate = grouped("d-2", 1);
    expect(resolveProposalBatchCohort([first, duplicate], first)).toMatchObject({ status: "invalid" });

    const stale = { ...grouped("d-2", 2), baseContentHash: "other" };
    expect(resolveProposalBatchCohort([first, stale], first)).toMatchObject({ status: "invalid" });

    const overlap = grouped("d-2", 2);
    overlap.blockOpsPayload = {
      ...overlap.blockOpsPayload!,
      operations: [{ ...overlap.blockOpsPayload!.operations[0], blockId: "p-1" }],
    };
    expect(resolveProposalBatchCohort([first, overlap], first)).toMatchObject({
      status: "invalid",
      reason: "DE_PROPOSAL_BATCH_TARGET_OVERLAP",
    });
  });

  it("一张卡进入终态不改变同批兄弟卡", () => {
    deStore.reset();
    deStore.addDiff(grouped("d-1", 1));
    deStore.addDiff(grouped("d-2", 2));
    deStore.applyBackendState("d-1", "Rejected", true);
    expect(deStore.getSnapshot().diffs.map((diff) => [diff.diffId, diff.state])).toEqual([
      ["d-1", "rejected"],
      ["d-2", "pending"],
    ]);
  });
});

// Migrated intact from src/services/deService.test.ts.
{
const tauriMock = vi.hoisted(() => ({ invoke: vi.fn() }));
const edServiceMock = vi.hoisted(() => ({ openDocument: vi.fn(), persistSessionSnapshot: vi.fn() }));
const edLocateMock = vi.hoisted(() => ({
  applyEditOps: vi.fn(),
  getActiveDocumentContent: vi.fn(),
  getActiveEditor: vi.fn(),
}));
const edBlockOpsMock = vi.hoisted(() => ({
  applyStructuredBlockOperations: vi.fn(),
  validateStructuredBlockOperations: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: tauriMock.invoke }));
vi.mock("./edService", () => edServiceMock);
vi.mock("./edLocate", () => edLocateMock);
vi.mock("./edBlockOps", () => edBlockOpsMock);


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
  edLocateMock.getActiveEditor.mockReset();
  edLocateMock.getActiveEditor.mockReturnValue(null);
  edBlockOpsMock.validateStructuredBlockOperations.mockReset();
  edBlockOpsMock.validateStructuredBlockOperations.mockReturnValue({ ok: true, affectedBlockIds: ["p-1"] });
  edBlockOpsMock.applyStructuredBlockOperations.mockReset();
  edBlockOpsMock.applyStructuredBlockOperations.mockReturnValue({ ok: true, affectedBlockIds: ["p-1"] });
  deStore.reset();
  edStore.reset();
});

function lastCommand(): string {
  const calls = invokeMock.mock.calls;
  const [, arg] = calls[calls.length - 1] as [string, { request: { command: string } }];
  return arg.request.command;
}

describe("deService 草稿 diff 裁决", () => {
  // covers: BR-DE-DATA-002
  it("persistImageDiffSnapshots：经受控 command 持久化完整前后图片快照", async () => {
    const before: DocumentImageSnapshot[] = [{
      imageId: "image_0_abcd",
      fingerprint: "abcd",
      ordinal: 0,
      blockId: "b-img",
      imagePos: 2,
      standalone: true,
      alt: "项目图",
      width: "240",
      height: null,
      mediaRef: "media/image.png",
      projectSourcePath: "images/image.png",
      projectSourceHash: "a".repeat(64),
      alignment: "left" as const,
      capabilities: ["delete_image", "resize_image", "move_image", "set_image_layout"],
      resizeBounds: { min: 40, max: 800 },
      moveDirections: ["up", "down"],
    }];
    invokeMock.mockResolvedValue({ result: { persisted: true } });

    await persistImageDiffSnapshots("d-image", before, [], ["b-img"]);

    expect(invokeMock).toHaveBeenCalledWith("dispatch", {
      request: expect.objectContaining({
        command: "de.update_image_diff_snapshots",
        params: {
          diffId: "d-image",
          before,
          after: [],
          affectedBlockIds: ["b-img"],
        },
      }),
    });
  });

  // covers: BR-DE-DATA-002, BR-ED-DATA-001
  it("rejectDiff：BlockOperation 先做反向冲突预检，再精确恢复节点而不整篇覆盖", async () => {
    const before = '<ul><li><p data-block-id="p-1">原第一条</p></li><li><p data-block-id="p-2">第二条</p></li></ul>';
    edStore.loadDoc("a.docx", before, 1, null, "html");
    const fakeEditor = { state: {} };
    edLocateMock.getActiveEditor.mockReturnValue(fakeEditor);
    edLocateMock.getActiveDocumentContent.mockReturnValue(before);
    invokeMock.mockResolvedValue({ result: { rejected: true } });
    const blockOpsPayload = {
      payloadVersion: 1 as const,
      base: { sessionVersion: 1, contentHash: "base-hash" },
      operations: [{
        op: "replace_node" as const,
        blockId: "p-1",
        fingerprint: "block-v1-old",
        expectedNodeType: "paragraph",
        beforeNode: {
          type: "paragraph",
          attrs: { blockId: "p-1" },
          content: [{ type: "text", text: "原第一条" }],
        },
        afterNode: {
          type: "paragraph",
          attrs: { blockId: "p-1" },
          content: [{ type: "text", text: "新第一条" }],
        },
      }],
    };
    deStore.addDiff({
      diffId: "d-block",
      blockId: "p-1",
      relPath: "a.docx",
      title: "结构化块修改",
      before: "原第一条",
      after: "新第一条",
      state: "pending",
      patchKind: "block_ops",
      blockOpsPayload,
      draftApplied: true,
      draftBaseContent: before,
      draftBaseTabState: "normal",
    });

    await rejectDiff("d-block");

    expect(edBlockOpsMock.validateStructuredBlockOperations).toHaveBeenCalledWith(
      fakeEditor,
      blockOpsPayload,
      "reverse",
    );
    expect(lastCommand()).toBe("de.reject_diff");
    expect(edBlockOpsMock.applyStructuredBlockOperations).toHaveBeenCalledWith(
      fakeEditor,
      blockOpsPayload,
      "reverse",
    );
    expect(edLocateMock.applyEditOps).not.toHaveBeenCalled();
    expect(edStore.getSnapshot().loadedContent).toBe(before);
  });

  it("rejectDiff：BlockOperation fingerprint 已变化时进入冲突且不提交后端拒绝", async () => {
    edStore.loadDoc("a.docx", '<p data-block-id="p-1">用户后改</p>', 1, null, "html");
    edLocateMock.getActiveEditor.mockReturnValue({ state: {} });
    edBlockOpsMock.validateStructuredBlockOperations.mockReturnValue({
      ok: false,
      reason: "ED_BLOCK_OPERATION_FINGERPRINT_CHANGED",
    });
    deStore.addDiff({
      diffId: "d-block-conflict",
      blockId: "p-1",
      relPath: "a.docx",
      title: "结构化块修改",
      before: "原文",
      after: "候选",
      state: "pending",
      patchKind: "block_ops",
      blockOpsPayload: {
        payloadVersion: 1,
        base: { sessionVersion: 1, contentHash: "base-hash" },
        operations: [{
          op: "replace_node",
          blockId: "p-1",
          fingerprint: "block-v1-old",
          expectedNodeType: "paragraph",
          beforeNode: { type: "paragraph", attrs: { blockId: "p-1" } },
          afterNode: { type: "paragraph", attrs: { blockId: "p-1" } },
        }],
      },
      draftApplied: true,
    });

    await rejectDiff("d-block-conflict");

    expect(invokeMock).not.toHaveBeenCalled();
    expect(edBlockOpsMock.applyStructuredBlockOperations).not.toHaveBeenCalled();
    expect(deStore.getSnapshot().diffs[0]).toMatchObject({
      state: "conflict",
      errorCode: "ED_BLOCK_OPERATION_FINGERPRINT_CHANGED",
    });
  });

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

  // covers: BR-CORE-STATE-001
  // 有 LogicalState：接受卡片只推进生命周期，内容已在预览时进入会话，绝不重复落盘。
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

  it("acceptDiff：当前编辑器预览尚在排队时不允许绕过预览直接写盘", async () => {
    edStore.loadDoc("a.md", "旧", 1);
    deStore.addDiff({
      diffId: "d-preview-queued",
      blockId: "b1",
      relPath: "a.md",
      title: "等待预览",
      before: "旧",
      after: "新",
      state: "pending",
      draftMode: "current_editor",
      draftPreviewState: "queued",
    });

    await acceptDiff("d-preview-queued");

    expect(invokeMock).not.toHaveBeenCalled();
    expect(deStore.getSnapshot().diffs[0]).toMatchObject({
      state: "pending",
      draftPreviewState: "queued",
    });
  });

  it("acceptDiff：BlockOperation 被后置编辑时进入冲突且不确认 accepted", async () => {
    edStore.loadDoc("a.docx", '<p data-block-id="p-1">用户后改</p>', 1, null, "html");
    const fakeEditor = { state: {} };
    edLocateMock.getActiveEditor.mockReturnValue(fakeEditor);
    edBlockOpsMock.validateStructuredBlockOperations.mockReturnValue({
      ok: false,
      reason: "ED_BLOCK_OPERATION_FINGERPRINT_CHANGED",
    });
    const blockOpsPayload = {
      payloadVersion: 1 as const,
      base: { sessionVersion: 1, contentHash: "base-hash" },
      operations: [{
        op: "replace_node" as const,
        blockId: "p-1",
        fingerprint: "block-v1-old",
        expectedNodeType: "paragraph",
        beforeNode: { type: "paragraph", attrs: { blockId: "p-1" } },
        afterNode: { type: "paragraph", attrs: { blockId: "p-1" } },
      }],
    };
    deStore.addDiff({
      diffId: "d-block-accept-conflict",
      blockId: "p-1",
      relPath: "a.docx",
      title: "结构化块修改",
      before: "原文",
      after: "候选",
      state: "pending",
      patchKind: "block_ops",
      blockOpsPayload,
      draftApplied: true,
    });

    await acceptDiff("d-block-accept-conflict");

    expect(edBlockOpsMock.validateStructuredBlockOperations).toHaveBeenCalledWith(
      fakeEditor,
      blockOpsPayload,
      "reverse",
    );
    expect(invokeMock).not.toHaveBeenCalled();
    expect(deStore.getSnapshot().diffs[0]).toMatchObject({
      state: "conflict",
      errorCode: "ED_BLOCK_OPERATION_FINGERPRINT_CHANGED",
    });
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

  // covers: BR-CORE-STATE-002
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

  it("acceptDiff：未应用错误态不再次接受，也不覆盖首要错误码", async () => {
    edStore.loadDoc("a.docx", "<p>当前</p>", 1, null, "html");
    deStore.addDiff({
      diffId: "d-image-stale",
      blockId: "b1",
      relPath: "a.docx",
      title: "图片居中",
      before: "当前图片",
      after: "图片操作：对齐 1",
      state: "error",
      errorCode: "ED_IMAGE_SNAPSHOT_STALE",
      patchKind: "image_ops",
    });

    await acceptDiff("d-image-stale");

    expect(invokeMock).not.toHaveBeenCalled();
    expect(deStore.getSnapshot().diffs[0]).toMatchObject({
      state: "error",
      errorCode: "ED_IMAGE_SNAPSHOT_STALE",
    });
  });

  it("applyOpenSessionDiff：已打开非激活文档应用 LogicalState，但保持 pending 等待用户裁决", async () => {
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
    });
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
    expect(invokeMock).not.toHaveBeenCalled();
    expect(deStore.getSnapshot().diffs[0]).toMatchObject({
      state: "pending",
      draftApplied: true,
    });
  });

  // covers: BR-CORE-STATE-001
  it("acceptDiff：非活动 LogicalState 预览后仍可由用户显式接受", async () => {
    edStore.loadDoc("target.md", "一\n旧", 1);
    edStore.loadDoc("active.md", "当前", 2);
    const target = edStore.getSnapshot().sessions["target.md"];
    deStore.addDiff({
      diffId: "d-open-accept",
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
    });
    await applyOpenSessionDiff("d-open-accept");
    invokeMock.mockResolvedValue({ result: { accepted: true } });

    await acceptDiff("d-open-accept");

    expect(lastCommand()).toBe("de.accept_draft_diff");
    expect(invokeMock).toHaveBeenLastCalledWith("dispatch", expect.objectContaining({
      request: expect.objectContaining({
        params: expect.objectContaining({ applyContext: "open_session" }),
      }),
    }));
    expect(edStore.getSnapshot().sessions["target.md"]?.content).toBe("一\n新");
  });

  // covers: BR-CORE-STATE-003
  it("promotePendingDiffOnOpen：目标打开后只迁移到 LogicalState 路径，保持 pending", async () => {
    deStore.addDiff({
      diffId: "d-promote",
      blockId: "block-2",
      relPath: "target.md",
      title: "后台建议",
      before: "旧",
      after: "新",
      state: "pending",
      draftMode: "background",
      snapshotSource: "disk_snapshot",
    });
    invokeMock.mockResolvedValue({ result: { diff_id: "d-promote", pending: true } });

    await promotePendingDiffOnOpen("d-promote", "block-live");

    expect(lastCommand()).toBe("de.promote_pending_diff_on_open");
    expect(deStore.getSnapshot().diffs[0]).toMatchObject({
      state: "pending",
      blockId: "block-live",
      draftMode: "current_editor",
    });
  });

  // covers: BR-CORE-STATE-003
  it("rejectDiff：打开后迁移的非活动 LogicalState 可回滚且卡片仍可拒绝", async () => {
    edStore.loadDoc("target.md", "旧", 1);
    edStore.loadDoc("active.md", "当前", 2);
    edStore.updateContent("target.md", "新");
    deStore.addDiff({
      diffId: "d-promote-reject",
      blockId: "",
      relPath: "target.md",
      title: "迁移建议",
      before: "旧",
      after: "新",
      state: "pending",
      draftMode: "current_editor",
      draftApplied: true,
      draftBaseContent: "旧",
      draftBaseTabState: "normal",
      draftAfterContent: "新",
    });
    invokeMock.mockResolvedValue({ result: { rejected: true } });

    await rejectDiff("d-promote-reject");

    expect(lastCommand()).toBe("de.reject_diff");
    expect(edStore.getSnapshot().sessions["target.md"]).toMatchObject({
      content: "旧",
      tabState: "normal",
    });
    expect(edServiceMock.persistSessionSnapshot).toHaveBeenCalledWith(expect.objectContaining({
      relPath: "target.md",
      content: "旧",
    }));
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

  // covers: BR-DE-DATA-002, BR-ED-DATA-003
  it("rejectDiff：ImageOperation 即使有 anchor blockId 也按草稿前全文恢复", async () => {
    edStore.loadDoc("a.docx", '<p data-block-id="b1">旧</p><p><img src="media/new.png"></p>', 1, null, "html");
    edStore.markDirty();
    deStore.addDiff({
      diffId: "d-image",
      blockId: "b1",
      relPath: "a.docx",
      title: "插入图片",
      before: "当前图片 0 张",
      after: "图片操作：插入 1 张，删除 0 张",
      state: "pending",
      patchKind: "image_ops",
      draftApplied: true,
      draftBaseContent: '<p data-block-id="b1">旧</p>',
      draftBaseTabState: "saved",
      imageOpsPayload: {
        base: { sessionVersion: 1, contentHash: "h1" },
        operations: [{
          op: "insert_image",
          anchorBlockId: "b1",
          position: "after",
          assetId: "asset_new",
          sourcePath: "images/new.png",
          sourceHash: "a".repeat(64),
          mediaRef: `media/binder-project-${"a".repeat(64)}.png`,
        }],
      },
    });
    invokeMock.mockResolvedValue({ result: { rejected: true } });

    await rejectDiff("d-image");

    expect(edLocateMock.applyEditOps).toHaveBeenCalledWith([{
      op: "replace_document",
      content: '<p data-block-id="b1">旧</p>',
      format: "html",
      contentFormat: "html",
    }]);
    expect(edStore.getSnapshot()).toMatchObject({
      loadedContent: '<p data-block-id="b1">旧</p>',
      tabState: "saved",
    });
  });

  it("rejectDiff：多图片组合 Diff 后有人工编辑时完整回滚且保留 dirty", async () => {
    const before = '<p data-block-id="a"><img src="media/a.png"></p><p data-block-id="b"><img src="media/b.png"></p>';
    const after = '<p data-block-id="a" style="text-align:center"><img src="media/a.png" width="240"></p><p data-block-id="b"><img src="media/b.png"></p>';
    edStore.loadDoc("a.docx", after, 1, null, "html");
    deStore.addDiff({
      diffId: "d-image-composite",
      blockId: "",
      relPath: "a.docx",
      title: "调整多张图片",
      before: "当前图片 2 张",
      after: "图片操作 3 项",
      state: "pending",
      patchKind: "image_ops",
      draftApplied: true,
      draftBaseContent: before,
      draftAfterContent: after,
      draftBaseTabState: "saved",
      imageOpsPayload: {
        base: { sessionVersion: 1, contentHash: "h1" },
        operations: [
          { op: "resize_image", imageId: "image-a", fingerprint: "fp-a", width: 240 },
          { op: "set_image_layout", imageId: "image-a", fingerprint: "fp-a", alignment: "center" },
          { op: "move_image", imageId: "image-b", fingerprint: "fp-b", direction: "up" },
        ],
      },
    });
    invokeMock.mockResolvedValue({ result: { rejected: true } });
    edLocateMock.getActiveDocumentContent.mockReturnValue(`${after}<p>用户后置编辑</p>`);

    await rejectDiff("d-image-composite");

    expect(edLocateMock.applyEditOps).toHaveBeenCalledWith([{
      op: "replace_document",
      content: before,
      format: "html",
      contentFormat: "html",
    }]);
    expect(edStore.getSnapshot()).toMatchObject({
      loadedContent: before,
      tabState: "dirty",
    });
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
}

// Migrated intact from src/store/deStore.test.ts.
{
const mk = (id: string, state: DiffView["state"]): DiffView => ({
  diffId: id,
  blockId: "",
  title: "t",
  before: "a",
  after: "b",
  state,
});

beforeEach(() => deStore.reset());

describe("deStore 状态投影（SM-DE-01）", () => {
  // covers: BR-DE-STATE-001
  it("addDiff 追加；reset 清空", () => {
    deStore.addDiff(mk("d1", "pending"));
    expect(deStore.getSnapshot().diffs.map((d) => d.diffId)).toEqual(["d1"]);
    deStore.reset();
    expect(deStore.getSnapshot().diffs).toEqual([]);
  });

  // covers: BR-DE-STATE-001
  // 后端 SM-DE-01 广播投影：Rust DiffState → 视觉态；三终态不被本地呈现/再次广播覆盖。
  it("applyBackendState 投影后端态：Pending→pending，Accepted→accepted；终态不回退", () => {
    deStore.addDiff(mk("d", "pending"));
    deStore.applyBackendState("d", "Accepted", true);
    expect(deStore.getSnapshot().diffs[0].state).toBe("accepted");
    // 已终态：再投影 Pending 不回退（与后端 transition 守护一致）
    deStore.applyBackendState("d", "Pending", false);
    expect(deStore.getSnapshot().diffs[0].state).toBe("accepted");
  });

  // covers: BR-DE-STATE-001
  it("三终态互斥不可逆：本地呈现 setLocalView 不覆盖后端终态", () => {
    for (const terminal of ["accepted", "rejected", "expired"] as const) {
      deStore.reset();
      deStore.addDiff(mk("d", terminal));
      deStore.setLocalView("d", "applying");
      expect(deStore.getSnapshot().diffs[0].state).toBe(terminal);
    }
  });

  // covers: BR-WS-STATE-001
  // SM-WS-02 落盘机广播投影：Applying→applying 过渡；Applied→accepted；Discarded 不投影（终态归 SM-DE-01）。
  it("applyWsApplyState 投影落盘机：Applying→applying，Applied→accepted，Discarded 不覆盖", () => {
    deStore.addDiff(mk("d", "pending"));
    deStore.applyWsApplyState("d", "Applying");
    expect(deStore.getSnapshot().diffs[0].state).toBe("applying");
    deStore.applyWsApplyState("d", "Applied");
    expect(deStore.getSnapshot().diffs[0].state).toBe("accepted");
    // Discarded 不投影（不回退已 accepted）
    deStore.reset();
    deStore.addDiff(mk("d2", "pending"));
    deStore.applyWsApplyState("d2", "Discarded");
    expect(deStore.getSnapshot().diffs[0].state).toBe("pending");
  });

  // covers: BR-DE-STATE-001
  it("setLocalView 推进非终态过渡呈现；attachMeta 回填元数据不改状态", () => {
    deStore.addDiff(mk("d", "pending"));
    deStore.setLocalView("d", "applying");
    expect(deStore.getSnapshot().diffs[0].state).toBe("applying");
    deStore.attachMeta("d", { timelineSeq: 7 });
    expect(deStore.getSnapshot().diffs[0]).toMatchObject({ state: "applying", timelineSeq: 7 });
  });

  // covers: A-CORE-C-T-01 §5.4（D4 缺陷修复）
  it("D4 修复：markDraftApplied 记录 draftBaseDirty 和 draftAfterContent", () => {
    deStore.addDiff({ ...mk("d", "pending"), relPath: "a.md", blockId: "b1", draftPreviewState: "queued" });
    deStore.markDraftApplied("d", "base", "dirty", "afterContent");
    expect(deStore.getSnapshot().diffs[0]).toMatchObject({
      draftPreviewState: "applied",
      draftApplied: true,
      draftBaseContent: "base",
      draftBaseTabState: "dirty",
      draftBaseDirty: true,
      draftAfterContent: "afterContent",
    });
    deStore.reset();
    deStore.addDiff({ ...mk("d2", "pending"), relPath: "b.md", blockId: "b2" });
    deStore.markDraftApplied("d2", "clean", "normal", "cleanAfter");
    expect(deStore.getSnapshot().diffs[0].draftBaseDirty).toBe(false);
    expect(deStore.getSnapshot().diffs[0].draftAfterContent).toBe("cleanAfter");
  });

  it("编辑器预览失败保留 pending proposal 的未应用事实和错误码", () => {
    deStore.addDiff({ ...mk("d-preview-failed", "pending"), draftPreviewState: "queued" });
    deStore.markDraftPreviewFailed("d-preview-failed", "ED_BLOCK_OPERATION_RESULT_MISMATCH");
    expect(deStore.getSnapshot().diffs[0]).toMatchObject({
      state: "error",
      draftPreviewState: "failed",
      errorCode: "ED_BLOCK_OPERATION_RESULT_MISMATCH",
    });
    expect(deStore.getSnapshot().diffs[0].draftApplied).toBeUndefined();
  });

  it("编辑器预览结果写入可检索的最小运行日志，不记录建议正文", () => {
    const trace = vi.spyOn(console, "info").mockImplementation(() => undefined);
    try {
      deStore.addDiff({ ...mk("d-preview-trace", "pending"), draftPreviewState: "queued", after: "不应出现在日志中" });
      deStore.markDraftPreviewFailed("d-preview-trace", "ED_BLOCK_OPERATION_RESULT_MISMATCH");

      expect(trace).toHaveBeenCalledWith(expect.stringContaining('"diff_id":"d-preview-trace"'));
      expect(trace).toHaveBeenCalledWith(expect.stringContaining('"preview_state":"failed"'));
      expect(trace).toHaveBeenCalledWith(expect.not.stringContaining("不应出现在日志中"));
    } finally {
      trace.mockRestore();
    }
  });

  it("草稿 diff：应用到编辑器后保留标记，accepted 保存后清理 GreenReview 标记", () => {
    deStore.addDiff({ ...mk("d", "pending"), relPath: "a.md", blockId: "b1" });
    deStore.markDraftApplied("d", "base", "saved");
    expect(deStore.getSnapshot().diffs[0]).toMatchObject({
      draftApplied: true,
      draftBaseContent: "base",
      draftBaseTabState: "saved",
      draftSaved: false,
    });
    deStore.applyBackendState("d", "Accepted", true);
    deStore.markDraftSaved("a.md");
    expect(deStore.getSnapshot().diffs[0]).toMatchObject({ state: "accepted", draftSaved: true });
  });

  it("保存投影不得把 pending 草稿伪造成 accepted", () => {
    deStore.addDiff({ ...mk("d-pending", "pending"), relPath: "a.md", blockId: "b1" });
    deStore.markDraftApplied("d-pending", "base", "saved");
    deStore.markDraftSaved("a.md");
    expect(deStore.getSnapshot().diffs[0]).toMatchObject({ state: "pending", draftSaved: false });
  });

  // covers: BR-DE-DATA-002
  it("ImageOperation 前后实例和受影响 Block 与同一 DiffView 绑定", () => {
    const image: DocumentImageSnapshot = {
      imageId: "image_0_fp",
      fingerprint: "fp",
      ordinal: 0,
      blockId: "b-image",
      imagePos: 1,
      standalone: true,
      alt: null,
      width: null,
      height: null,
      mediaRef: "media/image.png",
      projectSourcePath: null,
      projectSourceHash: null,
      alignment: "left" as const,
      capabilities: ["delete_image", "resize_image", "move_image", "set_image_layout"],
      resizeBounds: { min: 40, max: 800 },
      moveDirections: [],
    };
    deStore.addDiff({ ...mk("d-image", "pending"), patchKind: "image_ops" });
    deStore.setImageSnapshots("d-image", [], [image], ["b1", "b-image"]);
    expect(deStore.getSnapshot().diffs[0]).toMatchObject({
      imageBefore: [],
      imageAfter: [image],
      imageAffectedBlockIds: ["b1", "b-image"],
    });
  });

  it("toGreenReview 映射：rejected/error→null，过渡态→applying，pending/conflict→pending", () => {
    expect(toGreenReview("rejected")).toBeNull();
    expect(toGreenReview("validating")).toBe("applying");
    expect(toGreenReview("applying")).toBe("applying");
    expect(toGreenReview("accepted")).toBe("accepted");
    expect(toGreenReview("expired")).toBe("expired");
    expect(toGreenReview("conflict")).toBe("pending");
    expect(toGreenReview("error")).toBeNull();
  });

  it("expired 5 种细分原因均有引导文案", () => {
    const reasons = [
      "document_revision_advanced",
      "block_resolve_failed",
      "overlapping_range",
      "external_file_changed",
      "apply_replace_failed",
    ] as const;
    for (const r of reasons) expect(EXPIRED_REASON_TEXT[r]).toBeTruthy();
  });
});

describe("连续 Proposal 前卡失效（BR-SYS-STATE-008 · REQ-DE-009）", () => {
  function pendingDiff(id: string, relPath: string, opts: { version?: number; hash?: string; mode?: string }) {
    deStore.addDiff({
      diffId: id,
      blockId: "",
      relPath,
      title: id,
      before: "",
      after: "",
      state: "pending",
      draftMode: opts.mode as DiffView["draftMode"],
      baseSessionVersion: opts.version,
      baseContentHash: opts.hash,
    });
  }

  // covers: BR-SYS-STATE-008
  it("后续 Proposal 基准推进使同文档旧 pending 卡失效（document_revision_advanced）", async () => {
    vi.mocked(invoke).mockResolvedValue({ correlation_id: "c1", result: { expired: true } });
    pendingDiff("diff-1", "notes/a.md", { version: 1, hash: "h1", mode: "open_session" });
    pendingDiff("diff-2", "notes/a.md", { version: 3, hash: "h3", mode: "open_session" });

    await expireStalePendingForPath("notes/a.md", "diff-2", { sessionVersion: 3, contentHash: "h3" });

    const diffs = deStore.getSnapshot().diffs;
    expect(diffs.find((d) => d.diffId === "diff-1")).toMatchObject({ state: "expired", readonly: true });
    expect(diffs.find((d) => d.diffId === "diff-2")?.state).toBe("pending"); // 新卡不被误失效
    // 保留历史不可裁决：本地未覆盖为 accepted/rejected
    expect(deStore.getSnapshot().diffs.find((d) => d.diffId === "diff-1")?.expiredReason).toBe("document_revision_advanced");
  });

  // covers: BR-SYS-STATE-008
  it("并行同基准 pending 卡不失效（不构成基准推进）", async () => {
    vi.mocked(invoke).mockResolvedValue({ correlation_id: "c1", result: { expired: true } });
    pendingDiff("diff-1", "notes/a.md", { version: 2, hash: "h2", mode: "open_session" });
    pendingDiff("diff-2", "notes/a.md", { version: 2, hash: "h2", mode: "open_session" });

    await expireStalePendingForPath("notes/a.md", "diff-2", { sessionVersion: 2, contentHash: "h2" });

    const diffs = deStore.getSnapshot().diffs;
    expect(diffs.find((d) => d.diffId === "diff-1")?.state).toBe("pending");
    expect(diffs.find((d) => d.diffId === "diff-2")?.state).toBe("pending");
  });

  // covers: BR-SYS-STATE-008
  it("background pending 卡（无 LogicalState 基准）不参与失效", async () => {
    vi.mocked(invoke).mockResolvedValue({ correlation_id: "c1", result: { expired: true } });
    pendingDiff("diff-bg", "notes/a.md", { mode: "background" }); // 无 base 版本/哈希
    pendingDiff("diff-2", "notes/a.md", { version: 3, hash: "h3", mode: "open_session" });

    await expireStalePendingForPath("notes/a.md", "diff-2", { sessionVersion: 3, contentHash: "h3" });

    const diffs = deStore.getSnapshot().diffs;
    expect(diffs.find((d) => d.diffId === "diff-bg")?.state).toBe("pending");
    expect(diffs.find((d) => d.diffId === "diff-2")?.state).toBe("pending");
  });

  // covers: BR-SYS-STATE-008
  it("不同文档的 pending 卡互不影响", async () => {
    vi.mocked(invoke).mockResolvedValue({ correlation_id: "c1", result: { expired: true } });
    pendingDiff("diff-a", "notes/a.md", { version: 1, hash: "h1", mode: "open_session" });
    pendingDiff("diff-b", "notes/b.md", { version: 1, hash: "h1", mode: "open_session" });

    await expireStalePendingForPath("notes/a.md", "diff-b", { sessionVersion: 5, contentHash: "h5" });

    const diffs = deStore.getSnapshot().diffs;
    expect(diffs.find((d) => d.diffId === "diff-a")?.state).toBe("expired");
    expect(diffs.find((d) => d.diffId === "diff-b")?.state).toBe("pending");
  });
});
}
