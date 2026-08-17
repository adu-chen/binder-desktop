import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { makeMemoryReference, requestReferenceInsert } from "../../services/referencePayload";

interface MemoryEntryDto {
  entryId: string;
  memoryKind: string;
  content: string;
  status: string;
  originTurn: string;
}

interface DispatchEnvelope<T> {
  correlation_id: string;
  result?: T;
  error?: { code: string; message: string };
}

/**
 * 记忆库面板（BR-SYS-DATA-007 / TERM-AST-002 · PI-AST-*）：
 * 浏览当前 workspace 的 active 记忆条目，用户**显式选择**构造 MemorySelectionRef（data-only）注入
 * Composer 输入区。仅显式选择进入当前回合（不写回 AST、跨会话不串）；自动写入/自动供给在 1.0 关闭。
 */
export function MemoryPanel() {
  const [entries, setEntries] = useState<MemoryEntryDto[]>([]);
  const [status, setStatus] = useState<"idle" | "loading" | "ready" | "error">("idle");
  const [injected, setInjected] = useState<string | null>(null);

  const load = useCallback(async () => {
    setStatus("loading");
    try {
      const request = {
        command: "ast.list_entries",
        correlation_id: `ast-${Date.now()}`,
        params: {},
      };
      const res = await invoke<DispatchEnvelope<MemoryEntryDto[]>>("dispatch", { request });
      if (res?.error) throw res.error;
      setEntries(res?.result ?? []);
      setStatus("ready");
    } catch {
      setStatus("error");
    }
  }, []);

  useEffect(() => {
    setInjected(null);
    void load();
  }, [load]);

  const inject = (entry: MemoryEntryDto) => {
    // MemorySelectionRef：仅显式选择进入当前回合（data-only 快照，不写回 AST）。
    requestReferenceInsert(makeMemoryReference(entry.entryId, entry.memoryKind, entry.content));
    setInjected(entry.entryId);
  };

  return (
    <>
      <div className="filetree-hd">
        <span className="label">记忆库</span>
        <button className="ghost ftn-refresh" onClick={() => void load()} title="刷新">↻</button>
      </div>
      {status === "loading" && <p className="muted ftn-hint">加载中…</p>}
      {status === "error" && <p className="ftn-error">加载失败</p>}
      {status === "ready" && entries.length === 0 && (
        <p className="muted ftn-hint">暂无记忆条目。记忆由用户管理，仅显式选择的条目会进入回合。</p>
      )}
      <ul className="tl-list" aria-label="记忆条目">
        {entries.map((entry) => (
          <li key={entry.entryId} className="tl-node memory-item">
            <span className="tl-seq">{entry.memoryKind}</span>
            <span className="memory-content">{entry.content}</span>
            <button className="tl-restore memory-inject" onClick={() => inject(entry)}>
              {injected === entry.entryId ? "已引用 ✓" : "引用"}
            </button>
          </li>
        ))}
      </ul>
    </>
  );
}
