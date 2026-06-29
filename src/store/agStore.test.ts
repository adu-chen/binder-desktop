import { beforeEach, describe, expect, it } from "vitest";
import { agStore } from "./agStore";

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
