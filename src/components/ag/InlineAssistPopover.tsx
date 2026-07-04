import { useState } from "react";
import { useAssistStore } from "../../store/assistStore";

/**
 * ⌘K InlineAssist 弹窗（TERM-AG-003 · 参考稿 §6.6）· PI-FE-08。
 * 选区上方对话弹窗：输入指令 → 生成 → 预览 → Apply 直接替换选区。
 * **不产出 DiffCard、不进入 Diff 链路**——与 Chat Diff 严格区分。
 */
interface Props {
  onGenerate: (instruction: string) => void;
  onApply: (text: string) => void;
}

export function InlineAssistPopover({ onGenerate, onApply }: Props) {
  const { kind, anchor, selectionText, inlinePreview, generating } = useAssistStore();
  const [instruction, setInstruction] = useState("");
  if (kind !== "inline" || !anchor) return null;

  return (
    <div className="assist-pop" style={{ left: anchor.x, top: anchor.y }} role="dialog">
      <div className="assist-pop-head">
        <span className="label">⌘K 局部修改</span>
        <span className="assist-pop-sel">“{selectionText.slice(0, 24)}{selectionText.length > 24 ? "…" : ""}”</span>
      </div>
      <input
        className="assist-pop-input"
        autoFocus
        placeholder="如何修改选中内容…"
        value={instruction}
        onChange={(e) => setInstruction(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && instruction.trim()) {
            e.preventDefault();
            onGenerate(instruction.trim());
          }
        }}
      />
      {generating && <div className="assist-pop-hint">生成中…</div>}
      {inlinePreview && (
        <>
          <div className="assist-pop-preview">{inlinePreview}</div>
          <div className="assist-pop-actions">
            <button onClick={() => onApply(inlinePreview)}>Apply（替换选区）</button>
            <span className="label">不进 Diff 链路</span>
          </div>
        </>
      )}
    </div>
  );
}
