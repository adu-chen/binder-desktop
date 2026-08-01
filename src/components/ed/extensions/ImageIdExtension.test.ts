import { describe, expect, it } from "vitest";
import { reconcileImageIds } from "./ImageIdExtension";

// covers: BR-ED-DATA-001
// covers: BR-ED-DATA-003
describe("ImageIdExtension", () => {
  it("保持唯一 ID，修复缺失和重复且不复用", () => {
    const generated = ["image_new_1", "image_kept", "image_new_2"];
    expect(reconcileImageIds(
      ["image_kept", null, "image_kept", "image_other"],
      () => generated.shift() ?? "image_fallback",
    )).toEqual([
      null,
      "image_new_1",
      "image_new_2",
      null,
    ]);
  });
});
