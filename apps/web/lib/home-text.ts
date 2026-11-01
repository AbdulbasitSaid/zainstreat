import { createTranslator } from "next-intl";
import { getMessages, getTranslations } from "next-intl/server";
import type { AppLocale } from "@/i18n/routing";
import type { SiteContent } from "@/lib/site-content";

type HomeTranslator = Awaited<ReturnType<typeof getTranslations<"HomePage">>>;

// `{` and `}` are ICU syntax; an override is plain text plus <em>, so quote
// them rather than let a stray brace break formatting.
function escapeIcu(text: string): string {
  return text.replace(/[{}]/g, (brace) => `'${brace}'`);
}

/**
 * The homepage's translator with the admin's per-locale overrides applied
 * (Phase 25 Decision 12). Same `t` / `t.rich` / `t.markup` signatures as
 * `getTranslations("HomePage")`; a key without an override falls back to the
 * message file. Overrides go through the same ICU parser, so only the tags
 * the page maps (`<em>`) render.
 */
export async function getHomeText(locale: AppLocale, content: SiteContent): Promise<HomeTranslator> {
  const overrides = content.home_text?.[locale];
  if (!overrides || Object.keys(overrides).length === 0) {
    return getTranslations({ locale, namespace: "HomePage" });
  }

  const messages = await getMessages({ locale });
  const defaults = messages.HomePage as Record<string, string>;
  const merged: Record<string, string> = { ...defaults };
  for (const [key, text] of Object.entries(overrides)) {
    if (text) merged[key] = escapeIcu(text);
  }

  return createTranslator({
    locale,
    messages: { HomePage: merged },
    namespace: "HomePage",
    // A malformed override shows the default rather than the raw key.
    getMessageFallback: ({ key }) => defaults[key] ?? key,
    onError: () => {},
  }) as unknown as HomeTranslator;
}
