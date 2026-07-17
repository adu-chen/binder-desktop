/** @vitest-environment jsdom */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/react";
import { DOC_REF_MIME } from "../../services/edLocate";
import { REFERENCE_MIME } from "../../services/referencePayload";
import { deStore } from "../../store/deStore";
import { edStore } from "../../store/edStore";
import { tabsStore } from "../../store/tabsStore";
import { TabStrip } from "./TabStrip";

function dragData() {
  const values = new Map<string, string>();
  return {
    values,
    dataTransfer: {
      getData: vi.fn((type: string) => values.get(type) ?? ""),
      setData: vi.fn((type: string, value: string) => values.set(type, value)),
    },
  };
}

beforeEach(() => {
  Object.defineProperty(globalThis, "CSS", {
    value: { escape: (value: string) => value },
    configurable: true,
  });
  Object.defineProperty(window.HTMLElement.prototype, "scrollIntoView", {
    value: vi.fn(),
    configurable: true,
  });
  tabsStore.reset();
  edStore.reset();
  deStore.reset();
});

afterEach(cleanup);

describe("TabStrip · 文档引用拖出", () => {
  it("标签拖出时写入旧文档引用 MIME 与统一 ReferencePayload MIME", () => {
    tabsStore.openDoc("notes/a.md");
    const { container } = render(<TabStrip />);
    const tab = container.querySelector("[data-tab-id]") as HTMLElement;
    const { values, dataTransfer } = dragData();

    fireEvent.dragStart(tab, { dataTransfer });

    expect(JSON.parse(values.get(DOC_REF_MIME) ?? "{}")).toEqual({ relPath: "notes/a.md" });
    expect(JSON.parse(values.get(REFERENCE_MIME) ?? "{}")).toEqual({ kind: "doc", relPath: "notes/a.md" });
    expect(values.get("text/plain")).toBe("notes/a.md");
  });
});
