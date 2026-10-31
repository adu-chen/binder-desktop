export interface AssistAdoptionTarget {
  blockId: string;
  start: number;
  end: number;
}

export interface AssistAdoptionSnapshot {
  kind: "none" | "inline" | "autocomplete";
  requestId: string | null;
  target: AssistAdoptionTarget | null;
}

export interface AssistAdoptionDeps {
  editorReady: boolean;
  targetCurrent: boolean;
  snapshot: AssistAdoptionSnapshot;
  close: () => void;
  apply: (target: AssistAdoptionTarget, text: string) => { ok: boolean };
  finishRequest: (requestId: string, event: "accept" | "anchor_lost") => void;
  finishCurrent: (event: "anchor_lost") => void;
}

export type AssistAdoptionResult = "accepted" | "anchor_lost";

/**
 * AG-ASSIST 采纳的唯一落地路径：先校验编辑器/锚，再经 ED apply，最后回写终态。
 * 失败只回写 anchor_lost，不得把未应用的候选标成 accept。
 */
export function applyAssistAdoption(text: string, deps: AssistAdoptionDeps): AssistAdoptionResult {
  const { snapshot } = deps;
  if (!deps.editorReady || snapshot.kind !== "inline" || !snapshot.target || !deps.targetCurrent) {
    deps.finishCurrent("anchor_lost");
    return "anchor_lost";
  }
  const target = snapshot.target;
  const requestId = snapshot.requestId;
  deps.close();
  const applied = deps.apply(target, text);
  if (requestId) deps.finishRequest(requestId, applied.ok ? "accept" : "anchor_lost");
  return applied.ok ? "accepted" : "anchor_lost";
}
