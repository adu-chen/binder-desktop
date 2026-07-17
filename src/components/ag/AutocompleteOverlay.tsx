import { useEffect } from "react";
import { useAssistStore } from "../../store/assistStore";

/**
 * ⌘J Autocomplete 浮层（TERM-AG-004 · AG-ASSIST / BR-AG-STATE-003）。
 * 光标处低权重浮层，3 条续写建议，Tab/↩ 选第一条、数字键选对应条。
 * **不产出 DiffCard、不进入 Diff 链路**。
 */
interface Props {
  onPick: (text: string) => void;
  onDismiss: () => void;
  isKeyboardScope?: (event: KeyboardEvent) => boolean;
}

/**
 * @GOV
 * codes:    BR-AG-STATE-003-RB-AG-AG-ASSIST-009
 * type:     RB
 * chain:    AG-ASSIST
 * rules:    BR-AG-STATE-003
 * term_ref: TERM-AG-004
 * boundary: in=AutocompleteAssistViewState
 *           | out=AutocompleteOverlay(threeCandidatesOrEmptyStateWithDiagnosticReason)
 *           | does_not=expose_internal_failure_as_user_error_or_drop_diagnostic_reason
 */
export function AutocompleteOverlay({ onPick, onDismiss, isKeyboardScope }: Props) {
  const { kind, anchor, suggestions, generating, autocompleteFailureReason } = useAssistStore();

  useEffect(() => {
    if (kind !== "autocomplete") return;
    const handler = (e: KeyboardEvent) => {
      if (e.isComposing || (isKeyboardScope && !isKeyboardScope(e))) return;
      if (e.key === "Tab") {
        e.preventDefault();
        if (suggestions[0]) onPick(suggestions[0]);
      } else if (/^[1-3]$/.test(e.key)) {
        const idx = Number(e.key) - 1;
        if (suggestions[idx]) {
          e.preventDefault();
          onPick(suggestions[idx]);
        }
      } else if (e.key === "Escape") {
        e.preventDefault();
        onDismiss();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [kind, suggestions, onPick, onDismiss, isKeyboardScope]);

  if (kind !== "autocomplete" || !anchor) return null;

  return (
    <div
      className="assist-ac"
      data-placement={anchor.placement}
      data-failure-reason={autocompleteFailureReason ?? undefined}
      style={{ left: anchor.x, top: anchor.y, width: anchor.width }}
    >
      <div className="label assist-ac-head">⌘J 续写 · Tab/1/2/3 采纳</div>
      {generating && suggestions.length === 0 && <div className="assist-ac-item">生成中…</div>}
      {!generating && suggestions.length === 0 && <div className="assist-ac-item assist-ac-empty">暂无可用续写</div>}
      {suggestions.map((s, i) => (
        <button key={i} className="assist-ac-item" onClick={() => onPick(s)}>
          <span className="assist-ac-num">{i + 1}</span>
          <span className="assist-ac-text">{s}</span>
        </button>
      ))}
    </div>
  );
}
