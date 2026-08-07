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
import { MessageList } from "./MessageList";

const agServiceMock = vi.hoisted(() => ({
  startTurn: vi.fn(),
  uploadByok: vi.fn(),
  deleteAgentSession: vi.fn(),
  newAgentSession: vi.fn(() => "s-new"),
  switchAgentSession: vi.fn(),
  closeAgentSession: vi.fn(),
  setAgentMode: vi.fn(),
}));

vi.mock("../../services/agService", () => ({
  startTurn: agServiceMock.startTurn,
  uploadByok: agServiceMock.uploadByok,
  deleteAgentSession: agServiceMock.deleteAgentSession,
  newAgentSession: agServiceMock.newAgentSession,
  switchAgentSession: agServiceMock.switchAgentSession,
  closeAgentSession: agServiceMock.closeAgentSession,
  setAgentMode: agServiceMock.setAgentMode,
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
  agServiceMock.startTurn.mockReset();
  agServiceMock.uploadByok.mockReset();
  agStore.reset();
  agStore.applyTurnState("test-turn", "Completed");
  wsStore.reset();
});

afterEach(cleanup);

describe("Composer · 引用令牌", () => {
  // covers: BR-REF-DATA-004
  // SelectionRef 粘贴必须生成输入流内 ReferenceToken，并同步显示草稿引用。
  it("粘贴编辑器 SelectionRef 时在输入流和草稿区同时生成引用", async () => {
    const { input, editor } = renderComposer();
    const ref = selectionRef();

    fireEvent.paste(input, {
      clipboardData: dataTransfer({ [SELECTION_REF_MIME]: JSON.stringify(ref) }),
    });

    await waitFor(() => {
      expect(editor.querySelectorAll(".ref-token").length).toBe(1);
    });
    expect(screen.queryByLabelText("草稿引用")).not.toBeNull();
    expect(screen.getAllByText("选区 a.md#abcdef[2-5]").length).toBeGreaterThanOrEqual(1);
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

    it("draft 重试保留引用 payload", () => {
      const sessionId = agStore.newSession();
      agStore.pushMessage(sessionId, {
        id: "u1",
        role: "user",
        content: "请参考  修改",
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
      screen.getByText("重试").click();

      expect(vi.mocked(startTurn)).toHaveBeenCalledWith(
        "请参考  修改",
        expect.objectContaining({
          references: [{ kind: "doc", relPath: "notes/a.md" }],
        }),
      );
    });
  });
});
