/**
 * 分页结构变化的视口锚定纯逻辑。
 *
 * 浏览器 rect 坐标是屏幕 CSS px；scrollTop 是滚动容器坐标。通过运行时测得的
 * viewportPixelsPerScrollPixel 换算两者，避免把内容 CSS zoom 当作滚动坐标比例。
 */
export interface ViewportAnchorMeasurement {
  beforeTop: number;
  afterTop: number;
  viewportPixelsPerScrollPixel: number;
}

export interface ViewportRect {
  left: number;
  right: number;
  top: number;
  bottom: number;
}

export interface ViewportPoint {
  x: number;
  y: number;
}

export interface ClientViewportMeasurement {
  borderRect: ViewportRect & { width: number; height: number };
  offsetWidth: number;
  offsetHeight: number;
  clientLeft: number;
  clientTop: number;
  clientWidth: number;
  clientHeight: number;
}

function finitePositiveOrOne(value: number): number {
  return Number.isFinite(value) && value > 0 ? value : 1;
}

/**
 * 把滚动容器的 border rect 收窄为实际 client viewport。输出仍是屏幕 CSS px，
 * 可直接与 Selection Range 的 rect 比较；内容 CSS zoom 不参与换算。
 */
export function computeClientViewportRect(measurement: ClientViewportMeasurement): ViewportRect | null {
  const { borderRect } = measurement;
  if (
    !Number.isFinite(borderRect.left)
    || !Number.isFinite(borderRect.top)
    || !Number.isFinite(borderRect.width)
    || !Number.isFinite(borderRect.height)
    || !Number.isFinite(measurement.clientWidth)
    || !Number.isFinite(measurement.clientHeight)
  ) {
    return null;
  }
  const scaleX = finitePositiveOrOne(borderRect.width / measurement.offsetWidth);
  const scaleY = finitePositiveOrOne(borderRect.height / measurement.offsetHeight);
  const left = borderRect.left + measurement.clientLeft * scaleX;
  const top = borderRect.top + measurement.clientTop * scaleY;
  return {
    left,
    right: left + Math.max(0, measurement.clientWidth) * scaleX,
    top,
    bottom: top + Math.max(0, measurement.clientHeight) * scaleY,
  };
}

function isFiniteViewportRect(rect: ViewportRect): boolean {
  return Number.isFinite(rect.left)
    && Number.isFinite(rect.right)
    && Number.isFinite(rect.top)
    && Number.isFinite(rect.bottom);
}

/**
 * 光标已经完整可见时，允许至多 1 屏幕像素的亚像素测量误差。超过容差的任何方向越界均须
 * 交还 ProseMirror 原生 scrollIntoView，以保留真实越界后的自动跟随。
 */
export function isRenderedRectInsideViewport(
  renderedRect: ViewportRect,
  viewportRect: ViewportRect,
  tolerance = 1,
): boolean {
  if (!isFiniteViewportRect(renderedRect) || !isFiniteViewportRect(viewportRect)) return false;
  const safeTolerance = Number.isFinite(tolerance) ? Math.max(0, tolerance) : 0;
  const left = Math.min(renderedRect.left, renderedRect.right);
  const right = Math.max(renderedRect.left, renderedRect.right);
  const top = Math.min(renderedRect.top, renderedRect.bottom);
  const bottom = Math.max(renderedRect.top, renderedRect.bottom);
  return left >= viewportRect.left - safeTolerance
    && right <= viewportRect.right + safeTolerance
    && top >= viewportRect.top - safeTolerance
    && bottom <= viewportRect.bottom + safeTolerance;
}

/** 最终选区同步后，只有单击形成的分页折叠文本光标可以建立输入滚动保护周期。 */
export function shouldArmPointerActivatedInputScroll(input: {
  button: number;
  clickCount: number;
  paginated: boolean;
  loading: boolean;
  collapsedTextSelection: boolean;
  activationPoint: ViewportPoint;
  viewportRect: ViewportRect | null;
}): boolean {
  const viewport = input.viewportRect;
  return input.button === 0
    && input.clickCount <= 1
    && input.paginated
    && !input.loading
    && input.collapsedTextSelection
    && viewport !== null
    && Number.isFinite(input.activationPoint.x)
    && Number.isFinite(input.activationPoint.y)
    && input.activationPoint.x >= viewport.left
    && input.activationPoint.x <= viewport.right
    && input.activationPoint.y >= viewport.top
    && input.activationPoint.y <= viewport.bottom;
}

export function shouldSuppressPaginatedSelectionScroll(input: {
  paginated: boolean;
  loading: boolean;
  composing: boolean;
  collapsedTextSelection: boolean;
  renderedRect: ViewportRect | null;
  viewportRect: ViewportRect | null;
}): boolean {
  return input.paginated
    && !input.loading
    && !input.composing
    && input.collapsedTextSelection
    && input.renderedRect !== null
    && input.viewportRect !== null
    && isRenderedRectInsideViewport(input.renderedRect, input.viewportRect);
}

/**
 * 指针是可信的屏幕坐标；Range 在 WKWebView/CSS zoom 下可能是未缩放坐标。以激活时 Range 为原点，
 * 只换算后续 Range 的相对位移，可避开绝对坐标空间偏差。
 */
export function estimatePointerActivatedCaretRect(input: {
  activationPoint: ViewportPoint;
  activationRenderedRect: ViewportRect;
  currentRenderedRect: ViewportRect;
  renderedDeltaScale: number;
}): ViewportRect | null {
  if (
    !Number.isFinite(input.activationPoint.x)
    || !Number.isFinite(input.activationPoint.y)
    || !isFiniteViewportRect(input.activationRenderedRect)
    || !isFiniteViewportRect(input.currentRenderedRect)
  ) {
    return null;
  }
  const scale = finitePositiveOrOne(input.renderedDeltaScale);
  const activationLeft = Math.min(input.activationRenderedRect.left, input.activationRenderedRect.right);
  const activationRight = Math.max(input.activationRenderedRect.left, input.activationRenderedRect.right);
  const activationTop = Math.min(input.activationRenderedRect.top, input.activationRenderedRect.bottom);
  const activationBottom = Math.max(input.activationRenderedRect.top, input.activationRenderedRect.bottom);
  const currentLeft = Math.min(input.currentRenderedRect.left, input.currentRenderedRect.right);
  const currentRight = Math.max(input.currentRenderedRect.left, input.currentRenderedRect.right);
  const currentTop = Math.min(input.currentRenderedRect.top, input.currentRenderedRect.bottom);
  const currentBottom = Math.max(input.currentRenderedRect.top, input.currentRenderedRect.bottom);
  const activationCenterX = (activationLeft + activationRight) / 2;
  const activationCenterY = (activationTop + activationBottom) / 2;
  const currentCenterX = (currentLeft + currentRight) / 2;
  const currentCenterY = (currentTop + currentBottom) / 2;
  const width = Math.max(1, (currentRight - currentLeft) * scale);
  const height = Math.max(1, (currentBottom - currentTop) * scale);
  const centerX = input.activationPoint.x + (currentCenterX - activationCenterX) * scale;
  const centerY = input.activationPoint.y + (currentCenterY - activationCenterY) * scale;
  return {
    left: centerX - width / 2,
    right: centerX + width / 2,
    top: centerY - height / 2,
    bottom: centerY + height / 2,
  };
}

/**
 * 指针激活周期内，只要每次文档变化后的估算光标仍可见且用户没有主动滚动，就持续吞掉冗余滚动。
 * 不按字符数计数；真正越界后自然释放给 ProseMirror。
 */
export function shouldSuppressPointerActivatedInputScroll(input: {
  documentChangedSinceLastDecision: boolean;
  scrollTopAtPointer: number;
  currentScrollTop: number;
  activationPoint: ViewportPoint;
  activationRenderedRect: ViewportRect;
  currentRenderedRect: ViewportRect;
  renderedDeltaScale: number;
  viewportRect: ViewportRect;
}): boolean {
  if (
    !input.documentChangedSinceLastDecision
    || !Number.isFinite(input.scrollTopAtPointer)
    || !Number.isFinite(input.currentScrollTop)
    || Math.abs(input.currentScrollTop - input.scrollTopAtPointer) > 1
  ) {
    return false;
  }
  const estimatedRect = estimatePointerActivatedCaretRect(input);
  return estimatedRect !== null && isRenderedRectInsideViewport(estimatedRect, input.viewportRect);
}

/** 只有非零且有限的比例可用于补偿；容器未缩放时应为 1。 */
export function normalizeViewportPixelsPerScrollPixel(value: number): number {
  return Number.isFinite(value) && value > 0 ? value : 1;
}

/**
 * 令锚点回到提交前的屏幕 Y 所需的 scrollTop 增量。
 * afterTop 更大表示分页缝把锚点向下推，因此 scrollTop 应正向增加。
 */
export function computeViewportAnchorScrollDelta(measurement: ViewportAnchorMeasurement): number {
  const scale = normalizeViewportPixelsPerScrollPixel(measurement.viewportPixelsPerScrollPixel);
  const delta = (measurement.afterTop - measurement.beforeTop) / scale;
  return Number.isFinite(delta) ? delta : 0;
}

/** 低于半个屏幕像素的差异不写 scrollTop，避免测量小数噪声造成新一轮视觉抖动。 */
export function shouldApplyViewportAnchorCorrection(delta: number): boolean {
  return Number.isFinite(delta) && Math.abs(delta) > 0.5;
}

/**
 * 把残差补偿写入滚动容器的合法范围。浏览器本身也会钳制 scrollTop，但纯逻辑先钳制可避免
 * 在顶部/底部反复写入不可达值并触发额外 scroll 事件。
 */
export function computeClampedViewportScrollTop(input: {
  currentScrollTop: number;
  delta: number;
  scrollHeight: number;
  clientHeight: number;
}): number {
  const current = Number.isFinite(input.currentScrollTop) ? input.currentScrollTop : 0;
  const delta = Number.isFinite(input.delta) ? input.delta : 0;
  const max = Math.max(0, input.scrollHeight - input.clientHeight);
  return Math.min(max, Math.max(0, current + delta));
}
