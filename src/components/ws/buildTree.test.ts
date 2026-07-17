import { describe, expect, it } from "vitest";
import { buildTree } from "./FileTree";
import type { DocRefView } from "../../store/wsStore";

const doc = (relPath: string, docType = "md"): DocRefView => ({ relPath, docType, isDir: false });
const dir = (relPath: string): DocRefView => ({ relPath, docType: "", isDir: true });

describe("buildTree 扁平 FsEntry → 嵌套树", () => {
  it("按 / 构建目录层级，叶子为文件", () => {
    const tree = buildTree([doc("a.md"), dir("notes"), doc("notes/b.txt", "txt")]);
    expect(tree.map((n) => `${n.kind}:${n.name}`)).toEqual(["dir:notes", "file:a.md"]);
    const notes = tree.find((n) => n.name === "notes")!;
    expect(notes.children?.map((c) => c.name)).toEqual(["b.txt"]);
    expect(notes.children?.[0]).toMatchObject({ kind: "file", path: "notes/b.txt", docType: "txt" });
  });

  it("空目录可见（isDir 条目，无文件子节点）", () => {
    const tree = buildTree([dir("empty"), doc("a.md")]);
    const empty = tree.find((n) => n.name === "empty")!;
    expect(empty).toMatchObject({ kind: "dir", path: "empty" });
    expect(empty.children).toEqual([]);
  });

  it("目录条目与文件父路径合并到同一节点（去重）", () => {
    const tree = buildTree([dir("d"), doc("d/a.md")]);
    expect(tree.filter((n) => n.name === "d")).toHaveLength(1);
    expect(tree.find((n) => n.name === "d")!.children).toHaveLength(1);
  });

  it("目录在前、同级按名排序", () => {
    const tree = buildTree([doc("z.md"), doc("a.md"), doc("dir2/x.md"), doc("dir1/y.md")]);
    expect(tree.map((n) => n.name)).toEqual(["dir1", "dir2", "a.md", "z.md"]);
  });

  it("同一目录下多文件合并到同一 dir 节点", () => {
    const tree = buildTree([doc("d/a.md"), doc("d/b.md")]);
    const dirs = tree.filter((n) => n.kind === "dir");
    expect(dirs).toHaveLength(1);
    expect(dirs[0].children).toHaveLength(2);
  });

  it("空列表 → 空树", () => {
    expect(buildTree([])).toEqual([]);
  });

  it("保留文件节点的 BDocx 身份标记", () => {
    const tree = buildTree([{ relPath: "generated.docx", docType: "docx", isDir: false, isBDocx: true }]);
    expect(tree[0]).toMatchObject({ kind: "file", path: "generated.docx", isBDocx: true });
  });
});
