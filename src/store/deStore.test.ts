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
  it("三终态互斥不可逆：accepted/rejected/expired 不被 setDiffState 覆盖", () => {
    for (const terminal of ["accepted", "rejected", "expired"] as const) {
      deStore.reset();
      deStore.addDiff(mk("d", terminal));
      deStore.setDiffState("d", "pending");
      expect(deStore.getSnapshot().diffs[0].state).toBe(terminal);
    }
  });

  // covers: BR-DE-STATE-001
  it("非终态可推进，并能附带 patch（如时间轴号）", () => {
    deStore.addDiff(mk("d", "pending"));
    deStore.setDiffState("d", "applying");
    expect(deStore.getSnapshot().diffs[0].state).toBe("applying");
    deStore.setDiffState("d", "accepted", { timelineSeq: 7 });
    expect(deStore.getSnapshot().diffs[0]).toMatchObject({ state: "accepted", timelineSeq: 7 });
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
