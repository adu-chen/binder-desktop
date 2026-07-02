import { useRef } from "react";

/**
 * 垂直分栏拖柄（视图层布局控件）。指针拖动上报**增量** dx（父级据语义加/减到当前栏宽并钳制——
 * 增量 + 读当前值 + 钳制可自然处理越界回拖，无死区）；双击复位默认宽；方向键 ±步进（无障碍）。
 * 不承载业务态（X-FORBID-007 无关）。
 */

const KEY_STEP = 16; // 方向键单步（px）

export function Splitter({
  onResize,
  onReset,
  ariaLabel,
  className,
}: {
  /** 拖动增量 dx（正=指针右移）；父级按该拖柄语义换算为栏宽变化。 */
  onResize: (dx: number) => void;
  /** 双击复位默认宽。 */
  onReset?: () => void;
  ariaLabel: string;
  /** 栅格定位变体类（如 gut-left/gut-right）。 */
  className?: string;
}) {
  const dragging = useRef(false);
  const lastX = useRef(0);

  const onPointerDown = (e: React.PointerEvent<HTMLDivElement>) => {
    e.preventDefault();
    dragging.current = true;
    lastX.current = e.clientX;
    e.currentTarget.setPointerCapture(e.pointerId);
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
  };
  const onPointerMove = (e: React.PointerEvent<HTMLDivElement>) => {
    if (!dragging.current) return;
    const dx = e.clientX - lastX.current;
    if (dx !== 0) {
      lastX.current = e.clientX;
      onResize(dx);
    }
  };
  const end = (e: React.PointerEvent<HTMLDivElement>) => {
    if (!dragging.current) return;
    dragging.current = false;
    try {
      e.currentTarget.releasePointerCapture(e.pointerId);
    } catch {
      /* 指针已释放 */
    }
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
  };
  const onKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
    if (e.key === "ArrowLeft") {
      e.preventDefault();
      onResize(-KEY_STEP);
    } else if (e.key === "ArrowRight") {
      e.preventDefault();
      onResize(KEY_STEP);
    }
  };

  return (
    <div
      className={`pane-splitter${className ? ` ${className}` : ""}`}
      role="separator"
      aria-orientation="vertical"
      aria-label={ariaLabel}
      tabIndex={0}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={end}
      onPointerCancel={end}
      onDoubleClick={onReset}
      onKeyDown={onKeyDown}
    >
      <span className="pane-splitter-grip" aria-hidden="true" />
    </div>
  );
}
