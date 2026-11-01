"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { Notice } from "@/components/notice";
import { AdminImageUpload } from "@/components/admin-image-upload";
import { AdminDraftBar, AdminDraftStatus } from "@/components/admin-draft-bar";
import { useRefreshPreview } from "@/components/admin-preview-panel";
import { routing, type AppLocale } from "@/i18n/routing";
import { discardDraft, publishEntries } from "@/lib/admin-drafts";
import {
  HOME_TEXT_KEYS,
  RICH_HOME_TEXT_KEYS,
  SITE_IMAGE_ASPECT,
  SITE_IMAGE_LABELS,
  SITE_IMAGE_SLOTS,
  type HomeTextKey,
  type SiteContent,
  type SiteImageSlot,
} from "@/lib/site-content";
import { putDraft, useAutosaveDraft } from "@/lib/use-autosave-draft";

export interface AdminSiteContentState {
  published: SiteContent;
  draft: SiteContent | null;
  draft_updated_at: string | null;
}

interface FieldErrorItem {
  field: string;
  message: string;
}

const LOCALE_NAMES: Record<AppLocale, string> = { en: "English", nl: "Nederlands", fr: "Français" };

// Longer copy gets a textarea.
const MULTILINE_KEYS: readonly HomeTextKey[] = [
  "heroSupporting",
  "aboutPreview",
  "trustHalalDesc",
  "trustFreshDesc",
  "trustHygienicDesc",
  "trustReliableDesc",
];

const ERROR_TEXT: Record<string, string> = {
  invalid_markup: "Only <em>…</em> is allowed here, opened and closed once.",
  too_long: "Keep it under 400 characters.",
  unknown_key: "This text can't be edited.",
  invalid_locale: "Unknown language.",
  invalid: "That image couldn't be used — upload it again.",
};

function isValidationBody(body: unknown): body is { fields: FieldErrorItem[] } {
  return typeof body === "object" && body !== null && "fields" in body && Array.isArray(body.fields);
}

/**
 * The homepage text and site images (Phase 25 Decisions 11–14, plan 9.4).
 * Autosaves a draft; the public site changes only on Publish.
 */
export function AdminSiteContentEditor({
  initial,
  defaultImages,
  defaultTexts,
}: {
  initial: AdminSiteContentState;
  /** The bundled image each slot falls back to. */
  defaultImages: Record<SiteImageSlot, string>;
  /** The message-file strings, shown as placeholders. */
  defaultTexts: Record<AppLocale, Record<HomeTextKey, string>>;
}) {
  const router = useRouter();
  const refreshPreview = useRefreshPreview();
  const [content, setContent] = useState<SiteContent>(initial.draft ?? initial.published);
  const [locale, setLocale] = useState<AppLocale>(routing.defaultLocale);
  const [fieldErrors, setFieldErrors] = useState<FieldErrorItem[]>([]);
  const [pending, setPending] = useState(initial.draft !== null);

  const autosave = useAutosaveDraft({
    value: content,
    isValid: () => true,
    save: (value, draftUpdatedAt) =>
      putDraft<{ draft: SiteContent; draft_updated_at: string }>("/api/admin/site-content/draft", {
        content: value,
        draft_updated_at: draftUpdatedAt,
      }),
    initialDraftUpdatedAt: initial.draft_updated_at,
    onSaved: () => {
      setFieldErrors([]);
      setPending(true);
      refreshPreview();
    },
    onInvalid: (body) => {
      if (isValidationBody(body)) setFieldErrors(body.fields);
    },
  });

  function fieldError(field: string) {
    const message = fieldErrors.find((error) => error.field === field)?.message;
    return message ? (ERROR_TEXT[message] ?? "Check this value.") : null;
  }

  function setImage(slot: SiteImageSlot, url: string | null) {
    setContent((current) => ({ ...current, images: { ...current.images, [slot]: url } }));
  }

  function setText(key: HomeTextKey, text: string) {
    setContent((current) => ({
      ...current,
      home_text: { ...current.home_text, [locale]: { ...current.home_text?.[locale], [key]: text } },
    }));
  }

  async function handlePublish() {
    await autosave.flush();
    const result = await publishEntries([{ kind: "site_content", draft_updated_at: autosave.draftUpdatedAt() }]);
    if (result === "ok") {
      autosave.reset(null);
      setPending(false);
      router.refresh();
    }
    return result;
  }

  async function handleDiscard() {
    const result = await discardDraft("site_content");
    if (result === "ok") {
      router.refresh();
      refreshPreview();
    }
    return result;
  }

  return (
    <div className="flex max-w-[48rem] flex-col gap-10">
      <div className="flex min-h-6 items-center">
        <AdminDraftStatus status={autosave.status} />
      </div>
      {autosave.status === "conflict" && (
        <Notice>
          The site content was changed elsewhere since you opened it. Reload the page to see the
          latest version; autosave is paused until then.
        </Notice>
      )}

      <section className="flex flex-col gap-6">
        <h2 className="m-0 text-2xl">Images</h2>
        <div className="grid gap-8 sm:grid-cols-2">
          {SITE_IMAGE_SLOTS.map((slot) => {
            const current = content.images?.[slot] ?? null;
            const error = fieldError(`images.${slot}`);
            return (
              <div key={slot} className="flex flex-col gap-3 rounded-card border border-text/10 p-4">
                <div className="flex items-center justify-between gap-3">
                  <span className="text-sm font-semibold">{SITE_IMAGE_LABELS[slot]}</span>
                  {current === null ? (
                    <span className="rounded-full bg-text/10 px-2.5 py-0.5 text-xs font-semibold text-text-muted">
                      Default
                    </span>
                  ) : (
                    <button
                      type="button"
                      onClick={() => setImage(slot, null)}
                      className="text-xs font-semibold text-primary underline"
                    >
                      Reset to default
                    </button>
                  )}
                </div>
                <AdminImageUpload
                  inputId={`site-image-${slot}`}
                  label="Replace"
                  aspect={SITE_IMAGE_ASPECT[slot]}
                  currentImageUrl={current ?? defaultImages[slot]}
                  onUploaded={(url) => setImage(slot, url)}
                />
                {error && <p className="m-0 text-sm text-primary-dark">{error}</p>}
              </div>
            );
          })}
        </div>
      </section>

      <section className="flex flex-col gap-6">
        <h2 className="m-0 text-2xl">Homepage text</h2>
        <p className="m-0 -mt-3 text-sm text-text-muted">
          Leave a field empty to keep the standard text shown in grey.
        </p>
        <div className="flex gap-2" role="tablist" aria-label="Language">
          {routing.locales.map((code) => (
            <button
              key={code}
              type="button"
              role="tab"
              aria-selected={locale === code}
              onClick={() => setLocale(code)}
              className={`rounded-full px-4 py-2 text-xs font-semibold uppercase tracking-wider ${
                locale === code ? "bg-primary text-white" : "bg-card text-text hover:bg-background-soft"
              }`}
            >
              {LOCALE_NAMES[code]}
            </button>
          ))}
        </div>

        <div role="tabpanel" className="flex flex-col gap-5">
          {HOME_TEXT_KEYS.map((key) => {
            const id = `home-text-${locale}-${key}`;
            const value = content.home_text?.[locale]?.[key] ?? "";
            const error = fieldError(`home_text.${locale}.${key}`);
            const common = {
              id,
              className: "field",
              value,
              placeholder: defaultTexts[locale][key],
              onChange: (event: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) =>
                setText(key, event.target.value),
            };
            return (
              <div key={id}>
                <label htmlFor={id} className="mb-1.5 block text-sm font-semibold">
                  {key}
                </label>
                {MULTILINE_KEYS.includes(key) ? <textarea rows={3} {...common} /> : <input type="text" {...common} />}
                {RICH_HOME_TEXT_KEYS.includes(key) && (
                  <p className="m-0 mt-1 text-xs text-text-muted">
                    Use &lt;em&gt;…&lt;/em&gt; for the highlighted word.
                  </p>
                )}
                {error && <p className="m-0 mt-1 text-sm text-primary-dark">{error}</p>}
              </div>
            );
          })}
        </div>
      </section>

      <AdminDraftBar
        pending={pending}
        isNew={false}
        disabled={autosave.status === "conflict"}
        onPublish={handlePublish}
        onDiscard={handleDiscard}
      />
    </div>
  );
}
