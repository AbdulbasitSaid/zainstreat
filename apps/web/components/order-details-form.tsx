"use client";

import { useRef, useState, type FormEvent } from "react";
import { useTranslations } from "next-intl";
import { AddressCombobox } from "@/components/address-combobox";
import { Button, ButtonAnchor } from "@/components/button";
import { FormPrivacyNote } from "@/components/form-privacy-note";
import { Notice } from "@/components/notice";
import { fetchQuote, type DeliveryInfo, type DeliveryQuote, type QuoteFailure } from "@/lib/delivery";
import { formatPrice } from "@/lib/format";
import { buildWhatsAppLink } from "@/lib/whatsapp";

export type DeliveryType = "pickup" | "delivery";

export interface OrderDetailsValues {
  name: string;
  email: string;
  phone: string;
  deliveryType: DeliveryType;
  // The picked address's label, or what the customer typed while address
  // lookup was down (Phase 37 Decision 17).
  deliveryAddress: string;
  deliveryAddressId: string | null;
  deliveryAddressExtra: string;
  // The fee shown before ordering; the API re-checks it (Decision 12).
  deliveryQuote: DeliveryQuote | null;
  addressLookupDown: boolean;
  // yyyy-mm-dd, only asked for when the cart has rentals (Decision 4).
  rentalDate: string;
  notes: string;
}

type FieldErrors = Partial<Record<keyof OrderDetailsValues, string>>;

const EMPTY_VALUES: OrderDetailsValues = {
  name: "",
  email: "",
  phone: "",
  deliveryType: "pickup",
  deliveryAddress: "",
  deliveryAddressId: null,
  deliveryAddressExtra: "",
  deliveryQuote: null,
  addressLookupDown: false,
  rentalDate: "",
  notes: "",
};

const NOTES_MAX_LENGTH = 500;
const NAME_MIN_LENGTH = 2;
const ADDRESS_MIN_LENGTH = 5;
const ADDRESS_EXTRA_MAX_LENGTH = 120;
const RENTAL_MAX_DAYS_AHEAD = 365;

type QuoteStatus = "idle" | "loading" | "ready" | QuoteFailure;

// yyyy-mm-dd in Dutch time, the API's calendar for rental dates.
function amsterdamDate(daysAhead: number): string {
  const date = new Date(Date.now() + daysAhead * 86_400_000);
  return new Intl.DateTimeFormat("en-CA", { timeZone: "Europe/Amsterdam" }).format(date);
}

function isValidEmail(value: string): boolean {
  return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value);
}

function hasLetter(value: string): boolean {
  return /\p{L}/u.test(value);
}

const ASCENDING_DIGITS = "01234567890123456789";
const DESCENDING_DIGITS = "09876543210987654321";

function isValidPhone(value: string): boolean {
  // Must look like a real phone number: an optional leading "+" with a
  // country code, or a leading trunk "0", then only digits/spaces/hyphens/
  // parentheses — "+" may only appear as the very first character.
  if (!/^(\+[1-9]|0)[0-9\s\-()]*$/.test(value)) return false;

  const digits = value.replace(/\D/g, "");
  if (digits.length < 7 || digits.length > 15) return false;

  // Reject obviously fake numbers: all the same digit, or a straight
  // ascending/descending run (e.g. "1234567890", "0123456789").
  if (/^(\d)\1+$/.test(digits)) return false;
  if (ASCENDING_DIGITS.includes(digits) || DESCENDING_DIGITS.includes(digits)) return false;

  return true;
}

function validate(values: OrderDetailsValues, quoteStatus: QuoteStatus, hasRentals: boolean): FieldErrors {
  const errors: FieldErrors = {};

  const trimmedName = values.name.trim();
  if (!trimmedName) errors.name = "required";
  else if (trimmedName.length < NAME_MIN_LENGTH || !hasLetter(trimmedName)) errors.name = "invalid";

  if (!values.email.trim()) errors.email = "required";
  else if (!isValidEmail(values.email)) errors.email = "invalid";

  const trimmedPhone = values.phone.trim();
  if (!trimmedPhone) errors.phone = "required";
  else if (!isValidPhone(trimmedPhone)) errors.phone = "invalid";

  if (values.deliveryType === "delivery") {
    if (values.addressLookupDown) {
      const trimmedAddress = values.deliveryAddress.trim();
      if (!trimmedAddress) errors.deliveryAddress = "required";
      else if (trimmedAddress.length < ADDRESS_MIN_LENGTH) errors.deliveryAddress = "tooShort";
    } else if (!values.deliveryAddressId) {
      errors.deliveryAddress = "pickAddress";
    } else if (!values.deliveryQuote) {
      errors.deliveryAddress = quoteStatus === "loading" ? "calculating" : "noQuote";
    }
    if (values.deliveryAddressExtra.trim().length > ADDRESS_EXTRA_MAX_LENGTH) errors.deliveryAddressExtra = "tooLong";
  }

  if (hasRentals) {
    if (!values.rentalDate) errors.rentalDate = "required";
    else if (values.rentalDate < amsterdamDate(1)) errors.rentalDate = "tooSoon";
    else if (values.rentalDate > amsterdamDate(RENTAL_MAX_DAYS_AHEAD)) errors.rentalDate = "tooFar";
  }

  if (values.notes.length > NOTES_MAX_LENGTH) errors.notes = "tooLong";

  return errors;
}

const FIELD_LABEL_CLASS = "mb-1.5 block text-sm font-semibold text-text";
const FIELD_ERROR_CLASS = "mt-1 text-sm text-primary";

export function OrderDetailsForm({
  initialValues,
  deliveryInfo,
  hasRentals,
  notice,
  onSubmit,
}: {
  initialValues: OrderDetailsValues | null;
  deliveryInfo: DeliveryInfo | null;
  hasRentals: boolean;
  // Why checkout came back here from the review step, already translated.
  notice: string | null;
  onSubmit: (values: OrderDetailsValues) => void;
}) {
  const t = useTranslations("Order");
  const deliveryEnabled = deliveryInfo?.enabled === true;
  const [values, setValues] = useState<OrderDetailsValues>(() => {
    const start = initialValues ?? EMPTY_VALUES;
    return deliveryEnabled ? start : { ...start, deliveryType: "pickup" };
  });
  const [errors, setErrors] = useState<FieldErrors>({});
  const [quoteStatus, setQuoteStatus] = useState<QuoteStatus>(values.deliveryQuote ? "ready" : "idle");
  // Only the latest pick's quote may land.
  const quoteRequest = useRef(0);

  function update<K extends keyof OrderDetailsValues>(key: K, value: OrderDetailsValues[K]) {
    setValues((prev) => ({ ...prev, [key]: value }));
  }

  function switchToTypedAddress() {
    setValues((prev) => ({ ...prev, addressLookupDown: true, deliveryAddressId: null, deliveryQuote: null }));
    setQuoteStatus("idle");
  }

  async function pickAddress(id: string | null, label: string) {
    const request = ++quoteRequest.current;
    setValues((prev) => ({ ...prev, deliveryAddressId: id, deliveryAddress: label, deliveryQuote: null }));
    if (!id) {
      setQuoteStatus("idle");
      return;
    }
    setQuoteStatus("loading");
    setErrors((prev) => ({ ...prev, deliveryAddress: undefined }));
    const result = await fetchQuote("/api/delivery", id);
    if (request !== quoteRequest.current) return;
    if (result.ok) {
      setValues((prev) => ({ ...prev, deliveryQuote: result.quote }));
      setQuoteStatus("ready");
    } else if (result.reason === "lookup_down") {
      switchToTypedAddress();
    } else {
      setQuoteStatus(result.reason);
    }
  }

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    const nextErrors = validate(values, quoteStatus, hasRentals);
    setErrors(nextErrors);
    if (Object.keys(nextErrors).length === 0) {
      onSubmit(values);
    }
  }

  return (
    <form onSubmit={handleSubmit} noValidate className="mx-auto flex max-w-[640px] flex-col gap-5 py-8">
      {notice && <Notice>{notice}</Notice>}
      <div>
        <label htmlFor="order-name" className={FIELD_LABEL_CLASS}>{t("nameLabel")}</label>
        <input
          id="order-name"
          className="field"
          value={values.name}
          onChange={(e) => update("name", e.target.value)}
          aria-invalid={!!errors.name}
          aria-describedby={errors.name ? "order-name-error" : undefined}
        />
        {errors.name && (
          <p id="order-name-error" className={FIELD_ERROR_CLASS}>
            {t(`errors.${errors.name}`, { field: t("nameLabel") })}
          </p>
        )}
      </div>

      <div>
        <label htmlFor="order-email" className={FIELD_LABEL_CLASS}>{t("emailLabel")}</label>
        <input
          id="order-email"
          type="email"
          className="field"
          value={values.email}
          onChange={(e) => update("email", e.target.value)}
          aria-invalid={!!errors.email}
          aria-describedby={errors.email ? "order-email-error" : undefined}
        />
        {errors.email && (
          <p id="order-email-error" className={FIELD_ERROR_CLASS}>
            {t(`errors.${errors.email}`, { field: t("emailLabel") })}
          </p>
        )}
      </div>

      <div>
        <label htmlFor="order-phone" className={FIELD_LABEL_CLASS}>{t("phoneLabel")}</label>
        <input
          id="order-phone"
          type="tel"
          className="field"
          value={values.phone}
          onChange={(e) => update("phone", e.target.value)}
          aria-invalid={!!errors.phone}
          aria-describedby={errors.phone ? "order-phone-error" : undefined}
        />
        {errors.phone && (
          <p id="order-phone-error" className={FIELD_ERROR_CLASS}>
            {t(`errors.${errors.phone}`, { field: t("phoneLabel") })}
          </p>
        )}
      </div>

      <fieldset className="flex flex-col gap-2">
        <legend className={FIELD_LABEL_CLASS}>{t("deliveryTypeLabel")}</legend>
        <label className="flex items-center gap-2">
          <input
            type="radio"
            name="deliveryType"
            checked={values.deliveryType === "pickup"}
            onChange={() => update("deliveryType", "pickup")}
          />
          {t("pickupOption")}
        </label>
        {deliveryEnabled ? (
          <label className="flex items-center gap-2">
            <input
              type="radio"
              name="deliveryType"
              checked={values.deliveryType === "delivery"}
              onChange={() => update("deliveryType", "delivery")}
            />
            {t("deliveryOption")}
            {deliveryInfo && (
              <span className="text-sm text-text-muted">
                {t("deliveryRateHint", { rate: formatPrice(deliveryInfo.rate_per_km) })}
              </span>
            )}
          </label>
        ) : (
          <p className="m-0 text-sm text-text-muted">{t("deliveryOff")}</p>
        )}
      </fieldset>

      {values.deliveryType === "delivery" && (
        <div className="flex flex-col gap-4">
          {values.addressLookupDown ? (
            <div>
              <Notice>{t("lookupDown")}</Notice>
              <label htmlFor="order-address" className={`${FIELD_LABEL_CLASS} mt-3`}>{t("addressLabel")}</label>
              <input
                id="order-address"
                className="field"
                autoComplete="street-address"
                value={values.deliveryAddress}
                onChange={(e) => update("deliveryAddress", e.target.value)}
                aria-invalid={!!errors.deliveryAddress}
                aria-describedby={errors.deliveryAddress ? "order-address-error" : undefined}
              />
            </div>
          ) : (
            <AddressCombobox
              id="order-address"
              apiBase="/api/delivery"
              initialQuery={values.deliveryAddress}
              labels={{
                label: t("addressLabel"),
                hint: t("addressSearchHint"),
                searching: t("searchingAddresses"),
                noMatches: t("noMatches"),
              }}
              invalid={!!errors.deliveryAddress}
              describedBy={["order-address-estimate", errors.deliveryAddress ? "order-address-error" : ""]
                .filter(Boolean)
                .join(" ")}
              onSelect={(suggestion) => void pickAddress(suggestion?.id ?? null, suggestion?.label ?? "")}
              onLookupDown={switchToTypedAddress}
            />
          )}
          {errors.deliveryAddress && (
            <p id="order-address-error" className={`${FIELD_ERROR_CLASS} -mt-3`}>
              {t(`errors.${errors.deliveryAddress}`, { field: t("addressLabel") })}
            </p>
          )}
          {/* Announced as it changes (Decision 11). */}
          <div id="order-address-estimate" aria-live="polite">
            {!values.addressLookupDown && quoteStatus === "loading" && (
              <p className="m-0 text-sm text-text-muted">{t("calculating")}</p>
            )}
            {!values.addressLookupDown && quoteStatus === "ready" && values.deliveryQuote && (
              <>
                <p className="m-0 font-semibold text-primary">
                  {t("deliveryEstimate", {
                    km: values.deliveryQuote.distance_km,
                    fee: formatPrice(values.deliveryQuote.fee),
                  })}
                </p>
                {/* Mapbox Product Terms §1.4.1: attribution wherever its routing is used. */}
                {values.deliveryQuote.estimated_by === "route" && (
                  <p className="m-0 mt-1 text-xs text-text-muted">
                    {t("routeAttribution")}{" "}
                    <a href="https://www.mapbox.com/about/maps" target="_blank" rel="noopener noreferrer" className="underline">
                      © Mapbox
                    </a>{" "}
                    <a href="https://www.openstreetmap.org/about" target="_blank" rel="noopener noreferrer" className="underline">
                      © OpenStreetMap
                    </a>
                  </p>
                )}
              </>
            )}
            {(quoteStatus === "out_of_range" || quoteStatus === "unavailable" || quoteStatus === "error" || quoteStatus === "not_found") && (
              <Notice>
                {t(quoteStatus === "out_of_range" ? "outOfRange" : "deliveryUnavailable")}
                <ButtonAnchor
                  href={buildWhatsAppLink(t("deliveryWhatsappMessage"))}
                  target="_blank"
                  rel="noopener noreferrer"
                  color="whatsapp"
                  variant="outline"
                  className="mt-3 flex w-fit"
                >
                  {t("askOnWhatsapp")}
                </ButtonAnchor>
              </Notice>
            )}
          </div>
          <div>
            <label htmlFor="order-address-extra" className={FIELD_LABEL_CLASS}>{t("addressExtraLabel")}</label>
            <input
              id="order-address-extra"
              className="field"
              maxLength={ADDRESS_EXTRA_MAX_LENGTH}
              value={values.deliveryAddressExtra}
              onChange={(e) => update("deliveryAddressExtra", e.target.value)}
              aria-invalid={!!errors.deliveryAddressExtra}
              aria-describedby={errors.deliveryAddressExtra ? "order-address-extra-error" : undefined}
            />
            {errors.deliveryAddressExtra && (
              <p id="order-address-extra-error" className={FIELD_ERROR_CLASS}>
                {t(`errors.${errors.deliveryAddressExtra}`, { field: t("addressExtraLabel") })}
              </p>
            )}
          </div>
        </div>
      )}

      {hasRentals && (
        <div>
          <label htmlFor="order-rental-date" className={FIELD_LABEL_CLASS}>{t("rentalDateLabel")}</label>
          <input
            id="order-rental-date"
            type="date"
            className="field"
            min={amsterdamDate(1)}
            max={amsterdamDate(RENTAL_MAX_DAYS_AHEAD)}
            value={values.rentalDate}
            onChange={(e) => update("rentalDate", e.target.value)}
            aria-invalid={!!errors.rentalDate}
            aria-describedby={["order-rental-date-hint", errors.rentalDate ? "order-rental-date-error" : ""]
              .filter(Boolean)
              .join(" ")}
          />
          <p id="order-rental-date-hint" className="mt-1 text-sm text-text-muted">{t("rentalDateHint")}</p>
          {errors.rentalDate && (
            <p id="order-rental-date-error" className={FIELD_ERROR_CLASS}>
              {t(`errors.${errors.rentalDate}`, { field: t("rentalDateLabel") })}
            </p>
          )}
        </div>
      )}

      <div>
        <label htmlFor="order-notes" className={FIELD_LABEL_CLASS}>{t("notesLabel")}</label>
        <textarea
          id="order-notes"
          className="field"
          rows={3}
          maxLength={NOTES_MAX_LENGTH}
          value={values.notes}
          onChange={(e) => update("notes", e.target.value)}
          aria-invalid={!!errors.notes}
          aria-describedby={errors.notes ? "order-notes-error" : undefined}
        />
        {errors.notes && (
          <p id="order-notes-error" className={FIELD_ERROR_CLASS}>
            {t(`errors.${errors.notes}`, { field: t("notesLabel") })}
          </p>
        )}
      </div>

      <Button type="submit" className="self-start">{t("continueToReview")}</Button>
      <FormPrivacyNote form="order" />
    </form>
  );
}
