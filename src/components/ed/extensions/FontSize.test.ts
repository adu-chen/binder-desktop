import { describe, expect, it } from "vitest";
import { parseFontSize, renderFontSize } from "./FontSize";

describe("FontSize", () => {
  it("preserves DOCX-derived pt units instead of rewriting them as px", () => {
    expect(parseFontSize({ style: { fontSize: "12pt" } } as HTMLElement)).toBe("12pt");
    expect(renderFontSize("12pt")).toEqual({ style: "font-size: 12pt" });
  });
});
