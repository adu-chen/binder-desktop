import { beforeEach, describe, it, expect } from "vitest";
import { edStore } from "./edStore";

beforeEach(() => edStore.reset());

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
    edStore.markSaved("a.md", 42);
    expect(edStore.getSnapshot().tabState).toBe("saved");
    expect(edStore.getSnapshot().baseline).toBe(42);
  });

  it("SM-WS-01 广播投影：applyWsSaveState 驱动 saving/saved；setBaseline 仅刷基线", () => {
    edStore.openDoc("a.md");
    edStore.markDirty();
    edStore.applyWsSaveState("a.md", "Saving");
    expect(edStore.getSnapshot().tabState).toBe("saving");
    edStore.setBaseline("a.md", 77);
    expect(edStore.getSnapshot()).toMatchObject({ baseline: 77, tabState: "saving" });
    edStore.applyWsSaveState("a.md", "Saved");
    expect(edStore.getSnapshot().tabState).toBe("saved");
  });

  it("markDirty 不覆盖 pending_diff", () => {
    edStore.openDoc("a.md");
    edStore.setTabState("a.md", "pending_diff");
    edStore.markDirty();
    expect(edStore.getSnapshot().tabState).toBe("pending_diff");
  });

  it("载入真实文档：saved 态 + 基线 + loadSeq 递增", () => {
    const before = edStore.getSnapshot().loadSeq;
    edStore.loadDoc("b.md", "# 内容", 7);
    const s = edStore.getSnapshot();
    expect(s.tabState).toBe("saved");
    expect(s.loadedContent).toBe("# 内容");
    expect(s.baseline).toBe(7);
    expect(s.loadSeq).toBe(before + 1);
  });
});

describe("edStore · 多文档会话 (PI-FE-27)", () => {
  it("切换活动会话保留各自未保存编辑", () => {
    edStore.loadDoc("a.md", "A 原文", 1);
    edStore.loadDoc("b.md", "B 原文", 2);
    edStore.updateContent("a.md", "A 草稿");
    edStore.markDirty();
    expect(edStore.getSnapshot().sessions["a.md"].tabState).toBe("saved");
    edStore.setActivePath("a.md");
    const s = edStore.getSnapshot();
    expect(s.openPath).toBe("a.md");
    expect(s.loadedContent).toBe("A 草稿");
    expect(s.baseline).toBe(1);
    expect(s.loadSeq).toBeGreaterThan(0);
  });

  it("loadDoc 携 editFormat 入会话；派生 editFormat 跟随活动会话", () => {
    edStore.loadDoc("a.md", "文本", 1, null, "text");
    edStore.loadDoc("x.docx", "<p>HTML</p>", 2, null, "html");
    expect(edStore.getSnapshot().editFormat).toBe("html");
    edStore.setActivePath("a.md");
    expect(edStore.getSnapshot().editFormat).toBe("text");
  });

  it("closeSession 销毁会话；关活动会话则置空活动", () => {
    edStore.loadDoc("a.md", "A", 1);
    edStore.loadDoc("b.md", "B", 2);
    edStore.closeSession("b.md");
    let s = edStore.getSnapshot();
    expect(s.sessions["b.md"]).toBeUndefined();
    expect(s.openPath).toBeNull();
    expect(s.tabState).toBe("normal");
    edStore.setActivePath("a.md");
    s = edStore.getSnapshot();
    expect(s.openPath).toBe("a.md");
    expect(s.loadedContent).toBe("A");
  });

  it("setActivePath 对未打开路径/同路径为空操作", () => {
    edStore.loadDoc("a.md", "A", 1);
    const seq = edStore.getSnapshot().loadSeq;
    edStore.setActivePath("nope.md");
    edStore.setActivePath("a.md");
    const s = edStore.getSnapshot();
    expect(s.loadSeq).toBe(seq);
    expect(s.sessions["nope.md"]).toBeUndefined();
  });

  it("setTabState/markSaved 按 path 锁定（PI-FE-29）", () => {
    edStore.loadDoc("a.md", "A", 1);
    edStore.loadDoc("b.md", "B", 2);
    edStore.setTabState("a.md", "saving");
    expect(edStore.getSnapshot().sessions["a.md"].tabState).toBe("saving");
    expect(edStore.getSnapshot().sessions["b.md"].tabState).toBe("saved");
    expect(edStore.getSnapshot().tabState).toBe("saved");
    edStore.markSaved("a.md", 99);
    expect(edStore.getSnapshot().sessions["a.md"]).toMatchObject({ tabState: "saved", baseline: 99 });
    expect(edStore.getSnapshot()).toMatchObject({ openPath: "b.md", baseline: 2 });
  });

  it("会话已关闭时 markSaved/setTabState 为空操作", () => {
    edStore.loadDoc("a.md", "A", 1);
    edStore.closeSession("a.md");
    expect(() => {
      edStore.markSaved("a.md", 7);
      edStore.setTabState("a.md", "dirty");
    }).not.toThrow();
    expect(edStore.getSnapshot().sessions["a.md"]).toBeUndefined();
  });
});
