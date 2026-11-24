// Phase 38: nl/fr menu text (requirement.md Decisions 1, 5, 20). Mirrors
// apps/api's translation.rs.

export const TRANSLATION_LOCALES = ["nl", "fr"] as const;
export type TranslationLocale = (typeof TRANSLATION_LOCALES)[number];

export const LOCALE_NAMES: Record<"en" | TranslationLocale, string> = { en: "English", nl: "Dutch", fr: "French" };

export type TranslationStatus = "ok" | "missing" | "stale" | "manual" | "manual_stale" | "parked";
export type TranslationStatusPair = Record<TranslationLocale, TranslationStatus>;

// One locale's stored entry. `source`, `keep` and `pending` are the API's
// bookkeeping: the form sends them back untouched.
export interface TranslationEntry {
  name?: string;
  description?: string;
  label?: string;
  manual?: boolean;
  // Set by the form when the owner types: the API turns it into `manual`.
  edited?: boolean;
  source?: unknown;
  keep?: number;
  pending?: unknown;
}

export type Translations = Partial<Record<TranslationLocale, TranslationEntry>>;

export type TranslationField = "name" | "description" | "label";

/** The owner typed into a locale: it becomes theirs (Decision 5). */
export function editTranslation(
  translations: Translations,
  locale: TranslationLocale,
  field: TranslationField,
  value: string,
): Translations {
  return { ...translations, [locale]: { ...translations[locale], [field]: value, edited: true } };
}

/** Re-translate: drop the locale so the next publish (rental: save) fills it. */
export function dropTranslation(translations: Translations, locale: TranslationLocale): Translations {
  const next = { ...translations };
  delete next[locale];
  return next;
}

export interface DeeplUsage {
  count: number;
  limit: number;
}

export interface TranslationSettingsStatus {
  configured: boolean;
  auto_translate: boolean;
  stop_at_percent: number;
  keep_words: string[];
  updated_at: string;
  usage: DeeplUsage | null;
  stop_line: number | null;
  pending_rows: number;
  pending_chars: number;
  keep_change_chars: number;
}

/** Why DeepL won't fill a missing translation right now, if it won't. */
export function automaticTranslationBlocked(settings: TranslationSettingsStatus | null): string | null {
  if (!settings) return null;
  if (!settings.configured) return "Automatic translation isn't set up";
  if (!settings.auto_translate) return "Automatic translation is switched off in Settings";
  if (!settings.usage) return "DeepL can't be reached right now";
  if (settings.stop_line !== null && settings.usage.count >= settings.stop_line) {
    return "Automatic translation is paused: the allowance is nearly used";
  }
  return null;
}

export function needsTranslation(status: TranslationStatusPair | undefined): boolean {
  if (!status) return false;
  return TRANSLATION_LOCALES.some((locale) => ["missing", "stale", "parked"].includes(status[locale]));
}
