/**
 * 编辑预览区缩放（ED 视图态，PI-FE-05）。纯呈现层缩放：只改编辑纸张的视觉比例，不改文档模型、
 * 不落盘、不产生脏态（同分页 BR-ED-DATA-003 呈现层纯化精神）。缩放经 CSS `zoom` 施加于编辑纸张，
 * 与分页测量按当前缩放归一化协作（Pagination.measureBlockHeights 自读 computed zoom）。
 * 纯钳制/步进/格式化逻辑与 DOM 解耦，可单测。
 */

/** 缩放约束：50%–200%，步进 10%，默认 100%。 */
export const ZOOM = { min: 0.5, max: 2, step: 0.1, default: 1 } as const;

/** 钳制到 [min, max] 并规整到 2 位小数（消 FP 漂移）；非有限 → 默认 100%。 */
export function clampZoom(z: number): number {
  if (!Number.isFinite(z)) return ZOOM.default;
  const r = Math.round(z * 100) / 100;
  return Math.max(ZOOM.min, Math.min(ZOOM.max, r));
}

/** 在当前缩放上步进 delta（放大/缩小），结果钳制。 */
export function stepZoom(z: number, delta: number): number {
  return clampZoom(z + delta);
}

/** 缩放比例显示文本（如 1.2 → "120%"）。 */
export function formatZoom(z: number): string {
  return `${Math.round(clampZoom(z) * 100)}%`;
}
