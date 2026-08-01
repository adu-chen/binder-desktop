import { beforeEach, describe, expect, it } from "vitest";
import { closeStore } from "./closeStore";
import { clampWidth, layoutStore, PANE_LIMITS } from "./layoutStore";
import { tabsStore } from "./tabsStore";
import { edStore } from "./edStore";

// Migrated intact from src/store/closeStore.test.ts.
{
/** covers: BR-SYS-STATE-004 — SM-SYS-02 前端投影的基础转移语义（供 CloseConfirmDialog 渲染）。 */
describe("closeStore（SM-SYS-02 前端投影）", () => {
  beforeEach(() => {
    closeStore.reset();
  });

  it("初始态为 idle，无残留计数", () => {
    const s = closeStore.getSnapshot();
    expect(s.phase).toBe("idle");
    expect(s.pendingDiffCount).toBe(0);
    expect(s.dirtyDocuments).toEqual([]);
    expect(s.busy).toBe(false);
    expect(s.error).toBeNull();
  });

  it("setChecking → checking 且 busy，清除旧错误", () => {
    closeStore.setFailed("旧错误");
    closeStore.setChecking();
    const s = closeStore.getSnapshot();
    expect(s.phase).toBe("checking");
    expect(s.busy).toBe(true);
    expect(s.error).toBeNull();
  });

  it("setAwaitingDecision 携带计数且 busy=false", () => {
    closeStore.setAwaitingDecision(2, [{ relPath: "a.md", hasAcceptedAwaitingSave: true }]);
    const s = closeStore.getSnapshot();
    expect(s.phase).toBe("awaitingDecision");
    expect(s.pendingDiffCount).toBe(2);
    expect(s.dirtyDocuments).toEqual([{ relPath: "a.md", hasAcceptedAwaitingSave: true }]);
    expect(s.busy).toBe(false);
  });

  it("setResolving → resolving 且 busy", () => {
    closeStore.setResolving();
    const s = closeStore.getSnapshot();
    expect(s.phase).toBe("resolving");
    expect(s.busy).toBe(true);
  });

  it("setFailed 回 awaitingDecision 且 busy=false，不静默丢失失败原因", () => {
    closeStore.setResolving();
    closeStore.setFailed("保存冲突");
    const s = closeStore.getSnapshot();
    expect(s.phase).toBe("awaitingDecision");
    expect(s.busy).toBe(false);
    expect(s.error).toBe("保存冲突");
  });

  it("setClosing → closing 且 busy", () => {
    closeStore.setClosing();
    const s = closeStore.getSnapshot();
    expect(s.phase).toBe("closing");
    expect(s.busy).toBe(true);
  });

  it("reset 恢复初始态（取消关闭场景）", () => {
    closeStore.setAwaitingDecision(3, []);
    closeStore.reset();
    expect(closeStore.getSnapshot().phase).toBe("idle");
  });

  it("订阅者在状态变化时收到通知", () => {
    let notified = 0;
    const unsubscribe = closeStore.subscribe(() => {
      notified += 1;
    });
    closeStore.setChecking();
    closeStore.setAwaitingDecision(1, []);
    unsubscribe();
    closeStore.setResolving();
    expect(notified).toBe(2); // 取消订阅后的变化不再计数
  });
});
}

// Migrated intact from src/store/layoutStore.test.ts.
{
describe("layoutStore · clampWidth", () => {
  it("在区间内原样返回", () => {
    expect(clampWidth(300, 180, 520)).toBe(300);
  });
  it("低于 min 钳到 min，高于 max 钳到 max", () => {
    expect(clampWidth(100, 180, 520)).toBe(180);
    expect(clampWidth(900, 180, 520)).toBe(520);
  });
  it("边界值含入", () => {
    expect(clampWidth(180, 180, 520)).toBe(180);
    expect(clampWidth(520, 180, 520)).toBe(520);
  });
  it("NaN/非有限（含 Infinity）→ 回落 min（安全兜底）", () => {
    expect(clampWidth(NaN, 180, 520)).toBe(180);
    expect(clampWidth(Infinity, 180, 520)).toBe(180); // 非有限值一律回落 min
    expect(clampWidth(-Infinity, 180, 520)).toBe(180);
  });
});

describe("layoutStore · 设置与钳制", () => {
  beforeEach(() => {
    layoutStore.resetSide();
    layoutStore.resetChat();
  });

  it("setSideW 钳制到 side 约束", () => {
    layoutStore.setSideW(10_000);
    expect(layoutStore.getSnapshot().sideW).toBe(PANE_LIMITS.side.max);
    layoutStore.setSideW(0);
    expect(layoutStore.getSnapshot().sideW).toBe(PANE_LIMITS.side.min);
  });

  it("setChatW 钳制到 chat 约束", () => {
    layoutStore.setChatW(10_000);
    expect(layoutStore.getSnapshot().chatW).toBe(PANE_LIMITS.chat.max);
    layoutStore.setChatW(0);
    expect(layoutStore.getSnapshot().chatW).toBe(PANE_LIMITS.chat.min);
  });

  it("reset 恢复默认宽", () => {
    layoutStore.setSideW(200);
    layoutStore.resetSide();
    expect(layoutStore.getSnapshot().sideW).toBe(PANE_LIMITS.side.default);
  });

  it("subscribe 在变化时通知，无变化不通知", () => {
    layoutStore.setSideW(PANE_LIMITS.side.default); // 先归默认
    let n = 0;
    const unsub = layoutStore.subscribe(() => (n += 1));
    layoutStore.setSideW(300);
    expect(n).toBe(1);
    layoutStore.setSideW(300); // 同值不通知
    expect(n).toBe(1);
    unsub();
  });
});
}

// Migrated intact from src/store/tabsStore.test.ts.
{
beforeEach(() => {
  tabsStore.reset();
  edStore.reset();
});

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

  it("getPersistedOpenDocTabs：只持久化文档标签，保留顺序和当前文档", () => {
    tabsStore.openDoc("a.md");
    tabsStore.openPreview("x.png");
    edStore.loadDoc("a.md", "A", 1);
    tabsStore.openDoc("b.md");
    edStore.loadDoc("b.md", "B", 2);

    expect(tabsStore.getPersistedOpenDocTabs()).toEqual([
      { relPath: "a.md", kind: "doc", openIndex: 0, isActive: false },
      { relPath: "b.md", kind: "doc", openIndex: 2, isActive: true },
    ]);
  });

  it("restoreDocTabs：恢复标签集合并同步 edStore 当前文档", () => {
    edStore.loadDoc("a.md", "A", 1);
    edStore.loadDoc("b.md", "B", 2);
    edStore.setActivePath("a.md");

    tabsStore.restoreDocTabs([
      { relPath: "a.md", kind: "doc", openIndex: 0, isActive: false },
      { relPath: "b.md", kind: "doc", openIndex: 1, isActive: true },
    ]);

    const tabs = tabsStore.getSnapshot();
    expect(tabs.tabs.map((tab) => tab.path)).toEqual(["a.md", "b.md"]);
    expect(tabs.tabs.find((tab) => tab.id === tabs.activeTabId)?.path).toBe("b.md");
    expect(edStore.getSnapshot().openPath).toBe("b.md");
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
}
