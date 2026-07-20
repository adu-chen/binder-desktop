/** @vitest-environment jsdom */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { deStore, type DiffView } from "../../store/deStore";
import { DiffAllActionsBar } from "./DiffAllActionsBar";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const diff = (id: string, state: DiffView["state"]): DiffView => ({
  diffId: id,
  blockId: "",
  title: id,
  before: "before",
  after: "after",
  state,
});

beforeEach(() => {
  deStore.reset();
});

afterEach(() => {
  cleanup();
  deStore.reset();
});

describe("DiffAllActionsBar", () => {
  // covers: BR-DE-STATE-001
  it("有待裁决修改时显示批量操作按钮", () => {
    deStore.addDiff(diff("d1", "pending"));

    render(<DiffAllActionsBar />);

    expect(screen.getByText("1 条待裁决")).toBeTruthy();
    expect(screen.getByRole("button", { name: "全部接受" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "全部拒绝" })).toBeTruthy();
  });

  // covers: BR-DE-STATE-001
  it("批量应用中显示进度栏", () => {
    deStore.setBatch("applying");

    render(<DiffAllActionsBar />);

    expect(screen.getByText(/倒序应用中/)).toBeTruthy();
  });

  // covers: BR-DE-STATE-001
  it("批量完成后不显示终态栏", () => {
    deStore.addDiff(diff("d1", "accepted"));
    deStore.setBatch("done");

    const { container } = render(<DiffAllActionsBar />);

    expect(container.firstChild).toBeNull();
    expect(screen.queryByText(/全部应用完成/)).toBeNull();
  });
});
