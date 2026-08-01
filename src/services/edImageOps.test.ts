import { describe, expect, it, vi } from "vitest";
import { Schema } from "@tiptap/pm/model";
import { EditorState, NodeSelection, type Transaction } from "@tiptap/pm/state";
import type { Editor } from "@tiptap/core";
import {
  activeImageSnapshot,
  applyImageOperations,
  deleteActiveImage,
  moveActiveImage,
  parseImageDiffSnapshotProjection,
  parseImageEditPayload,
  projectDocumentImages,
} from "./edImageOps";

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
    hardBreak: { inline: true, group: "inline", selectable: false },
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

function paragraph(blockId: string, ...content: Array<ReturnType<typeof schema.text> | ReturnType<typeof schema.nodes.image.create>>) {
  return schema.nodes.paragraph.create({ blockId }, content);
}

function image(imageId = "image_existing") {
  return schema.nodes.image.create({
    src: "asset://localhost/media/picture.png",
    alt: "插图",
    width: "160",
    height: null,
    binderMediaSrc: "media/picture.png",
    binderImageId: imageId,
  });
}

function imagePosition(state: EditorState): number {
  let found = -1;
  state.doc.descendants((node, pos) => {
    if (node.type.name === "image" && found < 0) found = pos;
  });
  return found;
}

function selectImage(state: EditorState): EditorState {
  const pos = imagePosition(state);
  return state.apply(state.tr.setSelection(NodeSelection.create(state.doc, pos)));
}

function fakeEditor(initial: EditorState): { editor: Editor; state: () => EditorState; focus: ReturnType<typeof vi.fn> } {
  let current = initial;
  const focus = vi.fn();
  const editor = {
    get state() {
      return current;
    },
    view: {
      dispatch(transaction: Transaction) {
        current = current.apply(transaction);
      },
      focus,
    },
  } as unknown as Editor;
  return { editor, state: () => current, focus };
}

function standaloneState() {
  return selectImage(EditorState.create({
    schema,
    doc: schema.nodes.doc.create(null, [
      paragraph("before", schema.text("前文")),
      paragraph("image-block", image()),
      paragraph("after", schema.text("后文")),
    ]),
  }));
}

describe("edImageOps · 图片文档流操作", () => {
  it("独立图片段落可上下移动，并保持 Block、媒体属性和图片 NodeSelection", () => {
    const runtime = fakeEditor(standaloneState());
    expect(activeImageSnapshot(runtime.editor)).toMatchObject({
      blockId: "image-block",
      standalone: true,
      moveDirections: ["up", "down"],
    });

    expect(moveActiveImage(runtime.editor, "up")).toBe(true);
    expect(runtime.state().doc.child(0).attrs.blockId).toBe("image-block");
    expect(runtime.state().doc.child(0).firstChild?.attrs).toMatchObject({
      src: "asset://localhost/media/picture.png",
      binderMediaSrc: "media/picture.png",
      width: "160",
    });
    expect(runtime.state().selection).toBeInstanceOf(NodeSelection);
    expect((runtime.state().selection as NodeSelection).node.type.name).toBe("image");
    expect(activeImageSnapshot(runtime.editor)?.moveDirections).toEqual(["down"]);
    expect(runtime.focus).toHaveBeenCalled();
  });

  it("Pandoc 图片段落中的空白和换行不应把上下移动误判为混排", () => {
    const initial = selectImage(EditorState.create({
      schema,
      doc: schema.nodes.doc.create(null, [
        paragraph("before", schema.text("前文")),
        paragraph(
          "image-block",
          schema.text(" "),
          image(),
          schema.nodes.hardBreak.create(),
          schema.text("\t"),
        ),
        paragraph("after", schema.text("后文")),
      ]),
    }));
    const runtime = fakeEditor(initial);

    expect(activeImageSnapshot(runtime.editor)).toMatchObject({
      blockId: "image-block",
      standalone: true,
      moveDirections: ["up", "down"],
    });
    expect(moveActiveImage(runtime.editor, "down")).toBe(true);
    expect(runtime.state().doc.child(2).attrs.blockId).toBe("image-block");
  });

  it("混排文字中的图片没有块移动方向，移动拒绝且不会改写同段文字", () => {
    const initial = selectImage(EditorState.create({
      schema,
      doc: schema.nodes.doc.create(null, [
        paragraph("mixed", schema.text("前"), image(), schema.text("后")),
      ]),
    }));
    const runtime = fakeEditor(initial);
    expect(activeImageSnapshot(runtime.editor)).toMatchObject({
      blockId: "mixed",
      standalone: false,
      moveDirections: [],
    });
    expect(moveActiveImage(runtime.editor, "down")).toBe(false);
    expect(runtime.state().doc.eq(initial.doc)).toBe(true);

    expect(deleteActiveImage(runtime.editor)).toBe(true);
    expect(runtime.state().doc.firstChild?.textContent).toBe("前后");
    expect(runtime.state().doc.firstChild?.attrs.blockId).toBe("mixed");
  });

  it("删除独立图片会删除对应段落；文档只有该图片时保留可编辑空段落", () => {
    const runtime = fakeEditor(standaloneState());
    expect(deleteActiveImage(runtime.editor)).toBe(true);
    expect(runtime.state().doc.childCount).toBe(2);
    expect(runtime.state().doc.toJSON()).not.toContainEqual(expect.objectContaining({ attrs: expect.objectContaining({ blockId: "image-block" }) }));
    expect(imagePosition(runtime.state())).toBe(-1);

    const only = fakeEditor(selectImage(EditorState.create({
      schema,
      doc: schema.nodes.doc.create(null, [paragraph("only", image())]),
    })));
    expect(deleteActiveImage(only.editor)).toBe(true);
    expect(only.state().doc.childCount).toBe(1);
    expect(only.state().doc.firstChild?.type.name).toBe("paragraph");
    expect(only.state().doc.firstChild?.childCount).toBe(0);
  });
});

describe("edImageOps · ImageOperation", () => {
  it("投影稳定图片实例并以单事务插入项目图片独立段落", () => {
    const runtime = fakeEditor(EditorState.create({
      schema,
      doc: schema.nodes.doc.create(null, [
        paragraph("anchor", schema.text("锚点")),
        paragraph("existing", image()),
      ]),
    }));
    const before = projectDocumentImages(runtime.editor);
    expect(before).toHaveLength(1);
    expect(before[0]).toMatchObject({
      imageId: "image_existing",
      blockId: "existing",
      standalone: true,
      mediaRef: "media/picture.png",
    });

    const result = applyImageOperations(runtime.editor, {
      base: { sessionVersion: 2, contentHash: "h2" },
      operations: [{
        op: "insert_image",
        anchorBlockId: "anchor",
        position: "after",
        assetId: "asset_cover",
        sourcePath: "images/cover.png",
        sourceHash: "a".repeat(64),
        mediaRef: `media/binder-project-${"a".repeat(64)}.png`,
        displaySrc: "asset://localhost/media/cover.png",
        alt: "封面",
        width: 320,
      }],
    });
    expect(result).not.toBeNull();
    expect(result?.before).toHaveLength(1);
    expect(result?.after).toHaveLength(2);
    expect(runtime.state().doc.child(1).type.name).toBe("paragraph");
    expect(runtime.state().doc.child(1).childCount).toBe(1);
    expect(runtime.state().doc.child(1).firstChild?.attrs).toMatchObject({
      src: "asset://localhost/media/cover.png",
      alt: "封面",
      width: "320",
      binderProjectSrc: "images/cover.png",
      binderProjectHash: "a".repeat(64),
    });
    expect(result?.affectedBlockIds).toContain("anchor");
  });

  it("按 imageId/fingerprint 删除文档节点且非法批次不产生部分应用", () => {
    const runtime = fakeEditor(EditorState.create({
      schema,
      doc: schema.nodes.doc.create(null, [
        paragraph("before", schema.text("前文")),
        paragraph("image-block", image()),
        paragraph("after", schema.text("后文")),
      ]),
    }));
    const target = projectDocumentImages(runtime.editor)[0];
    const valid = applyImageOperations(runtime.editor, {
      base: { sessionVersion: 2, contentHash: "h2" },
      operations: [{
        op: "delete_image",
        imageId: target.imageId,
        fingerprint: target.fingerprint,
      }],
    });
    expect(valid?.after).toHaveLength(0);
    expect(runtime.state().doc.childCount).toBe(2);

    const second = fakeEditor(EditorState.create({
      schema,
      doc: schema.nodes.doc.create(null, [
        paragraph("anchor", schema.text("锚点")),
        paragraph("image-block", image()),
      ]),
    }));
    const original = second.state().doc;
    const stale = projectDocumentImages(second.editor)[0];
    expect(applyImageOperations(second.editor, {
      base: { sessionVersion: 2, contentHash: "h2" },
      operations: [
        {
          op: "insert_image",
          anchorBlockId: "anchor",
          position: "after",
          assetId: "asset_cover",
          sourcePath: "images/cover.png",
          sourceHash: "a".repeat(64),
          mediaRef: "media/cover.png",
        },
        {
          op: "delete_image",
          imageId: stale.imageId,
          fingerprint: "stale",
        },
      ],
    })).toBeNull();
    expect(second.state().doc.eq(original)).toBe(true);
  });

  it("严格解析来源路径、hash、宽度和受支持操作", () => {
    const valid = parseImageEditPayload({
      base: { sessionVersion: 1, contentHash: "h1" },
      operations: [{
        op: "insert_image",
        anchorBlockId: "b1",
        position: "before",
        assetId: "asset_a",
        sourcePath: "images/a.jpg",
        sourceHash: "b".repeat(64),
        mediaRef: `media/binder-project-${"b".repeat(64)}.jpg`,
        width: 240,
      }],
    });
    expect(valid?.operations[0].op).toBe("insert_image");
    expect(parseImageEditPayload({
      base: { sessionVersion: 1, contentHash: "h1" },
      operations: [{
        op: "insert_image",
        anchorBlockId: "b1",
        position: "before",
        assetId: "asset_a",
        sourcePath: "../a.jpg",
        sourceHash: "b".repeat(64),
        mediaRef: "media/a.jpg",
      }],
    })).toBeNull();
    expect(parseImageEditPayload({
      base: { sessionVersion: 1, contentHash: "h1" },
      operations: [{
        op: "insert_image",
        anchorBlockId: "b1",
        position: "before",
        assetId: "asset_a",
        sourcePath: "images/a.jpg",
        sourceHash: "b".repeat(64),
        mediaRef: `media/binder-project-${"c".repeat(64)}.jpg`,
      }],
    })).toBeNull();
    expect(parseImageEditPayload({
      base: { sessionVersion: 1, contentHash: "h1" },
      operations: [{ op: "move_image", imageId: "i1" }],
    })).toBeNull();
  });

  it("稳定 imageId，按当前 fingerprint 原子缩放、移动和设置独立段落对齐", () => {
    const runtime = fakeEditor(EditorState.create({
      schema,
      doc: schema.nodes.doc.create(null, [
        paragraph("before", schema.text("前文")),
        paragraph("image-block", image("image_stable")),
        paragraph("after", schema.text("后文")),
      ]),
    }));
    const initial = projectDocumentImages(runtime.editor)[0];
    expect(initial).toMatchObject({
      imageId: "image_stable",
      alignment: "left",
      capabilities: ["delete_image", "resize_image", "set_image_layout", "move_image"],
      moveDirections: ["up", "down"],
      resizeBounds: { min: 40, max: 4096 },
    });

    const resized = applyImageOperations(runtime.editor, {
      base: { sessionVersion: 3, contentHash: "h3" },
      operations: [{
        op: "resize_image",
        imageId: initial.imageId,
        fingerprint: initial.fingerprint,
        width: 240,
      }],
    });
    expect(resized?.after[0]).toMatchObject({ imageId: "image_stable", width: "240" });
    expect(resized?.after[0].fingerprint).not.toBe(initial.fingerprint);

    const afterResize = resized!.after[0];
    const moved = applyImageOperations(runtime.editor, {
      base: { sessionVersion: 3, contentHash: "h3" },
      operations: [{
        op: "move_image",
        imageId: afterResize.imageId,
        fingerprint: afterResize.fingerprint,
        direction: "down",
      }],
    });
    expect(moved?.after[0]).toMatchObject({
      imageId: "image_stable",
      ordinal: 0,
      moveDirections: ["up"],
    });
    expect(runtime.state().doc.child(2).attrs.blockId).toBe("image-block");

    const afterMove = moved!.after[0];
    const aligned = applyImageOperations(runtime.editor, {
      base: { sessionVersion: 3, contentHash: "h3" },
      operations: [{
        op: "set_image_layout",
        imageId: afterMove.imageId,
        fingerprint: afterMove.fingerprint,
        alignment: "center",
      }],
    });
    expect(aligned?.after[0]).toMatchObject({
      imageId: "image_stable",
      alignment: "center",
    });
    expect(runtime.state().doc.child(2).attrs.textAlign).toBe("center");
  });

  it("同一原始 fingerprint 可在单事务中缩放、对齐并移动同一图片", () => {
    const runtime = fakeEditor(EditorState.create({
      schema,
      doc: schema.nodes.doc.create(null, [
        paragraph("before", schema.text("前文")),
        paragraph("image-block", image("image_composite")),
        paragraph("after", schema.text("后文")),
      ]),
    }));
    const target = projectDocumentImages(runtime.editor)[0];
    const applied = applyImageOperations(runtime.editor, {
      base: { sessionVersion: 4, contentHash: "h4" },
      operations: [
        {
          op: "move_image",
          imageId: target.imageId,
          fingerprint: target.fingerprint,
          direction: "down",
        },
        {
          op: "set_image_layout",
          imageId: target.imageId,
          fingerprint: target.fingerprint,
          alignment: "center",
        },
        {
          op: "resize_image",
          imageId: target.imageId,
          fingerprint: target.fingerprint,
          width: 240,
        },
      ],
    });

    expect(applied?.after[0]).toMatchObject({
      imageId: "image_composite",
      width: "240",
      alignment: "center",
      moveDirections: ["up"],
    });
    expect(runtime.state().doc.child(2).attrs).toMatchObject({
      blockId: "image-block",
      textAlign: "center",
    });
  });

  it("拒绝过期 fingerprint、越界宽度、不可用方向、重复操作和 delete 组合且不部分应用", () => {
    const runtime = fakeEditor(EditorState.create({
      schema,
      doc: schema.nodes.doc.create(null, [
        paragraph("image-block", image("image_stable")),
        paragraph("after", schema.text("后文")),
      ]),
    }));
    const original = runtime.state().doc;
    const target = projectDocumentImages(runtime.editor)[0];
    expect(applyImageOperations(runtime.editor, {
      base: { sessionVersion: 4, contentHash: "h4" },
      operations: [{
        op: "move_image",
        imageId: target.imageId,
        fingerprint: target.fingerprint,
        direction: "up",
      }],
    })).toBeNull();
    expect(applyImageOperations(runtime.editor, {
      base: { sessionVersion: 4, contentHash: "h4" },
      operations: [{
        op: "resize_image",
        imageId: target.imageId,
        fingerprint: target.fingerprint,
        width: 4097,
      }],
    })).toBeNull();
    expect(applyImageOperations(runtime.editor, {
      base: { sessionVersion: 4, contentHash: "h4" },
      operations: [
        {
          op: "resize_image",
          imageId: target.imageId,
          fingerprint: target.fingerprint,
          width: 200,
        },
        {
          op: "delete_image",
          imageId: target.imageId,
          fingerprint: target.fingerprint,
        },
      ],
    })).toBeNull();
    expect(applyImageOperations(runtime.editor, {
      base: { sessionVersion: 4, contentHash: "h4" },
      operations: [
        {
          op: "resize_image",
          imageId: target.imageId,
          fingerprint: target.fingerprint,
          width: 200,
        },
        {
          op: "resize_image",
          imageId: target.imageId,
          fingerprint: target.fingerprint,
          width: 220,
        },
      ],
    })).toBeNull();
    expect(runtime.state().doc.eq(original)).toBe(true);
  });

  // covers: BR-DE-DATA-002
  it("严格恢复持久化的前后图片快照并拒绝路径漂移或部分字段", () => {
    const snapshot = {
      imageId: "image_0_abcd",
      fingerprint: "abcd",
      ordinal: 0,
      blockId: "b-img",
      imagePos: 2,
      standalone: true,
      alt: "项目图",
      width: "240",
      height: null,
      mediaRef: "media/image.png",
      projectSourcePath: "images/image.png",
      projectSourceHash: "a".repeat(64),
    };
    expect(parseImageDiffSnapshotProjection(
      [snapshot],
      [],
      ["b-img"],
    )).toEqual({
      before: [{
        ...snapshot,
        alignment: "left",
        capabilities: ["delete_image", "resize_image", "set_image_layout"],
        resizeBounds: { min: 40, max: 4096 },
        moveDirections: [],
      }],
      after: [],
      affectedBlockIds: ["b-img"],
    });
    expect(parseImageDiffSnapshotProjection(
      [{ ...snapshot, projectSourcePath: "../outside.png" }],
      [],
      ["b-img"],
    )).toBeNull();
    expect(parseImageDiffSnapshotProjection(
      [{ ...snapshot, mediaRef: undefined }],
      [],
      ["b-img"],
    )).toBeNull();
    expect(parseImageDiffSnapshotProjection(
      [snapshot],
      [],
      ["b-img", "b-img"],
    )).toBeNull();
  });
});
