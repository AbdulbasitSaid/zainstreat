"use client";

export function QuantityStepper({
  quantity,
  onDecrement,
  onIncrement,
  decrementLabel,
  incrementLabel,
}: {
  quantity: number;
  onDecrement: () => void;
  onIncrement: () => void;
  decrementLabel: string;
  incrementLabel: string;
}) {
  return (
    <div className="inline-flex items-center gap-3 rounded-full border border-text/15 px-1 py-1">
      <button
        type="button"
        aria-label={decrementLabel}
        onClick={onDecrement}
        className="flex h-7 w-7 items-center justify-center rounded-full text-text hover:bg-background-soft"
      >
        −
      </button>
      <span aria-live="polite" className="w-5 text-center text-sm font-semibold">
        {quantity}
      </span>
      <button
        type="button"
        aria-label={incrementLabel}
        onClick={onIncrement}
        className="flex h-7 w-7 items-center justify-center rounded-full text-text hover:bg-background-soft"
      >
        +
      </button>
    </div>
  );
}
