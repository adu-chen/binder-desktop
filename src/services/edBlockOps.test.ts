// @vitest-environment jsdom

import { describe, expect, it } from "vitest";
import { Editor } from "@tiptap/core";
import { buildEditorExtensions } from "../components/ed/extensions";
import {
  applyStructuredBlockOperations,
  fingerprintBlockNode,
  projectAgentEditableBlocks,
  validateStructuredBlockOperations,
} from "./edBlockOps";
import type { StructuredBlockEditPayload } from "../store/edStore";

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
      capabilities: ["replace_node", "style_ops"],
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
    )).not.toBe(fingerprintBlockNode(refreshed.operations[0].afterNode));
    expect(validateStructuredBlockOperations(editor, refreshed, "reverse"))
      .toEqual({ ok: false, reason: "ED_BLOCK_OPERATION_FINGERPRINT_CHANGED" });
    editor.destroy();
  });
});
