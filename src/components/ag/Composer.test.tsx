/** @vitest-environment jsdom */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { agStore } from "../../store/agStore";
import { wsStore } from "../../store/wsStore";
import { DOC_REF_MIME, SELECTION_REF_MIME, type SelectionRef } from "../../services/edLocate";
import { LONG_TEXT_REF_THRESHOLD, REFERENCE_MIME, type ReferencePayload } from "../../services/referencePayload";
import { Composer } from "./Composer";

const agServiceMock = vi.hoisted(() => ({
  startTurn: vi.fn(),
  uploadByok: vi.fn(),
}));

vi.mock("../../services/agService", () => ({
  startTurn: agServiceMock.startTurn,
  uploadByok: agServiceMock.uploadByok,
}));

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
