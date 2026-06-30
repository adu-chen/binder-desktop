import { useEffect } from "react";
import { useAssistStore } from "../../store/assistStore";

/**
 * ⌘J Autocomplete 浮层（TERM-AG-004 · 参考稿 §6.6）· PI-FE-08。
 * 光标处低权重浮层，3 条续写建议，Tab/↩ 选第一条、数字键选对应条。
 * **不产出 DiffCard、不进入 Diff 链路**。
 */
interface Props {
  onPick: (text: string) => void;
}

export function AutocompleteOverlay({ onPick }: Props) {
  const { kind, anchor, suggestions, generating } = useAssistStore();

  useEffect(() => {
    if (kind !== "autocomplete") return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Tab" || e.key === "Enter") {
        e.preventDefault();
        if (suggestions[0]) onPick(suggestions[0]);
      } else if (/^[1-3]$/.test(e.key)) {
        const idx = Number(e.key) - 1;
        if (suggestions[idx]) {
          e.preventDefault();
          onPick(suggestions[idx]);
        }
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [kind, suggestions, onPick]);

  if (kind !== "autocomplete" || !anchor) return null;

  return (
    <div className="assist-ac" style={{ left: anchor.x, top: anchor.y }}>
      <div className="label assist-ac-head">⌘J 续写 · Tab 选第一条</div>
      {generating && suggestions.length === 0 && <div className="assist-ac-item">生成中…</div>}
      {suggestions.map((s, i) => (
        <button key={i} className="assist-ac-item" onClick={() => onPick(s)}>
          <span className="assist-ac-num">{i + 1}</span>
          <span className="assist-ac-text">{s}</span>
        </button>
      ))}
    </div>
  );
}
