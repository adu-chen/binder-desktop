import { useEffect, useMemo, useRef, useState } from "react";

/**
 * Binder 统一上下文菜单（SYS 前端交互 · 右键菜单简化方案 v2，2026-08-06）。
 * 纯展示组件：不持有业务状态，不决定启停/disabled（X-FORBID-007——由调用方从既有投影 store
 * 计算后经 items 传入）。行为：视口钳制、Esc/backdrop 关闭、↑/↓ 导航、Enter 触发、role/aria。
 * 目标面：文件树文件/目录、编辑器选区/光标、DiffCard、表格/图片、标签栏。
 */

export interface ContextMenuItemDef {
  type: "item";
  id: string;
  label: string;
  danger?: boolean;
  disabled?: boolean;
  onSelect: () => void;
}

export interface ContextMenuSeparatorDef {
  type: "separator";
  id: string;
}

export type ContextMenuDef = ContextMenuItemDef | ContextMenuSeparatorDef;

interface ContextMenuProps {
  items: ContextMenuDef[];
  x: number;
  y: number;
  onClose: () => void;
}

/** 距视口边缘的最小留白（px）。 */
const VIEWPORT_MARGIN = 8;

export function ContextMenu({ items, x, y, onClose }: ContextMenuProps) {
  const menuRef = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ left: x, top: y });
  // 菜单项在 enabled 列表中的当前位置（键盘导航光标）。
  const [cursor, setCursor] = useState(0);

  // 可选项下标（用于导航 + 渲染 data-ctx-index）。
  const enabled = useMemo(
    () => items.map((it, i) => (it.type === "item" && !it.disabled ? i : -1)).filter((i) => i >= 0),
    [items],
  );

  // 打开即视口钳制 + 聚焦菜单容器（键盘可达）。
  useEffect(() => {
    const el = menuRef.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    const left = Math.min(Math.max(x, VIEWPORT_MARGIN), Math.max(VIEWPORT_MARGIN, window.innerWidth - rect.width - VIEWPORT_MARGIN));
    const top = Math.min(Math.max(y, VIEWPORT_MARGIN), Math.max(VIEWPORT_MARGIN, window.innerHeight - rect.height - VIEWPORT_MARGIN));
    setPos({ left, top });
    el.focus();
    // eslint-disable-next-line react-hooks/exhaustive-deps -- 仅在打开时执行一次（x/y 变化视为重开）。
  }, [x, y]);

  const focusMenuItem = (index: number) => {
    menuRef.current?.querySelector<HTMLElement>(`[data-ctx-index="${index}"]`)?.focus();
  };

  const select = (index: number) => {
    const item = items[index];
    if (item?.type !== "item" || item.disabled) return;
    onClose();
    item.onSelect();
  };

  const onKeyDown = (event: React.KeyboardEvent) => {
    switch (event.key) {
      case "Escape":
        event.preventDefault();
        onClose();
        break;
      case "ArrowDown":
        event.preventDefault();
        if (enabled.length > 0) {
          const next = (cursor + 1) % enabled.length;
          setCursor(next);
          focusMenuItem(enabled[next]);
        }
        break;
      case "ArrowUp":
        event.preventDefault();
        if (enabled.length > 0) {
          const prev = (cursor - 1 + enabled.length) % enabled.length;
          setCursor(prev);
          focusMenuItem(enabled[prev]);
        }
        break;
      case "Enter":
      case " ":
        event.preventDefault();
        if (enabled[cursor] !== undefined) select(enabled[cursor]);
        break;
      default:
        break;
    }
  };

  return (
    <>
      <div
        className="ctx-menu-backdrop"
        onClick={onClose}
        onContextMenu={(e) => {
          e.preventDefault();
          onClose();
        }}
      />
      <div
        ref={menuRef}
        className="ctx-menu"
        role="menu"
        tabIndex={-1}
        style={{ left: pos.left, top: pos.top }}
        onKeyDown={onKeyDown}
        onContextMenu={(e) => e.preventDefault()}
      >
        {items.map((item) => {
          if (item.type === "separator") {
            return <div key={item.id} className="ctx-menu-sep" role="separator" />;
          }
          const isEnabledItem = !item.disabled;
          const navIndex = enabled.indexOf(items.indexOf(item));
          return (
            <button
              key={item.id}
              type="button"
              className={`ctx-menu-item${item.danger ? " ctx-menu-del" : ""}`}
              role="menuitem"
              disabled={item.disabled}
              aria-disabled={item.disabled || undefined}
              data-ctx-index={isEnabledItem ? navIndex : undefined}
              onMouseDown={(e) => {
                e.preventDefault(); // 保持触发面（编辑器选区等）不因点击菜单而失焦
                if (!item.disabled) select(items.indexOf(item));
              }}
              onFocus={() => {
                if (isEnabledItem) setCursor(navIndex);
              }}
            >
              {item.label}
            </button>
          );
        })}
      </div>
    </>
  );
}
