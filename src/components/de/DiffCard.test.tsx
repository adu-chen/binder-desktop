/** @vitest-environment jsdom */
import { afterEach, describe, expect, it, vi } from "vitest";
import { render, screen, cleanup, fireEvent } from "@testing-library/react";

// DiffCard 经 deService→wsService 引入 invoke；渲染不触发调用，mock 以隔离。
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const mocks = vi.hoisted(() => ({ revealBlock: vi.fn(() => "located") }));
vi.mock("../../services/edLocate", () => ({ revealBlock: mocks.revealBlock }));
import { DiffCard } from "./DiffCard";
import type { DiffView } from "../../store/deStore";

afterEach(() => {
  cleanup();
  mocks.revealBlock.mockClear();
});

const base: DiffView = { diffId: "d1", blockId: "", relPath: "demo.md", title: "改写开头", before: "旧", after: "新", state: "pending" };
const renderCard = (p: Partial<DiffView>) => render(<DiffCard diff={{ ...base, ...p }} />);

describe("DiffCard 样式变更摘要（PI-ED-11 · D-3 方案A）", () => {
  // covers: BR-DE-DATA-002
  it("styleOps 存在 → 渲染样式标签行；缺省 → 不渲染", () => {
    renderCard({ styleOps: ["加粗 [2–4]", "颜色 #ff0000 [5–9]"] });
    expect(screen.getByText("加粗 [2–4]")).toBeTruthy();
    expect(screen.getByText("颜色 #ff0000 [5–9]")).toBeTruthy();
    cleanup();
    renderCard({});
    expect(screen.queryByText(/加粗/)).toBeNull();
  });
});

describe("DiffCard 8 状态渲染（互斥）", () => {
  it("pending：PENDING badge + 接受/拒绝", () => {
    renderCard({ state: "pending" });
    expect(screen.getByText("PENDING")).toBeTruthy();
    expect((screen.getByRole("button", { name: /接受/ }) as HTMLButtonElement).disabled).toBe(false);
    expect(screen.getByRole("button", { name: /拒绝/ })).toBeTruthy();
  });

  it("conflict：接受禁用 + 显示冲突对象", () => {
    renderCard({ state: "conflict", conflictWith: "d5" });
    expect(screen.getByText("冲突")).toBeTruthy();
    expect((screen.getByRole("button", { name: /接受/ }) as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByText(/d5/)).toBeTruthy();
  });

  it("expired：原因引导文案 + 重新生成", () => {
    renderCard({ state: "expired", expiredReason: "external_file_changed" });
    expect(screen.getByText("已失效")).toBeTruthy();
    expect(screen.getByText(/文件在磁盘上被外部修改/)).toBeTruthy();
    expect(screen.getByRole("button", { name: /重新生成/ })).toBeTruthy();
  });

  it("error：错误码 + 重试", () => {
    renderCard({ state: "error", errorCode: "WS_APPLY_REPLACE_FAILED" });
    expect(screen.getByText(/WS_APPLY_REPLACE_FAILED/)).toBeTruthy();
    expect(screen.getByRole("button", { name: /重试/ })).toBeTruthy();
  });

  it("accepted：森林绿终态 + 时间轴号", () => {
    const { container } = renderCard({ state: "accepted", timelineSeq: 101 });
    expect(screen.getByText("ACCEPTED")).toBeTruthy();
    expect(screen.getByText(/时间轴 #101/)).toBeTruthy();
    expect(container.querySelector(".diffcard-accepted")).not.toBeNull();
  });

  it("validating/applying：过渡态显示处理中（按钮禁用）", () => {
    const { container } = renderCard({ state: "applying" });
    expect(screen.getByText("写入中")).toBeTruthy();
    expect(container.querySelector(".diffcard-progress")).not.toBeNull();
    expect((screen.getByRole("button", { name: /处理中/ }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("rejected：弱化终态 class", () => {
    const { container } = renderCard({ state: "rejected" });
    expect(container.querySelector(".diffcard-rejected")).not.toBeNull();
  });

  it("before/after 内容渲染", () => {
    renderCard({ state: "pending", before: "原文X", after: "新文Y" });
    expect(screen.getByText(/原文X/)).toBeTruthy();
    expect(screen.getByText(/新文Y/)).toBeTruthy();
  });

  it("长 before/after 默认短预览，可展开查看完整内容", () => {
    const long = `第一行\n${"长文本".repeat(90)}`;
    const { container } = renderCard({ state: "pending", before: long, after: "新文" });
    const beforeLine = container.querySelector(".diffcard-before");
    expect(beforeLine?.textContent).not.toContain(long);
    fireEvent.click(screen.getByRole("button", { name: /展开预览/ }));
    expect(beforeLine?.textContent).toContain(long);
  });

  it("有 blockId 时显示定位按钮并调用编辑器锚定位", () => {
    renderCard({ blockId: "b1", locationLabel: "Block 1" });
    expect(screen.getByText(/demo.md · Block 1/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "定位" }));
    expect(mocks.revealBlock).toHaveBeenCalledWith("b1");
  });
});
