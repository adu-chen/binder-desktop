/** @vitest-environment jsdom */
import { afterEach, describe, expect, it, vi } from "vitest";
import { render, cleanup, fireEvent } from "@testing-library/react";
import { DOC_REF_MIME } from "../../services/edLocate";
import { REFERENCE_MIME } from "../../services/referencePayload";
import { FileTreeNode, type TreeNode, type FileNodeState } from "./FileTreeNode";

afterEach(cleanup);

const fileNode = (docType = "md", patch: Partial<TreeNode> = {}): TreeNode => ({
  name: "a.md",
  path: "a.md",
  kind: "file",
  docType,
  ...patch,
});

function renderNode(opts: {
  docType?: string;
  node?: TreeNode;
  openPath?: string | null;
  override?: FileNodeState;
  pending?: number;
  onOpenFile?: (path: string) => void;
  onPreviewFile?: (path: string) => void;
}) {
  return render(
    <FileTreeNode
      node={opts.node ?? fileNode(opts.docType)}
      depth={0}
      openPath={opts.openPath ?? null}
      onOpenFile={opts.onOpenFile ?? (() => {})}
      stateOverride={opts.override ? { "a.md": opts.override } : undefined}
      pendingCount={opts.pending != null ? { "a.md": opts.pending } : undefined}
      onPreviewFile={opts.onPreviewFile}
    />,
  );
}

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

describe("FileTreeNode 状态与只读预览模式标记", () => {
  it("normal：默认态，无标记", () => {
    const { container } = renderNode({});
    const btn = container.querySelector(".ftn-file")!;
    expect(btn.className).toContain("ftn-normal");
    expect(container.querySelector(".ftn-dot")).toBeNull();
    expect(container.querySelector(".ftn-mode")).toBeNull();
    expect(container.querySelector(".ftn-badge")).toBeNull();
  });

  it("open：openPath 命中 → ftn-open（锈红左条）", () => {
    const { container } = renderNode({ openPath: "a.md" });
    expect(container.querySelector(".ftn-file")!.className).toContain("ftn-open");
  });

  it("preview_only（预览类型）：xlsx 保持可点击行，并叠加 VIEW 标记", () => {
    const { container } = renderNode({ docType: "xlsx", openPath: "a.md" });
    const btn = container.querySelector(".ftn-file") as HTMLButtonElement;
    expect(btn.className).toContain("ftn-open");
    expect(btn.className).toContain("ftn-preview_only");
    expect(container.querySelector(".ftn-mode")?.textContent).toBe("VIEW");
    expect(btn.disabled).toBe(false); // 预览类型可点击触发只读预览（PI-FE-22）
  });

  it("原生 docx 走只读预览，BDocx 走编辑打开", () => {
    const open = vi.fn();
    const preview = vi.fn();
    const native = renderNode({
      node: fileNode("docx", { name: "native.docx", path: "native.docx" }),
      onOpenFile: open,
      onPreviewFile: preview,
    });
    fireEvent.click(native.container.querySelector(".ftn-file") as HTMLButtonElement);
    expect(preview).toHaveBeenCalledWith("native.docx");
    expect(open).not.toHaveBeenCalled();
    expect(native.container.querySelector(".ftn-mode")?.textContent).toBe("VIEW");
    native.unmount();

    const bdocx = renderNode({
      node: fileNode("docx", { name: "generated.docx", path: "generated.docx", isBDocx: true }),
      onOpenFile: open,
      onPreviewFile: preview,
    });
    fireEvent.click(bdocx.container.querySelector(".ftn-file") as HTMLButtonElement);
    expect(open).toHaveBeenCalledWith("generated.docx");
    expect(bdocx.container.querySelector(".ftn-mode")).toBeNull();
  });

  it("dirty_open：红点标记", () => {
    const { container } = renderNode({ override: "dirty_open" });
    expect(container.querySelector(".ftn-file")!.className).toContain("ftn-dirty_open");
    expect(container.querySelector(".ftn-dot")).not.toBeNull();
  });

  it("pending_diff：数字 badge", () => {
    const { container } = renderNode({ override: "pending_diff", pending: 3 });
    expect(container.querySelector(".ftn-badge")?.textContent).toBe("3");
  });

  it("文件节点拖出时写入旧文档引用 MIME 与统一 ReferencePayload MIME", () => {
    const { container } = renderNode({});
    const btn = container.querySelector(".ftn-file") as HTMLButtonElement;
    const { values, dataTransfer } = dragData();

    fireEvent.dragStart(btn, { dataTransfer });

    expect(JSON.parse(values.get(DOC_REF_MIME) ?? "{}")).toEqual({ relPath: "a.md" });
    expect(JSON.parse(values.get(REFERENCE_MIME) ?? "{}")).toEqual({ kind: "doc", relPath: "a.md" });
    expect(values.get("text/plain")).toBe("a.md");
  });
});
