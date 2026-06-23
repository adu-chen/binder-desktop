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

/** 仅预览不可编辑的类型（APC §1：Excel/PPT 仅预览）。 */
const PREVIEW_ONLY = new Set(["xlsx", "xls", "pptx", "ppt"]);

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
}

export function FileTreeNode({
  node,
  depth,
  openPath,
  onOpenFile,
  stateOverride,
  pendingCount,
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
            />
          ))}
      </div>
    );
  }

  // 文件节点：状态优先级 not_supported > 覆盖态 > open > normal。
  const notSupported = PREVIEW_ONLY.has(node.docType ?? "");
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
      disabled={notSupported}
      aria-disabled={notSupported}
      onClick={() => !notSupported && onOpenFile(node.path)}
      title={node.path}
    >
      <span className="ftn-type">{typeGlyph(node.docType)}</span>
      <span className="ftn-name">{node.name}</span>
      {nodeState === "dirty_open" && <span className="ftn-dot" aria-label="未保存" />}
      {nodeState === "external_changed" && <span className="ftn-ext">!</span>}
      {nodeState === "pending_diff" && badge != null && <span className="ftn-badge">{badge}</span>}
    </button>
  );
}
