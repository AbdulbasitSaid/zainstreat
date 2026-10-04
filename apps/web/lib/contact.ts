export interface CreateContactMessageRequest {
  name: string;
  email: string;
  phone: string;
  subject: string;
  message: string;
  website: string; // honeypot — always "" for a real submission
}

export interface ContactMessageResponse {
  id: number;
  created_at: string;
}

export interface FieldError {
  field: string;
  message: string;
}

export type SubmitContactMessageResult =
  | { ok: true; message: ContactMessageResponse }
  | { ok: false; kind: "validation_error"; fields: FieldError[] }
  | { ok: false; kind: "unknown" };

function isFieldError(value: unknown): value is FieldError {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  return typeof v.field === "string" && typeof v.message === "string";
}

export async function submitContactMessage(
  payload: CreateContactMessageRequest,
): Promise<SubmitContactMessageResult> {
  const response = await fetch("/api/contact-messages", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload),
  });

  const body: unknown = await response.json().catch(() => null);

  if (response.ok) {
    return { ok: true, message: body as ContactMessageResponse };
  }

  if (
    response.status === 400 &&
    typeof body === "object" &&
    body !== null &&
    Array.isArray((body as Record<string, unknown>).fields) &&
    (body as Record<string, unknown[]>).fields.every(isFieldError)
  ) {
    return { ok: false, kind: "validation_error", fields: (body as { fields: FieldError[] }).fields };
  }

  // Covers both a genuinely unexpected error and a 429 from the rate
  // limiter — the generic error Notice is the right UX for a real user who
  // hit a shared/misbehaving-client limit either way.
  return { ok: false, kind: "unknown" };
}
