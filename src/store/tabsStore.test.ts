import { beforeEach, describe, expect, it } from "vitest";
import { tabsStore } from "./tabsStore";

beforeEach(() => tabsStore.reset());

describe("tabsStore 标签模型（视图层）", () => {
  it("openDoc 单文档标签：复用同一 doc 标签、更新路径", () => {
    tabsStore.openDoc("a.md");
    tabsStore.openDoc("b.md");
    const { tabs } = tabsStore.getSnapshot();
    expect(tabs.filter((t) => t.kind === "doc")).toHaveLength(1);
    expect(tabs[0].path).toBe("b.md");
  });

  it("openPreview 多预览共存，按 path 去重 + 激活", () => {
    tabsStore.openPreview("x.png");
    tabsStore.openPreview("y.mp4");
    tabsStore.openPreview("x.png"); // 重复 → 不新增，激活已有
    const s = tabsStore.getSnapshot();
    expect(s.tabs.filter((t) => t.kind === "preview")).toHaveLength(2);
    expect(s.tabs.find((t) => t.id === s.activeTabId)?.path).toBe("x.png");
  });

  it("文档 + 预览同列共存", () => {
    tabsStore.openDoc("a.md");
    tabsStore.openPreview("x.png");
    expect(tabsStore.getSnapshot().tabs.map((t) => t.kind)).toEqual(["doc", "preview"]);
  });

  it("close 激活相邻；关闭后空则 activeTabId=null", () => {
    tabsStore.openPreview("x.png");
    tabsStore.openPreview("y.png");
    const { tabs } = tabsStore.getSnapshot();
    tabsStore.close(tabs[1].id); // 关闭激活的 y → 激活相邻
    expect(tabsStore.getSnapshot().activeTabId).toBe(tabs[0].id);
    tabsStore.close(tabs[0].id);
    expect(tabsStore.getSnapshot()).toMatchObject({ tabs: [], activeTabId: null });
  });

  it("reset 清空全部标签", () => {
    tabsStore.openDoc("a.md");
    tabsStore.openPreview("x.png");
    tabsStore.reset();
    expect(tabsStore.getSnapshot()).toEqual({ tabs: [], activeTabId: null });
  });
});
