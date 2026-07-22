/** @vitest-environment jsdom */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { closeStore } from "../../store/closeStore";
import { CloseConfirmDialog } from "./CloseConfirmDialog";

const closeServiceMock = vi.hoisted(() => ({
  cancelClose: vi.fn(),
  submitDecision: vi.fn(),
  forceQuit: vi.fn(),
}));
vi.mock("../../services/closeService", () => closeServiceMock);

beforeEach(() => {
  closeStore.reset();
  closeServiceMock.cancelClose.mockReset();
  closeServiceMock.submitDecision.mockReset();
  closeServiceMock.forceQuit.mockReset();
});

afterEach(() => {
  cleanup();
  closeStore.reset();
});

describe("CloseConfirmDialog（BR-SYS-STATE-004 关闭前处理弹窗）", () => {
  // covers: BR-SYS-STATE-004
  it("idle 态不渲染", () => {
    const { container } = render(<CloseConfirmDialog />);
    expect(container.firstChild).toBeNull();
  });

  // covers: BR-SYS-STATE-004
  it("closing 态不渲染（已进入实际关闭流程，不应再打断）", () => {
    closeStore.setClosing();
    const { container } = render(<CloseConfirmDialog />);
    expect(container.firstChild).toBeNull();
  });

  // covers: BR-SYS-STATE-004
  it("awaitingDecision：未选择前确认按钮禁用", () => {
    closeStore.setAwaitingDecision(2, [{ relPath: "a.md", hasAcceptedAwaitingSave: false }]);
    render(<CloseConfirmDialog />);

    expect((screen.getByRole("button", { name: "关闭应用" }) as HTMLButtonElement).disabled).toBe(true);
  });

  // covers: BR-SYS-STATE-004（2026-07-20 人类决策 2：接受全部强制耦合保存）
  it("选择接受全部后，未保存文件的放弃选项被禁用且保存被强制选中", () => {
    closeStore.setAwaitingDecision(2, [{ relPath: "a.md", hasAcceptedAwaitingSave: false }]);
    render(<CloseConfirmDialog />);

    fireEvent.click(screen.getByRole("button", { name: /接受全部/ }));

    const saveRadio = screen.getByLabelText(/保存修改/) as HTMLInputElement;
    const discardRadio = screen.getByLabelText(/放弃修改/) as HTMLInputElement;
    expect(saveRadio.checked).toBe(true);
    expect(discardRadio.disabled).toBe(true);
    expect((screen.getByRole("button", { name: "关闭应用" }) as HTMLButtonElement).disabled).toBe(false);
  });

  // covers: BR-SYS-STATE-004
  it("拒绝全部 + 放弃修改 → 提交 rejectAllAndDiscard", () => {
    closeStore.setAwaitingDecision(1, [{ relPath: "a.md", hasAcceptedAwaitingSave: false }]);
    render(<CloseConfirmDialog />);

    fireEvent.click(screen.getByRole("button", { name: /拒绝全部/ }));
    fireEvent.click(screen.getByLabelText(/放弃修改/));
    fireEvent.click(screen.getByRole("button", { name: "关闭应用" }));

    expect(closeServiceMock.submitDecision).toHaveBeenCalledWith("rejectAllAndDiscard");
  });

  // covers: BR-SYS-STATE-004
  it("只有未保存文件、无 diff 时不展示 AI 修改分区", () => {
    closeStore.setAwaitingDecision(0, [{ relPath: "a.md", hasAcceptedAwaitingSave: false }]);
    render(<CloseConfirmDialog />);

    expect(screen.queryByText("AI 修改")).toBeNull();
    expect(screen.getByText("未保存文件")).toBeTruthy();
  });

  // covers: BR-SYS-STATE-006（2026-07-20 人类决策 3：强制退出仅在失败后作为显式兜底出现）
  it("无失败时不展示强制退出；失败后展示且可触发", () => {
    closeStore.setAwaitingDecision(1, []);
    const { rerender } = render(<CloseConfirmDialog />);
    expect(screen.queryByText(/强制退出/)).toBeNull();

    closeStore.setFailed("保存失败：磁盘冲突");
    rerender(<CloseConfirmDialog />);
    const forceBtn = screen.getByRole("button", { name: /强制退出/ });
    fireEvent.click(forceBtn);
    expect(closeServiceMock.forceQuit).toHaveBeenCalled();
  });

  // covers: BR-SYS-STATE-004
  it("取消按钮调用 cancelClose", () => {
    closeStore.setAwaitingDecision(1, []);
    render(<CloseConfirmDialog />);
    fireEvent.click(screen.getByRole("button", { name: "取消" }));
    expect(closeServiceMock.cancelClose).toHaveBeenCalled();
  });
});
