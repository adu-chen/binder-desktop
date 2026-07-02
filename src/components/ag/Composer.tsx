import { useState, type ChangeEvent } from "react";
import { useAgStore } from "../../store/agStore";
import { startTurn, listDocBlocks, uploadByok, type DocBlock } from "../../services/agService";
import { generateSampleDiff } from "../../services/deService";
import { useEdStore } from "../../store/edStore";
import { useWsStore } from "../../store/wsStore";
import { captureSelectionRef, SELECTION_REF_MIME, type SelectionRef } from "../../services/edLocate";

interface BlockRef {
  relPath: string;
  blockId: string;
  text: string;
  /** 块内纯文本偏移区间（CharAnchor，PI-ED-10 位置引用）；可空=块级锚（后端 serde 忽略未知/缺省兼容）。 */
  start?: number;
  end?: number;
}

/** 检测光标前的活跃 @ token（@ 前为行首/空白、@ 后到光标无空白）→ { query, start }；否则 null。 */
function activeMentionQuery(text: string, caret: number): { query: string; start: number } | null {
  const upto = text.slice(0, caret);
  const at = upto.lastIndexOf("@");
  if (at < 0) return null;
  const before = at === 0 ? "" : upto[at - 1];
  if (before && !/\s/.test(before)) return null;
  const query = upto.slice(at + 1);
  if (/\s/.test(query)) return null;
  return { query, start: at };
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
  const [atQuery, setAtQuery] = useState<{ query: string; start: number; caret: number } | null>(null);
  const [byokOpen, setByokOpen] = useState(false);
  const [byokProvider, setByokProvider] = useState("deepseek");
  const [byokKey, setByokKey] = useState("");
  const [byokStatus, setByokStatus] = useState("");

  const fileDocs = docs.filter((d) => !d.isDir);

  const saveByok = async () => {
    if (!byokKey.trim()) return;
    setByokStatus("上送中…");
    try {
      await uploadByok(byokProvider, byokKey);
      setByokKey(""); // 客户端不保留密钥
      setByokStatus(`✓ ${byokProvider} 密钥已送网关`);
    } catch (e) {
      const err = e as { message?: string };
      setByokStatus(`✗ ${err?.message ?? "上送失败"}`);
    }
  };
  const atMatches = atQuery
    ? fileDocs.filter((d) => d.relPath.toLowerCase().includes(atQuery.query.toLowerCase())).slice(0, 8)
    : [];

  const onTextChange = (e: ChangeEvent<HTMLTextAreaElement>) => {
    const val = e.target.value;
    const caret = e.target.selectionStart ?? val.length;
    setText(val);
    const m = activeMentionQuery(val, caret);
    setAtQuery(m ? { ...m, caret } : null);
  };

  const selectAtMention = (relPath: string) => {
    if (!atQuery) return;
    setMentions((cur) => (cur.includes(relPath) ? cur : [...cur, relPath]));
    setText((t) => t.slice(0, atQuery.start) + t.slice(atQuery.caret));
    setAtQuery(null);
  };

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
    pushSelectionRef({ ...ref, path: openPath });
  };

  // 选区引用去重入列（按钮捕获与拖拽 drop 共用）：含字符锚区间（有偏移时），同块同区间不重复。
  const pushSelectionRef = (ref: SelectionRef) => {
    if (!ref.blockId || !ref.path) return;
    const withRange = ref.end > ref.start; // start=end=0 为块级回落锚
    setBlockRefs((cur) =>
      cur.some(
        (r) => r.blockId === ref.blockId && r.relPath === ref.path && r.start === (withRange ? ref.start : undefined),
      )
        ? cur
        : [
            ...cur,
            {
              relPath: ref.path,
              blockId: ref.blockId,
              text: ref.text,
              ...(withRange ? { start: ref.start, end: ref.end } : {}),
            },
          ],
    );
  };

  // 选区拖拽入 Composer → 位置引用 chip（PI-ED-10 #6：SelectionRef 经自定义 MIME 携带字符锚）。
  const onDragOver = (e: React.DragEvent) => {
    if (e.dataTransfer.types.includes(SELECTION_REF_MIME)) e.preventDefault();
  };
  const onDrop = (e: React.DragEvent) => {
    const raw = e.dataTransfer.getData(SELECTION_REF_MIME);
    if (!raw) return;
    e.preventDefault();
    try {
      pushSelectionRef(JSON.parse(raw) as SelectionRef);
    } catch {
      /* 载荷损坏 → 忽略本次拖放 */
    }
  };

  const toggleMention = (relPath: string) => {
    setMentions((cur) => (cur.includes(relPath) ? cur.filter((p) => p !== relPath) : [...cur, relPath]));
  };

  return (
    <div className="composer" onDragOver={onDragOver} onDrop={onDrop}>
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
              {r.start !== undefined && r.end !== undefined ? `[${r.start}–${r.end}]` : ""}
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

      {byokOpen && (
        <div className="byok-form">
          <select className="byok-provider" value={byokProvider} onChange={(e) => setByokProvider(e.target.value)}>
            <option value="deepseek">deepseek</option>
            <option value="openai">openai</option>
            <option value="anthropic">anthropic</option>
          </select>
          <input
            className="byok-input"
            type="password"
            placeholder="粘贴 provider 密钥（仅送网关，不在本地保存）"
            value={byokKey}
            onChange={(e) => setByokKey(e.target.value)}
          />
          <button onClick={() => void saveByok()} disabled={!byokKey.trim()}>
            保存
          </button>
          {byokStatus && <span className="byok-status">{byokStatus}</span>}
        </div>
      )}

      {atQuery && atMatches.length > 0 && (
        <div className="mention-inline">
          {atMatches.map((d) => (
            <button key={d.relPath} className="mention-block-option" onClick={() => selectAtMention(d.relPath)}>
              @{d.relPath}
            </button>
          ))}
        </div>
      )}

      <textarea
        className="composer-input"
        placeholder="描述你想要的修改…（输入 @ 引用文档）"
        value={text}
        onChange={onTextChange}
        onKeyDown={(e) => {
          if (atQuery && e.key === "Escape") {
            setAtQuery(null);
            return;
          }
          if (atQuery && atMatches.length && e.key === "Enter" && !e.metaKey && !e.ctrlKey && !e.shiftKey) {
            e.preventDefault();
            selectAtMention(atMatches[0].relPath);
            return;
          }
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
        <button
          className="ghost"
          onClick={() => setByokOpen((v) => !v)}
          title="填自己的 provider 密钥（BYOK，仅送网关代持，不在本地保存）"
        >
          🔑 key
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
