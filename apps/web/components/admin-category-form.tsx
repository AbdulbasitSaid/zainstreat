"use client";

import { useState, type FormEvent } from "react";
import { useRouter } from "next/navigation";
import { Button } from "@/components/button";
import { Notice } from "@/components/notice";
import { AdminDraftBar, AdminDraftStatus } from "@/components/admin-draft-bar";
import { useRefreshPreview } from "@/components/admin-preview-panel";
import { discardDraft, publishEntries } from "@/lib/admin-drafts";
import type { AdminCategory, CategoryFields } from "@/lib/admin-categories";
import { putDraft, useAutosaveDraft } from "@/lib/use-autosave-draft";

interface FieldErrorItem {
  field: string;
  message: string;
}

function isValidationBody(body: unknown): body is { fields: FieldErrorItem[] } {
  return typeof body === "object" && body !== null && "fields" in body && Array.isArray(body.fields);
}

/**
 * Same model as the menu item form (Phase 25): create posts an unpublished
 * category, edits autosave a draft until "Publish this change".
 */
export function AdminCategoryForm({ initialValue }: { initialValue?: AdminCategory }) {
  const router = useRouter();
  const refreshPreview = useRefreshPreview();
  const start: Partial<CategoryFields> = initialValue?.draft ?? initialValue ?? {};
  const [name, setName] = useState(start.name ?? "");
  const [description, setDescription] = useState(start.description ?? "");
  const [fieldErrors, setFieldErrors] = useState<FieldErrorItem[]>([]);
  const [submitting, setSubmitting] = useState(false);
  const [pending, setPending] = useState(initialValue ? initialValue.has_draft || !initialValue.is_published : false);

  function fieldError(field: string) {
    return fieldErrors.find((error) => error.field === field)?.message;
  }

  const fields: CategoryFields = { name, description: description.trim() === "" ? null : description };

  const autosave = useAutosaveDraft({
    value: fields,
    isValid: () => initialValue !== undefined,
    save: (value, draftUpdatedAt) =>
      putDraft<AdminCategory>(`/api/admin/categories/${initialValue?.id}/draft`, {
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
      { kind: "category", id: initialValue.id, draft_updated_at: autosave.draftUpdatedAt() },
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
    const result = await discardDraft("category", initialValue.id);
    if (result === "ok") {
      if (!initialValue.is_published) router.push("/categories");
      router.refresh();
      refreshPreview();
    }
    return result;
  }

  async function handleCreate(event: FormEvent) {
    event.preventDefault();
    setFieldErrors([]);
    setSubmitting(true);

    const response = await fetch("/api/admin/categories", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(fields),
    });

    setSubmitting(false);

    if (response.ok) {
      const created = (await response.json()) as AdminCategory;
      router.push(`/categories/${created.id}`);
      return;
    }

    const body: unknown = await response.json();
    setFieldErrors(isValidationBody(body) ? body.fields : [{ field: "name", message: "required" }]);
  }

  return (
    <form
      onSubmit={initialValue ? (event) => event.preventDefault() : handleCreate}
      noValidate
      className="flex max-w-[32rem] flex-col gap-5"
    >
      {initialValue && (
        <div className="flex min-h-6 items-center">
          <AdminDraftStatus status={autosave.status} />
        </div>
      )}
      {autosave.status === "conflict" && (
        <Notice>
          This category was changed elsewhere since you opened it. Reload the page to see the
          latest version; autosave is paused until then.
        </Notice>
      )}

      <div>
        <label htmlFor="category-name" className="mb-1.5 block text-sm font-semibold">
          Name
        </label>
        <input
          id="category-name"
          type="text"
          className="field"
          value={name}
          onChange={(event) => setName(event.target.value)}
        />
        {fieldError("name") && <p className="m-0 mt-1 text-sm text-primary-dark">Name is required.</p>}
      </div>

      <div>
        <label htmlFor="category-description" className="mb-1.5 block text-sm font-semibold">
          Description
        </label>
        <textarea
          id="category-description"
          className="field"
          rows={3}
          value={description}
          onChange={(event) => setDescription(event.target.value)}
        />
      </div>

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
          {submitting ? "Saving…" : "Create category"}
        </Button>
      )}
    </form>
  );
}
