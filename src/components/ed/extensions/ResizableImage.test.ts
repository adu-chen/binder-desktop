/** @vitest-environment jsdom */
import { describe, expect, it } from "vitest";
import { Editor } from "@tiptap/core";
import StarterKit from "@tiptap/starter-kit";
import { NodeSelection } from "@tiptap/pm/state";
import { imageCssLengthToPx, ResizableImage } from "./ResizableImage";

// covers: BR-ED-DATA-003
// covers: BR-ED-STATE-001
describe("ResizableImage · BDocx 图片属性恢复", () => {
  it("把 Pandoc 物理长度按 96dpi 规范为 px", () => {
    expect(imageCssLengthToPx("1.25in")).toBe("120");
    expect(imageCssLengthToPx("72pt")).toBe("96");
    expect(imageCssLengthToPx("2.54cm")).toBe("96");
    expect(imageCssLengthToPx("120px")).toBe("120");
    expect(imageCssLengthToPx("auto")).toBeNull();
  });

  it("真实 TipTap schema 保留稳定媒体引用并恢复 CSS 宽高", () => {
    const editor = new Editor({
      extensions: [
        StarterKit,
        ResizableImage.configure({ inline: true, allowBase64: true }),
      ],
      content:
        '<p><img src="asset://localhost/cache/media/image1.png" data-binder-media-src="media/image1.png" data-binder-project-src="images/cover.png" data-binder-project-hash="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" style="width:1.25in;height:0.75in"></p>',
    });
    const image = editor.state.doc.firstChild?.firstChild;
    expect(image?.type.name).toBe("image");
    expect(image?.attrs).toMatchObject({
      width: "120",
      height: "72",
      binderMediaSrc: "media/image1.png",
      binderProjectSrc: "images/cover.png",
      binderProjectHash: "a".repeat(64),
    });
    const html = editor.getHTML();
    expect(html).toContain('width="120"');
    expect(html).toContain('height="72"');
    expect(html).toContain('data-binder-media-src="media/image1.png"');
    expect(html).toContain('data-binder-project-src="images/cover.png"');
    expect(html).toContain(`data-binder-project-hash="${"a".repeat(64)}"`);
    editor.destroy();
  });

  it("document 捕获鼠标事件在缩放下实时预览，并于 mouseup 提交一次 width 事务", () => {
    const element = document.createElement("div");
    document.body.append(element);
    const editor = new Editor({
      element,
      extensions: [
        StarterKit,
        ResizableImage.configure({ inline: true, allowBase64: true }),
      ],
      content:
        '<p><img src="data:image/png;base64,AA==" width="100" height="50" data-binder-media-src="media/image1.png"></p>',
    });
    editor.view.dom.style.zoom = "1.25";
    let imagePos = -1;
    editor.state.doc.descendants((node, pos) => {
      if (node.type.name === "image") imagePos = pos;
    });
    editor.view.dispatch(editor.state.tr.setSelection(NodeSelection.create(editor.state.doc, imagePos)));
    const image = element.querySelector(".ed-img-view img") as HTMLImageElement;
    const imageView = element.querySelector(".ed-img-view") as HTMLElement;
    const handle = element.querySelector(".ed-img-handle") as HTMLElement;
    const parent = imageView.parentElement as HTMLElement;
    Object.defineProperty(image, "getBoundingClientRect", {
      value: () => ({ width: 125, height: 62.5, left: 0, right: 125, top: 0, bottom: 62.5, x: 0, y: 0, toJSON() {} }),
    });
    Object.defineProperty(parent, "getBoundingClientRect", {
      value: () => ({ width: 500, height: 100, left: 0, right: 500, top: 0, bottom: 100, x: 0, y: 0, toJSON() {} }),
    });
    let changedTransactions = 0;
    editor.on("transaction", ({ transaction }) => {
      if (transaction.docChanged) changedTransactions += 1;
    });

    handle.dispatchEvent(new MouseEvent("mousedown", {
      bubbles: true,
      cancelable: true,
      button: 0,
      clientX: 100,
    }));
    document.dispatchEvent(new MouseEvent("mousemove", {
      bubbles: true,
      cancelable: true,
      clientX: 150,
    }));
    expect(image.style.width).toBe("140px");
    expect(editor.getAttributes("image").width).toBe("100");

    document.dispatchEvent(new MouseEvent("mouseup", {
      bubbles: true,
      cancelable: true,
      clientX: 150,
    }));
    expect(editor.state.doc.nodeAt(imagePos)?.attrs).toMatchObject({ width: "140", height: null });
    expect(changedTransactions).toBe(1);
    expect(editor.getHTML()).toContain('width="140"');
    expect(editor.getHTML()).not.toContain('height="50"');
    editor.destroy();
    element.remove();
  });

  it("只按下并松开缩放手柄不会制造空事务或清除来源高度", () => {
    const element = document.createElement("div");
    document.body.append(element);
    const editor = new Editor({
      element,
      extensions: [
        StarterKit,
        ResizableImage.configure({ inline: true, allowBase64: true }),
      ],
      content: '<p><img src="data:image/png;base64,AA==" width="100" height="50"></p>',
    });
    const image = element.querySelector(".ed-img-view img") as HTMLImageElement;
    const handle = element.querySelector(".ed-img-handle") as HTMLElement;
    Object.defineProperty(image, "getBoundingClientRect", {
      value: () => ({ width: 100, height: 50, left: 0, right: 100, top: 0, bottom: 50, x: 0, y: 0, toJSON() {} }),
    });
    let changedTransactions = 0;
    editor.on("transaction", ({ transaction }) => {
      if (transaction.docChanged) changedTransactions += 1;
    });

    handle.dispatchEvent(new MouseEvent("mousedown", {
      bubbles: true,
      cancelable: true,
      button: 0,
      clientX: 100,
    }));
    document.dispatchEvent(new MouseEvent("mouseup", {
      bubbles: true,
      cancelable: true,
      clientX: 100,
    }));

    expect(changedTransactions).toBe(0);
    expect(editor.getHTML()).toContain('width="100"');
    expect(editor.getHTML()).toContain('height="50"');
    editor.destroy();
    element.remove();
  });
});
