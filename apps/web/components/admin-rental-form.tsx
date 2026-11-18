"use client";

import { useState, type FormEvent } from "react";
import { useRouter } from "next/navigation";
import { Button } from "@/components/button";
import { Notice } from "@/components/notice";
import { AdminImageUpload } from "@/components/admin-image-upload";
import type { AdminRental, RentalFields } from "@/lib/admin-rentals";

interface FieldErrorItem {
  field: string;
  message: string;
}

function isValidationBody(body: unknown): body is { fields: FieldErrorItem[] } {
  return typeof body === "object" && body !== null && "fields" in body && Array.isArray(body.fields);
}

const PRICE_PATTERN = /^\d{1,8}(\.\d{1,2})?$/;

const ERROR_TEXT: Record<string, string> = {
  name: "Enter a name (up to 120 characters).",
  description: "Keep the description under 500 characters.",
  price: "Enter a price of 0 or more, with at most two decimals.",
  image_url: "That photo could not be used — try uploading again.",
  display_order: "Enter a whole number from 0 to 9999.",
};

/**
 * Phase 36 Decision 9: unlike the menu, a rental saves live straight away
 * (no draft → publish); `updated_at` guards against overwriting an edit
 * made elsewhere.
 */
export function AdminRentalForm({ initialValue }: { initialValue?: AdminRental }) {
  const router = useRouter();
  const [name, setName] = useState(initialValue?.name ?? "");
  const [description, setDescription] = useState(initialValue?.description ?? "");
  const [price, setPrice] = useState(initialValue?.price ?? "");
  const [imageUrl, setImageUrl] = useState<string | null>(initialValue?.image_url ?? null);
  const [isAvailable, setIsAvailable] = useState(initialValue?.is_available ?? true);
  const [displayOrder, setDisplayOrder] = useState(String(initialValue?.display_order ?? 0));
  const [fieldErrors, setFieldErrors] = useState<FieldErrorItem[]>([]);
  const [notice, setNotice] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [confirmingArchive, setConfirmingArchive] = useState(false);

  function fieldError(field: string) {
    return fieldErrors.some((error) => error.field === field) ? ERROR_TEXT[field] : undefined;
  }

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    setNotice(null);
    // A non-number would fail the API's JSON parsing, not its field checks.
    const normalizedPrice = price.trim().replace(",", ".");
    if (!PRICE_PATTERN.test(normalizedPrice)) {
      setFieldErrors([{ field: "price", message: "invalid" }]);
      return;
    }
    setFieldErrors([]);
    setSubmitting(true);

    const fields: RentalFields = {
      name,
      description: description.trim() === "" ? null : description,
      price: normalizedPrice,
      image_url: imageUrl,
      is_available: isAvailable,
      display_order: Number.parseInt(displayOrder, 10) || 0,
    };
    const response = await fetch(initialValue ? `/api/admin/rentals/${initialValue.id}` : "/api/admin/rentals", {
      method: initialValue ? "PUT" : "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(initialValue ? { ...fields, updated_at: initialValue.updated_at } : fields),
    });
    setSubmitting(false);

    if (response.ok) {
      router.push("/rentals");
      router.refresh();
      return;
    }

    const body: unknown = await response.json().catch(() => null);
    if (isValidationBody(body)) {
      setFieldErrors(body.fields);
    } else if (response.status === 409) {
      setNotice("This rental was changed elsewhere since you opened it. Reload the page to see the latest version.");
    } else {
      setNotice("Could not save this rental. Please try again.");
    }
  }

  async function handleArchive() {
    if (!initialValue) return;
    setSubmitting(true);
    const response = await fetch(`/api/admin/rentals/${initialValue.id}/archive`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ updated_at: initialValue.updated_at }),
    });
    setSubmitting(false);
    setConfirmingArchive(false);

    if (response.ok) {
      router.push("/rentals");
      router.refresh();
      return;
    }
    setNotice(
      response.status === 409
        ? "This rental was changed elsewhere since you opened it. Reload the page and try again."
        : "Could not archive this rental. Please try again.",
    );
  }

  const readOnly = initialValue?.is_archived ?? false;

  return (
    <form onSubmit={handleSubmit} noValidate className="flex max-w-[32rem] flex-col gap-5">
      {notice && <Notice>{notice}</Notice>}

      <div>
        <label htmlFor="rental-name" className="mb-1.5 block text-sm font-semibold">
          Name
        </label>
        <input
          id="rental-name"
          type="text"
          className="field"
          maxLength={120}
          value={name}
          aria-invalid={fieldError("name") ? true : undefined}
          onChange={(event) => setName(event.target.value)}
        />
        {fieldError("name") && <p className="m-0 mt-1 text-sm text-primary-dark">{fieldError("name")}</p>}
      </div>

      <div>
        <label htmlFor="rental-description" className="mb-1.5 block text-sm font-semibold">
          Description <span className="font-normal text-text-muted">(optional)</span>
        </label>
        <textarea
          id="rental-description"
          className="field"
          rows={3}
          maxLength={500}
          value={description}
          aria-invalid={fieldError("description") ? true : undefined}
          onChange={(event) => setDescription(event.target.value)}
        />
        {fieldError("description") && (
          <p className="m-0 mt-1 text-sm text-primary-dark">{fieldError("description")}</p>
        )}
      </div>

      <div>
        <label htmlFor="rental-price" className="mb-1.5 block text-sm font-semibold">
          Price per event (€)
        </label>
        <input
          id="rental-price"
          type="text"
          inputMode="decimal"
          className="field max-w-40"
          value={price}
          aria-invalid={fieldError("price") ? true : undefined}
          onChange={(event) => setPrice(event.target.value)}
        />
        {fieldError("price") && <p className="m-0 mt-1 text-sm text-primary-dark">{fieldError("price")}</p>}
      </div>

      <AdminImageUpload
        currentImageUrl={imageUrl}
        onUploaded={setImageUrl}
        inputId="rental-photo"
        label="Photo (optional)"
      />
      {fieldError("image_url") && <p className="m-0 -mt-4 text-sm text-primary-dark">{fieldError("image_url")}</p>}

      <div>
        <label htmlFor="rental-order" className="mb-1.5 block text-sm font-semibold">
          Display order
        </label>
        <input
          id="rental-order"
          type="number"
          inputMode="numeric"
          min={0}
          max={9999}
          step={1}
          className="field max-w-32"
          value={displayOrder}
          aria-describedby="rental-order-hint"
          aria-invalid={fieldError("display_order") ? true : undefined}
          onChange={(event) => setDisplayOrder(event.target.value)}
        />
        <p id="rental-order-hint" className="m-0 mt-1 text-sm text-text-muted">
          Lower numbers show first.
        </p>
        {fieldError("display_order") && (
          <p className="m-0 mt-1 text-sm text-primary-dark">{fieldError("display_order")}</p>
        )}
      </div>

      <label className="flex items-center gap-2 text-sm font-semibold">
        <input type="checkbox" checked={isAvailable} onChange={(event) => setIsAvailable(event.target.checked)} />
        Show on the Rentals page
      </label>

      {!readOnly && (
        <div className="flex flex-wrap items-center gap-3">
          <Button type="submit" disabled={submitting}>
            {submitting ? "Saving…" : initialValue ? "Save changes" : "Create rental"}
          </Button>
          {initialValue &&
            (confirmingArchive ? (
              <>
                <span className="text-sm text-text-muted">Archive this rental?</span>
                <Button type="button" variant="secondary" disabled={submitting} onClick={handleArchive}>
                  Confirm
                </Button>
                <Button type="button" variant="secondary" disabled={submitting} onClick={() => setConfirmingArchive(false)}>
                  Cancel
                </Button>
              </>
            ) : (
              <Button type="button" variant="secondary" onClick={() => setConfirmingArchive(true)}>
                Archive
              </Button>
            ))}
        </div>
      )}
    </form>
  );
}
