import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import { listLocalFonts, resetLocalFontCache } from "./fontService";

const invokeMock = vi.mocked(invoke);
const ok = <T>(result: T) => ({ correlation_id: "c", result });

// covers: BR-SYS-DATA-003 — 字体枚举经 command 注册表路由（X-SEQ-002 信封）
describe("fontService · 本地字体库只读枚举", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    resetLocalFontCache();
  });

  it("listLocalFonts → fonts.list_fonts，返回族名数组", async () => {
    invokeMock.mockResolvedValue(ok({ families: ["Arial", "PingFang SC", "Times New Roman"] }));
    const families = await listLocalFonts();
    expect((invokeMock.mock.calls[0] as [string, { request: { command: string } }])[1].request.command).toBe("fonts.list_fonts");
    expect(families).toEqual(["Arial", "PingFang SC", "Times New Roman"]);
  });

  it("module 级缓存：第二次调用不重复 invoke", async () => {
    invokeMock.mockResolvedValue(ok({ families: ["Arial"] }));
    await listLocalFonts();
    await listLocalFonts();
    expect(invokeMock).toHaveBeenCalledTimes(1);
  });

  it("结果缺失回空数组（成功但空契约）", async () => {
    invokeMock.mockResolvedValue(ok({ families: undefined }));
    const families = await listLocalFonts();
    expect(families).toEqual([]);
  });

  it("失败抛错且缓存不置位（上层回退常用子集）", async () => {
    invokeMock.mockResolvedValue({ correlation_id: "c", error: { code: "FONTS_LIST_FAILED", message: "boom", retriable: true, reauth_required: false } });
    await expect(listLocalFonts()).rejects.toMatchObject({ code: "FONTS_LIST_FAILED" });
    expect(invokeMock).toHaveBeenCalledTimes(1);
  });

  it("resetLocalFontCache 后允许重试", async () => {
    invokeMock.mockResolvedValueOnce({ correlation_id: "c", error: { code: "X", message: "m", retriable: true, reauth_required: false } });
    await expect(listLocalFonts()).rejects.toBeTruthy();
    resetLocalFontCache();
    invokeMock.mockResolvedValueOnce(ok({ families: ["Arial"] }));
    await expect(listLocalFonts()).resolves.toEqual(["Arial"]);
  });
});
