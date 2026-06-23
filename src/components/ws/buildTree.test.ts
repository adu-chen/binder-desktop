import { describe, expect, it } from "vitest";
import { buildTree } from "./FileTree";
import type { DocRefView } from "../../store/wsStore";

const doc = (relPath: string, docType = "md"): DocRefView => ({ workspaceId: "w", relPath, docType });

describe("buildTree 扁平 DocRef → 嵌套树", () => {
  it("按 / 构建目录层级，叶子为文件", () => {
    const tree = buildTree([doc("a.md"), doc("notes/b.txt", "txt")]);
    expect(tree.map((n) => `${n.kind}:${n.name}`)).toEqual(["dir:notes", "file:a.md"]);
    const notes = tree.find((n) => n.name === "notes")!;
    expect(notes.children?.map((c) => c.name)).toEqual(["b.txt"]);
    expect(notes.children?.[0]).toMatchObject({ kind: "file", path: "notes/b.txt", docType: "txt" });
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
});
