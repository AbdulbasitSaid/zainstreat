"use client";

import { useState, type FormEvent } from "react";
import { useTranslations } from "next-intl";
import { Button } from "@/components/button";

export type DeliveryType = "pickup" | "delivery";

export interface OrderDetailsValues {
  name: string;
  email: string;
  phone: string;
  deliveryType: DeliveryType;
  deliveryAddress: string;
  notes: string;
}

type FieldErrors = Partial<Record<keyof OrderDetailsValues, string>>;

const EMPTY_VALUES: OrderDetailsValues = {
  name: "",
  email: "",
  phone: "",
  deliveryType: "pickup",
  deliveryAddress: "",
  notes: "",
};

const NOTES_MAX_LENGTH = 500;
const NAME_MIN_LENGTH = 2;
const ADDRESS_MIN_LENGTH = 5;

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

function validate(values: OrderDetailsValues): FieldErrors {
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
    const trimmedAddress = values.deliveryAddress.trim();
    if (!trimmedAddress) errors.deliveryAddress = "required";
    else if (trimmedAddress.length < ADDRESS_MIN_LENGTH) errors.deliveryAddress = "tooShort";
  }

  if (values.notes.length > NOTES_MAX_LENGTH) errors.notes = "tooLong";

  return errors;
}

const FIELD_LABEL_CLASS = "mb-1.5 block text-sm font-semibold text-text";
const FIELD_ERROR_CLASS = "mt-1 text-sm text-primary";

export function OrderDetailsForm({
  initialValues,
  onSubmit,
}: {
  initialValues: OrderDetailsValues | null;
  onSubmit: (values: OrderDetailsValues) => void;
}) {
  const t = useTranslations("Order");
  const [values, setValues] = useState<OrderDetailsValues>(initialValues ?? EMPTY_VALUES);
  const [errors, setErrors] = useState<FieldErrors>({});

  function update<K extends keyof OrderDetailsValues>(key: K, value: OrderDetailsValues[K]) {
    setValues((prev) => ({ ...prev, [key]: value }));
  }

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    const nextErrors = validate(values);
    setErrors(nextErrors);
    if (Object.keys(nextErrors).length === 0) {
      onSubmit(values);
    }
  }

  return (
    <form onSubmit={handleSubmit} noValidate className="mx-auto flex max-w-[640px] flex-col gap-5 py-8">
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
        <label className="flex items-center gap-2">
          <input
            type="radio"
            name="deliveryType"
            checked={values.deliveryType === "delivery"}
            onChange={() => update("deliveryType", "delivery")}
          />
          {t("deliveryOption")}
        </label>
      </fieldset>

      {values.deliveryType === "delivery" && (
        <div>
          <label htmlFor="order-address" className={FIELD_LABEL_CLASS}>{t("addressLabel")}</label>
          <input
            id="order-address"
            className="field"
            value={values.deliveryAddress}
            onChange={(e) => update("deliveryAddress", e.target.value)}
            aria-invalid={!!errors.deliveryAddress}
            aria-describedby={errors.deliveryAddress ? "order-address-error" : undefined}
          />
          {errors.deliveryAddress && (
            <p id="order-address-error" className={FIELD_ERROR_CLASS}>
              {t(`errors.${errors.deliveryAddress}`, { field: t("addressLabel") })}
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
    </form>
  );
}
