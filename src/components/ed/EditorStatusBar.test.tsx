// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { EditorStatusBar } from "./EditorStatusBar";

const editor = {
  storage: { characterCount: { characters: () => 2, words: () => 1 } },
  state: {
    selection: { from: 1, to: 1, head: 1 },
    doc: {
      textContent: "红字",
      content: { size: 2 },
      textBetween: () => "红字",
    },
  },
  on: () => undefined,
  off: () => undefined,
} as never;

describe("EditorStatusBar · DOCX 样式风险告知", () => {
  afterEach(cleanup);

  it("有转换风险时显示可访问的样式提示", () => {
    render(<EditorStatusBar editor={editor} tabState="saved" converting={false} styleNotices={["颜色片段未能恢复"]} />);
    expect(screen.getByLabelText("样式提示：颜色片段未能恢复").textContent).toContain("样式提示：1");
  });

  it("无风险时不显示样式提示", () => {
    render(<EditorStatusBar editor={editor} tabState="saved" converting={false} />);
    expect(screen.queryByText(/样式提示：/)).toBeNull();
  });
});
