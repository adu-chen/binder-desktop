/** @vitest-environment jsdom */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { agStore } from "../../store/agStore";
import { REFERENCE_MIME } from "../../services/referencePayload";
import { AgentPanel } from "./AgentPanel";
import { deleteAgentSession } from "../../services/agService";

vi.mock("../../services/agService", () => ({
  startTurn: vi.fn(),
  uploadByok: vi.fn(),
  deleteAgentSession: vi.fn(),
  newAgentSession: vi.fn(() => "s-new"),
  switchAgentSession: vi.fn(),
  closeAgentSession: vi.fn(),
  setAgentMode: vi.fn(),
}));

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
