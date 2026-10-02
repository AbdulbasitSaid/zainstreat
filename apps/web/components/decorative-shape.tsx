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
      className={`absolute z-0 rounded-full bg-accent-light opacity-60 pointer-events-none motion-safe:animate-float-rotate ${className}`.trim()}
      style={style}
      aria-hidden="true"
    />
  );
}
