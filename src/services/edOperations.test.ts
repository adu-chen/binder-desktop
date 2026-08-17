/** @vitest-environment jsdom */
import { describe, expect, it, vi } from "vitest";
import { Editor } from "@tiptap/core";
import { Schema } from "@tiptap/pm/model";
import {
  EditorState,
  NodeSelection,
  Selection,
  type Transaction,
} from "@tiptap/pm/state";
import { TableMap, tableNodes } from "@tiptap/pm/tables";
import { buildEditorExtensions } from "../components/ed/extensions";
import type { StructuredBlockEditPayload, StructuredBlockOperation } from "../store/edStore";
import {
  applyStructuredBlockOperations,
  fingerprintBlockNode,
  projectAgentEditableBlocks,
  validateStructuredBlockOperations,
} from "./edBlockOps";
import {
  activeImageSnapshot,
  applyImageOperations,
  deleteActiveImage,
  moveActiveImage,
  parseImageDiffSnapshotProjection,
  parseImageEditPayload,
  projectDocumentImages,
} from "./edImageOps";
import {
  extractBlockStyles,
  projectDocStyles,
  parseStyleOps,
  posFromCharOffset,
  planStyleOps,
  type StyleOp,
} from "./edStyleOps";
import { registerEditor } from "./edLocate";
import {
  activeTableSnapshot,
  moveActiveTable,
  parseTableEditPayload,
  planTableOperations,
  projectTables,
} from "./edTableOps";

// Migrated intact from src/services/edBlockOps.test.ts
{
// covers: BR-ED-DATA-001
// covers: BR-ED-STATE-001
// covers: BR-DE-DATA-001
// covers: BR-DE-DATA-002

function editorWith(content: string): Editor {
  return new Editor({ extensions: buildEditorExtensions(), content });
}

function payloadFor(
  editor: Editor,
  blockId: string,
  afterText: string,
): StructuredBlockEditPayload {
  const block = projectAgentEditableBlocks(editor).find((item) => item.blockId === blockId);
  if (!block) throw new Error(`missing ${blockId}`);
  return {
    payloadVersion: 1,
    base: { sessionVersion: 7, contentHash: "base-hash" },
    operations: [{
      op: "replace_node",
      blockId,
      fingerprint: block.fingerprint,
      expectedNodeType: block.nodeType,
      beforeNode: block.nodeJson,
      afterNode: {
        ...block.nodeJson,
        content: [{ type: "text", text: afterText }],
      },
    }],
  };
}

function insertedHeading(
  op: "insert_before" | "insert_after",
  anchorId: string,
  blockId: string,
  text: string,
): StructuredBlockOperation {
  return {
    op,
    blockId: anchorId,
    fingerprint: "",
    expectedNodeType: "heading",
    beforeNode: {},
    afterNode: {
      type: "heading",
      attrs: { blockId, level: 2, textAlign: null },
      content: [{ type: "text", text }],
    },
  };
}

function deleteOperation(editor: Editor, blockId: string): StructuredBlockOperation {
  const block = projectAgentEditableBlocks(editor).find((item) => item.blockId === blockId);
  if (!block) throw new Error(`missing ${blockId}`);
  return {
    op: "delete_node",
    blockId,
    fingerprint: block.fingerprint,
    expectedNodeType: block.nodeType,
    beforeNode: block.nodeJson,
    afterNode: null,
  };
}

function appendParagraph(blockId: string, text: string): StructuredBlockOperation {
  return {
    op: "append",
    blockId: "",
    fingerprint: "",
    expectedNodeType: "paragraph",
    beforeNode: {},
    afterNode: {
      type: "paragraph",
      attrs: { blockId, textAlign: null },
      content: [{ type: "text", text }],
    },
  };
}

describe("projectAgentEditableBlocks", () => {
  it("projects list containers as readonly context and list paragraphs as independent writable blocks", () => {
    const editor = editorWith(`
      <ul data-block-id="list">
        <li data-block-id="item-1"><p data-block-id="p-1"><a href="#one">第一条</a></p></li>
        <li data-block-id="item-2"><p data-block-id="p-2">第二条</p></li>
      </ul>
    `);
    const blocks = projectAgentEditableBlocks(editor);
    expect(blocks.find((block) => block.blockId === "list")).toMatchObject({
      role: "container",
      capabilities: [],
    });
    expect(blocks.find((block) => block.blockId === "item-1")).toMatchObject({
      role: "container",
      parentBlockId: "list",
      capabilities: [],
    });
    expect(blocks.find((block) => block.blockId === "p-1")).toMatchObject({
      role: "content",
      parentBlockId: "item-1",
      text: "第一条",
      // AG-TASK-01 F2：可写 content 块附带 delete_node 能力。
      capabilities: ["replace_node", "style_ops", "delete_node"],
    });
    expect(blocks.find((block) => block.blockId === "p-2")?.blockIndex)
      .not.toBe(blocks.find((block) => block.blockId === "p-1")?.blockIndex);
    editor.destroy();
  });

  it("does not expose generic write capability for table or image-bearing blocks", () => {
    const editor = editorWith(`
      <table><tbody><tr><td><p data-block-id="cell-p">单元格</p></td></tr></tbody></table>
      <p data-block-id="image-p"><img src="project://image.png"></p>
    `);
    const blocks = projectAgentEditableBlocks(editor);
    expect(blocks.find((block) => block.blockId === "cell-p")?.capabilities).toEqual([]);
    expect(blocks.find((block) => block.blockId === "image-p")?.capabilities).toEqual([]);
    editor.destroy();
  });
});

describe("applyStructuredBlockOperations", () => {
  it("replaces two list-item paragraphs in one transaction without collapsing the list", () => {
    const editor = editorWith(`
      <ul data-block-id="list">
        <li data-block-id="item-1"><p data-block-id="p-1">第一条</p></li>
        <li data-block-id="item-2"><p data-block-id="p-2">第二条</p></li>
      </ul>
    `);
    const first = payloadFor(editor, "p-1", "新第一条");
    const second = payloadFor(editor, "p-2", "新第二条");
    const payload: StructuredBlockEditPayload = {
      ...first,
      operations: [...first.operations, ...second.operations],
    };
    const dispatch = editor.view.dispatch.bind(editor.view);
    let dispatchCount = 0;
    editor.view.dispatch = (transaction) => {
      dispatchCount += 1;
      dispatch(transaction);
    };
    expect(applyStructuredBlockOperations(
      editor,
      payload,
      "forward",
      { sessionVersion: 7, contentHash: "base-hash" },
    )).toEqual({ ok: true, affectedBlockIds: ["p-1", "p-2"] });
    expect(dispatchCount).toBe(1);
    expect(editor.state.doc.child(0).type.name).toBe("bulletList");
    expect(editor.state.doc.child(0).childCount).toBe(2);
    expect(editor.getText()).toContain("新第一条");
    expect(editor.getText()).toContain("新第二条");
    editor.destroy();
  });

  it("keeps each inserted heading and removes every old tail when insert_before and replace_node share anchors", () => {
    const editor = editorWith(`
      <p data-block-id="p-1">旧段一。残尾一</p>
      <p data-block-id="p-2">旧段二。残尾二</p>
      <p data-block-id="p-3">旧段三。残尾三</p>
      <p data-block-id="p-4">旧段四。残尾四</p>
    `);
    const before = editor.getJSON();
    const operations: StructuredBlockOperation[] = [];
    for (const [index, blockId] of ["p-1", "p-2", "p-3", "p-4"].entries()) {
      operations.push(
        insertedHeading("insert_before", blockId, `heading-${index + 1}`, `第 ${index + 1} 幕`),
        payloadFor(editor, blockId, `新正文${index + 1}`).operations[0],
      );
    }
    const payload: StructuredBlockEditPayload = {
      payloadVersion: 1,
      base: { sessionVersion: 7, contentHash: "base-hash" },
      operations,
    };

    expect(applyStructuredBlockOperations(
      editor,
      payload,
      "forward",
      { sessionVersion: 7, contentHash: "base-hash" },
    ).ok).toBe(true);
    const text = editor.getText();
    for (const index of [1, 2, 3, 4]) {
      expect(projectAgentEditableBlocks(editor).find((block) => block.blockId === `heading-${index}`)?.text)
        .toBe(`第 ${index} 幕`);
      expect(text).toContain(`新正文${index}`);
      expect(text).not.toContain(`残尾${index}`);
    }

    expect(validateStructuredBlockOperations(editor, payload, "reverse").ok).toBe(true);
    expect(applyStructuredBlockOperations(editor, payload, "reverse").ok).toBe(true);
    expect(editor.getJSON()).toEqual(before);
    editor.destroy();
  });

  it("keeps insert_after in reading order and reverses the actual inserted node", () => {
    const editor = editorWith('<p data-block-id="p-1">旧正文。残尾</p>');
    const before = editor.getJSON();
    const payload: StructuredBlockEditPayload = {
      payloadVersion: 1,
      base: { sessionVersion: 7, contentHash: "base-hash" },
      operations: [
        payloadFor(editor, "p-1", "新正文").operations[0],
        insertedHeading("insert_after", "p-1", "heading-after", "尾声"),
      ],
    };
    expect(applyStructuredBlockOperations(
      editor,
      payload,
      "forward",
      { sessionVersion: 7, contentHash: "base-hash" },
    ).ok).toBe(true);
    expect(editor.getText()).toContain("新正文");
    expect(editor.getText()).toContain("尾声");
    expect(editor.getText()).not.toContain("残尾");
    expect(validateStructuredBlockOperations(editor, payload, "reverse").ok).toBe(true);
    expect(applyStructuredBlockOperations(editor, payload, "reverse").ok).toBe(true);
    expect(editor.getJSON()).toEqual(before);
    editor.destroy();
  });

  it("reverses insert_before plus delete_node without parsing the empty beforeNode", () => {
    const editor = editorWith(`
      <p data-block-id="p-1">待删除正文</p>
      <p data-block-id="p-2">保留正文</p>
    `);
    const before = editor.getJSON();
    const payload: StructuredBlockEditPayload = {
      payloadVersion: 1,
      base: { sessionVersion: 7, contentHash: "base-hash" },
      operations: [
        insertedHeading("insert_before", "p-1", "heading-delete", "章节标题"),
        deleteOperation(editor, "p-1"),
      ],
    };
    expect(applyStructuredBlockOperations(
      editor,
      payload,
      "forward",
      { sessionVersion: 7, contentHash: "base-hash" },
    ).ok).toBe(true);
    expect(editor.getText()).toContain("章节标题");
    expect(editor.getText()).not.toContain("待删除正文");
    expect(validateStructuredBlockOperations(editor, payload, "reverse").ok).toBe(true);
    expect(applyStructuredBlockOperations(editor, payload, "reverse").ok).toBe(true);
    expect(editor.getJSON()).toEqual(before);
    editor.destroy();
  });

  it("reverses appended blocks by their stable afterNode blockId", () => {
    const editor = editorWith('<p data-block-id="p-1">正文</p>');
    const before = editor.getJSON();
    const payload: StructuredBlockEditPayload = {
      payloadVersion: 1,
      base: { sessionVersion: 7, contentHash: "base-hash" },
      operations: [appendParagraph("append-1", "附录")],
    };
    expect(applyStructuredBlockOperations(
      editor,
      payload,
      "forward",
      { sessionVersion: 7, contentHash: "base-hash" },
    ).ok).toBe(true);
    expect(editor.getText()).toContain("附录");
    expect(validateStructuredBlockOperations(editor, payload, "reverse").ok).toBe(true);
    expect(applyStructuredBlockOperations(editor, payload, "reverse").ok).toBe(true);
    expect(editor.getJSON()).toEqual(before);
    editor.destroy();
  });

  // AG-TASK-01 F2：delete_node 前端原子应用——删除块后段落移除、列表保留；reverse 精确恢复。
  it("applies delete_node by removing the block and restores it on reverse", () => {
    const editor = editorWith(`
      <ul data-block-id="list">
        <li data-block-id="item-1"><p data-block-id="p-1">第一条</p></li>
        <li data-block-id="item-2"><p data-block-id="p-2">第二条</p></li>
      </ul>
    `);
    const payload: StructuredBlockEditPayload = {
      payloadVersion: 1,
      base: { sessionVersion: 7, contentHash: "base-hash" },
      operations: [{
        op: "delete_node",
        blockId: "p-1",
        fingerprint: "",
        expectedNodeType: "paragraph",
        beforeNode: projectAgentEditableBlocks(editor)
          .find((block) => block.blockId === "p-1")!.nodeJson,
        afterNode: null,
      }],
    };
    expect(applyStructuredBlockOperations(
      editor,
      payload,
      "forward",
      { sessionVersion: 7, contentHash: "base-hash" },
    )).toEqual({ ok: true, affectedBlockIds: ["p-1"] });
    expect(editor.getText()).toContain("第二条");
    expect(editor.getText()).not.toContain("第一条");
    // reverse 精确恢复。
    expect(applyStructuredBlockOperations(editor, payload, "reverse").ok).toBe(true);
    expect(editor.getText()).toContain("第一条");
    editor.destroy();
  });

  it("rejects the entire batch before dispatch when one fingerprint is stale", () => {
    const editor = editorWith(`
      <p data-block-id="p-1">第一条</p>
      <p data-block-id="p-2">第二条</p>
    `);
    const first = payloadFor(editor, "p-1", "新第一条");
    const second = payloadFor(editor, "p-2", "新第二条");
    second.operations[0].fingerprint = "stale";
    const before = editor.getJSON();
    expect(applyStructuredBlockOperations(
      editor,
      { ...first, operations: [...first.operations, ...second.operations] },
      "forward",
      { sessionVersion: 7, contentHash: "base-hash" },
    )).toEqual({ ok: false, reason: "ED_BLOCK_OPERATION_FINGERPRINT_CHANGED" });
    expect(editor.getJSON()).toEqual(before);
    editor.destroy();
  });

  it("restores exact attrs and marks, but reports conflict after a target post-edit", () => {
    const editor = editorWith('<p data-block-id="p-1"><strong>原文</strong></p>');
    const payload = payloadFor(editor, "p-1", "候选");
    expect(applyStructuredBlockOperations(
      editor,
      payload,
      "forward",
      { sessionVersion: 7, contentHash: "base-hash" },
    ).ok).toBe(true);
    expect(applyStructuredBlockOperations(editor, payload, "reverse").ok).toBe(true);
    expect(editor.getHTML()).toContain("<strong>原文</strong>");

    const refreshed = payloadFor(editor, "p-1", "第二候选");
    expect(applyStructuredBlockOperations(
      editor,
      refreshed,
      "forward",
      { sessionVersion: 7, contentHash: "base-hash" },
    ).ok).toBe(true);
    const paragraph = projectAgentEditableBlocks(editor).find((block) => block.blockId === "p-1");
    if (!paragraph) throw new Error("paragraph missing");
    editor.commands.setContent('<p data-block-id="p-1">用户后改</p>');
    expect(fingerprintBlockNode(
      projectAgentEditableBlocks(editor).find((block) => block.blockId === "p-1")!.nodeJson,
    )).not.toBe(fingerprintBlockNode(refreshed.operations[0].afterNode as Record<string, unknown>));
    expect(validateStructuredBlockOperations(editor, refreshed, "reverse"))
      .toEqual({ ok: false, reason: "ED_BLOCK_OPERATION_FINGERPRINT_CHANGED" });
    editor.destroy();
  });
});
}

// Migrated intact from src/services/edImageOps.test.ts
{
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
}

// Migrated intact from src/services/edStyleOps.test.ts
{
// covers: BR-ED-DATA-005
// 样式协议纯逻辑：读侧投影（marks range 在块内纯文本平面、相邻同型合并、无 blockId 块跳过）；
// 写侧规划（锚校验批内原子——任一失效整批拒绝；字符锚 → PM 位置换算精确）。
// 最小 prosemirror schema（含 marks），不依赖 TipTap 运行时。

const schema = new Schema({
  nodes: {
    doc: { content: "block+" },
    paragraph: {
      group: "block",
      content: "inline*",
      attrs: { blockId: { default: null }, textAlign: { default: null } },
    },
    text: { group: "inline" },
  },
  marks: {
    bold: {},
    textStyle: { attrs: { color: { default: null } } },
  },
});

const bold = schema.marks.bold.create();
const red = schema.marks.textStyle.create({ color: "#ff0000" });

function para(blockId: string | null, parts: { t: string; marks?: import("@tiptap/pm/model").Mark[] }[]) {
  return schema.node(
    "paragraph",
    { blockId },
    parts.map((p) => schema.text(p.t, p.marks)),
  );
}

describe("edStyleOps · 读侧投影（BR-ED-DATA-005）", () => {
  it("mark 段 range 在块内纯文本平面 + attrs 透出", () => {
    const block = para("b1", [{ t: "普通" }, { t: "加粗", marks: [bold] }, { t: "红字", marks: [red] }]);
    const { text, marks } = extractBlockStyles(block);
    expect(text).toBe("普通加粗红字");
    expect(marks).toEqual([
      { type: "bold", range: [2, 4], attrs: undefined },
      { type: "textStyle", range: [4, 6], attrs: { color: "#ff0000" } },
    ]);
  });

  it("相邻同型同 attrs 段合并", () => {
    const block = para("b1", [{ t: "ab", marks: [bold] }, { t: "cd", marks: [bold] }]);
    expect(extractBlockStyles(block).marks).toEqual([{ type: "bold", range: [0, 4], attrs: undefined }]);
  });

  it("文档投影：无 blockId 块跳过；blockAttrs 剔除 blockId 内部属性", () => {
    const doc = schema.node("doc", null, [
      schema.node("paragraph", { blockId: "b1", textAlign: "center" }, [schema.text("居中")]),
      para(null, [{ t: "无锚" }]),
    ]);
    const views = projectDocStyles(doc);
    expect(views).toHaveLength(1);
    expect(views[0]).toMatchObject({ blockId: "b1", type: "paragraph", text: "居中", blockAttrs: { textAlign: "center" } });
    expect(views[0].blockAttrs.blockId).toBeUndefined();
  });
});

describe("edStyleOps · posFromCharOffset", () => {
  it("多段文本中偏移 → 精确内容 pos", () => {
    const block = para("b1", [{ t: "ab" }, { t: "cde", marks: [bold] }]);
    expect(posFromCharOffset(block, 0)).toBe(0);
    expect(posFromCharOffset(block, 2)).toBe(2);
    expect(posFromCharOffset(block, 4)).toBe(4); // "ab"+"cd" 之后
    expect(posFromCharOffset(block, 5)).toBe(5);
  });
});

describe("edStyleOps · planStyleOps（批内原子）", () => {
  const doc = schema.node("doc", null, [para("b1", [{ t: "第一段落" }]), para("b2", [{ t: "第二段" }])]);

  it("合法批 → 事务步骤（字符锚换算 PM 位置）", () => {
    const ops: StyleOp[] = [
      { op: "set_mark", blockId: "b1", start: 1, end: 3, mark: { type: "bold" } },
      { op: "set_block_attr", blockId: "b2", attrs: { textAlign: "right" } },
    ];
    const r = planStyleOps(doc, ops);
    expect(r.ok).toBe(true);
    if (r.ok) {
      // b1 内容起点 pos=1 → from=1+1=2, to=1+3=4
      expect(r.steps[0]).toEqual({ kind: "addMark", from: 2, to: 4, markType: "bold", attrs: undefined });
      expect(r.steps[1]).toMatchObject({ kind: "setBlockAttrs", blockPos: 6 });
    }
  });

  it("任一锚失效 → 整批拒绝（块不存在 / 偏移越界 / 空区间）", () => {
    const good: StyleOp = { op: "set_mark", blockId: "b1", start: 0, end: 2, mark: { type: "bold" } };
    expect(planStyleOps(doc, [good, { op: "set_mark", blockId: "不存在", start: 0, end: 1, mark: { type: "bold" } }]).ok).toBe(false);
    expect(planStyleOps(doc, [good, { op: "unset_mark", blockId: "b2", start: 0, end: 99, markType: "bold" }]).ok).toBe(false);
    expect(planStyleOps(doc, [good, { op: "set_mark", blockId: "b2", start: 1, end: 1, mark: { type: "bold" } }]).ok).toBe(false);
  });

  it("空批 → ok 空步骤", () => {
    expect(planStyleOps(doc, [])).toEqual({ ok: true, steps: [] });
  });

  it("未知 mark、非法 attrs 与不支持块属性在规划前整批拒绝", () => {
    expect(planStyleOps(doc, [
      { op: "set_mark", blockId: "b1", start: 0, end: 1, mark: { type: "sub" } },
    ]).ok).toBe(false);
    expect(planStyleOps(doc, [
      { op: "set_mark", blockId: "b1", start: 0, end: 1, mark: { type: "bold", attrs: { color: "#FF0000" } } },
    ]).ok).toBe(false);
  });
});

describe("edStyleOps · 不可信载荷严格解析", () => {
  it("接受真实 mark 名与受控 attrs", () => {
    expect(parseStyleOps([
      { op: "set_mark", blockId: "b1", start: 0, end: 2, mark: { type: "subscript" } },
      { op: "set_mark", blockId: "b1", start: 0, end: 2, mark: { type: "textStyle", attrs: { color: "#FF0000", fontSize: "12pt" } } },
      { op: "set_mark", blockId: "b1", start: 0, end: 2, mark: { type: "link", attrs: { href: "https://example.com" } } },
      { op: "set_block_attr", blockId: "b1", attrs: { textAlign: "center" } },
    ])).toHaveLength(4);
  });

  it("错误别名、危险链接、未知字段或批内任一非法项使整批拒绝", () => {
    for (const value of [
      [{ op: "set_mark", blockId: "b1", start: 0, end: 2, mark: { type: "sub" } }],
      [{ op: "set_mark", blockId: "b1", start: 0, end: 2, mark: { type: "bold", attrs: {} } }],
      [{ op: "set_mark", blockId: "b1", start: 0, end: 2, mark: { type: "textStyle", attrs: { fontSize: "01pt" } } }],
      [{ op: "set_mark", blockId: "b1", start: 0, end: 2, mark: { type: "link", attrs: { href: "javascript:alert(1)" } } }],
      [{ op: "set_block_attr", blockId: "b1", attrs: { textAlign: "middle" } }],
      [
        { op: "set_mark", blockId: "b1", start: 0, end: 2, mark: { type: "bold" } },
        { op: "set_mark", blockId: "b1", start: 0, end: 2, mark: { type: "bold" }, extra: true },
      ],
    ]) {
      expect(parseStyleOps(value)).toEqual([]);
    }
  });
});
}

// Migrated intact from src/services/edTableOps.test.ts
{
// covers: BR-ED-DATA-007
// 表格语义操作：快照以 tableId+fingerprint 定位；操作在临时 EditorState 完整规划，任一失败不改变原 doc。
const base = tableNodes({ tableGroup: "block", cellContent: "paragraph+", cellAttributes: {} });
const schema = new Schema({
  nodes: {
    doc: { content: "block+" },
    paragraph: { group: "block", content: "inline*" },
    text: { group: "inline" },
    table: { ...base.table, attrs: { ...base.table.attrs, tableId: { default: null } } },
    table_row: base.table_row,
    table_cell: { ...base.table_cell, attrs: { ...base.table_cell.attrs, backgroundColor: { default: null } } },
    table_header: { ...base.table_header, attrs: { ...base.table_header.attrs, backgroundColor: { default: null } } },
  },
  marks: { strong: {} },
});

function cell(text: string, attrs?: Record<string, unknown>) {
  return schema.nodes.table_cell.createAndFill(attrs, schema.nodes.paragraph.create(null, schema.text(text)))!;
}

function table() {
  return schema.nodes.table.create({ tableId: "table_1" }, [
    schema.nodes.table_row.create(null, [cell("A"), cell("B")]),
    schema.nodes.table_row.create(null, [cell("C"), cell("D")]),
  ]);
}

function state() {
  return EditorState.create({ schema, doc: schema.nodes.doc.create(null, [schema.nodes.paragraph.create(null, schema.text("前言")), table(), schema.nodes.paragraph.create(null, schema.text("后记"))]) });
}

describe("edTableOps · 结构快照与原子规划", () => {
  it("投影表格行列、单元格和会话锚", () => {
    const tables = projectTables(state().doc);
    expect(tables).toHaveLength(1);
    expect(tables[0]).toMatchObject({ tableId: "table_1", rows: 2, columns: 2 });
    expect(tables[0].cells.map((x) => x.text)).toEqual(["A", "B", "C", "D"]);
    expect(tables[0].cells.every((x) => x.contentKind === "plain_text")).toBe(true);
    expect(tables[0].cells.every((x) => x.backgroundColor === null)).toBe(true);
    expect(tables[0].columnWidths).toEqual([null, null]);
    expect(tables[0].capabilities).toContain("set_cell_background");
    expect(tables[0].capabilities).toContain("move_table");
    expect(tables[0].moveDirections).toEqual(["up", "down"]);
    expect(tables[0].fingerprint).toHaveLength(8);
  });

  it("合法批次在临时状态完整应用：插行、合并、单元格写入", () => {
    const beforeState = state();
    const before = projectTables(beforeState.doc)[0];
    const planned = planTableOperations(beforeState, {
      tableId: before.tableId,
      fingerprint: before.fingerprint,
      operations: [
        { op: "insert_row", at: 2 },
        { op: "merge_cells", from: { row: 1, column: 0 }, to: { row: 1, column: 1 } },
        { op: "set_cell_text", at: { row: 2, column: 0 }, text: "新增" },
      ],
    });
    expect(planned.ok).toBe(true);
    if (!planned.ok) return;
    expect(planned.after.rows).toBe(3);
    expect(planned.after.cells.some((x) => x.text === "新增")).toBe(true);
    // 规划不触碰原状态，保证后续操作失败时可整体拒绝。
    expect(projectTables(beforeState.doc)[0].rows).toBe(2);
  });

  it("指纹失配或删除唯一列均整批拒绝，原文不变", () => {
    const beforeState = state();
    const before = projectTables(beforeState.doc)[0];
    expect(planTableOperations(beforeState, { tableId: before.tableId, fingerprint: "stale", operations: [{ op: "insert_row", at: 1 }] })).toMatchObject({ ok: false, reason: "table_fingerprint_mismatch" });

    const oneColumn = EditorState.create({
      schema,
      doc: schema.nodes.doc.create(null, [schema.nodes.table.create({ tableId: "one" }, [schema.nodes.table_row.create(null, [cell("唯一")])])]),
    });
    const only = projectTables(oneColumn.doc)[0];
    expect(planTableOperations(oneColumn, { tableId: only.tableId, fingerprint: only.fingerprint, operations: [{ op: "delete_column", column: 0 }] })).toMatchObject({ ok: false, reason: "table_operation_failed:delete_column" });
    expect(projectTables(oneColumn.doc)[0].columns).toBe(1);
  });

  it("不可信载荷必须字段齐全；错误坐标和错误方向不得进入执行器", () => {
    expect(parseTableEditPayload({ tableId: "t", fingerprint: "f", operations: [{ op: "merge_cells", from: { row: 0, column: 0 } }] })).toBeUndefined();
    expect(parseTableEditPayload({ tableId: "t", fingerprint: "f", operations: [{ op: "move_table", direction: "left" }] })).toBeUndefined();
    expect(parseTableEditPayload({ tableId: "t", fingerprint: "f", operations: [{ op: "set_cell_text", at: { row: 0, column: 0 }, text: "安全" }] })).toMatchObject({ operations: [{ op: "set_cell_text" }] });
    expect(parseTableEditPayload({ tableId: "t", fingerprint: "f", operations: [{ op: "set_cell_background", at: { row: 0, column: 0 }, color: "#ffe699" }] })).toMatchObject({ operations: [{ op: "set_cell_background", color: "#FFE699" }] });
    expect(parseTableEditPayload({ tableId: "t", fingerprint: "f", operations: [{ op: "set_cell_background", at: { row: 0, column: 0 }, color: "transparent" }] })).toBeUndefined();
  });

  it("代理不得将富内容单元格退化为纯文本，且无可拆分或移动时能力会收紧", () => {
    const strong = schema.marks.strong.create();
    const rich = schema.nodes.table_cell.createAndFill(null, schema.nodes.paragraph.create(null, schema.text("粗体", [strong])))!;
    const onlyTable = schema.nodes.table.create({ tableId: "rich" }, [schema.nodes.table_row.create(null, [rich])]);
    const richState = EditorState.create({ schema, doc: schema.nodes.doc.create(null, [onlyTable]) });
    const snapshot = projectTables(richState.doc)[0];
    expect(snapshot.cells[0].contentKind).toBe("rich_content");
    expect(snapshot.capabilities).not.toContain("set_cell_text");
    expect(snapshot.capabilities).not.toContain("split_cell");
    expect(snapshot.capabilities).not.toContain("move_table");
    expect(snapshot.moveDirections).toEqual([]);
    expect(planTableOperations(richState, {
      tableId: snapshot.tableId,
      fingerprint: snapshot.fingerprint,
      operations: [{ op: "set_cell_text", at: { row: 0, column: 0 }, text: "不应覆盖" }],
    })).toMatchObject({ ok: false, reason: "table_operation_failed:set_cell_text" });
  });

  it("底纹操作只改节点属性、进入指纹，并保留富内容子树", () => {
    const strong = schema.marks.strong.create();
    const rich = schema.nodes.table_cell.createAndFill(
      null,
      schema.nodes.paragraph.create(null, schema.text("粗体", [strong])),
    )!;
    const richState = EditorState.create({
      schema,
      doc: schema.nodes.doc.create(null, [
        schema.nodes.table.create({ tableId: "shading" }, [schema.nodes.table_row.create(null, [rich])]),
      ]),
    });
    const before = projectTables(richState.doc)[0];
    const planned = planTableOperations(richState, {
      tableId: before.tableId,
      fingerprint: before.fingerprint,
      operations: [{ op: "set_cell_background", at: { row: 0, column: 0 }, color: "#ffe699" }],
    });
    expect(planned.ok).toBe(true);
    if (!planned.ok) return;
    expect(planned.after.cells[0]).toMatchObject({
      text: "粗体",
      contentKind: "rich_content",
      backgroundColor: "#FFE699",
    });
    expect(planned.after.fingerprint).not.toBe(before.fingerprint);
    const markedText = planned.state.doc.firstChild?.firstChild?.firstChild?.firstChild?.firstChild;
    expect(markedText?.marks.map((mark) => mark.type.name)).toContain("strong");
  });

  it("合并单元格只允许左上锚设置或清除底纹", () => {
    const merged = schema.nodes.table.create({ tableId: "merged" }, [
      schema.nodes.table_row.create(null, [cell("合并", { colspan: 2 })]),
    ]);
    const mergedState = EditorState.create({ schema, doc: schema.nodes.doc.create(null, [merged]) });
    const before = projectTables(mergedState.doc)[0];
    expect(planTableOperations(mergedState, {
      tableId: before.tableId,
      fingerprint: before.fingerprint,
      operations: [{ op: "set_cell_background", at: { row: 0, column: 1 }, color: "#FF0000" }],
    })).toMatchObject({ ok: false, reason: "table_operation_failed:set_cell_background" });

    const colored = planTableOperations(mergedState, {
      tableId: before.tableId,
      fingerprint: before.fingerprint,
      operations: [{ op: "set_cell_background", at: { row: 0, column: 0 }, color: "#FF0000" }],
    });
    expect(colored.ok).toBe(true);
    if (!colored.ok) return;
    expect(colored.after.cells[0].backgroundColor).toBe("#FF0000");
    const cleared = planTableOperations(colored.state, {
      tableId: colored.after.tableId,
      fingerprint: colored.after.fingerprint,
      operations: [{ op: "set_cell_background", at: { row: 0, column: 0 }, color: null }],
    });
    expect(cleared.ok && cleared.after.cells[0].backgroundColor).toBeNull();
  });

  it("顶层表格可以文档流上移，模拟结果与实际执行使用同一规划器", () => {
    const beforeState = state();
    const before = projectTables(beforeState.doc)[0];
    const planned = planTableOperations(beforeState, {
      tableId: before.tableId,
      fingerprint: before.fingerprint,
      operations: [{ op: "move_table", direction: "up" }],
    });
    expect(planned.ok).toBe(true);
    if (!planned.ok) return;
    expect(planned.state.doc.firstChild?.type.name).toBe("table");
  });

  it("人工移动后保持同一 tableId 的表格选区，邻近控件可继续操作", () => {
    let editorState = state();
    const tablePos = editorState.doc.child(0).nodeSize;
    const tableNode = editorState.doc.child(1);
    const firstCellOffset = TableMap.get(tableNode).map[0];
    editorState = editorState.apply(editorState.tr.setSelection(
      Selection.near(editorState.doc.resolve(tablePos + 1 + firstCellOffset + 1), 1),
    ));
    const fakeEditor = {
      get state() {
        return editorState;
      },
      view: {
        dispatch(transaction: Transaction) {
          editorState = editorState.apply(transaction);
        },
        focus() {},
      },
    } as unknown as Editor;
    registerEditor(fakeEditor);
    try {
      expect(activeTableSnapshot()?.tableId).toBe("table_1");
      const moved = moveActiveTable("up");
      expect(moved.ok).toBe(true);
      expect(editorState.doc.firstChild?.type.name).toBe("table");
      expect(activeTableSnapshot()?.tableId).toBe("table_1");
    } finally {
      registerEditor(null);
    }
  });
});
}
