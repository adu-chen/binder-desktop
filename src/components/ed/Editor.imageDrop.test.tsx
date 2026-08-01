/** @vitest-environment jsdom */
import { describe, expect, it, vi } from "vitest";
import type { Editor } from "@tiptap/core";
import {
  clipboardImageFiles,
  clipboardImageDataSourcesFromHtml,
  imageDropAnchorAtClientPoint,
  imagePasteAnchorAtSelection,
  imageDropTransferKind,
  isManagedImageClipboardHtml,
  selectionContainsImage,
} from "./Editor";
import { DOC_REF_MIME } from "../../services/edLocate";

// covers: BR-ED-DATA-001, BR-ED-DATA-003
describe("Editor · BDocx local image drop anchor", () => {
  it("按命中顶层 Block 的视觉中线生成 before/after 稳定锚", () => {
    const block = document.createElement("p");
    block.getBoundingClientRect = () => ({
      left: 20,
      right: 420,
      top: 100,
      bottom: 200,
      width: 400,
      height: 100,
      x: 20,
      y: 100,
      toJSON: () => ({}),
    });
    const node = { nodeSize: 10, attrs: { blockId: "b1" } };
    const editor = {
      view: {
        posAtCoords: vi.fn(() => ({ pos: 5, inside: 0 })),
        nodeDOM: vi.fn(() => block),
      },
      state: {
        doc: {
          forEach: (callback: (value: unknown, offset: number) => void) => callback(node, 0),
        },
      },
    } as unknown as Editor;

    expect(imageDropAnchorAtClientPoint(editor, 100, 120)).toEqual({
      anchorBlockId: "b1",
      position: "before",
    });
    expect(imageDropAnchorAtClientPoint(editor, 100, 180)).toEqual({
      anchorBlockId: "b1",
      position: "after",
    });
  });

  it("无坐标命中或命中块缺稳定 blockId 时拒绝插入", () => {
    const editor = {
      view: { posAtCoords: () => null, nodeDOM: () => null },
      state: { doc: { forEach: vi.fn() } },
    } as unknown as Editor;
    expect(imageDropAnchorAtClientPoint(editor, 0, 0)).toBeNull();
  });

  it("纸张空白或分页间隙按纵向距离降级到最近稳定 Block", () => {
    const first = document.createElement("p");
    const second = document.createElement("p");
    first.getBoundingClientRect = () => ({
      left: 20, right: 420, top: 100, bottom: 140, width: 400, height: 40, x: 20, y: 100,
      toJSON: () => ({}),
    });
    second.getBoundingClientRect = () => ({
      left: 20, right: 420, top: 300, bottom: 340, width: 400, height: 40, x: 20, y: 300,
      toJSON: () => ({}),
    });
    const nodes = [
      { nodeSize: 10, attrs: { blockId: "b1" } },
      { nodeSize: 10, attrs: { blockId: "b2" } },
    ];
    const editor = {
      view: {
        posAtCoords: () => null,
        nodeDOM: (offset: number) => (offset === 0 ? first : second),
      },
      state: {
        doc: {
          forEach: (callback: (node: unknown, offset: number) => void) => {
            callback(nodes[0], 0);
            callback(nodes[1], 10);
          },
        },
      },
    } as unknown as Editor;

    expect(imageDropAnchorAtClientPoint(editor, 600, 260)).toEqual({
      anchorBlockId: "b2",
      position: "before",
    });
    expect(imageDropAnchorAtClientPoint(editor, 600, 500)).toEqual({
      anchorBlockId: "b2",
      position: "after",
    });
  });

  it("只接管项目 DocRef 与 Finder Files，富文本/文档内图片拖动交给 ProseMirror", () => {
    const transfer = (types: string[], files: File[] = []) => ({
      types,
      files,
      items: [],
    } as unknown as DataTransfer);

    expect(imageDropTransferKind(transfer([DOC_REF_MIME]))).toBe("project_image");
    expect(imageDropTransferKind(transfer(["Files"], [
      new File(["png"], "shot.png", { type: "image/png" }),
    ]))).toBe("local_files");
    expect(imageDropTransferKind(transfer(["text/html", "text/plain"]))).toBeNull();
  });

  it("剪贴板锚使用当前光标所在顶层 Block 之后", () => {
    const editor = {
      state: {
        selection: { head: 5 },
        doc: {
          forEach: (callback: (node: unknown, offset: number) => void) => {
            callback({ nodeSize: 10, attrs: { blockId: "b-current" } }, 0);
          },
        },
      },
    } as unknown as Editor;
    expect(imagePasteAnchorAtSelection(editor)).toEqual({
      anchorBlockId: "b-current",
      position: "after",
    });
  });

  it("只提取 PNG/JPEG 剪贴板文件并识别 Binder 自身图片 HTML", () => {
    const png = new File(["png"], "shot.png", { type: "image/png" });
    const jpeg = new File(["jpg"], "photo", { type: "image/jpeg" });
    const webp = new File(["webp"], "other.webp", { type: "image/webp" });
    const transfer = {
      files: [png, jpeg, webp],
      items: [],
    } as unknown as DataTransfer;
    expect(clipboardImageFiles(transfer)).toEqual([png, jpeg]);
    expect(isManagedImageClipboardHtml(
      '<p><img src="asset://localhost/a" data-binder-media-src="media/a.png"></p>',
    )).toBe(true);
    expect(isManagedImageClipboardHtml('<img src="https://example.com/a.png">')).toBe(false);
    expect(clipboardImageDataSourcesFromHtml(
      '<p><img src="data:image/png;base64,iVBORw0KGgo="></p>',
    )).toEqual([{
      sourceName: "clipboard-1.png",
      sourceDataUrl: "data:image/png;base64,iVBORw0KGgo=",
    }]);
    expect(clipboardImageDataSourcesFromHtml(
      '<p>说明<img src="data:image/png;base64,iVBORw0KGgo="></p>',
    )).toEqual([]);
    expect(clipboardImageDataSourcesFromHtml('<img src="https://example.com/a.png">')).toEqual([]);
  });

  it("含图片的选区由原生富剪贴板处理，不降级成 SelectionRef", () => {
    const doc = {
      nodesBetween: (_from: number, _to: number, callback: (node: unknown) => boolean | void) => {
        callback({ type: { name: "paragraph" } });
        callback({ type: { name: "image" } });
      },
    } as unknown as Parameters<typeof selectionContainsImage>[0];
    expect(selectionContainsImage(doc, 1, 2)).toBe(true);
  });
});
