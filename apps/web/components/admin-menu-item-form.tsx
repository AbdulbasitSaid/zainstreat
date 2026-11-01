"use client";

import { useState, type FormEvent } from "react";
import { useRouter } from "next/navigation";
import { Button } from "@/components/button";
import { Notice } from "@/components/notice";
import { AdminImageUpload } from "@/components/admin-image-upload";
import { AdminDraftBar, AdminDraftStatus } from "@/components/admin-draft-bar";
import { useRefreshPreview } from "@/components/admin-preview-panel";
import {
  AdminPriceOptionsEditor,
  type PriceOptionsValue,
} from "@/components/admin-price-options-editor";
import { discardDraft, publishEntries } from "@/lib/admin-drafts";
import type { AdminCategory } from "@/lib/admin-categories";
import type { AdminMenuItem, MenuItemFields } from "@/lib/admin-menu-items";
import { putDraft, useAutosaveDraft } from "@/lib/use-autosave-draft";

interface FieldErrorItem {
  field: string;
  message: string;
}

function isValidationBody(body: unknown): body is { error: "validation_error"; fields: FieldErrorItem[] } {
  return typeof body === "object" && body !== null && "fields" in body && Array.isArray(body.fields);
}

/**
 * Creating an item posts it (it starts unpublished) and opens its edit
 * page. Editing autosaves a draft (Phase 25 Decisions 15, 18, 22); the live
 * menu only changes on "Publish this change". A published item's form
 * starts from its pending draft, if any.
 */
export function AdminMenuItemForm({
  categories,
  initialValue,
}: {
  categories: AdminCategory[];
  initialValue?: AdminMenuItem;
}) {
  const router = useRouter();
  const refreshPreview = useRefreshPreview();
  const start: Partial<MenuItemFields> = initialValue?.draft ?? initialValue ?? {};
  const [categoryId, setCategoryId] = useState(start.category_id ?? categories[0]?.id);
  const [name, setName] = useState(start.name ?? "");
  const [description, setDescription] = useState(start.description ?? "");
  const [isFeatured, setIsFeatured] = useState(start.is_featured ?? false);
  const startOptions = start.price_options?.map(({ label, price }) => ({ label, price })) ?? [];
  const [pricing, setPricing] = useState<PriceOptionsValue>({
    price: start.price ?? (startOptions.length > 0 ? null : ""),
    price_options: startOptions,
  });
  const [imageUrl, setImageUrl] = useState<string | null>(start.image_url ?? null);
  const [fieldErrors, setFieldErrors] = useState<FieldErrorItem[]>([]);
  const [submitting, setSubmitting] = useState(false);
  const [pending, setPending] = useState(initialValue ? initialValue.has_draft || !initialValue.is_published : false);

  function fieldError(field: string) {
    return fieldErrors.find((error) => error.field === field)?.message;
  }

  // An empty price field would otherwise serialize as `""`, which fails
  // Decimal parsing before the backend's own field-level validation ever
  // runs (a raw deserialization error, not a clean `validation_error`) —
  // substitute "0" so an incomplete submission still gets the backend's
  // normal "must be positive" message instead of an opaque network error.
  const fields: Omit<MenuItemFields, "category_id" | "image_url"> & {
    category_id: number | undefined;
    image_url: string | null;
  } = {
    category_id: categoryId,
    name,
    description: description.trim() === "" ? null : description,
    price: pricing.price === null ? null : pricing.price.trim() === "" ? "0" : pricing.price,
    price_options: pricing.price_options.map((option) => ({
      label: option.label,
      price: option.price.trim() === "" ? "0" : option.price,
    })),
    image_url: imageUrl,
    is_featured: isFeatured,
  };

  const autosave = useAutosaveDraft({
    value: fields,
    // Category and photo are pickers, not typed; everything else is left
    // to the API so its field errors show inline.
    isValid: (value) => initialValue !== undefined && value.category_id !== undefined && value.image_url !== null,
    save: (value, draftUpdatedAt) =>
      putDraft<AdminMenuItem>(`/api/admin/menu-items/${initialValue?.id}/draft`, {
        ...value,
        draft_updated_at: draftUpdatedAt,
      }),
    initialDraftUpdatedAt: initialValue?.draft_updated_at ?? null,
    onSaved: () => {
      setFieldErrors([]);
      setPending(true);
      refreshPreview();
    },
    onInvalid: (body) => {
      if (isValidationBody(body)) setFieldErrors(body.fields);
    },
  });

  async function handlePublish() {
    if (!initialValue) return "error" as const;
    await autosave.flush();
    const result = await publishEntries([
      { kind: "menu_item", id: initialValue.id, draft_updated_at: autosave.draftUpdatedAt() },
    ]);
    if (result === "ok") {
      autosave.reset(null);
      setPending(false);
      router.refresh();
    }
    return result;
  }

  async function handleDiscard() {
    if (!initialValue) return "error" as const;
    const result = await discardDraft("menu_item", initialValue.id);
    if (result === "ok") {
      if (!initialValue.is_published) {
        router.push("/menu");
      }
      // The page remounts this form from the restored live values.
      router.refresh();
      refreshPreview();
    }
    return result;
  }

  async function handleCreate(event: FormEvent) {
    event.preventDefault();
    setFieldErrors([]);

    if (categoryId === undefined) {
      setFieldErrors([{ field: "category_id", message: "required" }]);
      return;
    }

    if (imageUrl === null) {
      setFieldErrors([{ field: "image_url", message: "required" }]);
      return;
    }

    setSubmitting(true);
    const response = await fetch("/api/admin/menu-items", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(fields),
    });
    setSubmitting(false);

    if (response.ok) {
      const created = (await response.json()) as AdminMenuItem;
      router.push(`/menu/${created.id}`);
      return;
    }

    const body: unknown = await response.json();
    setFieldErrors(isValidationBody(body) ? body.fields : [{ field: "name", message: "unexpected" }]);
  }

  return (
    <form
      onSubmit={initialValue ? (event) => event.preventDefault() : handleCreate}
      noValidate
      className="flex max-w-[36rem] flex-col gap-6"
    >
      {initialValue && (
        <div className="flex min-h-6 flex-wrap items-center gap-3">
          <AdminDraftStatus status={autosave.status} />
        </div>
      )}
      {autosave.status === "conflict" && (
        <Notice>
          This item was changed elsewhere since you opened it. Reload the page to see the latest
          version; autosave is paused until then.
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
        initialPrice={start.price ?? null}
        initialOptions={startOptions}
        onChange={setPricing}
      />
      {(fieldError("price") || fieldError("price_options")) && (
        <p className="m-0 -mt-4 text-sm text-primary-dark">
          Set exactly one of a flat price or one-or-more sized price options, all positive.
        </p>
      )}

      <AdminImageUpload currentImageUrl={imageUrl} onUploaded={setImageUrl} />
      {fieldError("image_url") && (
        <p className="m-0 -mt-4 text-sm text-primary-dark">
          {fieldError("image_url") === "required"
            ? "A photo is required before you can save this item."
            : "That photo could not be used — try uploading again."}
        </p>
      )}

      <label className="flex items-center gap-2 text-sm font-semibold">
        <input
          type="checkbox"
          checked={isFeatured}
          onChange={(event) => setIsFeatured(event.target.checked)}
        />
        Featured
      </label>

      {initialValue ? (
        <AdminDraftBar
          pending={pending}
          isNew={!initialValue.is_published}
          disabled={autosave.status === "conflict"}
          onPublish={handlePublish}
          onDiscard={handleDiscard}
        />
      ) : (
        <Button type="submit" disabled={submitting} className="self-start">
          {submitting ? "Saving…" : "Create item"}
        </Button>
      )}
    </form>
  );
}
