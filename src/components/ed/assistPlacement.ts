export type AssistOverlayKind = "inline" | "autocomplete";
export type AssistPlacementSide = "top" | "bottom";
export type AssistPlacementAlign = "start" | "center" | "end";
export type AssistPlacementName = `${AssistPlacementSide}-${AssistPlacementAlign}`;

export interface AssistRect {
  left: number;
  top: number;
  right: number;
  bottom: number;
  width: number;
  height: number;
}

export interface AssistFloatingSize {
  width: number;
  height: number;
  minWidth: number;
  minHeight: number;
}

export interface ComputedAssistPlacement {
  x: number;
  y: number;
  placement: AssistPlacementName;
  width: number;
  height: number;
}

export const ASSIST_FLOATING_SIZE: Record<AssistOverlayKind, AssistFloatingSize> = {
  inline: { width: 360, height: 112, minWidth: 300, minHeight: 96 },
  autocomplete: { width: 420, height: 156, minWidth: 320, minHeight: 120 },
};

/**
 * @GOV
 * codes:    BR-AG-STATE-003-RB-AG-AG-ASSIST-004
 * type:     RB
 * chain:    AG-ASSIST
 * rules:    BR-AG-STATE-003
 * term_ref: TERM-AG-003, TERM-AG-004
 * boundary: in=viewportDomRectangle | out=AssistVisualRectangle(finiteViewportCoordinates) | consumer=BR-AG-STATE-003
 */
export function toAssistRect(rect: Pick<DOMRectReadOnly, "left" | "top" | "right" | "bottom">): AssistRect {
  const left = finite(rect.left);
  const top = finite(rect.top);
  const right = finite(rect.right);
  const bottom = finite(rect.bottom);
  return {
    left,
    top,
    right,
    bottom,
    width: Math.max(0, right - left),
    height: Math.max(0, bottom - top),
  };
}

/**
 * @GOV
 * codes:    BR-AG-STATE-003-RB-AG-AG-ASSIST-005
 * type:     RB
 * chain:    AG-ASSIST
 * rules:    BR-AG-STATE-003
 * term_ref: TERM-AG-003, TERM-AG-004
 * boundary: in=assistSelectionEndpointRectangles | out=AssistSelectionVisualRectangle(containingSelectionEndpoints) | consumer=BR-AG-STATE-003
 */
export function unionAssistRects(a: AssistRect, b: AssistRect): AssistRect {
  return toAssistRect({
    left: Math.min(a.left, b.left),
    top: Math.min(a.top, b.top),
    right: Math.max(a.right, b.right),
    bottom: Math.max(a.bottom, b.bottom),
  });
}

/**
 * @GOV
 * codes:    BR-AG-STATE-003-RB-AG-AG-ASSIST-006
 * type:     RB
 * chain:    AG-ASSIST
 * rules:    BR-AG-STATE-003
 * term_ref: TERM-AG-003, TERM-AG-004
 * boundary: in=editorScrollViewportAndPageRectangle | out=AssistVisibleContentRectangle(intersectionOrAnchorLost) | consumer=BR-AG-STATE-003
 */
export function intersectAssistRects(a: AssistRect, b: AssistRect): AssistRect | null {
  const left = Math.max(a.left, b.left);
  const top = Math.max(a.top, b.top);
  const right = Math.min(a.right, b.right);
  const bottom = Math.min(a.bottom, b.bottom);
  if (right <= left || bottom <= top) return null;
  return toAssistRect({ left, top, right, bottom });
}

/**
 * @GOV
 * codes:    BR-AG-STATE-003-RB-AG-AG-ASSIST-003
 * type:     RB
 * chain:    AG-ASSIST
 * rules:    BR-AG-STATE-003
 * term_ref: TERM-AG-003, TERM-AG-004
 * boundary: in=assistVisualAnchorAndEditorViewport | out=AssistOverlayPlacement(targetAdjacentFlipAndBoundedShift) | consumer=BR-AG-STATE-003
 */
/// AG-ASSIST 视觉放置守则（BR-AG-STATE-003）：逻辑写入锚由 AssistTarget 持有，视觉浮层消费
/// 缩放后的光标/选区矩形与未缩放的编辑宿主可视区；优先贴近目标，上下空间不足时翻转，横向仅做最小边界偏移。
export function computeAssistPlacement(input: {
  kind: AssistOverlayKind;
  anchorRect: AssistRect;
  boundaryRect: AssistRect;
  hostRect: AssistRect;
  floatingSize?: AssistFloatingSize;
  gap?: number;
  padding?: number;
}): ComputedAssistPlacement {
  const gap = input.gap ?? (input.kind === "inline" ? 8 : 6);
  const padding = input.padding ?? 12;
  const size = input.floatingSize ?? ASSIST_FLOATING_SIZE[input.kind];
  const width = fitFloatingSize(size.width, size.minWidth, input.boundaryRect.width - padding * 2);
  const height = fitFloatingSize(size.height, size.minHeight, input.boundaryRect.height - padding * 2);
  const side = chooseSide(input.kind, input.anchorRect, input.boundaryRect, height, gap, padding);
  const align = chooseAlign(input.kind, input.anchorRect, input.boundaryRect, width, padding);
  const xAbs = clamp(alignX(input.anchorRect, width, align), input.boundaryRect.left + padding, input.boundaryRect.right - width - padding);
  const yRaw = side === "bottom" ? input.anchorRect.bottom + gap : input.anchorRect.top - height - gap;
  const yAbs = clamp(yRaw, input.boundaryRect.top + padding, input.boundaryRect.bottom - height - padding);
  return {
    x: Math.round(xAbs - input.hostRect.left),
    y: Math.round(yAbs - input.hostRect.top),
    placement: `${side}-${align}`,
    width: Math.round(width),
    height: Math.round(height),
  };
}

function fitFloatingSize(preferred: number, minimum: number, available: number): number {
  const safeAvailable = Math.max(0, available);
  if (safeAvailable >= preferred) return preferred;
  if (safeAvailable >= minimum) return safeAvailable;
  return safeAvailable;
}

function chooseSide(
  kind: AssistOverlayKind,
  anchorRect: AssistRect,
  boundaryRect: AssistRect,
  floatingHeight: number,
  gap: number,
  padding: number,
): AssistPlacementSide {
  const above = anchorRect.top - boundaryRect.top - gap - padding;
  const below = boundaryRect.bottom - anchorRect.bottom - gap - padding;
  if (kind === "autocomplete") {
    if (below >= floatingHeight || below >= above) return "bottom";
    return "top";
  }
  if (above >= floatingHeight || above >= below) return "top";
  return "bottom";
}

function chooseAlign(
  kind: AssistOverlayKind,
  anchorRect: AssistRect,
  boundaryRect: AssistRect,
  floatingWidth: number,
  padding: number,
): AssistPlacementAlign {
  if (kind === "inline") {
    const centered = alignX(anchorRect, floatingWidth, "center");
    if (centered < boundaryRect.left + padding) return "start";
    if (centered > boundaryRect.right - floatingWidth - padding) return "end";
    return "center";
  }
  const start = alignX(anchorRect, floatingWidth, "start");
  const rightSpace = boundaryRect.right - anchorRect.left - padding;
  if (rightSpace < floatingWidth * 1.15) return "end";
  if (start <= boundaryRect.right - floatingWidth - padding) return "start";
  const end = alignX(anchorRect, floatingWidth, "end");
  if (end >= boundaryRect.left + padding) return "end";
  return "start";
}

function alignX(anchorRect: AssistRect, floatingWidth: number, align: AssistPlacementAlign): number {
  if (align === "center") return anchorRect.left + anchorRect.width / 2 - floatingWidth / 2;
  if (align === "end") return anchorRect.right - floatingWidth;
  return anchorRect.left;
}

function clamp(value: number, min: number, max: number): number {
  const lo = Math.min(min, max);
  const hi = Math.max(min, max);
  return Math.min(Math.max(value, lo), hi);
}

function finite(value: number): number {
  return Number.isFinite(value) ? value : 0;
}
