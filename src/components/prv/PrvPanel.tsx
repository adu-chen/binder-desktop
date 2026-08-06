import { useEffect, useRef, useState } from "react";
import { tabsStore } from "../../store/tabsStore";
import { previewDoc, mediaSrc, exportDoc, openExternal } from "../../services/prvService";
import { TablePreview } from "./TablePreview";
import { isTableFile } from "../../services/tableData";
import { createDocxDraft } from "../../services/docService";
import { openDocument } from "../../services/edService";
import { PrvToolbar } from "./PrvToolbar";
import { ZOOM, stepZoom, clampZoom } from "../ed/zoom";

/**
 * PRV 预览面板（PRV-VIEW · 只读硬边界 · PI-FE-22/24 · 2026-07-03 #1 表格式预览 · PRV-PREVIEW-OPT-01 缓存链路）。
 * 按扩展名分流：
 * - docx/PPT → prv.preview（DOC 链）出 PDF，缓存落盘 + asset 授权 → iframe 直接加载 asset URL；
 * - csv/tsv/xlsx/xls → 表格式预览（TablePreview：行列/多 sheet/可选中复制）；
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
  const bodyRef = useRef<HTMLDivElement>(null);
  const zoomRef = useRef(zoom);
  zoomRef.current = zoom;

  // 鼠标/触控板缩放适配（image/table 视图态，PRV-PREVIEW-OPT-01 追加）：
  // - Ctrl/Cmd + 滚轮 → 步进缩放（普通滚轮留给滚动，不拦截）；
  // - 触控板捏合 → WebKit gesture 事件（scale 相对手势起始基线）。
  // 只作用于 CSS zoom 视图态（--prv-zoom），无文档模型变更、无脏态（BR-PRV-DATA-001）。
  // pdf 模式不在此列：iframe 内 WKWebView 原生查看器自持缩放（悬浮条 + 原生捏合）。
  useEffect(() => {
    if (mode !== "image" && mode !== "table") return;
    const el = bodyRef.current;
    if (!el) return;
    const onWheel = (event: WheelEvent) => {
      if (!event.ctrlKey && !event.metaKey) return;
      event.preventDefault();
      setZoom((z) => stepZoom(z, event.deltaY < 0 ? ZOOM.step : -ZOOM.step));
    };
    let base = zoomRef.current;
    const onGestureStart = () => {
      base = zoomRef.current;
    };
    const onGestureChange = (event: Event) => {
      const scale = (event as unknown as { scale?: number }).scale;
      if (typeof scale !== "number" || !Number.isFinite(scale)) return;
      setZoom(clampZoom(base * scale));
    };
    el.addEventListener("wheel", onWheel, { passive: false });
    el.addEventListener("gesturestart", onGestureStart);
    el.addEventListener("gesturechange", onGestureChange);
    return () => {
      el.removeEventListener("wheel", onWheel);
      el.removeEventListener("gesturestart", onGestureStart);
      el.removeEventListener("gesturechange", onGestureChange);
    };
  }, [mode]);

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

  // 本地打开：系统默认应用打开 workspace 内文件（只读查看，X-FORBID-004 守界）
  const openLocal = async () => {
    try {
      await openExternal(path);
    } catch (e) {
      const err = e as { code?: string; message?: string };
      setErrMsg(err?.message ? `[${err.code ?? "?"}] ${err.message}` : String(e));
      setStatus("error");
    }
  };

  useEffect(() => {
    let alive = true;
    if (mode === "table") return; // 表格式预览由 TablePreview 自管数据加载
    setStatus("loading");
    setUrl(null);
    const loader = mode === "pdf" ? previewDoc(path) : mediaSrc(path);
    loader
      .then((u) => {
        if (!alive) return;
        setUrl(u); // asset URL 无需 revoke
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
        onOpenExternal={() => void openLocal()}
      />
      <div ref={bodyRef} className="prv-body" style={{ "--prv-zoom": String(zoom) } as React.CSSProperties}>
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
