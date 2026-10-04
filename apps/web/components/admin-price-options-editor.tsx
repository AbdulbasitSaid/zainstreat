"use client";

import { useState } from "react";
import { Button } from "@/components/button";

export interface PriceOptionInput {
  label: string;
  price: string;
}

export interface PriceOptionsValue {
  price: string | null;
  price_options: PriceOptionInput[];
}

interface OptionDraft extends PriceOptionInput {
  id: string;
}

function withIds(options: PriceOptionInput[]): OptionDraft[] {
  return options.map((option) => ({ ...option, id: crypto.randomUUID() }));
}

function withoutIds(options: OptionDraft[]): PriceOptionInput[] {
  return options.map(({ label, price }) => ({ label, price }));
}

/**
 * requirement.md Decision 5 — an item is either flat-priced or has labeled
 * size options, never both. Lifts the combined value up on every change so
 * the parent form owns submission; this component holds no submit button.
 */
export function AdminPriceOptionsEditor({
  initialPrice,
  initialOptions,
  onChange,
}: {
  initialPrice: string | null;
  initialOptions: PriceOptionInput[];
  onChange: (value: PriceOptionsValue) => void;
}) {
  const [mode, setMode] = useState<"flat" | "multiple">(
    initialOptions.length > 0 ? "multiple" : "flat",
  );
  const [flatPrice, setFlatPrice] = useState(initialPrice ?? "");
  const [options, setOptions] = useState<OptionDraft[]>(
    withIds(initialOptions.length > 0 ? initialOptions : [{ label: "", price: "" }]),
  );

  function emit(nextMode: "flat" | "multiple", nextFlatPrice: string, nextOptions: OptionDraft[]) {
    onChange(
      nextMode === "flat"
        ? { price: nextFlatPrice, price_options: [] }
        : { price: null, price_options: withoutIds(nextOptions) },
    );
  }

  function handleModeChange(nextMode: "flat" | "multiple") {
    setMode(nextMode);
    emit(nextMode, flatPrice, options);
  }

  function handleFlatPriceChange(value: string) {
    setFlatPrice(value);
    emit("flat", value, options);
  }

  function handleOptionChange(id: string, field: "label" | "price", value: string) {
    const next = options.map((option) => (option.id === id ? { ...option, [field]: value } : option));
    setOptions(next);
    emit("multiple", flatPrice, next);
  }

  function handleAddOption() {
    const next = [...options, { id: crypto.randomUUID(), label: "", price: "" }];
    setOptions(next);
    emit("multiple", flatPrice, next);
  }

  function handleRemoveOption(id: string) {
    const next = options.filter((option) => option.id !== id);
    setOptions(next);
    emit("multiple", flatPrice, next);
  }

  return (
    <div className="flex flex-col gap-3">
      <div className="flex gap-2" role="radiogroup" aria-label="Pricing mode">
        {(["flat", "multiple"] as const).map((value) => (
          <button
            key={value}
            type="button"
            role="radio"
            aria-checked={mode === value}
            onClick={() => handleModeChange(value)}
            className={`rounded-full px-4 py-2 text-xs font-semibold uppercase tracking-wider transition-colors duration-150 focus-visible:outline focus-visible:outline-[3px] focus-visible:outline-accent focus-visible:outline-offset-2 ${
              mode === value ? "bg-primary text-white" : "bg-card text-text hover:bg-background-soft"
            }`}
          >
            {value === "flat" ? "Flat price" : "Multiple sizes"}
          </button>
        ))}
      </div>

      {mode === "flat" ? (
        <div>
          <label htmlFor="menu-item-price" className="mb-1.5 block text-sm font-semibold">
            Price (€)
          </label>
          <input
            id="menu-item-price"
            type="text"
            inputMode="decimal"
            className="field max-w-[12rem]"
            value={flatPrice}
            onChange={(event) => handleFlatPriceChange(event.target.value)}
          />
        </div>
      ) : (
        <div className="flex flex-col gap-3">
          {options.map((option, index) => (
            <div key={option.id} className="flex items-end gap-3">
              <div className="flex-1">
                <label htmlFor={`price-option-label-${option.id}`} className="mb-1.5 block text-sm font-semibold">
                  Label
                </label>
                <input
                  id={`price-option-label-${option.id}`}
                  type="text"
                  className="field"
                  value={option.label}
                  onChange={(event) => handleOptionChange(option.id, "label", event.target.value)}
                />
              </div>
              <div className="w-32">
                <label htmlFor={`price-option-price-${option.id}`} className="mb-1.5 block text-sm font-semibold">
                  Price (€)
                </label>
                <input
                  id={`price-option-price-${option.id}`}
                  type="text"
                  inputMode="decimal"
                  className="field"
                  value={option.price}
                  onChange={(event) => handleOptionChange(option.id, "price", event.target.value)}
                />
              </div>
              <Button
                type="button"
                variant="secondary"
                onClick={() => handleRemoveOption(option.id)}
                disabled={options.length <= 1}
              >
                Remove
              </Button>
            </div>
          ))}
          <Button type="button" variant="secondary" onClick={handleAddOption} className="self-start">
            Add size {options.length + 1}
          </Button>
        </div>
      )}
    </div>
  );
}
