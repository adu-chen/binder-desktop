import { useCallback, useEffect, useState } from "react";
import { useEdStore } from "../../store/edStore";
import { listTimeline, previewNode, restoreNode, type TimelineEntryDto } from "../../services/wsService";
import { openDocument } from "../../services/edService";
import { activeSession, agStore } from "../../store/agStore";

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

  // 保存成功（binder:doc-saved）后刷新：ws.save_doc / doc.save_from_edit 会记时间轴 save 节点
  // （BR-WS-DATA-002，2026-08-04 save 准入），面板停留在当前文档时应即时呈现新历史。
  useEffect(() => {
    const onDocSaved = (e: Event) => {
      const detail = (e as CustomEvent<{ path?: string }>).detail;
      if (detail?.path === openPath) void load();
    };
    window.addEventListener("binder:doc-saved", onDocSaved);
    return () => window.removeEventListener("binder:doc-saved", onDocSaved);
  }, [load, openPath]);

  const select = async (entryId: string) => {
    setSelected(entryId);
    try {
      setPreview(await previewNode(entryId));
    } catch (e) {
      const err = e as { message?: string };
      setPreview(`（预览失败：${err?.message ?? e}）`);
    }
  };

  const restore = async (node: TimelineEntryDto) => {
    if (node.entry_kind === "document" && !window.confirm("还原到此版本？当前内容将被该版本覆盖（会记录一个新的时间轴节点，历史不丢失）。")) return;
    try {
      const result = await restoreNode(
        node.entry_id,
        node.entry_kind === "workspace_operation"
          ? activeSession(agStore.getSnapshot()).id
          : undefined,
      );
      if (result.decision_request) {
        agStore.setPendingDecision({
          requestId: result.decision_request.request_id,
          kind: result.decision_request.kind,
          payload: result.decision_request.payload,
        });
        setPreview("已生成资源恢复确认；确认后系统会重新核验并执行，历史不会被覆盖。");
        return;
      }
      if (openPath) await openDocument(openPath); // 重载编辑器为还原后内容
      await load();
    } catch (e) {
      const err = e as { message?: string };
      window.alert(`还原失败：${err?.message ?? e}`);
    }
  };

  return (
    <>
      <div className="filetree-hd">
        <span className="label">{openPath ? `时间轴 · ${openPath.split("/").pop()}` : "工作区时间轴"}</span>
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
            <span className="tl-seq">{n.entry_kind === "workspace_operation" ? "资源" : `#${n.version_seq}`}</span>
            <span className="tl-label">{n.label ?? "落盘"}</span>
            <span className="tl-time">{new Date(n.created_at).toLocaleTimeString()}</span>
          </button>
        ))}
      </div>
      {selected && (
        <div className="tl-preview">
          <div className="tl-preview-hd">
            <span className="label">版本预览</span>
            <button
              className="tl-restore"
              disabled={nodes.find((node) => node.entry_id === selected)?.recoverable === false}
              onClick={() => {
                const node = nodes.find((candidate) => candidate.entry_id === selected);
                if (node) void restore(node);
              }}
            >
              {nodes.find((node) => node.entry_id === selected)?.entry_kind === "workspace_operation"
                ? "生成恢复确认"
                : "还原此版本"}
            </button>
          </div>
          <pre className="tl-preview-body">{preview}</pre>
        </div>
      )}
    </>
  );
}
