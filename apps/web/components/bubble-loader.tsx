// A small pot of rising, popping bubbles (globals.css `.bubble-loader`)
// with a label. Fades in after a beat (`.loader-in`) so quick loads don't
// flash it.
export function BubbleLoader({ label, className = "" }: { label: string; className?: string }) {
  return (
    <div
      role="status"
      className={`loader-in flex items-center gap-3 rounded-full bg-background/95 py-2.5 pr-5 pl-4 shadow-lg ${className}`.trim()}
    >
      <span aria-hidden="true" className="bubble-loader">
        <span />
        <span />
        <span />
        <span />
        <span />
      </span>
      <span className="text-sm font-semibold text-primary">{label}</span>
    </div>
  );
}
