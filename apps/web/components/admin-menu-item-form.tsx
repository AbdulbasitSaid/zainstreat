"use client";

import { useState, type FormEvent } from "react";
import { useRouter } from "next/navigation";
import { Button } from "@/components/button";
import { Notice } from "@/components/notice";
import { AdminImageUpload } from "@/components/admin-image-upload";
import {
  AdminPriceOptionsEditor,
  type PriceOptionsValue,
} from "@/components/admin-price-options-editor";
import type { AdminCategory } from "@/lib/admin-categories";
import type { AdminMenuItem } from "@/lib/admin-menu-items";

interface FieldErrorItem {
  field: string;
  message: string;
}

export function AdminMenuItemForm({
  categories,
  initialValue,
}: {
  categories: AdminCategory[];
  initialValue?: AdminMenuItem;
}) {
  const router = useRouter();
  const [categoryId, setCategoryId] = useState(initialValue?.category_id ?? categories[0]?.id);
  const [name, setName] = useState(initialValue?.name ?? "");
  const [description, setDescription] = useState(initialValue?.description ?? "");
  const [isFeatured, setIsFeatured] = useState(initialValue?.is_featured ?? false);
  const [pricing, setPricing] = useState<PriceOptionsValue>({
    price: initialValue?.price ?? "",
    price_options: initialValue?.price_options.map(({ label, price }) => ({ label, price })) ?? [],
  });
  const [imageUrl, setImageUrl] = useState<string | null>(initialValue?.image_url ?? null);
  const [fieldErrors, setFieldErrors] = useState<FieldErrorItem[]>([]);
  const [conflict, setConflict] = useState(false);
  const [submitting, setSubmitting] = useState(false);

  function fieldError(field: string) {
    return fieldErrors.find((error) => error.field === field)?.message;
  }

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    setFieldErrors([]);
    setConflict(false);

    if (categoryId === undefined) {
      setFieldErrors([{ field: "category_id", message: "required" }]);
      return;
    }

    setSubmitting(true);

    // An empty price field would otherwise serialize as `""`, which fails
    // Decimal parsing before the backend's own field-level validation ever
    // runs (a raw deserialization error, not a clean `validation_error`) —
    // substitute "0" so an incomplete submission still gets the backend's
    // normal "must be positive" message instead of an opaque network error.
    const price = pricing.price === null ? null : pricing.price.trim() === "" ? "0" : pricing.price;
    const priceOptions = pricing.price_options.map((option) => ({
      label: option.label,
      price: option.price.trim() === "" ? "0" : option.price,
    }));

    const payload = {
      category_id: categoryId,
      name,
      description: description.trim() === "" ? null : description,
      price,
      price_options: priceOptions,
      image_url: imageUrl,
      is_featured: isFeatured,
      // Ignored by create (nothing to conflict against yet) — required by
      // the request shape regardless (requirement.md Decision 17).
      updated_at: initialValue?.updated_at ?? new Date().toISOString(),
    };
    const url = initialValue ? `/api/admin/menu-items/${initialValue.id}` : "/api/admin/menu-items";
    const method = initialValue ? "PATCH" : "POST";

    const response = await fetch(url, {
      method,
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });

    setSubmitting(false);

    if (response.ok) {
      router.push("/admin/menu");
      router.refresh();
      return;
    }

    const body = (await response.json()) as { error: string; fields?: FieldErrorItem[] };
    if (body.error === "validation_error" && body.fields) {
      setFieldErrors(body.fields);
    } else if (body.error === "conflict") {
      setConflict(true);
    } else {
      setFieldErrors([{ field: "name", message: "unexpected" }]);
    }
  }

  return (
    <form onSubmit={handleSubmit} noValidate className="flex max-w-[36rem] flex-col gap-6">
      {conflict && (
        <Notice>
          This item was changed elsewhere since you opened it. Reload the page to see the latest
          version before saving again.
        </Notice>
      )}

      <div>
        <label htmlFor="menu-item-name" className="mb-1.5 block text-sm font-semibold">
          Name
        </label>
        <input
          id="menu-item-name"
          type="text"
          className="field"
          value={name}
          onChange={(event) => setName(event.target.value)}
        />
        {fieldError("name") && <p className="m-0 mt-1 text-sm text-primary-dark">Name is required.</p>}
      </div>

      <div>
        <label htmlFor="menu-item-description" className="mb-1.5 block text-sm font-semibold">
          Description
        </label>
        <textarea
          id="menu-item-description"
          className="field"
          rows={3}
          value={description}
          onChange={(event) => setDescription(event.target.value)}
        />
      </div>

      <div>
        <label htmlFor="menu-item-category" className="mb-1.5 block text-sm font-semibold">
          Category
        </label>
        <select
          id="menu-item-category"
          className="field"
          value={categoryId ?? ""}
          onChange={(event) => setCategoryId(Number(event.target.value))}
        >
          {categories.map((category) => (
            <option key={category.id} value={category.id}>
              {category.name}
            </option>
          ))}
        </select>
        {fieldError("category_id") && (
          <p className="m-0 mt-1 text-sm text-primary-dark">Choose a valid, non-archived category.</p>
        )}
      </div>

      <AdminPriceOptionsEditor
        initialPrice={initialValue?.price ?? null}
        initialOptions={initialValue?.price_options.map(({ label, price }) => ({ label, price })) ?? []}
        onChange={setPricing}
      />
      {(fieldError("price") || fieldError("price_options")) && (
        <p className="m-0 -mt-4 text-sm text-primary-dark">
          Set exactly one of a flat price or one-or-more sized price options, all positive.
        </p>
      )}

      <AdminImageUpload currentImageUrl={imageUrl} onUploaded={setImageUrl} />
      {fieldError("image_url") && (
        <p className="m-0 -mt-4 text-sm text-primary-dark">That photo could not be used — try uploading again.</p>
      )}

      <label className="flex items-center gap-2 text-sm font-semibold">
        <input
          type="checkbox"
          checked={isFeatured}
          onChange={(event) => setIsFeatured(event.target.checked)}
        />
        Featured
      </label>

      <Button type="submit" disabled={submitting || conflict} className="self-start">
        {submitting ? "Saving…" : initialValue ? "Save changes" : "Create item"}
      </Button>
    </form>
  );
}
