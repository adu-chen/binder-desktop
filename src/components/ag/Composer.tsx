import { useState, type ChangeEvent, type KeyboardEvent } from "react";
import { activeSession, agStore, useAgStore, type AgMode } from "../../store/agStore";
import { modelKey, modelStore, useModelStore } from "../../store/modelStore";
import { startTurn } from "../../services/agService";
import { useWsStore } from "../../store/wsStore";
import { SELECTION_REF_MIME, type SelectionRef } from "../../services/edLocate";
import { ModelConfig } from "./ModelConfig";

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

const MODES: { id: AgMode; label: string; hint: string }[] = [
  { id: "edit", label: "edit", hint: "编辑 · 精确修改当前内容（diff 裁决）" },
  { id: "build", label: "build", hint: "构建 · 项目蓝图与整体构建" },
];

/**
 * Composer（A-SYS-C-T-02 §3 输入区 · PI-FE-06 / PI-AG-UI-01 重构）。
 * 框内底部左：模式按钮（edit/build，取代原顶部切换，同源 session.mode）+ 模型选择（含「添加模型」
 * 进配置界面）；右：发送按钮三态（空内容禁用 / 待发送 / AI 回复中）。
 * 发送：Enter 直接发送、Shift+Enter 换行；IME（中文输入法）合成中的回车仅确认候选不发送（需求5）。
 * 引用：输入 @ 触发文档下拉（原「引用」按钮移除）；编辑器选区拖拽入框成为位置引用（原按钮移除）。
 * BYOK 入口移入模型配置界面（原 🔑 按钮移除）；「示例修改」开发入口移除。
 */
export function Composer() {
  const agState = useAgStore();
  const { sending } = agState;
  const mode = activeSession(agState).mode;
  const { models, selectedKey } = useModelStore();
  const currentModel = modelStore.current();
  const { docs } = useWsStore();

  const [text, setText] = useState("");
  const [mentions, setMentions] = useState<string[]>([]);
  const [blockRefs, setBlockRefs] = useState<BlockRef[]>([]);
  const [atQuery, setAtQuery] = useState<{ query: string; start: number; caret: number } | null>(null);
  const [modePop, setModePop] = useState(false);
  const [modelPop, setModelPop] = useState(false);
  const [configOpen, setConfigOpen] = useState(false);

  const fileDocs = docs.filter((d) => !d.isDir);
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

  const send = () => {
    if (sending || !text.trim()) return;
    void startTurn(text, {
      ...(mentions.length ? { mentions } : {}),
      ...(blockRefs.length ? { blockRefs } : {}),
      provider: currentModel.provider,
      model: currentModel.model,
    });
    setText("");
    setMentions([]);
    setBlockRefs([]);
    setAtQuery(null);
  };

  /**
   * 键盘协议（PI-AG-UI-01 需求5）：
   * - IME 合成中（中文输入法候选未确认）：Enter 仅确认输入法内容，不发送（isComposing / keyCode 229）。
   * - @ 下拉活跃：Enter 选中第一项。
   * - Shift+Enter：换行（textarea 默认行为）。
   * - Enter：发送。
   */
  const onKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Escape" && atQuery) {
      setAtQuery(null);
      return;
    }
    if (e.key !== "Enter") return;
    if (e.nativeEvent.isComposing || e.keyCode === 229) return; // IME 合成确认，不发送
    if (atQuery && atMatches.length && !e.shiftKey) {
      e.preventDefault();
      selectAtMention(atMatches[0].relPath);
      return;
    }
    if (e.shiftKey) return; // Shift+Enter 换行
    e.preventDefault();
    send();
  };

  // 选区引用去重入列（拖拽 drop）：含字符锚区间（有偏移时），同块同区间不重复。
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

  const removeMention = (relPath: string) => setMentions((cur) => cur.filter((p) => p !== relPath));

  const sendCls = sending ? "is-busy" : text.trim() ? "is-ready" : "is-disabled";

  return (
    <div className="composer" onDragOver={onDragOver} onDrop={onDrop}>
      {mentions.length > 0 && (
        <div className="composer-mentions">
          {mentions.map((p) => (
            <span key={p} className="mention-chip">
              @{p}
              <button className="mention-chip-x" onClick={() => removeMention(p)} aria-label="移除引用">
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

      {atQuery && atMatches.length > 0 && (
        <div className="mention-inline">
          {atMatches.map((d) => (
            <button key={d.relPath} className="mention-block-option" onClick={() => selectAtMention(d.relPath)}>
              @{d.relPath}
            </button>
          ))}
        </div>
      )}

      <div className="composer-box">
        <textarea
          className="composer-input"
          placeholder="描述你想要的修改…（@ 引用文档 · Enter 发送 · Shift+Enter 换行）"
          value={text}
          onChange={onTextChange}
          onKeyDown={onKeyDown}
          rows={3}
        />
        <div className="composer-bar">
          <div className="composer-bar-left">
            <div className="cbar-wrap">
              <button
                className="cbar-btn"
                title="模式：edit 编辑 / build 构建"
                aria-haspopup="menu"
                onClick={() => {
                  setModePop((v) => !v);
                  setModelPop(false);
                }}
              >
                {mode} <span className="cbar-caret">▾</span>
              </button>
              {modePop && (
                <div className="cbar-pop" role="menu">
                  {MODES.map((m) => (
                    <button
                      key={m.id}
                      className={`cbar-pop-item${mode === m.id ? " selected" : ""}`}
                      onClick={() => {
                        agStore.setMode(m.id);
                        setModePop(false);
                      }}
                    >
                      {mode === m.id ? "✓ " : ""}
                      {m.label}
                      <span className="label cbar-pop-hint">{m.hint}</span>
                    </button>
                  ))}
                </div>
              )}
            </div>

            <div className="cbar-wrap">
              <button
                className="cbar-btn"
                title="模型选择"
                aria-haspopup="menu"
                onClick={() => {
                  setModelPop((v) => !v);
                  setModePop(false);
                }}
              >
                {currentModel.label} <span className="cbar-caret">▾</span>
              </button>
              {modelPop && (
                <div className="cbar-pop" role="menu">
                  {models.map((m) => {
                    const key = modelKey(m);
                    return (
                      <button
                        key={key}
                        className={`cbar-pop-item${selectedKey === key ? " selected" : ""}`}
                        onClick={() => {
                          modelStore.select(key);
                          setModelPop(false);
                        }}
                      >
                        {selectedKey === key ? "✓ " : ""}
                        {m.label}
                        <span className="label cbar-pop-hint">{key}</span>
                      </button>
                    );
                  })}
                  <button
                    className="cbar-pop-item cbar-pop-add"
                    onClick={() => {
                      setModelPop(false);
                      setConfigOpen(true);
                    }}
                  >
                    ＋ 添加模型
                  </button>
                </div>
              )}
            </div>
          </div>

          <button
            className={`composer-send ${sendCls}`}
            onClick={send}
            disabled={sending || !text.trim()}
            title={sending ? "AI 回复中…" : text.trim() ? "发送（Enter）" : "输入内容后可发送"}
          >
            {sending ? "生成中" : "发送"}
          </button>
        </div>
      </div>

      {configOpen && <ModelConfig onClose={() => setConfigOpen(false)} />}
    </div>
  );
}
