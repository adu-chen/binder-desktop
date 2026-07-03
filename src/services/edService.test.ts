import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
import { invoke } from "@tauri-apps/api/core";
import { saveDocument, pollExternalChange, openDocument } from "./edService";
import { edStore } from "../store/edStore";

const invokeMock = vi.mocked(invoke);
const ok = <T>(result: T) => ({ correlation_id: "c", result });
const fail = (code: string) => ({ correlation_id: "c", error: { code, message: "冲突", retriable: false, reauth_required: false } });

beforeEach(() => {
  invokeMock.mockReset();
  edStore.reset();
  edStore.loadDoc("a.md", "content", 100); // openPath + baseline 就绪，tabState=saved
});

describe("edService 保存契约（SM-WS-01 / BR-WS-OBS-001）", () => {
  it("保存成功 → setBaseline 刷新基线；saved 态由 SM-WS-01 广播投影（A-CORE-X-L-04 Task 2-C）", async () => {
    invokeMock.mockResolvedValue(ok({ baseline: 200 }));
    // 先转脏态，验证 saveDocument 自身不再乐观置 saved（改由广播驱动）。
    edStore.markDirty();
    expect(edStore.getSnapshot().tabState).toBe("dirty");
    const r = await saveDocument("new content");
    expect(r).toEqual({ ok: true });
    // 基线经后端返回值回填（setBaseline）；tabState 仍是 dirty——saved 需后端广播。
    expect(edStore.getSnapshot()).toMatchObject({ baseline: 200, tabState: "dirty" });
    // 模拟后端 SM-WS-01 广播 Saved → 标签转 saved（smListeners 投影路径）。
    edStore.applyWsSaveState("a.md", "Saved");
    expect(edStore.getSnapshot().tabState).toBe("saved");
    expect(vi.mocked(invoke).mock.calls[0][1]).toMatchObject({
      request: { command: "ws.save_doc", params: { relPath: "a.md", baseline: 100 } },
    });
  });

  // covers: BR-ED-STATE-003
  it("WS_SAVE_CONFLICT → external_changed（不静默覆盖，待用户显式覆盖/重载）", async () => {
    invokeMock.mockResolvedValue(fail("WS_SAVE_CONFLICT"));
    const r = await saveDocument("mine");
    expect(r).toMatchObject({ ok: false, conflict: true });
    expect(edStore.getSnapshot().tabState).toBe("external_changed");
  });

  it("无打开文档 → 不保存", async () => {
    edStore.reset();
    const r = await saveDocument("x");
    expect(r).toMatchObject({ ok: false, conflict: false });
    expect(invokeMock).not.toHaveBeenCalled();
  });

  // covers: BR-ED-STATE-003
  it("pollExternalChange 有脏态：磁盘变更 → external_changed 显式裁决（不静默覆盖/重载）", async () => {
    edStore.markDirty(); // 逻辑状态有独立修改 → 双向内容损失须用户显式选择
    invokeMock.mockResolvedValue(ok({ changed: true, deleted: false, current: 999 }));
    await pollExternalChange();
    expect(edStore.getSnapshot().tabState).toBe("external_changed");
    expect(vi.mocked(invoke).mock.calls[0][1]).toMatchObject({
      request: { command: "ws.check_external_change", params: { relPath: "a.md", baseline: 100 } },
    });
  });

  // covers: BR-ED-STATE-003
  it("pollExternalChange 无脏态：磁盘变更 → 自动重载刷新基线（2026-07-03 三态审计修复，不弹裁决）", async () => {
    // saved 态（无独立修改）；第一次调用回 changed，第二次（openDocument→ws.read_doc）回新内容。
    invokeMock
      .mockResolvedValueOnce(ok({ changed: true, deleted: false, current: 999 }))
      .mockResolvedValueOnce(ok({ content: "外部新内容", baseline: 999 }));
    await pollExternalChange();
    const s = edStore.getSnapshot();
    expect(s.tabState).toBe("saved"); // 重载后回 saved，非 external_changed
    expect(s).toMatchObject({ loadedContent: "外部新内容", baseline: 999 });
    expect(vi.mocked(invoke).mock.calls[1][1]).toMatchObject({ request: { command: "ws.read_doc" } });
  });

  // covers: BR-ED-STATE-003
  it("pollExternalChange saving 态：跳过检测（磁盘已写基线未回填的竞态窗口防误报）", async () => {
    edStore.setTabState("a.md", "saving");
    await pollExternalChange();
    expect(invokeMock).not.toHaveBeenCalled();
  });
});

describe("edService openDocument 路由（BR-ED-DATA-002 md/docx 统一编辑模型）", () => {
  // covers: BR-ED-DATA-002
  it("md → ws.read_doc 文本模型（editFormat=text）", async () => {
    edStore.reset();
    invokeMock.mockResolvedValue(ok({ content: "# md", baseline: 11 }));
    await openDocument("notes/x.md");
    const s = edStore.getSnapshot();
    expect(s).toMatchObject({ openPath: "notes/x.md", editFormat: "text", loadedContent: "# md", baseline: 11 });
    expect(vi.mocked(invoke).mock.calls[0][1]).toMatchObject({ request: { command: "ws.read_doc" } });
  });

  // covers: BR-ED-DATA-002
  it("*.draft.docx → doc.open_for_edit HTML 模型（editFormat=html，ED 不解析二进制）", async () => {
    edStore.reset();
    invokeMock.mockResolvedValue(ok({ format: "docx", html: "<p>h</p>", baseline: 22 }));
    await openDocument("notes/y.draft.docx");
    const s = edStore.getSnapshot();
    expect(s).toMatchObject({ openPath: "notes/y.draft.docx", editFormat: "html", loadedContent: "<p>h</p>", baseline: 22 });
    expect(vi.mocked(invoke).mock.calls[0][1]).toMatchObject({ request: { command: "doc.open_for_edit" } });
  });

  // covers: BR-ED-DATA-002
  it(".html 独立文件 → ws.read_doc 富文本模型（editFormat=html_file，不经 DOC，PI-ED-13/REQ-ED-008）", async () => {
    edStore.reset();
    invokeMock.mockResolvedValue(ok({ content: '<p data-block-id="b1">页面</p>', baseline: 33 }));
    await openDocument("site/index.html");
    const s = edStore.getSnapshot();
    expect(s).toMatchObject({ openPath: "site/index.html", editFormat: "html_file", baseline: 33 });
    // 文本读取路径（ws.read_doc）而非 DOC 编辑链——.html 非二进制（X-SEQ-004 无涉）。
    expect(vi.mocked(invoke).mock.calls[0][1]).toMatchObject({ request: { command: "ws.read_doc" } });
  });

  it("打开失败 → 撤销占位会话（不留假空文档，PI-ED-07）", async () => {
    edStore.reset();
    invokeMock.mockResolvedValue(fail("WS_NOT_FOUND"));
    await openDocument("gone.md");
    expect(edStore.getSnapshot().sessions["gone.md"]).toBeUndefined();
    expect(edStore.getSnapshot().openPath).toBeNull();
  });
});
