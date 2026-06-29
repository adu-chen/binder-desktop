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
  it("pollExternalChange：磁盘变更 → external_changed", async () => {
    invokeMock.mockResolvedValue(ok({ changed: true, deleted: false, current: 999 }));
    await pollExternalChange();
    expect(edStore.getSnapshot().tabState).toBe("external_changed");
    expect(vi.mocked(invoke).mock.calls[0][1]).toMatchObject({
      request: { command: "ws.check_external_change", params: { relPath: "a.md", baseline: 100 } },
    });
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
});
