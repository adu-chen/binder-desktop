/** @vitest-environment jsdom */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Editor } from "@tiptap/react";

vi.mock("../../services/edImageOps", () => ({
  activeImageSnapshot: vi.fn(() => ({
    blockId: "image-block",
    imagePos: 12,
    standalone: true,
    moveDirections: ["up"],
  })),
  moveActiveImage: vi.fn(() => true),
  deleteActiveImage: vi.fn(() => true),
}));

import {
  activeImageSnapshot,
  deleteActiveImage,
  moveActiveImage,
} from "../../services/edImageOps";
import { ImageMoveControls } from "./ImageMoveControls";

afterEach(() => {
  cleanup();
  document.body.innerHTML = "";
  vi.clearAllMocks();
});

// covers: BR-ED-DATA-001
// covers: BR-ED-DATA-003
describe("ImageMoveControls", () => {
  it("控件直接挂到选中图片 NodeView 上方，遵循可移动方向且删除走受控操作", () => {
    const viewDom = document.createElement("div");
    const imageView = document.createElement("span");
    imageView.className = "ed-img-view selected";
    viewDom.append(imageView);
    document.body.append(viewDom);
    const editor = {
      view: {
        dom: viewDom,
        nodeDOM: () => imageView,
      },
      on: vi.fn(),
      off: vi.fn(),
    } as unknown as Editor;

    render(<ImageMoveControls editor={editor} />);

    const group = screen.getByRole("group", { name: "图片操作" });
    expect(group.parentElement).toBe(imageView);
    expect(group.style.top).toBe("");
    expect(group.style.left).toBe("");
    expect(group.getAttribute("data-block-id")).toBe("image-block");
    expect(group.getAttribute("data-standalone")).toBe("true");
    expect(screen.getByRole("button", { name: "图片上移一块" })).toHaveProperty("disabled", false);
    expect(screen.getByRole("button", { name: "图片下移一块" })).toHaveProperty("disabled", true);

    const mouseDown = new MouseEvent("mousedown", { bubbles: true, cancelable: true });
    group.dispatchEvent(mouseDown);
    expect(mouseDown.defaultPrevented).toBe(true);

    fireEvent.click(screen.getByRole("button", { name: "图片上移一块" }));
    expect(moveActiveImage).toHaveBeenCalledWith(editor, "up");

    fireEvent.click(screen.getByRole("button", { name: "删除图片" }));
    expect(deleteActiveImage).toHaveBeenCalledWith(editor);
    expect(screen.queryByRole("group", { name: "图片操作" })).toBeNull();
  });

  it("混排图片仍提供删除，但两个块移动方向都禁用", () => {
    vi.mocked(activeImageSnapshot).mockReturnValue({
      blockId: "mixed",
      imagePos: 8,
      standalone: false,
      moveDirections: [],
    });
    const viewDom = document.createElement("div");
    const imageView = document.createElement("span");
    imageView.className = "ed-img-view selected";
    viewDom.append(imageView);
    document.body.append(viewDom);
    const editor = {
      view: { dom: viewDom, nodeDOM: () => imageView },
      on: vi.fn(),
      off: vi.fn(),
    } as unknown as Editor;

    render(<ImageMoveControls editor={editor} />);

    expect(screen.getByRole("button", { name: "图片上移一块" })).toHaveProperty("disabled", true);
    expect(screen.getByRole("button", { name: "图片下移一块" })).toHaveProperty("disabled", true);
    expect(screen.getByRole("button", { name: "删除图片" })).toHaveProperty("disabled", false);
  });
});
