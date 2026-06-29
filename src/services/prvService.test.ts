import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: vi.fn((p: string) => `asset://localhost/${encodeURIComponent(p)}`),
}));
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { previewDoc, exportDoc, mediaSrc } from "./prvService";

const invokeMock = vi.mocked(invoke);
const ok = <T>(result: T) => ({ correlation_id: "c", result });
const cmd = (call: unknown) => (call as [string, { request: { command: string } }])[1].request.command;

// 只读/转换命令白名单：预览=出 PDF 字节、媒体=出 asset URL、导出=落新文件，均不原地改源。
const READONLY_CMDS = new Set(["prv.preview", "prv.media_src", "prv.export"]);

beforeEach(() => {
  invokeMock.mockReset();
  vi.mocked(convertFileSrc).mockClear();
  // jsdom 默认无 URL.createObjectURL（previewDoc 组 blob 用），测试桩。
  Object.defineProperty(URL, "createObjectURL", { value: vi.fn(() => "blob:stub"), writable: true, configurable: true });
});

describe("prvService 只读预览边界（PRV-VIEW）", () => {
  // covers: BR-PRV-DATA-002
  it("mediaSrc → prv.media_src 守界 + convertFileSrc 得 asset URL（流式只读，不进内存）", async () => {
    invokeMock.mockResolvedValue(ok({ abs_path: "/ws/clip.mp4" }));
    const url = await mediaSrc("clip.mp4");
    expect((invokeMock.mock.calls[0] as [string, { request: { command: string; params: unknown } }])[1].request).toMatchObject({
      command: "prv.media_src",
      params: { relPath: "clip.mp4" },
    });
    expect(convertFileSrc).toHaveBeenCalledWith("/ws/clip.mp4");
    expect(url).toContain("asset://");
  });

  // covers: BR-PRV-DATA-001
  it("只读硬边界：预览/媒体/导出命令面全为只读或转换，无原地写命令；导出落新文件不改源", async () => {
    invokeMock.mockResolvedValueOnce(ok({ format: "pdf", bytes: [37, 80, 68, 70] })); // previewDoc
    invokeMock.mockResolvedValueOnce(ok({ abs_path: "/ws/a.png" })); // mediaSrc
    invokeMock.mockResolvedValueOnce(ok({ rel_path: "out/a.pdf" })); // exportDoc
    await previewDoc("a.docx");
    await mediaSrc("a.png");
    const dest = await exportDoc("a.docx", "pdf");
    expect(dest).toBe("out/a.pdf"); // 导出到新路径（不原地改源）
    for (const call of invokeMock.mock.calls) {
      expect(READONLY_CMDS.has(cmd(call))).toBe(true);
    }
  });
});
