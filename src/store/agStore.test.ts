import { beforeEach, describe, expect, it } from "vitest";
import { activeSession, agStore, emptyComposerDraft } from "./agStore";

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
