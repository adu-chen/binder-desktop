import { useEffect, useMemo, useState, type ReactNode } from "react";
import { useWsStore, type DocRefView } from "../../store/wsStore";
import { loadDocs, createDoc, renameDoc, deleteDoc } from "../../services/wsService";
import { openDocument } from "../../services/edService";
import { edStore } from "../../store/edStore";
import { FileTreeNode, type TreeNode } from "./FileTreeNode";

/**
 * 文件树容器（A-SYS-C-T-02 §3 Side Panel WS · PI-FE-04）。
 * 从 ws.list_docs 投影的扁平 DocRef 列表建嵌套树；三态：loading / ready（含空态）/ error。
 * 目录在前、文件在后，同级按名排序。
 */

/** 扁平 DocRef（rel_path）列表 → 嵌套目录树。导出供单测（纯函数）。 */
export function buildTree(docs: DocRefView[]): TreeNode[] {
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

  // 右键菜单（重命名/删除）+ 内联输入（新建/重命名）状态。
  const [menu, setMenu] = useState<{ path: string; x: number; y: number } | null>(null);
  const [input, setInput] = useState<{ mode: "create" | "rename"; path?: string } | null>(null);
  const [value, setValue] = useState("");

  // 点击文件：高亮 open 态 + 经 ws.read_doc 载入编辑器（X-SEQ-002）。
  const handleOpenFile = (path: string) => {
    setOpenPath(path);
    void openDocument(path);
  };

  const closeOverlays = () => {
    setMenu(null);
    setInput(null);
    setValue("");
  };

  // 新建：内联输入文件名 → ws.create_doc → 刷新 + 打开新文件（X-SEQ-002）。
  const submitCreate = async (name: string) => {
    const rel = name.trim();
    if (!rel) return;
    try {
      const created = await createDoc(rel);
      await loadDocs();
      setOpenPath(created);
      await openDocument(created);
    } catch (e) {
      const err = e as { code?: string; message?: string };
      window.alert(`新建失败：${err?.message ?? String(e)}`);
    }
    closeOverlays();
  };

  // 重命名：ws.rename_doc → 刷新；若重命名的是当前打开文档则重开新路径。
  const submitRename = async (from: string, name: string) => {
    const to = name.trim();
    if (!to || to === from) return closeOverlays();
    try {
      const renamed = await renameDoc(from, to);
      await loadDocs();
      if (edStore.getSnapshot().openPath === from) {
        setOpenPath(renamed);
        await openDocument(renamed);
      }
    } catch (e) {
      const err = e as { code?: string; message?: string };
      window.alert(`重命名失败：${err?.message ?? String(e)}`);
    }
    closeOverlays();
  };

  // 删除：确认 → ws.delete_doc → 刷新；若删的是当前打开文档则清空编辑器。
  const doDelete = async (path: string) => {
    setMenu(null);
    if (!window.confirm(`删除「${path}」？此操作不可撤销。`)) return;
    try {
      await deleteDoc(path);
      if (edStore.getSnapshot().openPath === path) edStore.reset();
      await loadDocs();
    } catch (e) {
      const err = e as { code?: string; message?: string };
      window.alert(`删除失败：${err?.message ?? String(e)}`);
    }
  };

  // 仅 idle 时自动加载（启动恢复预置 / 切换重置会置 idle）；避免覆盖已预置结果。
  useEffect(() => {
    if (status === "idle") void loadDocs();
  }, [status]);

  const header = (
    <div className="filetree-hd">
      <span className="label">工作区</span>
      <span className="filetree-hd-actions">
        <button
          className="ghost ftn-refresh"
          onClick={() => { setInput({ mode: "create" }); setValue(""); }}
          title="新建文件"
        >
          ＋
        </button>
        <button className="ghost ftn-refresh" onClick={() => void loadDocs()} title="刷新文件树">
          ↻
        </button>
      </span>
    </div>
  );

  // 新建/重命名内联输入行。
  const inputRow = input && (
    <form
      className="ftn-input-row"
      onSubmit={(e) => {
        e.preventDefault();
        if (input.mode === "create") void submitCreate(value);
        else if (input.path) void submitRename(input.path, value);
      }}
    >
      <input
        autoFocus
        className="ftn-input"
        placeholder={input.mode === "create" ? "新文件相对路径，如 notes/a.md" : "新路径"}
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={(e) => e.key === "Escape" && closeOverlays()}
        onBlur={closeOverlays}
      />
    </form>
  );

  // 右键上下文菜单（重命名/删除）。
  const contextMenu = menu && (
    <div className="ftn-menu" style={{ left: menu.x, top: menu.y }} role="menu">
      <button
        className="ftn-menu-item"
        onMouseDown={(e) => {
          e.preventDefault();
          setInput({ mode: "rename", path: menu.path });
          setValue(menu.path);
          setMenu(null);
        }}
      >
        重命名
      </button>
      <button className="ftn-menu-item ftn-menu-del" onMouseDown={(e) => { e.preventDefault(); void doDelete(menu.path); }}>
        删除
      </button>
    </div>
  );

  let body: ReactNode;
  if (status === "loading" || status === "idle") {
    body = <p className="muted ftn-hint">加载文件…</p>;
  } else if (status === "error") {
    body = (
      <p className="ftn-error">
        加载失败：{error}
        <button className="ghost ftn-retry" onClick={() => void loadDocs()}>
          重试
        </button>
      </p>
    );
  } else if (tree.length === 0) {
    body = <p className="muted ftn-hint">workspace 为空（新增文件后点 ↻ 刷新）</p>;
  } else {
    body = (
      <div className="filetree">
        {tree.map((n) => (
          <FileTreeNode
            key={n.path}
            node={n}
            depth={0}
            openPath={openPath}
            onOpenFile={handleOpenFile}
            onContextMenu={(path, x, y) => setMenu({ path, x, y })}
          />
        ))}
      </div>
    );
  }

  return (
    <>
      {header}
      {inputRow}
      {body}
      {contextMenu}
      {menu && <div className="ftn-menu-backdrop" onClick={() => setMenu(null)} />}
    </>
  );
}
