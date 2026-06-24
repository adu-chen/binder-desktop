import { useState } from "react";

/**
 * 文件树单节点（A-SYS-C-T-02 §3 Side Panel WS · 参考稿 §6.4）。
 * FileNodeState 6 状态视觉互斥：
 *   normal · open（锈红左条）· dirty_open（红点）· pending_diff（数字 badge）·
 *   external_changed（金 !）· not_supported（灰禁用，xlsx/pptx 仅预览不可编辑）。
 * PI-FE-04 接通 normal + not_supported（由 docType 推断）；其余状态预留，
 * 由后续 PI 的 ed/de 投影经 stateOverride 注入。
 */

export type FileNodeState =
  | "normal"
  | "open"
  | "dirty_open"
  | "pending_diff"
  | "external_changed"
  | "not_supported";

export interface TreeNode {
  name: string;
  path: string; // rel_path（文件）或目录相对路径
  kind: "dir" | "file";
  docType?: string;
  children?: TreeNode[];
}

/** 仅预览不可编辑的类型（APC §1：Excel/PPT 仅预览；docx 经 DOC 预览链只读呈现）。点击 → PRV 预览。 */
const PREVIEW_ONLY = new Set(["docx", "doc", "xlsx", "xls", "pptx", "ppt"]);

/** 文件类型 mono 字形（参考稿 §10：用 mono 字符，实现期可换图标库）。 */
function typeGlyph(docType?: string): string {
  switch (docType) {
    case "md":
    case "markdown":
      return "M↓";
    case "txt":
      return "TXT";
    case "docx":
      return "DOC";
    case "xlsx":
    case "xls":
      return "XLS";
    case "pptx":
    case "ppt":
      return "PPT";
    case "html":
      return "<>";
    default:
      return "·";
  }
}

interface FileTreeNodeProps {
  node: TreeNode;
  depth: number;
  openPath: string | null;
  onOpenFile: (path: string) => void;
  /** 后续 PI 注入的动态状态覆盖（dirty_open/pending_diff/external_changed），按 path 查。 */
  stateOverride?: Record<string, FileNodeState>;
  /** 后续 PI 注入的待裁决计数，按 path 查（pending_diff badge）。 */
  pendingCount?: Record<string, number>;
  /** 右键文件节点（重命名/删除菜单，PI-FE-13）。 */
  onContextMenu?: (path: string, x: number, y: number) => void;
  /** 预览类型（docx/Excel/PPT）点击 → 只读预览（PI-FE-22）。 */
  onPreviewFile?: (path: string) => void;
}

export function FileTreeNode({
  node,
  depth,
  openPath,
  onOpenFile,
  stateOverride,
  pendingCount,
  onContextMenu,
  onPreviewFile,
}: FileTreeNodeProps) {
  const [expanded, setExpanded] = useState(true);
  const indent = { paddingLeft: `${depth * 12 + 8}px` };

  if (node.kind === "dir") {
    return (
      <div className="ftn-dir">
        <button className="ftn-row ftn-dir-row" style={indent} onClick={() => setExpanded((v) => !v)}>
          <span className="ftn-caret">{expanded ? "▾" : "▸"}</span>
          <span className="ftn-name">{node.name}</span>
        </button>
        {expanded &&
          node.children?.map((c) => (
            <FileTreeNode
              key={c.path}
              node={c}
              depth={depth + 1}
              openPath={openPath}
              onOpenFile={onOpenFile}
              stateOverride={stateOverride}
              pendingCount={pendingCount}
              onContextMenu={onContextMenu}
              onPreviewFile={onPreviewFile}
            />
          ))}
      </div>
    );
  }

  // 文件节点：预览类型（不可编辑但可只读预览）视觉 not_supported，但可点击触发预览。
  const previewOnly = PREVIEW_ONLY.has(node.docType ?? "");
  const notSupported = previewOnly;
  const overridden = stateOverride?.[node.path];
  const isOpen = openPath === node.path;
  const nodeState: FileNodeState = notSupported
    ? "not_supported"
    : overridden ?? (isOpen ? "open" : "normal");
  const badge = pendingCount?.[node.path];

  return (
    <button
      className={`ftn-row ftn-file ftn-${nodeState}`}
      style={indent}
      onClick={() => (previewOnly ? onPreviewFile?.(node.path) : onOpenFile(node.path))}
      onContextMenu={(e) => {
        if (!onContextMenu) return;
        e.preventDefault();
        onContextMenu(node.path, e.clientX, e.clientY);
      }}
      title={previewOnly ? `${node.path}（只读预览）` : node.path}
    >
      <span className="ftn-type">{typeGlyph(node.docType)}</span>
      <span className="ftn-name">{node.name}</span>
      {nodeState === "dirty_open" && <span className="ftn-dot" aria-label="未保存" />}
      {nodeState === "external_changed" && <span className="ftn-ext">!</span>}
      {nodeState === "pending_diff" && badge != null && <span className="ftn-badge">{badge}</span>}
    </button>
  );
}
