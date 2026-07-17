/** @vitest-environment jsdom */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { agStore } from "../../store/agStore";
import { REFERENCE_MIME } from "../../services/referencePayload";
import { AgentPanel } from "./AgentPanel";

vi.mock("../../services/agService", () => ({
  startTurn: vi.fn(),
  uploadByok: vi.fn(),
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
  agStore.applyTurnState("test-turn", "Completed");
  const id = agStore.newSession();
  agStore.pushMessage(id, { id: "u1", role: "user", content: "引用这段会话" });
  agStore.pushMessage(id, { id: "a1", role: "assistant", content: "可以" });
});

afterEach(cleanup);

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
});
