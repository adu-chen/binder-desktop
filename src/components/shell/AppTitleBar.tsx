interface AppTitleBarProps {
  title: string;
}

/**
 * Binder 自有窗口标题栏：配合 Tauri macOS overlay titlebar，保留系统 traffic lights。
 * 标题栏只显示当前标题；workspace 交互统一放到模块栏 Logo。
 * 拖动交给 Tauri 注入的 data-tauri-drag-region 脚本处理，避免 React 事件与 overlay 命中层偏差。
 */
export function AppTitleBar({ title }: AppTitleBarProps) {
  return (
    <header className="app-titlebar" data-tauri-drag-region="deep">
      <div className="app-titlebar-title" title={title}>
        {title}
      </div>
    </header>
  );
}
