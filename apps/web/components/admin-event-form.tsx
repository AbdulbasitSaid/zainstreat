"use client";

import { useState, type FormEvent } from "react";
import { useRouter } from "next/navigation";
import { AdminImageUpload } from "@/components/admin-image-upload";
import { AdminPlateOptions } from "@/components/admin-plate-options";
import { Button } from "@/components/button";
import { Notice } from "@/components/notice";
import {
  EVENT_LOCALES,
  PLATE_TEXT_KEYS,
  emptyContent,
  type AdminEvent,
  type EventInput,
  type EventLocale,
  type EventTextKey,
  type PlateInput,
} from "@/lib/admin-events";
import type { FieldError } from "@/lib/orders";

const LABEL = "mb-1.5 block text-sm font-semibold text-text";
const HINT = "mt-1 text-xs text-text-muted";
const LOCALE_NAMES: Record<EventLocale, string> = { en: "English", nl: "Dutch", fr: "French" };

// Text fields on the details tab; the recap is on the past-event tab.
const TEXT_FIELDS: { key: EventTextKey; label: string; rows: number; hint?: string }[] = [
  { key: "title", label: "Title", rows: 1 },
  { key: "summary", label: "Summary", rows: 2, hint: "One or two sentences for the event card." },
  { key: "story", label: "Story", rows: 6, hint: "The event page's main text. Blank lines start new paragraphs." },
  { key: "terms", label: "Cancellation and refund terms", rows: 4, hint: "Shown on the form before guests pay, and in the payment email." },
];

const ERROR_MESSAGES: Record<string, string> = {
  required: "is required",
  invalid: "is invalid",
  locked: "can't change once someone has booked",
  before_start: "must be after the start",
  after_start: "must be before the start",
  out_of_range: "is out of range",
  too_many: "has too many entries",
  too_few: "needs at least two choices",
};

function newPlate(): PlateInput {
  return { id: null, content: emptyContent(PLATE_TEXT_KEYS), price: "15.00", is_available: true, options: [] };
}

function blankInput(): EventInput {
  return {
    slug: null,
    content: emptyContent(["title", "summary", "story", "terms", "recap"] as const),
    starts_at: "",
    ends_at: null,
    area: "",
    address: "",
    capacity: 16,
    max_party_size: 6,
    meal_orders_close_at: null,
    rsvp_closes_at: null,
    payment_hold_hours: 48,
    cover_image_url: null,
    plates: [newPlate()],
  };
}

/**
 * Create or edit an event's details and plates (Phase 31 Decision 18). New
 * events save as unpublished drafts. Text has an English, Dutch and French
 * version; English is required and the others fall back to it.
 */
export function AdminEventForm({ event, initial }: { event: AdminEvent | null; initial: EventInput | null }) {
  const router = useRouter();
  const [input, setInput] = useState<EventInput>(initial ?? blankInput());
  const [locale, setLocale] = useState<EventLocale>("en");
  const [saving, setSaving] = useState(false);
  const [errors, setErrors] = useState<FieldError[]>([]);
  const [saved, setSaved] = useState(false);
  const [failed, setFailed] = useState(false);

  const set = <K extends keyof EventInput>(key: K, value: EventInput[K]) => {
    setInput((current) => ({ ...current, [key]: value }));
    setSaved(false);
  };
  const setText = (key: EventTextKey, value: string) =>
    set("content", { ...input.content, [locale]: { ...input.content[locale], [key]: value } });
  const setPlate = (index: number, plate: PlateInput) =>
    set("plates", input.plates.map((p, i) => (i === index ? plate : p)));
  const movePlate = (index: number, delta: number) => {
    const next = [...input.plates];
    const [plate] = next.splice(index, 1);
    next.splice(index + delta, 0, plate);
    set("plates", next);
  };

  async function handleSubmit(formEvent: FormEvent) {
    formEvent.preventDefault();
    setSaving(true);
    setErrors([]);
    setFailed(false);
    const response = await fetch(event ? `/api/admin/events/${event.id}` : "/api/admin/events", {
      method: event ? "PUT" : "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        ...input,
        ends_at: input.ends_at || null,
        meal_orders_close_at: input.meal_orders_close_at || null,
        rsvp_closes_at: input.rsvp_closes_at || null,
      }),
    });
    setSaving(false);
    const body: unknown = await response.json().catch(() => null);
    if (response.ok && typeof body === "object" && body !== null && "id" in body) {
      const saved = body as AdminEvent;
      if (!event) {
        router.push(`/events/${saved.id}`);
        return;
      }
      setInput((current) => ({ ...current, slug: saved.slug, plates: saved.plates.map((p) => ({ ...p })) }));
      setSaved(true);
      router.refresh();
      return;
    }
    if (response.status === 400 && typeof body === "object" && body !== null && "fields" in body) {
      setErrors((body as { fields: FieldError[] }).fields);
      return;
    }
    setFailed(true);
  }

  const slugLocked = event?.has_bookings ?? false;

  return (
    <form onSubmit={handleSubmit} noValidate className="flex flex-col gap-8">
      {errors.length > 0 && (
        <div role="alert" className="rounded-card bg-accent-light px-5 py-4 text-sm">
          <p className="m-0 font-semibold">Please fix these and save again:</p>
          <ul className="mb-0">
            {errors.map((error) => (
              <li key={`${error.field}-${error.message}`}>
                <code>{error.field}</code> {ERROR_MESSAGES[error.message] ?? error.message}
              </li>
            ))}
          </ul>
        </div>
      )}

      <section className="flex flex-col gap-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <h2 className="m-0 text-xl">Text</h2>
          <div role="tablist" aria-label="Language" className="flex gap-1 rounded-full bg-background-soft p-1">
            {EVENT_LOCALES.map((code) => (
              <button
                key={code}
                type="button"
                role="tab"
                aria-selected={locale === code}
                onClick={() => setLocale(code)}
                className={`rounded-full px-4 py-1.5 text-sm font-semibold ${locale === code ? "bg-primary text-white" : "text-text"}`}
              >
                {LOCALE_NAMES[code]}
              </button>
            ))}
          </div>
        </div>
        {locale !== "en" && <p className="m-0 text-sm text-text-muted">Leave a field blank to show the English text.</p>}
        {TEXT_FIELDS.map(({ key, label, rows, hint }) => (
          <div key={key}>
            <label htmlFor={`event-${key}`} className={LABEL}>
              {label}
              {key === "title" && locale === "en" ? " (required)" : ""}
            </label>
            {rows === 1 ? (
              <input
                id={`event-${key}`}
                className="field"
                value={input.content[locale][key]}
                placeholder={locale === "en" ? "" : input.content.en[key]}
                onChange={(e) => setText(key, e.target.value)}
              />
            ) : (
              <textarea
                id={`event-${key}`}
                className="field"
                rows={rows}
                value={input.content[locale][key]}
                placeholder={locale === "en" ? "" : input.content.en[key]}
                onChange={(e) => setText(key, e.target.value)}
              />
            )}
            {hint && <p className={HINT}>{hint}</p>}
          </div>
        ))}
      </section>

      <section className="grid gap-4 sm:grid-cols-2">
        <h2 className="m-0 text-xl sm:col-span-2">When and where</h2>
        <div>
          <label htmlFor="event-starts" className={LABEL}>Starts (required)</label>
          <input id="event-starts" type="datetime-local" className="field" value={input.starts_at} onChange={(e) => set("starts_at", e.target.value)} />
        </div>
        <div>
          <label htmlFor="event-ends" className={LABEL}>Ends</label>
          <input id="event-ends" type="datetime-local" className="field" value={input.ends_at ?? ""} onChange={(e) => set("ends_at", e.target.value)} />
          <p className={HINT}>The event moves to “past events” after this (or after the start, if blank).</p>
        </div>
        <div>
          <label htmlFor="event-meal-cutoff" className={LABEL}>Meal orders close</label>
          <input
            id="event-meal-cutoff"
            type="datetime-local"
            className="field"
            value={input.meal_orders_close_at ?? ""}
            onChange={(e) => set("meal_orders_close_at", e.target.value)}
          />
          <p className={HINT}>After this, guests can only RSVP without a meal, and can&apos;t cancel online.</p>
        </div>
        <div>
          <label htmlFor="event-rsvp-close" className={LABEL}>Bookings close</label>
          <input
            id="event-rsvp-close"
            type="datetime-local"
            className="field"
            value={input.rsvp_closes_at ?? ""}
            onChange={(e) => set("rsvp_closes_at", e.target.value)}
          />
          <p className={HINT}>Blank: open until the event starts.</p>
        </div>
        <div>
          <label htmlFor="event-area" className={LABEL}>Area (public, required)</label>
          <input id="event-area" className="field" value={input.area} onChange={(e) => set("area", e.target.value)} />
          <p className={HINT}>e.g. “A home in Utrecht”. Shown to everyone.</p>
        </div>
        <div>
          <label htmlFor="event-address" className={LABEL}>Address (private, required)</label>
          <input id="event-address" className="field" value={input.address} onChange={(e) => set("address", e.target.value)} />
          <p className={HINT}>Only shown to confirmed guests, on their booking page and confirmation email.</p>
        </div>
      </section>

      <section className="grid gap-4 sm:grid-cols-3">
        <h2 className="m-0 text-xl sm:col-span-3">Seats and payment</h2>
        <div>
          <label htmlFor="event-capacity" className={LABEL}>Seats</label>
          <input id="event-capacity" type="number" min={1} className="field" value={input.capacity} onChange={(e) => set("capacity", Number(e.target.value))} />
          <p className={HINT}>Guests without a meal take a seat too.</p>
        </div>
        <div>
          <label htmlFor="event-party" className={LABEL}>Max people per booking</label>
          <input id="event-party" type="number" min={1} className="field" value={input.max_party_size} onChange={(e) => set("max_party_size", Number(e.target.value))} />
        </div>
        <div>
          <label htmlFor="event-hold" className={LABEL}>Hours to pay</label>
          <input id="event-hold" type="number" min={1} className="field" value={input.payment_hold_hours} onChange={(e) => set("payment_hold_hours", Number(e.target.value))} />
          <p className={HINT}>Unpaid bookings are released after this.</p>
        </div>
      </section>

      <section className="flex flex-col gap-4">
        <h2 className="m-0 text-xl">Plates</h2>
        <p className="m-0 text-sm text-text-muted">
          Guests pick one per person, or “attending without a meal”. The first plate is the headline.
        </p>
        <ol className="m-0 flex list-none flex-col gap-4 p-0">
          {input.plates.map((plate, index) => (
            <li key={plate.id ?? `new-${index}`} className="flex flex-col gap-3 rounded-card border border-text/10 p-4">
              <div className="grid gap-3 sm:grid-cols-[1fr_8rem]">
                <div>
                  <label htmlFor={`plate-${index}-name`} className={LABEL}>
                    Name ({LOCALE_NAMES[locale]})
                  </label>
                  <input
                    id={`plate-${index}-name`}
                    className="field"
                    value={plate.content[locale].name}
                    placeholder={locale === "en" ? "" : plate.content.en.name}
                    onChange={(e) =>
                      setPlate(index, { ...plate, content: { ...plate.content, [locale]: { ...plate.content[locale], name: e.target.value } } })
                    }
                  />
                </div>
                <div>
                  <label htmlFor={`plate-${index}-price`} className={LABEL}>Price (€)</label>
                  <input
                    id={`plate-${index}-price`}
                    className="field"
                    inputMode="decimal"
                    value={plate.price}
                    onChange={(e) => setPlate(index, { ...plate, price: e.target.value.replace(",", ".") })}
                  />
                </div>
              </div>
              <div>
                <label htmlFor={`plate-${index}-description`} className={LABEL}>
                  Description ({LOCALE_NAMES[locale]})
                </label>
                <input
                  id={`plate-${index}-description`}
                  className="field"
                  value={plate.content[locale].description}
                  placeholder={locale === "en" ? "" : plate.content.en.description}
                  onChange={(e) =>
                    setPlate(index, {
                      ...plate,
                      content: { ...plate.content, [locale]: { ...plate.content[locale], description: e.target.value } },
                    })
                  }
                />
              </div>
              <AdminPlateOptions
                groups={plate.options}
                locale={locale}
                localeName={LOCALE_NAMES[locale]}
                idPrefix={`plate-${index}`}
                onChange={(options) => setPlate(index, { ...plate, options })}
              />
              <div className="flex flex-wrap items-center gap-3 text-sm">
                <label className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={plate.is_available}
                    onChange={(e) => setPlate(index, { ...plate, is_available: e.target.checked })}
                    className="accent-primary"
                  />
                  Available
                </label>
                <span className="flex-1" />
                <Button type="button" variant="secondary" className="px-4 py-2" disabled={index === 0} onClick={() => movePlate(index, -1)}>
                  ↑ Up
                </Button>
                <Button
                  type="button"
                  variant="secondary"
                  className="px-4 py-2"
                  disabled={index === input.plates.length - 1}
                  onClick={() => movePlate(index, 1)}
                >
                  ↓ Down
                </Button>
                <Button
                  type="button"
                  variant="outline"
                  className="px-4 py-2"
                  onClick={() => set("plates", input.plates.filter((_, i) => i !== index))}
                >
                  Remove
                </Button>
              </div>
            </li>
          ))}
        </ol>
        <Button type="button" variant="outline" className="self-start" onClick={() => set("plates", [...input.plates, newPlate()])}>
          Add a plate
        </Button>
      </section>

      <section className="flex flex-col gap-4">
        <h2 className="m-0 text-xl">Cover photo</h2>
        <AdminImageUpload
          currentImageUrl={input.cover_image_url}
          onUploaded={(url) => set("cover_image_url", url)}
          aspect={16 / 9}
          inputId="event-cover"
          label="Cover photo (16:9)"
        />
      </section>

      <section className="flex flex-col gap-2">
        <label htmlFor="event-slug" className={LABEL}>Link</label>
        <div className="flex items-center gap-2">
          <span className="text-sm text-text-muted">/events/</span>
          <input
            id="event-slug"
            className="field"
            value={input.slug ?? ""}
            disabled={slugLocked}
            placeholder="made from the English title"
            onChange={(e) => set("slug", e.target.value.toLowerCase().replace(/[^a-z0-9-]/g, "-"))}
          />
        </div>
        <p className={HINT}>{slugLocked ? "Locked: guests already have links to this page." : "Lowercase letters, numbers and dashes."}</p>
      </section>

      <div className="sticky bottom-0 -mx-2 flex flex-wrap items-center gap-4 border-t border-text/10 bg-background px-2 py-4">
        <Button type="submit" disabled={saving}>
          {saving ? "Saving…" : event ? "Save changes" : "Create draft"}
        </Button>
        {saved && <span className="text-sm font-semibold text-whatsapp-dark">Saved</span>}
        {failed && <Notice className="m-0 py-2">Couldn&apos;t save. Please try again.</Notice>}
      </div>
    </form>
  );
}
