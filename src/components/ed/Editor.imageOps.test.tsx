// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from "vitest";
import { Schema } from "@tiptap/pm/model";
import { EditorState, type Transaction } from "@tiptap/pm/state";
import type { Editor as TiptapEditor } from "@tiptap/core";
import { applyPendingImageDiffToEditor } from "./Editor";
import { projectDocumentImages } from "../../services/edImageOps";
import { deStore, type DiffView } from "../../store/deStore";
import { edStore } from "../../store/edStore";

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
