/** @vitest-environment jsdom */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { agStore } from "../../store/agStore";
import { deStore, type DiffView } from "../../store/deStore";
import { startTurn } from "../../services/agService";
import { MessageList } from "./MessageList";

vi.mock("../../services/deService", () => ({
  acceptDiff: vi.fn(),
  rejectDiff: vi.fn(),
  generateSampleDiff: vi.fn(),
}));

vi.mock("../../services/agService", () => ({
  startTurn: vi.fn(),
}));

vi.mock("../../services/edLocate", () => ({
  revealBlock: vi.fn(() => "located"),
}));

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

afterEach(cleanup);

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
