// @vitest-environment jsdom
import { describe, expect, it } from "vitest";
import { isEditorContentReady } from "./Editor";

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
