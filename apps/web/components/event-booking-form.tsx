"use client";

import { useState, type FormEvent } from "react";
import { useLocale, useTranslations } from "next-intl";
import { useRouter } from "@/i18n/navigation";
import { Button } from "@/components/button";
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
      return message;
    case "too_long":
      return "tooLong";
    default:
      return "invalid";
  }
}

/**
 * One form for the RSVP and the meals (Phase 31 Decision 17): contact
 * details, how many people, and a plate (or no meal) per person. A party
 * that doesn't fit gets the seats left and a "Join the waitlist" retry.
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
  const [choices, setChoices] = useState<string[]>([defaultChoice]);
  const [acceptTerms, setAcceptTerms] = useState(false);
  const [errors, setErrors] = useState<Errors>({});
  const [submitting, setSubmitting] = useState(false);
  const [seatsShort, setSeatsShort] = useState<number | null>(null);
  const [failure, setFailure] = useState<"closed" | "unknown" | null>(null);
  // Starting full means the form is a waitlist form from the start.
  const waitlistOnly = seatsLeft === 0;

  const priceOf = (choice: string) => plates.find((plate) => String(plate.id) === choice)?.price ?? null;
  const total = choices.reduce((sum, choice) => sum + Number(priceOf(choice) ?? 0), 0);
  const wantsMeals = choices.some((choice) => choice !== NO_MEAL);

  function resize(size: number) {
    const next = Math.max(1, Math.min(maxPartySize, size));
    setChoices((current) =>
      next > current.length
        ? [...current, ...Array.from({ length: next - current.length }, () => defaultChoice)]
        : current.slice(0, next),
    );
    setSeatsShort(null);
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
    setErrors(nextErrors);
    if (Object.keys(nextErrors).length > 0) return;

    setSubmitting(true);
    setFailure(null);
    const result = await createBooking(slug, {
      customer_name: name.trim(),
      customer_email: email.trim(),
      customer_phone: phone.trim(),
      guests: choices.map((choice) => (choice === NO_MEAL ? null : Number(choice))),
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
            quantity={choices.length}
            onDecrement={() => resize(choices.length - 1)}
            onIncrement={() => resize(choices.length + 1)}
            decrementLabel={t("fewer")}
            incrementLabel={t("more")}
          />
        </div>
      </div>

      {!mealsOpen && plates.length > 0 && <Notice>{t("mealsClosed")}</Notice>}

      <ol className="m-0 flex list-none flex-col gap-4 p-0">
        {choices.map((choice, index) => (
          <li key={index}>
            <fieldset className="m-0 flex flex-col gap-2 rounded-card border border-text/10 p-4">
              <legend className="px-1 text-sm font-semibold">
                {index === 0 ? t("guestYou") : t("guestN", { n: index + 1 })}
              </legend>
              {plates.map((plate) => (
                <label
                  key={plate.id}
                  className={`flex items-center justify-between gap-3 rounded-xl px-3 py-2 ${
                    choice === String(plate.id) ? "bg-background-soft" : ""
                  } ${mealsOpen ? "cursor-pointer" : "opacity-50"}`}
                >
                  <span className="flex items-center gap-3">
                    <input
                      type="radio"
                      name={`guest-${index}`}
                      value={plate.id}
                      checked={choice === String(plate.id)}
                      disabled={!mealsOpen}
                      onChange={() => setChoices((current) => current.map((c, i) => (i === index ? String(plate.id) : c)))}
                      className="accent-primary"
                    />
                    {plate.name}
                  </span>
                  <span className="font-semibold text-primary">{formatPrice(plate.price)}</span>
                </label>
              ))}
              <label className={`flex cursor-pointer items-center justify-between gap-3 rounded-xl px-3 py-2 ${choice === NO_MEAL ? "bg-background-soft" : ""}`}>
                <span className="flex items-center gap-3">
                  <input
                    type="radio"
                    name={`guest-${index}`}
                    value={NO_MEAL}
                    checked={choice === NO_MEAL}
                    onChange={() => setChoices((current) => current.map((c, i) => (i === index ? NO_MEAL : c)))}
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
      {errors.guests && <p className={FIELD_ERROR_CLASS}>{t(`errors.${errors.guests}`, { field: t("partyLabel") })}</p>}

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
          <p className="m-0">{t("notEnoughSeats", { count: seatsShort, party: choices.length })}</p>
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

      {failure && (
        <p role="alert" className={FIELD_ERROR_CLASS}>
          {failure === "closed" ? t("closed") : t("unknownError")}
        </p>
      )}
    </form>
  );
}
