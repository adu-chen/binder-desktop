/** @vitest-environment jsdom */
import { beforeEach, describe, expect, it } from "vitest";
import { Schema } from "@tiptap/pm/model";
import { edStore } from "./edStore";
import {
  getPageGeometry,
  pageContentHeight,
  edViewPrefsStore,
  PAPERS,
} from "./edViewPrefsStore";
import { extractOutline, sameOutline, outlineStore } from "./outlineStore";

// Migrated intact from src/store/edStore.test.ts
{
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

  it("用户编辑将 pending_diff 转为 dirty，保留未落盘事实", () => {
    edStore.openDoc("a.md");
    edStore.setTabState("a.md", "pending_diff");
    edStore.markDirty();
    expect(edStore.getSnapshot().tabState).toBe("dirty");
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

  it("DOCX 样式风险随会话装载并在活动投影中可见", () => {
    edStore.loadDoc("color.draft.docx", "<p>红字</p>", 8, null, "html", "html", {
      styleNotices: ["检测到 1 个颜色片段未能恢复"],
    });
    expect(edStore.getSnapshot().styleNotices).toEqual(["检测到 1 个颜色片段未能恢复"]);
    expect(edStore.getSnapshot().sessions["color.draft.docx"].styleNotices).toEqual([
      "检测到 1 个颜色片段未能恢复",
    ]);
  });

  it("后台载入只补全非活动会话，不改 activePath 或触发当前编辑器重载", () => {
    edStore.loadDoc("active.md", "当前文档", 1);
    const before = edStore.getSnapshot();

    edStore.loadDoc("late.docx", "迟到结果", 2, null, "html", "html", { activate: false });

    const after = edStore.getSnapshot();
    expect(after).toMatchObject({ openPath: "active.md", loadedContent: "当前文档", loadSeq: before.loadSeq });
    expect(after.sessions["late.docx"]).toMatchObject({ content: "迟到结果", editFormat: "html" });
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

  it("Markdown 终态会话显式保存 contentFormat=html", () => {
    edStore.loadDoc("notes.md", "<h1>标题</h1>", 3, null, "markdown", "html");
    const snapshot = edStore.getOpenSnapshots()[0];
    expect(snapshot).toMatchObject({ editFormat: "markdown", contentFormat: "html", content: "<h1>标题</h1>" });
  });

  it("getOpenSnapshots 可显式标识 active_editor_session source", () => {
    edStore.loadDoc("a.md", "A", 1);
    edStore.loadDoc("b.md", "B", 2);
    edStore.setActivePath("a.md");

    expect(edStore.getOpenSnapshots("active_editor_session")).toEqual(expect.arrayContaining([
      expect.objectContaining({ relPath: "a.md", source: "active_editor_session" }),
      expect.objectContaining({ relPath: "b.md", source: "open_editor_session" }),
    ]));
  });

  it("restoreSnapshots：按打开标签当前路径恢复 activePath 并推进 loadSeq", () => {
    edStore.restoreSnapshots([
      {
        relPath: "a.md",
        content: "A",
        editFormat: "text",
        contentFormat: "text",
        source: "open_editor_session",
        dirty: false,
        sessionVersion: 1,
        contentHash: "ha",
        capturedAt: 1,
      },
      {
        relPath: "b.md",
        content: "B",
        baseline: 42,
        editFormat: "text",
        contentFormat: "text",
        source: "open_editor_session",
        dirty: true,
        sessionVersion: 3,
        contentHash: "hb",
        capturedAt: 2,
      },
    ], "b.md");

    const s = edStore.getSnapshot();
    expect(s.openPath).toBe("b.md");
    expect(s.loadedContent).toBe("B");
    expect(s.loadSeq).toBe(1);
    expect(s.sessions["b.md"]).toMatchObject({
      baseline: 42,
      tabState: "dirty",
      sessionVersion: 3,
      contentHash: "hb",
    });
  });

  it("closeSession 保留会话缓存；关活动会话则置空活动", () => {
    edStore.loadDoc("a.md", "A", 1);
    edStore.loadDoc("b.md", "B", 2);
    edStore.closeSession("b.md");
    let s = edStore.getSnapshot();
    expect(s.sessions["b.md"]?.content).toBe("B");
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

  it("关闭标签后仍保留会话，保存态投影继续按路径更新", () => {
    edStore.loadDoc("a.md", "A", 1);
    edStore.closeSession("a.md");
    expect(() => {
      edStore.markSaved("a.md", 7);
      edStore.setTabState("a.md", "dirty");
    }).not.toThrow();
    expect(edStore.getSnapshot().sessions["a.md"]?.tabState).toBe("dirty");
  });
});
}

// Migrated intact from src/store/edViewPrefsStore.test.ts
{
describe("edViewPrefsStore · getPageGeometry（纯函数）", () => {
  it("A4 竖排基准尺寸", () => {
    const g = getPageGeometry("A4", "portrait");
    expect(g.width).toBe(PAPERS.A4.width);
    expect(g.height).toBe(PAPERS.A4.height);
  });

  it("横排交换宽高（各纸张）", () => {
    for (const k of ["A4", "A3", "A5", "Letter"] as const) {
      const p = getPageGeometry(k, "portrait");
      const l = getPageGeometry(k, "landscape");
      expect(l.width).toBe(p.height);
      expect(l.height).toBe(p.width);
      expect(l.marginX).toBe(p.marginX); // 边距/缝隙恒定
      expect(l.gap).toBe(p.gap);
    }
  });

  it("内容高度预算 = 纸高 − 2×上下边距，恒为正", () => {
    for (const k of ["A4", "A3", "A5", "Letter"] as const) {
      for (const o of ["portrait", "landscape"] as const) {
        const g = getPageGeometry(k, o);
        expect(pageContentHeight(g)).toBe(g.height - g.marginY * 2);
        expect(pageContentHeight(g)).toBeGreaterThan(0);
      }
    }
  });
});

describe("edViewPrefsStore · 持久化与订阅", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("未存偏好回默认（A4 竖排 100%）", () => {
    const p = edViewPrefsStore.getPrefs("fresh-" + Math.random());
    expect(p).toEqual({ paper: "A4", orientation: "portrait", zoom: 1 });
  });

  it("setPaper/setOrientation 写穿 localStorage 并可回读", () => {
    const path = "docs/t1.docx";
    edViewPrefsStore.setPaper(path, "A3");
    edViewPrefsStore.setOrientation(path, "landscape");
    const raw = JSON.parse(localStorage.getItem("binder.ed.viewprefs." + path)!);
    expect(raw.paper).toBe("A3");
    expect(raw.orientation).toBe("landscape");
    expect(edViewPrefsStore.getPrefs(path).paper).toBe("A3");
  });

  it("损坏/脏数据回落默认（字段级校验）", () => {
    const path = "docs/t2.docx";
    localStorage.setItem("binder.ed.viewprefs." + path, '{"paper":"B5","orientation":"diagonal","zoom":"x"}');
    const p = edViewPrefsStore.getPrefs(path);
    expect(p).toEqual({ paper: "A4", orientation: "portrait", zoom: 1 });
  });

  it("subscribe 在变更时通知", () => {
    const path = "docs/t3-" + Math.random();
    let n = 0;
    const unsub = edViewPrefsStore.subscribe(() => (n += 1));
    edViewPrefsStore.setPaper(path, "A5");
    expect(n).toBe(1);
    unsub();
  });
});
}

// Migrated intact from src/store/outlineStore.test.ts
{
// PI-ED-12 大纲纯逻辑：顶层 heading → {blockId, level, text}；无 blockId 标题跳过（无锚不可跳转）；
// 相等比较避免无谓发布。

const schema = new Schema({
  nodes: {
    doc: { content: "block+" },
    heading: {
      group: "block",
      content: "inline*",
      attrs: { blockId: { default: null }, level: { default: 1 } },
    },
    paragraph: { group: "block", content: "inline*", attrs: { blockId: { default: null } } },
    text: { group: "inline" },
  },
});

const h = (id: string | null, level: number, text: string) =>
  schema.node("heading", { blockId: id, level }, text ? [schema.text(text)] : []);
const p = (text: string) => schema.node("paragraph", { blockId: "pb" }, [schema.text(text)]);

describe("outlineStore · extractOutline", () => {
  it("提取标题树（级别 + 文本 + 锚），段落不入大纲", () => {
    const doc = schema.node("doc", null, [h("h1", 1, "第一章"), p("正文"), h("h2", 2, "第一节")]);
    expect(extractOutline(doc)).toEqual([
      { blockId: "h1", level: 1, text: "第一章" },
      { blockId: "h2", level: 2, text: "第一节" },
    ]);
  });

  it("无 blockId 的标题跳过（不产死项）", () => {
    const doc = schema.node("doc", null, [h(null, 1, "无锚标题"), h("h2", 2, "有锚")]);
    expect(extractOutline(doc)).toEqual([{ blockId: "h2", level: 2, text: "有锚" }]);
  });
});

describe("outlineStore · sameOutline / set", () => {
  it("相等不发布，变化才通知", () => {
    outlineStore.reset();
    let n = 0;
    const unsub = outlineStore.subscribe(() => (n += 1));
    const items = [{ blockId: "a", level: 1, text: "T" }];
    outlineStore.set(items);
    expect(n).toBe(1);
    outlineStore.set([{ blockId: "a", level: 1, text: "T" }]); // 内容相等 → 跳过
    expect(n).toBe(1);
    outlineStore.set([{ blockId: "a", level: 2, text: "T" }]); // level 变 → 通知
    expect(n).toBe(2);
    expect(sameOutline([], [])).toBe(true);
    unsub();
    outlineStore.reset();
  });
});
}
