/** @vitest-environment jsdom */
import { afterEach, describe, expect, it, vi } from "vitest";
import { render, screen, cleanup } from "@testing-library/react";

// DiffCard 经 deService→wsService 引入 invoke；渲染不触发调用，mock 以隔离。
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
import { DiffCard } from "./DiffCard";
import type { DiffView } from "../../store/deStore";

afterEach(cleanup);

const base: DiffView = { diffId: "d1", blockId: "", title: "改写开头", before: "旧", after: "新", state: "pending" };
const renderCard = (p: Partial<DiffView>) => render(<DiffCard diff={{ ...base, ...p }} />);

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
});
