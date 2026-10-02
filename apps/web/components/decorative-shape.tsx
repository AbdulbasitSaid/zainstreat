import type { CSSProperties } from "react";

export function DecorativeShape({
  className = "",
  style,
}: {
  className?: string;
  style?: CSSProperties;
}) {
  return (
    <div
      className={`decorative-shape decorative-shape--circle ${className}`}
      style={style}
      aria-hidden="true"
    />
  );
}
