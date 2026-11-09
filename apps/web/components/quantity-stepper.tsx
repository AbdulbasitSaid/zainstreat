"use client";

import { LiquidSurface } from "@/components/liquid-surface";

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
        className="group/liquid relative isolate flex h-7 w-7 items-center justify-center rounded-full text-text hover:bg-background-soft"
      >
        −
        <StepperLiquid />
      </button>
      <span aria-live="polite" className="w-5 text-center text-sm font-semibold">
        {quantity}
      </span>
      <button
        type="button"
        aria-label={incrementLabel}
        onClick={onIncrement}
        className="group/liquid relative isolate flex h-7 w-7 items-center justify-center rounded-full text-text hover:bg-background-soft"
      >
        +
        <StepperLiquid />
      </button>
    </div>
  );
}

function StepperLiquid() {
  return (
    <LiquidSurface
      pathClassName="fill-background-soft/0 transition-[fill] duration-150 group-hover/liquid:fill-background-soft"
      splashClassName="fill-primary"
    />
  );
}
