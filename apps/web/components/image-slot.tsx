export function ImageSlot({ label, className = "" }: { label: string; className?: string }) {
  return (
    <div
      className={`flex min-h-[200px] items-center justify-center rounded-card border border-text/10 bg-card p-6 text-center text-sm font-semibold text-text-muted ${className}`.trim()}
      role="img"
      aria-label={label}
    >
      {label}
    </div>
  );
}
