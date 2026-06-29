import { beforeEach, describe, expect, it } from "vitest";
import { deStore, toGreenReview, EXPIRED_REASON_TEXT, type DiffView } from "./deStore";

const mk = (id: string, state: DiffView["state"]): DiffView => ({
  diffId: id,
  blockId: "",
  title: "t",
  before: "a",
  after: "b",
  state,
});

beforeEach(() => deStore.reset());

describe("deStore 状态投影（SM-DE-01）", () => {
  // covers: BR-DE-STATE-001
  it("addDiff 追加；reset 清空", () => {
    deStore.addDiff(mk("d1", "pending"));
    expect(deStore.getSnapshot().diffs.map((d) => d.diffId)).toEqual(["d1"]);
    deStore.reset();
    expect(deStore.getSnapshot().diffs).toEqual([]);
  });

  // covers: BR-DE-STATE-001
  // 后端 SM-DE-01 广播投影：Rust DiffState → 视觉态；三终态不被本地呈现/再次广播覆盖。
  it("applyBackendState 投影后端态：Pending→pending，Accepted→accepted；终态不回退", () => {
    deStore.addDiff(mk("d", "pending"));
    deStore.applyBackendState("d", "Accepted", true);
    expect(deStore.getSnapshot().diffs[0].state).toBe("accepted");
    // 已终态：再投影 Pending 不回退（与后端 transition 守护一致）
    deStore.applyBackendState("d", "Pending", false);
    expect(deStore.getSnapshot().diffs[0].state).toBe("accepted");
  });

  // covers: BR-DE-STATE-001
  it("三终态互斥不可逆：本地呈现 setLocalView 不覆盖后端终态", () => {
    for (const terminal of ["accepted", "rejected", "expired"] as const) {
      deStore.reset();
      deStore.addDiff(mk("d", terminal));
      deStore.setLocalView("d", "applying");
      expect(deStore.getSnapshot().diffs[0].state).toBe(terminal);
    }
  });

  // covers: BR-WS-STATE-001
  // SM-WS-02 落盘机广播投影：Applying→applying 过渡；Applied→accepted；Discarded 不投影（终态归 SM-DE-01）。
  it("applyWsApplyState 投影落盘机：Applying→applying，Applied→accepted，Discarded 不覆盖", () => {
    deStore.addDiff(mk("d", "pending"));
    deStore.applyWsApplyState("d", "Applying");
    expect(deStore.getSnapshot().diffs[0].state).toBe("applying");
    deStore.applyWsApplyState("d", "Applied");
    expect(deStore.getSnapshot().diffs[0].state).toBe("accepted");
    // Discarded 不投影（不回退已 accepted）
    deStore.reset();
    deStore.addDiff(mk("d2", "pending"));
    deStore.applyWsApplyState("d2", "Discarded");
    expect(deStore.getSnapshot().diffs[0].state).toBe("pending");
  });

  // covers: BR-DE-STATE-001
  it("setLocalView 推进非终态过渡呈现；attachMeta 回填元数据不改状态", () => {
    deStore.addDiff(mk("d", "pending"));
    deStore.setLocalView("d", "applying");
    expect(deStore.getSnapshot().diffs[0].state).toBe("applying");
    deStore.attachMeta("d", { timelineSeq: 7 });
    expect(deStore.getSnapshot().diffs[0]).toMatchObject({ state: "applying", timelineSeq: 7 });
  });

  it("toGreenReview 映射：rejected→null，过渡态→applying，pending/conflict/error→pending", () => {
    expect(toGreenReview("rejected")).toBeNull();
    expect(toGreenReview("validating")).toBe("applying");
    expect(toGreenReview("applying")).toBe("applying");
    expect(toGreenReview("accepted")).toBe("accepted");
    expect(toGreenReview("expired")).toBe("expired");
    expect(toGreenReview("conflict")).toBe("pending");
    expect(toGreenReview("error")).toBe("pending");
  });

  it("expired 5 种细分原因均有引导文案", () => {
    const reasons = [
      "document_revision_advanced",
      "block_resolve_failed",
      "overlapping_range",
      "external_file_changed",
      "apply_replace_failed",
    ] as const;
    for (const r of reasons) expect(EXPIRED_REASON_TEXT[r]).toBeTruthy();
  });
});
