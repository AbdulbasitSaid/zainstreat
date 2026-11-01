"use client";

import { useState, type ReactNode } from "react";
import { useRouter } from "next/navigation";
import Link from "next/link";
import { Button, buttonClasses } from "@/components/button";
import { Notice } from "@/components/notice";
import type { CateringEnquiryDetail } from "@/lib/admin-enquiries";
import { FULFILMENT_LABELS } from "@/lib/admin-orders";
import { formatDateTime, formatPrice } from "@/lib/format";
import type { FulfilmentType } from "@/lib/orders";
import { centsToAmount, quoteShareMessage, toCents, type SendQuoteResponse } from "@/lib/quotes";
import { buildWhatsAppLinkTo, toWhatsAppNumber } from "@/lib/whatsapp";

interface Line {
  description: string;
  quantity: string;
  unitPrice: string;
}

type Errors = Record<string, string>;

const FULFILMENT_OPTIONS: FulfilmentType[] = ["on_site", "delivery", "pickup"];
const EMPTY_LINE: Line = { description: "", quantity: "1", unitPrice: "" };
// Phase 23 Decision 16: a nudge, never a block.
const LARGE_TOTAL_CENTS = 1_000_000;
const DEFAULT_VALIDITY_DAYS = 14;

// The API's field-error messages, in admin's English.
const MESSAGES: Record<string, string> = {
  required: "Required.",
  empty: "Add at least one line.",
  too_many: "At most 50 lines.",
  too_long: "Too long.",
  too_short: "Too short.",
  out_of_range: "Out of range.",
  too_precise: "At most 2 decimals.",
  must_be_future: "Must be today or later.",
  must_be_positive: "The total must be more than €0.",
  invalid: "Invalid.",
};

// "Today" as the business sees it, as YYYY-MM-DD; the same on server and
// client, so the default doesn't cause a hydration mismatch.
function amsterdamDate(offsetDays = 0): string {
  const date = new Date(Date.now() + offsetDays * 86_400_000);
  return new Intl.DateTimeFormat("en-CA", { timeZone: "Europe/Amsterdam" }).format(date);
}

// Phase 23 Decision 6: on-site if they asked for catering, delivery if they
// asked for meal delivery, otherwise pickup.
function defaultFulfilment(services: string[]): FulfilmentType {
  if (services.includes("catering")) return "on_site";
  if (services.includes("meal_delivery")) return "delivery";
  return "pickup";
}

function Label({ htmlFor, children }: { htmlFor: string; children: ReactNode }) {
  return (
    <label htmlFor={htmlFor} className="mb-1.5 block text-sm font-semibold">
      {children}
    </label>
  );
}

function FieldError({ message }: { message: string | undefined }) {
  return message ? <p className="m-0 mt-1 text-sm text-primary-dark">{message}</p> : null;
}

// Phase 23 Decision 8: build, save and send the enquiry's quote. Saving a
// sent or declined quote pulls it back to draft (Decision 3); sharing the
// link by WhatsApp works even when email is off (Decision 14).
export function AdminQuoteEditor({ enquiry }: { enquiry: CateringEnquiryDetail }) {
  const router = useRouter();
  const quote = enquiry.quote;

  const [fulfilment, setFulfilment] = useState<FulfilmentType>(
    quote?.delivery_type ?? defaultFulfilment(enquiry.services_required),
  );
  const [address, setAddress] = useState(quote?.delivery_address ?? enquiry.location);
  const [validUntil, setValidUntil] = useState(() => {
    if (quote) return quote.valid_until;
    const twoWeeks = amsterdamDate(DEFAULT_VALIDITY_DAYS);
    return enquiry.event_date < twoWeeks ? enquiry.event_date : twoWeeks;
  });
  const [notes, setNotes] = useState(quote?.notes ?? "");
  const [lines, setLines] = useState<Line[]>(
    quote?.items.map((item) => ({
      description: item.description,
      quantity: String(item.quantity),
      unitPrice: item.unit_price,
    })) ?? [EMPTY_LINE],
  );
  const [dirty, setDirty] = useState(false);
  const [saving, setSaving] = useState(false);
  const [confirmingSend, setConfirmingSend] = useState(false);
  const [errors, setErrors] = useState<Errors>({});
  const [error, setError] = useState<string | null>(null);
  const [emailOff, setEmailOff] = useState(false);
  const [copied, setCopied] = useState(false);

  function touch() {
    setDirty(true);
    setConfirmingSend(false);
  }

  function updateLine(index: number, patch: Partial<Line>) {
    setLines((current) => current.map((line, i) => (i === index ? { ...line, ...patch } : line)));
    touch();
  }

  const lineCents = lines.map((line) => {
    const cents = toCents(line.unitPrice);
    const quantity = Number.parseInt(line.quantity, 10);
    return cents === null || !Number.isInteger(quantity) ? 0 : cents * quantity;
  });
  const totalCents = lineCents.reduce((sum, cents) => sum + cents, 0);

  if (quote?.status === "accepted") {
    return (
      <div className="flex flex-col gap-4">
        <p className="m-0">
          Accepted{quote.accepted_at ? ` on ${formatDateTime(quote.accepted_at)}` : ""}.{" "}
          {quote.order_id !== null && (
            <Link href={`/orders/${quote.order_id}`} className="font-semibold text-primary underline">
              View order {quote.order_reference ?? `#${quote.order_id}`}
            </Link>
          )}
        </p>
        <p className="m-0 text-sm text-text-muted">
          {FULFILMENT_LABELS[quote.delivery_type]}
          {quote.delivery_address ? `: ${quote.delivery_address}` : ""} · valid until {quote.valid_until}
        </p>
        <ul className="m-0 list-none p-0 text-sm">
          {quote.items.map((item, index) => (
            <li key={index} className="flex justify-between gap-4 border-b border-text/10 py-2">
              <span>
                {item.quantity} × {item.description}
              </span>
              <span className="font-semibold">{formatPrice(item.subtotal)}</span>
            </li>
          ))}
        </ul>
        <p className="m-0 text-right font-semibold">Total: {formatPrice(quote.total)}</p>
      </div>
    );
  }

  function validateLocally(): Errors {
    const found: Errors = {};
    lines.forEach((line, index) => {
      if (toCents(line.unitPrice) === null) found[`items[${index}].unit_price`] = "Enter an amount like 12.50.";
      if (!/^\d+$/.test(line.quantity.trim())) found[`items[${index}].quantity`] = "Whole number.";
    });
    return found;
  }

  async function save() {
    setError(null);
    const local = validateLocally();
    setErrors(local);
    if (Object.keys(local).length > 0) return;

    setSaving(true);
    const response = await fetch(`/api/admin/catering-enquiries/${enquiry.id}/quote`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        delivery_type: fulfilment,
        delivery_address: fulfilment === "pickup" ? null : address,
        notes: notes.trim() === "" ? null : notes,
        valid_until: validUntil,
        items: lines.map((line) => ({
          description: line.description,
          quantity: Number.parseInt(line.quantity, 10),
          // Normalized ("12,5" -> "12.50"); validateLocally already ruled out null.
          unit_price: centsToAmount(toCents(line.unitPrice) ?? 0),
        })),
      }),
    }).catch(() => null);
    setSaving(false);

    if (response?.status === 400) {
      const body: unknown = await response.json().catch(() => null);
      const fields =
        typeof body === "object" && body !== null && Array.isArray((body as Record<string, unknown>).fields)
          ? ((body as { fields: { field: string; message: string }[] }).fields)
          : [];
      setErrors(Object.fromEntries(fields.map((f) => [f.field, MESSAGES[f.message] ?? f.message])));
      return;
    }
    if (response?.status === 409) {
      setError("The customer accepted this quote in the meantime. Refresh to see the order.");
      return;
    }
    if (!response?.ok) {
      setError("Could not save the quote. Please try again.");
      return;
    }

    setErrors({});
    setDirty(false);
    router.refresh();
  }

  async function send() {
    setError(null);
    setSaving(true);
    const response = await fetch(`/api/admin/catering-enquiries/${enquiry.id}/quote/send`, {
      method: "POST",
    }).catch(() => null);
    setSaving(false);
    setConfirmingSend(false);

    if (response?.status === 400) {
      setError("The valid-until date has passed. Change it, save, then send.");
      return;
    }
    if (response?.status === 409) {
      setError("The customer accepted this quote in the meantime. Refresh to see the order.");
      return;
    }
    if (!response?.ok) {
      setError("Could not send the quote. Please try again.");
      return;
    }

    const body = (await response.json()) as SendQuoteResponse;
    setEmailOff(!body.email_queued);
    router.refresh();
  }

  async function copyLink(url: string) {
    try {
      await navigator.clipboard.writeText(url);
      setCopied(true);
    } catch {
      // Clipboard blocked: the link is in the field to select by hand.
      setCopied(false);
    }
  }

  const sharable = quote && ["sent", "expired", "declined"].includes(quote.display_status);
  const whatsAppNumber = toWhatsAppNumber(enquiry.phone);

  return (
    <div className="flex flex-col gap-6">
      <fieldset className="m-0 flex flex-wrap gap-4 border-0 p-0">
        <legend className="mb-1.5 text-sm font-semibold">Fulfilment</legend>
        {FULFILMENT_OPTIONS.map((option) => (
          <label key={option} className="flex items-center gap-2 text-sm">
            <input
              type="radio"
              name="quote-fulfilment"
              value={option}
              checked={fulfilment === option}
              onChange={() => {
                setFulfilment(option);
                touch();
              }}
            />
            {option === "on_site" ? "On-site at the venue" : FULFILMENT_LABELS[option]}
          </label>
        ))}
      </fieldset>

      {fulfilment !== "pickup" && (
        <div>
          <Label htmlFor="quote-address">{fulfilment === "on_site" ? "Venue address" : "Delivery address"}</Label>
          <input
            id="quote-address"
            className="field"
            value={address}
            onChange={(event) => {
              setAddress(event.target.value);
              touch();
            }}
          />
          <FieldError message={errors.delivery_address} />
        </div>
      )}

      <div className="flex flex-col gap-3">
        <p className="m-0 text-sm font-semibold">Lines</p>
        {lines.map((line, index) => (
          <div key={index} className="grid grid-cols-[1fr_5rem_7rem_auto] items-start gap-2">
            <div>
              <label htmlFor={`quote-line-${index}-description`} className="sr-only">
                Description, line {index + 1}
              </label>
              <input
                id={`quote-line-${index}-description`}
                className="field"
                placeholder="Description"
                value={line.description}
                onChange={(event) => updateLine(index, { description: event.target.value })}
              />
              <FieldError message={errors[`items[${index}].description`]} />
            </div>
            <div>
              <label htmlFor={`quote-line-${index}-quantity`} className="sr-only">
                Quantity, line {index + 1}
              </label>
              <input
                id={`quote-line-${index}-quantity`}
                className="field"
                inputMode="numeric"
                placeholder="Qty"
                value={line.quantity}
                onChange={(event) => updateLine(index, { quantity: event.target.value })}
              />
              <FieldError message={errors[`items[${index}].quantity`]} />
            </div>
            <div>
              <label htmlFor={`quote-line-${index}-price`} className="sr-only">
                Unit price in euros, line {index + 1}
              </label>
              <input
                id={`quote-line-${index}-price`}
                className="field"
                inputMode="decimal"
                placeholder="€ each"
                value={line.unitPrice}
                onChange={(event) => updateLine(index, { unitPrice: event.target.value })}
              />
              <FieldError message={errors[`items[${index}].unit_price`]} />
            </div>
            <button
              type="button"
              className="mt-3 text-sm font-semibold text-text-muted underline disabled:opacity-50"
              disabled={lines.length === 1}
              onClick={() => {
                setLines((current) => current.filter((_, i) => i !== index));
                touch();
              }}
            >
              Remove
            </button>
          </div>
        ))}
        <FieldError message={errors.items} />
        <button
          type="button"
          className="self-start text-sm font-semibold text-primary underline"
          onClick={() => {
            setLines((current) => [...current, EMPTY_LINE]);
            touch();
          }}
        >
          + Add line
        </button>
        <p className="m-0 text-right text-lg font-semibold">
          Total: <span className="text-primary">{formatPrice(centsToAmount(totalCents))}</span>
        </p>
        <FieldError message={errors.total} />
        {totalCents >= LARGE_TOTAL_CENTS && <Notice>Large total, please double-check.</Notice>}
      </div>

      <div>
        <Label htmlFor="quote-valid-until">Valid until</Label>
        <input
          id="quote-valid-until"
          type="date"
          className="field max-w-[14rem]"
          value={validUntil}
          onChange={(event) => {
            setValidUntil(event.target.value);
            touch();
          }}
        />
        <FieldError message={errors.valid_until} />
      </div>

      <div>
        <Label htmlFor="quote-notes">Notes for the customer</Label>
        <textarea
          id="quote-notes"
          className="field"
          rows={3}
          maxLength={1000}
          value={notes}
          onChange={(event) => {
            setNotes(event.target.value);
            touch();
          }}
        />
        <FieldError message={errors.notes} />
      </div>

      <div className="flex flex-wrap items-center gap-3">
        <Button type="button" variant="secondary" disabled={saving} onClick={save}>
          {saving ? "Saving…" : "Save draft"}
        </Button>
        {quote &&
          (confirmingSend ? (
            <>
              <Button type="button" disabled={saving} onClick={send}>
                Send quote for {formatPrice(quote.total)} to{" "}
                <span className="normal-case">{enquiry.email}</span>?
              </Button>
              <button
                type="button"
                className="text-sm font-semibold text-text-muted underline"
                disabled={saving}
                onClick={() => setConfirmingSend(false)}
              >
                Not yet
              </button>
            </>
          ) : (
            <Button type="button" disabled={saving || dirty} onClick={() => setConfirmingSend(true)}>
              {quote.status === "sent" ? "Send again" : "Send to customer"}
            </Button>
          ))}
        {dirty && quote && <span className="text-sm text-text-muted">Save first, then send.</span>}
      </div>

      {error && <Notice>{error}</Notice>}
      {emailOff && <Notice>Email is switched off. Share the link with the customer instead.</Notice>}

      {sharable && (
        <section className="flex flex-col gap-3 rounded-card bg-background-soft p-5">
          <h3 className="m-0 text-base font-semibold">Share with the customer</h3>
          <label htmlFor="quote-share-url" className="sr-only">
            Quote link
          </label>
          <input
            id="quote-share-url"
            className="field"
            readOnly
            value={quote.quote_url}
            onFocus={(event) => event.target.select()}
          />
          <div className="flex flex-wrap items-center gap-3">
            <Button type="button" variant="secondary" onClick={() => copyLink(quote.quote_url)}>
              {copied ? "Copied" : "Copy link"}
            </Button>
            {whatsAppNumber && (
              <a
                href={buildWhatsAppLinkTo(
                  whatsAppNumber,
                  quoteShareMessage(enquiry.locale, enquiry.name, quote.id, enquiry.event_date, quote.quote_url),
                )}
                target="_blank"
                rel="noopener noreferrer"
                className={buttonClasses({ variant: "outline", color: "whatsapp" })}
              >
                Send via WhatsApp
              </a>
            )}
          </div>
        </section>
      )}
    </div>
  );
}
