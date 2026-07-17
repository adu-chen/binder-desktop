import { beforeEach, describe, expect, it, vi } from "vitest";

const tauriMock = vi.hoisted(() => {
  class MockChannel<T> {
    onmessage: ((message: T) => void) | null = null;
  }

  return {
    Channel: MockChannel,
    invoke: vi.fn(),
  };
});

const edLocateMock = vi.hoisted(() => ({
  getActiveBlockSnapshotById: vi.fn(),
  getActiveBlockSnapshotByIndex: vi.fn(),
  getActiveDocumentContent: vi.fn(),
  locateBlock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  Channel: tauriMock.Channel,
  invoke: tauriMock.invoke,
}));

vi.mock("./edLocate", () => edLocateMock);

import { invoke } from "@tauri-apps/api/core";
import { agStore, activeSession } from "../store/agStore";
import { deStore } from "../store/deStore";
import { edStore } from "../store/edStore";
import type { MessageContentPart, ReferencePayload } from "./referencePayload";
import { finishAssist, requestAssist, startTurn, type AgTurnEvent } from "./agService";

const invokeMock = vi.mocked(invoke);

function completeTurn(arg: unknown) {
  const onEvent = (arg as { onEvent: { onmessage: ((message: AgTurnEvent) => void) | null } }).onEvent;
  onEvent.onmessage?.({ type: "done", turn_id: "t", stop_reason: "stop" });
}

function projectionResult(patch: Partial<Record<string, unknown>> = {}) {
  const has = (key: string) => Object.prototype.hasOwnProperty.call(patch, key);
  return JSON.stringify({
    summary: "已生成修改建议",
    diff_id: patch.diff_id ?? "diff-1",
    rel_path: patch.rel_path ?? "notes/current.md",
    projection: {
      diff_id: patch.diff_id ?? "diff-1",
      rel_path: patch.rel_path ?? "notes/current.md",
      patch_kind: patch.patch_kind ?? "replace_block",
      block_id: has("block_id") ? patch.block_id : "b2",
      block_index: has("block_index") ? patch.block_index : 1,
      before_text: patch.before_text ?? "第二段原文",
      after_text: patch.after_text ?? "第二段改写",
      location_label: patch.location_label ?? "Block 1",
      draft_mode: patch.draft_mode ?? "current_editor",
      style_ops: patch.style_ops ?? null,
    },
  });
}

beforeEach(() => {
  invokeMock.mockReset();
  edLocateMock.getActiveBlockSnapshotById.mockReset();
  edLocateMock.getActiveBlockSnapshotByIndex.mockReset();
  edLocateMock.getActiveDocumentContent.mockReset();
  edLocateMock.locateBlock.mockReset();
  edLocateMock.getActiveDocumentContent.mockReturnValue(null);
  edLocateMock.locateBlock.mockReturnValue("located");
  edStore.reset();
  deStore.reset();
  agStore.applyTurnState("test-turn", "Completed");
  agStore.newSession();
});

describe("requestAssist 契约", () => {
  // covers: BR-AG-STATE-003
  // request_assist 携带 requestId + 文档/block/字符锚上下文，并接收 3 candidates。
  it("forwards assist anchor context and returns autocomplete candidates", async () => {
    invokeMock.mockResolvedValue({
      request_id: "as-1",
      candidate: "候选 1",
      candidates: ["候选 1", "候选 2", "候选 3"],
    });

    const result = await requestAssist({
      requestId: "as-1",
      kind: "autocomplete",
      docPath: "notes/current.md",
      loadSeq: 7,
      blockId: "b1",
      start: 5,
      end: 5,
      prefix: "前文",
      before: "前",
      after: "后",
    });

    expect(invokeMock).toHaveBeenCalledWith("ag_request_assist", {
      assistId: "as-1",
      kind: "autocomplete",
      docPath: "notes/current.md",
      loadSeq: 7,
      blockId: "b1",
      start: 5,
      end: 5,
      instruction: undefined,
      selection: undefined,
      prefix: "前文",
      before: "前",
      after: "后",
      baseText: undefined,
    });
    expect(result).toEqual({
      requestId: "as-1",
      candidate: "候选 1",
      candidates: ["候选 1", "候选 2", "候选 3"],
      failureReason: null,
    });
  });

  it("keeps assist failure reason internal while returning empty candidates", async () => {
    invokeMock.mockResolvedValue({
      request_id: "as-empty",
      candidate: null,
      candidates: [],
      failure_reason: "parse_underflow",
    });

    const result = await requestAssist({
      requestId: "as-empty",
      kind: "autocomplete",
      docPath: "notes/current.md",
      loadSeq: 7,
      blockId: "b1",
      start: 5,
      end: 5,
      prefix: "前文",
      before: "前",
      after: "后",
    });

    expect(result).toEqual({
      requestId: "as-empty",
      candidate: null,
      candidates: [],
      failureReason: "parse_underflow",
    });
  });

  it("preserves invoke failure as internal failure reason", async () => {
    invokeMock.mockRejectedValue(new Error("offline"));
    await expect(
      requestAssist({
        requestId: "as-2",
        kind: "inline",
        docPath: "notes/current.md",
        loadSeq: 7,
        blockId: "b1",
        start: 0,
        end: 2,
        instruction: "改短",
        selection: "原文",
        baseText: "上一轮",
      }),
    ).resolves.toEqual({
      requestId: "as-2",
      candidate: null,
      candidates: [],
      failureReason: "invoke_error",
    });
  });

  // covers: BR-AG-STATE-003
  // 采纳/忽略/锚失效必须回写 SM-AG-03 终态；回写失败不打断编辑。
  it("finishes assist lifecycle with terminal event", async () => {
    invokeMock.mockResolvedValue({});

    await finishAssist("as-3", "accept");

    expect(invokeMock).toHaveBeenCalledWith("ag_finish_assist", {
      assistId: "as-3",
      event: "accept",
    });
  });
});

describe("agService ReferencePayload 发送契约", () => {
  // covers: BR-AG-DATA-001, BR-REF-DATA-003, BR-REF-DATA-004
  // 非拖拽引用入口必须从 Composer 发送层进入 ag_start_turn.references，并保留 displayParts 供消息回放。
  it("startTurn forwards explicit references and display parts into ag_start_turn", async () => {
    edStore.loadDoc("notes/current.md", "当前文档内容", 101);
    const references: ReferencePayload[] = [
      {
        kind: "selection",
        relPath: "notes/a.md",
        blockId: "block-1",
        start: 2,
        end: 6,
        text: "选区内容",
      },
      {
        kind: "memory",
        entryId: "m1",
        title: "偏好",
        content: "用户偏好简洁回答",
      },
      {
        kind: "url",
        url: "https://example.com/ref",
        title: "参考链接",
      },
    ];
    const displayParts: MessageContentPart[] = [
      { type: "text", text: "请结合 " },
      {
        type: "reference",
        refId: "r1",
        kind: "memory",
        label: "记忆 偏好",
        payload: references[1],
      },
      { type: "text", text: " 修改。" },
    ];

    invokeMock.mockImplementation(async (_command, arg) => {
      completeTurn(arg);
      return undefined;
    });

    await startTurn("请结合 [引用:记忆 偏好] 修改。", {
      provider: "openai",
      model: "gpt-test",
      references,
      displayParts,
      mentions: ["legacy.md"],
      blockRefs: [{ relPath: "legacy.md", blockId: "b", text: "legacy" }],
    });

    expect(invokeMock).toHaveBeenCalledTimes(1);
    const [command, arg] = invokeMock.mock.calls[0] as [string, Record<string, unknown>];
    expect(command).toBe("ag_start_turn");
    expect(arg).toMatchObject({
      prompt: "请结合 [引用:记忆 偏好] 修改。",
      provider: "openai",
      model: "gpt-test",
      activeDocPath: "notes/current.md",
      activeDocContent: "当前文档内容",
      activeDocFormat: "text",
      references,
      mentions: [],
      blockRefs: [],
    });

    const session = activeSession(agStore.getSnapshot());
    const user = session.messages.find((message) => message.role === "user");
    expect(user).toMatchObject({
      content: "请结合 [引用:记忆 偏好] 修改。",
      parts: displayParts,
    });
  });

  it("activeDocContent 优先取当前编辑器实时正文，而不是 edStore 载入快照", async () => {
    edStore.loadDoc("notes/current.md", "旧快照", 101);
    edLocateMock.getActiveDocumentContent.mockReturnValue("实时编辑器正文");
    invokeMock.mockImplementation(async (_command, arg) => {
      completeTurn(arg);
      return undefined;
    });

    await startTurn("继续修改");

    const [, arg] = invokeMock.mock.calls[0] as [string, Record<string, unknown>];
    expect(arg.activeDocContent).toBe("实时编辑器正文");
  });

  it("diff_proposed 的 before/after 来自后端 projection，不取整篇 fallback", async () => {
    edStore.loadDoc("notes/current.md", "第一段\n第二段原文\n最后一段", 101);
    invokeMock.mockImplementation(async (_command, arg) => {
      const onEvent = (arg as { onEvent: { onmessage: ((message: AgTurnEvent) => void) | null } }).onEvent;
      onEvent.onmessage?.({
        type: "tool_request",
        turn_id: "t",
        tool_name: "propose_edit",
        params: { path: "notes/current.md", operation: "replace_block", block_index: 1, new_text: "第二段改写" },
      });
      onEvent.onmessage?.({
        type: "tool_result",
        turn_id: "t",
        tool_name: "propose_edit",
        result: projectionResult(),
        success: true,
      });
      onEvent.onmessage?.({ type: "diff_proposed", turn_id: "t", diff_id: "diff-1", rel_path: "notes/current.md" });
      completeTurn(arg);
      return undefined;
    });

    await startTurn("把第二段改写");

    const session = activeSession(agStore.getSnapshot());
    const assistant = session.messages.find((message) => message.role === "assistant");
    expect(deStore.getSnapshot().diffs[0]).toMatchObject({
      diffId: "diff-1",
      sessionId: session.id,
      turnId: expect.stringMatching(/^t-/),
      anchorMessageId: assistant?.id,
      createdAt: expect.any(Number),
      blockId: "b2",
      before: "第二段原文",
      after: "第二段改写",
      locationLabel: "Block 1",
      patchKind: "replace_block",
    });
  });

  it("前端不再根据数字型 block_id 自行解析目标块", async () => {
    edStore.loadDoc("notes/current.md", "第一段\n第二段原文", 101);
    invokeMock.mockImplementation(async (_command, arg) => {
      const onEvent = (arg as { onEvent: { onmessage: ((message: AgTurnEvent) => void) | null } }).onEvent;
      onEvent.onmessage?.({
        type: "tool_request",
        turn_id: "t",
        tool_name: "propose_edit",
        params: { path: "notes/current.md", operation: "replace_block", block_id: "1", new_text: "第二段改写" },
      });
      onEvent.onmessage?.({
        type: "tool_result",
        turn_id: "t",
        tool_name: "propose_edit",
        result: projectionResult({ diff_id: "diff-2" }),
        success: true,
      });
      onEvent.onmessage?.({ type: "diff_proposed", turn_id: "t", diff_id: "diff-2", rel_path: "notes/current.md" });
      completeTurn(arg);
      return undefined;
    });

    await startTurn("把第二段改写");

    expect(edLocateMock.getActiveBlockSnapshotByIndex).not.toHaveBeenCalled();
    expect(edLocateMock.getActiveBlockSnapshotById).not.toHaveBeenCalled();
    expect(deStore.getSnapshot().diffs[0]).toMatchObject({ blockId: "b2", before: "第二段原文", patchKind: "replace_block" });
  });

  it("缺少后端 projection 时，diff_proposed 投影为 error 卡", async () => {
    edStore.loadDoc("notes/current.md", "第一段\n第二段", 101);
    invokeMock.mockImplementation(async (_command, arg) => {
      const onEvent = (arg as { onEvent: { onmessage: ((message: AgTurnEvent) => void) | null } }).onEvent;
      onEvent.onmessage?.({ type: "diff_proposed", turn_id: "t", diff_id: "diff-missing", rel_path: "notes/current.md" });
      completeTurn(arg);
      return undefined;
    });

    await startTurn("改写这一段");

    expect(deStore.getSnapshot().diffs[0]).toMatchObject({
      diffId: "diff-missing",
      blockId: "",
      state: "error",
      errorCode: "DE_DIFF_PROJECTION_MISSING",
    });
  });

  it("块级 projection 缺少 block_id 时投影为精准目标缺失错误", async () => {
    edStore.loadDoc("notes/current.md", "第一段\n第二段", 101);
    invokeMock.mockImplementation(async (_command, arg) => {
      const onEvent = (arg as { onEvent: { onmessage: ((message: AgTurnEvent) => void) | null } }).onEvent;
      onEvent.onmessage?.({
        type: "tool_result",
        turn_id: "t",
        tool_name: "propose_edit",
        result: projectionResult({ diff_id: "diff-target-missing", block_id: null, block_index: null }),
        success: true,
      });
      onEvent.onmessage?.({ type: "diff_proposed", turn_id: "t", diff_id: "diff-target-missing", rel_path: "notes/current.md" });
      completeTurn(arg);
      return undefined;
    });

    await startTurn("把第二段改写");

    expect(deStore.getSnapshot().diffs[0]).toMatchObject({
      diffId: "diff-target-missing",
      blockId: "",
      state: "error",
      errorCode: "ED_DRAFT_TARGET_MISSING",
      patchKind: "replace_block",
    });
  });

  it("propose_edit 的 style_ops 投影为可执行 StyleOp 载荷和样式摘要", async () => {
    edStore.loadDoc("notes/current.docx", '<p data-block-id="b1">正文</p>', 101, null, "html");
    edLocateMock.getActiveBlockSnapshotById.mockReturnValue({
      blockId: "b1",
      index: 0,
      text: "正文",
      pos: 0,
      size: 4,
      contentFrom: 1,
      contentTo: 3,
    });
    const styleOps = [{ op: "set_mark", blockId: "b1", start: 0, end: 2, mark: { type: "bold" } }];
    invokeMock.mockImplementation(async (_command, arg) => {
      const onEvent = (arg as { onEvent: { onmessage: ((message: AgTurnEvent) => void) | null } }).onEvent;
      onEvent.onmessage?.({
        type: "tool_result",
        turn_id: "t",
        tool_name: "propose_edit",
        result: projectionResult({
          diff_id: "diff-style",
          rel_path: "notes/current.docx",
          patch_kind: "style_ops",
          block_id: "b1",
          block_index: 0,
          before_text: "正文",
          after_text: "（样式修改）",
          location_label: "样式修改",
          style_ops: styleOps,
        }),
        success: true,
      });
      onEvent.onmessage?.({ type: "diff_proposed", turn_id: "t", diff_id: "diff-style", rel_path: "notes/current.docx" });
      completeTurn(arg);
      return undefined;
    });

    await startTurn("把正文加粗");

    expect(deStore.getSnapshot().diffs[0]).toMatchObject({
      diffId: "diff-style",
      blockId: "b1",
      before: "正文",
      after: "（样式修改）",
      styleOpsPayload: styleOps,
      styleOps: ["b1 设置 bold [0,2]"],
      locationLabel: "样式修改",
      patchKind: "style_ops",
    });
  });

  it("显式 replace_document 投影为整篇 diff，供 Editor 执行整篇草稿替换", async () => {
    edStore.loadDoc("notes/current.md", "旧全文", 101);
    edLocateMock.getActiveDocumentContent.mockReturnValue("实时旧全文");
    invokeMock.mockImplementation(async (_command, arg) => {
      const onEvent = (arg as { onEvent: { onmessage: ((message: AgTurnEvent) => void) | null } }).onEvent;
      onEvent.onmessage?.({
        type: "tool_result",
        turn_id: "t",
        tool_name: "propose_edit",
        result: projectionResult({
          diff_id: "diff-doc",
          patch_kind: "replace_document",
          block_id: null,
          block_index: null,
          before_text: "实时旧全文",
          after_text: "新全文",
          location_label: "整篇",
        }),
        success: true,
      });
      onEvent.onmessage?.({ type: "diff_proposed", turn_id: "t", diff_id: "diff-doc", rel_path: "notes/current.md" });
      completeTurn(arg);
      return undefined;
    });

    await startTurn("整篇重写");

    expect(deStore.getSnapshot().diffs[0]).toMatchObject({
      diffId: "diff-doc",
      blockId: "",
      before: "实时旧全文",
      after: "新全文",
      locationLabel: "整篇",
      patchKind: "replace_document",
      state: "pending",
    });
  });

  it("结构工具成功后刷新文件树", async () => {
    invokeMock.mockImplementation(async (command, arg) => {
      if (command === "ag_start_turn") {
        const onEvent = (arg as { onEvent: { onmessage: ((message: AgTurnEvent) => void) | null } }).onEvent;
        onEvent.onmessage?.({
          type: "tool_result",
          turn_id: "t",
          tool_name: "move_doc",
          result: JSON.stringify({ operation: "move_doc", from_path: "a.md", to_path: "分类/a.md" }),
          success: true,
        });
        completeTurn(arg);
        return undefined;
      }
      if (command === "dispatch") {
        return { correlation_id: "ws-test", result: [] };
      }
      return undefined;
    });

    await startTurn("把文件整理到分类目录");
    await Promise.resolve();

    expect(invokeMock).toHaveBeenCalledWith(
      "dispatch",
      expect.objectContaining({
        request: expect.objectContaining({ command: "ws.list_docs" }),
      }),
    );
  });
});
