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

/** 仅预览不可编辑的类型（APC §1：Excel/PPT 仅预览；2026-07-03 #1 补 csv/tsv——表格式只读预览，
 *  修复 csv 误入编辑态）。点击 → PRV 预览（xlsx/csv 走 TablePreview，pptx 走 PDF）。
 *  注：docx/doc 可编辑——经 DOC 编辑链（doc.open_for_edit/save_from_edit）打开进 TipTap，点击 → 编辑。 */
const PREVIEW_ONLY = new Set(["xlsx", "xls", "pptx", "ppt", "csv", "tsv"]);

/** 音视频/图片类型（asset 协议预览）；可点击预览，视觉不灰显（与 office 区分）。 */
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

  // 文件节点：office 预览类型视觉 not_supported（灰显）；音视频正常视觉；点击触发只读预览。
  // docx 成熟流程：原生 docx（非 *.draft.docx）点击 → 只读预览（经"创建草稿"进编辑）；草稿 docx（*.draft.docx）
  //   点击 → 直接编辑（不灰显，与 md 一致）。Excel/PPT 恒只读预览。
  const dt = node.docType ?? "";
  const isDocx = dt === "docx" || dt === "doc";
  const isDraftDocx = isDocx && node.name.toLowerCase().includes(".draft.");
  const nativeDocx = isDocx && !isDraftDocx;
  const officePreview = PREVIEW_ONLY.has(dt) || nativeDocx;
  const mediaPreview = MEDIA_TYPES.has(dt);
  const previewable = officePreview || mediaPreview;
  const notSupported = officePreview;
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
      {nodeState === "dirty_open" && <span className="ftn-dot" aria-label="未保存" />}
      {nodeState === "external_changed" && <span className="ftn-ext">!</span>}
      {nodeState === "pending_diff" && badge != null && <span className="ftn-badge">{badge}</span>}
    </button>
  );
}
