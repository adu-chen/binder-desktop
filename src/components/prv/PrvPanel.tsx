import { useEffect, useState } from "react";
import { prvStore } from "../../store/prvStore";
import { previewDoc } from "../../services/prvService";

/**
 * PRV 预览面板（PRV-VIEW · 只读硬边界 · PI-FE-22）。
 * docx/Excel/PPT 经 prv.preview（DOC 预览链）出 PDF，字节→blob URL 以 iframe 只读呈现。
 * 加载/失败/不支持/LibreOffice 未装各有明确态（不静默空白，BR-PRV-DATA-002）；无任何编辑入口。
 */
export function PrvPanel({ path }: { path: string }) {
  const [url, setUrl] = useState<string | null>(null);
  const [status, setStatus] = useState<"loading" | "ready" | "error">("loading");
  const [errMsg, setErrMsg] = useState("");

  useEffect(() => {
    let revoked: string | null = null;
    let alive = true;
    setStatus("loading");
    setUrl(null);
    previewDoc(path)
      .then((u) => {
        if (!alive) {
          URL.revokeObjectURL(u);
          return;
        }
        revoked = u;
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
  }, [path]);

  const name = path.split("/").pop();
  return (
    <div className="prv-panel">
      <div className="prv-hd">
        <span className="label">预览（只读） · {name}</span>
        <button className="ghost" onClick={() => prvStore.close()} title="关闭预览">关闭</button>
      </div>
      <div className="prv-body">
        {status === "loading" && <p className="muted">渲染预览中…（首次启动 LibreOffice 可能稍慢）</p>}
        {status === "error" && (
          <p className="prv-error">
            预览失败：{errMsg}
            {errMsg.includes("LIBREOFFICE") || errMsg.includes("soffice") ? "（请确认已安装 LibreOffice）" : ""}
          </p>
        )}
        {status === "ready" && url && <iframe className="prv-frame" src={url} title="文档预览" />}
      </div>
    </div>
  );
}
