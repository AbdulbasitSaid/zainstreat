"use client";

import { useState, type FormEvent } from "react";
import { useTranslations } from "next-intl";
import { Button } from "@/components/button";
import { Notice } from "@/components/notice";
import { submitContactMessage, type CreateContactMessageRequest } from "@/lib/contact";

const SUBJECTS = ["general", "catering", "event_rental", "menu", "order", "other"] as const;

const SUBJECT_LABEL_KEYS: Record<(typeof SUBJECTS)[number], string> = {
  general: "formSubjectGeneral",
  catering: "formSubjectCatering",
  event_rental: "formSubjectEventRental",
  menu: "formSubjectMenu",
  order: "formSubjectOrder",
  other: "formSubjectOther",
};

interface Values {
  name: string;
  email: string;
  phone: string;
  subject: string;
  message: string;
  website: string;
}

type FieldErrors = Partial<Record<keyof Values, string>>;

const EMPTY_VALUES: Values = {
  name: "",
  email: "",
  phone: "",
  subject: "general",
  message: "",
  website: "",
};

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

function validate(values: Values): FieldErrors {
  const errors: FieldErrors = {};

  const trimmedName = values.name.trim();
  if (!trimmedName) errors.name = "required";

  if (!values.email.trim()) errors.email = "required";
  else if (!isValidEmail(values.email)) errors.email = "invalid";

  const trimmedPhone = values.phone.trim();
  if (!trimmedPhone) errors.phone = "required";
  else if (!isValidPhone(trimmedPhone)) errors.phone = "invalid";

  const trimmedMessage = values.message.trim();
  if (!trimmedMessage) errors.message = "required";
  else if (trimmedMessage.length > MESSAGE_MAX_LENGTH) errors.message = "tooLong";

  return errors;
}

const SERVER_MESSAGE_TO_ERROR_KEY: Record<string, string> = {
  required: "required",
  invalid: "invalid",
  too_short: "tooShort",
  too_long: "tooLong",
};

const SERVER_FIELD_TO_LOCAL: Record<string, keyof Values> = {
  name: "name",
  email: "email",
  phone: "phone",
  subject: "subject",
  message: "message",
};

const FIELD_LABEL_CLASS = "mb-1.5 block text-sm font-semibold text-text";
const FIELD_ERROR_CLASS = "mt-1 text-sm text-primary";

export function ContactForm() {
  const t = useTranslations("ContactPage");
  const [values, setValues] = useState<Values>(EMPTY_VALUES);
  const [errors, setErrors] = useState<FieldErrors>({});
  const [submitting, setSubmitting] = useState(false);
  const [submitError, setSubmitError] = useState(false);
  const [success, setSuccess] = useState(false);

  function update<K extends keyof Values>(key: K, value: Values[K]) {
    setValues((prev) => ({ ...prev, [key]: value }));
  }

  const labelFor = (field: keyof Values): string => {
    switch (field) {
      case "name":
        return t("formName");
      case "email":
        return t("formEmail");
      case "phone":
        return t("formPhone");
      case "subject":
        return t("formSubject");
      case "message":
        return t("formMessage");
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

    const payload: CreateContactMessageRequest = {
      name: values.name.trim(),
      email: values.email.trim(),
      phone: values.phone.trim(),
      subject: values.subject,
      message: values.message.trim(),
      website: values.website,
    };

    const result = await submitContactMessage(payload);
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
        <h2 className="m-0">{t("contactSuccessHeading")}</h2>
        <p className="m-0 text-text-muted">{t("contactSuccessBody")}</p>
      </div>
    );
  }

  return (
    <form onSubmit={handleSubmit} noValidate>
      <div className="mb-5">
        <label htmlFor="contact-name" className={FIELD_LABEL_CLASS}>{t("formName")}</label>
        <input
          id="contact-name"
          className="field"
          value={values.name}
          onChange={(e) => update("name", e.target.value)}
          aria-invalid={!!errors.name}
          aria-describedby={errors.name ? "contact-name-error" : undefined}
        />
        {errors.name && (
          <p id="contact-name-error" className={FIELD_ERROR_CLASS}>
            {t(`errors.${errors.name}`, { field: labelFor("name") })}
          </p>
        )}
      </div>

      <div className="mb-5">
        <label htmlFor="contact-email" className={FIELD_LABEL_CLASS}>{t("formEmail")}</label>
        <input
          id="contact-email"
          type="email"
          className="field"
          value={values.email}
          onChange={(e) => update("email", e.target.value)}
          aria-invalid={!!errors.email}
          aria-describedby={errors.email ? "contact-email-error" : undefined}
        />
        {errors.email && (
          <p id="contact-email-error" className={FIELD_ERROR_CLASS}>
            {t(`errors.${errors.email}`, { field: labelFor("email") })}
          </p>
        )}
      </div>

      <div className="mb-5">
        <label htmlFor="contact-phone" className={FIELD_LABEL_CLASS}>{t("formPhone")}</label>
        <input
          id="contact-phone"
          type="tel"
          className="field"
          value={values.phone}
          onChange={(e) => update("phone", e.target.value)}
          aria-invalid={!!errors.phone}
          aria-describedby={errors.phone ? "contact-phone-error" : undefined}
        />
        {errors.phone && (
          <p id="contact-phone-error" className={FIELD_ERROR_CLASS}>
            {t(`errors.${errors.phone}`, { field: labelFor("phone") })}
          </p>
        )}
      </div>

      <div className="mb-5">
        <label htmlFor="contact-subject" className={FIELD_LABEL_CLASS}>{t("formSubject")}</label>
        <select
          id="contact-subject"
          className="field"
          value={values.subject}
          onChange={(e) => update("subject", e.target.value)}
          aria-invalid={!!errors.subject}
          aria-describedby={errors.subject ? "contact-subject-error" : undefined}
        >
          {SUBJECTS.map((value) => (
            <option key={value} value={value}>
              {t(SUBJECT_LABEL_KEYS[value])}
            </option>
          ))}
        </select>
        {errors.subject && (
          <p id="contact-subject-error" className={FIELD_ERROR_CLASS}>
            {t(`errors.${errors.subject}`, { field: labelFor("subject") })}
          </p>
        )}
      </div>

      <div className="mb-5">
        <label htmlFor="contact-message" className={FIELD_LABEL_CLASS}>{t("formMessage")}</label>
        <textarea
          id="contact-message"
          rows={5}
          className="field"
          maxLength={MESSAGE_MAX_LENGTH}
          value={values.message}
          onChange={(e) => update("message", e.target.value)}
          aria-invalid={!!errors.message}
          aria-describedby={errors.message ? "contact-message-error" : undefined}
        />
        {errors.message && (
          <p id="contact-message-error" className={FIELD_ERROR_CLASS}>
            {t(`errors.${errors.message}`, { field: labelFor("message") })}
          </p>
        )}
      </div>

      <div
        aria-hidden="true"
        style={{ position: "absolute", left: "-9999px", width: "1px", height: "1px", overflow: "hidden" }}
      >
        <label htmlFor="contact-website">Leave this field blank</label>
        <input
          id="contact-website"
          name="website"
          type="text"
          tabIndex={-1}
          autoComplete="off"
          value={values.website}
          onChange={(event) => update("website", event.target.value)}
        />
      </div>

      {submitError && <Notice className="mb-4">{t("submitErrorGeneric")}</Notice>}

      <Button type="submit" disabled={submitting} className="mb-4">
        {submitting ? t("submittingMessage") : t("submitMessage")}
      </Button>
    </form>
  );
}
