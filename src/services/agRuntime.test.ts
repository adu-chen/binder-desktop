import { beforeEach, describe, expect, it, vi } from "vitest";
import { closeStore } from "../store/closeStore";
import { modelKey, modelStore } from "../store/modelStore";

const tauriMock = vi.hoisted(() => {
  const seqByTurn = new Map<string, number>();
  class MockChannel<T> {
    private consumer: ((message: T) => void) | null = null;
    private turnId = "";
    private seq = 0;
    private pendingToolIds = new Map<string, string[]>();

    setTurnId(turnId: string) {
      this.turnId = turnId;
    }

    set onmessage(consumer: ((message: T) => void) | null) {
      this.consumer = consumer;
    }

    get onmessage(): ((message: T) => void) | null {
      const consumer = this.consumer;
      if (!consumer) return null;
      return (message: T) => {
        const raw = message as Record<string, unknown>;
        const type = String(raw.type ?? "");
        const previousSeq = this.turnId ? (seqByTurn.get(this.turnId) ?? 0) : this.seq;
        const nextSeq = typeof raw.seq === "number" ? raw.seq : previousSeq + 1;
        this.seq = Math.max(this.seq, nextSeq);
        if (this.turnId) seqByTurn.set(this.turnId, Math.max(previousSeq, nextSeq));
        const toolName = String(raw.toolName ?? raw.tool_name ?? "");
        let toolCallId = String(raw.toolCallId ?? raw.tool_call_id ?? "");
        if (type === "tool_request" && !toolCallId) {
          toolCallId = `call-${nextSeq}`;
          this.pendingToolIds.set(toolName, [...(this.pendingToolIds.get(toolName) ?? []), toolCallId]);
        } else if (type === "tool_result" && !toolCallId) {
          const pending = this.pendingToolIds.get(toolName) ?? [];
          toolCallId = pending.shift() ?? `call-${nextSeq}`;
          this.pendingToolIds.set(toolName, pending);
        }
        const diffId = String(raw.diffId ?? raw.diff_id ?? "");
        consumer({
          ...raw,
          eventId: raw.eventId ?? `event-${nextSeq}`,
          turnId: this.turnId || String(raw.turnId ?? raw.turn_id ?? ""),
          seq: nextSeq,
          occurredAt: raw.occurredAt ?? nextSeq,
          payloadVersion: raw.payloadVersion ?? 1,
          toolCallId,
          toolName,
          evidenceIds: raw.evidenceIds ?? [],
          diffId,
          proposalBatchId: raw.proposalBatchId ?? diffId,
          groupIndex: raw.groupIndex ?? 1,
          groupCount: raw.groupCount ?? 1,
          relPath: raw.relPath ?? raw.rel_path,
          taskRunId: raw.taskRunId ?? raw.task_run_id,
          requestId: raw.requestId ?? raw.request_id,
          stopReason: raw.stopReason ?? raw.stop_reason,
        } as T);
      };
    }
  }

  return {
    Channel: MockChannel,
    resetChannelState: () => seqByTurn.clear(),
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
  isDocxEditable: vi.fn(() => false),
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
import { tabsStore } from "../store/tabsStore";
import { workspaceShell } from "../store/workspaceShell";
import type { MessageContentPart, ReferencePayload } from "./referencePayload";
import {
  AGENT_EDIT_RUNTIME_PROFILE,
  AGENT_CONTEXT_FACTS_VERSION,
  acknowledgePreviewAndContinue,
  cancelActiveTurn,
  deleteAgentSession,
  finishAssist,
  hydrateAgentHistory,
  refreshSessionTaskRuns,
  refreshSessionPlans,
  resolveDecision,
  requestAssist,
  resolveDecisionAndContinue,
  startTurn,
  buildAgentContextFacts,
  validateToolCallReceipt,
  queryTurnExecutionEvidence,
  inspectTurnEffects,
} from "./agService";

const invokeMock = vi.mocked(invoke);

// Transport stimulus only: exercises the production host collector, not a replacement for it.
function requestContext(arg: unknown, paths: string[], overrides: Record<string, unknown> = {}) {
  const a = arg as { turnId: string; sessionId: string; agentContextFacts: { workspaceId: string; contextRevision: number }; onContextRequest: { onmessage: (r: unknown) => void } };
  a.onContextRequest.onmessage({ requestId: `context-${a.turnId}`, workspaceId: a.agentContextFacts.workspaceId, sessionId: a.sessionId, turnId: a.turnId, contextRevision: a.agentContextFacts.contextRevision + 1, paths, allDocuments: false, ...overrides });
}

function lastContextResponse() {
  const calls = invokeMock.mock.calls.filter(([command]) => command === "ag_supply_context");
  const call = calls[calls.length - 1];
  expect(call, "host must send a context response").toBeDefined();
  return (call![1] as { response: { snapshots: Array<Record<string, unknown>>; facts: { contextRevision: number; activeTarget: unknown; openResources: Array<Record<string, unknown>> } } }).response;
}
type TestTurnEvent = Record<string, unknown>;
const diffDetailsById = new Map<string, Record<string, unknown>>();

function bindTurnChannel(invocation: Record<string, unknown>) {
  const turnId = typeof invocation.turnId === "string" ? invocation.turnId : "";
  const channel = invocation.onEvent as { setTurnId?: (value: string) => void } | undefined;
  if (turnId) channel?.setTurnId?.(turnId);
}

function completeTurn(arg: unknown) {
  bindTurnChannel(arg as Record<string, unknown>);
  const onEvent = (arg as { onEvent: { onmessage: ((message: TestTurnEvent) => void) | null } }).onEvent;
  onEvent.onmessage?.({ type: "done", turn_id: "t", stop_reason: "stop" });
}

function dispatchOk(arg: unknown) {
  const request = (arg as {
    request?: { command?: string; params?: { diffIds?: string[]; outcome?: string } };
  }).request;
  const command = request?.command;
  if (command === "ws.list_docs") return { correlation_id: "ag-test", result: [] };
  if (command === "de.list_diffs") {
    return {
      correlation_id: "ag-test",
      result: (request?.params?.diffIds ?? []).flatMap((id) => {
        const detail = diffDetailsById.get(id);
        return detail ? [detail] : [];
      }),
    };
  }
  if (command === "ag.resolve_decision") {
    return {
      correlation_id: "ag-test",
      result: {
        task_run_id: "task-confirmed",
        status: request?.params?.outcome === "cancel" ? "cancelled" : "waiting",
      },
    };
  }
  return { correlation_id: "ag-test", result: { ok: true } };
}

function mockTurnInvoke(handler: (arg: unknown) => void) {
  invokeMock.mockImplementation(async (command, arg) => {
    if (command === "dispatch") return dispatchOk(arg);
    if (command === "ag_supply_context") return undefined;
    const invocation = arg as Record<string, unknown>;
    if (command === "ag_start_turn") bindTurnChannel(invocation);
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
  const result = {
    summary: "已生成修改建议",
    diff_id: patch.diff_id ?? "diff-1",
    rel_path: patch.rel_path ?? "notes/current.md",
    projection: {
      diff_id: patch.diff_id ?? "diff-1",
      proposal_batch_id: patch.proposal_batch_id ?? patch.diff_id ?? "diff-1",
      group_index: patch.group_index ?? 1,
      group_count: patch.group_count ?? 1,
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
  };
  const diffId = String(result.diff_id);
  diffDetailsById.set(diffId, {
    diff_id: diffId,
    target: { rel_path: result.rel_path },
    status: "pending",
    created_at: 1,
    projection: result.projection,
  });
  return JSON.stringify(result);
}

beforeEach(() => {
  invokeMock.mockReset();
  tauriMock.resetChannelState();
  diffDetailsById.clear();
  docServiceMock.ensureDocumentMediaDisplayBase.mockClear();
  docServiceMock.isDocxEditable.mockClear();
  docServiceMock.isDocxEditable.mockReturnValue(false);
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
  tabsStore.reset();
  workspaceShell._projectActive({ workspaceId: "ws-test", rootPath: "/tmp/ws-test" });
});

describe("AgentContextFacts", () => {
  // covers: BR-AG-DATA-007
  it("rejects an unavailable selected AgentEdit model before invoking the backend", async () => {
    const selected = modelStore.getSnapshot().models[1];
    const selectedKey = modelKey(selected);
    modelStore.setCapabilityPref("agentEdit", { mode: "explicit", modelKey: selectedKey });
    modelStore.removeModel(selectedKey);
    try {
      await expect(startTurn("你好")).rejects.toThrow("AG_MODEL_UNAVAILABLE");
      expect(invokeMock.mock.calls.some(([command]) => command === "ag_start_turn")).toBe(false);
    } finally {
      modelStore.addModel(selected);
      modelStore.setCapabilityPref("agentEdit", { mode: "followGlobal" });
    }
  });
  // covers: BR-AG-DATA-008
  it("v2 greeting freezes identity without serializing an unready rich editor", async () => {
    edStore.openDoc("loading.docx");
    mockTurnInvoke(completeTurn);
    await startTurn("你好");
    expect(agStartArg()).toMatchObject({ activeDocPath: "loading.docx", openDocSnapshots: [], agentContextFacts: { version: "agent-context-facts-v2", contextRevision: 1, activeTarget: { relPath: "loading.docx" }, openResources: [{ relPath: "loading.docx", availability: "not_requested", snapshot: null }] } });
    expect(edLocateMock.getActiveDocumentContent).not.toHaveBeenCalled();
    expect(invokeMock.mock.calls.some(([command]) => command === "ag_supply_context")).toBe(false);
  });

  // covers: BR-AG-DATA-008
  // covers: BR-SYS-STATE-007
  it("context requests cannot cross session or survive a terminal event", async () => {
    edStore.loadDoc("a.txt", "logical", 1);
    mockTurnInvoke((arg) => {
      requestContext(arg, ["a.txt"], { sessionId: "other-session" });
      expect(invokeMock.mock.calls.some(([command]) => command === "ag_supply_context")).toBe(false);
      completeTurn(arg);
    });
    await startTurn("你好");
    requestContext(agStartArg(), ["a.txt"]);
    expect(invokeMock.mock.calls.some(([command]) => command === "ag_supply_context")).toBe(false);
    expect(edLocateMock.getActiveDocumentContent).not.toHaveBeenCalled();
  });

  // covers: BR-AG-DATA-001
  // covers: BR-AG-DATA-008
  it("closed cached sessions remain unavailable for the frozen turn, not ready snapshots", async () => {
    edStore.loadDoc("a.txt", "cached logical", 1);
    mockTurnInvoke((arg) => {
      edStore.closeSession("a.txt");
      expect(edStore.getSnapshot().sessions["a.txt"].content).toBe("cached logical");
      requestContext(arg, ["a.txt"]);
      expect(lastContextResponse().snapshots).toEqual([]);
      expect(lastContextResponse().facts.openResources).toContainEqual({ relPath: "a.txt", availability: "unavailable", reasonCode: "AG_CONTEXT_RESOURCE_CLOSED", snapshot: null });
      expect(lastContextResponse().facts.activeTarget).toMatchObject({ relPath: "a.txt" });
      completeTurn(arg);
    });
    await startTurn("读取当前文档");
  });

  // covers: BR-AG-DATA-008
  // covers: BR-ED-DATA-008
  // covers: BR-SYS-DATA-008
  it("freezes LogicalState session facts, explicit selection, and proposal state without document body", () => {
    edStore.restoreSnapshots([
      {
        relPath: "a.md",
        content: "unsaved logical body",
        baseline: 7,
        editFormat: "markdown",
        contentFormat: "text",
        source: "active_editor",
        dirty: true,
        sessionVersion: 3,
        contentHash: "h3",
        capturedAt: 1,
      },
    ], "a.md");
    deStore.addDiff({
      diffId: "diff-1",
      sessionId: "session-1",
      relPath: "a.md",
      blockId: "b1",
      title: "建议",
      before: "前",
      after: "后",
      state: "pending",
    });
    const openDocs = edStore.getOpenSnapshots();
    deStore.addDiff({
      diffId: "diff-other-session",
      sessionId: "session-other",
      relPath: "other.md",
      blockId: "b2",
      title: "其他会话建议",
      before: "前",
      after: "后",
      state: "pending",
    });
    const facts = buildAgentContextFacts("turn-1", "session-1", openDocs, [{
      kind: "selection",
      relPath: "a.md",
      blockId: "b1",
      start: 0,
      end: 2,
      text: "选区",
    }], [{ relPath: "a.md", blockId: "legacy", text: "旧选区" }]);

    expect(facts).toMatchObject({
      version: AGENT_CONTEXT_FACTS_VERSION,
      workspaceId: "ws-test",
      turnId: "turn-1",
      activeDoc: {
        relPath: "a.md",
        source: "active_editor",
        dirty: true,
        baseline: "7",
        sessionVersion: 3,
        contentHash: "h3",
      },
      selections: [
        { relPath: "a.md", blockId: "b1", start: 0, end: 2 },
        { relPath: "a.md", blockId: "legacy" },
      ],
      proposalFacts: [{
        diffId: "diff-1",
        sessionId: "session-1",
        relPath: "a.md",
        state: "pending",
        saved: false,
      }],
    });
    expect(facts.proposalFacts).toHaveLength(1);
    expect(JSON.stringify(facts)).not.toContain("unsaved logical body");
  });
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
  // covers: BR-SYS-STATE-007
  it("重启将 failure 恢复到未完成提示，保持 assistant 正文原样且不自动续跑", async () => {
    const failure = { code: "AG_EVENT_DB_ERROR", message: "事件表不可用", retriable: true };
    invokeMock.mockImplementation(async (command, arg) => {
      if (command !== "dispatch") return undefined;
      const request = (arg as { request: { command: string } }).request;
      if (request.command === "ag.list_sessions") return {
        correlation_id: "hist", result: { active_session_id: "recovered", sessions: [{
          session_id: "recovered", title: "历史", mode: "edit", archived: false, open_index: 0,
          is_active: true, created_at: 1, updated_at: 2,
        }] },
      };
      if (request.command === "ag.list_turns") return {
        correlation_id: "hist", result: [{
          turn_id: "unfinished", session_id: "recovered", status: "draft", created_at: 1, updated_at: 2,
          payload: { version: 2, lastEventSeq: 0,
            userMessage: { id: "u", role: "user", content: "你好" },
            assistantMessage: { id: "a", role: "assistant", content: "" },
            toolSummaries: [], diffRefs: [], failure, incompleteReason: "stream_interrupted",
          },
        }],
      };
      return { correlation_id: "hist", result: [] };
    });
    await hydrateAgentHistory();
    const messages = activeSession(agStore.getSnapshot()).messages;
    expect(messages).toHaveLength(2);
    expect(messages[0]).toMatchObject({ turnStatus: "draft", turnFailure: failure });
    expect(messages[1].content).toBe("");
    expect(invokeMock.mock.calls.some(([command]) => command === "ag_start_turn")).toBe(false);
  });

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
                proposal_batch_id: "d-accepted",
                group_index: 1,
                group_count: 1,
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
                proposal_batch_id: "d-image",
                group_index: 1,
                group_count: 1,
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
      if (request.command === "ag.list_turn_events") {
        return {
          correlation_id: "hist",
          result: [{
            eventId: "event-report",
            turnId: "t1",
            seq: 7,
            occurredAt: 7,
            payloadVersion: 1,
            type: "project_report",
            report: {
              version: "project-report-v2",
              turnId: "t1",
              evidenceThroughSeq: 6,
              requestedScope: ["notes/a.md", "notes/unread.md"],
              readItems: [{ relPath: "notes/a.md", source: "read_doc", coverage: "full_document", evidenceIds: ["e1"] }],
              unreadItems: [{ relPath: "notes/unread.md", reason: "not_read" }],
              findings: [],
              uncertainties: ["notes/unread.md 未读取"],
            },
          }],
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
    expect(agStore.getSnapshot().projectReports.t1).toMatchObject({
      evidenceThroughSeq: 6,
      unreadItems: [{ relPath: "notes/unread.md", reason: "not_read" }],
    });
    expect(agStore.lastTurnEventSeq("t1")).toBe(7);
    expect(invokeMock.mock.calls.some(([command]) => command === "ag_start_turn")).toBe(false);
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
    agStore.setActiveTurn("session-active", "turn-active");
    invokeMock.mockResolvedValue({ correlation_id: "delete", result: { ok: true } });

    await expect(deleteAgentSession(agStore.getSnapshot().activeSessionId)).rejects.toMatchObject({ code: "AG_TURN_ACTIVE" });
    expect(invokeMock).not.toHaveBeenCalled();
    invokeMock.mockResolvedValue(undefined);
    await cancelActiveTurn();
    expect(invokeMock).toHaveBeenCalledWith("ag_cancel_turn", {
      sessionId: "session-active",
      turnId: "turn-active",
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

    mockTurnInvoke((arg) => {
      const onEvent = (arg as { onEvent: { onmessage: ((message: TestTurnEvent) => void) | null } }).onEvent;
      onEvent.onmessage?.({
        type: "error",
        turn_id: "t",
        code: "AG_PROVIDER_FAILED",
        message: "provider failed",
        retriable: true,
      });
      onEvent.onmessage?.({ type: "done", turn_id: "t", stop_reason: "stop" });
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
      runtimeProfile: AGENT_EDIT_RUNTIME_PROFILE,
      prompt: "请结合 [引用:记忆 偏好] 修改。",
      provider: "openai",
      model: "gpt-test",
      activeDocPath: "notes/current.md",
      activeDocContent: undefined,
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
    const turnStatuses = session.messages
      .filter((message) => message.turnId)
      .map((message) => message.turnStatus);
    expect(turnStatuses).toContain("error");
    expect(turnStatuses).not.toContain("completed");
    expect(agStore.getSnapshot().activeTurn).toBeNull();

    // H07: the real service assembles history from the originating AgentSession;
    // explicit memory is sent for only its selected turn, never persisted as history.
    mockTurnInvoke(completeTurn);
    const sessionA = agStore.newSession();
    const latestTurnArg = () => {
      const calls = invokeMock.mock.calls.filter(([command]) => command === "ag_start_turn");
      return calls[calls.length - 1][1] as Record<string, unknown>;
    };
    await startTurn("H07-A-OLD");
    await startTurn("H07-A-CORRECTED", { references: [{ kind: "memory", entryId: "h07", title: "项目代号", content: "H07-MEMORY" }] });
    const selected = latestTurnArg();
    expect(selected.sessionId).toBe(sessionA);
    expect(selected.history).toEqual(expect.arrayContaining([expect.objectContaining({ sessionId: sessionA, content: "H07-A-OLD" })]));
    expect(selected.references).toEqual([expect.objectContaining({ kind: "memory", content: "H07-MEMORY" })]);
    await startTurn("H07-A-NO-SELECTION");
    const unselected = latestTurnArg();
    expect(unselected.references).toEqual([]);
    expect(unselected.history).toEqual(expect.arrayContaining([expect.objectContaining({ sessionId: sessionA, content: "H07-A-CORRECTED" })]));
    expect(JSON.stringify(unselected.history)).not.toContain("H07-MEMORY");
    const sessionB = agStore.newSession();
    await startTurn("H07-B-ONLY");
    const other = latestTurnArg();
    expect(other.sessionId).toBe(sessionB);
    expect(other.history).toEqual([]);
    expect(other.references).toEqual([]);
    agStore.switchSession(sessionA);
    await startTurn("H07-A-RETURN");
    const returned = latestTurnArg();
    expect(returned.sessionId).toBe(sessionA);
    expect(JSON.stringify(returned.history)).toContain("H07-A-CORRECTED");
    expect(JSON.stringify(returned.history)).not.toContain("H07-B-ONLY");
  });

  it("v2 准入不采集正文，按需读取优先取当前编辑器实时正文", async () => {
    edStore.loadDoc("notes/current.md", "旧快照", 101);
    edLocateMock.getActiveDocumentContent.mockReturnValue("实时编辑器正文");
    mockTurnInvoke((arg) => {
      requestContext(arg, ["notes/current.md"]);
      completeTurn(arg);
    });

    await startTurn("继续修改");

    const arg = agStartArg();
    expect(arg.activeDocContent).toBeUndefined();
    expect(arg.openDocSnapshots).toEqual([]);
    expect(lastContextResponse().snapshots).toEqual([
      expect.objectContaining({
        relPath: "notes/current.md",
        content: "实时编辑器正文",
        source: "active_editor",
      }),
    ]);
  });

  it("active editor 绑定未切到新文档时，使用新文档 session LogicalState 且先回写旧绑定文档", async () => {
    edStore.loadDoc("notes/a.md", "A 旧缓存", 1);
    tabsStore.openDoc("notes/a.md");
    const aLoadSeq = edStore.getSnapshot().loadSeq;
    edStore.loadDoc("notes/b.md", "B 当前 session", 2);
    edLocateMock.getActiveEditorBinding.mockReturnValue({ relPath: "notes/a.md", loadSeq: aLoadSeq, editFormat: "text" });
    edLocateMock.getActiveDocumentContent.mockReturnValue("A 实时未保存");
    mockTurnInvoke((arg) => {
      requestContext(arg, ["notes/a.md", "notes/b.md"]);
      completeTurn(arg);
    });

    await startTurn("整理当前文档");

    const arg = agStartArg();
    expect(arg.activeDocPath).toBe("notes/b.md");
    expect(arg.activeDocContent).toBeUndefined();
    expect(lastContextResponse().snapshots).toEqual(expect.arrayContaining([
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
      requestContext(arg, ["notes/current.md"]);
      completeTurn(arg);
    });

    await startTurn("整理当前文档");

    const arg = agStartArg();
    expect(arg.activeDocPath).toBe("notes/current.md");
    expect(arg.activeDocContent).toBeUndefined();
    expect(lastContextResponse().snapshots).toEqual([
      expect.objectContaining({
        relPath: "notes/current.md",
        content: "session 当前内容",
        source: "active_editor_session",
      }),
    ]);
  });

  it("新活动文档未载入不阻断准入，按需采集明确返回 loading", async () => {
    vi.useFakeTimers();
    try {
      edStore.loadDoc("notes/a.md", "A 缓存", 1);
      const aLoadSeq = edStore.getSnapshot().loadSeq;
      edStore.openDoc("notes/b.md");
      edLocateMock.getActiveEditorBinding.mockReturnValue({ relPath: "notes/a.md", loadSeq: aLoadSeq, editFormat: "text" });
      edLocateMock.getActiveDocumentContent.mockReturnValue("A 实时未保存");
      mockTurnInvoke((arg) => {
        requestContext(arg, ["notes/b.md"]);
        completeTurn(arg);
      });

      const sending = startTurn("整理当前文档");
      await Promise.resolve();
      expect(invokeMock.mock.calls.some(([command]) => command === "ag_start_turn")).toBe(true);

      edStore.loadDoc("notes/b.md", "B 载入完成", 2);
      await vi.advanceTimersByTimeAsync(25);
      await sending;

      const arg = agStartArg();
      expect(arg.activeDocPath).toBe("notes/b.md");
      expect(arg.activeDocContent).toBeUndefined();
      expect(arg.openDocSnapshots).toEqual([]);
      expect(lastContextResponse().snapshots).toEqual([]);
      expect(lastContextResponse().facts.openResources).toContainEqual(expect.objectContaining({ relPath: "notes/b.md", availability: "loading", reasonCode: "AG_ACTIVE_SESSION_LOADING", snapshot: null }));
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

  // covers: BR-AG-PERSIST-001
  // covers: BR-SYS-STATE-007
  it("启动错误单独持久化，保持未完成且不进入下轮模型历史", async () => {
    edStore.loadDoc("notes/a.md", "A 文档", 1);
    const failure = { code: "AG_EVENT_DB_ERROR", message: "事件表不可用", retriable: true };
    invokeMock.mockImplementation(async (command, arg) => {
      if (command === "dispatch") return dispatchOk(arg);
      if (command === "ag_start_turn") throw failure;
      return undefined;
    });
    await startTurn("你好");
    await vi.waitFor(() => {
      const saves = invokeMock.mock.calls.filter(([command, arg]) =>
        command === "dispatch" && (arg as { request?: { command?: string } }).request?.command === "ag.save_turn");
      expect(saves.length).toBeGreaterThan(0);
    });
    const session = activeSession(agStore.getSnapshot());
    expect(session.messages.find((message) => message.role === "user")).toMatchObject({
      turnStatus: "draft", turnFailure: failure,
    });
    expect(session.messages.find((message) => message.role === "assistant")?.content).toBe("");
    expect(agStore.getSnapshot().activeTurn).toBeNull();
    const saves = invokeMock.mock.calls.filter(([command, arg]) =>
      command === "dispatch" && (arg as { request?: { command?: string } }).request?.command === "ag.save_turn");
    const save = saves[saves.length - 1];
    const payload = (save![1] as { request: { params: { payload: Record<string, unknown> } } }).request.params.payload;
    expect(payload).toMatchObject({ failure, incompleteReason: "stream_interrupted", assistantMessage: { content: "" } });
    expect(payload).toMatchObject({ failure: { diagnostic: { version: 1, observedAt: "frontend_invoke", category: "unknown", rootCauseStatus: "unknown" } } });
    mockTurnInvoke(completeTurn);
    await startTurn("重新问候");
    const starts = invokeMock.mock.calls.filter(([command]) => command === "ag_start_turn");
    expect(starts[starts.length - 1]?.[1]).toMatchObject({ history: [] });
  });

  // covers: BR-SYS-STATE-007, BR-AG-PERSIST-001
  it("失败诊断只消费安全字段，错误不污染下一次模型历史", async () => {
    edStore.loadDoc("notes/a.md", "A 文档", 1);
    const diagnostic = { version: 1, observedAt: "runtime", category: "core_context", rootCauseStatus: "unknown" };
    mockTurnInvoke((arg) => {
      const onEvent = (arg as { onEvent: { onmessage: ((message: TestTurnEvent) => void) | null } }).onEvent;
      onEvent.onmessage?.({ type: "error", code: "AG_PROMPT_BUDGET_EXCEEDED", message: "预算失败", retriable: true, diagnostic: { ...diagnostic, secret: "not-for-history" } });
      completeTurn(arg); // 迟到 completed 不覆盖首 error。
    });
    await startTurn("检查文档");
    await vi.waitFor(() => {
      const saves = invokeMock.mock.calls.filter(([command, arg]) => command === "dispatch" && (arg as { request?: { command?: string } }).request?.command === "ag.save_turn");
      expect(JSON.stringify(saves)).toContain('"diagnostic"');
      expect(JSON.stringify(saves)).not.toContain("not-for-history");
    });
    const session = activeSession(agStore.getSnapshot());
    expect(session.messages.find(m => m.error)?.content).toContain("根因未确认");
    expect(session.messages.find(m => m.role === "user")?.turnStatus).toBe("error");
    mockTurnInvoke(completeTurn);
    await startTurn("重新问候");
    const starts = invokeMock.mock.calls.filter(([command]) => command === "ag_start_turn");
    // error 回合的用户原文仍可作为历史；只有诊断/错误回复不得成为模型正文。
    expect(starts[starts.length - 1]?.[1]).toMatchObject({ history: [expect.objectContaining({ role: "user", content: "检查文档" })] });
    expect(JSON.stringify((starts[starts.length - 1]?.[1] as Record<string, unknown>).history)).not.toMatch(/diagnostic|根因未确认|预算失败|not-for-history/);
    for (const [code, expected] of [
      ["ENT_GATE_BAD_RESPONSE", "error"], ["UNKNOWN_GATE_ERROR", "error"],
      ["ENT_GATE_TIMEOUT", "error"], ["ENT_GATE_UNREACHABLE", "error"],
      ["ENT_GATE_HTTP_ERROR", "error"], ["ENT_AUTH_TIMEOUT", "error"],
      ["ENT_AUTH_UNREACHABLE", "error"], ["ENT_AUTH_HTTP_ERROR", "error"],
      ["ENT_AUTH_BAD_RESPONSE", "error"], ["ENT_CLIENT_SETUP", "error"],
      ["UNRELATED_401", "error"], ["AG_GW_TIMEOUT", "error"],
      ["AG_GATE_DENIED", "gate_rejected"], ["AG_GATE_OFFLINE", "gate_rejected"],
    ] as const) {
      mockTurnInvoke((arg) => {
        const onEvent = (arg as { onEvent: { onmessage: ((message: TestTurnEvent) => void) | null } }).onEvent;
        onEvent.onmessage?.({ type: "error", code, message: "测试失败", retriable: true });
        completeTurn(arg);
      });
      await startTurn(`测试 ${code}`);
      const current = activeSession(agStore.getSnapshot());
      const users = current.messages.filter(m => m.role === "user");
      const user = users[users.length - 1];
      expect(user.turnStatus).toBe(expected);
      const errors = current.messages.filter(m => m.error);
      expect(errors[errors.length - 1]?.turnStatus).toBe(expected);
      await vi.waitFor(() => {
        const saves = invokeMock.mock.calls.filter(([command, arg]) => command === "dispatch" && (arg as { request?: { command?: string } }).request?.command === "ag.save_turn");
        const saved = saves[saves.length - 1];
        expect(saved?.[1]).toMatchObject({ request: { params: {
          turnId: user.turnId, payload: { failure: { code, retriable: true } },
        } } });
        // Terminal status is persisted from the backend event, not overwritten by UI save.
        expect((saved?.[1] as { request: { params: Record<string, unknown> } }).request.params).not.toHaveProperty("status");
      });
    }
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

  it("编辑器无法序列化时仍可准入，但按需采集不回退旧正文", async () => {
    edStore.loadDoc("notes/current.md", "旧快照", 101);
    edLocateMock.getActiveDocumentContent.mockReturnValue(null);
    mockTurnInvoke((arg) => { requestContext(arg, ["notes/current.md"]); completeTurn(arg); });

    await startTurn("继续修改");

    expect(invokeMock.mock.calls.some(([command]) => command === "ag_start_turn")).toBe(true);
    expect(lastContextResponse().snapshots).toEqual([]);
    expect(lastContextResponse().facts.openResources).toContainEqual(expect.objectContaining({ relPath: "notes/current.md", availability: "unavailable", reasonCode: "AG_ACTIVE_SNAPSHOT_REQUIRED", snapshot: null }));
    const session = activeSession(agStore.getSnapshot());
    expect(session.messages.some((m) => m.error)).toBe(false);
    expect(session.messages.find((m) => m.role === "user")?.turnStatus).toBe("completed");
    expect(session.messages.find((m) => m.role === "assistant")?.turnStatus).toBe("completed");
    expect(agStore.getSnapshot().activeTurn).toBeNull();
  });

  it("diff_proposed 的 before/after 来自后端 projection，不取整篇 fallback", async () => {
    edStore.loadDoc("notes/current.md", "第一段\n第二段原文\n最后一段", 101);
    mockTurnInvoke((arg) => {
      const onEvent = (arg as { onEvent: { onmessage: ((message: TestTurnEvent) => void) | null } }).onEvent;
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
      draftPreviewState: "queued",
    });
  });

  it("前端不再根据数字型 block_id 自行解析目标块", async () => {
    edStore.loadDoc("notes/current.md", "第一段\n第二段原文", 101);
    mockTurnInvoke((arg) => {
      const onEvent = (arg as { onEvent: { onmessage: ((message: TestTurnEvent) => void) | null } }).onEvent;
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
      const onEvent = (arg as { onEvent: { onmessage: ((message: TestTurnEvent) => void) | null } }).onEvent;
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
      const onEvent = (arg as { onEvent: { onmessage: ((message: TestTurnEvent) => void) | null } }).onEvent;
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
      const onEvent = (arg as { onEvent: { onmessage: ((message: TestTurnEvent) => void) | null } }).onEvent;
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
      const onEvent = (arg as { onEvent: { onmessage: ((message: TestTurnEvent) => void) | null } }).onEvent;
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
      const onEvent = (arg as { onEvent: { onmessage: ((message: TestTurnEvent) => void) | null } }).onEvent;
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
      const onEvent = (arg as { onEvent: { onmessage: ((message: TestTurnEvent) => void) | null } }).onEvent;
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
      const onEvent = (arg as { onEvent: { onmessage: ((message: TestTurnEvent) => void) | null } }).onEvent;
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
      const onEvent = (arg as { onEvent: { onmessage: ((message: TestTurnEvent) => void) | null } }).onEvent;
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

  // covers: BR-DE-DATA-001, BR-DE-STATE-001
  it("同一 ProposalBatch 的 DiffProposed 逐卡投影并保留独立组序", async () => {
    edStore.loadDoc("notes/current.md", "旧全文", 101);
    projectionResult({
      diff_id: "diff-group-1",
      proposal_batch_id: "pb-1",
      group_index: 1,
      group_count: 2,
      block_id: "b1",
      block_index: 0,
      base_session_version: 1,
      base_content_hash: "base",
      block_ops: { payloadVersion: 1, base: { sessionVersion: 1, contentHash: "base" }, operations: [{
        op: "replace_node", blockId: "b1", fingerprint: "f1", expectedNodeType: "paragraph",
        beforeNode: { type: "paragraph", attrs: { blockId: "b1" } },
        afterNode: { type: "paragraph", attrs: { blockId: "b1" } },
      }] },
      patch_kind: "block_ops",
    });
    projectionResult({
      diff_id: "diff-group-2",
      proposal_batch_id: "pb-1",
      group_index: 2,
      group_count: 2,
      block_id: "b3",
      block_index: 2,
      base_session_version: 1,
      base_content_hash: "base",
      block_ops: { payloadVersion: 1, base: { sessionVersion: 1, contentHash: "base" }, operations: [{
        op: "replace_node", blockId: "b3", fingerprint: "f3", expectedNodeType: "paragraph",
        beforeNode: { type: "paragraph", attrs: { blockId: "b3" } },
        afterNode: { type: "paragraph", attrs: { blockId: "b3" } },
      }] },
      patch_kind: "block_ops",
    });
    mockTurnInvoke((arg) => {
      const onEvent = (arg as { onEvent: { onmessage: ((message: TestTurnEvent) => void) | null } }).onEvent;
      onEvent.onmessage?.({ type: "diff_proposed", turn_id: "t", proposalBatchId: "pb-1", groupIndex: 1, groupCount: 2, diff_id: "diff-group-1", rel_path: "notes/current.md" });
      onEvent.onmessage?.({ type: "diff_proposed", turn_id: "t", proposalBatchId: "pb-1", groupIndex: 2, groupCount: 2, diff_id: "diff-group-2", rel_path: "notes/current.md" });
      completeTurn(arg);
    });

    await startTurn("分别修改第一段和第三段");

    expect(deStore.getSnapshot().diffs.map((diff) => ({
      id: diff.diffId,
      batch: diff.proposalBatch,
    }))).toEqual([
      { id: "diff-group-1", batch: { id: "pb-1", groupIndex: 1, groupCount: 2 } },
      { id: "diff-group-2", batch: { id: "pb-1", groupIndex: 2, groupCount: 2 } },
    ]);
  });

  it("DiffProposed 与持久化 ProposalBatch 身份不一致时失败关闭", async () => {
    edStore.loadDoc("notes/current.md", "旧全文", 101);
    projectionResult({
      diff_id: "diff-batch-mismatch",
      proposal_batch_id: "persisted-batch",
      group_index: 1,
      group_count: 2,
    });
    mockTurnInvoke((arg) => {
      const onEvent = (arg as { onEvent: { onmessage: ((message: TestTurnEvent) => void) | null } }).onEvent;
      onEvent.onmessage?.({
        type: "diff_proposed",
        turn_id: "t",
        diff_id: "diff-batch-mismatch",
        proposalBatchId: "event-batch",
        groupIndex: 1,
        groupCount: 2,
        rel_path: "notes/current.md",
      });
      completeTurn(arg);
    });

    await startTurn("修改当前文档");

    expect(deStore.getSnapshot().diffs[0]).toMatchObject({
      diffId: "diff-batch-mismatch",
      state: "error",
      errorCode: "DE_PROPOSAL_BATCH_INVALID",
      proposalBatch: undefined,
    });
  });

});

// Migrated intact from agStore.test.ts.
describe("agStore migrated suite", () => {
  beforeEach(() => {
    agStore.reset();
  });

  describe("agStore TurnEvent 单一归约投影（TERM-AG-022）", () => {
    it("按 seq 归约、冻结终态，并允许澄清后的同 turn 恢复", () => {
      const sessionId = agStore.getSnapshot().activeSessionId;
      agStore.setActiveTurn(sessionId, "t");
      expect(agStore.acceptTurnEvent("t", 2, "chunk")).toBe(true);
      expect(agStore.acceptTurnEvent("t", 2, "tool_request")).toBe(false);
      expect(agStore.acceptTurnEvent("t", 1, "chunk")).toBe(false);
      expect(agStore.lastTurnEventSeq("t")).toBe(2);
      expect(agStore.getSnapshot().sending).toBe(true);

      expect(agStore.acceptTurnEvent("t", 5, "done")).toBe(true);
      expect(agStore.getSnapshot().sending).toBe(false);
      expect(agStore.getSnapshot().activeTurn).toBeNull();
      expect(agStore.acceptTurnEvent("t", 6, "tool_result")).toBe(false);
      agStore.setActiveTurn(sessionId, "t");
      expect(agStore.getSnapshot().activeTurn).toBeNull();

      // 澄清只释放输入门控，不锁存终态；同一 turn 可用更大 seq 恢复。
      agStore.setActiveTurn(sessionId, "t-clarify");
      expect(agStore.acceptTurnEvent("t-clarify", 3, "clarification_required")).toBe(true);
      expect(agStore.getSnapshot().sending).toBe(false);
      agStore.setActiveTurn(sessionId, "t-clarify");
      expect(agStore.acceptTurnEvent("t-clarify", 7, "chunk")).toBe(true);
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

    it("工具卡按会话隔离：addToolCall/resolveToolCall/clearToolCalls", async () => {
      const identity = { workspaceId: "ws", sessionId: "session", turnId: "turn", toolCallId: "call", toolName: "read_doc" };
      const receipt = { ...identity, version: "tool-call-receipt-v1", attempt: 1,
        capabilityId: "read", grantId: "grant", executionStatus: "succeeded", completionStatus: "completed",
        effectStatus: "not_applicable", references: [], failure: null, evidenceIds: ["proof"], nextAction: "none" };
      expect(validateToolCallReceipt(receipt, identity, true, ["proof"])).toEqual(receipt);
      const rejectedReceipt = {
        ...receipt, executionStatus: "rejected", completionStatus: "blocked",
        effectStatus: "not_started", evidenceIds: [], nextAction: "retry_explicitly",
        failure: { code: "AG_MODEL_CONTROL_CONFLICT", retriable: true, reauthRequired: false },
      };
      expect(validateToolCallReceipt(rejectedReceipt, identity, false, [])).toEqual(rejectedReceipt);
      for (const mutation of [{ version: "tool-call-receipt-v2" }, { turnId: "other" },
        { sessionId: "other" }, { workspaceId: "other" }, { toolCallId: "other" },
        { effectStatus: "saved" }, { executionStatus: "failed" }, { evidenceIds: [] },
        { completionStatus: "waiting" }, { failure: { code: "FAIL", retriable: true, reauthRequired: false } }]) {
        expect(() => validateToolCallReceipt({ ...receipt, ...mutation }, identity, true, ["proof"]))
          .toThrow("AG_TOOL_RECEIPT_INVALID");
      }
      expect(validateToolCallReceipt({ ...receipt, rawError: "SECRET" }, identity, true, ["proof"]))
        .not.toHaveProperty("rawError");
      const event = (seq: number, payload: Record<string, unknown>) => ({ eventId: `query-${seq}`, turnId: identity.turnId, payloadVersion: 1, seq, occurredAt: seq, ...payload });
      let evidence: Record<string, unknown>[] = [event(1, { type: "tool_request", toolCallId: "call", toolName: "read_doc", params: {} }),
        event(2, { type: "tool_result", toolCallId: "call", toolName: "read_doc", success: true, result: "完成", evidenceIds: ["proof"], executionReceipt: receipt }),
        event(3, { type: "done", stopReason: "stop" })];
      invokeMock.mockImplementation(async (_command, arg) => {
        const request = (arg as { request: { correlation_id: string } }).request;
        return { correlation_id: request.correlation_id, result: evidence };
      });
      expect(await queryTurnExecutionEvidence(identity)).toMatchObject({ receipts: [receipt], unresolvedToolCallIds: [] });
      evidence = [evidence[0], { ...evidence[1], executionReceipt: { ...receipt, turnId: "other" } }, evidence[2]];
      await expect(queryTurnExecutionEvidence(identity)).rejects.toThrow("AG_TOOL_RECEIPT_INVALID");
      evidence = [event(1, { type: "tool_request", toolCallId: "unknown", toolName: "propose_edit", params: {} }), event(2, { type: "error", code: "INTERRUPTED", message: "中断", retriable: true })];
      expect(await queryTurnExecutionEvidence(identity)).toMatchObject({ receipts: [], unresolvedToolCallIds: ["unknown"] });
      evidence.push(event(3, { type: "tool_request", toolCallId: "late", toolName: "read_doc" }));
      await expect(queryTurnExecutionEvidence(identity)).rejects.toThrow("AG_RETRY_EVIDENCE_UNAVAILABLE");
      const effectIdentity = { ...identity, workspaceId: "ws-test", toolName: "propose_edit", capabilityId: "document_edit", grantId: "effect-grant", attempt: 1 };
      const effectReceipt = { ...receipt, ...effectIdentity, completionStatus: "waiting", effectStatus: "proposal_created", references: [{ kind: "pending_diff", id: "real-diff" }], evidenceIds: [], nextAction: "await_user" };
      let effectFacts: unknown[] = [{ identity: effectIdentity, phase: "started", receipt: null }];
      invokeMock.mockImplementation(async (_command, arg) => {
        const request = (arg as { request: { command: string; correlation_id: string } }).request;
        expect(request.command).toBe("ag.list_turns");
        return { correlation_id: request.correlation_id, result: [{ turn_id: "turn", session_id: "session", workspace_id: "ws-test", execution_facts: effectFacts }] };
      });
      expect(await inspectTurnEffects("session", "turn")).toContain("禁止重放");
      effectFacts.push({ identity: effectIdentity, phase: "settled", receipt: effectReceipt });
      expect(await inspectTurnEffects("session", "turn")).toContain("pending_diff=real-diff");
      await expect(inspectTurnEffects("wrong-session", "turn")).rejects.toThrow("AG_EFFECT_FACT_UNAVAILABLE");
      effectFacts = [{ identity: effectIdentity, phase: "settled", receipt: effectReceipt }];
      await expect(inspectTurnEffects("session", "turn")).rejects.toThrow("AG_TOOL_RECEIPT_INVALID");
      effectFacts = [{ identity: effectIdentity, phase: "started", receipt: null }, { identity: effectIdentity, phase: "settled", receipt: { ...effectReceipt, grantId: "wrong" } }];
      await expect(inspectTurnEffects("session", "turn")).rejects.toThrow("AG_TOOL_RECEIPT_INVALID");
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

// covers: BR-AG-UI-001
// AG-MSG-RENDER-01 · TERM-AG-011：事件流按时序构造消息片段（chunk→tool→chunk→tool→diff 交错），
// 同名工具按后端 toolCallId 精确配对，不再依赖 name+executing 或前端 FIFO 猜测。
describe("startTurn 回合内消息片段时序（AG-MSG-RENDER-01）", () => {
  it("执行回执投影与工具身份绑定，畸形回执不冒充成功且不触发重试", async () => {
    for (const scenario of ["valid", "wrong_turn", "wrong_tool"]) {
      const invalid = scenario !== "valid";
      mockTurnInvoke((arg) => {
        const a = arg as { turnId: string; sessionId: string; onEvent: { onmessage: ((message: TestTurnEvent) => void) | null } };
        a.onEvent.onmessage?.({ type: "tool_request", toolCallId: "receipt-call", toolName: scenario === "wrong_tool" ? "workspace_glob" : "read_doc", params: {} });
        a.onEvent.onmessage?.({ type: "tool_result", toolCallId: "receipt-call", toolName: "read_doc", success: true, result: "成功", evidenceIds: ["proof"],
          executionReceipt: { version: "tool-call-receipt-v1", workspaceId: "ws-test", sessionId: a.sessionId,
            turnId: scenario === "wrong_turn" ? "other-turn" : a.turnId, toolCallId: "receipt-call", toolName: "read_doc",
            capabilityId: "read", grantId: "grant", attempt: 1, executionStatus: "succeeded",
            completionStatus: "completed", effectStatus: "not_applicable", references: [], failure: null,
            evidenceIds: ["proof"], nextAction: "none" } });
        a.onEvent.onmessage?.({ type: "done", stopReason: "stop" });
      });
      startTurn("读取文档");
      await new Promise((resolve) => setTimeout(resolve, 0));
      const tool = activeSession(agStore.getSnapshot()).toolCalls.find(t => t.id === "receipt-call")!;
      expect(tool.state).toBe(invalid ? "failed" : "done");
      expect(tool.detail).toContain(invalid ? "AG_TOOL_RECEIPT_INVALID" : "只读调用完成");
      expect(tool.executionReceipt?.effectStatus).toBe(invalid ? undefined : "not_applicable");
    }
    expect(invokeMock.mock.calls.filter(([command]) => command === "ag_start_turn")).toHaveLength(3);
  });
  it("chunk/tool_request/tool_result/diff_proposed 交错构造 segments", async () => {
    edStore.loadDoc("notes/current.md", "第一段\n第二段", 101);
    mockTurnInvoke((arg) => {
      const onEvent = (arg as { onEvent: { onmessage: ((message: TestTurnEvent) => void) | null } }).onEvent;
      onEvent.onmessage?.({ type: "chunk", turn_id: "t", text: "先读取文档" });
      onEvent.onmessage?.({
        type: "tool_request",
        turn_id: "t",
        tool_name: "read_doc",
        params: { path: "notes/a.md" },
      });
      onEvent.onmessage?.({ type: "tool_result", turn_id: "t", tool_name: "read_doc", result: "内容", success: true });
      onEvent.onmessage?.({ type: "chunk", turn_id: "t", text: "再读取另一篇" });
      onEvent.onmessage?.({
        type: "tool_request",
        turn_id: "t",
        tool_name: "read_doc",
        params: { path: "notes/b.md" },
      });
      onEvent.onmessage?.({ type: "tool_result", turn_id: "t", tool_name: "read_doc", result: "内容2", success: false });
      onEvent.onmessage?.({
        type: "tool_request",
        turn_id: "t",
        tool_name: "propose_edit",
        params: { path: "notes/current.md", operation: "replace_block", block_index: 1, new_text: "改写" },
      });
      onEvent.onmessage?.({
        type: "tool_result",
        turn_id: "t",
        tool_name: "propose_edit",
        result: projectionResult({ diff_id: "diff-seq" }),
        success: true,
      });
      onEvent.onmessage?.({ type: "diff_proposed", turn_id: "t", diff_id: "diff-seq", rel_path: "notes/current.md" });
      completeTurn(arg);
    });

    await startTurn("读取两个文档后改写");

    const session = activeSession(agStore.getSnapshot());
    const assistant = session.messages.find((message) => message.role === "assistant");
    expect(assistant?.content).toBe("先读取文档再读取另一篇");
    expect(assistant?.segments).toEqual([
      { kind: "text", text: "先读取文档" },
      { kind: "tool", toolCallId: "call-2" },
      { kind: "text", text: "再读取另一篇" },
      { kind: "tool", toolCallId: "call-5" },
      { kind: "tool", toolCallId: "call-7" },
      { kind: "diff", diffId: "diff-seq" },
    ]);
    // 同名 read_doc 两次按 toolCallId 精确配对：第一次成功、第二次失败（BR-AG-UI-001）。
    const readTools = session.toolCalls.filter((t) => t.name === "read_doc");
    expect(readTools.map((t) => t.state)).toEqual(["done", "failed"]);
  });

  it("无工具/无 diff 的纯文本回合产生单一 text 段并只读投影 ProjectReportV2", async () => {
    edStore.loadDoc("notes/current.md", "第一段", 101);
    mockTurnInvoke((arg) => {
      const runtimeTurnId = String((arg as Record<string, unknown>).turnId);
      const onEvent = (arg as { onEvent: { onmessage: ((message: TestTurnEvent) => void) | null } }).onEvent;
      onEvent.onmessage?.({ type: "chunk", turn_id: "t", text: "好的" });
      onEvent.onmessage?.({
        type: "project_report",
        report: {
          version: "project-report-v2",
          turnId: runtimeTurnId,
          evidenceThroughSeq: 3,
          requestedScope: ["notes/current.md"],
          readItems: [{ relPath: "notes/current.md", source: "active_editor", coverage: "full_document", evidenceIds: ["evidence:1"] }],
          unreadItems: [],
          findings: [],
          uncertainties: [],
        },
      });
      completeTurn(arg);
    });

    await startTurn("只回复");

    const session = activeSession(agStore.getSnapshot());
    const assistant = session.messages.find((message) => message.role === "assistant");
    expect(assistant?.segments).toEqual([{ kind: "text", text: "好的" }]);
    expect(Object.values(agStore.getSnapshot().projectReports)[0]).toMatchObject({
      version: "project-report-v2",
      evidenceThroughSeq: 3,
      requestedScope: ["notes/current.md"],
    });
    expect(session.toolCalls).toHaveLength(0);
  });

  it("字段损坏的 ProjectReport 安全事件不进入只读报告投影", async () => {
    edStore.loadDoc("notes/current.md", "第一段", 101);
    mockTurnInvoke((arg) => {
      const runtimeTurnId = String((arg as Record<string, unknown>).turnId);
      const onEvent = (arg as { onEvent: { onmessage: ((message: TestTurnEvent) => void) | null } }).onEvent;
      onEvent.onmessage?.({
        type: "project_report",
        report: {
          version: "project-report-v2",
          turnId: runtimeTurnId,
          evidenceThroughSeq: 1,
          requestedScope: ["notes/current.md"],
          readItems: "corrupt",
          unreadItems: [],
          findings: [],
          uncertainties: [],
        },
      });
      completeTurn(arg);
    });

    await startTurn("只回复");

    expect(Object.values(agStore.getSnapshot().projectReports)).toHaveLength(0);
  });

  it("Channel 丢失终态时按 lastEventSeq 补流并经同一 reducer 完成", async () => {
    let backendTurnId = "";
    invokeMock.mockImplementation(async (command, arg) => {
      if (command === "ag_start_turn") {
        backendTurnId = String((arg as Record<string, unknown>).turnId);
        return undefined;
      }
      const request = (arg as { request?: { command?: string; correlation_id?: string; params?: Record<string, unknown> } }).request;
      if (request?.command === "ag.list_turn_events") {
        expect(request.params).toMatchObject({ turnId: backendTurnId, afterSeq: 0 });
        return {
          correlation_id: request.correlation_id,
          result: [{
            eventId: `turn-event:${backendTurnId}:4`,
            turnId: backendTurnId,
            seq: 4,
            occurredAt: 4,
            payloadVersion: 1,
            type: "done",
            stopReason: "stop",
          }],
        };
      }
      return dispatchOk(arg);
    });

    await startTurn("只回复");

    const assistant = activeSession(agStore.getSnapshot()).messages.find((message) => message.role === "assistant");
    expect(assistant?.turnStatus).toBe("completed");
    expect(agStore.lastTurnEventSeq(backendTurnId)).toBe(4);
    expect(agStore.getSnapshot().sending).toBe(false);
  });

  it("Channel 无终态保持 draft；重试核对终态和效果，不盲目重复执行", async () => {
    invokeMock.mockImplementation(async (command, arg) => {
      if (command === "ag_start_turn") return undefined;
      const request = (arg as { request?: { command?: string; correlation_id?: string } }).request;
      if (request?.command === "ag.list_turn_events") {
        return { correlation_id: request.correlation_id, result: [] };
      }
      return dispatchOk(arg);
    });

    await startTurn("只回复");

    const messages = activeSession(agStore.getSnapshot()).messages;
    expect(messages.filter((message) => message.turnId).map((message) => message.turnStatus))
      .toEqual(["draft", "draft"]);
    expect(messages.some((message) => message.error)).toBe(false);
    expect(agStore.getSnapshot().activeTurn).toBeNull();
    expect(agStore.getSnapshot().sending).toBe(false);
    const source = messages.find((message) => message.role === "user")!;
    const sessionId = agStore.getSnapshot().activeSessionId;
    const retryOf = { sessionId, turnId: source.turnId! };
    const initialCalls = invokeMock.mock.calls.filter(([command]) => command === "ag_start_turn").length;
    await expect(startTurn("只回复", { retryOf })).rejects.toThrow("AG_RETRY_EFFECT_UNVERIFIED");
    const event = (seq: number, payload: Record<string, unknown>) => ({
      eventId: `retry-${seq}`, turnId: source.turnId, seq, occurredAt: seq, payloadVersion: 1, ...payload,
    });
    const failure = event(3, { type: "error", code: "AG_GW_STREAM_INCOMPLETE", message: "中断", retriable: true });
    let evidence: unknown[] = [];
    let failLookup = false;
    let switchDuringLookup = false;
    invokeMock.mockImplementation(async (command, arg) => {
      if (command === "ag_start_turn") {
        bindTurnChannel(arg as Record<string, unknown>);
        const channel = (arg as { onEvent: { onmessage: (event: TestTurnEvent) => void } }).onEvent;
        channel.onmessage({ type: "done", stopReason: "stop" });
        return undefined;
      }
      const request = (arg as { request?: { command?: string; correlation_id?: string } }).request;
      if (request?.command === "ag.list_turn_events") {
        if (failLookup) throw new Error("数据库不可用");
        if (switchDuringLookup) agStore.newSession();
        return { correlation_id: request.correlation_id, result: evidence };
      }
      return dispatchOk(arg);
    });
    for (const rejected of [
      [event(1, { type: "tool_request", toolName: "propose_edit", toolCallId: "write", params: {} }), failure],
      [event(1, { type: "tool_request", toolName: "unknown_tool", toolCallId: "unknown", params: {} }), failure],
      [event(1, { type: "tool_result", toolName: "propose_edit", success: true }), failure],
      [event(1, { type: "diff_proposed", diffId: "d" }), failure],
      [event(1, { type: "decision_request_proposed", requestId: "r" }), failure],
      [event(1, { type: "done", stopReason: "stop" })],
      [event(1, { type: "error", retriable: false })],
      [{ ...failure, turnId: "another-turn" }],
      [failure, event(2, { type: "chunk", text: "迟到" })],
    ]) {
      evidence = rejected;
      await expect(startTurn("只回复", { retryOf })).rejects.toThrow("AG_RETRY_EFFECT_UNVERIFIED");
    }
    failLookup = true;
    await expect(startTurn("只回复", { retryOf })).rejects.toThrow("AG_RETRY_EVIDENCE_UNAVAILABLE");
    failLookup = false;
    evidence = [failure];
    switchDuringLookup = true;
    await expect(startTurn("只回复", { retryOf })).rejects.toThrow("AG_RETRY_CONTEXT_CHANGED");
    switchDuringLookup = false;
    agStore.switchSession(sessionId);
    expect(invokeMock.mock.calls.filter(([command]) => command === "ag_start_turn")).toHaveLength(initialCalls);
    evidence = [event(1, { type: "tool_request", toolName: "workspace_glob", toolCallId: "read", params: {} }), failure];
    const retry = startTurn("只回复", { retryOf });
    await expect(startTurn("只回复", { retryOf })).rejects.toThrow("AG_RETRY_ALREADY_PENDING");
    await retry;
    const calls = invokeMock.mock.calls.filter(([command]) => command === "ag_start_turn");
    expect(calls).toHaveLength(initialCalls + 1);
    expect((calls[calls.length - 1][1] as { turnId: string }).turnId).not.toBe(source.turnId);
    expect(activeSession(agStore.getSnapshot()).messages.find((message) => message.id === source.id)?.turnStatus).toBe("draft");
  });
});

describe("澄清非终态（R5-A · BR-SYS-STATE-007）", () => {
  it("clarification_required 投影为 clarifying、释放 command 且不伪造终态", async () => {
    edStore.loadDoc("notes/current.md", "第一段", 101);
    mockTurnInvoke((arg) => {
      const onEvent = (arg as { onEvent: { onmessage: ((m: TestTurnEvent) => void) | null } }).onEvent;
      onEvent.onmessage?.({
        type: "clarification_required",
        turn_id: "t",
        text: "需要你先确认：当前请求缺少安全执行所需的信息。",
      });
    });
    await startTurn("修改文档");

    const snapshot = agStore.getSnapshot();
    const assistant = activeSession(snapshot).messages.find((m) => m.role === "assistant");
    expect(assistant?.turnStatus).toBe("clarifying");
    expect(assistant?.content).toContain("需要你先确认");
    expect(snapshot.activeTurn).toBeNull();
    expect(snapshot.sending).toBe(false);
  });

  it("用户补充说明以同 turn_id 受控重入，不取消或创建第二个 AgentTurn", async () => {
    edStore.loadDoc("notes/current.md", "第一段", 101);
    invokeMock.mockClear();
    const turnInvocations: Record<string, unknown>[] = [];
    invokeMock.mockImplementation(async (command, arg) => {
      if (command === "dispatch") return dispatchOk(arg);
      if (command !== "ag_start_turn") return undefined;
      const invocation = arg as Record<string, unknown>;
      bindTurnChannel(invocation);
      turnInvocations.push(invocation);
      const onEvent = invocation.onEvent as { onmessage: ((m: TestTurnEvent) => void) | null };
      if (turnInvocations.length === 1) {
        onEvent.onmessage?.({ type: "clarification_required", turn_id: "ignored", text: "澄清问题" });
      } else {
        onEvent.onmessage?.({ type: "chunk", turn_id: "ignored", text: "已按补充说明完成" });
        onEvent.onmessage?.({ type: "done", turn_id: "ignored", stop_reason: "stop" });
      }
      return undefined;
    });
    await startTurn("修改文档", {
      mentions: ["notes/legacy.md"],
      blockRefs: [{ relPath: "notes/legacy.md", blockId: "b1", text: "旧引用" }],
    });
    await startTurn("目标是 notes/current.md");

    expect(turnInvocations).toHaveLength(2);
    expect(turnInvocations[1]?.turnId).toBe(turnInvocations[0]?.turnId);
    expect(turnInvocations[0]?.clarificationResume).toBe(false);
    expect(turnInvocations[1]?.clarificationResume).toBe(true);
    expect(turnInvocations[1]?.prompt).toBe("目标是 notes/current.md");
    expect(turnInvocations[1]?.mentions).toEqual(["notes/legacy.md"]);
    expect(turnInvocations[1]?.blockRefs).toEqual([
      { relPath: "notes/legacy.md", blockId: "b1", text: "旧引用" },
    ]);
    expect(invokeMock.mock.calls.some(([command]) => command === "ag_cancel_turn")).toBe(false);

    const messages = activeSession(agStore.getSnapshot()).messages;
    expect(messages.filter((message) => message.turnId === turnInvocations[0]?.turnId)).toHaveLength(2);
    expect(messages.find((message) => message.role === "user")?.content).toContain("补充说明：目标是 notes/current.md");
    expect(messages.find((message) => message.role === "assistant")?.turnStatus).toBe("completed");
  });

  it("澄清补充不因无关编辑器无法序列化阻断，保留原 turn_id 和递增上下文版本", async () => {
    edStore.loadDoc("notes/current.md", "第一段", 101);
    const turnInvocations: Record<string, unknown>[] = [];
    invokeMock.mockImplementation(async (command, arg) => {
      if (command === "dispatch") return dispatchOk(arg);
      if (command !== "ag_start_turn") return undefined;
      const invocation = arg as Record<string, unknown>;
      bindTurnChannel(invocation);
      turnInvocations.push(invocation);
      const onEvent = invocation.onEvent as { onmessage: ((m: TestTurnEvent) => void) | null };
      if (turnInvocations.length === 1) {
        onEvent.onmessage?.({ type: "clarification_required", turn_id: "ignored", text: "澄清问题" });
      } else {
        onEvent.onmessage?.({ type: "done", turn_id: "ignored", stop_reason: "stop" });
      }
      return undefined;
    });

    await startTurn("修改文档");
    const originalTurnId = turnInvocations[0]?.turnId;
    edLocateMock.getActiveDocumentContent.mockReturnValue(null);
    await startTurn("目标是当前文档");

    expect(turnInvocations).toHaveLength(2);
    expect(turnInvocations[1]).toMatchObject({
      turnId: originalTurnId,
      clarificationResume: true,
      prompt: "目标是当前文档",
      agentContextFacts: expect.objectContaining({ version: "agent-context-facts-v2", contextRevision: 2 }),
    });
    expect(invokeMock.mock.calls.some(([command]) => command === "ag_cancel_turn")).toBe(false);
  });

  it("等待澄清时取消使用原 turn_id 并投影 Cancelled", async () => {
    edStore.loadDoc("notes/current.md", "第一段", 101);
    mockTurnInvoke((arg) => {
      const onEvent = (arg as { onEvent: { onmessage: ((m: TestTurnEvent) => void) | null } }).onEvent;
      onEvent.onmessage?.({ type: "clarification_required", turn_id: "ignored", text: "澄清问题" });
    });
    await startTurn("修改文档");
    const turnId = activeSession(agStore.getSnapshot()).messages.find((message) => message.role === "user")?.turnId;
    invokeMock.mockImplementation(async (command, arg) => {
      if (command === "ag_cancel_turn") return undefined;
      const request = (arg as { request?: { command?: string; correlation_id?: string } }).request;
      if (request?.command === "ag.list_turn_events") {
        return {
          correlation_id: request.correlation_id,
          result: [{
            eventId: `turn-event:${turnId}:2`,
            turnId,
            seq: 2,
            occurredAt: 2,
            payloadVersion: 1,
            type: "done",
            stopReason: "cancelled",
          }],
        };
      }
      return dispatchOk(arg);
    });

    await cancelActiveTurn();

    const cancelCall = invokeMock.mock.calls.find(([command]) => command === "ag_cancel_turn");
    expect(cancelCall?.[1]).toMatchObject({ turnId });
    const assistant = activeSession(agStore.getSnapshot()).messages.find((message) => message.role === "assistant");
    expect(assistant?.turnStatus).toBe("cancelled");
    expect(assistant?.streaming).toBe(false);
    expect(agStore.lastTurnEventSeq(turnId ?? "")).toBe(2);
  });
});

describe("agService TaskRun 查询（AG-1.0-IMP-TASKRUN-UI-01）", () => {
  // covers: BR-SYS-STATE-008
  // listSessionTaskRuns 走 ag.list_task_runs 信封并映射为 TaskRunView（snake→camel），投影到 agStore。
  it("fetches session task runs and hydrates store", async () => {
    const sessionId = agStore.getSnapshot().activeSessionId;
    invokeMock.mockImplementation(async (_cmd: string, arg: unknown) => {
      const req = (arg as { request: { command: string; correlation_id: string } }).request;
      if (req.command === "ag.list_task_runs") {
        return {
          correlation_id: req.correlation_id,
          result: [
            {
              task_run_id: "task-ws1-1-0",
              source_turn_ids: ["turn-1"],
              through_seq: 8,
              goal: "修改会议纪要",
              status: "waiting",
              observed_steps: [
                { step_id: "t1", kind: "tool", label: "read_doc", status: "completed", depends_on: [], source_event_id: "e1", source_event_seq: 5 },
                { step_id: "p1", kind: "propose", label: "提案 notes/a.md", status: "waiting", depends_on: [], source_event_id: "e2", source_event_seq: 8 },
              ],
              waiting_on: ["decision-1"],
              current_route_revision: 2,
              candidate_steps: [],
              created_at: 1,
              updated_at: 2,
            },
          ],
        };
      }
      return { correlation_id: req.correlation_id, result: { sessions: [], active_session_id: null } };
    });

    await refreshSessionTaskRuns(sessionId);

    const runs = agStore.getSnapshot().sessions.find((s) => s.id === sessionId)?.taskRuns ?? [];
    expect(runs).toHaveLength(1);
    expect(runs[0]).toMatchObject({
      taskRunId: "task-ws1-1-0",
      status: "waiting",
      goal: "修改会议纪要",
    });
    expect(runs[0].observedSteps[0]).toEqual({
      stepId: "t1",
      kind: "tool",
      label: "read_doc",
      status: "completed",
      dependsOn: [],
      sourceEventId: "e1",
      sourceEventSeq: 5,
      result: undefined,
    });
    expect(runs[0].observedSteps[1].status).toBe("waiting");
  });

  // covers: BR-SYS-STATE-008
  // 查询失败不阻塞（best-effort）：保持上次投影状态。
  it("keeps previous projection when query fails", async () => {
    const sessionId = agStore.getSnapshot().activeSessionId;
    agStore.hydrateSessionTaskRuns(sessionId, [
      {
        taskRunId: "old-1",
        sourceTurnIds: ["old-turn"],
        throughSeq: 1,
        goal: "old",
        status: "active",
        observedSteps: [],
        waitingOn: [],
        candidateSteps: [],
        createdAt: 1,
        updatedAt: 1,
      },
    ]);
    invokeMock.mockRejectedValue(new Error("network"));
    await refreshSessionTaskRuns(sessionId);
    const runs = agStore.getSnapshot().sessions.find((s) => s.id === sessionId)?.taskRuns ?? [];
    expect(runs[0].taskRunId).toBe("old-1");
  });
});

describe("agService 回合实时事件（AG-1.0-TASKRUN-REWORK-IMP-UI-01）", () => {
  // covers: BR-SYS-STATE-008
  // decision_request_proposed 停放原回合并释放发送门控，不接受同一次 invocation 伪造的 done。
  it("parks a pending decision as a nonterminal wait", async () => {
    let invocation = 0;
    mockTurnInvoke((arg) => {
      invocation += 1;
      const onEvent = (arg as { onEvent: { onmessage: ((m: TestTurnEvent) => void) | null } }).onEvent;
      if (invocation === 1) {
        onEvent.onmessage?.({ type: "step_progress", turn_id: "t", action: "tool", label: "read_doc" });
        onEvent.onmessage?.({
          type: "decision_request_proposed",
          turn_id: "t",
          request_id: "decision-1",
          kind: "permission",
          payload: { summary: "需要读取敏感文档" },
        });
        completeTurn(arg);
      } else {
        onEvent.onmessage?.({
          type: "decision_request_resolved",
          turn_id: "t",
          request_id: "decision-1",
          outcome: "cancel",
        });
        completeTurn(arg);
      }
    });

    await startTurn("读取文档并修改");

    expect(agStore.getSnapshot().sending).toBe(false);
    expect(agStore.getSnapshot().currentAction).toMatchObject({ action: "deciding" });
    expect(agStore.getSnapshot().pendingDecision).toMatchObject({
      requestId: "decision-1",
      kind: "permission",
    });
    await resolveDecisionAndContinue("decision-1", "cancel");
    expect(agStore.getSnapshot().pendingDecision).toBeNull();
  });

  // covers: BR-SYS-STATE-008
  // 裁决恢复必须复用同一 turn/assistant，不插入替代用户消息；resolved 后再正常终结。
  it("resumes the original turn identity and clears the decision on resolution", async () => {
    const invocations: Array<Record<string, unknown>> = [];
    mockTurnInvoke((arg) => {
      invocations.push(arg as Record<string, unknown>);
      const onEvent = (arg as { onEvent: { onmessage: ((m: TestTurnEvent) => void) | null } }).onEvent;
      if (invocations.length === 1) {
        onEvent.onmessage?.({
          type: "decision_request_proposed",
          turn_id: "t",
          request_id: "decision-2",
          kind: "plan_choice",
          payload: { summary: "确认方案 A" },
        });
      } else {
        onEvent.onmessage?.({
          type: "decision_request_resolved",
          turn_id: "t",
          request_id: "decision-2",
          outcome: "confirm",
        });
        onEvent.onmessage?.({ type: "chunk", turn_id: "t", text: "已继续执行" });
        completeTurn(arg);
      }
    });

    await startTurn("按方案执行");
    const sessionId = agStore.getSnapshot().activeSessionId;
    const beforeResume = agStore.getSnapshot().sessions.find((session) => session.id === sessionId)?.messages ?? [];
    const originalTurnId = String(invocations[0]?.turnId);
    await resolveDecisionAndContinue("decision-2", "confirm");

    expect(invocations).toHaveLength(2);
    expect(invocations[1]).toMatchObject({
      turnId: originalTurnId,
      decisionResume: true,
      clarificationResume: false,
      continuationDecisionRequestId: "decision-2",
    });
    const afterResume = agStore.getSnapshot().sessions.find((session) => session.id === sessionId)?.messages ?? [];
    expect(afterResume).toHaveLength(beforeResume.length);
    expect(afterResume.filter((message) => message.turnId === originalTurnId && message.role === "user")).toHaveLength(1);
    expect(afterResume.find((message) => message.turnId === originalTurnId && message.role === "assistant")?.content)
      .toContain("已继续执行");
    expect(agStore.getSnapshot().pendingDecision).toBeNull();
    expect(agStore.getSnapshot().currentAction).toBeNull();
  });

  it("cancels a parked decision by its original turn identity", async () => {
    mockTurnInvoke((arg) => {
      const onEvent = (arg as { onEvent: { onmessage: ((m: TestTurnEvent) => void) | null } }).onEvent;
      onEvent.onmessage?.({
        type: "decision_request_proposed",
        turn_id: "t",
        request_id: "decision-cancel",
        kind: "manual_confirm",
        payload: { summary: "等待确认" },
      });
    });
    await startTurn("执行待确认任务");
    const sessionId = agStore.getSnapshot().activeSessionId;
    const turnId = activeSession(agStore.getSnapshot()).messages.find((message) => message.role === "user")?.turnId;
    await expect(deleteAgentSession(sessionId)).rejects.toMatchObject({ code: "AG_TURN_ACTIVE" });

    invokeMock.mockImplementation(async (command, arg) => {
      if (command === "ag_cancel_turn") return undefined;
      const request = (arg as { request?: { command?: string; correlation_id?: string } }).request;
      if (request?.command === "ag.list_turn_events") {
        return {
          correlation_id: request.correlation_id,
          result: [{
            eventId: `turn-event:${turnId}:3`,
            turnId,
            seq: 3,
            occurredAt: 3,
            payloadVersion: 1,
            type: "done",
            stopReason: "cancelled",
          }],
        };
      }
      return dispatchOk(arg);
    });

    await cancelActiveTurn();

    const cancelCall = invokeMock.mock.calls.find(([command]) => command === "ag_cancel_turn");
    expect(cancelCall?.[1]).toMatchObject({ sessionId, turnId });
    expect(agStore.getSnapshot().pendingDecision).toBeNull();
    const assistant = activeSession(agStore.getSnapshot()).messages.find((message) => message.role === "assistant");
    expect(assistant?.turnStatus).toBe("cancelled");
    expect(assistant?.streaming).toBe(false);
  });
});

describe("auto-continue（AG-1.0-TASKRUN-REWORK-CONTINUE-01）", () => {
  // covers: BR-SYS-STATE-007
  it("marks continuation failure after committed resolution instead of reporting an unsubmitted decision", async () => {
    mockTurnInvoke(arg=>(arg as {onEvent:{onmessage:((event:TestTurnEvent)=>void)|null}}).onEvent.onmessage?.({type:"decision_request_proposed",turn_id:"t",request_id:"p06-close-fail",kind:"manual_confirm",payload:{}}));
    await startTurn("待确认任务");
    const original=invokeMock.getMockImplementation()!;
    invokeMock.mockImplementation((command,arg)=>command==="ag_cancel_turn" ? Promise.reject({code:"CANCEL_FAILED",message:"停止后续执行失败"}) : original(command,arg));
    closeStore.setChecking();
    try {
      await expect(resolveDecisionAndContinue("p06-close-fail","confirm")).rejects.toMatchObject({code:"CANCEL_FAILED",decisionResolved:true,taskRunId:"task-confirmed"});
      expect(agStore.getSnapshot().pendingDecision).toBeNull();
      expect(invokeMock.mock.calls.filter(([command])=>command==="ag_start_turn")).toHaveLength(1);
    } finally {
      closeStore.reset();
    }
  });
  // covers: BR-SYS-STATE-007
  it("does not clear another request or action on a late resolved and terminal event", async () => {
    const sessionId=agStore.getSnapshot().activeSessionId;
    const foreign={sessionId,turnId:"new-turn",requestId:"new-request",kind:"manual_confirm"};
    const action={sessionId,turnId:"new-turn",action:"deciding" as const,label:"等待新决策"};
    mockTurnInvoke(arg=>{
      agStore.setPendingDecision(foreign);
      agStore.setCurrentAction(action);
      (arg as {onEvent:{onmessage:((event:TestTurnEvent)=>void)|null}}).onEvent.onmessage?.({type:"decision_request_resolved",turn_id:"t",request_id:"old-request",outcome:"confirm"});
      completeTurn(arg);
    });
    await startTurn("旧回合");
    expect(agStore.getSnapshot().pendingDecision).toEqual(foreign);
    expect(agStore.getSnapshot().currentAction).toBe(action);
  });
  // covers: BR-SYS-STATE-007
  // covers: BR-SYS-STATE-008
  it("serializes a decision submission and preserves A to B across a session switch", async () => {
    const turns: Array<Record<string, unknown>> = [];
    mockTurnInvoke(arg => {
      turns.push(arg as Record<string, unknown>);
      const channel=(arg as {onEvent:{onmessage:((event:TestTurnEvent)=>void)|null}}).onEvent;
      if (turns.length<=2) channel.onmessage?.({type:"decision_request_proposed",turn_id:"t",request_id:turns.length===1?"p06-a":"p06-b",kind:"manual_confirm",payload:{summary:"confirm"}});
      else completeTurn(arg);
    });
    await startTurn("连续决策");
    const origin=agStore.getSnapshot().activeSessionId;
    const original=invokeMock.getMockImplementation()!;
    let finish!:()=>void;
    invokeMock.mockImplementation((command,arg) => {
      if (command==="dispatch" && (arg as {request:{command:string}}).request.command==="ag.resolve_decision") {
        return new Promise(resolve=>{finish=()=>resolve(dispatchOk(arg));});
      }
      return original(command,arg);
    });
    const first=resolveDecisionAndContinue("p06-a","confirm");
    await expect(resolveDecisionAndContinue("p06-a","cancel")).rejects.toMatchObject({code:"AG_DECISION_IN_FLIGHT"});
    const other=agStore.newSession();
    finish();
    await first;
    expect(turns).toHaveLength(2);
    expect(turns[1]).toMatchObject({sessionId:origin,turnId:turns[0].turnId,decisionResume:true});
    expect(agStore.getSnapshot().pendingDecision).toMatchObject({requestId:"p06-b",sessionId:origin});
    expect(agStore.getSnapshot().currentAction).toMatchObject({sessionId:origin,action:"deciding"});
    expect(agStore.getSnapshot().sessions.find(s=>s.id===other)?.messages).toHaveLength(0);
    invokeMock.mockImplementation(original);
    await resolveDecisionAndContinue("p06-b","confirm");
    expect(turns).toHaveLength(3);
    expect(turns[2].turnId).toBe(turns[0].turnId);
    expect(agStore.getSnapshot().pendingDecision).toBeNull();
  });

  // covers: BR-SYS-STATE-007
  it("releases the submission lock after rejection without discarding the pending request", async () => {
    let count=0;
    mockTurnInvoke(arg=> {
      if (++count===1) (arg as {onEvent:{onmessage:((event:TestTurnEvent)=>void)|null}}).onEvent.onmessage?.({type:"decision_request_proposed",turn_id:"t",request_id:"p06-retry",kind:"manual_confirm",payload:{}});
      else completeTurn(arg);
    });
    await startTurn("重试决策");
    const original=invokeMock.getMockImplementation()!;
    invokeMock.mockImplementation((command,arg)=>{
      if (command==="dispatch" && (arg as {request:{command:string}}).request.command==="ag.resolve_decision") return Promise.reject({code:"RPC_FAILED",message:"未提交"});
      return original(command,arg);
    });
    await expect(resolveDecisionAndContinue("p06-retry","confirm")).rejects.toMatchObject({code:"RPC_FAILED"});
    expect(agStore.getSnapshot().pendingDecision?.requestId).toBe("p06-retry");
    invokeMock.mockImplementation(original);
    await resolveDecisionAndContinue("p06-retry","confirm");
    expect(count).toBe(2);
    expect(agStore.getSnapshot().pendingDecision).toBeNull();
  });

  // covers: BR-SYS-STATE-007
  it("does not resume into another workspace when a decision response arrives late", async () => {
    mockTurnInvoke(arg=>(arg as {onEvent:{onmessage:((event:TestTurnEvent)=>void)|null}}).onEvent.onmessage?.({type:"decision_request_proposed",turn_id:"t",request_id:"p06-workspace",kind:"manual_confirm",payload:{}}));
    await startTurn("原工作区任务");
    const original=invokeMock.getMockImplementation()!;
    let finish!:()=>void;
    invokeMock.mockImplementation((command,arg)=>{
      if(command==="dispatch" && (arg as {request:{command:string}}).request.command==="ag.resolve_decision") return new Promise(resolve=>{finish=()=>resolve(dispatchOk(arg));});
      return original(command,arg);
    });
    const promise=resolveDecisionAndContinue("p06-workspace","confirm");
    workspaceShell._projectActive({workspaceId:"ws-other",rootPath:"/tmp/ws-other"});
    agStore.reset();
    const other={sessionId:agStore.getSnapshot().activeSessionId,turnId:"other",requestId:"new-workspace-decision",kind:"manual_confirm"};
    agStore.setPendingDecision(other);
    finish();
    await expect(promise).rejects.toMatchObject({code:"AG_DECISION_CONTEXT_CHANGED",decisionResolved:true});
    expect(invokeMock.mock.calls.filter(([command])=>command==="ag_start_turn")).toHaveLength(1);
    expect(agStore.getSnapshot().pendingDecision).toEqual(other);
    await expect(resolveDecisionAndContinue("p06-workspace","confirm")).rejects.toMatchObject({code:"AG_DECISION_CONTEXT_CHANGED"});
  });

  // covers: BR-SYS-STATE-007
  it("bounds decision submission and does not replay or resume after a late response", async () => {
    vi.useFakeTimers();
    try {
      mockTurnInvoke(arg => {
        (arg as {onEvent:{onmessage:((event:TestTurnEvent)=>void)|null}}).onEvent.onmessage?.({
          type:"decision_request_proposed",turn_id:"t",request_id:"decision-timeout-owned",kind:"manual_confirm",payload:{}
        });
      });
      await startTurn("确认超时隔离");
      const original = invokeMock.getMockImplementation()!;
      let submitted = 0;
      let finish!: () => void;
      invokeMock.mockImplementation((command,arg) => {
        if (command === "dispatch" && (arg as {request:{command:string}}).request.command === "ag.resolve_decision") {
          submitted += 1;
          return new Promise(resolve => {finish = () => resolve(dispatchOk(arg));});
        }
        return original(command,arg);
      });
      const pending = resolveDecisionAndContinue("decision-timeout-owned", "confirm");
      const rejected = expect(pending).rejects.toMatchObject({code:"AG_DECISION_TIMEOUT_EFFECT_UNKNOWN"});
      await vi.advanceTimersByTimeAsync(30_001);
      await rejected;
      expect(submitted).toBe(1);
      finish();
      await Promise.resolve();
      await expect(resolveDecisionAndContinue("decision-timeout-owned", "confirm")).rejects.toMatchObject({code:"AG_DECISION_EFFECT_UNKNOWN"});
      expect(submitted).toBe(1);
      expect(invokeMock.mock.calls.filter(([command]) => command === "ag_start_turn")).toHaveLength(1);
      expect(agStore.getSnapshot().pendingDecision?.requestId).toBe("decision-timeout-owned");
    } finally {
      vi.useRealTimers();
    }
  });

  // covers: BR-WS-STATE-003
  // covers: BR-WS-OBS-002
  // 资源裁决由同一个 resolve 命令执行并刷新真实文件树，不另起模型续片段。
  it("workspace operation confirmation returns receipt and refreshes the file tree", async () => {
    invokeMock.mockImplementation(async (command, arg) => {
      if (command !== "dispatch") return undefined;
      const dispatched = arg as { request: { command: string } };
      if (dispatched.request.command === "ag.resolve_decision") {
        return {
          correlation_id: "ag-test",
          result: {
            task_run_id: "task-resource",
            status: "completed",
            workspace_operation_receipt: {
              operation_id: "operation-1",
              plan_id: "plan-1",
              final_status: "completed",
              item_receipts: [{ item_id: "item-1", seq: 0, status: "completed" }],
              compensation_receipts: [],
              entry_id: "entry-1",
            },
          },
        };
      }
      if (dispatched.request.command === "ws.list_docs") {
        return { correlation_id: "ws-test", result: [] };
      }
      return { correlation_id: "test", result: { ok: true } };
    });

    const result = await resolveDecision("decision-resource", "confirm");

    expect(result.workspaceOperationReceipt).toMatchObject({ operation_id: "operation-1" });
    expect(invokeMock.mock.calls.some(([command]) => command === "ag_start_turn")).toBe(false);
    expect(invokeMock).toHaveBeenCalledWith(
      "dispatch",
      expect.objectContaining({ request: expect.objectContaining({ command: "ws.list_docs" }) }),
    );

    // 同一资源裁决用例继续验证：确认前同步所有 LogicalState；成功回执同步 Diff 失效与引用重绑。
    edStore.loadDoc("notes/dirty.md", "disk", 1);
    edStore.updateContent("notes/dirty.md", "unsaved logical state");
    edStore.setTabState("notes/dirty.md", "dirty");
    const sessionId = activeSession(agStore.getSnapshot()).id;
    const reference = {
      refId: "ref-a",
      kind: "doc" as const,
      label: "a.md",
      payload: { kind: "doc" as const, relPath: "notes/a.md", label: "a.md" },
    };
    agStore.setComposerDraft(sessionId, {
      prompt: "整理引用",
      tokens: [reference],
      doc: {
        type: "doc",
        content: [{ type: "paragraph", content: [{ type: "referenceToken", attrs: reference }] }],
      },
    });
    deStore.addDiff({
      diffId: "diff-stale",
      sessionId,
      relPath: "notes/a.md",
      blockId: "b1",
      title: "旧建议",
      before: "a",
      after: "b",
      state: "pending",
    });
    agStore.setPendingDecision({
      requestId: "decision-rename",
      kind: "permission",
      payload: { workspaceOperationPlanId: "plan-rename" },
    });
    const dispatchedCommands: string[] = [];
    invokeMock.mockImplementation(async (command, arg) => {
      if (command !== "dispatch") return undefined;
      const request = (arg as { request: { command: string } }).request;
      dispatchedCommands.push(request.command);
      if (request.command === "ag.resolve_decision") {
        return {
          correlation_id: "ag-test",
          result: {
            task_run_id: "task-rename",
            status: "completed",
            workspace_operation_receipt: {
              operation_id: "operation-rename",
              plan_id: "plan-rename",
              final_status: "completed",
              item_receipts: [{ item_id: "item-rename", seq: 0, status: "completed" }],
              compensation_receipts: [],
              entry_id: "entry-rename",
              invalidated_diff_ids: ["diff-stale"],
            },
            workspace_operation_items: [{
              item_id: "item-rename",
              seq: 0,
              kind: "rename_resource",
              source_rel_path: "notes/a.md",
              target_rel_path: "notes/b.md",
              resource_kind: "file",
            }],
          },
        };
      }
      if (request.command === "ws.list_docs") return { correlation_id: "ws-test", result: [] };
      return { correlation_id: "test", result: { ok: true } };
    });

    await resolveDecision("decision-rename", "confirm");

    expect(dispatchedCommands.indexOf("ed.persist_session_snapshot"))
      .toBeLessThan(dispatchedCommands.indexOf("ag.resolve_decision"));
    const persistedDirty = invokeMock.mock.calls.find(([, payload]) => (
      (payload as { request?: { command?: string } })?.request?.command === "ed.persist_session_snapshot"
    ));
    expect(persistedDirty?.[1]).toEqual(expect.objectContaining({
      request: expect.objectContaining({
        params: expect.objectContaining({ snapshot: expect.objectContaining({ dirty: true }) }),
      }),
    }));
    expect(deStore.getSnapshot().diffs.find((diff) => diff.diffId === "diff-stale")?.state).toBe("expired");
    const draft = agStore.getSnapshot().sessions.find((session) => session.id === sessionId)?.composerDraft;
    expect(draft?.tokens[0]).toMatchObject({ label: "b.md", payload: { relPath: "notes/b.md" } });
    expect(draft?.doc.content?.[0].content?.[0].attrs).toMatchObject({
      label: "b.md",
      payload: { relPath: "notes/b.md" },
    });
  });

  // covers: BR-WS-DATA-003
  // covers: BR-WS-PERSIST-003
  // 新建文档确认后必须先读取真实目标建立 LogicalState，再把冻结初始内容投影为普通内容 Diff。
  it("create document confirmation opens the real file before projecting its initial content diff", async () => {
    invokeMock.mockImplementation(async (command, arg) => {
      if (command !== "dispatch") return undefined;
      const dispatched = arg as { request: { command: string } };
      switch (dispatched.request.command) {
        case "ag.resolve_decision":
          return {
            correlation_id: "ag-test",
            result: {
              task_run_id: "task-create",
              status: "waiting",
              workspace_operation_receipt: {
                operation_id: "operation-create",
                plan_id: "plan-create",
                final_status: "completed",
                item_receipts: [{ item_id: "item-create", seq: 0, status: "completed" }],
                compensation_receipts: [],
                entry_id: "entry-create",
              },
              workspace_operation_items: [{
                seq: 0,
                kind: "create_document",
                source_rel_path: null,
                target_rel_path: "draft.md",
                resource_kind: "file",
              }],
              create_document_bundle: {
                plan_id: "plan-create",
                target: { rel_path: "draft.md" },
                status: "created",
              },
            },
          };
        case "ws.list_docs":
          return { correlation_id: "ws-test", result: [] };
        case "ws.read_doc":
          return { correlation_id: "ws-test", result: { content: "", baseline: "disk-empty" } };
        case "ed.persist_session_snapshot":
          return { correlation_id: "ed-test", result: { ok: true } };
        case "ws.project_create_document_bundle":
          return {
            correlation_id: "ws-test",
            result: {
              diff_id: "diff-create",
              task_run_id: "task-create",
              source_turn_id: "turn-create",
              already_projected: false,
              projection: {
                diff_id: "diff-create",
                rel_path: "draft.md",
                patch_kind: "replace_document",
                block_id: null,
                block_index: null,
                before_text: "",
                after_text: "# 初稿\n\n正文",
                location_label: "整篇",
                draft_mode: "current_editor",
                snapshot_source: "active_editor_session",
                base_session_version: 1,
                base_content_hash: "logical-empty",
              },
            },
          };
        default:
          return { correlation_id: "test", result: { ok: true } };
      }
    });

    const result = await resolveDecision("decision-create", "confirm");

    expect(result.createDocumentDiffId).toBe("diff-create");
    expect(tabsStore.getSnapshot().tabs).toEqual([
      expect.objectContaining({ kind: "doc", path: "draft.md" }),
    ]);
    expect(edStore.getSnapshot().sessions["draft.md"]).toBeDefined();
    expect(deStore.getSnapshot().diffs).toEqual([
      expect.objectContaining({
        diffId: "diff-create",
        relPath: "draft.md",
        patchKind: "replace_document",
        state: "pending",
        before: "",
        after: "# 初稿\n\n正文",
        previewContinuationRequired: true,
      }),
    ]);
    const commands = invokeMock.mock.calls
      .filter(([command]) => command === "dispatch")
      .map(([, payload]) => (payload as { request: { command: string } }).request.command);
    expect(commands.indexOf("ws.read_doc")).toBeLessThan(commands.indexOf("ws.project_create_document_bundle"));
    const projectionRequest = invokeMock.mock.calls.find(([, payload]) => (
      (payload as { request?: { command?: string } })?.request?.command === "ws.project_create_document_bundle"
    ));
    expect(projectionRequest?.[1]).toEqual(expect.objectContaining({
      request: expect.objectContaining({
        params: expect.objectContaining({ beforeSnapshot: "" }),
      }),
    }));

    // 后端裁决已经成功后，投影 command 的前置失败必须走投影补偿，并把终态身份传给 UI。
    invokeMock.mockReset();
    edStore.reset();
    deStore.reset();
    tabsStore.reset();
    const failureCommands: string[] = [];
    invokeMock.mockImplementation(async (command, arg) => {
      if (command !== "dispatch") return undefined;
      const request = (arg as { request: { command: string } }).request;
      failureCommands.push(request.command);
      if (request.command === "ag.resolve_decision") {
        return {
          correlation_id: "ag-test",
          result: {
            task_run_id: "task-create-failed",
            status: "waiting",
            workspace_operation_receipt: {
              operation_id: "operation-create-failed",
              plan_id: "plan-create-failed",
              final_status: "completed",
              item_receipts: [{ item_id: "item-create-failed", seq: 0, status: "completed" }],
              compensation_receipts: [],
            },
            workspace_operation_items: [{
              item_id: "item-create-failed",
              seq: 0,
              kind: "create_document",
              source_rel_path: null,
              target_rel_path: "failed.md",
              resource_kind: "file",
            }],
            create_document_bundle: {
              plan_id: "plan-create-failed",
              target: { rel_path: "failed.md" },
              status: "resource_created",
            },
          },
        };
      }
      if (request.command === "ws.list_docs") return { correlation_id: "ws-test", result: [] };
      if (request.command === "ws.read_doc") {
        return { correlation_id: "ws-test", result: { content: "", baseline: "disk-empty" } };
      }
      if (request.command === "ed.persist_session_snapshot") {
        return { correlation_id: "ed-test", result: { ok: true } };
      }
      if (request.command === "ws.project_create_document_bundle") {
        return {
          correlation_id: "ws-test",
          error: {
            code: "WS_CONTENT_REQUIRED",
            message: "模拟投影前置失败",
            retriable: false,
            reauth_required: false,
          },
        };
      }
      if (request.command === "ws.compensate_create_document_projection_preflight_failure") {
        return { correlation_id: "ws-test", result: { final_status: "failed_compensated" } };
      }
      return { correlation_id: "test", result: { ok: true } };
    });

    await expect(resolveDecision("decision-create-failed", "confirm")).rejects.toMatchObject({
      code: "WS_CREATE_BUNDLE_PROJECTION_FAILED_COMPENSATED",
      decisionResolved: true,
      taskRunId: "task-create-failed",
    });
    expect(failureCommands).toContain("ws.compensate_create_document_projection_preflight_failure");
    expect(failureCommands).not.toContain("ws.compensate_create_document_open_failure");

    // guard 已创建后的后端投影写失败会直接返回补偿终态，不得伪造成 diff id。
    invokeMock.mockImplementation(async (command, arg) => {
      if (command !== "dispatch") return undefined;
      const request = (arg as { request: { command: string } }).request;
      if (request.command === "ag.resolve_decision") {
        return {
          correlation_id: "ag-test",
          result: {
            task_run_id: "task-create-compensated",
            status: "failed",
            workspace_operation_items: [{
              item_id: "item-create-compensated",
              seq: 0,
              kind: "create_document",
              source_rel_path: null,
              target_rel_path: "compensated.md",
              resource_kind: "file",
            }],
            create_document_bundle: {
              plan_id: "plan-create-compensated",
              target: { rel_path: "compensated.md" },
              status: "resource_created",
            },
          },
        };
      }
      if (request.command === "ws.list_docs") return { correlation_id: "ws-test", result: [] };
      if (request.command === "ws.read_doc") {
        return { correlation_id: "ws-test", result: { content: "", baseline: "disk-empty" } };
      }
      if (request.command === "ed.persist_session_snapshot") {
        return { correlation_id: "ed-test", result: { ok: true } };
      }
      if (request.command === "ws.project_create_document_bundle") {
        return {
          correlation_id: "ws-test",
          result: {
            projection_failed_compensated: true,
            error: { code: "DE_WRITE_FAILED", message: "模拟写入失败" },
          },
        };
      }
      return { correlation_id: "test", result: { ok: true } };
    });

    await expect(resolveDecision("decision-create-compensated", "confirm")).rejects.toMatchObject({
      code: "WS_CREATE_BUNDLE_PROJECTION_FAILED_COMPENSATED",
      decisionResolved: true,
    });
  });

  // covers: BR-SYS-STATE-008
  // 任务级确认恢复产生裁决请求的原 AgentTurn，而不是自动创建新回合。
  it("confirm resumes the parked AgentTurn", async () => {
    const available = modelStore.getSnapshot().models;
    const original = available[1];
    const changed = available[0];
    modelStore.setCapabilityPref("agentEdit", { mode: "explicit", modelKey: modelKey(original) });
    const turns: Array<Record<string, unknown>> = [];
    mockTurnInvoke((arg) => {
      turns.push(arg as Record<string, unknown>);
      const onEvent = (arg as { onEvent: { onmessage: ((m: TestTurnEvent) => void) | null } }).onEvent;
      if (turns.length === 1) {
        onEvent.onmessage?.({
          type: "decision_request_proposed",
          turn_id: "t",
          request_id: "decision-live",
          kind: "manual_confirm",
          payload: { summary: "请确认" },
        });
      } else {
        onEvent.onmessage?.({
          type: "decision_request_resolved",
          turn_id: "t",
          request_id: "decision-live",
          outcome: "confirm",
        });
        completeTurn(arg);
      }
    });

    await startTurn("执行需要确认的任务");
    modelStore.setCapabilityPref("agentEdit", { mode: "explicit", modelKey: modelKey(changed) });
    await resolveDecisionAndContinue("decision-live", "confirm");

    expect(turns).toHaveLength(2);
    expect(turns[1]).toMatchObject({
      turnId: turns[0].turnId,
      decisionResume: true,
      continuationTaskRunId: "task-confirmed",
      continuationDecisionRequestId: "decision-live",
      provider: original.provider,
      model: original.model,
    });
    expect(turns[0]).toMatchObject({ provider: original.provider, model: original.model });
    modelStore.setCapabilityPref("agentEdit", { mode: "followGlobal" });
  });

  // covers: BR-SYS-STATE-008, BR-AG-DATA-001
  // 只有 editor 已把 Diff 应用到 LogicalState 后，才向后端确认 checkpoint 并以新快照启动续片段。
  it("preview acknowledgement starts exactly one fresh-snapshot continuation", async () => {
    edStore.loadDoc("notes/current.md", "预览前", 101);
    edStore.updateContent("notes/current.md", "预览后");
    const session = edStore.getSnapshot().sessions["notes/current.md"];
    deStore.addDiff({
      diffId: "diff-preview",
      sessionId: activeSession(agStore.getSnapshot()).id,
      turnId: "turn-preview",
      taskRunId: "task-preview",
      relPath: "notes/current.md",
      blockId: "b1",
      title: "预览建议",
      before: "预览前",
      after: "预览后",
      state: "pending",
      draftApplied: true,
      previewContinuationRequired: true,
    });
    let turns = 0;
    invokeMock.mockImplementation(async (command, arg) => {
      if (command === "ag_ack_preview") return { task_run_id: "task-preview", ready: true };
      if (command === "ag_start_turn") {
        turns += 1;
        requestContext(arg, ["notes/current.md"]);
        completeTurn(arg);
        return undefined;
      }
      if (command === "dispatch") return dispatchOk(arg);
      return undefined;
    });

    await acknowledgePreviewAndContinue("diff-preview");

    expect(turns).toBe(1);
    expect(invokeMock).toHaveBeenCalledWith("ag_ack_preview", expect.objectContaining({
      taskRunId: "task-preview",
      turnId: "turn-preview",
      diffId: "diff-preview",
      relPath: "notes/current.md",
      sessionVersion: session.sessionVersion,
      contentHash: session.contentHash,
    }));
    expect(agStartArg()).toMatchObject({
      continuationTaskRunId: "task-preview",
      continuationPreviewDiffId: "diff-preview",
      continuationDecisionRequestId: undefined,
    });
    expect(agStartArg().openDocSnapshots).toEqual([]);
    expect(lastContextResponse().snapshots[0])
      .toMatchObject({ content: "预览后", sessionVersion: session.sessionVersion, contentHash: session.contentHash });
  });

  // covers: BR-SYS-STATE-008, BR-AG-DATA-001
  it("preview receipt preserves origin session and suppresses concurrent submission", async () => {
    edStore.loadDoc("notes/current.md", "preview", 101);
    const origin = activeSession(agStore.getSnapshot()).id;
    deStore.addDiff({ diffId: "p07-session", sessionId: origin, turnId: "p07-turn", taskRunId: "p07-task",
      relPath: "notes/current.md", blockId: "b1", title: "preview", before: "before", after: "preview",
      state: "pending", draftApplied: true, previewContinuationRequired: true });
    let release!: (value: unknown) => void;
    invokeMock.mockImplementation(async (command, arg) => {
      if (command === "ag_ack_preview") return new Promise((resolve) => { release = resolve; });
      if (command === "ag_start_turn") { requestContext(arg, ["notes/current.md"]); completeTurn(arg); }
      if (command === "dispatch") return dispatchOk(arg);
      return undefined;
    });
    const running = acknowledgePreviewAndContinue("p07-session");
    await acknowledgePreviewAndContinue("p07-session");
    const other = agStore.newSession();
    release({ task_run_id: "p07-task", ready: true });
    await running;
    expect(invokeMock.mock.calls.filter(([command]) => command === "ag_ack_preview")).toHaveLength(1);
    expect(invokeMock.mock.calls.filter(([command]) => command === "ag_start_turn")).toHaveLength(1);
    expect(agStartArg()).toMatchObject({ sessionId: origin, continuationTaskRunId: "p07-task" });
    expect(agStore.getSnapshot().sessions.find((s) => s.id === other)?.messages).toHaveLength(0);
  });

  // covers: BR-SYS-STATE-008, BR-AG-DATA-001
  it("preview receipt rejects a changed version and preserves a newer action", async () => {
    edStore.loadDoc("notes/current.md", "preview", 101);
    const origin = activeSession(agStore.getSnapshot()).id;
    deStore.addDiff({ diffId: "p07-stale", sessionId: origin, turnId: "p07-turn", taskRunId: "p07-task",
      relPath: "notes/current.md", blockId: "b1", title: "preview", before: "before", after: "preview",
      state: "pending", draftApplied: true, previewContinuationRequired: true });
    const newer = { sessionId: origin, turnId: "newer", action: "previewing" as const, label: "newer" };
    invokeMock.mockImplementation(async (command, arg) => {
      if (command === "ag_ack_preview") {
        edStore.updateContent("notes/current.md", "user changed content");
        agStore.setCurrentAction(newer);
        return { task_run_id: "p07-task", ready: true };
      }
      if (command === "dispatch") return dispatchOk(arg);
      return undefined;
    });
    await acknowledgePreviewAndContinue("p07-stale");
    expect(invokeMock).not.toHaveBeenCalledWith("ag_start_turn", expect.anything());
    expect(agStore.getSnapshot().currentAction).toBe(newer);
    expect(activeSession(agStore.getSnapshot()).messages.slice(-1)[0]?.content).toContain("AG_PREVIEW_CONTINUATION_STALE");
    expect(deStore.getSnapshot().diffs.find((d) => d.diffId === "p07-stale")?.state).toBe("pending");
  });

  // covers: BR-SYS-STATE-008, BR-AG-DATA-001
  it("preview rejected receipt reports in origin session without continuation", async () => {
    edStore.loadDoc("notes/current.md", "preview", 101);
    const origin = activeSession(agStore.getSnapshot()).id;
    deStore.addDiff({ diffId: "p07-reject", sessionId: origin, turnId: "p07-turn", taskRunId: "p07-task",
      relPath: "notes/current.md", blockId: "b1", title: "preview", before: "before", after: "preview",
      state: "pending", draftApplied: true, previewContinuationRequired: true });
    let other = "";
    invokeMock.mockImplementation(async (command, arg) => {
      if (command === "ag_ack_preview") {
        other = agStore.newSession();
        throw { code: "AG_PREVIEW_ACK_REJECTED", message: "expired" };
      }
      if (command === "dispatch") return dispatchOk(arg);
      return undefined;
    });
    await acknowledgePreviewAndContinue("p07-reject");
    expect(invokeMock).not.toHaveBeenCalledWith("ag_start_turn", expect.anything());
    expect(agStore.getSnapshot().sessions.find((s) => s.id === origin)?.messages.slice(-1)[0]?.content).toContain("AG_PREVIEW_ACK_REJECTED");
    expect(agStore.getSnapshot().sessions.find((s) => s.id === other)?.messages).toHaveLength(0);
    expect(agStore.getSnapshot().currentAction).toBeNull();
  });

  // covers: BR-SYS-STATE-008, BR-AG-DATA-001
  it("preview acknowledgement timeout releases UI but treats a late commit as unknown effect", async () => {
    vi.useFakeTimers();
    try {
      edStore.loadDoc("notes/current.md", "preview", 101);
      const origin = activeSession(agStore.getSnapshot()).id;
      deStore.addDiff({ diffId: "p07-timeout", sessionId: origin, turnId: "p07-turn", taskRunId: "p07-task",
        relPath: "notes/current.md", blockId: "b1", title: "preview", before: "before", after: "preview",
        state: "pending", draftApplied: true, previewContinuationRequired: true });
      let release!: (value: unknown) => void;
      invokeMock.mockImplementation(async (command, arg) => {
        if (command === "ag_ack_preview") return new Promise((resolve) => { release = resolve; });
        if (command === "dispatch") return dispatchOk(arg);
        return undefined;
      });
      const running = acknowledgePreviewAndContinue("p07-timeout");
      await vi.advanceTimersByTimeAsync(15_000);
      await running;
      expect(agStore.getSnapshot().currentAction).toBeNull();
      expect(agStore.getSnapshot().sessions.find((s) => s.id === origin)?.messages.slice(-1)[0]?.content)
        .toContain("AG_PREVIEW_ACK_TIMEOUT_EFFECT_UNKNOWN");
      release({ task_run_id: "p07-task", ready: true });
      await Promise.resolve();
      await acknowledgePreviewAndContinue("p07-timeout");
      expect(invokeMock.mock.calls.filter(([command]) => command === "ag_ack_preview")).toHaveLength(1);
      expect(invokeMock.mock.calls.filter(([command]) => command === "ag_start_turn")).toHaveLength(0);
    } finally {
      vi.useRealTimers();
    }
  });

  // covers: BR-SYS-STATE-008, BR-AG-DATA-001
  it("preview receipt does not continue after expiry or workspace switch or invalid readiness", async () => {
    for (const scenario of ["expired", "workspace", "not-ready"]) {
      workspaceShell._projectActive({ workspaceId: "ws-test", rootPath: "/tmp/ws-test" });
      edStore.loadDoc("notes/current.md", "preview", 101);
      const origin = activeSession(agStore.getSnapshot()).id;
      const diffId = `p07-${scenario}`;
      deStore.addDiff({ diffId, sessionId: origin, turnId: "p07-turn", taskRunId: "p07-task",
        relPath: "notes/current.md", blockId: "b1", title: "preview", before: "before", after: "preview",
        state: "pending", draftApplied: true, previewContinuationRequired: true });
      invokeMock.mockClear();
      const count = activeSession(agStore.getSnapshot()).messages.length;
      invokeMock.mockImplementation(async (command, arg) => {
        if (command === "ag_ack_preview") {
          if (scenario === "expired") deStore.expireLocal(diffId, { expiredReason: "document_revision_advanced", errorCode: "ED_DRAFT_BASE_ADVANCED" });
          if (scenario === "workspace") workspaceShell._projectActive({ workspaceId: "other", rootPath: "/tmp/other" });
          return { task_run_id: "p07-task", ready: scenario !== "not-ready" };
        }
        if (command === "dispatch") return dispatchOk(arg);
        return undefined;
      });
      await acknowledgePreviewAndContinue(diffId);
      expect(invokeMock).not.toHaveBeenCalledWith("ag_start_turn", expect.anything());
      expect(activeSession(agStore.getSnapshot()).messages.length).toBe(count + (scenario === "workspace" ? 0 : 1));
    }
  });

  // covers: BR-SYS-STATE-008, BR-AG-DATA-001
  // 简单局部编辑没有 TaskRun；即使旧投影误带 continuation 标记，前端也不得伪造任务身份自动续片段。
  it("does not acknowledge or continue a simple diff without task run identity", async () => {
    edStore.loadDoc("notes/current.md", "预览前", 101);
    edStore.updateContent("notes/current.md", "预览后");
    deStore.addDiff({
      diffId: "diff-simple",
      sessionId: activeSession(agStore.getSnapshot()).id,
      turnId: "turn-simple",
      relPath: "notes/current.md",
      blockId: "b1",
      title: "简单建议",
      before: "预览前",
      after: "预览后",
      state: "pending",
      draftApplied: true,
      previewContinuationRequired: true,
    });

    await acknowledgePreviewAndContinue("diff-simple");

    expect(invokeMock).not.toHaveBeenCalledWith("ag_ack_preview", expect.anything());
    expect(invokeMock.mock.calls.some(([command]) => command === "ag_start_turn")).toBe(false);
  });

  // covers: BR-SYS-STATE-008
  // 仅有历史 DecisionRequest 投影、没有当前进程内 AwaitingDecision 身份时，不猜测并新建回合。
  it("does not fabricate continuation turns for historical decisions", async () => {
    let turnCount = 0;
    mockTurnInvoke((arg) => {
      turnCount += 1;
      completeTurn(arg);
    });
    await resolveDecisionAndContinue("historical-confirm", "confirm");
    await resolveDecisionAndContinue("historical-cancel", "cancel");
    expect(turnCount).toBe(0);
  });

  // covers: BR-SYS-STATE-008
  // 当前 route revision 投影（TERM-AG-018）不携带执行游标。
  it("projects task plans per session", () => {
    const sessionId = agStore.getSnapshot().activeSessionId;
    agStore.hydrateSessionPlans(sessionId, [
      {
        planId: "plan-1",
        taskRunId: "run-1",
        throughSeq: 4,
        routeRevision: 2,
        summary: "读取后生成建议",
        candidateSteps: [
          { stepId: "s1", kind: "candidate", label: "读取 a.md", status: "candidate", dependsOn: [], sourceEventId: "e4", sourceEventSeq: 4 },
          { stepId: "s2", kind: "candidate", label: "生成建议", status: "candidate", dependsOn: ["s1"], sourceEventId: "e4", sourceEventSeq: 4 },
        ],
      },
    ]);
    const plans = agStore.getSnapshot().sessions.find((s) => s.id === sessionId)?.taskPlans ?? [];
    expect(plans[0].routeRevision).toBe(2);
    expect(plans[0].candidateSteps[1].dependsOn).toEqual(["s1"]);
  });

  // covers: BR-SYS-STATE-008
  // AG-1.0-TASKRUN-REWORK-INTENT-01：刷新会话任务计划（取最近 run 的计划并投影）。
  it("refreshes session plans from latest run", async () => {
    const sessionId = agStore.getSnapshot().activeSessionId;
    invokeMock.mockImplementation(async (_cmd: string, arg: unknown) => {
      const req = (arg as { request: { command: string } }).request;
      if (req.command === "ag.list_task_runs") {
        return {
          correlation_id: "c",
          result: [
            {
              task_run_id: "run-1",
              source_turn_ids: ["turn-1"],
              through_seq: 2,
              goal: "完成任务",
              status: "active",
              observed_steps: [],
              waiting_on: [],
              candidate_steps: [],
              created_at: 1,
              updated_at: 1,
            },
          ],
        };
      }
      if (req.command === "ag.list_plans") {
        return {
          correlation_id: "c",
          result: [{
            plan_id: "plan-1",
            task_run_id: "run-1",
            through_seq: 2,
            route_revision: 1,
            summary: "当前路线",
            candidate_steps: [],
          }],
        };
      }
      return { correlation_id: "c", result: {} };
    });

    await refreshSessionPlans(sessionId);

    const plans = agStore.getSnapshot().sessions.find((s) => s.id === sessionId)?.taskPlans ?? [];
    expect(plans[0]?.planId).toBe("plan-1");
  });
});

describe("决策门控（AG-1.0-DECISION-CARD-01）", () => {
  // covers: BR-SYS-STATE-008
  // 存在待裁决任务级请求时，手动 startTurn 被门控（不发起回合），避免"确认"文本被当编辑请求。
  it("blocks manual startTurn while a decision is pending", async () => {
    agStore.reset();
    agStore.setPendingDecision({ requestId: "d-1", kind: "permission", payload: { summary: "需要权限" } });
    let turnCount = 0;
    mockTurnInvoke(() => {
      turnCount += 1;
    });
    await startTurn("确认");
    expect(turnCount).toBe(0); // 被门控，未发起 ag_start_turn
    agStore.setPendingDecision(null);
  });

  // covers: BR-SYS-STATE-008
  // 历史待裁决项可被解析，但没有活动 AwaitingDecision 时不得伪造 AgentTurn。
  it("resolves a historical pending decision without fabricating an AgentTurn", async () => {
    agStore.reset();
    agStore.setPendingDecision({ requestId: "d-2", kind: "plan_choice" });
    let turnCount = 0;
    mockTurnInvoke((arg) => {
      turnCount += 1;
      completeTurn(arg);
    });
    await resolveDecisionAndContinue("d-2", "confirm");
    expect(turnCount).toBe(0);
    agStore.setPendingDecision(null);
  });
});
