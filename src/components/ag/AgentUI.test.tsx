/** @vitest-environment jsdom */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { agStore } from "../../store/agStore";
import { assistStore, type AssistTarget } from "../../store/assistStore";
import { deStore, type DiffView } from "../../store/deStore";
import { wsStore } from "../../store/wsStore";
import { DOC_REF_MIME, SELECTION_REF_MIME, type SelectionRef } from "../../services/edLocate";
import { startReferenceDrag } from "../../services/referenceDrag";
import { LONG_TEXT_REF_THRESHOLD, REFERENCE_MIME, type ReferencePayload } from "../../services/referencePayload";
import { deleteAgentSession, startTurn } from "../../services/agService";
import { AgentPanel } from "./AgentPanel";
import { AutocompleteOverlay } from "./AutocompleteOverlay";
import { Composer } from "./Composer";
import { modelKey, modelStore } from "../../store/modelStore";
import { MessageList } from "./MessageList";
import { MessageFlow } from "./MessageFlow";
import { ToolCallCard } from "./ToolCallCard";
import { MessageMarkdown } from "./MessageMarkdown";
import { DecisionBar } from "./DecisionBar";
import { AgentVerificationPanel } from "./AgentVerificationPanel";
import { invoke } from "@tauri-apps/api/core";
vi.mock("@tauri-apps/api/core", async (original) => ({
  ...await original<typeof import("@tauri-apps/api/core")>(),
  invoke: vi.fn(),
  Channel: class { onmessage: ((event: unknown) => void) | null = null; },
}));
import type { ChatMessage, ToolCallView } from "../../store/agStore";

/** DiffView 测试构造（AG-MSG-RENDER-01：MessageFlow 时序测试复用）。 */
function diff(patch: Partial<DiffView>): DiffView {
  return {
    diffId: patch.diffId ?? "diff",
    blockId: patch.blockId ?? "b1",
    relPath: patch.relPath ?? "notes/a.md",
    title: patch.title ?? "修改建议",
    before: patch.before ?? "旧文本",
    after: patch.after ?? "新文本",
    state: patch.state ?? "pending",
    ...patch,
  };
}

const agServiceMock = vi.hoisted(() => ({
  startTurn: vi.fn(),
  uploadByok: vi.fn(),
  deleteAgentSession: vi.fn(),
  newAgentSession: vi.fn(() => "s-new"),
  switchAgentSession: vi.fn(),
  closeAgentSession: vi.fn(),
  cancelActiveTurn: vi.fn(),
  resolveDecisionAndContinue: vi.fn(),
  inspectTurnEffects: vi.fn(),
}));

vi.mock("../../services/agService", () => ({
  startTurn: agServiceMock.startTurn,
  uploadByok: agServiceMock.uploadByok,
  deleteAgentSession: agServiceMock.deleteAgentSession,
  newAgentSession: agServiceMock.newAgentSession,
  switchAgentSession: agServiceMock.switchAgentSession,
  closeAgentSession: agServiceMock.closeAgentSession,
  cancelActiveTurn: agServiceMock.cancelActiveTurn,
  resolveDecisionAndContinue: agServiceMock.resolveDecisionAndContinue,
  inspectTurnEffects: agServiceMock.inspectTurnEffects,
}));

vi.mock("../../services/deService", () => ({
  acceptDiff: vi.fn(),
  rejectDiff: vi.fn(),
  generateSampleDiff: vi.fn(),
}));

vi.mock("../../services/edLocate", async () => {
  const actual = await vi.importActual<typeof import("../../services/edLocate")>("../../services/edLocate");
  return { ...actual, revealBlock: vi.fn(() => "located") };
});

function dataTransfer(values: Record<string, string>) {
  const types = Object.keys(values);
  return {
    types,
    dropEffect: "none",
    getData(type: string) {
      return values[type] ?? "";
    },
    setData(type: string, value: string) {
      values[type] = value;
      if (!types.includes(type)) types.push(type);
    },
  };
}

function domStringListDataTransfer(values: Record<string, string>) {
  const base = dataTransfer(values);
  const typeNames = Object.keys(values);
  return {
    ...base,
    types: {
      contains: (value: string) => typeNames.includes(value),
      length: typeNames.length,
      item: (index: number) => typeNames[index] ?? null,
    },
  };
}

function dispatchDragEvent(target: HTMLElement, type: "dragover" | "drop", transfer: unknown) {
  const event = new Event(type, { bubbles: true, cancelable: true });
  Object.defineProperty(event, "dataTransfer", { value: transfer });
  target.dispatchEvent(event);
  return event;
}

function selectionRef(overrides: Partial<SelectionRef> = {}): SelectionRef {
  return {
    path: "notes/a.md",
    blockId: "abcdef123",
    start: 2,
    end: 5,
    text: "选中的内容",
    ...overrides,
  };
}

function renderComposer() {
  const rendered = render(<Composer />);
  const input = screen.getByRole("textbox") as HTMLElement;
  const dropTarget = rendered.container.querySelector(".composer") as HTMLElement;
  const editor = rendered.container.querySelector(".composer-prosemirror") as HTMLElement;
  return { ...rendered, input, dropTarget, editor };
}

beforeEach(() => {
  vi.mocked(invoke).mockReset();
  agServiceMock.startTurn.mockReset();
  agServiceMock.uploadByok.mockReset();
  agServiceMock.cancelActiveTurn.mockReset();
  agServiceMock.resolveDecisionAndContinue.mockReset();
  agServiceMock.inspectTurnEffects.mockReset();
  agStore.reset();
  wsStore.reset();
});

afterEach(cleanup);

describe("development verification entry", () => {
  // covers: BR-AG-UI-001
  it("exposes production read recovery paths through the existing isolated selector", () => {
    render(<AgentVerificationPanel />);
    const selector=screen.getByLabelText("验证场景");
    for (const scenario of ["page-contract-recovery","page-contract-exhausted","repeated-oversized-read","repeated-read-tools-blocked","read-failure-same-batch","read-failure-with-evidence","read-failure-gate-denied","read-failure-clarification"]) {
      fireEvent.change(selector,{target:{value:scenario}});
      expect((selector as HTMLSelectElement).value).toBe(scenario);
    }
    expect(invoke).not.toHaveBeenCalled();
  });
  // covers: BR-AG-UI-001
  it("batch entry preserves real receipts, stops on mismatch and does not manufacture a pass", async () => {
    vi.mocked(invoke).mockImplementation(async (_command, args) => {
      const scenario = (args as { scenario: string }).scenario;
      return { status: "completed", events: [{ type: "done", stopReason: "stop" }], marker: scenario };
    });
    render(<AgentVerificationPanel />);
    fireEvent.click(screen.getByText("批量运行核心验收"));
    fireEvent.click(screen.getByText("批量运行核心验收"));
    await act(async () => { await Promise.resolve(); });
    expect(screen.getByRole("status").textContent).toContain("批量验收失败，已核验 2/25");
    expect(vi.mocked(invoke).mock.calls.filter(([command]) => command === "ag_verify_run")).toHaveLength(3);
    const evidence = JSON.parse(screen.getByLabelText("批量核心验收证据").textContent!);
    expect(evidence[0]).toMatchObject({ scenario: "chat", matched: true, receipt: { marker: "chat" } });
    expect(evidence[2]).toMatchObject({ scenario: "invalid-json", matched: false, receipt: { status: "completed" } });
    expect((screen.getByText("批量运行核心验收") as HTMLButtonElement).disabled).toBe(false);
  });
  // covers: BR-AG-UI-001
  it("shows actual receipts without declaring business success and blocks double start", async () => {
    let finish!: (value: unknown) => void;
    vi.mocked(invoke).mockImplementation(async () => new Promise((resolve) => { finish = resolve; }));
    render(<AgentVerificationPanel />);
    fireEvent.click(screen.getByText("运行验证"));
    fireEvent.click(screen.getByText("运行验证"));
    expect(invoke).toHaveBeenCalledTimes(1);
    expect(vi.mocked(invoke).mock.calls[0][0]).toBe("ag_verify_run");
    await act(async () => finish({ status: "error", events: [{ type: "error", code: "AG_MODEL_CONTROL_INVALID" }], modelToolMessages: [{ toolCallId: "observed-call", content: "observed-tool-output" }] }));
    expect(screen.getByRole("status").textContent).toContain("不等于业务成功");
    expect(screen.getByLabelText("运行证据").textContent).toContain("AG_MODEL_CONTROL_INVALID");
    expect(screen.getByLabelText("工具回注证据").textContent).toContain("observed-tool-output");
  });
  // covers: BR-AG-UI-001
  // covers: BR-AG-DATA-005
  it("selects the isolated budget scenario and separates internal counts from safe events", async () => {
    vi.mocked(invoke).mockResolvedValue({
      status: "error",
      events: [{ type: "error", code: "AG_PROMPT_BUDGET_EXCEEDED" }],
      budgetChecks: [{ assemblyReason: "initial", exceeded: true, measurement: { messageLayers: { user: 120000 } } }],
      requestCounts: { gate: 0, model: 0 },
      modelToolMessages: [],
    });
    render(<AgentVerificationPanel />);
    fireEvent.change(screen.getByLabelText("验证场景"), { target: { value: "budget-initial" } });
    fireEvent.click(screen.getByText("运行验证"));
    await waitFor(() => expect(screen.getByRole("status").textContent).toContain("Agent 状态：error"));
    expect(vi.mocked(invoke).mock.calls[0][1]).toEqual(expect.objectContaining({ scenario: "budget-initial" }));
    expect(screen.getByLabelText("运行证据").textContent).toContain("AG_PROMPT_BUDGET_EXCEEDED");
    expect(screen.getByLabelText("预算分项证据").textContent).toContain("messageLayers");
    expect(screen.getByText(/隔离网关实际请求：Gate 0 次，模型 0 次/)).toBeTruthy();
    vi.mocked(invoke).mockResolvedValue({
      status: "error",
      followUp: { turnId: "new-turn", sessionId: "session-1", status: "completed" },
      events: [{ type: "error", turnId: "first-turn", code: "AG_PROMPT_BUDGET_EXCEEDED" }, { type: "done", turnId: "new-turn" }],
      budgetChecks: [{ turnId: "first-turn", exceeded: true }, { turnId: "new-turn", exceeded: false }],
      requestCounts: { gate: 1, model: 1 },
      modelToolMessages: [],
    });
    fireEvent.change(screen.getByLabelText("验证场景"), { target: { value: "budget-recovery" } });
    fireEvent.click(screen.getByText("运行验证"));
    await waitFor(() => expect(screen.getByLabelText("后续回合状态").textContent).toContain("new-turn：completed"));
    expect(screen.getByLabelText("后续回合状态").textContent).toContain("session-1");
    expect(vi.mocked(invoke).mock.calls[1][1]).toEqual(expect.objectContaining({ scenario: "budget-recovery" }));
    expect(screen.getByLabelText("运行证据").textContent).toContain("first-turn");
    expect(screen.getByText(/隔离网关实际请求：Gate 1 次，模型 1 次/)).toBeTruthy();
  });
  // covers: BR-AG-UI-001
  // covers: BR-SYS-STATE-008
  it("labels automatic decision continuation as core evidence rather than a user click", async () => {
    vi.mocked(invoke).mockResolvedValue({
      status: "completed",
      events: [{ type: "decision_request_proposed", requestId: "A" }, { type: "decision_request_resolved", requestId: "A" }],
      decisionTrace: [{ requestId: "A", taskRunId: "task-1", outcome: "confirm", alreadyResolved: false, duplicateAlreadyResolved: true }],
      requestCounts: { gate: 3, model: 3 },
      modelToolMessages: [],
    });
    render(<AgentVerificationPanel />);
    fireEvent.change(screen.getByLabelText("验证场景"), { target: { value: "decision-chain" } });
    fireEvent.click(screen.getByText("运行验证"));
    await waitFor(() => expect(screen.getByRole("status").textContent).toContain("Agent 状态：completed"));
    expect(vi.mocked(invoke).mock.calls[0][1]).toEqual(expect.objectContaining({ scenario: "decision-chain" }));
    expect(screen.getByLabelText("自动裁决证据").textContent).toContain("task-1");
    expect(screen.getByLabelText("自动裁决证据").textContent).toContain("duplicateAlreadyResolved");
    expect(screen.getByText(/非用户点击、非 DecisionBar 布局验收/)).toBeTruthy();
  });
  // covers: BR-AG-UI-001, BR-SYS-STATE-008
  it("labels isolated preview and session evidence without claiming product success", async () => {
    vi.mocked(invoke).mockResolvedValue({
      status: "preview_guard_checked", events: [], requestCounts: { gate: 0, model: 0 },
      previewTrace: { staleV2ContinuationRejected: true, matchingV1ContinuationOnce: true },
      modelToolMessages: [],
    });
    render(<AgentVerificationPanel />);
    fireEvent.change(screen.getByLabelText("验证场景"), { target: { value: "preview-guard" } });
    fireEvent.click(screen.getByText("运行验证"));
    await waitFor(() => expect(screen.getByRole("status").textContent).toContain("非 Agent 回合、非编辑器验收"));
    expect(vi.mocked(invoke).mock.calls[0][1]).toEqual(expect.objectContaining({ scenario: "preview-guard" }));
    expect(screen.getByLabelText("预览检查点证据").textContent).toContain("staleV2ContinuationRejected");
    expect(screen.getByText(/不验证前端超时、实际编辑器、接受或保存/)).toBeTruthy();
    vi.mocked(invoke).mockResolvedValue({
      status: "session_memory_checked", events: [], requestCounts: { gate: 6, model: 6 },
      promptTrace: [{ selectedMemory: true }, { selectedMemory: false }],
      sessionTrace: { waitingTaskStatus: "waiting", crossSessionHistoryRejectedBeforeGate: true },
      modelToolMessages: [],
    });
    fireEvent.change(screen.getByLabelText("验证场景"), { target: { value: "session-memory" } });
    fireEvent.click(screen.getByText("运行验证"));
    await waitFor(() => expect(screen.getByRole("status").textContent).toContain("非真实模型、非记忆库面板验收"));
    expect(vi.mocked(invoke).mock.calls[1][1]).toEqual(expect.objectContaining({ scenario: "session-memory" }));
    expect(screen.getByLabelText("会话装配证据").textContent).toContain("selectedMemory");
    expect(screen.getByLabelText("会话状态证据").textContent).toContain("crossSessionHistoryRejectedBeforeGate");
    expect(screen.getByText(/非实际标签切换布局验收/)).toBeTruthy();
  });
  // covers: BR-AG-UI-001
  it("sends cancellation only to its verification identity and retains transport failure", async () => {
    let fail!: (reason: unknown) => void;
    let finishCancel!: (value: unknown) => void;
    vi.mocked(invoke).mockImplementation(async (command) => command === "ag_verify_run"
      ? new Promise((_, reject) => { fail = reject; }) : new Promise((resolve) => { finishCancel = resolve; }));
    render(<AgentVerificationPanel />);
    fireEvent.change(screen.getByLabelText("验证场景"), { target: { value: "budget-recovery-delay" } });
    fireEvent.click(screen.getByText("运行验证"));
    const id = (vi.mocked(invoke).mock.calls[0][1] as { runId: string }).runId;
    expect(vi.mocked(invoke).mock.calls[0][1]).toEqual(expect.objectContaining({ scenario: "budget-recovery-delay" }));
    fireEvent.click(screen.getByText("取消验证"));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("ag_verify_cancel", { runId: id }));
    await act(async () => fail({ code: "AG_VERIFICATION_ERROR", message: "transport failed" }));
    await act(async () => finishCancel(undefined));
    expect(screen.getByRole("status").textContent).toContain("transport failed");
  });
  // covers: BR-AG-UI-001
  it("releases the verification controls after a cancelled follow-up and permits another run", async () => {
    let finishFirst!: (value: unknown) => void;
    let runCount = 0;
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === "ag_verify_cancel") return undefined;
      runCount += 1;
      if (runCount === 1) return new Promise((resolve) => { finishFirst = resolve; });
      return { status: "completed", events: [{ type: "done", stopReason: "stop" }], modelToolMessages: [] };
    });
    render(<AgentVerificationPanel />);
    fireEvent.change(screen.getByLabelText("验证场景"), { target: { value: "budget-recovery-delay" } });
    const runButton = screen.getByRole("button", { name: "运行验证" }) as HTMLButtonElement;
    const cancelButton = screen.getByRole("button", { name: "取消验证" }) as HTMLButtonElement;
    fireEvent.click(runButton);
    expect(runButton.disabled).toBe(true);
    fireEvent.click(cancelButton);
    await waitFor(() => expect(screen.getByRole("status").textContent).toContain("已请求取消"));
    await act(async () => finishFirst({
      status: "error",
      followUp: { turnId: "second-turn", sessionId: "same-session", status: "cancelled" },
      events: [{ type: "error", turnId: "first-turn", code: "AG_PROMPT_BUDGET_EXCEEDED" }, { type: "done", turnId: "second-turn", stopReason: "cancelled" }],
      modelToolMessages: [],
    }));
    expect(runButton.disabled).toBe(false);
    expect(cancelButton.disabled).toBe(true);
    expect(screen.getByLabelText("后续回合状态").textContent).toContain("second-turn：cancelled");
    fireEvent.click(runButton);
    await waitFor(() => expect(screen.getByRole("status").textContent).toContain("Agent 状态：completed"));
    expect(runCount).toBe(2);
  });
});

describe("DecisionRequest identity isolation", () => {
  function pending(requestId: string) {
    return { requestId, sessionId: agStore.getSnapshot().activeSessionId, turnId: "turn-deciding", kind: "manual_confirm", payload: {summary: requestId} };
  }

  // covers: BR-SYS-STATE-007
  // covers: BR-AG-UI-001
  it("keeps successor request and action when the old submission finishes", async () => {
    let finish!: (value: unknown) => void;
    agServiceMock.resolveDecisionAndContinue.mockImplementation(() => new Promise(resolve => { finish = resolve; }));
    agStore.setPendingDecision(pending("A"));
    render(<DecisionBar />);
    const button = screen.getByRole("button", {name:"确认并继续"});
    act(() => { fireEvent.click(button); fireEvent.click(button); });
    expect(agServiceMock.resolveDecisionAndContinue).toHaveBeenCalledTimes(1);
    const b = pending("B");
    const action = {sessionId:b.sessionId,turnId:b.turnId,action:"deciding" as const,label:"等待 B"};
    act(() => { agStore.setPendingDecision(b); agStore.setCurrentAction(action); });
    expect((screen.getByRole("button", {name:"确认并继续"}) as HTMLButtonElement).disabled).toBe(false);
    await act(async () => finish({status:"waiting"}));
    expect(agStore.getSnapshot().pendingDecision).toEqual(b);
    expect(agStore.getSnapshot().currentAction).toBe(action);
    expect(screen.getByText("B")).not.toBeNull();
  });

  // covers: BR-SYS-STATE-007
  it("does not leak old errors into the new request and permits a failed request retry", async () => {
    let fail!: (cause: unknown) => void;
    agServiceMock.resolveDecisionAndContinue.mockImplementationOnce(() => new Promise((_,reject) => {fail=reject;}));
    agStore.setPendingDecision(pending("old"));
    render(<DecisionBar />);
    fireEvent.click(screen.getByRole("button", {name:"确认并继续"}));
    const next = pending("next");
    act(() => agStore.setPendingDecision(next));
    await act(async () => fail({code:"OLD",message:"old failure",decisionResolved:true}));
    expect(agStore.getSnapshot().pendingDecision).toEqual(next);
    expect(screen.queryByText(/old failure/)).toBeNull();
    expect(agStore.getSnapshot().sessions.find(s=>s.id===next.sessionId)?.messages.some(m=>m.error && m.content.includes("old failure"))).toBe(true);
    agServiceMock.resolveDecisionAndContinue.mockRejectedValueOnce({code:"RETRY",message:"暂时失败"});
    fireEvent.click(screen.getByRole("button", {name:"确认并继续"}));
    await screen.findByText("[RETRY] 暂时失败");
    expect(agStore.getSnapshot().pendingDecision).toEqual(next);
    agServiceMock.resolveDecisionAndContinue.mockResolvedValueOnce({status:"completed"});
    fireEvent.click(screen.getByRole("button", {name:"确认并继续"}));
    await waitFor(() => expect(agStore.getSnapshot().pendingDecision).toBeNull());
  });

  // covers: BR-SYS-STATE-007
  it("writes a delayed resource receipt only to the originating session", async () => {
    let finish!: (value: unknown) => void;
    agServiceMock.resolveDecisionAndContinue.mockImplementationOnce(() => new Promise(resolve => {finish=resolve;}));
    const origin = pending("resource");
    agStore.setPendingDecision({...origin,payload:{workspaceOperationPlanId:"plan",summary:"资源操作"}});
    render(<DecisionBar />);
    fireEvent.click(screen.getByRole("button",{name:"确认并执行"}));
    let other = "";
    act(() => { other = agStore.newSession(); });
    expect(screen.queryByText("等待你的决策")).toBeNull();
    await act(async () => finish({workspaceOperationReceipt:{final_status:"failed",item_receipts:[],compensation_receipts:[]}}));
    const sessions=agStore.getSnapshot().sessions;
    expect(sessions.find(s=>s.id===origin.sessionId)?.messages.some(m=>m.content.includes("资源操作未完整完成"))).toBe(true);
    expect(sessions.find(s=>s.id===other)?.messages).toHaveLength(0);
  });

  // covers: BR-SYS-STATE-007
  it("clears only matching request identity and never a replacement action object", () => {
    const decision=pending("guard");
    agStore.setPendingDecision(decision);
    for (const wrong of [{...decision,sessionId:"other"},{...decision,turnId:"other"},{...decision,requestId:"other"}]) {
      expect(agStore.clearPendingDecision(wrong)).toBe(false);
      expect(agStore.getSnapshot().pendingDecision).toEqual(decision);
    }
    const old = {sessionId:decision.sessionId,turnId:decision.turnId,action:"tool" as const,label:"old"};
    const replacement={...old};
    agStore.setCurrentAction(replacement);
    agStore.clearCurrentAction(old);
    expect(agStore.getSnapshot().currentAction).toBe(replacement);
    expect(agStore.clearPendingDecision(decision)).toBe(true);
    expect(agStore.getSnapshot().currentAction).toBe(replacement);
    agStore.setPendingDecision({requestId:"legacy-timeline",kind:"manual_confirm"});
    const boundSession=agStore.getSnapshot().activeSessionId;
    agStore.newSession();
    expect(agStore.getSnapshot().pendingDecision?.sessionId).toBe(boundSession);
    render(<DecisionBar />);
    expect(screen.queryByText("等待你的决策")).toBeNull();
  });
});

describe("Composer · 引用令牌", () => {
  // covers: BR-AG-DATA-007
  it("显示代理编辑的实际偏好，选择只改变代理编辑而不改全局默认", () => {
    const models = modelStore.getSnapshot().models;
    modelStore.select(modelKey(models[0]));
    modelStore.setCapabilityPref("agentEdit", { mode: "explicit", modelKey: modelKey(models[1]) });
    renderComposer();
    const selector = screen.getByTitle("模型选择");
    expect(selector.textContent).toContain(models[1].label);
    fireEvent.click(selector);
    fireEvent.click(screen.getByText(models[2].label));
    expect(screen.getByTitle("模型选择").textContent).toContain(models[2].label);
    expect(modelStore.getSnapshot().selectedKey).toBe(modelKey(models[0]));
    expect(modelStore.resolveFor("agentEdit")).toEqual(models[2]);
    modelStore.setCapabilityPref("agentEdit", { mode: "followGlobal" });
  });
  // covers: BR-REF-DATA-004
  // SelectionRef 粘贴必须生成输入流内 ReferenceToken，并同步显示草稿引用。
  it("粘贴编辑器 SelectionRef 时在输入流和草稿区同时生成引用", async () => {
    const { input, editor } = renderComposer();
    expect(screen.queryByText("build")).toBeNull();
    expect(screen.getByTitle("agent-edit-1.0 · 文档编辑模式").textContent).toBe("edit");
    const ref = selectionRef();

    fireEvent.paste(input, {
      clipboardData: dataTransfer({ [SELECTION_REF_MIME]: JSON.stringify(ref) }),
    });

    await waitFor(() => {
      expect(editor.querySelectorAll(".ref-token").length).toBe(1);
    });
    expect(screen.queryByLabelText("草稿引用")).not.toBeNull();
    expect(screen.getAllByText("选区 a.md#abcdef[2-5]").length).toBeGreaterThanOrEqual(1);

    act(() => agStore.setActiveTurn("session-1", "turn-1"));
    fireEvent.click(screen.getByTitle("停止当前 Agent 回合"));
    expect(agServiceMock.cancelActiveTurn).toHaveBeenCalledTimes(1);
  });

  // covers: BR-REF-DATA-004
  // 文档引用拖入输入面必须进入正文流，而不是只停留在输入框上方。
  it("拖入文件树或标签栏文档引用时生成 doc ReferencePayload token", async () => {
    const { dropTarget, editor } = renderComposer();

    fireEvent.drop(dropTarget, {
      dataTransfer: dataTransfer({ [DOC_REF_MIME]: JSON.stringify({ relPath: "notes/a.md" }) }),
    });

    await waitFor(() => {
      expect(editor.querySelectorAll(".ref-token-doc").length).toBe(1);
    });
    expect(screen.getAllByText("notes/a.md").length).toBeGreaterThanOrEqual(1);
  });

  // covers: BR-REF-DATA-004
  // drop 落在 TipTap 内容区时，Composer capture 阶段必须识别 DOMStringList 形态 types 并生成引用。
  it("在编辑器内部 drop 时兼容 DOMStringList 形态 DataTransfer.types", async () => {
    const { editor } = renderComposer();
    const transfer = domStringListDataTransfer({
      [DOC_REF_MIME]: JSON.stringify({ relPath: "notes/webview.md" }),
    });

    const dragover = dispatchDragEvent(editor, "dragover", transfer);
    expect(dragover.defaultPrevented).toBe(true);
    dispatchDragEvent(editor, "drop", transfer);

    await waitFor(() => {
      expect(editor.querySelectorAll(".ref-token-doc").length).toBe(1);
    });
    expect(screen.getAllByText("notes/webview.md").length).toBeGreaterThanOrEqual(1);
  });

  // covers: BR-REF-DATA-003
  // session 等新引用类型通过统一 ReferencePayload MIME 进入 Composer。
  it("支持会话等新 ReferencePayload MIME 进入 Composer", async () => {
    const { dropTarget, editor } = renderComposer();
    const payload: ReferencePayload = {
      kind: "session",
      sessionId: "s1",
      title: "需求讨论",
      messages: [{ role: "user", content: "引用需求" }],
    };

    fireEvent.drop(dropTarget, {
      dataTransfer: dataTransfer({ [REFERENCE_MIME]: JSON.stringify(payload) }),
    });

    await waitFor(() => {
      expect(editor.querySelectorAll(".ref-token-session").length).toBe(1);
    });
    expect(screen.getAllByText("会话 需求讨论").length).toBeGreaterThanOrEqual(1);
  });

  // covers: BR-REF-DATA-003
  // 外部 URL 与长文本粘贴映射为 url/long_text ReferencePayload。
  it("外部 URL 和超过阈值的纯文本粘贴会形成引用令牌", async () => {
    const { input, editor } = renderComposer();

    fireEvent.paste(input, {
      clipboardData: dataTransfer({ "text/plain": "https://example.com/a" }),
    });
    await waitFor(() => {
      expect(editor.querySelectorAll(".ref-token-url").length).toBe(1);
    });

    const long = "a".repeat(LONG_TEXT_REF_THRESHOLD + 1);
    fireEvent.paste(input, {
      clipboardData: dataTransfer({ "text/plain": long }),
    });
    await waitFor(() => {
      expect(editor.querySelectorAll(".ref-token-long_text").length).toBe(1);
    });
  });

  // covers: BR-REF-DATA-004
  // 草稿 chip 超过 5 条折叠，输入流内 ReferenceToken 不折叠。
  it("超过 5 条草稿引用时折叠 chip，但输入流令牌不折叠", async () => {
    const { dropTarget, editor } = renderComposer();

    for (let i = 0; i < 6; i += 1) {
      fireEvent.drop(dropTarget, {
        dataTransfer: dataTransfer({ [DOC_REF_MIME]: JSON.stringify({ relPath: `notes/${i}.md` }) }),
      });
    }

    await waitFor(() => {
      expect(editor.querySelectorAll(".ref-token-doc").length).toBe(6);
    });
    expect(screen.queryByText("+1 引用")).not.toBeNull();
  });

  // covers: BR-REF-DATA-004
  // 纯引用消息可发送；发送时带 references，displayParts 保留发送后令牌显示。
  it("纯引用消息可发送，发送时携带 references 与发送后显示 parts", async () => {
    const { dropTarget } = renderComposer();

    fireEvent.drop(dropTarget, {
      dataTransfer: dataTransfer({ [DOC_REF_MIME]: JSON.stringify({ relPath: "notes/a.md" }) }),
    });

    const sendButton = await waitFor(() => {
      const button = screen.getByTitle("发送（Enter）") as HTMLButtonElement;
      expect(button.disabled).toBe(false);
      return button;
    });
    fireEvent.click(sendButton);

    expect(agServiceMock.startTurn).toHaveBeenCalledWith(
      "[引用:notes/a.md]",
      expect.objectContaining({
        references: [{ kind: "doc", relPath: "notes/a.md" }],
        displayParts: [
          expect.objectContaining({
            type: "reference",
            kind: "doc",
            label: "notes/a.md",
            payload: { kind: "doc", relPath: "notes/a.md" },
          }),
          { type: "text", text: " " },
        ],
      }),
    );
  });

  // covers: BR-REF-DATA-003
  // drag session 快路径（REF-REFERENCE-REPAIR-01 F1）：源端 startReferenceDrag 后 drop 命中。
  it("drop 经 drag session 快路径生成引用令牌", async () => {
    const { dropTarget, editor } = renderComposer();
    const dt = dataTransfer({});
    startReferenceDrag(dt, { kind: "doc", relPath: "notes/fast.md" });

    fireEvent.drop(dropTarget, { dataTransfer: dt });

    await waitFor(() => {
      expect(editor.querySelectorAll(".ref-token-doc").length).toBe(1);
    });
  });

  // covers: BR-REF-DATA-003
  // drag session 保底通道：custom MIME 全部不可见时，仅 text/plain marker 仍可解析生成令牌。
  it("drop 在 custom MIME 不可见时经 text/plain marker 保底生成引用令牌", async () => {
    const { dropTarget, editor } = renderComposer();
    const dt = dataTransfer({});
    startReferenceDrag(dt, { kind: "doc", relPath: "notes/fallback.md" });
    // 构造仅 text/plain（marker）可见的 transfer：模拟 WebKit 清洗 custom MIME。
    const fallback = dataTransfer({ "text/plain": dt.getData("text/plain") });

    fireEvent.drop(dropTarget, { dataTransfer: fallback });

    await waitFor(() => {
      expect(editor.querySelectorAll(".ref-token-doc").length).toBe(1);
    });
  });

  // covers: BR-REF-DATA-003
  // 外部文件拖入（F5）：dataTransfer.files 非空生成 file 引用令牌，仅元数据不读内容。
  it("外部文件拖入生成 file 引用令牌", async () => {
    const { dropTarget, editor } = renderComposer();
    const dt = dataTransfer({ "text/uri-list": "file:///tmp/report.pdf" });
    Object.defineProperty(dt, "files", {
      value: [{ name: "report.pdf", size: 2048 }],
    });

    fireEvent.drop(dropTarget, { dataTransfer: dt });

    await waitFor(() => {
      expect(editor.querySelectorAll(".ref-token-file").length).toBe(1);
    });
    expect(screen.getAllByText("文件 report.pdf").length).toBeGreaterThanOrEqual(1);
  });

  it("切换会话时输入框草稿和引用跟随会话恢复", async () => {
    const first = agStore.getSnapshot().activeSessionId;
    const { dropTarget, editor } = renderComposer();

    fireEvent.drop(dropTarget, {
      dataTransfer: dataTransfer({ [DOC_REF_MIME]: JSON.stringify({ relPath: "notes/a.md" }) }),
    });

    await waitFor(() => {
      expect(editor.textContent).toContain("notes/a.md");
    });

    let second = "";
    act(() => {
      second = agStore.newSession();
    });

    await waitFor(() => {
      expect(editor.textContent).not.toContain("notes/a.md");
    });
    expect(screen.queryByLabelText("草稿引用")).toBeNull();

    fireEvent.drop(dropTarget, {
      dataTransfer: dataTransfer({ [DOC_REF_MIME]: JSON.stringify({ relPath: "notes/b.md" }) }),
    });

    await waitFor(() => {
      expect(editor.textContent).toContain("notes/b.md");
    });

    act(() => {
      agStore.switchSession(first);
    });

    await waitFor(() => {
      expect(editor.textContent).toContain("notes/a.md");
      expect(editor.textContent).not.toContain("notes/b.md");
    });

    act(() => {
      agStore.switchSession(second);
    });

    await waitFor(() => {
      expect(editor.textContent).toContain("notes/b.md");
      expect(editor.textContent).not.toContain("notes/a.md");
    });
  });
});

// Migrated intact from AgentPanel.test.tsx.
describe("AgentPanel migrated suite", () => {
  function dragData() {
    const values = new Map<string, string>();
    return {
      values,
      dataTransfer: {
        getData: vi.fn((type: string) => values.get(type) ?? ""),
        setData: vi.fn((type: string, value: string) => values.set(type, value)),
      },
    };
  }

  beforeEach(() => {
    vi.clearAllMocks();
    agStore.reset();
    const id = agStore.newSession();
    agStore.pushMessage(id, { id: "u1", role: "user", content: "引用这段会话" });
    agStore.pushMessage(id, { id: "a1", role: "assistant", content: "可以" });
  });

  afterEach(() => {
    vi.restoreAllMocks();
    cleanup();
  });

  describe("AgentPanel · 会话标签引用", () => {
    it("拖拽会话标签时写入 session ReferencePayload", () => {
      render(<AgentPanel />);
      const tab = screen.getByRole("tab", { selected: true });
      const { values, dataTransfer } = dragData();

      fireEvent.dragStart(tab, { dataTransfer });

      const payload = JSON.parse(values.get(REFERENCE_MIME) ?? "{}");
      expect(payload.kind).toBe("session");
      expect(payload.title).toBe("引用这段会话");
      expect(payload.messages).toEqual([
        { role: "user", content: "引用这段会话" },
        { role: "assistant", content: "可以" },
      ]);
    });

    it("历史菜单会话行确认后删除对应代理编辑会话", async () => {
      vi.spyOn(window, "confirm").mockReturnValue(true);
      vi.spyOn(window, "alert").mockImplementation(() => undefined);
      vi.mocked(deleteAgentSession).mockResolvedValue();

      render(<AgentPanel />);
      fireEvent.click(screen.getByTitle("历史会话"));
      fireEvent.click(screen.getByRole("button", { name: "删除会话 引用这段会话" }));

      expect(window.confirm).toHaveBeenCalledWith(expect.stringContaining("确认删除会话"));
      await waitFor(() => expect(deleteAgentSession).toHaveBeenCalledTimes(1));
      expect(deleteAgentSession).toHaveBeenCalledWith(agStore.getSnapshot().activeSessionId);
    });

    it("历史菜单删除会话取消确认时不调用删除服务", () => {
      vi.spyOn(window, "confirm").mockReturnValue(false);
      vi.mocked(deleteAgentSession).mockResolvedValue();

      render(<AgentPanel />);
      fireEvent.click(screen.getByTitle("历史会话"));
      fireEvent.click(screen.getByRole("button", { name: "删除会话 引用这段会话" }));

      expect(deleteAgentSession).not.toHaveBeenCalled();
    });

    it("历史菜单打开后点击外部区域自动关闭", async () => {
      render(<AgentPanel />);
      fireEvent.click(screen.getByTitle("历史会话"));
      expect(screen.getByText("历史保存在本地 workspace.db；未打开标签点击后加载内容")).toBeTruthy();

      fireEvent.mouseDown(document.body);

      await waitFor(() =>
        expect(screen.queryByText("历史保存在本地 workspace.db；未打开标签点击后加载内容")).toBeNull(),
      );
    });
  });
});

// Migrated intact from AutocompleteOverlay.test.tsx.
describe("AutocompleteOverlay migrated suite", () => {
  const autocompleteTarget: AssistTarget = {
    docPath: "notes/a.md",
    loadSeq: 1,
    blockId: "b1",
    start: 2,
    end: 2,
    text: "",
    prefix: "前文",
    before: "前",
    after: "后",
    mode: "cursor",
  };

  afterEach(() => {
    cleanup();
    assistStore.resetForTest();
  });

  function isKeyboardScope(event: KeyboardEvent): boolean {
    const host = document.querySelector(".editor-host");
    const targetNode = event.target instanceof Node ? event.target : null;
    const active = document.activeElement;
    return Boolean(host && ((targetNode && host.contains(targetNode)) || (active && host.contains(active))));
  }

  function renderOverlay(onPick = vi.fn()) {
    const requestId = assistStore.openAutocomplete({ x: 1, y: 2 }, autocompleteTarget);
    assistStore.setSuggestions(requestId, ["候选一", "候选二", "候选三"]);
    const onDismiss = vi.fn();
    render(
      <>
        <button data-testid="outside">outside</button>
        <div className="editor-host">
          <button data-testid="inside">inside</button>
          <AutocompleteOverlay onPick={onPick} onDismiss={onDismiss} isKeyboardScope={isKeyboardScope} />
        </div>
      </>,
    );
    return { onPick, onDismiss };
  }

  describe("AutocompleteOverlay keyboard scope", () => {
    // covers: BR-AG-STATE-003
    // window 级监听只能在编辑器/overlay 焦点范围内采纳，避免输入框外误触 Tab/数字键。
    it("ignores numeric picks outside editor keyboard scope", () => {
      const onPick = vi.fn();
      renderOverlay(onPick);

      screen.getByTestId("outside").focus();
      fireEvent.keyDown(window, { key: "1" });
      expect(onPick).not.toHaveBeenCalled();

      screen.getByTestId("inside").focus();
      fireEvent.keyDown(window, { key: "2" });
      expect(onPick).toHaveBeenCalledWith("候选二");
    });

    // covers: BR-AG-STATE-003
    // 输入法 composition 期间不得用数字键/Tab 采纳候选。
    it("does not pick while composing text", () => {
      const onPick = vi.fn();
      renderOverlay(onPick);
      screen.getByTestId("inside").focus();

      const event = new KeyboardEvent("keydown", { key: "1", bubbles: true });
      Object.defineProperty(event, "isComposing", { value: true });
      window.dispatchEvent(event);

      expect(onPick).not.toHaveBeenCalled();
    });

    // covers: BR-AG-STATE-003
    // 候选失败或不足时不应把刚触发的浮层瞬间关闭；保留轻量空态，避免用户体感为快捷键无效。
    it("keeps a visible empty state when generation returns no suggestions", () => {
      const requestId = assistStore.openAutocomplete({ x: 1, y: 2 }, autocompleteTarget);
      assistStore.setAutocompleteFailure(requestId, "parse_underflow");

      render(<AutocompleteOverlay onPick={vi.fn()} onDismiss={vi.fn()} />);

      const empty = screen.getByText("暂无可用续写");
      expect(empty).toBeTruthy();
      expect(empty.closest(".assist-ac")?.getAttribute("data-failure-reason")).toBe("parse_underflow");
    });
  });
});

// Migrated intact from MessageList.test.tsx.
describe("MessageList migrated suite", () => {
  // covers: BR-AG-UI-001
  // covers: BR-SYS-STATE-007
  it("effect inspection is read-only and keeps late feedback in its original session", async () => {
    const original = agStore.newSession();
    agStore.pushMessage(original, { id: "query-original", role: "user", content: "修改", turnId: "old-turn", turnStatus: "draft" });
    let resolve!: (value: string) => void;
    agServiceMock.inspectTurnEffects.mockReturnValue(new Promise<string>(done => { resolve = done; }));
    const { rerender } = render(<MessageList />);
    fireEvent.click(screen.getByText("核对执行效果"));
    expect(agServiceMock.inspectTurnEffects).toHaveBeenCalledWith(original, "old-turn");
    expect((screen.getByText("核对执行效果") as HTMLButtonElement).disabled).toBe(true);
    const other = agStore.newSession(); rerender(<MessageList />);
    await act(async () => { resolve("已生成修改建议；不代表接受或保存。 pending_diff=real-diff"); });
    expect(screen.queryByText(/pending_diff=real-diff/)).toBeNull();
    agStore.switchSession(original); rerender(<MessageList />);
    expect(screen.getByRole("status").textContent).toContain("pending_diff=real-diff");
    expect(agServiceMock.startTurn).not.toHaveBeenCalled();
    expect(agStore.getSnapshot().sessions.find(session => session.id === other)?.messages).toHaveLength(0);
  });
  function visibleFlow(container: HTMLElement): string[] {
    return Array.from(container.querySelectorAll(".msg-body, .diffcard-title")).map((node) =>
      node.textContent?.trim() ?? "",
    );
  }

  beforeEach(() => {
    vi.clearAllMocks();
    agStore.reset();
    deStore.reset();
  });

  describe("MessageList · DiffCard 回合归属渲染", () => {
    // covers: BR-DE-STATE-001, BR-AG-DATA-003
    it("DiffCard 插入各自 assistant 消息后，而不是沉底汇总", () => {
      const sessionId = agStore.newSession();
      agStore.pushMessage(sessionId, { id: "u1", role: "user", content: "第一轮" });
      agStore.pushMessage(sessionId, { id: "a1", role: "assistant", content: "第一轮 AI" });
      agStore.pushMessage(sessionId, { id: "u2", role: "user", content: "第二轮" });
      agStore.pushMessage(sessionId, { id: "a2", role: "assistant", content: "第二轮 AI" });
      deStore.addDiff(diff({ diffId: "d2", sessionId, anchorMessageId: "a2", title: "第二轮修改", createdAt: 2 }));
      deStore.addDiff(diff({ diffId: "d1", sessionId, anchorMessageId: "a1", title: "第一轮修改", createdAt: 1 }));

      const { container } = render(<MessageList />);

      expect(visibleFlow(container)).toEqual([
        "第一轮",
        "第一轮 AI",
        "第一轮修改",
        "第二轮",
        "第二轮 AI",
        "第二轮修改",
      ]);
    });

    it("不显示其他会话的 DiffCard，未归属卡进入当前消息流 fallback 区", () => {
      const current = agStore.newSession();
      agStore.pushMessage(current, { id: "u1", role: "user", content: "当前会话" });
      agStore.pushMessage(current, { id: "a1", role: "assistant", content: "当前 AI" });
      const other = agStore.newSession();
      agStore.pushMessage(other, { id: "u2", role: "user", content: "其他会话" });
      agStore.pushMessage(other, { id: "a2", role: "assistant", content: "其他 AI" });
      agStore.switchSession(current);
      deStore.addDiff(diff({ diffId: "current", sessionId: current, title: "未归属当前修改", createdAt: 1 }));
      deStore.addDiff(diff({ diffId: "other", sessionId: other, anchorMessageId: "a2", title: "其他会话修改", createdAt: 2 }));

      render(<MessageList />);

      expect(screen.getByText("未归属修改建议 · 1")).toBeTruthy();
      expect(screen.getByText("未归属当前修改")).toBeTruthy();
      expect(screen.queryByText("其他会话修改")).toBeNull();
    });

    it("不把无会话归属的全局 pending diff 混进当前历史流", () => {
      const current = agStore.newSession();
      agStore.pushMessage(current, { id: "u1", role: "user", content: "当前会话" });
      agStore.pushMessage(current, { id: "a1", role: "assistant", content: "当前 AI" });
      deStore.addDiff(diff({ diffId: "global", sessionId: undefined, title: "全局未归属修改", createdAt: 1 }));

      render(<MessageList />);

      expect(screen.queryByText("全局未归属修改")).toBeNull();
    });

    it("历史工具卡插入对应 assistant 消息后，而不是沉底汇总", () => {
      const sessionId = agStore.newSession();
      agStore.pushMessage(sessionId, { id: "u1", role: "user", content: "读取文档" });
      agStore.pushMessage(sessionId, {
        id: "a1",
        role: "assistant",
        content: "已读取",
        toolCalls: [{ id: "tc1", name: "read_doc", state: "done", detail: "notes/a.md", readonly: true, anchorMessageId: "a1" }],
      });
      agStore.pushMessage(sessionId, { id: "u2", role: "user", content: "第二轮" });

      const { container } = render(<MessageList />);

      expect(visibleFlow(container)).toEqual(["读取文档", "已读取", "第二轮"]);
      expect(screen.getByText("read_doc")).toBeTruthy();
      expect(screen.queryByText("未归属工具调用")).toBeNull();
    });

    it("活动回合不显示未完成，停止后仅显示一份诊断并允许重试", () => {
      const sessionId = agStore.newSession();
      agStore.pushMessage(sessionId, { id: "u1", role: "user", content: "你好", turnId: "t1", turnStatus: "draft" });
      agStore.setActiveTurn(sessionId, "t1");
      const { rerender } = render(<MessageList />);
      expect(screen.queryByText("重试")).toBeNull();
      agStore.setTurnStatus(sessionId, "t1", "draft", { code: "DB_ERROR", message: "数据库暂不可用", retriable: true, diagnostic: { version: 1, observedAt: "frontend_invoke", category: "unknown", rootCauseStatus: "unknown" } });
      agStore.clearActiveTurn("t1");
      rerender(<MessageList />);
      expect(screen.getAllByText(/数据库暂不可用/)).toHaveLength(1);
      expect(screen.getByText(/诊断阶段：frontend_invoke/).textContent).toContain("根因未确认");
      expect(screen.getAllByText("重试")).toHaveLength(1);
    });

    it("draft 重试保留身份和引用，并展示被阻断的真实原因", async () => {
      const sessionId = agStore.newSession();
      agStore.pushMessage(sessionId, {
        id: "u1",
        role: "user",
        content: "请参考  修改",
        turnId: "retry-source",
        turnStatus: "draft",
        parts: [
          { type: "text", text: "请参考 " },
          {
            type: "reference",
            refId: "r1",
            kind: "doc",
            label: "notes/a.md",
            payload: { kind: "doc", relPath: "notes/a.md" },
          },
          { type: "text", text: " 修改" },
        ],
      });

      render(<MessageList />);
      vi.mocked(startTurn).mockRejectedValueOnce(new Error("AG_RETRY_EFFECT_UNVERIFIED：请先核对已有结果"));
      fireEvent.click(screen.getByText("重试"));

      expect(vi.mocked(startTurn)).toHaveBeenCalledWith(
        "请参考  修改",
        expect.objectContaining({
          references: [{ kind: "doc", relPath: "notes/a.md" }],
          retryOf: { sessionId, turnId: "retry-source" },
        }),
      );
      expect((await screen.findByRole("status")).textContent).toContain("AG_RETRY_EFFECT_UNVERIFIED");
      expect((screen.getByText("重试") as HTMLButtonElement).disabled).toBe(false);
    });
  });
});

// AG-MSG-RENDER-01 · BR-AG-UI-001：assistant 消息 GFM Markdown 渲染与安全。
describe("MessageMarkdown · GFM 渲染与安全", () => {
  it("渲染标题、加粗、代码块、表格", () => {
    const content = "# 标题\n\n**加粗**\n\n```js\nconst a = 1;\n```\n\n| 列 | 值 |\n|---|---|\n| x | 1 |";
    const { container } = render(<MessageMarkdown content={content} />);
    expect(container.querySelector("h1")?.textContent).toBe("标题");
    expect(container.querySelector("strong")?.textContent).toBe("加粗");
    expect(container.querySelector("pre code")).toBeTruthy();
    expect(container.querySelector("table")).toBeTruthy();
  });

  it("sanitize：script 与事件属性不落地", () => {
    const content = '<script>alert(1)</script>\n\n<img src="x" onerror="alert(2)">\n\n普通文本';
    const { container } = render(<MessageMarkdown content={content} />);
    expect(container.querySelector("script")).toBeNull();
    expect(container.querySelector("img")).toBeNull();
    expect(container.textContent).toContain("普通文本");
  });

  it("链接阻止 WebView 内导航", () => {
    const { container } = render(<MessageMarkdown content="[链接](https://example.com)" />);
    const anchor = container.querySelector("a");
    expect(anchor?.getAttribute("href")).toBe("https://example.com");
    const event = new MouseEvent("click", { bubbles: true, cancelable: true });
    anchor?.dispatchEvent(event);
    expect(event.defaultPrevented).toBe(true);
  });

  it("mermaid 围栏块降级为普通代码块", () => {
    const { container } = render(<MessageMarkdown content={"```mermaid\nflowchart LR\nA-->B\n```"} />);
    expect(container.querySelector("pre code")).toBeTruthy();
    expect(container.textContent).toContain("flowchart");
  });
});

describe("ToolCallCard · 回执与可操作性一致", () => {
  it("执行前拒绝明确显示未执行，且不提供没有处理逻辑的按钮", () => {
    const tool: ToolCallView = {
      id: "call-1", name: "workspace_glob", state: "failed", detail: "请求受阻，工具未执行",
      executionReceipt: {
        version: "tool-call-receipt-v1", workspaceId: "ws", sessionId: "s", turnId: "t",
        toolCallId: "call-1", toolName: "workspace_glob", capabilityId: "workspace_read",
        grantId: "grant", attempt: 1, executionStatus: "rejected", completionStatus: "blocked",
        effectStatus: "not_started", references: [],
        failure: { code: "AG_MODEL_CONTROL_CONFLICT", retriable: true, reauthRequired: false },
        evidenceIds: [], nextAction: "retry_explicitly",
      },
    };
    render(<ToolCallCard tool={tool} />);
    expect(screen.getByText("受阻，未执行")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "重试" })).toBeNull();
    expect(screen.queryByRole("button", { name: "跳过" })).toBeNull();
    cleanup();
    render(<ToolCallCard tool={{ id: "restored", name: "workspace_glob", state: "failed", detail: "请求受阻，工具未执行", readonly: true }} />);
    expect(screen.getByText("未完成")).toBeTruthy();
    expect(screen.getByText("请求受阻，工具未执行")).toBeTruthy();
  });

  it("等待确认卡不伪造授权按钮，实际裁决仍由 DecisionBar 提供", () => {
    render(<ToolCallCard tool={{ id: "call-2", name: "plan_workspace_operation", state: "awaiting_confirmation" }} />);
    expect(screen.getByText("需授权")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "授权" })).toBeNull();
    expect(screen.queryByRole("button", { name: "拒绝" })).toBeNull();
  });
});

// AG-MSG-RENDER-01 · TERM-AG-011：回合内消息片段时序交错渲染与降级。
describe("MessageFlow · 回合内时序交错", () => {
  function assistantMsg(overrides: Partial<ChatMessage> = {}): ChatMessage {
    return { id: "a1", role: "assistant", content: "", ...overrides };
  }

  it("按 segments 顺序交错渲染 text/tool/text/diff", () => {
    const msg = assistantMsg({
      content: "先看文档 然后修改",
      segments: [
        { kind: "text", text: "先看文档" },
        { kind: "tool", toolCallId: "tc1" },
        { kind: "text", text: "然后修改" },
        { kind: "diff", diffId: "d1" },
      ],
    });
    const toolCalls: ToolCallView[] = [
      { id: "tc1", name: "read_doc", state: "done", detail: "a.md", readonly: false, anchorMessageId: "a1" },
    ];
    deStore.reset();
    deStore.addDiff(diff({ diffId: "d1", sessionId: "s1", anchorMessageId: "a1", title: "修改" }));
    const { container } = render(<MessageFlow message={msg} toolCalls={toolCalls} diffs={deStore.getSnapshot().diffs} />);
    const flow = Array.from(container.querySelectorAll(".msg-body, .toolcard-name, .diffcard-title")).map((node) =>
      node.textContent?.trim() ?? "",
    );
    expect(flow).toEqual(["先看文档", "read_doc", "然后修改", "修改"]);
  });

  it("无 segments 时降级：整段正文 + 沉底卡片", () => {
    const msg = assistantMsg({ content: "整段正文" });
    const toolCalls: ToolCallView[] = [
      { id: "tc1", name: "read_doc", state: "done", detail: "a.md", readonly: false, anchorMessageId: "a1" },
    ];
    const { container } = render(<MessageFlow message={msg} toolCalls={toolCalls} diffs={[]} />);
    expect(container.querySelector(".msg-body")?.textContent).toContain("整段正文");
    expect(container.querySelector(".toolcard")).toBeTruthy();
  });

  it("错误消息保持纯文本不渲染 Markdown", () => {
    const msg = assistantMsg({ content: "# 标题", error: true });
    const { container } = render(<MessageFlow message={msg} toolCalls={[]} diffs={[]} />);
    expect(container.querySelector("h1")).toBeNull();
    expect(container.querySelector(".msg-body")?.textContent).toBe("# 标题");
  });

  it("未被子段引用的卡沉底兜底，避免丢失", () => {
    const msg = assistantMsg({
      content: "正文",
      segments: [{ kind: "text", text: "正文" }],
    });
    const toolCalls: ToolCallView[] = [
      { id: "tc1", name: "read_doc", state: "done", readonly: false, anchorMessageId: "a1" },
    ];
    const { container } = render(<MessageFlow message={msg} toolCalls={toolCalls} diffs={[]} />);
    expect(container.querySelector(".toolcard")).toBeTruthy();
  });
});

describe("Agent 当前事实呈现", () => {
  it("等待编辑器时显示静态等待，不显示工作中动画", () => {
    const sessionId = agStore.getSnapshot().activeSessionId;
    agStore.setCurrentAction({
      sessionId,
      turnId: "t-wait",
      action: "previewing",
      label: "等待编辑器应用修改建议",
    });

    const { container } = render(<MessageList />);

    expect(screen.getByText("等待编辑器应用修改建议")).toBeTruthy();
    expect(container.querySelector(".turn-statusbar-waiting")).toBeTruthy();
    expect(container.querySelector(".turn-statusbar-spinner")).toBeNull();
  });

  it("活动读取只在所属会话显示工作动画", () => {
    const origin = agStore.getSnapshot().activeSessionId;
    agStore.setCurrentAction({ sessionId: origin, turnId: "t-read", action: "reading", label: "notes/a.md" });
    const other = agStore.newSession();
    const { container } = render(<MessageList />);
    expect(container.querySelector(".turn-statusbar")).toBeNull();

    act(() => agStore.switchSession(origin));
    expect(screen.getByText("正在读取 notes/a.md")).toBeTruthy();
    expect(container.querySelector(".turn-statusbar-spinner")).toBeTruthy();
    act(() => agStore.switchSession(other));
  });

  it("在对应 assistant 回合展示 ProjectReport 的已读、未读与不确定性", () => {
    const sessionId = agStore.getSnapshot().activeSessionId;
    agStore.pushMessage(sessionId, {
      id: "report-answer",
      role: "assistant",
      content: "检查完成",
      turnId: "t-report",
      turnStatus: "completed",
    });
    agStore.projectProjectReport("t-report", {
      version: "project-report-v2",
      turnId: "t-report",
      evidenceThroughSeq: 8,
      requestedScope: ["notes/a.md", "notes/b.md"],
      readItems: [{ relPath: "notes/a.md", source: "read_doc", coverage: "full_document", evidenceIds: ["e1"] }],
      unreadItems: [{ relPath: "notes/b.md", reason: "read_failed" }],
      findings: [{ statement: "标题需要统一", evidenceIds: ["e1"], uncertainty: "low" }],
      uncertainties: ["notes/b.md 尚未读取"],
    });

    render(<MessageList />);

    expect(screen.getByLabelText("项目报告")).toBeTruthy();
    expect(screen.getByText("已读 1")).toBeTruthy();
    expect(screen.getByText("未读 1")).toBeTruthy();
    expect(screen.getByText("notes/b.md 尚未读取")).toBeTruthy();
    expect(screen.getByText("不确定性：low")).toBeTruthy();
  });
});
