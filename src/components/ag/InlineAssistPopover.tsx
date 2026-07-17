import { useEffect, useState } from "react";
import { useAssistStore } from "../../store/assistStore";

/**
 * ⌘K InlineAssist 弹窗（TERM-AG-003 · AG-ASSIST / BR-AG-STATE-003）。
 * 选区上方对话弹窗：输入指令 → 生成 → 预览 → Apply 直接替换选区。
 * **不产出 DiffCard、不进入 Diff 链路**——与 Chat Diff 严格区分。
 */
interface Props {
  onGenerate: (instruction: string) => void;
  onApply: (text: string) => void;
  onDismiss: () => void;
}

export function InlineAssistPopover({ onGenerate, onApply, onDismiss }: Props) {
  const { kind, anchor, selectionText, inlinePreview, generating, target } = useAssistStore();
  const [instruction, setInstruction] = useState("");
  useEffect(() => {
    if (kind !== "inline") setInstruction("");
  }, [kind]);
  if (kind !== "inline" || !anchor) return null;
  const cursorMode = target?.mode === "cursor";

  return (
    <div className="assist-pop" data-placement={anchor.placement} style={{ left: anchor.x, top: anchor.y, width: anchor.width }} role="dialog">
      <div className="assist-pop-head">
        <span className="label">⌘K 局部修改</span>
        <span className="assist-pop-sel">
          {cursorMode ? "光标处插入" : `“${selectionText.slice(0, 24)}${selectionText.length > 24 ? "…" : ""}”`}
        </span>
      </div>
      <input
        className="assist-pop-input"
        autoFocus
        placeholder={cursorMode ? "要在光标处插入什么…" : "如何修改选中内容…"}
        value={instruction}
        onChange={(e) => setInstruction(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && instruction.trim()) {
            e.preventDefault();
            onGenerate(instruction.trim());
          } else if (e.key === "Escape") {
            e.preventDefault();
            onDismiss();
          }
        }}
      />
      {generating && <div className="assist-pop-hint">生成中…</div>}
      {inlinePreview && (
        <>
          <div className="assist-pop-preview">{inlinePreview}</div>
          <div className="assist-pop-actions">
            <button onClick={() => onApply(inlinePreview)}>{cursorMode ? "Apply（插入）" : "Apply（替换选区）"}</button>
            <span className="label">不进 Diff 链路</span>
          </div>
        </>
      )}
    </div>
  );
}
