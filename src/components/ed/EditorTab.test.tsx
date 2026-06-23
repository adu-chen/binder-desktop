/** @vitest-environment jsdom */
import { afterEach, describe, expect, it } from "vitest";
import { render, screen, cleanup } from "@testing-library/react";
import { EditorTab } from "./EditorTab";

afterEach(cleanup);

describe("EditorTab 6 状态标记渲染（互斥）", () => {
  it("normal/saved：仅名称，无状态标记", () => {
    const { container } = render(<EditorTab name="a.md" state="saved" />);
    expect(screen.getByText("a.md")).toBeTruthy();
    expect(container.querySelector(".edtab-dot")).toBeNull();
    expect(container.querySelector(".edtab-spinner")).toBeNull();
    expect(container.querySelector(".edtab-ext")).toBeNull();
  });

  it("dirty：红点", () => {
    const { container } = render(<EditorTab name="a.md" state="dirty" />);
    expect(container.querySelector(".edtab-dot")).not.toBeNull();
  });

  it("saving：旋转器", () => {
    const { container } = render(<EditorTab name="a.md" state="saving" />);
    expect(container.querySelector(".edtab-spinner")).not.toBeNull();
  });

  it("external_changed：金色 !", () => {
    const { container } = render(<EditorTab name="a.md" state="external_changed" />);
    expect(container.querySelector(".edtab-ext")?.textContent).toBe("!");
  });

  it("pending_diff：数字 badge（计数 > 0）", () => {
    const { container } = render(<EditorTab name="a.md" state="pending_diff" pendingDiffCount={2} />);
    expect(container.querySelector(".edtab-badge")?.textContent).toBe("2");
  });
});
