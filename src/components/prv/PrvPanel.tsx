import { useEffect, useRef, useState } from "react";
import { tabsStore } from "../../store/tabsStore";
import { previewDoc, mediaSrc } from "../../services/prvService";
import { createDocxDraft, isNativeDocxPreview } from "../../services/docService";
import { openDocument } from "../../services/edService";

/**
 * PRV 预览面板（PRV-VIEW · 只读硬边界 · PI-FE-22/24）。按扩展名分流：
 * - docx/Excel/PPT → prv.preview（DOC 链）出 PDF，字节→blob iframe；
 * - 音视频 → prv.media_src（asset 协议）流式喂 <video>/<audio>（不进内存，系统解码）。
 * 加载/失败/不支持各有明确态（不静默空白，BR-PRV-DATA-002）；无任何编辑入口。
 */

const VIDEO = new Set(["mp4", "webm", "mov", "m4v", "ogv"]);
const AUDIO = new Set(["mp3", "wav", "ogg", "m4a", "flac", "aac"]);
const IMAGE = new Set(["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "avif", "ico"]);

type Mode = "pdf" | "video" | "audio" | "image";
function modeOf(path: string): Mode {
  const ext = path.split(".").pop()?.toLowerCase() ?? "";
  if (VIDEO.has(ext)) return "video";
  if (AUDIO.has(ext)) return "audio";
  if (IMAGE.has(ext)) return "image";
  return "pdf";
}

export function PrvPanel({ path }: { path: string }) {
  const mode = modeOf(path);
  const [url, setUrl] = useState<string | null>(null);
  const [status, setStatus] = useState<"loading" | "ready" | "error">("loading");
  const [errMsg, setErrMsg] = useState("");
  const frameRef = useRef<HTMLIFrameElement>(null);

  const print = () => frameRef.current?.contentWindow?.print();

  // docx 成熟流程：原生 docx 预览 →"创建草稿进行编辑"→ 复制为 *.draft.docx → 打开草稿进编辑模式（HTML）。
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

  useEffect(() => {
    let revoked: string | null = null;
    let alive = true;
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
      <div className="prv-hd">
        <span className="label">预览（只读） · {name}</span>
        <span className="prv-hd-actions">
          {mode === "pdf" && isNativeDocxPreview(path) && (
            <button className="ghost" onClick={() => void enterEdit()} title="创建草稿进行编辑">
              创建草稿进行编辑
            </button>
          )}
          {mode === "pdf" && (
            <button className="ghost" onClick={print} disabled={status !== "ready"} title="打印">打印</button>
          )}
          <button
            className="ghost"
            onClick={() => {
              const id = tabsStore.getSnapshot().activeTabId;
              if (id) tabsStore.close(id);
            }}
            title="关闭预览"
          >
            关闭
          </button>
        </span>
      </div>
      <div className="prv-body">
        {status === "loading" && (
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
