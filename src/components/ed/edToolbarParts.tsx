import { useEffect, useRef } from "react";

/**
 * 编辑器/预览共用工具栏零件（ED · EditorToolbar / PRV · PrvToolbar 共享）。
 * TbBtn / Sep / ToolbarMenu 从 EditorToolbar 抽取，保证两处工具栏风格、语义与
 * 可访问性一致，避免两套实现漂移（PRV-TOOLBAR-ADAPTIVE-01）。
 */

/** 工具栏按钮：active 高亮 + disabled；mousedown 阻止默认避免编辑器失焦。 */
export function TbBtn({ onClick, active, disabled, title, children }: {
  onClick: () => void;
  active?: boolean;
  disabled?: boolean;
  title: string;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      className={"ed-tb-btn" + (active ? " active" : "")}
      title={title}
      aria-label={title}
      aria-pressed={active}
      disabled={disabled}
      onMouseDown={(event) => event.preventDefault()}
      onClick={onClick}
    >
      {children}
    </button>
  );
}

/** 工具栏分组分隔符。 */
export function Sep() {
  return <span className="ed-tb-sep" aria-hidden="true" />;
}

/**
 * 工具栏弹出菜单：原生 details 保留可访问开关语义；外部 pointer/focus 激活落到
 * 菜单外部时自动关闭，组内输入/色板/格式操作不触发关闭（ED-DOCUMENT-TOOL-ENRICHMENT-01）。
 */
export function ToolbarMenu({ label, title, children }: {
  label: string;
  title: string;
  children: React.ReactNode;
}) {
  const menuRef = useRef<HTMLDetailsElement>(null);

  useEffect(() => {
    const closeWhenInteractionLeavesMenu = (event: Event) => {
      const menu = menuRef.current;
      if (!menu?.open) return;
      const target = event.target;
      if (target instanceof Node && menu.contains(target)) return;
      menu.open = false;
    };

    document.addEventListener("pointerdown", closeWhenInteractionLeavesMenu, true);
    document.addEventListener("focusin", closeWhenInteractionLeavesMenu);
    return () => {
      document.removeEventListener("pointerdown", closeWhenInteractionLeavesMenu, true);
      document.removeEventListener("focusin", closeWhenInteractionLeavesMenu);
    };
  }, []);

  return (
    <details ref={menuRef} className="ed-tb-menu">
      <summary title={title} aria-label={title}>
        <span>{label}</span><span className="ed-tb-menu-caret" aria-hidden="true">⌄</span>
      </summary>
      <div className="ed-tb-menu-panel" role="group" aria-label={title}>{children}</div>
    </details>
  );
}
