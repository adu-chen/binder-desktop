import { beforeEach, describe, expect, it } from "vitest";
import { tabsStore } from "./tabsStore";

beforeEach(() => tabsStore.reset());

describe("tabsStore 标签模型（视图层）", () => {
  it("openDoc 多文档标签共存：不同路径各开标签，同路径去重 + 激活（PI-FE-27）", () => {
    tabsStore.openDoc("a.md");
    tabsStore.openDoc("b.md");
    let docs = tabsStore.getSnapshot().tabs.filter((t) => t.kind === "doc");
    expect(docs).toHaveLength(2);
    tabsStore.openDoc("a.md"); // 同路径 → 不新增，激活已有
    const s = tabsStore.getSnapshot();
    docs = s.tabs.filter((t) => t.kind === "doc");
    expect(docs).toHaveLength(2);
    expect(s.tabs.find((t) => t.id === s.activeTabId)?.path).toBe("a.md");
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

  // PI-ED-07 #5 标签拖动排序
  it("reorder：移到目标前 / 移到末尾 / 无效 id 空操作，不改激活态", () => {
    tabsStore.reset();
    tabsStore.openDoc("a.md");
    tabsStore.openDoc("b.md");
    tabsStore.openDoc("c.md");
    const [a, b, c] = tabsStore.getSnapshot().tabs;
    tabsStore.activate(b.id);

    tabsStore.reorder(c.id, a.id); // c 移到 a 前 → [c,a,b]
    expect(tabsStore.getSnapshot().tabs.map((t) => t.path)).toEqual(["c.md", "a.md", "b.md"]);
    expect(tabsStore.getSnapshot().activeTabId).toBe(b.id); // 激活不变

    tabsStore.reorder(c.id, null); // c 移到末尾 → [a,b,c]
    expect(tabsStore.getSnapshot().tabs.map((t) => t.path)).toEqual(["a.md", "b.md", "c.md"]);

    const before = tabsStore.getSnapshot().tabs;
    tabsStore.reorder("不存在", a.id); // 无效 → 空操作
    tabsStore.reorder(a.id, a.id); // 同位 → 空操作
    expect(tabsStore.getSnapshot().tabs).toEqual(before);
  });
});
