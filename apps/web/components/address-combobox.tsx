"use client";

import { useEffect, useId, useRef, useState, type KeyboardEvent } from "react";
import {
  SUGGEST_MIN_CHARS,
  fetchSuggestions,
  type AddressSuggestion,
  type DeliveryApiBase,
} from "@/lib/delivery";

const DEBOUNCE_MS = 300;

export interface AddressComboboxLabels {
  label: string;
  hint: string;
  searching: string;
  noMatches: string;
}

/**
 * Dutch address search for the delivery fee (Phase 37 Decisions 9, 11):
 * an ARIA 1.2 combobox over PDOK suggestions, proxied by the API. Typing
 * clears the picked address; `onLookupDown` fires when the lookup can't be
 * reached, so checkout can fall back to a typed address (Decision 17).
 */
export function AddressCombobox({
  id,
  apiBase,
  labels,
  initialQuery = "",
  invalid = false,
  describedBy,
  onSelect,
  onLookupDown,
}: {
  id: string;
  apiBase: DeliveryApiBase;
  labels: AddressComboboxLabels;
  initialQuery?: string;
  invalid?: boolean;
  describedBy?: string;
  onSelect: (suggestion: AddressSuggestion | null) => void;
  onLookupDown?: () => void;
}) {
  const listboxId = useId();
  const hintId = useId();
  const [query, setQuery] = useState(initialQuery);
  const [suggestions, setSuggestions] = useState<AddressSuggestion[]>([]);
  const [open, setOpen] = useState(false);
  const [searching, setSearching] = useState(false);
  const [activeIndex, setActiveIndex] = useState(-1);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const controller = useRef<AbortController | null>(null);

  useEffect(
    () => () => {
      if (timer.current) clearTimeout(timer.current);
      controller.current?.abort();
    },
    [],
  );

  function search(text: string) {
    if (timer.current) clearTimeout(timer.current);
    controller.current?.abort();
    if (text.trim().length < SUGGEST_MIN_CHARS) {
      setSuggestions([]);
      setOpen(false);
      setSearching(false);
      return;
    }
    setSearching(true);
    timer.current = setTimeout(async () => {
      const abort = new AbortController();
      controller.current = abort;
      const result = await fetchSuggestions(apiBase, text.trim(), abort.signal);
      if (!result.ok) {
        if (result.reason === "lookup_down") {
          setSearching(false);
          setOpen(false);
          onLookupDown?.();
        }
        return;
      }
      setSuggestions(result.suggestions);
      setActiveIndex(-1);
      setOpen(true);
      setSearching(false);
    }, DEBOUNCE_MS);
  }

  function choose(suggestion: AddressSuggestion) {
    setQuery(suggestion.label);
    setOpen(false);
    setSuggestions([]);
    onSelect(suggestion);
  }

  function handleKeyDown(event: KeyboardEvent<HTMLInputElement>) {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      if (!open && suggestions.length > 0) setOpen(true);
      setActiveIndex((index) => Math.min(index + 1, suggestions.length - 1));
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      setActiveIndex((index) => Math.max(index - 1, 0));
    } else if (event.key === "Enter" && open && activeIndex >= 0 && suggestions[activeIndex]) {
      // Picks the option instead of submitting the form.
      event.preventDefault();
      choose(suggestions[activeIndex]);
    } else if (event.key === "Escape" && open) {
      event.preventDefault();
      setOpen(false);
    }
  }

  const expanded = open && query.trim().length >= SUGGEST_MIN_CHARS;
  const optionId = (index: number) => `${listboxId}-option-${index}`;

  return (
    <div className="relative">
      <label htmlFor={id} className="mb-1.5 block text-sm font-semibold text-text">
        {labels.label}
      </label>
      <input
        id={id}
        className="field"
        role="combobox"
        aria-autocomplete="list"
        aria-expanded={expanded}
        aria-controls={listboxId}
        aria-activedescendant={expanded && activeIndex >= 0 ? optionId(activeIndex) : undefined}
        aria-invalid={invalid}
        aria-describedby={[hintId, describedBy].filter(Boolean).join(" ")}
        autoComplete="off"
        value={query}
        onChange={(event) => {
          setQuery(event.target.value);
          onSelect(null);
          search(event.target.value);
        }}
        onKeyDown={handleKeyDown}
        onBlur={() => setOpen(false)}
        onFocus={() => suggestions.length > 0 && setOpen(true)}
      />
      <p id={hintId} className="mt-1 text-sm text-text-muted">
        {searching ? labels.searching : labels.hint}
      </p>
      <ul
        id={listboxId}
        role="listbox"
        aria-label={labels.label}
        hidden={!expanded}
        className="absolute inset-x-0 top-[4.75rem] z-20 m-0 max-h-72 list-none overflow-auto rounded-card border border-accent/40 bg-background p-1 shadow-lg"
      >
        {suggestions.length === 0 ? (
          <li role="option" aria-selected={false} aria-disabled className="px-3 py-2 text-sm text-text-muted">
            {labels.noMatches}
          </li>
        ) : (
          suggestions.map((suggestion, index) => (
            <li
              key={suggestion.id}
              id={optionId(index)}
              role="option"
              aria-selected={index === activeIndex}
              // mousedown, not click: the input's blur would close the list first.
              onMouseDown={(event) => {
                event.preventDefault();
                choose(suggestion);
              }}
              onMouseEnter={() => setActiveIndex(index)}
              className={`cursor-pointer rounded-lg px-3 py-2 text-sm ${index === activeIndex ? "bg-background-soft text-primary" : ""}`}
            >
              {suggestion.label}
            </li>
          ))
        )}
      </ul>
    </div>
  );
}
