/** @vitest-environment jsdom */
import { afterEach, describe, expect, it } from "vitest";
import { render, cleanup } from "@testing-library/react";
import { FileTreeNode, type TreeNode, type FileNodeState } from "./FileTreeNode";

afterEach(cleanup);

const fileNode = (docType = "md"): TreeNode => ({ name: "a.md", path: "a.md", kind: "file", docType });

function renderNode(opts: {
  docType?: string;
  openPath?: string | null;
  override?: FileNodeState;
  pending?: number;
}) {
  return render(
    <FileTreeNode
      node={fileNode(opts.docType)}
      depth={0}
      openPath={opts.openPath ?? null}
      onOpenFile={() => {}}
      stateOverride={opts.override ? { "a.md": opts.override } : undefined}
      pendingCount={opts.pending != null ? { "a.md": opts.pending } : undefined}
    />,
  );
}

describe("FileTreeNode 6 状态渲染（互斥）", () => {
  it("normal：默认态，无标记", () => {
    const { container } = renderNode({});
    const btn = container.querySelector(".ftn-file")!;
    expect(btn.className).toContain("ftn-normal");
    expect(container.querySelector(".ftn-dot")).toBeNull();
    expect(container.querySelector(".ftn-ext")).toBeNull();
    expect(container.querySelector(".ftn-badge")).toBeNull();
  });

  it("open：openPath 命中 → ftn-open（锈红左条）", () => {
    const { container } = renderNode({ openPath: "a.md" });
    expect(container.querySelector(".ftn-file")!.className).toContain("ftn-open");
  });

  it("not_supported（预览类型）：xlsx 灰显但可点击预览（不禁用）", () => {
    const { container } = renderNode({ docType: "xlsx", openPath: "a.md" });
    const btn = container.querySelector(".ftn-file") as HTMLButtonElement;
    expect(btn.className).toContain("ftn-not_supported");
    expect(btn.disabled).toBe(false); // 预览类型可点击触发只读预览（PI-FE-22）
  });

  it("dirty_open：红点标记", () => {
    const { container } = renderNode({ override: "dirty_open" });
    expect(container.querySelector(".ftn-file")!.className).toContain("ftn-dirty_open");
    expect(container.querySelector(".ftn-dot")).not.toBeNull();
  });

  it("external_changed：金色 ! 标记", () => {
    const { container } = renderNode({ override: "external_changed" });
    expect(container.querySelector(".ftn-ext")?.textContent).toBe("!");
  });

  it("pending_diff：数字 badge", () => {
    const { container } = renderNode({ override: "pending_diff", pending: 3 });
    expect(container.querySelector(".ftn-badge")?.textContent).toBe("3");
  });
});
