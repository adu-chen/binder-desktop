/** @vitest-environment jsdom */
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { AutocompleteOverlay } from "./AutocompleteOverlay";
import { assistStore, type AssistTarget } from "../../store/assistStore";

const target: AssistTarget = {
  docPath: "notes/a.md",
  loadSeq: 1,
  blockId: "b1",
  start: 2,
  end: 2,
  text: "",
  prefix: "前文",
  before: "前",
  after: "后",
  mode: "cursor",
};

afterEach(() => {
  cleanup();
  assistStore.resetForTest();
});

function isKeyboardScope(event: KeyboardEvent): boolean {
  const host = document.querySelector(".editor-host");
  const targetNode = event.target instanceof Node ? event.target : null;
  const active = document.activeElement;
  return Boolean(host && ((targetNode && host.contains(targetNode)) || (active && host.contains(active))));
}

function renderOverlay(onPick = vi.fn()) {
  const requestId = assistStore.openAutocomplete({ x: 1, y: 2 }, target);
  assistStore.setSuggestions(requestId, ["候选一", "候选二", "候选三"]);
  const onDismiss = vi.fn();
  render(
    <>
      <button data-testid="outside">outside</button>
      <div className="editor-host">
        <button data-testid="inside">inside</button>
        <AutocompleteOverlay onPick={onPick} onDismiss={onDismiss} isKeyboardScope={isKeyboardScope} />
      </div>
    </>,
  );
  return { onPick, onDismiss };
}

describe("AutocompleteOverlay keyboard scope", () => {
  // covers: BR-AG-STATE-003
  // window 级监听只能在编辑器/overlay 焦点范围内采纳，避免输入框外误触 Tab/数字键。
  it("ignores numeric picks outside editor keyboard scope", () => {
    const onPick = vi.fn();
    renderOverlay(onPick);

    screen.getByTestId("outside").focus();
    fireEvent.keyDown(window, { key: "1" });
    expect(onPick).not.toHaveBeenCalled();

    screen.getByTestId("inside").focus();
    fireEvent.keyDown(window, { key: "2" });
    expect(onPick).toHaveBeenCalledWith("候选二");
  });

  // covers: BR-AG-STATE-003
  // 输入法 composition 期间不得用数字键/Tab 采纳候选。
  it("does not pick while composing text", () => {
    const onPick = vi.fn();
    renderOverlay(onPick);
    screen.getByTestId("inside").focus();

    const event = new KeyboardEvent("keydown", { key: "1", bubbles: true });
    Object.defineProperty(event, "isComposing", { value: true });
    window.dispatchEvent(event);

    expect(onPick).not.toHaveBeenCalled();
  });

  // covers: BR-AG-STATE-003
  // 候选失败或不足时不应把刚触发的浮层瞬间关闭；保留轻量空态，避免用户体感为快捷键无效。
  it("keeps a visible empty state when generation returns no suggestions", () => {
    const requestId = assistStore.openAutocomplete({ x: 1, y: 2 }, target);
    assistStore.setAutocompleteFailure(requestId, "parse_underflow");

    render(<AutocompleteOverlay onPick={vi.fn()} onDismiss={vi.fn()} />);

    const empty = screen.getByText("暂无可用续写");
    expect(empty).toBeTruthy();
    expect(empty.closest(".assist-ac")?.getAttribute("data-failure-reason")).toBe("parse_underflow");
  });

});
