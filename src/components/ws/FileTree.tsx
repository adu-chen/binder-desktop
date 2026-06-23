import { useEffect, useMemo, useState } from "react";
import { useWsStore, type DocRefView } from "../../store/wsStore";
import { loadDocs } from "../../services/wsService";
import { FileTreeNode, type TreeNode } from "./FileTreeNode";

/**
 * 文件树容器（A-SYS-C-T-02 §3 Side Panel WS · PI-FE-04）。
 * 从 ws.list_docs 投影的扁平 DocRef 列表建嵌套树；三态：loading / ready（含空态）/ error。
 * 目录在前、文件在后，同级按名排序。
 */

/** 扁平 DocRef（rel_path）列表 → 嵌套目录树。 */
function buildTree(docs: DocRefView[]): TreeNode[] {
  const root: TreeNode = { name: "", path: "", kind: "dir", children: [] };
  for (const doc of docs) {
    const parts = doc.relPath.split("/").filter(Boolean);
    let cursor = root;
    for (let i = 0; i < parts.length; i++) {
      const part = parts[i];
      const isLeaf = i === parts.length - 1;
      const path = parts.slice(0, i + 1).join("/");
      if (isLeaf) {
        cursor.children!.push({ name: part, path, kind: "file", docType: doc.docType });
      } else {
        let dir = cursor.children!.find((c) => c.kind === "dir" && c.name === part);
        if (!dir) {
          dir = { name: part, path, kind: "dir", children: [] };
          cursor.children!.push(dir);
        }
        cursor = dir;
      }
    }
  }
  sortTree(root);
  return root.children!;
}

/** 同级排序：目录在前，再按名（mono 列表的稳定视觉）。 */
function sortTree(node: TreeNode) {
  if (!node.children) return;
  node.children.sort((a, b) => {
    if (a.kind !== b.kind) return a.kind === "dir" ? -1 : 1;
    return a.name.localeCompare(b.name);
  });
  for (const c of node.children) sortTree(c);
}

export function FileTree() {
  const { docs, status, error } = useWsStore();
  const [openPath, setOpenPath] = useState<string | null>(null);
  const tree = useMemo(() => buildTree(docs), [docs]);

  useEffect(() => {
    void loadDocs();
  }, []);

  if (status === "loading" || status === "idle") {
    return <p className="muted ftn-hint">加载文件…</p>;
  }
  if (status === "error") {
    return (
      <p className="ftn-error">
        加载失败：{error}
        <button className="ghost ftn-retry" onClick={() => void loadDocs()}>
          重试
        </button>
      </p>
    );
  }
  if (tree.length === 0) {
    return <p className="muted ftn-hint">workspace 为空</p>;
  }

  return (
    <div className="filetree">
      {tree.map((n) => (
        <FileTreeNode key={n.path} node={n} depth={0} openPath={openPath} onOpenFile={setOpenPath} />
      ))}
    </div>
  );
}
