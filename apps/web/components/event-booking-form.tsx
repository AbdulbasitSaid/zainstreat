"use client";

import { useState, type FormEvent } from "react";
import { useLocale, useTranslations } from "next-intl";
import { useRouter } from "@/i18n/navigation";
import { Button } from "@/components/button";
import { FormPrivacyNote } from "@/components/form-privacy-note";
import { Notice } from "@/components/notice";
import { QuantityStepper } from "@/components/quantity-stepper";
import { createBooking, type EventPlate } from "@/lib/events";
import { formatPrice } from "@/lib/format";

const NO_MEAL = "none";
const NOTES_MAX_LENGTH = 500;
const FIELD_LABEL_CLASS = "mb-1.5 block text-sm font-semibold text-text";
const FIELD_ERROR_CLASS = "mt-1 text-sm text-primary";

type ContactField = "customer_name" | "customer_email" | "customer_phone" | "notes";
type Errors = Partial<Record<ContactField | "accept_terms" | "guests", string>>;

// A guest's plate (or NO_MEAL) and their pick per option group (Phase 32).
interface GuestState {
  plate: string;
  picks: Record<string, string>;
}

function isValidEmail(value: string): boolean {
  return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value);
}

// The API's field errors, mapped onto this form's message keys.
function errorKey(message: string): string {
  switch (message) {
    case "required":
    case "invalid":
    case "meals_closed":
    case "unavailable":
    case "too_many":
    case "option_required":
      return message;
    case "too_long":
      return "tooLong";
    default:
      return "invalid";
  }
}

/**
 * One form for the RSVP and the meals (Phase 31 Decision 17): contact
 * details, how many people, and a plate (or no meal) per person, with the
 * plate's options (Phase 32 Decision 8). A party that doesn't fit gets the
 * seats left and a "Join the waitlist" retry.
 */
export function EventBookingForm({
  slug,
  plates,
  maxPartySize,
  seatsLeft,
  mealsOpen,
  terms,
}: {
  slug: string;
  plates: EventPlate[];
  maxPartySize: number;
  seatsLeft: number;
  mealsOpen: boolean;
  terms: string;
}) {
  const t = useTranslations("EventBooking");
  const locale = useLocale();
  const router = useRouter();
  const defaultChoice = mealsOpen && plates[0] ? String(plates[0].id) : NO_MEAL;

  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [phone, setPhone] = useState("");
  const [notes, setNotes] = useState("");
  const [website, setWebsite] = useState("");
  const [guests, setGuests] = useState<GuestState[]>([{ plate: defaultChoice, picks: {} }]);
  // `${guest}:${group}` for each option left unpicked on submit.
  const [missing, setMissing] = useState<Set<string>>(new Set());
  const [acceptTerms, setAcceptTerms] = useState(false);
  const [errors, setErrors] = useState<Errors>({});
  const [submitting, setSubmitting] = useState(false);
  const [seatsShort, setSeatsShort] = useState<number | null>(null);
  const [failure, setFailure] = useState<"closed" | "unknown" | null>(null);
  // Starting full means the form is a waitlist form from the start.
  const waitlistOnly = seatsLeft === 0;

  const plateOf = (choice: string) => plates.find((plate) => String(plate.id) === choice);
  const total = guests.reduce((sum, guest) => sum + Number(plateOf(guest.plate)?.price ?? 0), 0);
  const wantsMeals = guests.some((guest) => guest.plate !== NO_MEAL);

  function resize(size: number) {
    const next = Math.max(1, Math.min(maxPartySize, size));
    setGuests((current) =>
      next > current.length
        ? [...current, ...Array.from({ length: next - current.length }, () => ({ plate: defaultChoice, picks: {} }))]
        : current.slice(0, next),
    );
    setSeatsShort(null);
  }

  // Changing plate clears the picks: the groups belong to the plate.
  function choosePlate(index: number, plate: string) {
    setGuests((current) => current.map((guest, i) => (i === index ? { plate, picks: {} } : guest)));
  }

  function pick(index: number, group: string, choice: string) {
    setGuests((current) =>
      current.map((guest, i) => (i === index ? { ...guest, picks: { ...guest.picks, [group]: choice } } : guest)),
    );
    setMissing((current) => {
      const next = new Set(current);
      next.delete(`${index}:${group}`);
      return next;
    });
  }

  function unpicked(): Set<string> {
    const keys = new Set<string>();
    guests.forEach((guest, index) => {
      for (const group of plateOf(guest.plate)?.options ?? []) {
        if (!guest.picks[group.key]) keys.add(`${index}:${group.key}`);
      }
    });
    return keys;
  }

  function validate(): Errors {
    const next: Errors = {};
    if (name.trim().length < 2) next.customer_name = name.trim() ? "invalid" : "required";
    if (!email.trim()) next.customer_email = "required";
    else if (!isValidEmail(email.trim())) next.customer_email = "invalid";
    if (!phone.trim()) next.customer_phone = "required";
    if (notes.length > NOTES_MAX_LENGTH) next.notes = "tooLong";
    if (wantsMeals && !acceptTerms) next.accept_terms = "required";
    return next;
  }

  async function submit(joinWaitlist: boolean) {
    const nextErrors = validate();
    const nextMissing = unpicked();
    setErrors(nextErrors);
    setMissing(nextMissing);
    if (Object.keys(nextErrors).length > 0 || nextMissing.size > 0) return;

    setSubmitting(true);
    setFailure(null);
    const result = await createBooking(slug, {
      customer_name: name.trim(),
      customer_email: email.trim(),
      customer_phone: phone.trim(),
      guests: guests.map((guest) =>
        guest.plate === NO_MEAL ? { plate_id: null, choices: {} } : { plate_id: Number(guest.plate), choices: guest.picks },
      ),
      notes: notes.trim() ? notes.trim() : null,
      locale,
      accept_terms: acceptTerms,
      join_waitlist: joinWaitlist,
      website,
    });
    setSubmitting(false);

    if (result.ok) {
      router.push(`/events/booking/${result.token}`);
      return;
    }
    if (result.kind === "not_enough_seats") {
      setSeatsShort(result.seatsLeft);
      return;
    }
    if (result.kind === "validation_error") {
      const mapped: Errors = {};
      for (const field of result.fields) {
        const key = field.field.startsWith("guests") ? "guests" : field.field;
        if (key === "customer_name" || key === "customer_email" || key === "customer_phone" || key === "notes" || key === "accept_terms" || key === "guests") {
          mapped[key] = errorKey(field.message);
        }
      }
      setErrors(mapped);
      return;
    }
    setFailure(result.kind === "booking_closed" ? "closed" : "unknown");
  }

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    void submit(waitlistOnly);
  }

  const contactFields: { key: ContactField; type: string; autoComplete: string }[] = [
    { key: "customer_name", type: "text", autoComplete: "name" },
    { key: "customer_email", type: "email", autoComplete: "email" },
    { key: "customer_phone", type: "tel", autoComplete: "tel" },
  ];
  const values: Record<ContactField, string> = { customer_name: name, customer_email: email, customer_phone: phone, notes };
  const setters: Record<ContactField, (value: string) => void> = {
    customer_name: setName,
    customer_email: setEmail,
    customer_phone: setPhone,
    notes: setNotes,
  };

  return (
    <form onSubmit={handleSubmit} noValidate className="flex flex-col gap-5">
      {waitlistOnly && <Notice>{t("fullNotice")}</Notice>}

      {contactFields.map(({ key, type, autoComplete }) => (
        <div key={key}>
          <label htmlFor={`booking-${key}`} className={FIELD_LABEL_CLASS}>
            {t(`labels.${key}`)}
          </label>
          <input
            id={`booking-${key}`}
            type={type}
            autoComplete={autoComplete}
            className="field"
            value={values[key]}
            onChange={(event) => setters[key](event.target.value)}
            aria-invalid={!!errors[key]}
            aria-describedby={errors[key] ? `booking-${key}-error` : undefined}
          />
          {errors[key] && (
            <p id={`booking-${key}-error`} className={FIELD_ERROR_CLASS}>
              {t(`errors.${errors[key]}`, { field: t(`labels.${key}`) })}
            </p>
          )}
        </div>
      ))}

      <div className="flex flex-wrap items-center justify-between gap-3 rounded-card bg-background-soft px-5 py-4">
        <div>
          <p id="booking-party-label" className="m-0 font-semibold">{t("partyLabel")}</p>
          <p className="m-0 text-sm text-text-muted">{t("partyHint", { max: maxPartySize })}</p>
        </div>
        <div role="group" aria-labelledby="booking-party-label">
          <QuantityStepper
            quantity={guests.length}
            onDecrement={() => resize(guests.length - 1)}
            onIncrement={() => resize(guests.length + 1)}
            decrementLabel={t("fewer")}
            incrementLabel={t("more")}
          />
        </div>
      </div>

      {!mealsOpen && plates.length > 0 && <Notice>{t("mealsClosed")}</Notice>}

      <ol className="m-0 flex list-none flex-col gap-4 p-0">
        {guests.map((guest, index) => (
          <li key={index}>
            <fieldset className="m-0 flex flex-col gap-2 rounded-card border border-text/10 p-4">
              <legend className="px-1 text-sm font-semibold">
                {index === 0 ? t("guestYou") : t("guestN", { n: index + 1 })}
              </legend>
              {plates.map((plate) => (
                <div key={plate.id} className={`rounded-xl ${guest.plate === String(plate.id) ? "bg-background-soft" : ""}`}>
                  <label
                    className={`flex items-center justify-between gap-3 px-3 py-2 ${mealsOpen ? "cursor-pointer" : "opacity-50"}`}
                  >
                    <span className="flex items-center gap-3">
                      <input
                        type="radio"
                        name={`guest-${index}`}
                        value={plate.id}
                        checked={guest.plate === String(plate.id)}
                        disabled={!mealsOpen}
                        onChange={() => choosePlate(index, String(plate.id))}
                        className="accent-primary"
                      />
                      {plate.name}
                    </span>
                    <span className="font-semibold text-primary">{formatPrice(plate.price)}</span>
                  </label>
                  {guest.plate === String(plate.id) &&
                    plate.options.map((group) => {
                      const errorId = `guest-${index}-${group.key}-error`;
                      const isMissing = missing.has(`${index}:${group.key}`);
                      return (
                        <fieldset
                          key={group.key}
                          aria-describedby={isMissing ? errorId : undefined}
                          className="m-0 flex flex-col gap-2 border-0 px-3 pb-3 sm:pl-10"
                        >
                          <legend className="mb-2 p-0 text-sm text-text-muted">
                            {t("chooseOption", { group: group.label.toLowerCase() })}
                          </legend>
                          <div className="flex flex-wrap gap-2">
                            {group.choices.map((choice) => (
                              <label key={choice.key} className="cursor-pointer">
                                <input
                                  type="radio"
                                  name={`guest-${index}-${group.key}`}
                                  value={choice.key}
                                  checked={guest.picks[group.key] === choice.key}
                                  onChange={() => pick(index, group.key, choice.key)}
                                  aria-describedby={isMissing ? errorId : undefined}
                                  className="peer sr-only"
                                />
                                <span className="inline-block rounded-full border border-text/15 bg-background px-3.5 py-1.5 text-sm transition-colors peer-checked:border-primary peer-checked:bg-primary peer-checked:text-white peer-focus-visible:outline-2 peer-focus-visible:outline-offset-2 peer-focus-visible:outline-primary hover:border-primary/60">
                                  {choice.label}
                                </span>
                              </label>
                            ))}
                          </div>
                          {isMissing && (
                            <p id={errorId} className="m-0 text-sm text-primary">
                              {t("errors.option_required", { group: group.label.toLowerCase() })}
                            </p>
                          )}
                        </fieldset>
                      );
                    })}
                </div>
              ))}
              <label className={`flex cursor-pointer items-center justify-between gap-3 rounded-xl px-3 py-2 ${guest.plate === NO_MEAL ? "bg-background-soft" : ""}`}>
                <span className="flex items-center gap-3">
                  <input
                    type="radio"
                    name={`guest-${index}`}
                    value={NO_MEAL}
                    checked={guest.plate === NO_MEAL}
                    onChange={() => choosePlate(index, NO_MEAL)}
                    className="accent-primary"
                  />
                  {t("noMeal")}
                </span>
                <span className="text-sm text-text-muted">{t("noMealHint")}</span>
              </label>
            </fieldset>
          </li>
        ))}
      </ol>
      {errors.guests && (
        <p className={FIELD_ERROR_CLASS}>
          {t(`errors.${errors.guests}`, { field: t("partyLabel"), group: t("optionFallback") })}
        </p>
      )}

      <div>
        <label htmlFor="booking-notes" className={FIELD_LABEL_CLASS}>
          {t("labels.notes")}
        </label>
        <textarea
          id="booking-notes"
          rows={3}
          className="field"
          placeholder={t("notesPlaceholder")}
          value={notes}
          onChange={(event) => setNotes(event.target.value)}
          aria-invalid={!!errors.notes}
          aria-describedby={errors.notes ? "booking-notes-error" : undefined}
        />
        {errors.notes && (
          <p id="booking-notes-error" className={FIELD_ERROR_CLASS}>
            {t(`errors.${errors.notes}`, { field: t("labels.notes") })}
          </p>
        )}
      </div>

      {/* Honeypot (Phase 15): off-screen, never filled by a person. */}
      <div aria-hidden="true" className="absolute -left-[9999px] h-px w-px overflow-hidden">
        <label htmlFor="booking-website">Website</label>
        <input id="booking-website" tabIndex={-1} autoComplete="off" value={website} onChange={(event) => setWebsite(event.target.value)} />
      </div>

      <div className="flex items-baseline justify-between border-t border-text/10 pt-4 text-lg">
        <span className="font-semibold">{t("total")}</span>
        <span className="font-semibold text-primary">{wantsMeals ? formatPrice(total.toFixed(2)) : t("totalFree")}</span>
      </div>

      {wantsMeals && (
        <div className="flex flex-col gap-3">
          {terms && (
            <details className="rounded-card bg-cream px-5 py-4">
              <summary className="cursor-pointer font-semibold">{t("termsHeading")}</summary>
              <p className="m-0 mt-3 text-sm whitespace-pre-line text-text-muted">{terms}</p>
            </details>
          )}
          <label className="flex items-start gap-3">
            <input
              type="checkbox"
              checked={acceptTerms}
              onChange={(event) => setAcceptTerms(event.target.checked)}
              aria-invalid={!!errors.accept_terms}
              aria-describedby={errors.accept_terms ? "booking-terms-error" : undefined}
              className="mt-1 accent-primary"
            />
            <span>{t("acceptTerms")}</span>
          </label>
          {errors.accept_terms && (
            <p id="booking-terms-error" className={FIELD_ERROR_CLASS}>
              {t("errors.acceptTerms")}
            </p>
          )}
          <p className="m-0 text-sm text-text-muted">{t("paymentNote")}</p>
        </div>
      )}

      {seatsShort !== null ? (
        <div role="alert" className="flex flex-col gap-3 rounded-card bg-accent-light px-5 py-4">
          <p className="m-0">{t("notEnoughSeats", { count: seatsShort, party: guests.length })}</p>
          <div className="flex flex-wrap gap-3">
            <Button type="button" onClick={() => void submit(true)} disabled={submitting}>
              {t("joinWaitlist")}
            </Button>
            {seatsShort > 0 && (
              <Button type="button" variant="outline" onClick={() => resize(seatsShort)}>
                {t("changeParty", { count: seatsShort })}
              </Button>
            )}
          </div>
        </div>
      ) : (
        <Button type="submit" disabled={submitting} className="self-stretch sm:self-start">
          {submitting ? t("submitting") : waitlistOnly ? t("joinWaitlist") : t("submit")}
        </Button>
      )}
      <FormPrivacyNote form="booking" />

      {failure && (
        <p role="alert" className={FIELD_ERROR_CLASS}>
          {failure === "closed" ? t("closed") : t("unknownError")}
        </p>
      )}
    </form>
  );
}
