"use client";

import { useEffect, useState, type ReactNode } from "react";
import {
  LOCALE_NAMES,
  TRANSLATION_LOCALES,
  automaticTranslationBlocked,
  dropTranslation,
  editTranslation,
  type TranslationLocale,
  type TranslationSettingsStatus,
  type TranslationStatus,
  type TranslationStatusPair,
  type Translations,
} from "@/lib/admin-translations";

const LABEL = "mb-1.5 block text-sm font-semibold";
const HINT = "m-0 text-sm text-text-muted";

/** DeepL's state, for the tabs' messages. Null until loaded or if it fails. */
export function useTranslationSettings(): TranslationSettingsStatus | null {
  const [settings, setSettings] = useState<TranslationSettingsStatus | null>(null);
  useEffect(() => {
    let cancelled = false;
    fetch("/api/admin/translations/status", { cache: "no-store" })
      .then((response) => (response.ok ? (response.json() as Promise<TranslationSettingsStatus>) : null))
      .then((data) => {
        if (!cancelled) setSettings(data);
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, []);
  return settings;
}

function needsYou(status: TranslationStatus): boolean {
  return status === "missing" || status === "stale" || status === "parked" || status === "manual_stale";
}

function StatusLine({
  status,
  fillsOn,
  blocked,
  onRetranslate,
}: {
  status: TranslationStatus;
  fillsOn: "publish" | "save";
  blocked: string | null;
  onRetranslate: () => void;
}) {
  const retranslate = (
    <button type="button" className="font-semibold text-primary underline" onClick={onRetranslate}>
      Re-translate
    </button>
  );
  const when = fillsOn === "publish" ? "when you publish" : "when you save";
  switch (status) {
    case "ok":
      return <p className={HINT}>Translated automatically. Type over it to use your own wording.</p>;
    case "manual":
      return (
        <p className={HINT}>
          Your own wording: DeepL won&apos;t change it. {retranslate}
        </p>
      );
    case "manual_stale":
      return (
        <p className={HINT}>
          The English changed since this was edited. Update it, or {retranslate}.
        </p>
      );
    case "parked":
      return (
        <p className={HINT}>
          Couldn&apos;t translate automatically. Please type it yourself, or {retranslate} to try again.
        </p>
      );
    case "missing":
    case "stale":
      return blocked ? (
        <p className={HINT}>{blocked}. Type it yourself, or the English is shown.</p>
      ) : (
        <p className={HINT}>
          {status === "stale" ? "The English changed: DeepL" : "DeepL"} translates this {when}. You can also type
          it yourself.
        </p>
      );
  }
}

export interface OptionTranslations {
  label: string;
  translations: Translations;
}

/**
 * Phase 38 Decision 5: English / Dutch / French tabs over a form's name and
 * description (and price-option labels). English is the form's own
 * fields; typing in Dutch or French makes that text the owner's, which
 * DeepL never overwrites. Re-translate drops it so the next publish (or,
 * for rentals, save) translates it again.
 */
export function AdminTranslationTabs({
  idPrefix,
  english,
  englishFields,
  translations,
  onChange,
  options,
  onOptionChange,
  status,
  fillsOn,
  settings,
}: {
  idPrefix: string;
  english: { name: string; description: string };
  englishFields: ReactNode;
  translations: Translations;
  onChange: (translations: Translations) => void;
  options?: OptionTranslations[];
  onOptionChange?: (index: number, translations: Translations) => void;
  status: TranslationStatusPair | undefined;
  fillsOn: "publish" | "save";
  settings: TranslationSettingsStatus | null;
}) {
  const [tab, setTab] = useState<"en" | TranslationLocale>("en");
  const blocked = automaticTranslationBlocked(settings);

  function retranslate(locale: TranslationLocale) {
    onChange(dropTranslation(translations, locale));
    options?.forEach((option, index) => onOptionChange?.(index, dropTranslation(option.translations, locale)));
  }

  return (
    <section className="flex flex-col gap-4">
      <div role="tablist" aria-label="Language" className="flex gap-1 self-start rounded-full bg-background-soft p-1">
        {(["en", ...TRANSLATION_LOCALES] as const).map((code) => {
          const attention = code !== "en" && status !== undefined && needsYou(status[code]);
          return (
            <button
              key={code}
              type="button"
              role="tab"
              aria-selected={tab === code}
              onClick={() => setTab(code)}
              className={`rounded-full px-4 py-1.5 text-sm font-semibold ${tab === code ? "bg-primary text-white" : "text-text"}`}
            >
              {LOCALE_NAMES[code]}
              {attention && (
                <span aria-label="needs attention" className="ml-1.5 inline-block size-2 rounded-full bg-accent" />
              )}
            </button>
          );
        })}
      </div>

      {tab === "en" ? (
        englishFields
      ) : (
        <div className="flex flex-col gap-4">
          {status && (
            <StatusLine status={status[tab]} fillsOn={fillsOn} blocked={blocked} onRetranslate={() => retranslate(tab)} />
          )}
          <div>
            <label htmlFor={`${idPrefix}-name-${tab}`} className={LABEL}>
              Name ({LOCALE_NAMES[tab]})
            </label>
            <input
              id={`${idPrefix}-name-${tab}`}
              type="text"
              className="field"
              value={translations[tab]?.name ?? ""}
              placeholder={english.name}
              onChange={(event) => onChange(editTranslation(translations, tab, "name", event.target.value))}
            />
          </div>
          {english.description.trim() !== "" && (
            <div>
              <label htmlFor={`${idPrefix}-description-${tab}`} className={LABEL}>
                Description ({LOCALE_NAMES[tab]})
              </label>
              <textarea
                id={`${idPrefix}-description-${tab}`}
                className="field"
                rows={3}
                value={translations[tab]?.description ?? ""}
                placeholder={english.description}
                onChange={(event) => onChange(editTranslation(translations, tab, "description", event.target.value))}
              />
            </div>
          )}
          {options && options.length > 0 && (
            <fieldset className="m-0 flex flex-col gap-2 border-0 p-0">
              <legend className={LABEL}>Size labels ({LOCALE_NAMES[tab]})</legend>
              {options.map((option, index) => (
                <input
                  key={index}
                  type="text"
                  aria-label={`${option.label} (${LOCALE_NAMES[tab]})`}
                  className="field"
                  value={option.translations[tab]?.label ?? ""}
                  placeholder={option.label}
                  onChange={(event) =>
                    onOptionChange?.(index, editTranslation(option.translations, tab, "label", event.target.value))
                  }
                />
              ))}
            </fieldset>
          )}
          <p className={HINT}>Leave a field blank to show the English.</p>
        </div>
      )}
    </section>
  );
}
