/**
 * BinderMark · Binder Core VI · D-06 品牌标识（PI-FE-02）。
 * 段落条堆叠的 B 剪影——纯 SVG，无依赖。移植自 docs/binder-vi/binder_ui/binder-parts.jsx。
 * 8 条 120×120 栅格 · padX/padY=14 · barH=8 · 第 4 条（accentBar=3）锈红强调。
 * 紧凑形态（≤22px）：4 条 [100,56,100,72]，barH=16，保留强调点（视觉值照搬，不重设计）。
 */

const BINDER_BARS = [100, 64, 82, 100, 72, 100, 56, 100];
const BINDER_BARS_COMPACT = [100, 56, 100, 72];

type MarkVariant = "accented" | "default" | "negative" | "compact" | "favicon";

interface BinderMarkProps {
  size?: number;
  variant?: MarkVariant;
  ink?: string;
  paper?: string;
  accent?: string;
  accentBar?: number;
  style?: React.CSSProperties;
}

export function BinderMark({
  size = 28,
  variant = "accented",
  ink = "var(--ink)",
  paper = "var(--bg-page)",
  accent = "var(--rust)",
  accentBar = 3,
  style,
}: BinderMarkProps) {
  // 极小尺寸自动切紧凑形态（与参考稿一致）。
  const v: MarkVariant =
    variant === "accented" || variant === "default" || variant === "negative"
      ? size <= 22
        ? "compact"
        : variant
      : variant;

  const isCompact = v === "compact";
  const isInverted = v === "negative";
  const isFavicon = v === "favicon";
  const showAccent = v === "accented";

  const bars = isCompact ? BINDER_BARS_COMPACT : BINDER_BARS;
  const barH = isCompact ? 16 : 8;
  const padX = 14;
  const padY = 14;
  const innerW = 120 - 2 * padX;
  const rhythm = bars.length === 1 ? 0 : (120 - 2 * padY - barH) / (bars.length - 1);

  const barFill = isInverted ? paper : ink;
  const accentFill = isFavicon ? barFill : accent;

  return (
    <svg
      viewBox="0 0 120 120"
      width={size}
      height={size}
      style={{ display: "block", flexShrink: 0, ...style }}
      aria-label="binder"
      role="img"
    >
      {isInverted && <rect width="120" height="120" fill={ink} />}
      {bars.map((w, i) => {
        const barW = innerW * (w / 100);
        const y = padY + i * rhythm;
        const fill = showAccent && i === accentBar ? accentFill : barFill;
        return <rect key={i} x={padX} y={y} width={barW} height={barH} fill={fill} />;
      })}
    </svg>
  );
}
