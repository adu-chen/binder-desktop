import { beforeEach, describe, expect, it } from "vitest";
import { activeSession, agStore } from "./agStore";

beforeEach(() => {
  // 复位 sending（无 reset API；经投影回到静止态）。
  agStore.applyTurnState("t", "Completed");
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

  it("setMode 作用于活动会话（编辑/构建互不影响其他会话）", () => {
    const a = agStore.newSession();
    agStore.setMode("build");
    const b = agStore.newSession();
    const snap = agStore.getSnapshot();
    expect(snap.sessions.find((s) => s.id === a)!.mode).toBe("build");
    expect(snap.sessions.find((s) => s.id === b)!.mode).toBe("edit");
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
});
