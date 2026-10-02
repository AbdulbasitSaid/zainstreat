import type { ReactNode } from "react";

export function Eyebrow({ children }: { children: ReactNode }) {
  return (
    <span className="mb-3 inline-block text-[0.8rem] font-semibold tracking-[0.14em] text-primary uppercase">
      {children}
    </span>
  );
}
