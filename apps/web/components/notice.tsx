import type { ReactNode } from "react";

export function Notice({ children, className = "" }: { children: ReactNode; className?: string }) {
  return (
    <p className={`rounded-card bg-accent-light px-5 py-4 text-text ${className}`.trim()}>
      {children}
    </p>
  );
}
