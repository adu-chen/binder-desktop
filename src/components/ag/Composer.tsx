import { useState } from "react";
import { useAgStore } from "../../store/agStore";
import { startTurn, listDocBlocks, type DocBlock } from "../../services/agService";
import { generateSampleDiff } from "../../services/deService";
import { useEdStore } from "../../store/edStore";
import { useWsStore } from "../../store/wsStore";
import { captureSelectionRef } from "../../services/edLocate";

interface BlockRef {
  relPath: string;
  blockId: string;
  text: string;
}

/**
 * Composer（A-SYS-C-T-02 §3 对话区输入 · PI-FE-06）。
 * 输入 + 发送；⌘/Ctrl+Enter 发送。发送中禁用（投影 sending 态）。
 * 文档引用（@mention，PI-AG-09E）：选工作区文档作显式引用，随消息高优先注入。
 */
export function Composer() {
  const { sending } = useAgStore();
  const { openPath } = useEdStore();
  const { docs } = useWsStore();
  const [text, setText] = useState("");
  const [mentions, setMentions] = useState<string[]>([]);
  const [blockRefs, setBlockRefs] = useState<BlockRef[]>([]);
  const [pickerOpen, setPickerOpen] = useState(false);
  const [expandedDoc, setExpandedDoc] = useState<string | null>(null);
  const [blocksByDoc, setBlocksByDoc] = useState<Record<string, DocBlock[]>>({});

  const fileDocs = docs.filter((d) => !d.isDir);

  const expandDocBlocks = async (relPath: string) => {
    if (expandedDoc === relPath) {
      setExpandedDoc(null);
      return;
    }
    if (!blocksByDoc[relPath]) {
      try {
        const blocks = await listDocBlocks(relPath);
        setBlocksByDoc((m) => ({ ...m, [relPath]: blocks }));
      } catch {
        setBlocksByDoc((m) => ({ ...m, [relPath]: [] }));
      }
    }
    setExpandedDoc(relPath);
  };

  const addDocBlockRef = (relPath: string, b: DocBlock) => {
    setBlockRefs((cur) => [...cur, { relPath, blockId: b.block_id ?? "", text: b.text }]);
  };

  const send = () => {
    if (sending || !text.trim()) return;
    const opts =
      mentions.length || blockRefs.length ? { mentions, blockRefs } : undefined;
    void startTurn(text, opts);
    setText("");
    setMentions([]);
    setBlockRefs([]);
    setPickerOpen(false);
  };

  const addSelectionRef = () => {
    const ref = captureSelectionRef();
    if (!ref || !openPath) return;
    setBlockRefs((cur) =>
      cur.some((r) => r.blockId === ref.blockId && r.relPath === openPath)
        ? cur
        : [...cur, { relPath: openPath, blockId: ref.blockId, text: ref.text }],
    );
  };

  const toggleMention = (relPath: string) => {
    setMentions((cur) => (cur.includes(relPath) ? cur.filter((p) => p !== relPath) : [...cur, relPath]));
  };

  return (
    <div className="composer">
      {mentions.length > 0 && (
        <div className="composer-mentions">
          {mentions.map((p) => (
            <span key={p} className="mention-chip">
              @{p}
              <button className="mention-chip-x" onClick={() => toggleMention(p)} aria-label="移除引用">
                ×
              </button>
            </span>
          ))}
        </div>
      )}

      {blockRefs.length > 0 && (
        <div className="composer-mentions">
          {blockRefs.map((r, i) => (
            <span key={`${r.blockId}-${i}`} className="mention-chip" title={r.text}>
              选区@{r.relPath}#{r.blockId.slice(0, 6)}
              <button
                className="mention-chip-x"
                onClick={() => setBlockRefs((c) => c.filter((_, j) => j !== i))}
                aria-label="移除引用"
              >
                ×
              </button>
            </span>
          ))}
        </div>
      )}

      {pickerOpen && (
        <div className="mention-picker">
          {fileDocs.length === 0 && <p className="muted">工作区暂无文档</p>}
          {fileDocs.map((d) => (
            <div key={d.relPath} className="mention-doc">
              <div className="mention-doc-row">
                <button
                  className={`mention-option${mentions.includes(d.relPath) ? " selected" : ""}`}
                  onClick={() => toggleMention(d.relPath)}
                >
                  {mentions.includes(d.relPath) ? "✓ " : ""}
                  {d.relPath}
                </button>
                <button
                  className="ghost mention-blocks-toggle"
                  onClick={() => void expandDocBlocks(d.relPath)}
                  title="引用其中某一块"
                >
                  {expandedDoc === d.relPath ? "块▾" : "块▸"}
                </button>
              </div>
              {expandedDoc === d.relPath && (
                <div className="mention-blocks">
                  {(blocksByDoc[d.relPath] ?? []).length === 0 && <p className="muted">（无块或读取失败）</p>}
                  {(blocksByDoc[d.relPath] ?? []).map((b, i) => (
                    <button
                      key={i}
                      className="mention-block-option"
                      onClick={() => addDocBlockRef(d.relPath, b)}
                      title={b.text}
                    >
                      Block {i}: {b.text.slice(0, 40)}
                    </button>
                  ))}
                </div>
              )}
            </div>
          ))}
        </div>
      )}

      <textarea
        className="composer-input"
        placeholder="描述你想要的修改…"
        value={text}
        onChange={(e) => setText(e.target.value)}
        onKeyDown={(e) => {
          if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
            e.preventDefault();
            send();
          }
        }}
        rows={3}
      />
      <div className="composer-actions">
        <button
          className="ghost composer-mention-toggle"
          onClick={() => setPickerOpen((v) => !v)}
          title="引用工作区文档（随消息高优先提供给 AI）"
        >
          @引用{mentions.length ? `（${mentions.length}）` : ""}
        </button>
        <button
          className="ghost"
          onClick={addSelectionRef}
          disabled={!openPath}
          title={openPath ? "引用编辑器中选中的内容（精确到块）" : "先打开一个文档并选中内容"}
        >
          @引用选区
        </button>
        {/* 示例修改入口：绕过模型，对当前文档生成真实可落盘 diff（PI-FE-11） */}
        <button
          className="ghost composer-sample"
          onClick={() => void generateSampleDiff()}
          disabled={!openPath}
          title={openPath ? "对当前文档生成可落盘的示例修改" : "先打开一个文档"}
        >
          示例修改
        </button>
        <button onClick={send} disabled={sending || !text.trim()}>
          {sending ? "生成中…" : "发送"}
        </button>
      </div>
    </div>
  );
}
