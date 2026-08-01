import { beforeEach, describe, it, expect, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: (path: string) => `asset://localhost/${path}`,
}));
import { invoke } from "@tauri-apps/api/core";
import {
  ensureDocumentMediaDisplayBase,
  getDocxStyleReport,
  importProjectImage,
  importProjectImageData,
  openDocxForEdit,
  prepareDocxSnapshotHtml,
  projectDocumentMediaDisplaySrc,
  projectMediaDisplaySrc,
  restoreDocxEditMedia,
  restoreMediaSrc,
  rewriteMediaSrc,
  saveDocxFromEdit,
} from "./docService";

const invokeMock = vi.mocked(invoke);

beforeEach(() => invokeMock.mockReset());

// covers: BR-ED-DATA-003
// 图文呈现：编辑链 HTML 相对 <img src> 改写为 asset URL；绝对/内联/已处理 src 原样保留；不改正文文本。
// 用 DI fake converter 断言改写落点，不依赖 tauri 运行时（convertFileSrc）。
const fake = (p: string) => `asset://localhost/${p}`;

describe("docService · rewriteMediaSrc (BR-ED-DATA-003)", () => {
  it("相对 media 路径 → asset URL（拼接绝对路径）", () => {
    const html = `<p>x</p><img src="media/image1.png" alt="a">`;
    const out = rewriteMediaSrc(html, "/tmp/work", fake);
    expect(out).toContain(`src="asset://localhost//tmp/work/media/image1.png"`);
    expect(out).toContain(`data-binder-media-src="media/image1.png"`);
    expect(out).toContain(`alt="a"`); // 其余属性保留
    expect(out).toContain(`<p>x</p>`); // 正文文本不动
  });

  it("多张图片全部改写", () => {
    const html = `<img src="media/a.png"><img src='media/b.jpg'>`;
    const out = rewriteMediaSrc(html, "/w", fake);
    expect(out).toContain(`asset://localhost//w/media/a.png`);
    expect(out).toContain(`asset://localhost//w/media/b.jpg`);
  });

  it("data: / https: / asset: / blob: src 原样保留", () => {
    const html =
      `<img src="data:image/png;base64,AAAA">` +
      `<img src="https://example.com/x.png">` +
      `<img src="asset://localhost/already">` +
      `<img src="blob:abc">`;
    expect(rewriteMediaSrc(html, "/w", fake)).toBe(html);
  });

  it("./ 前缀相对路径归一化后拼接", () => {
    const out = rewriteMediaSrc(`<img src="./media/x.png">`, "/w", fake);
    expect(out).toContain(`asset://localhost//w/media/x.png`);
  });

  it("base 尾部斜杠去重", () => {
    const out = rewriteMediaSrc(`<img src="media/x.png">`, "/w/", fake);
    expect(out).toContain(`asset://localhost//w/media/x.png`);
    expect(out).not.toContain(`//w//media`);
  });

  it("无图片的 HTML 原样返回", () => {
    const html = `<h1>标题</h1><p>正文 <strong>粗体</strong></p>`;
    expect(rewriteMediaSrc(html, "/w", fake)).toBe(html);
  });

  it("单双引号 src 均支持", () => {
    expect(rewriteMediaSrc(`<img src='media/x.png'>`, "/w", fake)).toContain(
      `asset://localhost//w/media/x.png`,
    );
  });

  it("保存反向映射以稳定 data 属性为真值，不依赖当前进程 Map", () => {
    const restored = restoreMediaSrc(
      `<img src="asset://localhost/fresh-process" data-binder-media-src="media/image9.png">`,
    );
    expect(restored).toContain(`src="media/image9.png"`);
    expect(restored).toContain(`data-binder-media-src="media/image9.png"`);
  });

  it("持久快照保持纯净规范 HTML", () => {
    const displayed = rewriteMediaSrc(`<img src="media/image1.png">`, "/cache/7", fake);
    const persisted = prepareDocxSnapshotHtml(displayed, "7");
    expect(persisted).not.toContain("binder-docx-media-baseline");
    expect(persisted).toContain(`src="media/image1.png"`);
  });

  it("代理物化图片只把已授权 baseline 媒体投影为显示 URL，持久化仍保留来源/hash", () => {
    const hash = "a".repeat(64);
    const display = projectMediaDisplaySrc("/cache/7", `media/binder-project-${hash}.png`, fake);
    expect(display).toBe(`asset://localhost//cache/7/media/binder-project-${hash}.png`);
    const persisted = prepareDocxSnapshotHtml(
      `<img src="${display}" data-binder-media-src="media/binder-project-${hash}.png" data-binder-project-src="images/cover.png" data-binder-project-hash="${hash}">`,
      "7",
    );
    expect(persisted).toContain(`src="media/binder-project-${hash}.png"`);
    expect(persisted).toContain(`data-binder-project-src="images/cover.png"`);
    expect(persisted).toContain(`data-binder-project-hash="${hash}"`);
  });

  it("应用重启恢复时按快照 baseline 重新请求媒体目录并生成新显示 URL", async () => {
    invokeMock.mockResolvedValue({
      correlation_id: "c",
      result: { baseline: "7", media_base: "/cache/7" },
    });
    const snapshot = `<img src="media/image1.png" data-binder-media-src="media/image1.png">`;
    const restored = await restoreDocxEditMedia("x.draft.docx", snapshot, "7");
    expect(restored).toMatchObject({ baseline: "7" });
    expect(restored.html).toContain("asset://localhost//cache/7/media/image1.png");
    expect(projectDocumentMediaDisplaySrc("x.draft.docx", "media/new.png"))
      .toBe("asset://localhost//cache/7/media/new.png");
    expect(invokeMock).toHaveBeenCalledWith("dispatch", expect.objectContaining({
      request: expect.objectContaining({
        command: "doc.restore_edit_media",
        params: expect.objectContaining({ relPath: "x.draft.docx", baseline: "7" }),
      }),
    }));
  });

  it("活动 BDocx 尚未登记媒体目录时幂等执行恢复并建立显示映射", async () => {
    invokeMock.mockResolvedValue({
      correlation_id: "c",
      result: { baseline: "12", media_base: "/cache/12" },
    });
    await ensureDocumentMediaDisplayBase(
      "fresh-image-session.draft.docx",
      '<p data-block-id="b1">正文</p>',
      "12",
    );
    expect(invokeMock).toHaveBeenCalledWith("dispatch", expect.objectContaining({
      request: expect.objectContaining({
        command: "doc.restore_edit_media",
        params: expect.objectContaining({
          relPath: "fresh-image-session.draft.docx",
          baseline: "12",
        }),
      }),
    }));
    expect(projectDocumentMediaDisplaySrc(
      "fresh-image-session.draft.docx",
      "media/new.png",
    )).toBe("asset://localhost//cache/12/media/new.png");

    invokeMock.mockClear();
    await ensureDocumentMediaDisplayBase(
      "fresh-image-session.draft.docx",
      '<p data-block-id="b1">正文</p>',
      "12",
    );
    expect(invokeMock).not.toHaveBeenCalled();

    invokeMock.mockResolvedValue({
      correlation_id: "c2",
      result: { baseline: "13", media_base: "/cache/13" },
    });
    await ensureDocumentMediaDisplayBase(
      "fresh-image-session.draft.docx",
      '<p data-block-id="b1">正文已更新</p>',
      "13",
    );
    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(projectDocumentMediaDisplaySrc(
      "fresh-image-session.draft.docx",
      "media/new.png",
    )).toBe("asset://localhost//cache/13/media/new.png");
  });

  it("保存成功后切换到新 baseline 媒体目录，后续代理图片不沿用旧缓存", async () => {
    invokeMock.mockResolvedValue({
      correlation_id: "c",
      result: { baseline: "8", media_base: "/cache/8" },
    });
    await saveDocxFromEdit("x.draft.docx", "<p>正文</p>", "7");
    expect(projectDocumentMediaDisplaySrc("x.draft.docx", "media/new.png"))
      .toBe("asset://localhost//cache/8/media/new.png");
  });

  it("受控导入返回稳定项目身份、媒体引用与仅用于显示的 asset URL", async () => {
    const hash = "a".repeat(64);
    invokeMock.mockResolvedValue({
      correlation_id: "c",
      result: {
        assetId: "asset_1",
        sourcePath: "images/imported/photo-aaaa.jpg",
        sourceHash: hash,
        mediaType: "image/jpeg",
        byteSize: 1024,
        mediaRef: `media/binder-project-${hash}.jpg`,
        mediaBase: "/cache/15",
        baseline: "15",
      },
    });

    await expect(importProjectImage(
      "x.draft.docx",
      "/Users/demo/Desktop/photo.jpg",
      "15",
    )).resolves.toEqual({
      assetId: "asset_1",
      sourcePath: "images/imported/photo-aaaa.jpg",
      sourceHash: hash,
      mediaType: "image/jpeg",
      byteSize: 1024,
      mediaRef: `media/binder-project-${hash}.jpg`,
      displaySrc: `asset://localhost//cache/15/media/binder-project-${hash}.jpg`,
    });
    expect(invokeMock).toHaveBeenCalledWith("dispatch", expect.objectContaining({
      request: expect.objectContaining({
        command: "doc.import_project_image",
        params: {
          relPath: "x.draft.docx",
          sourcePath: "/Users/demo/Desktop/photo.jpg",
          baseline: "15",
        },
      }),
    }));
  });

  it("剪贴板图片以 data URL 进入同一受控导入 command", async () => {
    const hash = "b".repeat(64);
    invokeMock.mockResolvedValue({
      correlation_id: "c",
      result: {
        assetId: "asset_clipboard",
        sourcePath: "images/imported/clipboard-bbbb.png",
        sourceHash: hash,
        mediaType: "image/png",
        byteSize: 68,
        mediaRef: `media/binder-project-${hash}.png`,
        mediaBase: "/cache/16",
        baseline: "16",
      },
    });

    await expect(importProjectImageData(
      "x.draft.docx",
      "clipboard.png",
      "data:image/png;base64,iVBORw0KGgo=",
      "16",
    )).resolves.toEqual(expect.objectContaining({
      assetId: "asset_clipboard",
      sourcePath: "images/imported/clipboard-bbbb.png",
      displaySrc: `asset://localhost//cache/16/media/binder-project-${hash}.png`,
    }));
    expect(invokeMock).toHaveBeenCalledWith("dispatch", expect.objectContaining({
      request: expect.objectContaining({
        command: "doc.import_project_image",
        params: {
          relPath: "x.draft.docx",
          sourceName: "clipboard.png",
          sourceDataUrl: "data:image/png;base64,iVBORw0KGgo=",
          baseline: "16",
        },
      }),
    }));
  });
});

describe("docService · DOCX 样式报告", () => {
  it("打开结果透传同次转换的样式报告", async () => {
    invokeMock.mockResolvedValue({
      correlation_id: "c",
      result: {
        format: "html",
        html: "<p><span style=\"color:#FF0000\">红字</span></p>",
        baseline: "9",
        style_report: { notices: ["样式已检测"], verified: ["text_color"] },
      },
    });
    await expect(openDocxForEdit("x.draft.docx")).resolves.toMatchObject({
      baseline: "9",
      styleReport: { verified: ["text_color"] },
    });
    expect(invokeMock).toHaveBeenCalledWith("dispatch", expect.objectContaining({
      request: expect.objectContaining({ command: "doc.open_for_edit" }),
    }));
  });

  it("独立样式报告调用已注册 command", async () => {
    invokeMock.mockResolvedValue({ correlation_id: "c", result: { style_report: { notices: ["风险"] } } });
    await expect(getDocxStyleReport("x.draft.docx")).resolves.toMatchObject({ notices: ["风险"] });
    expect(invokeMock).toHaveBeenCalledWith("dispatch", expect.objectContaining({
      request: expect.objectContaining({ command: "doc.get_style_report", params: { relPath: "x.draft.docx" } }),
    }));
  });
});
