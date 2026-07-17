import { useState } from "react";
import { writeDocRefData } from "../../services/edLocate";

/**
 * 文件树单节点（A-SYS-C-T-02 §3 Side Panel WS · 参考稿 §6.4）。
 * FileNodeState 是会话状态：normal · open（锈红左条）· dirty_open（红点）· pending_diff（数字 badge）。
 * 只读预览不是禁用态，而是打开模式标记；它可与 open 等状态叠加。
 */

export type FileNodeState =
  | "normal"
  | "open"
  | "dirty_open"
  | "pending_diff";

export interface TreeNode {
  name: string;
  path: string; // rel_path（文件）或目录相对路径
  kind: "dir" | "file";
  docType?: string;
  isBDocx?: boolean;
  children?: TreeNode[];
}

/** 仅预览不可编辑的类型（APC §1：Excel/PPT 仅预览；2026-07-03 #1 补 csv/tsv——表格式只读预览，
 *  修复 csv 误入编辑态）。点击 → PRV 预览（xlsx/csv 走 TablePreview，pptx 走 PDF）。
 *  注：docx/doc 可编辑——经 DOC 编辑链（doc.open_for_edit/save_from_edit）打开进 TipTap，点击 → 编辑。 */
const PREVIEW_ONLY = new Set(["xlsx", "xls", "pptx", "ppt", "csv", "tsv"]);

/** 音视频/图片类型（asset 协议预览）；可点击预览。 */
const MEDIA_TYPES = new Set([
  "mp4", "webm", "mov", "m4v", "ogv", "mp3", "wav", "ogg", "m4a", "flac", "aac",
  "png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "avif", "ico",
]);

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
    case "csv":
    case "tsv":
      return "CSV";
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
  /** 后续 PI 注入的动态状态覆盖（dirty_open/pending_diff），按 path 查。 */
  stateOverride?: Record<string, FileNodeState>;
  /** 后续 PI 注入的待裁决计数，按 path 查（pending_diff badge）。 */
  pendingCount?: Record<string, number>;
  /** 右键文件节点（重命名/删除菜单，PI-FE-13）。 */
  onContextMenu?: (path: string, x: number, y: number) => void;
  /** 预览类型（docx/Excel/PPT/媒体/图片）点击 → 只读预览（PI-FE-22/23/24）。 */
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

  // 文件节点：只读预览是打开模式，不是禁用态；视觉用轻量 VIEW 标记，不降低文件名层级。
  // docx 成熟流程：原生 docx 点击 → 只读预览（经"创建 b-docx"进编辑）；BDocx（草稿/文件树新建/Agent 创建）
  //   点击 → 直接编辑（与 md 一致）。Excel/PPT/CSV/媒体/图片恒只读预览。
  const dt = node.docType ?? "";
  const isDocx = dt === "docx" || dt === "doc";
  const isEditableDocx = isDocx && (node.isBDocx || node.name.toLowerCase().includes(".draft."));
  const nativeDocx = isDocx && !isEditableDocx;
  const officePreview = PREVIEW_ONLY.has(dt) || nativeDocx;
  const mediaPreview = MEDIA_TYPES.has(dt);
  const previewable = officePreview || mediaPreview;
  const overridden = stateOverride?.[node.path];
  const isOpen = openPath === node.path;
  const nodeState: FileNodeState = overridden ?? (isOpen ? "open" : "normal");
  const badge = pendingCount?.[node.path];

  return (
    <button
      className={`ftn-row ftn-file ftn-${nodeState}${previewable ? " ftn-preview_only" : ""}`}
      style={indent}
      draggable
      onDragStart={(e) => writeDocRefData(e.dataTransfer, node.path)}
      onClick={() => (previewable ? onPreviewFile?.(node.path) : onOpenFile(node.path))}
      onContextMenu={(e) => {
        if (!onContextMenu) return;
        e.preventDefault();
        onContextMenu(node.path, e.clientX, e.clientY);
      }}
      title={previewable ? `${node.path}（只读预览）` : node.path}
    >
      <span className="ftn-type">{typeGlyph(node.docType)}</span>
      <span className="ftn-name">{node.name}</span>
      {previewable && <span className="ftn-mode" aria-label="只读预览">VIEW</span>}
      {nodeState === "dirty_open" && <span className="ftn-dot" aria-label="未保存" />}
      {nodeState === "pending_diff" && badge != null && <span className="ftn-badge">{badge}</span>}
    </button>
  );
}
