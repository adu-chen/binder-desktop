import { useEffect, useMemo, useState, type ReactNode } from "react";
import { useWsStore, wsStore, type DocRefView } from "../../store/wsStore";
import { loadDocs, createDoc, createFolder, renameDoc, deleteDoc, deleteFolder, searchDocs, type SearchHitDto } from "../../services/wsService";
import { openDocument } from "../../services/edService";
import { createDocxDraft } from "../../services/docService";
import { edStore, useEdStore } from "../../store/edStore";
import { useDeStore } from "../../store/deStore";
import { tabsStore } from "../../store/tabsStore";
import { exportDoc } from "../../services/prvService";
import { FileTreeNode, PREVIEW_ONLY, type TreeNode, type FileNodeState } from "./FileTreeNode";
import { ContextMenu, type ContextMenuDef } from "../shell/ContextMenu";

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
          cursor.children!.push({ name: part, path, kind: "file", docType: doc.docType, isBDocx: doc.isBDocx });
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

/** 「＋」新建下拉格式清单（WS-NEW-BUTTON-01）。固定格式命名时自动补扩展名；custom 保留任意扩展名能力。 */
const CREATE_FORMATS = [
  { key: "folder", label: "文件夹", ext: null },
  { key: "md", label: "Markdown (.md)", ext: "md" },
  { key: "txt", label: "纯文本 (.txt)", ext: "txt" },
  { key: "html", label: "HTML (.html)", ext: "html" },
  { key: "docx", label: "Word 文档 (.docx)", ext: "docx" },
  { key: "csv", label: "CSV (.csv)", ext: "csv" },
  { key: "custom", label: "自定义格式…", ext: "custom" },
] as const;

type CreateFormatKey = (typeof CREATE_FORMATS)[number]["key"];

export function FileTree() {
  const { docs, status, error } = useWsStore();
  const { sessions } = useEdStore();
  const { diffs } = useDeStore();
  const [openPath, setOpenPath] = useState<string | null>(null);
  const tree = useMemo(() => buildTree(docs), [docs]);

  // 投影每个已打开文档会话的真实态到对应节点（优先级 external > pending > dirty；X-FORBID-007）。
  const { stateOverride, pendingCount } = useMemo(() => {
    const override: Record<string, FileNodeState> = {};
    const counts: Record<string, number> = {};
    for (const [path, sess] of Object.entries(sessions)) {
      const pending = diffs.filter((d) => d.state === "pending" && d.relPath === path).length;
      if (pending > 0) {
        override[path] = "pending_diff";
        counts[path] = pending;
      } else if (sess.tabState === "dirty") override[path] = "dirty_open";
    }
    return { stateOverride: override, pendingCount: counts };
  }, [sessions, diffs]);

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

  // 右键菜单（文件：重命名/导出/删除/复制路径；目录：在此新建文件）+ 内联输入（新建/重命名）状态。
  const [menu, setMenu] = useState<{ path: string; x: number; y: number; kind: "dir" | "file" } | null>(null);
  const [input, setInput] = useState<{ mode: "create" | "rename"; path?: string; format?: CreateFormatKey } | null>(null);
  const [value, setValue] = useState("");

  // 「＋」新建：当前激活区域（WS-NEW-BUTTON-01）。"" = 根目录；目录行点击即激活；头部可回根目录。
  const [activeDir, setActiveDir] = useState("");
  // 「＋」格式下拉锚点（复用 ContextMenu）。
  const [newMenu, setNewMenu] = useState<{ x: number; y: number } | null>(null);

  // 点击文件：开/激活文档标签 + 经 ws.read_doc 载入编辑器（X-SEQ-002）。
  // 已有会话（文档已打开）→ 仅切活动标签（openDoc 内 setActivePath），不重新读盘，保留未保存编辑。
  const handleOpenFile = (path: string) => {
    setOpenPath(path);
    const already = edStore.getSnapshot().sessions[path];
    tabsStore.openDoc(path);
    if (!already) void openDocument(path);
  };
  // 点击预览类型文件：开/激活预览标签（office/媒体/图片，多标签共存）。
  const handlePreviewFile = (path: string) => {
    setOpenPath(path);
    tabsStore.openPreview(path);
  };

  // 原生 docx「创建草稿并编辑」按钮（2026-08-04）：跳过预览直达草稿编辑。
  // 草稿路径 {stem}.draft.{ext} 已存在（docs 投影）→ 直接打开既有草稿（不重建，保留草稿编辑）；
  // 不存在 → doc.create_draft 基于原 docx 复制草稿后再打开编辑 tab（doc.open_for_edit 载入）。
  const handleEditDocx = (path: string) => {
    const dot = path.lastIndexOf(".");
    const draftRel = dot > 0 ? `${path.slice(0, dot)}.draft${path.slice(dot)}` : path;
    const hasDraft = wsStore
      .getSnapshot()
      .docs.some((d) => d.relPath === draftRel);
    void (async () => {
      if (!hasDraft) {
        try {
          await createDocxDraft(path);
          await loadDocs(); // 文件树同步显示草稿节点
        } catch (e) {
          const err = e as { code?: string; message?: string };
          window.alert(`创建草稿失败：${err?.message ?? String(e)}`);
          return;
        }
      }
      setOpenPath(draftRel);
      tabsStore.openDoc(draftRel); // 切/跳转到编辑 tab（已有则激活既有标签）
      await openDocument(draftRel);
    })();
  };

  const closeOverlays = () => {
    setMenu(null);
    setInput(null);
    setValue("");
    setNewMenu(null);
  };

  // 新建：内联输入文件名 → 按格式新建（WS-NEW-BUTTON-01：文件夹走 create_folder 不打开；固定格式自动补
  // 扩展名；文档创建后按类型分流——可编辑进编辑器、预览类进预览标签）。dirPath 为目录右键「在此新建文件」
  // 的目录前缀（CTX-MENU-01）或「＋」下拉的激活区域；根目录新建为空串。
  const submitCreate = async (name: string, dirPath = "", format?: CreateFormatKey) => {
    let fileName = name.trim();
    if (!fileName) return;
    // 命名输入预填了激活目录前缀；用户若保留前缀输入则先剥离，避免双重前缀（WS-NEW-BUTTON-01）。
    if (dirPath && fileName.startsWith(`${dirPath}/`)) {
      fileName = fileName.slice(dirPath.length + 1);
    }
    const fmt = CREATE_FORMATS.find((f) => f.key === format);
    // 固定格式：名称无扩展名时自动补（custom 不补，保留任意扩展名能力）。
    if (fmt?.ext && fmt.ext !== "custom" && !/\.[a-z0-9]+$/i.test(fileName)) {
      fileName = `${fileName}.${fmt.ext}`;
    }
    const rel = (dirPath ? `${dirPath}/${fileName}` : fileName).trim();
    if (!rel) return;
    try {
      if (format === "folder") {
        await createFolder(rel);
        await loadDocs();
        closeOverlays();
        return;
      }
      const created = await createDoc(rel);
      await loadDocs();
      setOpenPath(created);
      // 打开分流：仅「＋」下拉新建按类型分流（预览类进预览）；目录右键（无 format）保持原编辑路径。
      const ext = created.split(".").pop()?.toLowerCase() ?? "";
      if (format && PREVIEW_ONLY.has(ext)) tabsStore.openPreview(created);
      else await openDocument(created);
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

  // 删除目录（WS-DIR-DELETE-01，两级）：空目录直接删（mode=empty，无需确认）；非空目录确认后递归删。
  // 空判断基于 docs 投影（无 dir/ 前缀条目）；删除后精确关闭被删目录下的标签与编辑器会话（不误伤其他目录未保存编辑）。
  const doDeleteFolder = async (path: string) => {
    setMenu(null);
    const isEmpty = !docs.some((d) => d.relPath.startsWith(`${path}/`));
    if (!isEmpty && !window.confirm(`将删除文件夹「${path}」及其中所有内容，此操作不可撤销。`)) return;
    try {
      await deleteFolder(path, isEmpty ? "empty" : "recursive");
      const prefix = `${path}/`;
      for (const t of tabsStore.getSnapshot().tabs) {
        if (t.path.startsWith(prefix)) tabsStore.close(t.id);
      }
      edStore.closeDocsUnder(path);
      await loadDocs();
    } catch (e) {
      const err = e as { code?: string; message?: string };
      window.alert(`删除失败：${err?.message ?? String(e)}`);
    }
  };

  // 复制路径（CTX-MENU-01）：写剪贴板在用户手势内可用；失败静默（便利功能不阻断）。
  const copyPath = async (path: string) => {
    setMenu(null);
    try {
      await navigator.clipboard.writeText(path);
    } catch {
      // 静默：写剪贴板失败不打扰用户（可改用 ⌘C 复制标签/路径）。
    }
  };

  // 右键菜单项：按目标面生成（X-FORBID-007——全部动作复用既有命令面，无本地业务判断）。
  const menuItems: ContextMenuDef[] | null = useMemo(() => {
    if (!menu) return null;
    if (menu.kind === "dir") {
      return [
        {
          type: "item",
          id: "new",
          label: "在此新建文件",
          onSelect: () => {
            setInput({ mode: "create", path: menu.path });
            setValue(menu.path ? `${menu.path}/` : "");
            setMenu(null);
          },
        },
        { type: "separator", id: "sep-dir-del" },
        {
          type: "item",
          id: "delete-folder",
          label: "删除",
          danger: true,
          onSelect: () => void doDeleteFolder(menu.path),
        },
      ];
    }
    const items: ContextMenuDef[] = [
      {
        type: "item",
        id: "rename",
        label: "重命名",
        onSelect: () => {
          setInput({ mode: "rename", path: menu.path });
          setValue(menu.path);
          setMenu(null);
        },
      },
    ];
    for (const fmt of exportTargets(menu.path)) {
      items.push({
        type: "item",
        id: `export-${fmt}`,
        label: `导出 ${fmt.toUpperCase()}`,
        onSelect: () => void doExport(menu.path, fmt),
      });
    }
    items.push(
      { type: "item", id: "copy-path", label: "复制路径", onSelect: () => void copyPath(menu.path) },
      { type: "separator", id: "sep-del" },
      { type: "item", id: "delete", label: "删除", danger: true, onSelect: () => void doDelete(menu.path) },
    );
    return items;
    // eslint-disable-next-line react-hooks/exhaustive-deps -- menu 为唯一依赖；setInput/setValue 为稳定 setter。
  }, [menu]);

  // 仅 idle 时自动加载（启动恢复预置 / 切换重置会置 idle）；避免覆盖已预置结果。
  useEffect(() => {
    if (status === "idle") void loadDocs();
  }, [status]);

  const header = (
    <div className="filetree-hd">
      <span className="label">工作区</span>
      <span className="ftn-active-dir" title="当前新建位置">
        {activeDir ? `${activeDir}/` : "根目录"}
        {activeDir && (
          <button className="ghost ftn-active-reset" onClick={() => setActiveDir("")} title="回到根目录新建">
            ↖
          </button>
        )}
      </span>
      <span className="filetree-hd-actions">
        <button
          className="ghost ftn-refresh"
          onClick={(e) => {
            const rect = e.currentTarget.getBoundingClientRect();
            setNewMenu({ x: rect.left, y: rect.bottom + 4 });
          }}
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

  // 「＋」新建格式下拉（WS-NEW-BUTTON-01）：复用统一 ContextMenu；选格式后进入命名输入，落点在激活区域。
  const newMenuEl = newMenu && (
    <ContextMenu
      items={CREATE_FORMATS.map((f) => ({
        type: "item",
        id: f.key,
        label: f.label,
        onSelect: () => {
          setInput({ mode: "create", path: activeDir || undefined, format: f.key });
          setValue(activeDir ? `${activeDir}/` : "");
        },
      }))}
      x={newMenu.x}
      y={newMenu.y}
      onClose={() => setNewMenu(null)}
    />
  );

  // 新建/重命名内联输入行。
  const createPlaceholder = (() => {
    if (!input?.format) return "新文件相对路径，如 notes/a.md";
    if (input.format === "folder") return "文件夹名";
    if (input.format === "custom") return "文件名.扩展名（如 data.json）";
    const ext = CREATE_FORMATS.find((f) => f.key === input.format)?.ext;
    return `文件名（自动追加 .${ext}）`;
  })();

  const inputRow = input && (
    <form
      className="ftn-input-row"
      onSubmit={(e) => {
        e.preventDefault();
        if (input.mode === "create") void submitCreate(value, input.path ?? "", input.format);
        else if (input.path) void submitRename(input.path, value);
      }}
    >
      <input
        autoFocus
        className="ftn-input"
        placeholder={input.mode === "create" ? createPlaceholder : "新路径"}
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={(e) => e.key === "Escape" && closeOverlays()}
        onBlur={closeOverlays}
      />
    </form>
  );

  // 右键上下文菜单（统一 ContextMenu 组件，CTX-MENU-01：文件重命名/导出/复制路径/删除，目录在此新建）。
  const contextMenu = menu && menuItems && (
    <ContextMenu items={menuItems} x={menu.x} y={menu.y} onClose={() => setMenu(null)} />
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
            onContextMenu={(path, x, y, kind) => setMenu({ path, x, y, kind })}
            onEditDocx={handleEditDocx}
            stateOverride={stateOverride}
            pendingCount={pendingCount}
            activeDir={activeDir}
            onActivateDir={setActiveDir}
          />
        ))}
      </div>
    );
  }

  return (
    <>
      <div className="ftn-top">
        {header}
        {searchBox}
        {inputRow}
      </div>
      {body}
      {contextMenu}
      {newMenuEl}
    </>
  );
}
