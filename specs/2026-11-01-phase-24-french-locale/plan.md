# Phase 24 — French Locale: Implementation Plan

**Ordering:**
- The API (Groups 1–2b) and the web (Groups 3–5) are independent. Do the
  API first so `cargo test` is green before the web work.
- Run `cargo test` after Groups 2 and 2b. Run `pnpm check:messages` after Group
  4. Run `/validate` after Group 6.

Paths refer to `master` at `2607e20` (Phase 23 merged).

## Group 0 — Preconditions

Depends on: nothing.

0.1. **Phase 23 is on `master`** (PR #39). Done.
0.2. **The copy is fixed.** The email and WhatsApp text is in `copy.md`.
     There's no approval step (requirement.md Decision 2).

## Group 1 — Migration (repo)

Depends on: Group 0.

1.1. **`sqlx migrate add -r french_locale`** in `apps/api`. Up:

```sql
-- Phase 24: French as a third customer locale (requirement.md Decision 6).
ALTER TABLE orders
    DROP CONSTRAINT orders_locale_check,
    ADD CONSTRAINT orders_locale_check CHECK (locale IN ('en', 'nl', 'fr'));

ALTER TABLE catering_enquiries
    DROP CONSTRAINT catering_enquiries_locale_check,
    ADD CONSTRAINT catering_enquiries_locale_check CHECK (locale IN ('en', 'nl', 'fr'));
```

     Down: the same, back to `('en', 'nl')`. This fails if any `fr` rows
     exist, which is acceptable (requirement.md Risk 4).
1.2. Confirm both constraint names with `\d orders` and
     `\d catering_enquiries` before running it.

## Group 2 — API locale and French emails (repo)

Depends on: Group 1.

2.1. **`src/order_emails.rs`, `Locale`:**

```rust
pub enum Locale {
    En,
    Nl,
    Fr,
}

impl Locale {
    /// Every customer locale; tests loop over it so a new one can't miss copy.
    pub const ALL: [Self; 3] = [Self::En, Self::Nl, Self::Fr];

    /// Mirrors the CHECKs on `orders.locale` and `catering_enquiries.locale`.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "en" => Some(Self::En),
            "nl" => Some(Self::Nl),
            "fr" => Some(Self::Fr),
            _ => None,
        }
    }
    // as_str gains Self::Fr => "fr"
}
```

2.2. **`status_email`:**
     - add six `(Locale::Fr, …)` arms with the text in `copy.md` §2,
       each placed after its `Nl` arm
     - add `Locale::Fr => "Bonjour"` to the greeting
2.3. **`INVOICE_FR`** (`copy.md` §3) and **`QUOTE_FR`** (`copy.md` §4).
     Add the `Fr` arm to the matches in `invoice_email` and `quote_email`.
     Update the module doc comment to point at this phase's `copy.md`
     for French.
2.4. **Tests in `order_emails.rs`:**
     - `every_emailing_status_has_copy_in_both_languages` becomes
       `…_in_every_language` and loops over `Locale::ALL`
     - `french_status_email_is_french`: the `processing` subject and
       `Bonjour`
     - `french_invoice_matches_the_copy`: subject, intro and "Lieu" for
       on-site
     - `french_quote_email_matches_the_copy`: the full body, pinned to
       `copy.md` §4
     - `quote_url` and `tracking_url` with `Fr` contain `/fr/`
     - `parse("fr")` is `Some(Fr)`, `parse("de")` is `None`, and
       `as_str` round-trips for every item in `ALL`
2.5. **Integration tests:**
     - `tests/order_submission.rs`: `french_locale_is_stored` (`"fr"` is
       stored as `fr`). `unsupported_locale_is_rejected` keeps `"de"`.
     - `tests/catering_enquiries.rs`: the same for `fr`.
     - `tests/quote_acceptance.rs`: one case with an `fr` enquiry, so
       the created order is `fr`.
2.6. **Nothing else in the routes changes.** They already call
     `Locale::parse` with an `En` fallback (`routes/orders.rs`,
     `catering_enquiries.rs`, `quotes.rs`, `admin/orders.rs`,
     `admin/quotes.rs`).
2.7. `cargo sqlx prepare` only if a query changed (none should),
     `cargo test`, `cargo clippy`.

## Group 2b — One email in three languages (repo)

Depends on: Group 2. This is requirement.md Decision 11. It changes the
customer emails for every locale, not just `fr`.

2b.1. **Split each email into a section and a short subject.** In
      `order_emails.rs`:
      - `status_email`'s match → `status_section(locale, name, reference,
        status, tracking_url, contact_url) -> Option<String>` (greeting,
        message, link label, URL, no sign-off) and
        `status_subject(locale, status) -> Option<&'static str>` (`copy.md`
        §7).
      - `invoice_email`'s body → `invoice_section(locale, order,
        placed_at, tracking_url) -> String`.
      - `quote_email`'s body → `quote_section(locale, quote, url) ->
        String`. `QuoteEmail.url` goes, and each section builds its own
        URL.
      - Drop `InvoiceCopy.subject` and `QuoteCopy.subject`. Add an
        `INVOICE_SUBJECT` and a `QUOTE_SUBJECT` lookup, plus the six
        status subjects, from `copy.md` §7.
      - Every section stops appending `{SIGN_OFF}`.
2b.2. **The assembler:**

```rust
/// The customer's language first, then the rest in `Locale::ALL` order
/// (Phase 24 Decision 11).
fn ordered(primary: Locale) -> impl Iterator<Item = Locale> {
    std::iter::once(primary).chain(Locale::ALL.into_iter().filter(move |l| *l != primary))
}

/// One email in every language: a `--- {name} ---` header per section,
/// one sign-off at the end, and the short subjects joined with " / ".
fn multilingual(
    primary: Locale,
    suffix: &str,
    subject: impl Fn(Locale) -> &'static str,
    section: impl Fn(Locale) -> String,
) -> (String, String) {
    let subject = ordered(primary).map(&subject).collect::<Vec<_>>().join(" / ");
    let body = ordered(primary)
        .map(|l| format!("--- {} ---\n\n{}", l.name(), section(l)))
        .collect::<Vec<_>>()
        .join("\n\n");
    (format!("{subject} — {suffix}"), format!("{body}\n\n{SIGN_OFF}"))
}
```

      `Locale::name()` returns `English` / `Nederlands` / `Français`.
2b.3. **New public signatures.** Each section builds its own URL:

```rust
pub fn status_email(primary: Locale, customer_name: &str, reference: &str, status: &str,
                    site_url: &str, token: Uuid) -> Option<(String, String)>;
pub fn invoice_email(primary: Locale, order: &OrderResponse, placed_at: &str,
                     site_url: &str, token: Uuid) -> (String, String);
pub fn quote_email(primary: Locale, quote: &QuoteEmail, site_url: &str,
                   token: Uuid) -> (String, String);
```

      - `status_email` returns `None` when `status_subject(primary,
        status)` is `None`, so the statuses that send no email stay the
        same.
      - Call sites pass `email.site_url()` and the token instead of
        prebuilt URLs:
        - `routes/orders.rs:383-384`
        - `routes/admin/orders.rs:328-334`
        - `routes/admin/quotes.rs:376`
      - `quote_url` stays for `AdminQuoteResponse.quote_url`
        (`admin/quotes.rs:120`), which is the enquiry's own locale.
2b.4. **Tests in `order_emails.rs`.** Rewrite the single-language
      pinned tests (`english_invoice_keeps_the_phase_20_text`,
      `english_quote_email_matches_the_approved_copy`,
      `dutch_status_email_is_dutch`, …) to pin the **section** functions,
      so the per-language text is still checked word for word. Add:
      - `multilingual_email_puts_the_customers_language_first`: for each
        primary in `Locale::ALL`, the body's first header is that
        language's, and all three headers appear once.
      - `each_section_links_to_its_own_language`: the French section
        contains `/fr/order/`, the Dutch one `/nl/order/`, and so on.
      - `sign_off_appears_once`.
      - `subject_joins_all_three_in_body_order`: the `fr` primary gives
        `Paiement confirmé / Payment confirmed / Betaling bevestigd —
        ZTM-1042`.
      - `statuses_without_email_still_send_none` (`awaiting_payment`,
        `payment_claimed`).
2b.5. **Integration tests** that read sent email subjects or bodies, if
      any: update them to the joined subject.
      `grep -rn "subject\|Your invoice\|Je factuur" tests/`.
2b.6. **Sender name.** In `email.rs`, a `SENDER_NAME` constant and a
      `sender()` helper build `Mailbox::new(Some(name), address)`.
      `EMAIL_FROM_ADDRESS` stays a bare address, so there's no env
      change. Unit-test the parse. Note it in `docs/deployment.md`.

## Group 3 — Web routing, switcher, WhatsApp (repo)

Depends on: Group 0.

3.1. **`i18n/routing.ts`:**

```ts
import { defineRouting } from "next-intl/routing";

export const routing = defineRouting({
  locales: ["en", "nl", "fr"],
  defaultLocale: "en",
  // hreflang comes from page metadata and the sitemap (lib/seo.ts) instead;
  // the middleware's Link header would add a second x-default pointing at "/".
  alternateLinks: false,
});

export type AppLocale = (typeof routing.locales)[number];
```

3.2. **`components/locale-toggle.tsx`:**

```ts
// Each language in its own name, so visitors can find theirs whatever page
// language they're on.
const LOCALE_NAMES: Record<AppLocale, string> = {
  en: "English",
  nl: "Nederlands",
  fr: "Français",
};
```

     The option label becomes `{LOCALE_NAMES[l]}`.
3.3. **`lib/quotes.ts`:**
     - drop `ShareLocale` and key `SHARE_MESSAGES` by `AppLocale`
     - add the `fr` message from `copy.md` §5
     - `quoteShareMessage` picks
       `SHARE_MESSAGES[hasLocale(routing.locales, locale) ? locale : "en"]`
       (`hasLocale` from `next-intl`)
     - point the comment at both phases' `copy.md`
3.4. **Check for other two-locale assumptions:**

```sh
grep -rnE '"nl"|"en"' apps/web --include=*.ts --include=*.tsx \
  | grep -v node_modules
```

     Expect only `routing.ts`, `locale-toggle.tsx` and `lib/quotes.ts`,
     plus the deliberate `lang="en"` in `app/global-error.tsx` and
     `app/admin/layout.tsx`.

## Group 4 — French messages and parity check (repo)

Depends on: Group 3.

4.1. **`messages/fr.json`:** a full translation of `en.json`, with the
     same key order and every leaf key, in the style of requirement.md
     Decision 3:
     - Keep every `{placeholder}` and ICU `plural`/`select` structure
       exactly. Translate only the text inside the branches.
     - `LocaleToggle.label`: `Langue`.
     - `Whatsapp` and the other prefilled WhatsApp texts: French, so a
       French visitor writes to the owner in French.
     - `TermsPage` and `FoodRegulationsPage`: a faithful translation of
       the English. Don't add or drop clauses.
     - `Quote` and the on-site `Order` keys use the same words as
       `copy.md`: "devis", "Service traiteur en cours" for
       `serving_on_site`, "Lieu".
4.2. **`apps/web/scripts/check-messages.mjs`** (Decision 9). Plain Node
     ESM with no dependencies:
     - Read every `messages/*.json`, and use `en.json` as the reference.
     - Flatten each file to dotted leaf keys.
     - For each locale, report missing keys, extra keys, and keys whose
       placeholder argument names differ. Collect the names with a small
       brace scan that takes the identifier after `{`, so the `#` and
       branch bodies inside ICU are ignored.
     - Print a per-file summary. Exit 1 on any problem, otherwise print
       `messages OK: en, fr, nl (331 keys)`.
4.3. **`apps/web/package.json`:** add
     `"check:messages": "node scripts/check-messages.mjs"`.
4.4. **`.claude/skills/validate/SKILL.md`**, step 2 (web checks): add
     `pnpm check:messages` before `pnpm lint`.

## Group 5 — hreflang (repo)

Depends on: Group 3.

5.1. **New `apps/web/lib/seo.ts`:**

```ts
import type { Metadata } from "next";
import { routing } from "@/i18n/routing";

// Canonical + hreflang for a public page, relative to the layout's metadataBase.
// `path` is "" for home, else "/about" etc.
export function localeAlternates(locale: string, path: string): Metadata["alternates"] {
  return {
    canonical: `/${locale}${path}`,
    languages: {
      ...Object.fromEntries(routing.locales.map((l) => [l, `/${l}${path}`])),
      "x-default": `/${routing.defaultLocale}${path}`,
    },
  };
}
```

5.2. **The seven indexable pages** add
     `alternates: localeAlternates(locale, "<path>")` to their
     `generateMetadata` return:
     - `app/[locale]/page.tsx` (`""`)
     - `about`, `services`, `menu`, `contact`, `terms`, `food-regulations`
     Leave out `cart`, `order` (checkout), `order/[token]` and
     `quote/[token]`.
5.3. **`app/sitemap.ts`:** each entry gains
     `alternates: { languages: { en, nl, fr, "x-default" } }` with
     absolute URLs (`${base}/${l}${route}`). Keep `ROUTES` as the one
     list. Optionally export it from `lib/seo.ts` so 5.2 and 5.3 can't
     drift.

## Group 6 — Validation (repo)

Depends on: everything above.

6.1. `cargo test`, `cargo clippy`, `pnpm check:messages`, `pnpm lint`,
     `pnpm build`, `/validate`. Tick `validation.md`.
6.2. **Live checks on the dev stack** (validation.md, Web):
     - redirect for `Accept-Language: fr`
     - `/fr` pages
     - switcher
     - `<link rel="alternate">` tags and the sitemap
     - a French order and catering enquiry end to end, with the emails'
       text checked through the API's email log or tests
6.3. **Layout at phone width** (Risk 3): `/fr` home, header and mobile
     menu, menu, cart, checkout, order tracking stepper, quote page and
     contact forms, at 375px in Chrome. Shorten any French string that
     overflows, in `fr.json`, and in `copy.md` if it's an email string.
6.4. **Docs:** `docs/local-development.md` lists `/fr` and the three
     message files, plus `pnpm check:messages`.

## Implementation notes (2026-11-01)

- **Sender name in code (2b.6).**
  - The plan put `Zain's Treat n More <address>` in `EMAIL_FROM_ADDRESS`.
    But `_deploy.yml`'s env renderer rejects any value containing `'`,
    so that would fail the deploy.
  - Instead, `email.rs` has `SENDER_NAME` and `sender()`, which wrap the
    bare address in a named `Mailbox`. There's no env change.
  - Owner emails get the same sender name.
  - Requirement Decision 11 is updated to match.
- **Email code shape.**
  - `status_text(locale, status, reference, contact_url)` holds the
    per-language match and returns `(short subject, message, link
    label)`. `status_section` adds the greeting and the tracking URL.
  - `Locale` gained `name()` and a private `greeting()`.
  - `multilingual` takes ready-made `(locale, subject, section)` parts
    instead of two closures.
  - `invoice_copy` and `quote_copy` replace the inline matches.
    `InvoiceCopy.subject` and `QuoteCopy.subject` stay, now holding the
    short subject.
- **`AdminQuoteResponse.public_token`** is a `#[serde(skip)]` field. The
  send handler uses it to build every language's quote link, and it isn't
  serialized.
- **`lib/seo.ts`** exports `PUBLIC_ROUTES` (the sitemap uses it too, so
  the two can't drift) and `languageAlternates(path, base)`, shared by
  the metadata and the sitemap.
- **`check-messages.mjs`** compares `<tags>` (the `<em>` rich text) as
  well as `{placeholders}`.
- **Build.** The `[locale]` pages render on demand (`ƒ`), so nothing is
  prerendered per locale. The live `200` checks on every `/fr` page
  cover it instead.
- **Dev stack.** Restarting `api` and `web` together got the API
  container's linker OOM-killed (`ld terminated with signal 9`).
  Restarting `api` on its own worked. That's Docker memory, not the
  code.
- **Results.**
  - `cargo test`: 210 passed, 0 failed (194 before).
  - Clippy is clean, and the offline build passes.
  - `.sqlx` needed no re-prepare: no query macros changed, and the new
    test queries are unchecked `sqlx::query`.
  - `pnpm check:messages`, `pnpm lint` and `pnpm build` pass.
  - The live smoke test on the dev stack passed.
  - The smoke rows and the temporary admin were deleted afterwards.

