/** @vitest-environment jsdom */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render } from "@testing-library/react";
import { REFERENCE_MIME } from "../../services/referencePayload";
import { workspaceShell } from "../../store/workspaceShell";
import { wsStore, type DocRefView } from "../../store/wsStore";
import { edStore } from "../../store/edStore";
import { tabsStore } from "../../store/tabsStore";
import { buildTree } from "./FileTree";
import { FileTreeNode, type TreeNode, type FileNodeState } from "./FileTreeNode";

// FileTree 容器测试依赖 wsService/edService（WS-NEW-BUTTON-01）：mock 服务层，仅验证 UI 交互与命令组装。
vi.mock("../../services/wsService", () => ({
  loadDocs: vi.fn().mockResolvedValue(undefined),
  createDoc: vi.fn().mockResolvedValue("a.md"),
  createFolder: vi.fn().mockResolvedValue("notes"),
  renameDoc: vi.fn().mockResolvedValue("x.md"),
  deleteDoc: vi.fn().mockResolvedValue(undefined),
  deleteFolder: vi.fn().mockResolvedValue(undefined),
  searchDocs: vi.fn().mockResolvedValue([]),
}));
vi.mock("../../services/edService", () => ({
  openDocument: vi.fn().mockResolvedValue(undefined),
}));
vi.mock("../../services/agService", () => ({
  resolveDecision: vi.fn().mockResolvedValue({ taskRunId: "task", status: "cancelled" }),
  resolveDecisionAndContinue: vi.fn(),
}));

import { createDoc, createFolder, deleteFolder } from "../../services/wsService";
import { openDocument } from "../../services/edService";
import { resolveDecisionAndContinue } from "../../services/agService";
import { agStore } from "../../store/agStore";
import { DecisionBar } from "../ag/DecisionBar";
import { FileTree } from "./FileTree";

function assertUnsavedTabCloseGuard() {
  tabsStore.reset();
  edStore.reset();
  tabsStore.openDoc("draft.md");
  edStore.loadDoc("draft.md", "尚未保存", "new-file");
  edStore.setTabState("draft.md", "dirty");
  const tab = tabsStore.getSnapshot().tabs[0];
  const confirm = vi.spyOn(globalThis, "confirm").mockReturnValueOnce(false).mockReturnValueOnce(true);

  expect(tabsStore.close(tab.id)).toBe(false);
  expect(tabsStore.getSnapshot().tabs).toHaveLength(1);
  expect(edStore.getSnapshot().sessions["draft.md"]).toBeDefined();
  expect(tabsStore.close(tab.id)).toBe(true);
  expect(tabsStore.getSnapshot().tabs).toHaveLength(0);
  expect(edStore.getSnapshot().sessions["draft.md"]).toBeUndefined();
  expect(confirm).toHaveBeenCalledTimes(2);
  confirm.mockRestore();
}

async function assertWorkspaceOperationDecisionProjection() {
  agStore.reset();
  agStore.setPendingDecision({
      requestId: "decision-partial",
      kind: "permission",
      payload: {
        workspaceOperationPlanId: "plan-partial",
        summary: "删除资料目录",
        operations: [{ itemId: "item-1", kind: "delete_resource", sourcePath: "资料", resourceKind: "folder" }],
        consequences: {
          recursive_count: 6,
          affected_sessions: ["资料/a.md"],
          affected_diffs: ["diff-1"],
          affected_references: ["资料引用"],
          recoverable: true,
        },
        recoverable: true,
        failurePolicy: "fail_fast_compensate",
      },
  });
  vi.mocked(resolveDecisionAndContinue).mockResolvedValue({
      taskRunId: "task-partial",
      status: "failed",
      workspaceOperationReceipt: {
        operation_id: "operation-partial",
        plan_id: "plan-partial",
        final_status: "partial_failed",
        item_receipts: [{
          item_id: "item-2",
          seq: 1,
          status: "failed",
          error_message: "目标被外部占用",
        }],
        compensation_receipts: [{ item_id: "item-1", seq: 0, status: "compensated" }],
      },
  });

  const view = render(<DecisionBar />);
  expect(view.getByText("递归影响：6 个资源")).toBeTruthy();
  expect(view.getByText(/受影响标签：资料\/a\.md/)).toBeTruthy();
  expect(view.getByText(/受影响引用：资料引用/)).toBeTruthy();
  await act(async () => {
    fireEvent.click(view.getByRole("button", { name: "确认并执行" }));
  });

  const messages = agStore.getSnapshot().sessions.flatMap((session) => session.messages);
  const lastMessage = messages[messages.length - 1];
  expect(lastMessage).toMatchObject({
      role: "assistant",
      error: true,
      content: expect.stringContaining("partial_failed"),
  });
  expect(lastMessage?.content).toContain("目标被外部占用");
  expect(agStore.getSnapshot().pendingDecision).toBeNull();
  view.unmount();
  agStore.reset();
  vi.mocked(resolveDecisionAndContinue).mockReset();
}

// Migrated intact from src/components/ws/buildTree.test.ts.
{
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
}

// Migrated intact from src/components/ws/FileTreeNode.test.tsx.
{
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
  onEditDocx?: (path: string) => void;
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
      onEditDocx={opts.onEditDocx}
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

  it("preview_only（预览类型）：xlsx 保持可点击行，并叠加只读预览眼睛标记", () => {
    const { container } = renderNode({ docType: "xlsx", openPath: "a.md" });
    const btn = container.querySelector(".ftn-file") as HTMLButtonElement;
    expect(btn.className).toContain("ftn-open");
    expect(btn.className).toContain("ftn-preview_only");
    // 只读预览标记 = .ftn-mode 徽章内含眼睛 SVG 图标（2026-08-04 VIEW 文本改为图标）。
    expect(container.querySelector(".ftn-mode svg")).toBeTruthy();
    expect(btn.disabled).toBe(false); // 预览类型可点击触发只读预览（PI-FE-22）
  });

  it("pdf（原生 PDF）走只读预览，不误当文本打开（2026-08-22：修复 WS_NOT_UTF8 误开二进制）", () => {
    const open = vi.fn();
    const preview = vi.fn();
    const { container } = renderNode({
      node: fileNode("pdf", { name: "a.pdf", path: "a.pdf" }),
      onOpenFile: open,
      onPreviewFile: preview,
    });
    const btn = container.querySelector(".ftn-file") as HTMLButtonElement;
    expect(btn.className).toContain("ftn-preview_only");
    expect(btn.disabled).toBe(false);
    fireEvent.click(btn);
    expect(preview).toHaveBeenCalledWith("a.pdf");
    expect(open).not.toHaveBeenCalled();
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
    expect(native.container.querySelector(".ftn-mode svg")).toBeTruthy();
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

  it("原生 docx 显示编辑按钮，点击触发创建草稿编辑且不触发预览（stopPropagation）", () => {
    const edit = vi.fn();
    const preview = vi.fn();
    const { container } = renderNode({
      node: fileNode("docx", { name: "native.docx", path: "native.docx" }),
      onEditDocx: edit,
      onPreviewFile: preview,
    });
    const editBtn = container.querySelector(".ftn-mode-edit") as HTMLElement;
    expect(editBtn).toBeTruthy();
    // 点击编辑按钮 → 只触发 onEditDocx，不触发行点击的预览打开
    fireEvent.click(editBtn);
    expect(edit).toHaveBeenCalledWith("native.docx");
    expect(preview).not.toHaveBeenCalled();
    // 键盘可达（Enter 同样触发，不冒泡）
    edit.mockClear();
    fireEvent.keyDown(editBtn, { key: "Enter" });
    expect(edit).toHaveBeenCalledWith("native.docx");
    expect(preview).not.toHaveBeenCalled();
  });

  it("编辑按钮仅对原生 docx 显示（xlsx/BDocx 无编辑按钮）", () => {
    const edit = vi.fn();
    const xlsx = renderNode({ docType: "xlsx", onEditDocx: edit });
    expect(xlsx.container.querySelector(".ftn-mode-edit")).toBeNull();
    xlsx.unmount();
    const bdocx = renderNode({
      node: fileNode("docx", { name: "generated.docx", path: "generated.docx", isBDocx: true }),
      onEditDocx: edit,
    });
    expect(bdocx.container.querySelector(".ftn-mode-edit")).toBeNull();
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

  it("文件节点拖出时经统一 drag session 通道写入 ReferencePayload（REF-REFERENCE-REPAIR-01 F1）", () => {
    const { container } = renderNode({});
    const btn = container.querySelector(".ftn-file") as HTMLButtonElement;
    const { values, dataTransfer } = dragData();

    fireEvent.dragStart(btn, { dataTransfer });

    expect(JSON.parse(values.get(REFERENCE_MIME) ?? "{}")).toEqual({ kind: "doc", relPath: "a.md" });
    expect(values.get("text/plain")).toMatch(/^binder-ref:/);
  });
});

describe("FileTreeNode 右键（CTX-MENU-01）", () => {
  function renderCtx(node: TreeNode, onContextMenu: (path: string, x: number, y: number, kind: "dir" | "file") => void) {
    return render(
      <FileTreeNode
        node={node}
        depth={0}
        openPath={null}
        onOpenFile={() => {}}
        onContextMenu={onContextMenu}
      />,
    );
  }

  it("文件节点右键回调携带 kind=file", () => {
    const onContextMenu = vi.fn();
    const { container } = renderCtx(fileNode(), onContextMenu);
    fireEvent.contextMenu(container.querySelector(".ftn-file") as HTMLElement, { clientX: 11, clientY: 22 });
    expect(onContextMenu).toHaveBeenCalledWith("a.md", 11, 22, "file");
  });

  it("目录节点右键回调携带 kind=dir", () => {
    const onContextMenu = vi.fn();
    const { container } = renderCtx({ name: "notes", path: "notes", kind: "dir", children: [] }, onContextMenu);
    fireEvent.contextMenu(container.querySelector(".ftn-dir-row") as HTMLElement, { clientX: 33, clientY: 44 });
    expect(onContextMenu).toHaveBeenCalledWith("notes", 33, 44, "dir");
  });
});
}

// Migrated intact from src/store/workspaceShell.test.ts.
{
beforeEach(() => {
  workspaceShell._projectActive(null);
  workspaceShell._projectRecent([]);
  workspaceShell.setNotice(null);
});

describe("workspaceShell 外壳态投影（X-FORBID-007）", () => {
  it("投影活动 workspace；置 null 回欢迎态", () => {
    workspaceShell._projectActive({ workspaceId: "w1", rootPath: "/p" });
    expect(workspaceShell.getSnapshot().activeWorkspace).toEqual({ workspaceId: "w1", rootPath: "/p" });
    workspaceShell._projectActive(null);
    expect(workspaceShell.getSnapshot().activeWorkspace).toBeNull();
  });

  it("最近列表 loading → ready / error", () => {
    workspaceShell._projectRecentLoading();
    expect(workspaceShell.getSnapshot().recentStatus).toBe("loading");
    workspaceShell._projectRecent([{ workspaceId: "w1", rootPath: "/p" }]);
    expect(workspaceShell.getSnapshot()).toMatchObject({ recentStatus: "ready", recent: [{ workspaceId: "w1", rootPath: "/p" }] });
    workspaceShell._projectRecentError("e");
    expect(workspaceShell.getSnapshot()).toMatchObject({ recentStatus: "error", recentError: "e" });
  });

  it("死项目提示 notice 可设可清", () => {
    workspaceShell.setNotice("项目不可用");
    expect(workspaceShell.getSnapshot().notice).toBe("项目不可用");
    workspaceShell.setNotice(null);
    expect(workspaceShell.getSnapshot().notice).toBeNull();
  });
});
}

// Migrated intact from src/store/wsStore.test.ts.
{
beforeEach(() => wsStore.reset());

describe("wsStore 文件树状态投影", () => {
  it("初始 idle；reset 回 idle 空集", () => {
    expect(wsStore.getSnapshot()).toEqual({ docs: [], status: "idle", error: null });
  });

  it("loading → ready（投影 docs）", () => {
    wsStore._projectLoading();
    expect(wsStore.getSnapshot().status).toBe("loading");
    wsStore._projectDocs([{ relPath: "a.md", docType: "md", isDir: false }]);
    const s = wsStore.getSnapshot();
    expect(s.status).toBe("ready");
    expect(s.docs).toHaveLength(1);
    expect(s.error).toBeNull();
  });

  it("error 投影错误信息", () => {
    wsStore._projectError("boom");
    expect(wsStore.getSnapshot()).toMatchObject({ status: "error", error: "boom" });
  });

  it("subscribe 收到变更通知", () => {
    let calls = 0;
    const unsub = wsStore.subscribe(() => (calls += 1));
    wsStore._projectLoading();
    wsStore._projectError("x");
    unsub();
    wsStore._projectLoading();
    expect(calls).toBe(2); // 退订后不再收到
  });
});
}

// FileTree「＋」新建下拉 + 激活区域（WS-NEW-BUTTON-01）：wsService/edService 已 mock，仅验 UI 交互与命令组装。
{
afterEach(cleanup);

const dir = (relPath: string): DocRefView => ({ relPath, docType: "", isDir: true });
const doc = (relPath: string, docType = "md"): DocRefView => ({ relPath, docType, isDir: false });

const mockedCreateDoc = vi.mocked(createDoc);
const mockedCreateFolder = vi.mocked(createFolder);
const mockedDeleteFolder = vi.mocked(deleteFolder);
const mockedOpenDocument = vi.mocked(openDocument);

/** ContextMenu 菜单项用 onMouseDown 触发选择（ContextMenu.tsx），fireEvent.mouseDown 模拟。 */
function clickMenuItem(container: HTMLElement, label: string) {
  const item = [...container.querySelectorAll(".ctx-menu-item")].find((b) => b.textContent === label);
  if (!item) throw new Error(`菜单项不存在: ${label}`);
  fireEvent.mouseDown(item);
}

function renderFileTree(docs: DocRefView[]) {
  wsStore._projectDocs(docs);
  return render(<FileTree />);
}

beforeEach(() => {
  wsStore.reset();
  vi.clearAllMocks();
});

describe("FileTree「＋」新建下拉（WS-NEW-BUTTON-01）", () => {
  it("点击＋弹出格式下拉，选文件夹后提交触发 ws.create_folder（根目录）", () => {
    const { container } = renderFileTree([dir("notes"), doc("a.md")]);
    fireEvent.click(container.querySelector('button[title="新建文件"]')!);
    const labels = [...container.querySelectorAll(".ctx-menu-item")].map((b) => b.textContent);
    expect(labels).toEqual([
      "文件夹",
      "Markdown (.md)",
      "纯文本 (.txt)",
      "HTML (.html)",
      "Word 文档 (.docx)",
      "CSV (.csv)",
      "自定义格式…",
    ]);
    clickMenuItem(container, "文件夹");
    const input = container.querySelector(".ftn-input") as HTMLInputElement;
    expect(input.placeholder).toBe("文件夹名");
    fireEvent.change(input, { target: { value: "docs" } });
    fireEvent.submit(input.closest("form")!);
    expect(mockedCreateFolder).toHaveBeenCalledWith("docs");
    expect(mockedCreateDoc).not.toHaveBeenCalled();
  });

  it("目录行点击激活区域（高亮 + 头部显示），Markdown 新建自动补扩展名并落在激活目录", () => {
    const { container } = renderFileTree([dir("notes"), doc("notes/b.md"), doc("a.md")]);
    const dirRow = container.querySelector(".ftn-dir-row") as HTMLButtonElement;
    fireEvent.click(dirRow);
    expect(dirRow.className).toContain("ftn-dir-active");
    expect(container.querySelector(".ftn-active-dir")!.textContent).toContain("notes");
    // ＋ → Markdown：命名输入预填激活目录前缀，提交落点在激活目录
    fireEvent.click(container.querySelector('button[title="新建文件"]')!);
    clickMenuItem(container, "Markdown (.md)");
    const input = container.querySelector(".ftn-input") as HTMLInputElement;
    expect(input.value).toBe("notes/");
    fireEvent.change(input, { target: { value: "笔记" } });
    fireEvent.submit(input.closest("form")!);
    expect(mockedCreateDoc).toHaveBeenCalledWith("notes/笔记.md");
  });

  it("固定格式已有扩展名不重复追加；自定义格式保留任意扩展名", async () => {
    const { container } = renderFileTree([]);
    // Markdown 输入 a.md → 不重复追加
    fireEvent.click(container.querySelector('button[title="新建文件"]')!);
    clickMenuItem(container, "Markdown (.md)");
    const mdInput = container.querySelector(".ftn-input") as HTMLInputElement;
    fireEvent.change(mdInput, { target: { value: "a.md" } });
    fireEvent.submit(mdInput.closest("form")!);
    expect(mockedCreateDoc).toHaveBeenLastCalledWith("a.md");
    // 冲刷提交 async 链（createDoc→loadDocs→openDocument→closeOverlays），避免其延迟关闭下一次下拉
    await act(async () => {});
    // 自定义 data.json → 原样传递（不补扩展名）
    fireEvent.click(container.querySelector('button[title="新建文件"]')!);
    clickMenuItem(container, "自定义格式…");
    const customInput = container.querySelector(".ftn-input") as HTMLInputElement;
    expect(customInput.placeholder).toContain("data.json");
    fireEvent.change(customInput, { target: { value: "data.json" } });
    fireEvent.submit(customInput.closest("form")!);
    expect(mockedCreateDoc).toHaveBeenLastCalledWith("data.json");
  });

  it("目录右键「删除」：空目录直接删（mode=empty，无确认）", () => {
    const { container } = renderFileTree([dir("empty"), doc("a.md")]);
    fireEvent.contextMenu(container.querySelector(".ftn-dir-row")!, { clientX: 1, clientY: 2 });
    const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(true);
    clickMenuItem(container, "删除");
    expect(confirmSpy).not.toHaveBeenCalled();
    expect(mockedDeleteFolder).toHaveBeenCalledWith("empty", "empty");
    confirmSpy.mockRestore();
  });

  it("目录右键「删除」：非空目录确认后递归删（mode=recursive）", async () => {
    const { container } = renderFileTree([dir("notes"), doc("notes/b.md"), doc("a.md")]);
    fireEvent.contextMenu(container.querySelector(".ftn-dir-row")!, { clientX: 1, clientY: 2 });
    const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(true);
    clickMenuItem(container, "删除");
    expect(confirmSpy).toHaveBeenCalled();
    expect(mockedDeleteFolder).toHaveBeenCalledWith("notes", "recursive");
    confirmSpy.mockRestore();
    assertUnsavedTabCloseGuard();
    cleanup();
    await assertWorkspaceOperationDecisionProjection();
  });
});

describe("FileTree 文档打开失败日志", () => {
  it("文件树打开失败时捕获 Promise，并记录可诊断的错误字段", async () => {
    edStore.reset();
    tabsStore.reset();
    mockedOpenDocument.mockRejectedValueOnce({ code: "WS_NOT_FOUND", message: "文件不存在" });
    const errorSpy = vi.spyOn(console, "error").mockImplementation(() => undefined);
    const { container } = renderFileTree([doc("gone.md")]);

    fireEvent.click(container.querySelector(".ftn-file") as HTMLButtonElement);
    await act(async () => {});

    expect(errorSpy).toHaveBeenCalledWith("[ws-file-tree]", expect.objectContaining({
      event: "document_open_failed",
      source: "file_tree",
      rel_path: "gone.md",
      error_code: "WS_NOT_FOUND",
      error_message: "文件不存在",
    }));
    errorSpy.mockRestore();
  });
});
}
