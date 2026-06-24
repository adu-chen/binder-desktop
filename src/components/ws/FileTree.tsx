import { useEffect, useMemo, useState, type ReactNode } from "react";
import { useWsStore, type DocRefView } from "../../store/wsStore";
import { loadDocs, createDoc, renameDoc, deleteDoc, searchDocs, type SearchHitDto } from "../../services/wsService";
import { openDocument } from "../../services/edService";
import { edStore, useEdStore } from "../../store/edStore";
import { useDeStore } from "../../store/deStore";
import { tabsStore } from "../../store/tabsStore";
import { exportDoc } from "../../services/prvService";
import { FileTreeNode, type TreeNode, type FileNodeState } from "./FileTreeNode";

/**
 * 文件树容器（A-SYS-C-T-02 §3 Side Panel WS · PI-FE-04）。
 * 从 ws.list_docs 投影的扁平 DocRef 列表建嵌套树；三态：loading / ready（含空态）/ error。
 * 目录在前、文件在后，同级按名排序。
 */

/**
 * 扁平 FsEntry 列表 → 嵌套目录树。导出供单测（纯函数）。
 * 目录条目（isDir，含空目录）的全部路径段均建为目录节点 → 空目录可见；文件条目末段建叶子。
 * 同路径目录去重（目录条目与文件父路径合并到同一节点）。
 */
export function buildTree(docs: DocRefView[]): TreeNode[] {
  const root: TreeNode = { name: "", path: "", kind: "dir", children: [] };
  const ensureDir = (cursor: TreeNode, part: string, path: string): TreeNode => {
    let dir = cursor.children!.find((c) => c.kind === "dir" && c.name === part);
    if (!dir) {
      dir = { name: part, path, kind: "dir", children: [] };
      cursor.children!.push(dir);
    }
    return dir;
  };

  for (const doc of docs) {
    const parts = doc.relPath.split("/").filter(Boolean);
    let cursor = root;
    for (let i = 0; i < parts.length; i++) {
      const part = parts[i];
      const isLeaf = i === parts.length - 1;
      const path = parts.slice(0, i + 1).join("/");
      // 文件条目的末段为叶子；其余段（及目录条目的所有段）均为目录。
      if (isLeaf && !doc.isDir) {
        if (!cursor.children!.some((c) => c.kind === "file" && c.name === part)) {
          cursor.children!.push({ name: part, path, kind: "file", docType: doc.docType });
        }
      } else {
        cursor = ensureDir(cursor, part, path);
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
  const { openPath: edOpenPath, tabState } = useEdStore();
  const { diffs } = useDeStore();
  const [openPath, setOpenPath] = useState<string | null>(null);
  const tree = useMemo(() => buildTree(docs), [docs]);

  // 投影打开文档的真实态到节点（优先级 external > pending > dirty；X-FORBID-007）。
  const { stateOverride, pendingCount } = useMemo(() => {
    const override: Record<string, FileNodeState> = {};
    const counts: Record<string, number> = {};
    if (edOpenPath) {
      const pending = diffs.filter((d) => d.state === "pending" && d.relPath === edOpenPath).length;
      if (tabState === "external_changed") override[edOpenPath] = "external_changed";
      else if (pending > 0) {
        override[edOpenPath] = "pending_diff";
        counts[edOpenPath] = pending;
      } else if (tabState === "dirty") override[edOpenPath] = "dirty_open";
    }
    return { stateOverride: override, pendingCount: counts };
  }, [edOpenPath, tabState, diffs]);

  // 全文搜索（ws.search，防抖）：有查询时结果列表替代文件树。
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchHitDto[]>([]);
  const [searching, setSearching] = useState(false);

  useEffect(() => {
    const q = query.trim();
    if (!q) {
      setResults([]);
      return;
    }
    setSearching(true);
    const id = window.setTimeout(async () => {
      try {
        setResults(await searchDocs(q));
      } catch {
        setResults([]);
      } finally {
        setSearching(false);
      }
    }, 250);
    return () => window.clearTimeout(id);
  }, [query]);

  // 右键菜单（重命名/删除）+ 内联输入（新建/重命名）状态。
  const [menu, setMenu] = useState<{ path: string; x: number; y: number } | null>(null);
  const [input, setInput] = useState<{ mode: "create" | "rename"; path?: string } | null>(null);
  const [value, setValue] = useState("");

  // 点击文件：开/激活文档标签 + 经 ws.read_doc 载入编辑器（X-SEQ-002）。
  const handleOpenFile = (path: string) => {
    setOpenPath(path);
    tabsStore.openDoc(path);
    void openDocument(path);
  };
  // 点击预览类型文件：开/激活预览标签（office/媒体/图片，多标签共存）。
  const handlePreviewFile = (path: string) => {
    setOpenPath(path);
    tabsStore.openPreview(path);
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

  // 导出：prv.export → 刷新文件树 + 提示落点。
  const doExport = async (path: string, format: "pdf" | "docx" | "md") => {
    setMenu(null);
    try {
      const dest = await exportDoc(path, format);
      await loadDocs();
      window.alert(`已导出：${dest}`);
    } catch (e) {
      const err = e as { code?: string; message?: string };
      window.alert(`导出失败：${err?.message ?? String(e)}`);
    }
  };

  // 按源类型给出可导出目标格式。
  const exportTargets = (path: string): ("pdf" | "docx" | "md")[] => {
    const ext = path.split(".").pop()?.toLowerCase();
    if (ext === "md" || ext === "markdown" || ext === "txt") return ["docx"];
    if (ext === "docx" || ext === "doc") return ["pdf", "md"];
    if (["xlsx", "xls", "pptx", "ppt"].includes(ext ?? "")) return ["pdf"];
    return [];
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
      {exportTargets(menu.path).map((fmt) => (
        <button
          key={fmt}
          className="ftn-menu-item"
          onMouseDown={(e) => { e.preventDefault(); void doExport(menu.path, fmt); }}
        >
          导出 {fmt.toUpperCase()}
        </button>
      ))}
      <button className="ftn-menu-item ftn-menu-del" onMouseDown={(e) => { e.preventDefault(); void doDelete(menu.path); }}>
        删除
      </button>
    </div>
  );

  const searchBox = (
    <div className="ftn-search">
      <input
        className="ftn-search-input"
        placeholder="全文搜索…"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
      />
      {query && (
        <button className="ftn-search-clear" onClick={() => setQuery("")} title="清空">×</button>
      )}
    </div>
  );

  // 有查询 → 搜索结果列表替代文件树。
  const searchResults = (
    <div className="ftn-results">
      {searching && <p className="muted ftn-hint">搜索中…</p>}
      {!searching && results.length === 0 && <p className="muted ftn-hint">无匹配</p>}
      {results.map((h, i) => (
        <button
          key={`${h.rel_path}:${h.line}:${i}`}
          className="ftn-result"
          onClick={() => {
            setOpenPath(h.rel_path);
            void openDocument(h.rel_path, h.line); // 打开并跳到命中行
          }}
        >
          <span className="ftn-result-path">{h.rel_path}:{h.line}</span>
          <span className="ftn-result-prev">{h.preview}</span>
        </button>
      ))}
    </div>
  );

  let body: ReactNode;
  if (query.trim()) {
    body = searchResults;
  } else if (status === "loading" || status === "idle") {
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
            onPreviewFile={handlePreviewFile}
            onContextMenu={(path, x, y) => setMenu({ path, x, y })}
            stateOverride={stateOverride}
            pendingCount={pendingCount}
          />
        ))}
      </div>
    );
  }

  return (
    <>
      {header}
      {searchBox}
      {inputRow}
      {body}
      {contextMenu}
      {menu && <div className="ftn-menu-backdrop" onClick={() => setMenu(null)} />}
    </>
  );
}
