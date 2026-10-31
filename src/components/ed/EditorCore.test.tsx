/** @vitest-environment jsdom */
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Editor } from "@tiptap/core";
import type { Editor as TiptapEditor } from "@tiptap/core";
import { Schema } from "@tiptap/pm/model";
import { EditorState, type Transaction } from "@tiptap/pm/state";
import {
  applyPendingImageDiffToEditor,
  clipboardImageDataSourcesFromHtml,
  clipboardImageFiles,
  imageDropAnchorAtClientPoint,
  imageDropTransferKind,
  imagePasteAnchorAtSelection,
  isEditorContentReady,
  isManagedImageClipboardHtml,
  selectionContainsImage,
} from "./Editor";
import {
  applyAssistAdoption,
  type AssistAdoptionDeps,
  type AssistAdoptionTarget,
} from "./assistAdoption";
import { DOC_REF_MIME } from "../../services/edLocate";
import { projectDocumentImages } from "../../services/edImageOps";
import { deStore, type DiffView } from "../../store/deStore";
import { edStore } from "../../store/edStore";

// Migrated intact from src/components/ed/Editor.imageDrop.test.tsx
{
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
}
// Migrated intact from src/components/ed/Editor.imageOps.test.tsx
{
// covers: BR-DE-DATA-002
// covers: BR-ED-DATA-001
// covers: BR-ED-DATA-003
// covers: BR-ED-STATE-001
const schema = new Schema({
  nodes: {
    doc: { content: "block+" },
    paragraph: {
      group: "block",
      content: "inline*",
      attrs: { blockId: { default: null }, textAlign: { default: null } },
    },
    text: { group: "inline" },
    image: {
      inline: true,
      group: "inline",
      atom: true,
      attrs: {
        src: {},
        alt: { default: null },
        width: { default: null },
        height: { default: null },
        binderMediaSrc: { default: null },
        binderProjectSrc: { default: null },
        binderProjectHash: { default: null },
        binderImageId: { default: null },
        display: { default: "inline" },
      },
    },
  },
});

function paragraph(
  blockId: string,
  ...content: Array<ReturnType<typeof schema.text> | ReturnType<typeof schema.nodes.image.create>>
) {
  return schema.nodes.paragraph.create({ blockId, textAlign: null }, content);
}

function image(imageId: string, mediaName: string) {
  return schema.nodes.image.create({
    src: `asset://localhost/media/${mediaName}.png`,
    alt: mediaName,
    width: "160",
    height: null,
    binderMediaSrc: `media/${mediaName}.png`,
    binderImageId: imageId,
  });
}

function fakeEditor(
  initial: EditorState,
  onDispatch?: (state: EditorState) => void,
): { editor: TiptapEditor; state: () => EditorState } {
  let current = initial;
  const editor = {
    get state() {
      return current;
    },
    view: {
      dispatch(transaction: Transaction) {
        current = current.apply(transaction);
        onDispatch?.(current);
      },
    },
  } as unknown as TiptapEditor;
  return { editor, state: () => current };
}

function pendingImageDiff(
  sessionVersion: number,
  contentHash: string,
  operations: NonNullable<DiffView["imageOpsPayload"]>["operations"],
): DiffView {
  return {
    diffId: "diff-image-combined",
    blockId: "",
    relPath: "notes/current.draft.docx",
    title: "组合调整图片",
    before: "图片操作前",
    after: "图片操作后",
    state: "pending",
    patchKind: "image_ops",
    imageOpsPayload: {
      base: { sessionVersion, contentHash },
      operations,
    },
  };
}

beforeEach(() => {
  deStore.reset();
  edStore.reset();
});

describe("Editor pending ImageOperation 闭环", () => {
  it("同一图片组合缩放、对齐、移动只应用一次原始快照并生成一张完整 Diff 快照", () => {
    const relPath = "notes/current.draft.docx";
    const baseContent = "<p>原始图片状态</p>";
    edStore.loadDoc(relPath, baseContent, 101, null, "html", "html");
    const session = edStore.getSnapshot().sessions[relPath];
    const runtime = fakeEditor(EditorState.create({
      schema,
      doc: schema.nodes.doc.create(null, [
        paragraph("intro", schema.text("第一条")),
        paragraph("image-block", image("image-one", "one")),
        paragraph("outro", schema.text("第二条")),
      ]),
    }), () => edStore.markDirty());
    const target = projectDocumentImages(runtime.editor)[0];
    const diff = pendingImageDiff(session.sessionVersion, session.contentHash, [
      { op: "move_image", imageId: target.imageId, fingerprint: target.fingerprint, direction: "down" },
      { op: "set_image_layout", imageId: target.imageId, fingerprint: target.fingerprint, alignment: "center" },
      { op: "resize_image", imageId: target.imageId, fingerprint: target.fingerprint, width: 240 },
    ]);
    deStore.addDiff(diff);
    const persistSnapshots = vi.fn().mockResolvedValue(undefined);

    const applied = applyPendingImageDiffToEditor(
      runtime.editor,
      diff,
      session,
      baseContent,
      "saved",
      () => "<p>组合操作后</p>",
      persistSnapshots,
    );

    expect(applied?.after[0]).toMatchObject({
      imageId: "image-one",
      width: "240",
      alignment: "center",
      moveDirections: ["up"],
    });
    expect(runtime.state().doc.child(2).attrs).toMatchObject({
      blockId: "image-block",
      textAlign: "center",
    });
    expect(deStore.getSnapshot().diffs[0]).toMatchObject({
      state: "pending",
      draftApplied: true,
    });
    expect(persistSnapshots).toHaveBeenCalledOnce();
  });

  it("按当前 session 应用组合操作，建立拒绝基线并持久化精确前后快照", () => {
    const relPath = "notes/current.draft.docx";
    const baseContent = "<p>图片组合操作前</p>";
    const afterContent = "<p>图片组合操作后</p>";
    edStore.loadDoc(relPath, baseContent, 101, null, "html", "html");
    const baseSession = edStore.getSnapshot().sessions[relPath];
    const runtime = fakeEditor(EditorState.create({
      schema,
      doc: schema.nodes.doc.create(null, [
        paragraph("intro", schema.text("前文")),
        paragraph("resize-block", image("image-resize", "resize")),
        paragraph("move-block", image("image-move", "move")),
        paragraph("spacer", schema.text("间隔")),
        paragraph("align-block", image("image-align", "align")),
        paragraph("outro", schema.text("后文")),
      ]),
    }), () => edStore.markDirty());
    const initialImages = projectDocumentImages(runtime.editor);
    const resizeTarget = initialImages.find((entry) => entry.imageId === "image-resize")!;
    const moveTarget = initialImages.find((entry) => entry.imageId === "image-move")!;
    const alignTarget = initialImages.find((entry) => entry.imageId === "image-align")!;
    const diff = pendingImageDiff(baseSession.sessionVersion, baseSession.contentHash, [
      {
        op: "resize_image",
        imageId: resizeTarget.imageId,
        fingerprint: resizeTarget.fingerprint,
        width: 240,
      },
      {
        op: "move_image",
        imageId: moveTarget.imageId,
        fingerprint: moveTarget.fingerprint,
        direction: "down",
      },
      {
        op: "set_image_layout",
        imageId: alignTarget.imageId,
        fingerprint: alignTarget.fingerprint,
        alignment: "right",
      },
    ]);
    deStore.addDiff(diff);
    const persistSnapshots = vi.fn().mockResolvedValue(undefined);

    const applied = applyPendingImageDiffToEditor(
      runtime.editor,
      diff,
      baseSession,
      baseContent,
      "saved",
      () => afterContent,
      persistSnapshots,
    );

    expect(applied).not.toBeNull();
    expect(runtime.state().doc.child(1).firstChild?.attrs.width).toBe("240");
    expect(runtime.state().doc.child(2).attrs.blockId).toBe("spacer");
    expect(runtime.state().doc.child(3).attrs.blockId).toBe("move-block");
    expect(runtime.state().doc.child(4).attrs).toMatchObject({ blockId: "align-block", textAlign: "right" });
    expect(projectDocumentImages(runtime.editor).map((entry) => entry.imageId)).toEqual([
      "image-resize",
      "image-move",
      "image-align",
    ]);
    expect(edStore.getSnapshot().tabState).toBe("dirty");

    const stored = deStore.getSnapshot().diffs[0];
    expect(stored).toMatchObject({
      state: "pending",
      draftApplied: true,
      draftBaseContent: baseContent,
      draftBaseTabState: "saved",
      draftBaseDirty: false,
      draftAfterContent: afterContent,
    });
    expect(stored.imageBefore).toHaveLength(3);
    expect(stored.imageAfter).toHaveLength(3);
    expect(stored.imageAfter?.find((entry) => entry.imageId === "image-resize")?.width).toBe("240");
    expect(stored.imageAfter?.find((entry) => entry.imageId === "image-align")?.alignment).toBe("right");
    expect(stored.imageAffectedBlockIds).toEqual(expect.arrayContaining([
      "resize-block",
      "move-block",
      "spacer",
      "align-block",
    ]));
    expect(persistSnapshots).toHaveBeenCalledOnce();
    expect(persistSnapshots).toHaveBeenCalledWith(
      diff.diffId,
      stored.imageBefore,
      stored.imageAfter,
      stored.imageAffectedBlockIds,
    );
  });

  it("sessionVersion/contentHash 过期时拒绝且不改变编辑器或持久化快照", () => {
    const relPath = "notes/current.draft.docx";
    edStore.loadDoc(relPath, "<p>当前内容</p>", 101, null, "html", "html");
    const session = edStore.getSnapshot().sessions[relPath];
    const runtime = fakeEditor(EditorState.create({
      schema,
      doc: schema.nodes.doc.create(null, [
        paragraph("image-block", image("image-stable", "stable")),
        paragraph("after", schema.text("后文")),
      ]),
    }));
    const originalDoc = runtime.state().doc;
    const target = projectDocumentImages(runtime.editor)[0];
    const diff = pendingImageDiff(session.sessionVersion + 1, session.contentHash, [{
      op: "resize_image",
      imageId: target.imageId,
      fingerprint: target.fingerprint,
      width: 240,
    }]);
    deStore.addDiff(diff);
    const persistSnapshots = vi.fn();

    expect(applyPendingImageDiffToEditor(
      runtime.editor,
      diff,
      session,
      "<p>当前内容</p>",
      "saved",
      () => "<p>不应生成</p>",
      persistSnapshots,
    )).toBeNull();

    expect(runtime.state().doc.eq(originalDoc)).toBe(true);
    expect(deStore.getSnapshot().diffs[0]).toMatchObject({
      state: "error",
      errorCode: "ED_IMAGE_SNAPSHOT_STALE",
    });
    expect(deStore.getSnapshot().diffs[0].draftApplied).not.toBe(true);
    expect(persistSnapshots).not.toHaveBeenCalled();
  });
});
}

// Migrated intact from src/components/ed/Editor.lifecycle.test.tsx
{
describe("Editor BDocx 首轮揭示门禁", () => {
  it("内容尚未到达时保持加载遮罩", () => {
    expect(isEditorContentReady(null, 7, 7)).toBe(false);
  });

  it("setContent 已完成但当前 loadSeq 尚未分页 settled 时不揭示", () => {
    expect(isEditorContentReady("<p>合同</p>", 8, 7)).toBe(false);
  });

  it("只有当前 loadSeq settled 后才揭示；0 不是有效载入", () => {
    expect(isEditorContentReady("<p>合同</p>", 8, 8)).toBe(true);
    expect(isEditorContentReady("<p>合同</p>", 0, 0)).toBe(false);
  });
});
}

// AG-ASSIST 采纳落地；拆到纯函数 seam，避免 Tiptap DOM 渲染。
{
// covers: BR-AG-STATE-003
describe("AG-ASSIST 采纳落地", () => {
  const target: AssistAdoptionTarget = { blockId: "block-1", start: 2, end: 5 };

  function adoptionDependencies(overrides: Partial<AssistAdoptionDeps> = {}): AssistAdoptionDeps {
    return {
      editorReady: true,
      targetCurrent: true,
      snapshot: { kind: "inline", requestId: "assist-1", target },
      close: vi.fn(),
      apply: vi.fn(() => ({ ok: true })),
      finishRequest: vi.fn(),
      finishCurrent: vi.fn(),
      ...overrides,
    };
  }

  it("成功时经 ED apply 恰好一次并回写 accept", () => {
    const deps = adoptionDependencies();
    expect(applyAssistAdoption("新文本", deps)).toBe("accepted");
    expect(deps.close).toHaveBeenCalledTimes(1);
    expect(deps.apply).toHaveBeenCalledTimes(1);
    expect(deps.apply).toHaveBeenCalledWith(target, "新文本");
    expect(deps.finishRequest).toHaveBeenCalledTimes(1);
    expect(deps.finishRequest).toHaveBeenCalledWith("assist-1", "accept");
    expect(deps.finishCurrent).not.toHaveBeenCalled();
  });

  it("ED apply 失败时回写 anchor_lost，不伪装 accept", () => {
    const deps = adoptionDependencies({ apply: vi.fn(() => ({ ok: false })) });
    expect(applyAssistAdoption("新文本", deps)).toBe("anchor_lost");
    expect(deps.apply).toHaveBeenCalledTimes(1);
    expect(deps.finishRequest).toHaveBeenCalledWith("assist-1", "anchor_lost");
    expect(deps.finishRequest).not.toHaveBeenCalledWith("assist-1", "accept");
  });

  it("锚失效时关闭当前 assist，不执行 ED apply", () => {
    const deps = adoptionDependencies({ targetCurrent: false });
    expect(applyAssistAdoption("新文本", deps)).toBe("anchor_lost");
    expect(deps.finishCurrent).toHaveBeenCalledWith("anchor_lost");
    expect(deps.close).not.toHaveBeenCalled();
    expect(deps.apply).not.toHaveBeenCalled();
    expect(deps.finishRequest).not.toHaveBeenCalled();
  });

  it("编辑器缺失时同样不执行 ED apply", () => {
    const deps = adoptionDependencies({ editorReady: false });
    expect(applyAssistAdoption("新文本", deps)).toBe("anchor_lost");
    expect(deps.finishCurrent).toHaveBeenCalledWith("anchor_lost");
    expect(deps.apply).not.toHaveBeenCalled();
  });
});
}
