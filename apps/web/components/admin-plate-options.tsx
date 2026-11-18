"use client";

import { Button } from "@/components/button";
import { emptyLabels, type EventLocale, type OptionGroupInput, type OptionLabels } from "@/lib/admin-events";
import { isPickOne } from "@/lib/events";

const SMALL_BUTTON = "px-3 py-1.5 text-sm";

function move<T>(items: T[], index: number, delta: number): T[] {
  const next = [...items];
  const [item] = next.splice(index, 1);
  next.splice(index + delta, 0, item);
  return next;
}

function newGroup(): OptionGroupInput {
  return {
    key: "",
    label: emptyLabels(),
    choices: [{ key: "", label: emptyLabels() }, { key: "", label: emptyLabels() }],
    min: 1,
    max: 1,
  };
}

// The rule as guests read it, in English (Phase 35 Decision 12), or why
// the limits can't be saved.
function limitsPreview(group: OptionGroupInput): { ok: boolean; text: string } {
  const { min, max } = group;
  const count = group.choices.length;
  if (max < 1 || min > max || max > count) {
    return { ok: false, text: `Min must be 0–${Math.min(max, count)} and max 1–${count} (the number of choices).` };
  }
  if (isPickOne(group)) return { ok: true, text: "Guests choose exactly one." };
  if (min === max) return { ok: true, text: `Guests choose exactly ${max}.` };
  if (min === 0) return { ok: true, text: `Optional: guests choose up to ${max}.` };
  if (min === 1) return { ok: true, text: `Guests choose up to ${max}.` };
  return { ok: true, text: `Guests choose ${min} to ${max}.` };
}

/**
 * A plate's option groups, e.g. "Protein: assorted meat, fish" (Phase 32
 * Decision 7), each with how many choices a guest picks (Phase 35
 * Decision 12). Labels follow the form's language tabs; blank nl/fr labels
 * show the English.
 */
export function AdminPlateOptions({
  groups,
  locale,
  idPrefix,
  localeName,
  onChange,
}: {
  groups: OptionGroupInput[];
  locale: EventLocale;
  idPrefix: string;
  localeName: string;
  onChange: (groups: OptionGroupInput[]) => void;
}) {
  const setGroup = (index: number, group: OptionGroupInput) => onChange(groups.map((g, i) => (i === index ? group : g)));
  const relabel = (labels: OptionLabels, text: string): OptionLabels => ({ ...labels, [locale]: text });

  return (
    <div className="flex flex-col gap-3">
      <div>
        <p className="m-0 text-sm font-semibold">Options</p>
        <p className="m-0 text-xs text-text-muted">
          Guests choose in each group, per person, e.g. protein or side. Min and max set how many. Choices don&apos;t change the
          price.
        </p>
      </div>
      {groups.map((group, g) => (
        <fieldset key={group.key || `new-${g}`} className="m-0 flex flex-col gap-2 rounded-xl border-0 bg-background-soft p-3">
          <legend className="sr-only">Option group {g + 1}</legend>
          <div className="flex flex-wrap items-end gap-2">
            <div className="min-w-40 flex-1">
              <label htmlFor={`${idPrefix}-group-${g}`} className="mb-1 block text-xs font-semibold">
                Group ({localeName})
              </label>
              <input
                id={`${idPrefix}-group-${g}`}
                className="field"
                value={group.label[locale]}
                placeholder={locale === "en" ? "e.g. Protein" : group.label.en}
                onChange={(e) => setGroup(g, { ...group, label: relabel(group.label, e.target.value) })}
              />
            </div>
            <Button type="button" variant="secondary" className={SMALL_BUTTON} disabled={g === 0} onClick={() => onChange(move(groups, g, -1))} aria-label={`Move group ${g + 1} up`}>
              ↑
            </Button>
            <Button
              type="button"
              variant="secondary"
              className={SMALL_BUTTON}
              disabled={g === groups.length - 1}
              onClick={() => onChange(move(groups, g, 1))}
              aria-label={`Move group ${g + 1} down`}
            >
              ↓
            </Button>
            <Button type="button" variant="outline" className={SMALL_BUTTON} onClick={() => onChange(groups.filter((_, i) => i !== g))}>
              Remove group
            </Button>
          </div>
          <div className="flex flex-wrap items-end gap-3">
            {(["min", "max"] as const).map((limit) => (
              <div key={limit} className="w-20">
                <label htmlFor={`${idPrefix}-group-${g}-${limit}`} className="mb-1 block text-xs font-semibold">
                  {limit === "min" ? "Min" : "Max"}
                </label>
                <input
                  id={`${idPrefix}-group-${g}-${limit}`}
                  type="number"
                  min={limit === "min" ? 0 : 1}
                  max={group.choices.length}
                  className="field"
                  value={group[limit]}
                  aria-describedby={`${idPrefix}-group-${g}-limits`}
                  onChange={(e) => setGroup(g, { ...group, [limit]: Math.max(0, Math.trunc(Number(e.target.value))) })}
                />
              </div>
            ))}
            {(() => {
              const preview = limitsPreview(group);
              return (
                <p
                  id={`${idPrefix}-group-${g}-limits`}
                  className={`m-0 pb-2 text-xs ${preview.ok ? "text-text-muted" : "font-semibold text-primary"}`}
                >
                  {preview.text}
                </p>
              );
            })()}
          </div>
          <ol className="m-0 flex list-none flex-col gap-2 p-0 sm:pl-4">
            {group.choices.map((choice, c) => (
              <li key={choice.key || `new-${c}`} className="flex flex-wrap items-center gap-2">
                <label htmlFor={`${idPrefix}-group-${g}-choice-${c}`} className="sr-only">
                  Choice {c + 1} ({localeName})
                </label>
                <input
                  id={`${idPrefix}-group-${g}-choice-${c}`}
                  className="field min-w-40 flex-1"
                  value={choice.label[locale]}
                  placeholder={locale === "en" ? `Choice ${c + 1}` : choice.label.en}
                  onChange={(e) =>
                    setGroup(g, {
                      ...group,
                      choices: group.choices.map((ch, i) => (i === c ? { ...ch, label: relabel(ch.label, e.target.value) } : ch)),
                    })
                  }
                />
                <Button
                  type="button"
                  variant="secondary"
                  className={SMALL_BUTTON}
                  disabled={c === 0}
                  onClick={() => setGroup(g, { ...group, choices: move(group.choices, c, -1) })}
                  aria-label={`Move choice ${c + 1} up`}
                >
                  ↑
                </Button>
                <Button
                  type="button"
                  variant="secondary"
                  className={SMALL_BUTTON}
                  disabled={c === group.choices.length - 1}
                  onClick={() => setGroup(g, { ...group, choices: move(group.choices, c, 1) })}
                  aria-label={`Move choice ${c + 1} down`}
                >
                  ↓
                </Button>
                <Button
                  type="button"
                  variant="outline"
                  className={SMALL_BUTTON}
                  onClick={() => setGroup(g, { ...group, choices: group.choices.filter((_, i) => i !== c) })}
                  aria-label={`Remove choice ${c + 1}`}
                >
                  ✕
                </Button>
              </li>
            ))}
          </ol>
          <Button
            type="button"
            variant="outline"
            className={`${SMALL_BUTTON} self-start`}
            onClick={() => setGroup(g, { ...group, choices: [...group.choices, { key: "", label: emptyLabels() }] })}
          >
            Add a choice
          </Button>
        </fieldset>
      ))}
      <Button type="button" variant="outline" className={`${SMALL_BUTTON} self-start`} onClick={() => onChange([...groups, newGroup()])}>
        Add an option group
      </Button>
    </div>
  );
}
