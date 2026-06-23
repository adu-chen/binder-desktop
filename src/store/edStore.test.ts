import { describe, it, expect } from "vitest";
import { edStore } from "./edStore";

// covers: BR-ED-STATE-001
// 编辑会话脏态守则：打开/载入为 saved 基线；编辑转 dirty；落盘 saved；外部变更/待裁决互斥不被 dirty 覆盖。
describe("edStore · SM-ED-01 脏态投影 (BR-ED-STATE-001)", () => {
  it("打开文档为 saved 基线态", () => {
    edStore.openDoc("a.md");
    expect(edStore.getSnapshot().tabState).toBe("saved");
    expect(edStore.getSnapshot().pendingDiffCount).toBe(0);
  });

  it("编辑产生脏态 saved→dirty；落盘 dirty→saved 并刷新基线", () => {
    edStore.openDoc("a.md");
    edStore.markDirty();
    expect(edStore.getSnapshot().tabState).toBe("dirty");
    edStore.markSaved(42);
    expect(edStore.getSnapshot().tabState).toBe("saved");
    expect(edStore.getSnapshot().baseline).toBe(42);
  });

  it("外部变更/待裁决与 dirty 互斥：markDirty 不覆盖 external_changed / pending_diff", () => {
    edStore.openDoc("a.md");
    edStore.setTabState("external_changed");
    edStore.markDirty();
    expect(edStore.getSnapshot().tabState).toBe("external_changed"); // 不被脏态覆盖
    edStore.setTabState("pending_diff");
    edStore.markDirty();
    expect(edStore.getSnapshot().tabState).toBe("pending_diff");
  });

  it("载入真实文档：saved 态 + 基线 + loadSeq 递增（区分同路径重载）", () => {
    const before = edStore.getSnapshot().loadSeq;
    edStore.loadDoc("b.md", "# 内容", 7);
    const s = edStore.getSnapshot();
    expect(s.tabState).toBe("saved");
    expect(s.loadedContent).toBe("# 内容");
    expect(s.baseline).toBe(7);
    expect(s.loadSeq).toBe(before + 1);
  });
});
