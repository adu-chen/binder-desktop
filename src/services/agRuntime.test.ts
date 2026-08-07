import { beforeEach, describe, expect, it, vi } from "vitest";

const tauriMock = vi.hoisted(() => {
  class MockChannel<T> {
    onmessage: ((message: T) => void) | null = null;
  }

  return {
    Channel: MockChannel,
    invoke: vi.fn(),
    convertFileSrc: (path: string) => `asset://localhost/${path}`,
  };
});

const edLocateMock = vi.hoisted(() => ({
  getActiveEditor: vi.fn(),
  getActiveBlockSnapshotById: vi.fn(),
  getActiveBlockSnapshotByIndex: vi.fn(),
  getActiveDocumentContent: vi.fn(),
  getActiveEditorBinding: vi.fn(),
  locateBlock: vi.fn(),
}));
const docServiceMock = vi.hoisted(() => ({
  ensureDocumentMediaDisplayBase: vi.fn(async () => undefined),
  projectDocumentMediaDisplaySrc: vi.fn(
    (relPath: string, mediaRef: string) => `asset://document/${relPath}/${mediaRef}`,
  ),
}));

vi.mock("@tauri-apps/api/core", () => ({
  Channel: tauriMock.Channel,
  invoke: tauriMock.invoke,
  convertFileSrc: tauriMock.convertFileSrc,
}));

vi.mock("./edLocate", () => edLocateMock);
vi.mock("./docService", () => docServiceMock);

import { invoke } from "@tauri-apps/api/core";
import { agStore, activeSession, emptyComposerDraft } from "../store/agStore";
import { deStore } from "../store/deStore";
import { edStore } from "../store/edStore";
import type { MessageContentPart, ReferencePayload } from "./referencePayload";
import { deleteAgentSession, finishAssist, hydrateAgentHistory, requestAssist, startTurn, type AgTurnEvent } from "./agService";

const invokeMock = vi.mocked(invoke);

function completeTurn(arg: unknown) {
  const onEvent = (arg as { onEvent: { onmessage: ((message: AgTurnEvent) => void) | null } }).onEvent;
  onEvent.onmessage?.({ type: "done", turn_id: "t", stop_reason: "stop" });
}

function dispatchOk(arg: unknown) {
  const command = (arg as { request?: { command?: string } }).request?.command;
  return { correlation_id: "ag-test", result: command === "ws.list_docs" ? [] : { ok: true } };
}

function mockTurnInvoke(handler: (arg: unknown) => void) {
  invokeMock.mockImplementation(async (command, arg) => {
    if (command === "dispatch") return dispatchOk(arg);
    handler(arg);
    return undefined;
  });
}

function agStartArg(): Record<string, unknown> {
  const found = invokeMock.mock.calls.find(([command]) => command === "ag_start_turn");
  if (!found) throw new Error("ag_start_turn was not called");
  return found[1] as Record<string, unknown>;
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
      snapshot_source: patch.snapshot_source ?? "active_editor",
      base_session_version: has("base_session_version") ? patch.base_session_version : null,
      base_content_hash: has("base_content_hash") ? patch.base_content_hash : null,
      style_ops: patch.style_ops ?? null,
      block_ops: patch.block_ops ?? null,
      table_ops: patch.table_ops ?? null,
      image_ops: patch.image_ops ?? null,
    },
  });
}

beforeEach(() => {
  invokeMock.mockReset();
  docServiceMock.ensureDocumentMediaDisplayBase.mockClear();
  docServiceMock.projectDocumentMediaDisplaySrc.mockClear();
  edLocateMock.getActiveEditor.mockReset();
  edLocateMock.getActiveEditor.mockReturnValue(null);
  edLocateMock.getActiveBlockSnapshotById.mockReset();
  edLocateMock.getActiveBlockSnapshotByIndex.mockReset();
  edLocateMock.getActiveDocumentContent.mockReset();
  edLocateMock.getActiveEditorBinding.mockReset();
  edLocateMock.locateBlock.mockReset();
  edLocateMock.getActiveDocumentContent.mockImplementation(() => edStore.getSnapshot().loadedContent);
  edLocateMock.getActiveEditorBinding.mockImplementation(() => {
    const ed = edStore.getSnapshot();
    return ed.openPath
      ? {
          relPath: ed.openPath,
          loadSeq: ed.loadSeq,
          editFormat: ed.editFormat,
          contentFormat: ed.contentFormat,
        }
      : null;
  });
  edLocateMock.locateBlock.mockReturnValue("located");
  edStore.reset();
  deStore.reset();
  agStore.reset();
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
      provider: undefined,
      model: undefined,
    });
    expect(result).toEqual({
      requestId: "as-1",
      candidate: "候选 1",
      candidates: ["候选 1", "候选 2", "候选 3"],
      failureReason: null,
    });
  });

  // covers: BR-AG-DATA-007
  // requestAssist 透传调用点按能力偏好解析出的 provider/model（仅标识符，不含密钥）。
  it("forwards provider/model into ag_request_assist", async () => {
    invokeMock.mockResolvedValue({
      request_id: "as-m",
      candidate: "候选",
      candidates: ["候选"],
      failure_reason: null,
    });

    await requestAssist({
      requestId: "as-m",
      kind: "inline",
      docPath: "notes/current.md",
      loadSeq: 7,
      blockId: "b1",
      start: 0,
      end: 2,
      instruction: "改短",
      selection: "原文",
      provider: "openai",
      model: "gpt-4o-mini",
    });

    expect(invokeMock).toHaveBeenCalledWith(
      "ag_request_assist",
      expect.objectContaining({ provider: "openai", model: "gpt-4o-mini" }),
    );
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

describe("agService 历史恢复", () => {
  // covers: BR-AG-PERSIST-001
  // 历史 diffRefs 必须经 DE all-status 查询恢复真实状态并锚回对应 assistant 消息；缺失 diff 不得复活为 pending。
  it("hydrates turn messages, inline tool summaries, and diff refs without reviving missing diffs", async () => {
    invokeMock.mockImplementation(async (command, arg) => {
      if (command !== "dispatch") return undefined;
      const request = (arg as { request: { command: string; params: Record<string, unknown> } }).request;
      if (request.command === "ag.list_sessions") {
        return {
          correlation_id: "hist",
          result: {
            sessions: [
              {
                session_id: "s1",
                title: "历史会话",
                mode: "edit",
                archived: false,
                open_index: 0,
                is_active: true,
                created_at: 1,
                updated_at: 2,
                last_active_at: 2,
              },
            ],
            active_session_id: "s1",
          },
        };
      }
      if (request.command === "de.list_diffs" && Array.isArray(request.params.diffIds)) {
        return {
          correlation_id: "hist",
          result: [
            {
              diff_id: "d-accepted",
              target: { rel_path: "notes/a.md" },
              status: "accepted",
              created_at: 4,
              projection: {
                patch_kind: "replace_document",
                before_text: "旧内容",
                after_text: "新内容",
                location_label: "历史修改建议",
              },
            },
            {
              diff_id: "d-image",
              target: { rel_path: "notes/a.docx" },
              status: "rejected",
              created_at: 5,
              projection: {
                patch_kind: "image_ops",
                before_text: "当前图片 1 张",
                after_text: "图片操作：插入 0 张，删除 1 张",
                location_label: "图片结构",
                image_ops: {
                  base: { sessionVersion: 3, contentHash: "h3" },
                  operations: [{
                    op: "delete_image",
                    imageId: "image_0_abcd",
                    fingerprint: "abcd",
                  }],
                },
                image_before: [{
                  imageId: "image_0_abcd",
                  fingerprint: "abcd",
                  ordinal: 0,
                  blockId: "b-img",
                  imagePos: 2,
                  standalone: true,
                  alt: "旧图",
                  width: "240",
                  height: null,
                  mediaRef: "media/image.png",
                  projectSourcePath: "images/image.png",
                  projectSourceHash: "a".repeat(64),
                }],
                image_after: [],
                image_affected_block_ids: ["b-img"],
              },
            },
          ],
        };
      }
      if (request.command === "de.list_diffs") {
        return { correlation_id: "hist", result: [] };
      }
      if (request.command === "ag.list_turns") {
        return {
          correlation_id: "hist",
          result: [
            {
              turn_id: "t1",
              session_id: "s1",
              rel_path: "notes/a.md",
              status: "completed",
              created_at: 3,
              updated_at: 5,
              payload: {
                version: 1,
                userMessage: { id: "u1", role: "user", content: "修改" },
                assistantMessage: { id: "a1", role: "assistant", content: "已生成建议", streaming: false },
                toolSummaries: [
                  {
                    id: "tc1",
                    name: "read_doc",
                    inputSummary: "notes/a.md",
                    resultSummary: "done",
                    state: "done",
                    anchorMessageId: "a1",
                  },
                ],
                diffRefs: [
                  { diffId: "d-accepted", relPath: "notes/a.md", anchorMessageId: "a1" },
                  { diffId: "d-image", relPath: "notes/a.docx", anchorMessageId: "a1" },
                  { diffId: "d-missing", relPath: "notes/missing.md", anchorMessageId: "a1" },
                ],
              },
            },
          ],
        };
      }
      return { correlation_id: "hist", result: { ok: true } };
    });

    await hydrateAgentHistory();

    const session = agStore.getSnapshot().sessions.find((s) => s.id === "s1");
    expect(session?.messages[0]).toMatchObject({
      id: "u1",
      relPath: "notes/a.md",
      turnId: "t1",
    });
    expect(session?.messages[1]).toMatchObject({
      id: "a1",
      relPath: "notes/a.md",
      toolCalls: [expect.objectContaining({ name: "read_doc", readonly: true, anchorMessageId: "a1" })],
    });
    expect(deStore.getSnapshot().diffs).toEqual([
      expect.objectContaining({
        diffId: "d-accepted",
        sessionId: "s1",
        turnId: "t1",
        anchorMessageId: "a1",
        state: "accepted",
        readonly: true,
        before: "旧内容",
        after: "新内容",
      }),
      expect.objectContaining({
        diffId: "d-image",
        state: "rejected",
        readonly: true,
        patchKind: "image_ops",
        imageOps: ["删除图片 image_0_abcd"],
        imageBefore: [expect.objectContaining({
          imageId: "image_0_abcd",
          projectSourcePath: "images/image.png",
        })],
        imageAfter: [],
        imageAffectedBlockIds: ["b-img"],
      }),
      expect.objectContaining({
        diffId: "d-missing",
        sessionId: "s1",
        turnId: "t1",
        anchorMessageId: "a1",
        state: "error",
        readonly: true,
        errorCode: "DE_DIFF_UNAVAILABLE",
      }),
    ]);
    expect(invokeMock).toHaveBeenCalledWith(
      "dispatch",
      expect.objectContaining({
        request: expect.objectContaining({
          command: "de.list_diffs",
          params: { diffIds: ["d-accepted", "d-image", "d-missing"] },
        }),
      }),
    );
  });

  // covers: BR-AG-PERSIST-001
  it("deletes one persisted agent session and persists remaining metadata", async () => {
    const first = agStore.getSnapshot().activeSessionId;
    const second = agStore.newSession();
    agStore.pushMessage(first, { id: "u-old", role: "user", content: "旧问题" });
    agStore.pushMessage(second, { id: "u-new", role: "user", content: "保留问题" });
    const calls: { command: string; params: Record<string, unknown> }[] = [];
    invokeMock.mockImplementation(async (command, arg) => {
      if (command !== "dispatch") return undefined;
      const request = (arg as { request: { command: string; params: Record<string, unknown> } }).request;
      calls.push({ command: request.command, params: request.params });
      return { correlation_id: "delete", result: { ok: true } };
    });

    await deleteAgentSession(first);

    expect(calls[0]).toMatchObject({ command: "ag.delete_session", params: { sessionId: first } });
    expect(calls.map((c) => c.command)).toContain("ag.save_session");
    const snap = agStore.getSnapshot();
    expect(snap.sessions).toHaveLength(1);
    expect(snap.sessions[0].id).toBe(second);
    expect(snap.sessions[0].messages.map((m) => m.id)).toEqual(["u-new"]);
  });

  it("rejects deleting a session while an agent turn is active", async () => {
    agStore.setSending(true);
    invokeMock.mockResolvedValue({ correlation_id: "delete", result: { ok: true } });

    await expect(deleteAgentSession(agStore.getSnapshot().activeSessionId)).rejects.toMatchObject({ code: "AG_TURN_ACTIVE" });
    expect(invokeMock).not.toHaveBeenCalled();
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

    mockTurnInvoke((arg) => {
      completeTurn(arg);
    });

    await startTurn("请结合 [引用:记忆 偏好] 修改。", {
      provider: "openai",
      model: "gpt-test",
      references,
      displayParts,
      mentions: ["legacy.md"],
      blockRefs: [{ relPath: "legacy.md", blockId: "b", text: "legacy" }],
    });

    const arg = agStartArg();
    expect(arg).toMatchObject({
      sessionId: agStore.getSnapshot().activeSessionId,
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
    mockTurnInvoke((arg) => {
      completeTurn(arg);
    });

    await startTurn("继续修改");

    const arg = agStartArg();
    expect(arg.activeDocContent).toBe("实时编辑器正文");
    expect(arg.openDocSnapshots).toEqual([
      expect.objectContaining({
        relPath: "notes/current.md",
        content: "实时编辑器正文",
        source: "active_editor",
      }),
    ]);
  });

  it("active editor 绑定未切到新文档时，使用新文档 session LogicalState 且先回写旧绑定文档", async () => {
    edStore.loadDoc("notes/a.md", "A 旧缓存", 1);
    const aLoadSeq = edStore.getSnapshot().loadSeq;
    edStore.loadDoc("notes/b.md", "B 当前 session", 2);
    edLocateMock.getActiveEditorBinding.mockReturnValue({ relPath: "notes/a.md", loadSeq: aLoadSeq, editFormat: "text" });
    edLocateMock.getActiveDocumentContent.mockReturnValue("A 实时未保存");
    mockTurnInvoke((arg) => {
      completeTurn(arg);
    });

    await startTurn("整理当前文档");

    const arg = agStartArg();
    expect(arg.activeDocPath).toBe("notes/b.md");
    expect(arg.activeDocContent).toBe("B 当前 session");
    expect(arg.openDocSnapshots).toEqual(expect.arrayContaining([
      expect.objectContaining({
        relPath: "notes/a.md",
        content: "A 实时未保存",
        source: "open_editor_session",
      }),
      expect.objectContaining({
        relPath: "notes/b.md",
        content: "B 当前 session",
        source: "active_editor_session",
      }),
    ]));
    expect(edStore.getSnapshot().sessions["notes/b.md"].content).toBe("B 当前 session");
  });

  it("active editor binding 缺失但 session 已就绪时，使用当前文档 session LogicalState", async () => {
    edStore.loadDoc("notes/current.md", "session 当前内容", 101);
    edLocateMock.getActiveEditorBinding.mockReturnValue(null);
    edLocateMock.getActiveDocumentContent.mockReturnValue("不可归属的编辑器内容");
    mockTurnInvoke((arg) => {
      completeTurn(arg);
    });

    await startTurn("整理当前文档");

    const arg = agStartArg();
    expect(arg.activeDocPath).toBe("notes/current.md");
    expect(arg.activeDocContent).toBe("session 当前内容");
    expect(arg.openDocSnapshots).toEqual([
      expect.objectContaining({
        relPath: "notes/current.md",
        content: "session 当前内容",
        source: "active_editor_session",
      }),
    ]);
  });

  it("新活动文档 session 尚未载入时短暂等待，载入后自动继续本轮发送", async () => {
    vi.useFakeTimers();
    try {
      edStore.loadDoc("notes/a.md", "A 缓存", 1);
      const aLoadSeq = edStore.getSnapshot().loadSeq;
      edStore.openDoc("notes/b.md");
      edLocateMock.getActiveEditorBinding.mockReturnValue({ relPath: "notes/a.md", loadSeq: aLoadSeq, editFormat: "text" });
      edLocateMock.getActiveDocumentContent.mockReturnValue("A 实时未保存");
      mockTurnInvoke((arg) => {
        completeTurn(arg);
      });

      const sending = startTurn("整理当前文档");
      await Promise.resolve();
      expect(invokeMock.mock.calls.some(([command]) => command === "ag_start_turn")).toBe(false);

      edStore.loadDoc("notes/b.md", "B 载入完成", 2);
      await vi.advanceTimersByTimeAsync(25);
      await sending;

      const arg = agStartArg();
      expect(arg.activeDocPath).toBe("notes/b.md");
      expect(arg.activeDocContent).toBe("B 载入完成");
      expect(arg.openDocSnapshots).toEqual(expect.arrayContaining([
        expect.objectContaining({ relPath: "notes/b.md", source: "active_editor_session" }),
      ]));
    } finally {
      vi.useRealTimers();
    }
  });

  it("回合 relPath 在发送时冻结，异步期间切换文档不污染历史保存", async () => {
    edStore.loadDoc("notes/a.md", "A 文档", 1);
    edStore.loadDoc("notes/b.md", "B 文档", 2);
    edStore.setActivePath("notes/a.md");
    mockTurnInvoke((arg) => {
      edStore.setActivePath("notes/b.md");
      completeTurn(arg);
    });

    await startTurn("总结当前文档");
    await Promise.resolve();

    const saveTurnRelPaths = invokeMock.mock.calls
      .filter(([command, arg]) =>
        command === "dispatch" &&
        (arg as { request?: { command?: string } }).request?.command === "ag.save_turn",
      )
      .map(([, arg]) => (arg as { request: { params: { relPath?: string } } }).request.params.relPath);
    expect(saveTurnRelPaths.length).toBeGreaterThan(0);
    expect(new Set(saveTurnRelPaths)).toEqual(new Set(["notes/a.md"]));
  });

  it("发送给后端的 history 保留每条消息的冻结 relPath，供后端按文档降权", async () => {
    edStore.loadDoc("notes/a.md", "A 文档", 1);
    mockTurnInvoke((arg) => {
      completeTurn(arg);
    });
    await startTurn("先总结 A");

    edStore.loadDoc("notes/b.md", "B 文档", 2);
    await startTurn("整理当前文档");

    const starts = invokeMock.mock.calls
      .filter(([command]) => command === "ag_start_turn")
      .map(([, arg]) => arg as Record<string, unknown>);
    expect(starts).toHaveLength(2);
    expect(starts[1]).toMatchObject({
      activeDocPath: "notes/b.md",
      history: [
        expect.objectContaining({
          role: "user",
          content: "先总结 A",
          relPath: "notes/a.md",
          turnId: expect.stringMatching(/^t-/),
        }),
      ],
    });
  });

  it("active 编辑器无法序列化时阻断 AgentTurn，不调用后端模型入口", async () => {
    edStore.loadDoc("notes/current.md", "旧快照", 101);
    edLocateMock.getActiveDocumentContent.mockReturnValue(null);
    invokeMock.mockResolvedValue({ correlation_id: "ag-test", result: { ok: true } });

    await startTurn("继续修改");

    expect(invokeMock.mock.calls.some(([command]) => command === "ag_start_turn")).toBe(false);
    const session = activeSession(agStore.getSnapshot());
    expect(session.messages.some((m) => m.error && m.content.includes("AG_ACTIVE_SNAPSHOT_REQUIRED"))).toBe(true);
  });

  it("diff_proposed 的 before/after 来自后端 projection，不取整篇 fallback", async () => {
    edStore.loadDoc("notes/current.md", "第一段\n第二段原文\n最后一段", 101);
    mockTurnInvoke((arg) => {
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
    mockTurnInvoke((arg) => {
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
    });

    await startTurn("把第二段改写");

    expect(edLocateMock.getActiveBlockSnapshotByIndex).not.toHaveBeenCalled();
    expect(edLocateMock.getActiveBlockSnapshotById).not.toHaveBeenCalled();
    expect(deStore.getSnapshot().diffs[0]).toMatchObject({ blockId: "b2", before: "第二段原文", patchKind: "replace_block" });
  });

  it("缺少后端 projection 时，diff_proposed 投影为 error 卡", async () => {
    edStore.loadDoc("notes/current.md", "第一段\n第二段", 101);
    mockTurnInvoke((arg) => {
      const onEvent = (arg as { onEvent: { onmessage: ((message: AgTurnEvent) => void) | null } }).onEvent;
      onEvent.onmessage?.({ type: "diff_proposed", turn_id: "t", diff_id: "diff-missing", rel_path: "notes/current.md" });
      completeTurn(arg);
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
    mockTurnInvoke((arg) => {
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
    mockTurnInvoke((arg) => {
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

  it("propose_edit 的 block_ops 投影保留精确 beforeNode/afterNode 执行真值", async () => {
    edStore.loadDoc(
      "notes/current.docx",
      '<ul><li><p data-block-id="p-1">第一条</p></li><li><p data-block-id="p-2">第二条</p></li></ul>',
      101,
      null,
      "html",
    );
    const blockOps = {
      payloadVersion: 1,
      base: { sessionVersion: 1, contentHash: "h1" },
      operations: [{
        op: "replace_node",
        blockId: "p-1",
        fingerprint: "block-v1-old",
        expectedNodeType: "paragraph",
        beforeNode: {
          type: "paragraph",
          attrs: { blockId: "p-1" },
          content: [{ type: "text", text: "第一条" }],
        },
        afterNode: {
          type: "paragraph",
          attrs: { blockId: "p-1" },
          content: [{ type: "text", text: "新第一条" }],
        },
      }],
    };
    mockTurnInvoke((arg) => {
      const onEvent = (arg as { onEvent: { onmessage: ((message: AgTurnEvent) => void) | null } }).onEvent;
      onEvent.onmessage?.({
        type: "tool_result",
        turn_id: "t",
        tool_name: "propose_edit",
        result: projectionResult({
          diff_id: "diff-block",
          rel_path: "notes/current.docx",
          patch_kind: "block_ops",
          block_id: "p-1",
          block_index: 2,
          before_text: "p-1: 第一条",
          after_text: "p-1: 新第一条",
          location_label: "Block 2",
          block_ops: blockOps,
        }),
        success: true,
      });
      onEvent.onmessage?.({
        type: "diff_proposed",
        turn_id: "t",
        diff_id: "diff-block",
        rel_path: "notes/current.docx",
      });
      completeTurn(arg);
    });

    await startTurn("把第一条改写");

    expect(deStore.getSnapshot().diffs[0]).toMatchObject({
      diffId: "diff-block",
      patchKind: "block_ops",
      blockId: "p-1",
      before: "p-1: 第一条",
      after: "p-1: 新第一条",
      blockOpsPayload: blockOps,
      blockOps: ["p-1 · paragraph"],
    });
  });

  it("不可信 style_ops 投影不会进入可裁决 Diff", async () => {
    edStore.loadDoc("notes/current.docx", '<p data-block-id="b1">正文</p>', 101, null, "html");
    mockTurnInvoke((arg) => {
      const onEvent = (arg as { onEvent: { onmessage: ((message: AgTurnEvent) => void) | null } }).onEvent;
      onEvent.onmessage?.({
        type: "tool_result",
        turn_id: "t",
        tool_name: "propose_edit",
        result: projectionResult({
          diff_id: "diff-invalid-style",
          rel_path: "notes/current.docx",
          patch_kind: "style_ops",
          block_id: "b1",
          block_index: 0,
          before_text: "正文",
          after_text: "（样式修改）",
          location_label: "样式修改",
          style_ops: [{ op: "set_mark", blockId: "b1", start: 0, end: 2, mark: { type: "sub" } }],
        }),
        success: true,
      });
      onEvent.onmessage?.({
        type: "diff_proposed",
        turn_id: "t",
        diff_id: "diff-invalid-style",
        rel_path: "notes/current.docx",
      });
      completeTurn(arg);
    });

    await startTurn("设置下标");

    expect(deStore.getSnapshot().diffs[0]).toMatchObject({
      diffId: "diff-invalid-style",
      state: "error",
      errorCode: "AG_STYLE_OPERATION_INVALID",
      styleOpsPayload: undefined,
    });
  });

  it("image_ops 投影生成带授权显示 URL的可执行 ImageOperation 载荷", async () => {
    edStore.loadDoc("notes/current.docx", '<p data-block-id="b1">正文</p>', 101, null, "html");
    const hash = "a".repeat(64);
    const imageOps = {
      base: { sessionVersion: 1, contentHash: "h1" },
        operations: [{
          op: "insert_image",
          anchorBlockId: "b1",
          position: "after",
          assetId: "asset_cover",
          sourcePath: "images/cover.png",
        sourceHash: hash,
        mediaRef: `media/binder-project-${hash}.png`,
        alt: "封面",
        width: 320,
      }],
    };
    mockTurnInvoke((arg) => {
      const onEvent = (arg as { onEvent: { onmessage: ((message: AgTurnEvent) => void) | null } }).onEvent;
      onEvent.onmessage?.({
        type: "tool_result",
        turn_id: "t",
        tool_name: "propose_edit",
        result: projectionResult({
          diff_id: "diff-image",
          rel_path: "notes/current.docx",
          patch_kind: "image_ops",
          block_id: "b1",
          block_index: 0,
          before_text: "当前图片 0 张",
          after_text: "图片操作：插入 1 张，删除 0 张",
          location_label: "图片结构",
          image_ops: imageOps,
        }),
        success: true,
      });
      onEvent.onmessage?.({
        type: "diff_proposed",
        turn_id: "t",
        diff_id: "diff-image",
        rel_path: "notes/current.docx",
      });
      completeTurn(arg);
    });

    await startTurn("插入项目封面");

    expect(deStore.getSnapshot().diffs[0]).toMatchObject({
      diffId: "diff-image",
      patchKind: "image_ops",
      state: "pending",
      imageOps: ["插入 images/cover.png · 锚点后"],
      imageOpsPayload: {
        base: { sessionVersion: 1, contentHash: "h1" },
        operations: [{
          op: "insert_image",
          assetId: "asset_cover",
          sourcePath: "images/cover.png",
          mediaRef: `media/binder-project-${hash}.png`,
          displaySrc: `asset://document/notes/current.docx/media/binder-project-${hash}.png`,
        }],
      },
    });
    expect(docServiceMock.ensureDocumentMediaDisplayBase).toHaveBeenCalledWith(
      "notes/current.docx",
      '<p data-block-id="b1">正文</p>',
      101,
    );
  });

  it("显式 replace_document 投影为整篇 diff，供 Editor 执行整篇草稿替换", async () => {
    edStore.loadDoc("notes/current.md", "旧全文", 101);
    edLocateMock.getActiveDocumentContent.mockReturnValue("实时旧全文");
    mockTurnInvoke((arg) => {
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

  it("DiffProjection 保留 active_editor_session source，不降级为 active_editor", async () => {
    edStore.loadDoc("notes/current.md", "旧全文", 101);
    mockTurnInvoke((arg) => {
      const onEvent = (arg as { onEvent: { onmessage: ((message: AgTurnEvent) => void) | null } }).onEvent;
      onEvent.onmessage?.({
        type: "tool_result",
        turn_id: "t",
        tool_name: "propose_edit",
        result: projectionResult({ diff_id: "diff-session", snapshot_source: "active_editor_session" }),
        success: true,
      });
      onEvent.onmessage?.({ type: "diff_proposed", turn_id: "t", diff_id: "diff-session", rel_path: "notes/current.md" });
      completeTurn(arg);
    });

    await startTurn("改写当前文档");

    expect(deStore.getSnapshot().diffs[0]).toMatchObject({
      diffId: "diff-session",
      snapshotSource: "active_editor_session",
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

// Migrated intact from agStore.test.ts.
describe("agStore migrated suite", () => {
  beforeEach(() => {
    agStore.reset();
  });

  describe("agStore SM-AG-01 回合投影（Task 2-E · X-FORBID-007）", () => {
    it("applyTurnState：活动态→sending=true；终态/错误/Idle→sending=false", () => {
      // 活动态门控输入
      for (const s of ["Assembling", "AwaitingGate", "Streaming", "HandingOff"] as const) {
        agStore.applyTurnState("t", s);
        expect(agStore.getSnapshot().sending).toBe(true);
      }
      // 终态与错误态释放门控
      for (const s of ["Completed", "Cancelled", "Denied", "HandedOver", "TurnError", "Idle"] as const) {
        agStore.applyTurnState("t", "Streaming"); // 先置 true
        agStore.applyTurnState("t", s);
        expect(agStore.getSnapshot().sending).toBe(false);
      }
    });

    it("setSending 乐观置 true 仍可用（点击即门控防双发）", () => {
      agStore.setSending(true);
      expect(agStore.getSnapshot().sending).toBe(true);
    });
  });

  describe("agStore 多标签会话（PI-AG-UI-01，呈现层组织）", () => {
    it("newSession 新建并激活；switchSession 切换；消息按会话隔离且首条用户消息成为标题", () => {
      const a = agStore.getSnapshot().activeSessionId;
      agStore.pushMessage(a, { id: "m1", role: "user", content: "给第一章润色一下" });
      const b = agStore.newSession();
      expect(agStore.getSnapshot().activeSessionId).toBe(b);
      agStore.pushMessage(b, { id: "m2", role: "user", content: "写一份提纲" });

      const snap = agStore.getSnapshot();
      const sa = snap.sessions.find((s) => s.id === a)!;
      const sb = snap.sessions.find((s) => s.id === b)!;
      expect(sa.messages.map((m) => m.id)).toEqual(["m1"]);
      expect(sb.messages.map((m) => m.id)).toEqual(["m2"]);
      expect(sa.title).toBe("给第一章润色一下");
      expect(sb.title).toBe("写一份提纲");

      agStore.switchSession(a);
      expect(activeSession(agStore.getSnapshot()).id).toBe(a);
    });

    it("流式分片按会话定位：切走标签后 append/finish 仍写入原会话（不串会话）", () => {
      const a = agStore.newSession();
      agStore.pushMessage(a, { id: "as1", role: "assistant", content: "", streaming: true });
      agStore.newSession(); // 用户切到新标签
      agStore.appendToMessage(a, "as1", "你好");
      agStore.finishMessage(a, "as1");
      const sa = agStore.getSnapshot().sessions.find((s) => s.id === a)!;
      expect(sa.messages[0].content).toBe("你好");
      expect(sa.messages[0].streaming).toBe(false);
    });

    it("closeSession=归档可重开；关闭活动标签切换邻近；关闭最后一个自动新建空会话", () => {
      const before = agStore.getSnapshot();
      // 全部关闭
      for (const s of before.sessions.filter((x) => !x.archived)) agStore.closeSession(s.id);
      const snap = agStore.getSnapshot();
      const open = snap.sessions.filter((s) => !s.archived);
      expect(open.length).toBe(1); // 自动兜底新建
      expect(open[0].messages.length).toBe(0);
      // 归档的可经 switchSession 重开
      const archived = snap.sessions.find((s) => s.archived)!;
      agStore.switchSession(archived.id);
      const re = agStore.getSnapshot();
      expect(re.activeSessionId).toBe(archived.id);
      expect(re.sessions.find((s) => s.id === archived.id)!.archived).toBe(false);
    });

    it("deleteSession 直接移除会话；删除最后一个会自动新建空会话", () => {
      const first = agStore.getSnapshot().activeSessionId;
      const second = agStore.newSession();
      agStore.pushMessage(second, { id: "u2", role: "user", content: "待删除" });

      agStore.deleteSession(first);
      let snap = agStore.getSnapshot();
      expect(snap.sessions.map((s) => s.id)).toEqual([second]);
      expect(snap.activeSessionId).toBe(second);

      agStore.deleteSession(second);
      snap = agStore.getSnapshot();
      expect(snap.sessions).toHaveLength(1);
      expect(snap.sessions[0].messages).toEqual([]);
      expect(snap.activeSessionId).toBe(snap.sessions[0].id);
    });

    it("setMode 作用于活动会话（编辑/构建互不影响其他会话）", () => {
      const a = agStore.newSession();
      agStore.setMode("build");
      const b = agStore.newSession();
      const snap = agStore.getSnapshot();
      expect(snap.sessions.find((s) => s.id === a)!.mode).toBe("build");
      expect(snap.sessions.find((s) => s.id === b)!.mode).toBe("edit");
    });

    it("输入框草稿按会话隔离保存和恢复", () => {
      const first = agStore.getSnapshot().activeSessionId;
      agStore.setComposerDraft(first, {
        ...emptyComposerDraft(),
        doc: { type: "doc", content: [{ type: "paragraph", content: [{ type: "text", text: "A 草稿" }] }] },
        prompt: "A 草稿",
      });

      const second = agStore.newSession();
      expect(activeSession(agStore.getSnapshot()).composerDraft?.prompt).toBe("");
      agStore.setComposerDraft(second, {
        ...emptyComposerDraft(),
        doc: { type: "doc", content: [{ type: "paragraph", content: [{ type: "text", text: "B 草稿" }] }] },
        prompt: "B 草稿",
      });

      agStore.switchSession(first);
      expect(activeSession(agStore.getSnapshot()).composerDraft?.prompt).toBe("A 草稿");
      agStore.switchSession(second);
      expect(activeSession(agStore.getSnapshot()).composerDraft?.prompt).toBe("B 草稿");
    });

    it("工具卡按会话隔离：addToolCall/resolveToolCall/clearToolCalls", () => {
      const a = agStore.newSession();
      agStore.addToolCall(a, "read_doc", "a.md");
      agStore.resolveToolCall(a, "read_doc", true);
      const sa = () => agStore.getSnapshot().sessions.find((s) => s.id === a)!;
      expect(sa().toolCalls[0].state).toBe("done");
      agStore.clearToolCalls(a);
      expect(sa().toolCalls.length).toBe(0);
    });

    it("hydrateSessions + hydrateSessionTurns 支持历史元数据先恢复、内容后加载", () => {
      agStore.hydrateSessions(
        [
          { id: "s1", title: "历史", mode: "edit", messages: [], toolCalls: [], createdAt: 1, archived: false, turnsLoaded: false },
          { id: "s2", title: "归档", mode: "build", messages: [], toolCalls: [], createdAt: 2, archived: true, turnsLoaded: false },
        ],
        "s1",
      );
      expect(agStore.getSnapshot().activeSessionId).toBe("s1");
      expect(agStore.getSnapshot().sessions[0].turnsLoaded).toBe(false);
      agStore.hydrateSessionTurns("s1", [{ id: "u", role: "user", content: "继续", turnId: "t", turnStatus: "draft" }], []);
      const s1 = agStore.getSnapshot().sessions[0];
      expect(s1.turnsLoaded).toBe(true);
      expect(s1.messages[0].turnStatus).toBe("draft");
    });

    it("消息保留 relPath 供后端历史按文档归属分流", () => {
      const sessionId = agStore.getSnapshot().activeSessionId;

      agStore.pushMessage(sessionId, {
        id: "u-doc",
        role: "user",
        content: "整理当前文档",
        turnId: "t-doc",
        relPath: "notes/current.md",
        turnStatus: "completed",
      });
      agStore.setTurnStatus(sessionId, "t-doc", "completed");

      const message = agStore.getSnapshot().sessions[0].messages[0];
      expect(message.relPath).toBe("notes/current.md");
      expect(message.turnStatus).toBe("completed");
    });
  });
});

// covers: BR-AG-DATA-007
// MODEL-CONFIG-01 丢失回归（2026-08-07）：Editor 辅助续写调用点的 autocompleteModel 段曾因并行改动
// 在提交时静默丢失（requestAssist 的 provider/model 为可选参数，tsc 不报错，功能静默降级）。
// 该段在组件闭包内、依赖 DOM 布局测量（jsdom 无法稳定集成渲染），故用源码契约守卫调用点接线：
// openAutocompleteAssist 必须按 autocomplete 能力偏好解析模型并透传 provider/model。
describe("Editor autocomplete 调用点接线（MODEL-CONFIG-01 丢失回归）", () => {
  it("openAutocompleteAssist 解析能力偏好模型并透传", () => {
    // vite raw glob：直接读取 Editor.tsx 源码文本（避免 node:fs 类型依赖）。
    const editorSources = import.meta.glob("../components/ed/Editor.tsx", {
      query: "?raw",
      import: "default",
      eager: true,
    }) as Record<string, string>;
    const editorSource = editorSources["../components/ed/Editor.tsx"] ?? "";
    expect(editorSource).not.toBe("");
    // 能力偏好解析（BR-AG-DATA-007：跟随全局或显式指定）。
    expect(editorSource).toContain('const autocompleteModel = modelStore.resolveFor("autocomplete")');
    // 透传到 requestAssist（provider/model 与解析结果一致）。
    expect(editorSource).toContain("provider: autocompleteModel.provider");
    expect(editorSource).toContain("model: autocompleteModel.model");
  });
});
