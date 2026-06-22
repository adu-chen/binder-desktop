/**
 * BinderWordmark · 文字标 `binder.`（PI-FE-02）。
 * IBM Plex Sans 500 · 字距 −0.04em · 句点用锈红强调（D-CLAY）。
 * 移植自 docs/binder-vi/binder_ui/binder-parts.jsx，视觉值照搬。
 */

interface BinderWordmarkProps {
  size?: number;
  color?: string;
  dot?: boolean;
}

export function BinderWordmark({ size = 24, color = "var(--ink)", dot = true }: BinderWordmarkProps) {
  return (
    <span
      style={{
        fontFamily: "var(--font-sans)",
        fontWeight: 500,
        fontSize: size,
        letterSpacing: -size * 0.04,
        color,
        lineHeight: 0.95,
        display: "inline-block",
      }}
    >
      binder
      {dot && <span style={{ color: "var(--rust)" }}>.</span>}
    </span>
  );
}
