import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
import { invoke } from "@tauri-apps/api/core";
import { saveDocument, pollExternalChange } from "./edService";
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
  it("保存成功 → markSaved（刷新基线、回 saved）", async () => {
    invokeMock.mockResolvedValue(ok({ baseline: 200 }));
    const r = await saveDocument("new content");
    expect(r).toEqual({ ok: true });
    expect(edStore.getSnapshot()).toMatchObject({ baseline: 200, tabState: "saved" });
    expect(vi.mocked(invoke).mock.calls[0][1]).toMatchObject({
      request: { command: "ws.save_doc", params: { relPath: "a.md", baseline: 100 } },
    });
  });

  it("WS_SAVE_CONFLICT → external_changed（不静默覆盖）", async () => {
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

  it("pollExternalChange：磁盘变更 → external_changed", async () => {
    invokeMock.mockResolvedValue(ok({ changed: true, deleted: false, current: 999 }));
    await pollExternalChange();
    expect(edStore.getSnapshot().tabState).toBe("external_changed");
    expect(vi.mocked(invoke).mock.calls[0][1]).toMatchObject({
      request: { command: "ws.check_external_change", params: { relPath: "a.md", baseline: 100 } },
    });
  });
});
