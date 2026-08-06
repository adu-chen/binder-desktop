import { useState } from "react";
import { ZOOM, stepZoom, formatZoom } from "../ed/zoom";
import { TbBtn, Sep, ToolbarMenu } from "../ed/edToolbarParts";
import { isNativeDocxPreview } from "../../services/docService";

type PreviewMode = "pdf" | "video" | "audio" | "image" | "table";
type ExportFormat = "pdf" | "docx" | "md";

/** 预览工具栏的导出能力矩阵：按扩展名决定可导出目标（复用 prv.export；csv/tsv 后端未确认前不暴露）。 */
export function exportTargetsForPreview(path: string): ExportFormat[] {
  const ext = path.split(".").pop()?.toLowerCase() ?? "";
  if (ext === "docx" || ext === "doc") return ["pdf", "md"];
  if (ext === "xlsx" || ext === "xls" || ext === "pptx" || ext === "ppt") return ["pdf"];
  return [];
}

/** 是否原生 docx 预览（可"创建草稿进行编辑"）。 */
export function canCreateDraft(path: string): boolean {
  return isNativeDocxPreview(path);
}

/**
 * @GOV
 * codes:    BR-PRV-DATA-001-RB-PRV-PRV-VIEW-004, BR-PRV-DATA-002-RB-PRV-PRV-VIEW-004
 * type:     RB
 * chain:    PRV-VIEW
 * rules:    BR-PRV-DATA-001, BR-PRV-DATA-002
 * boundary: in=PreviewModeAndPreviewPath | out=ModeAdaptiveReadOnlyPreviewToolbar(createDraftOrExportOrPrintOrZoomViewState) | does_not=expose_edit_commands_or_close_tool
 */
export function PrvToolbar({
  mode,
  path,
  status,
  zoom,
  onZoomChange,
  onEnterEdit,
  onExport,
  onPrint,
  onOpenExternal,
}: {
  mode: PreviewMode;
  path: string;
  /** 预览加载态：pdf 打印需 ready；table 由 TablePreview 自管、视为可用。 */
  status: "loading" | "ready" | "error";
  /** 当前缩放（image/table 生效；PrvPanel 持有并施加于 body 容器）。 */
  zoom: number;
  onZoomChange: (zoom: number) => void;
  /** 创建草稿进行编辑（仅原生 docx pdf 模式触发）。 */
  onEnterEdit: () => void;
  /** 导出为指定格式（复用 prv.export，落 workspace 内）。 */
  onExport: (format: ExportFormat) => Promise<{ ok: boolean; message?: string }>;
  /** 打印当前预览（pdf 为 iframe 系统打印；table 为 window.print）。 */
  onPrint: () => void;
  /** 本地打开（系统默认应用打开，只读查看；PRV-PREVIEW-OPT-01）。 */
  onOpenExternal?: () => void;
}) {
  const [exportState, setExportState] = useState<"idle" | "exporting" | "success" | "error">("idle");
  const [exportMessage, setExportMessage] = useState("");

  const exportTargets = exportTargetsForPreview(path);
  const canCreate = mode === "pdf" && canCreateDraft(path);
  // pdf/table 均可打印；pdf 未就绪（转换中/失败）时禁用，不静默（BR-PRV-DATA-002 失败态明确）。
  const canPrint = mode === "pdf" || mode === "table";
  const printDisabled = mode === "pdf" && status !== "ready";
  // 缩放仅对 image/table 生效（CSS zoom 视图态）；pdf 由 iframe 内 WKWebView 原生查看器控制。
  const canZoom = mode === "image" || mode === "table";

  const runExport = async (format: ExportFormat) => {
    if (exportState === "exporting") return;
    setExportState("exporting");
    setExportMessage("正在导出…");
    const outcome = await onExport(format);
    setExportState(outcome.ok ? "success" : "error");
    setExportMessage(outcome.message ?? (outcome.ok ? "导出完成" : "导出失败"));
  };

  return (
    <div className="ed-toolbar prv-toolbar" role="toolbar" aria-label="预览工具栏">
      <div className="ed-tb-row">
        {canCreate && (
          <>
            <TbBtn title="创建草稿进行编辑" onClick={onEnterEdit}>创建草稿进行编辑</TbBtn>
            <Sep />
          </>
        )}
        {exportTargets.length > 0 && (
          <>
            <ToolbarMenu label="导出" title="导出文档">
              {exportTargets.map((format) => (
                <TbBtn
                  key={format}
                  title={`导出为 ${format.toUpperCase()}`}
                  disabled={exportState === "exporting"}
                  onClick={() => { void runExport(format); }}
                >
                  导出 {format.toUpperCase()}
                </TbBtn>
              ))}
              {exportMessage && (
                <span className={`ed-tb-action-state ${exportState}`} role="status">{exportMessage}</span>
              )}
            </ToolbarMenu>
            <Sep />
          </>
        )}
        {canZoom && (
          <>
            <TbBtn title="缩小" onClick={() => onZoomChange(stepZoom(zoom, -ZOOM.step))} disabled={zoom <= ZOOM.min}>−</TbBtn>
            <button
              type="button"
              className="ed-tb-zoom"
              title="重置缩放（100%）"
              aria-label={"当前缩放 " + formatZoom(zoom) + "，点击重置为 100%"}
              onMouseDown={(event) => event.preventDefault()}
              onClick={() => onZoomChange(ZOOM.default)}
            >
              {formatZoom(zoom)}
            </button>
            <TbBtn title="放大" onClick={() => onZoomChange(stepZoom(zoom, ZOOM.step))} disabled={zoom >= ZOOM.max}>+</TbBtn>
          </>
        )}
        {canPrint && (
          <>
            {canZoom && <Sep />}
            <TbBtn title="打印" disabled={printDisabled} onClick={onPrint}>打印</TbBtn>
          </>
        )}
        {onOpenExternal && (
          <>
            <Sep />
            <TbBtn title="在本地应用中打开" onClick={onOpenExternal}>本地打开</TbBtn>
          </>
        )}
      </div>
    </div>
  );
}

export type { PreviewMode, ExportFormat };
