import type { ReactNode } from "react";

// `light` is for eyebrows over a photo or a dark background.
export function Eyebrow({ children, tone = "default" }: { children: ReactNode; tone?: "default" | "light" }) {
  const color = tone === "light" ? "text-accent-light" : "text-primary";
  return (
    <span className={`mb-3 inline-block text-[0.8rem] font-semibold tracking-[0.14em] ${color} uppercase`}>
      {children}
    </span>
  );
}
