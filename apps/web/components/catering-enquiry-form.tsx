"use client";

import { useState, type FormEvent } from "react";
import { useTranslations } from "next-intl";
import { Button } from "@/components/button";
import { Notice } from "@/components/notice";
import { submitCateringEnquiry, type CreateCateringEnquiryRequest } from "@/lib/catering";

const EVENT_TYPES = ["wedding", "birthday", "corporate", "family_gathering", "outdoor_event", "other"] as const;
const SERVICES = ["catering", "meal_delivery", "event_rental", "other"] as const;

interface Values {
  name: string;
  phone: string;
  email: string;
  eventType: string;
  eventDate: string;
  guestCount: string;
  location: string;
  services: string[];
  message: string;
  website: string;
}

type FieldErrors = Partial<Record<keyof Values, string>>;

const EMPTY_VALUES: Values = {
  name: "",
  phone: "",
  email: "",
  eventType: "",
  eventDate: "",
  guestCount: "",
  location: "",
  services: [],
  message: "",
  website: "",
};

const LOCATION_MIN_LENGTH = 5;
const MESSAGE_MAX_LENGTH = 1000;

function isValidEmail(value: string): boolean {
  return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value);
}

const ASCENDING_DIGITS = "01234567890123456789";
const DESCENDING_DIGITS = "09876543210987654321";

function isValidPhone(value: string): boolean {
  if (!/^(\+[1-9]|0)[0-9\s\-()]*$/.test(value)) return false;

  const digits = value.replace(/\D/g, "");
  if (digits.length < 7 || digits.length > 15) return false;

  if (/^(\d)\1+$/.test(digits)) return false;
  if (ASCENDING_DIGITS.includes(digits) || DESCENDING_DIGITS.includes(digits)) return false;

  return true;
}

function todayIsoDate(): string {
  return new Date().toISOString().slice(0, 10);
}

function validate(values: Values): FieldErrors {
  const errors: FieldErrors = {};

  const trimmedName = values.name.trim();
  if (!trimmedName) errors.name = "required";

  const trimmedPhone = values.phone.trim();
  if (!trimmedPhone) errors.phone = "required";
  else if (!isValidPhone(trimmedPhone)) errors.phone = "invalid";

  if (!values.email.trim()) errors.email = "required";
  else if (!isValidEmail(values.email)) errors.email = "invalid";

  if (!(EVENT_TYPES as readonly string[]).includes(values.eventType)) errors.eventType = "required";

  if (!values.eventDate) errors.eventDate = "required";
  else if (values.eventDate < todayIsoDate()) errors.eventDate = "mustBeFuture";

  const guestCount = Number.parseInt(values.guestCount, 10);
  if (!values.guestCount.trim() || !Number.isFinite(guestCount) || guestCount <= 0) {
    errors.guestCount = "mustBePositive";
  }

  const trimmedLocation = values.location.trim();
  if (!trimmedLocation) errors.location = "required";
  else if (trimmedLocation.length < LOCATION_MIN_LENGTH) errors.location = "tooShort";

  if (values.services.length === 0) errors.services = "required";

  if (values.message.length > MESSAGE_MAX_LENGTH) errors.message = "tooLong";

  return errors;
}

const SERVER_MESSAGE_TO_ERROR_KEY: Record<string, string> = {
  required: "required",
  invalid: "invalid",
  too_short: "tooShort",
  too_long: "tooLong",
  must_be_future: "mustBeFuture",
  must_be_positive: "mustBePositive",
};

const SERVER_FIELD_TO_LOCAL: Record<string, keyof Values> = {
  name: "name",
  phone: "phone",
  email: "email",
  event_type: "eventType",
  event_date: "eventDate",
  guest_count: "guestCount",
  location: "location",
  services_required: "services",
  message: "message",
};

const FIELD_LABEL_CLASS = "mb-1.5 block text-sm font-semibold text-text";
const FIELD_ERROR_CLASS = "mt-1 text-sm text-primary";

export function CateringEnquiryForm() {
  const t = useTranslations("ContactPage");
  const [values, setValues] = useState<Values>(EMPTY_VALUES);
  const [errors, setErrors] = useState<FieldErrors>({});
  const [submitting, setSubmitting] = useState(false);
  const [submitError, setSubmitError] = useState(false);
  const [success, setSuccess] = useState(false);

  function update<K extends keyof Values>(key: K, value: Values[K]) {
    setValues((prev) => ({ ...prev, [key]: value }));
  }

  function toggleService(service: string) {
    setValues((prev) => ({
      ...prev,
      services: prev.services.includes(service)
        ? prev.services.filter((s) => s !== service)
        : [...prev.services, service],
    }));
  }

  const labelFor = (field: keyof Values): string => {
    switch (field) {
      case "name":
        return t("cateringNameLabel");
      case "phone":
        return t("cateringPhoneLabel");
      case "email":
        return t("cateringEmailLabel");
      case "eventType":
        return t("cateringEventTypeLabel");
      case "eventDate":
        return t("cateringEventDateLabel");
      case "guestCount":
        return t("cateringGuestCountLabel");
      case "location":
        return t("cateringLocationLabel");
      case "services":
        return t("cateringServicesLabel");
      case "message":
        return t("cateringMessageLabel");
      default:
        return "";
    }
  };

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    const nextErrors = validate(values);
    setErrors(nextErrors);
    if (Object.keys(nextErrors).length > 0) return;

    setSubmitting(true);
    setSubmitError(false);

    const payload: CreateCateringEnquiryRequest = {
      name: values.name.trim(),
      phone: values.phone.trim(),
      email: values.email.trim(),
      event_type: values.eventType,
      event_date: values.eventDate,
      guest_count: Number.parseInt(values.guestCount, 10),
      location: values.location.trim(),
      services_required: values.services,
      message: values.message.trim() ? values.message.trim() : null,
      website: values.website,
    };

    const result = await submitCateringEnquiry(payload);
    setSubmitting(false);

    if (result.ok) {
      setSuccess(true);
      return;
    }

    if (result.kind === "validation_error") {
      const nextFieldErrors: FieldErrors = {};
      for (const field of result.fields) {
        const localField = SERVER_FIELD_TO_LOCAL[field.field];
        if (localField) {
          nextFieldErrors[localField] = SERVER_MESSAGE_TO_ERROR_KEY[field.message] ?? "invalid";
        }
      }
      setErrors(nextFieldErrors);
      return;
    }

    setSubmitError(true);
  }

  if (success) {
    return (
      <div className="flex flex-col gap-2 py-8 text-center">
        <h2 className="m-0">{t("cateringSuccessHeading")}</h2>
        <p className="m-0 text-text-muted">{t("cateringSuccessBody")}</p>
      </div>
    );
  }

  return (
    <form onSubmit={handleSubmit} noValidate className="flex flex-col gap-5">
      <div>
        <label htmlFor="catering-name" className={FIELD_LABEL_CLASS}>{t("cateringNameLabel")}</label>
        <input
          id="catering-name"
          className="field"
          value={values.name}
          onChange={(e) => update("name", e.target.value)}
        />
        {errors.name && <p className={FIELD_ERROR_CLASS}>{t(`errors.${errors.name}`, { field: labelFor("name") })}</p>}
      </div>

      <div>
        <label htmlFor="catering-phone" className={FIELD_LABEL_CLASS}>{t("cateringPhoneLabel")}</label>
        <input
          id="catering-phone"
          type="tel"
          className="field"
          value={values.phone}
          onChange={(e) => update("phone", e.target.value)}
        />
        {errors.phone && <p className={FIELD_ERROR_CLASS}>{t(`errors.${errors.phone}`, { field: labelFor("phone") })}</p>}
      </div>

      <div>
        <label htmlFor="catering-email" className={FIELD_LABEL_CLASS}>{t("cateringEmailLabel")}</label>
        <input
          id="catering-email"
          type="email"
          className="field"
          value={values.email}
          onChange={(e) => update("email", e.target.value)}
        />
        {errors.email && <p className={FIELD_ERROR_CLASS}>{t(`errors.${errors.email}`, { field: labelFor("email") })}</p>}
      </div>

      <div>
        <label htmlFor="catering-event-type" className={FIELD_LABEL_CLASS}>{t("cateringEventTypeLabel")}</label>
        <select
          id="catering-event-type"
          className="field"
          value={values.eventType}
          onChange={(e) => update("eventType", e.target.value)}
        >
          <option value="" disabled>
            {t("cateringEventTypeLabel")}
          </option>
          {EVENT_TYPES.map((value) => (
            <option key={value} value={value}>
              {t(`eventType.${value}`)}
            </option>
          ))}
        </select>
        {errors.eventType && (
          <p className={FIELD_ERROR_CLASS}>{t(`errors.${errors.eventType}`, { field: labelFor("eventType") })}</p>
        )}
      </div>

      <div>
        <label htmlFor="catering-event-date" className={FIELD_LABEL_CLASS}>{t("cateringEventDateLabel")}</label>
        <input
          id="catering-event-date"
          type="date"
          min={todayIsoDate()}
          className="field"
          value={values.eventDate}
          onChange={(e) => update("eventDate", e.target.value)}
        />
        {errors.eventDate && (
          <p className={FIELD_ERROR_CLASS}>{t(`errors.${errors.eventDate}`, { field: labelFor("eventDate") })}</p>
        )}
      </div>

      <div>
        <label htmlFor="catering-guest-count" className={FIELD_LABEL_CLASS}>{t("cateringGuestCountLabel")}</label>
        <input
          id="catering-guest-count"
          type="number"
          min={1}
          className="field"
          value={values.guestCount}
          onChange={(e) => update("guestCount", e.target.value)}
        />
        {errors.guestCount && (
          <p className={FIELD_ERROR_CLASS}>{t(`errors.${errors.guestCount}`, { field: labelFor("guestCount") })}</p>
        )}
      </div>

      <div>
        <label htmlFor="catering-location" className={FIELD_LABEL_CLASS}>{t("cateringLocationLabel")}</label>
        <input
          id="catering-location"
          className="field"
          value={values.location}
          onChange={(e) => update("location", e.target.value)}
        />
        {errors.location && (
          <p className={FIELD_ERROR_CLASS}>{t(`errors.${errors.location}`, { field: labelFor("location") })}</p>
        )}
      </div>

      <fieldset className="flex flex-col gap-2">
        <legend className={FIELD_LABEL_CLASS}>{t("cateringServicesLabel")}</legend>
        {SERVICES.map((service) => (
          <label key={service} className="flex items-center gap-2">
            <input
              type="checkbox"
              checked={values.services.includes(service)}
              onChange={() => toggleService(service)}
            />
            {t(`service.${service}`)}
          </label>
        ))}
        {errors.services && (
          <p className={FIELD_ERROR_CLASS}>{t(`errors.${errors.services}`, { field: labelFor("services") })}</p>
        )}
      </fieldset>

      <div>
        <label htmlFor="catering-message" className={FIELD_LABEL_CLASS}>{t("cateringMessageLabel")}</label>
        <textarea
          id="catering-message"
          className="field"
          rows={4}
          maxLength={MESSAGE_MAX_LENGTH}
          value={values.message}
          onChange={(e) => update("message", e.target.value)}
        />
        {errors.message && (
          <p className={FIELD_ERROR_CLASS}>{t(`errors.${errors.message}`, { field: labelFor("message") })}</p>
        )}
      </div>

      <div
        aria-hidden="true"
        style={{ position: "absolute", left: "-9999px", width: "1px", height: "1px", overflow: "hidden" }}
      >
        <label htmlFor="catering-website">Leave this field blank</label>
        <input
          id="catering-website"
          name="website"
          type="text"
          tabIndex={-1}
          autoComplete="off"
          value={values.website}
          onChange={(event) => update("website", event.target.value)}
        />
      </div>

      {submitError && <Notice>{t("submitErrorGeneric")}</Notice>}

      <Button type="submit" disabled={submitting} className="self-start">
        {submitting ? t("submittingEnquiry") : t("submitEnquiry")}
      </Button>
    </form>
  );
}
