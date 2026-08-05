import { useEffect, useRef, useState } from "react";
import { tabsStore } from "../../store/tabsStore";
import { previewDoc, mediaSrc, exportDoc } from "../../services/prvService";
import { TablePreview } from "./TablePreview";
import { isTableFile } from "../../services/tableData";
import { createDocxDraft } from "../../services/docService";
import { openDocument } from "../../services/edService";
import { PrvToolbar } from "./PrvToolbar";
import { ZOOM } from "../ed/zoom";

/**
 * PRV 预览面板（PRV-VIEW · 只读硬边界 · PI-FE-22/24 · 2026-07-03 #1 表格式预览）。按扩展名分流：
 * - docx/PPT → prv.preview（DOC 链）出 PDF，字节→blob iframe；
 * - csv/tsv/xlsx/xls → 表格式预览（TablePreview：行列/多 sheet/可选中复制，PDF 保真不达另议管线的兑现）；
 * - 音视频 → prv.media_src（asset 协议）流式喂 <video>/<audio>（不进内存，系统解码）。
 * 加载/失败/不支持各有明确态（不静默空白，BR-PRV-DATA-002）；无任何编辑入口。
 */

const VIDEO = new Set(["mp4", "webm", "mov", "m4v", "ogv"]);
const AUDIO = new Set(["mp3", "wav", "ogg", "m4a", "flac", "aac"]);
const IMAGE = new Set(["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "avif", "ico"]);

type Mode = "pdf" | "video" | "audio" | "image" | "table";
function modeOf(path: string): Mode {
  const ext = path.split(".").pop()?.toLowerCase() ?? "";
  if (VIDEO.has(ext)) return "video";
  if (AUDIO.has(ext)) return "audio";
  if (IMAGE.has(ext)) return "image";
  if (isTableFile(path)) return "table";
  return "pdf";
}

/**
 * @GOV
 * codes:    BR-PRV-DATA-001-GUARD-PRV-PRV-VIEW-003
 * type:     GUARD
 * chain:    PRV-VIEW
 * rules:    BR-PRV-DATA-001
 * boundary: in=previewablePath | out=readOnlyRender(noInPlaceEditEntryDraftSpawnsSeparateEditSession)
 */
/// 预览只读硬边界守则（BR-PRV-DATA-001）：预览面板纯只读呈现（PDF/图片/音视频），不提供任何原地内容
/// 修改交互、不产生文档模型变更与脏态；Excel/PPT 无编辑入口（v1 冻结）。原生 docx 的「创建草稿进行编辑」
/// 不改预览源——复制为 *.draft.docx 后另起独立编辑会话（DOC 编辑链），预览与编辑链不黏连（承 BR-DOC-DATA-001）。
export function PrvPanel({ path }: { path: string }) {
  const mode = modeOf(path);
  const [url, setUrl] = useState<string | null>(null);
  const [status, setStatus] = useState<"loading" | "ready" | "error">("loading");
  const [errMsg, setErrMsg] = useState("");
  const [zoom, setZoom] = useState<number>(ZOOM.default);
  const frameRef = useRef<HTMLIFrameElement>(null);

  const print = () => {
    if (mode === "pdf") frameRef.current?.contentWindow?.print();
    else if (mode === "table") window.print();
  };

  // docx 成熟流程：原生 docx 预览 →"创建草稿进行编辑"→ 复制为 *.draft.docx → 打开草稿进编辑态（HTML）。
  const enterEdit = async () => {
    try {
      const draftPath = await createDocxDraft(path);
      tabsStore.openDoc(draftPath); // 切到编辑 tab（Workbench 渲染 Editor）
      await openDocument(draftPath); // 经 doc.open_for_edit 载入编辑器（docx→HTML 保留样式子集）
    } catch (e) {
      const err = e as { code?: string; message?: string };
      setErrMsg(err?.message ? `[${err.code ?? "?"}] ${err.message}` : String(e));
      setStatus("error");
    }
  };

  // 导出复用 prv.export（落 workspace 内，X-FORBID-004）；失败回错误不静默降级。
  const exportCurrent = async (format: "pdf" | "docx" | "md") => {
    try {
      const destination = await exportDoc(path, format);
      return { ok: true as const, message: `已导出：${destination}` };
    } catch (e) {
      const err = e as { code?: string; message?: string };
      return { ok: false as const, message: err?.message ?? "导出失败" };
    }
  };

  useEffect(() => {
    let revoked: string | null = null;
    let alive = true;
    if (mode === "table") return; // 表格式预览由 TablePreview 自管数据加载
    setStatus("loading");
    setUrl(null);
    const loader = mode === "pdf" ? previewDoc(path) : mediaSrc(path);
    loader
      .then((u) => {
        if (!alive) {
          if (mode === "pdf") URL.revokeObjectURL(u);
          return;
        }
        if (mode === "pdf") revoked = u; // 仅 blob URL 需回收；asset URL 不需
        setUrl(u);
        setStatus("ready");
      })
      .catch((e) => {
        if (!alive) return;
        const err = e as { code?: string; message?: string };
        setErrMsg(err?.message ? `[${err.code ?? "?"}] ${err.message}` : String(e));
        setStatus("error");
      });
    return () => {
      alive = false;
      if (revoked) URL.revokeObjectURL(revoked);
    };
  }, [path, mode]);

  const name = path.split("/").pop();
  return (
    <div className="prv-panel">
      <PrvToolbar
        mode={mode}
        path={path}
        status={status}
        zoom={zoom}
        onZoomChange={setZoom}
        onEnterEdit={() => void enterEdit()}
        onExport={exportCurrent}
        onPrint={print}
      />
      <div className="prv-body" style={{ "--prv-zoom": String(zoom) } as React.CSSProperties}>
        {mode === "table" && <TablePreview path={path} />}
        {mode !== "table" && status === "loading" && (
          <p className="muted">{mode === "pdf" ? "渲染预览中…（首次启动 LibreOffice 可能稍慢）" : "加载中…"}</p>
        )}
        {status === "error" && (
          <p className="prv-error">
            预览失败：{errMsg}
            {errMsg.includes("LIBREOFFICE") || errMsg.includes("soffice") ? "（请确认已安装 LibreOffice）" : ""}
          </p>
        )}
        {status === "ready" && url && mode === "pdf" && (
          <iframe ref={frameRef} className="prv-frame" src={url} title="文档预览" />
        )}
        {status === "ready" && url && mode === "video" && (
          <video className="prv-media" src={url} controls />
        )}
        {status === "ready" && url && mode === "audio" && (
          <audio className="prv-media prv-audio" src={url} controls />
        )}
        {status === "ready" && url && mode === "image" && (
          <img className="prv-media prv-image" src={url} alt={name} />
        )}
      </div>
    </div>
  );
}
