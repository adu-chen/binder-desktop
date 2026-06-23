import { useCallback, useEffect, useState } from "react";
import { useEdStore } from "../../store/edStore";
import { listTimeline, previewNode, restoreNode, type TimelineEntryDto } from "../../services/wsService";
import { openDocument } from "../../services/edService";

/**
 * 时间轴面板（A-SYS-C-T-02 §3 时间轴归 WS · DocumentTimeline/TERM-WS-002 · PI-FE-16）。
 * 随当前打开文档加载 ws.list_timeline；点节点 ws.preview_node 预览历史版本；还原 ws.restore_node
 * （写回 + 前进式记新节点）。历史/还原态以后端为准（X-FORBID-007）。
 */
export function TimelinePanel() {
  const { openPath } = useEdStore();
  const [nodes, setNodes] = useState<TimelineEntryDto[]>([]);
  const [status, setStatus] = useState<"idle" | "loading" | "ready" | "error">("idle");
  const [selected, setSelected] = useState<string | null>(null);
  const [preview, setPreview] = useState<string>("");

  const load = useCallback(async () => {
    if (!openPath) return;
    setStatus("loading");
    try {
      const list = await listTimeline(openPath);
      setNodes([...list].reverse()); // 最近版本在前
      setStatus("ready");
    } catch {
      setStatus("error");
    }
  }, [openPath]);

  useEffect(() => {
    setSelected(null);
    setPreview("");
    void load();
  }, [load]);

  const select = async (entryId: string) => {
    setSelected(entryId);
    try {
      setPreview(await previewNode(entryId));
    } catch (e) {
      const err = e as { message?: string };
      setPreview(`（预览失败：${err?.message ?? e}）`);
    }
  };

  const restore = async (entryId: string) => {
    if (!window.confirm("还原到此版本？当前内容将被该版本覆盖（会记录一个新的时间轴节点，历史不丢失）。")) return;
    try {
      await restoreNode(entryId);
      if (openPath) await openDocument(openPath); // 重载编辑器为还原后内容
      await load();
    } catch (e) {
      const err = e as { message?: string };
      window.alert(`还原失败：${err?.message ?? e}`);
    }
  };

  if (!openPath) {
    return (
      <>
        <div className="filetree-hd"><span className="label">时间轴</span></div>
        <p className="muted ftn-hint">打开文档以查看历史版本</p>
      </>
    );
  }

  return (
    <>
      <div className="filetree-hd">
        <span className="label">时间轴 · {openPath.split("/").pop()}</span>
        <button className="ghost ftn-refresh" onClick={() => void load()} title="刷新">↻</button>
      </div>
      {status === "loading" && <p className="muted ftn-hint">加载中…</p>}
      {status === "error" && <p className="ftn-error">加载失败</p>}
      {status === "ready" && nodes.length === 0 && <p className="muted ftn-hint">暂无历史版本（接受修改/还原后产生）</p>}
      <div className="tl-list">
        {nodes.map((n) => (
          <button
            key={n.entry_id}
            className={`tl-node${selected === n.entry_id ? " active" : ""}`}
            onClick={() => void select(n.entry_id)}
          >
            <span className="tl-seq">#{n.version_seq}</span>
            <span className="tl-label">{n.label ?? "落盘"}</span>
            <span className="tl-time">{new Date(n.created_at).toLocaleTimeString()}</span>
          </button>
        ))}
      </div>
      {selected && (
        <div className="tl-preview">
          <div className="tl-preview-hd">
            <span className="label">版本预览</span>
            <button className="tl-restore" onClick={() => void restore(selected)}>还原此版本</button>
          </div>
          <pre className="tl-preview-body">{preview}</pre>
        </div>
      )}
    </>
  );
}
