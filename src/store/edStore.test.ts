import { beforeEach, describe, it, expect } from "vitest";
import { edStore } from "./edStore";

beforeEach(() => edStore.reset());

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
    edStore.markSaved("a.md", 42);
    expect(edStore.getSnapshot().tabState).toBe("saved");
    expect(edStore.getSnapshot().baseline).toBe(42);
  });

  it("外部变更/待裁决与 dirty 互斥：markDirty 不覆盖 external_changed / pending_diff", () => {
    edStore.openDoc("a.md");
    edStore.setTabState("a.md", "external_changed");
    edStore.markDirty();
    expect(edStore.getSnapshot().tabState).toBe("external_changed"); // 不被脏态覆盖
    edStore.setTabState("a.md", "pending_diff");
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

// covers: BR-ED-STATE-001
// 多文档会话（PI-FE-27 / PI-B）：每文档会话独立，切换活动会话各自保留未保存编辑/格式/脏态。
describe("edStore · 多文档会话 (PI-FE-27)", () => {
  it("切换活动会话保留各自未保存编辑：updateContent 暂存 + setActivePath 还原", () => {
    edStore.loadDoc("a.md", "A 原文", 1);
    edStore.loadDoc("b.md", "B 原文", 2); // 切到 b（a 仍在会话表）
    edStore.updateContent("a.md", "A 草稿"); // 暂存 a 的未保存编辑
    edStore.markDirty(); // 当前活动是 b → 只脏 b
    expect(edStore.getSnapshot().sessions["a.md"].tabState).toBe("saved"); // a 不被 b 的脏态污染
    edStore.setActivePath("a.md"); // 切回 a
    const s = edStore.getSnapshot();
    expect(s.openPath).toBe("a.md");
    expect(s.loadedContent).toBe("A 草稿"); // 还原 a 的未保存编辑
    expect(s.baseline).toBe(1);
    expect(s.loadSeq).toBeGreaterThan(0); // 切换递增 → 触发编辑器载入
  });

  it("loadDoc 携 editFormat 入会话；派生 editFormat 跟随活动会话", () => {
    edStore.loadDoc("a.md", "文本", 1, null, "text");
    edStore.loadDoc("x.docx", "<p>HTML</p>", 2, null, "html");
    expect(edStore.getSnapshot().editFormat).toBe("html");
    edStore.setActivePath("a.md");
    expect(edStore.getSnapshot().editFormat).toBe("text");
  });

  it("closeSession 销毁会话；关活动会话则置空活动（派生回落默认）", () => {
    edStore.loadDoc("a.md", "A", 1);
    edStore.loadDoc("b.md", "B", 2);
    edStore.closeSession("b.md"); // 关活动 b
    let s = edStore.getSnapshot();
    expect(s.sessions["b.md"]).toBeUndefined();
    expect(s.openPath).toBeNull(); // 活动置空（相邻激活由 tabsStore 决定）
    expect(s.tabState).toBe("normal");
    edStore.setActivePath("a.md"); // tabsStore 切到相邻文档
    s = edStore.getSnapshot();
    expect(s.openPath).toBe("a.md");
    expect(s.loadedContent).toBe("A");
  });

  it("setActivePath 对未打开路径/同路径为空操作（不误建会话、不递增）", () => {
    edStore.loadDoc("a.md", "A", 1);
    const seq = edStore.getSnapshot().loadSeq;
    edStore.setActivePath("nope.md"); // 无会话 → 空操作
    edStore.setActivePath("a.md"); // 同活动 → 空操作
    const s = edStore.getSnapshot();
    expect(s.loadSeq).toBe(seq);
    expect(s.sessions["nope.md"]).toBeUndefined();
  });

  // covers: BR-ED-STATE-001
  // 保存按 path 锁定（PI-FE-29）：模拟异步保存 a 期间切到 b，回写只改 a，不污染 b/活动派生。
  it("setTabState/markSaved 按 path 锁定：保存途中切标签只改目标会话", () => {
    edStore.loadDoc("a.md", "A", 1);
    edStore.loadDoc("b.md", "B", 2); // 活动切到 b
    edStore.setTabState("a.md", "saving"); // 对 a 发起保存（活动是 b）
    expect(edStore.getSnapshot().sessions["a.md"].tabState).toBe("saving");
    expect(edStore.getSnapshot().sessions["b.md"].tabState).toBe("saved"); // b 不受影响
    expect(edStore.getSnapshot().tabState).toBe("saved"); // 活动 b 派生不被 a 污染
    edStore.markSaved("a.md", 99); // a 保存完成
    expect(edStore.getSnapshot().sessions["a.md"]).toMatchObject({ tabState: "saved", baseline: 99 });
    expect(edStore.getSnapshot()).toMatchObject({ openPath: "b.md", baseline: 2 }); // 活动仍 b
  });

  it("会话已关闭时 markSaved/setTabState 为空操作（文件已落盘但无会话可更新，不崩）", () => {
    edStore.loadDoc("a.md", "A", 1);
    edStore.closeSession("a.md");
    expect(() => {
      edStore.markSaved("a.md", 7);
      edStore.setTabState("a.md", "external_changed");
    }).not.toThrow();
    expect(edStore.getSnapshot().sessions["a.md"]).toBeUndefined();
  });
});
