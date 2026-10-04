"use client";

import { useState, type FormEvent } from "react";
import { useRouter } from "next/navigation";
import { Button } from "@/components/button";
import { Notice } from "@/components/notice";
import type { AdminCategory } from "@/lib/admin-categories";

interface FieldErrorItem {
  field: string;
  message: string;
}

export function AdminCategoryForm({ initialValue }: { initialValue?: AdminCategory }) {
  const router = useRouter();
  const [name, setName] = useState(initialValue?.name ?? "");
  const [description, setDescription] = useState(initialValue?.description ?? "");
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
    setSubmitting(true);

    const payload = {
      name,
      description: description.trim() === "" ? null : description,
      // Ignored by create (nothing to conflict against yet) — required by
      // the request shape regardless (requirement.md Decision 17).
      updated_at: initialValue?.updated_at ?? new Date().toISOString(),
    };
    const url = initialValue ? `/api/admin/categories/${initialValue.id}` : "/api/admin/categories";
    const method = initialValue ? "PATCH" : "POST";

    const response = await fetch(url, {
      method,
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });

    setSubmitting(false);

    if (response.ok) {
      router.push("/admin/categories");
      router.refresh();
      return;
    }

    const body = (await response.json()) as { error: string; fields?: FieldErrorItem[] };
    if (body.error === "validation_error" && body.fields) {
      setFieldErrors(body.fields);
    } else if (body.error === "conflict") {
      setConflict(true);
    } else {
      setFieldErrors([{ field: "name", message: "required" }]);
    }
  }

  return (
    <form onSubmit={handleSubmit} noValidate className="flex max-w-[32rem] flex-col gap-5">
      {conflict && (
        <Notice>
          This category was changed elsewhere since you opened it. Reload the page to see the
          latest version before saving again.
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

      <Button type="submit" disabled={submitting || conflict} className="self-start">
        {submitting ? "Saving…" : initialValue ? "Save changes" : "Create category"}
      </Button>
    </form>
  );
}
