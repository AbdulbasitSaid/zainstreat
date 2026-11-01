# Phase 24 — French Locale: Requirements

## Source

`specs/roadmap.md`, Phase 24:

> - Add `fr` as a third public locale alongside `en`/`nl` (`next-intl`
>   routing, `messages/fr.json`, language toggle, sitemap,
>   `Accept-Language` detection). Admin and the API stay English-only.
> - Translate the customer emails into French as well.

Phase 23's requirement.md, "Out of scope": "Phase 24 adds the `fr` arm
to `quote_email` and the `Quote` messages."

## Context

### What exists

- **Routing (Phase 3).** `apps/web/i18n/routing.ts` is the only list of
  locales (`["en", "nl"]`, default `en`). Several things already follow
  it, so they need no change:
  - `proxy.ts` uses next-intl's default `Accept-Language` detection and
    the `NEXT_LOCALE` cookie
  - `i18n/request.ts` loads `messages/${locale}.json`
  - `[locale]/layout.tsx` builds its static params and 404s an unknown
    locale
  - `sitemap.ts` loops over `routing.locales`
- **Messages.** `messages/en.json` and `nl.json` have 331 leaf keys in
  15 namespaces, and their keys match exactly. Nothing checks that.
  Terms and Food Regulations are in the message files too; there's no
  MDX.
- **Hard-coded to two locales on the web side:**
  - `components/locale-toggle.tsx:23` labels options with
    `l === "en" ? "English" : "Nederlands"`. French would show as
    "Nederlands".
  - `lib/quotes.ts:69-94` has `ShareLocale = "en" | "nl"`, and anything
    else falls back to English.
- **Formatting.**
  - `lib/format.ts` formats prices with `nl-NL` (`€12,50`) in every
    locale.
  - Customer dates use `Intl.DateTimeFormat(locale)` and so follow the
    locale already: `order-invoice.tsx`, `order-progress.tsx` and the
    quote page.
- **SEO.**
  - No page sets `alternates` (hreflang).
  - next-intl's middleware sends hreflang `Link` headers by default
    (`alternateLinks`). Its `x-default` points to the unprefixed `/`.
- **API.**
  - `order_emails.rs` has `enum Locale { En, Nl }`. `parse` gates the
    `locale` sent by the order and catering enquiry forms.
  - `status_email`, `invoice_email` and `quote_email` branch on it.
    `status_email` ends in `_ => None`, so a missing French arm would
    send no email and nothing would fail.
  - DB CHECKs `orders_locale_check` (migration `20261031120000`) and
    `catering_enquiries_locale_check` (`20261101132739`) allow
    `('en', 'nl')` only.
  - Owner emails are English.
- **Email sending.** Each customer email is sent once, in the
  customer's locale only. `EMAIL_FROM_ADDRESS` is a bare address with no
  display name, so the brand shows only in the subject.
- **Menu content** (category and item names and descriptions) has no
  per-locale columns. It's shown as typed in every locale.

## Decisions

1. **`fr` joins `routing.locales`; `en` stays the default.** next-intl's
   default detection sends a first-time visitor whose browser prefers
   French (`fr`, `fr-BE`, `fr-FR` and so on) to `/fr`. The `NEXT_LOCALE`
   cookie still overrides it. No region variants, as in Phase 3
   Decision 7.

2. **Claude writes the French copy, with no approval step** (asked
   during planning: "no need for approval step just do it").
   - `messages/fr.json` is translated from `en.json` during
     implementation.
   - The email and WhatsApp copy is fixed now, in `copy.md` in this
     folder, and the code uses it word for word.
   - No client, user or native-speaker review gate.

3. **French style.**
   - Formal `vous`, the norm for a French-speaking business. Dutch
     stays informal `je`.
   - The brand name, dish names and `ZTM-N` references stay as they are.
   - Sentences use French spacing before `:`, `!`, `?` and `;`, with a
     plain space and no special characters.
   - Lines that code builds as `{label}: {value}` (for example
     `Commande: ZTM-12` in the invoice) keep the shared format string's
     `: `. The shared format strings aren't split per locale.
   - Amounts keep `€{amount}`, and dates keep `DD-MM-YYYY` in emails.

4. **Formatting doesn't change.** `formatPrice` keeps `nl-NL` (`€12,50`)
   for every locale. The business is Dutch, and French uses a decimal
   comma too. Customer dates already follow the locale, so `/fr` shows
   "5 déc. 2026". Admin stays `en-GB`.

5. **Language switcher labels come from a typed map.** `i18n/routing.ts`
   exports `type AppLocale = (typeof routing.locales)[number]`. The toggle
   uses `LOCALE_NAMES: Record<AppLocale, string>` (`English`,
   `Nederlands`, `Français`): each language in its own name, the same in
   every locale, so a visitor can find their own. A future locale fails
   to compile until it has a name. It stays a `<select>`.

6. **API: `Locale::Fr`, and the two CHECKs widened.**
   - A new migration widens `orders_locale_check` and
     `catering_enquiries_locale_check` to `('en', 'nl', 'fr')`.
   - `Locale` gains `Fr` and `Locale::ALL`. `parse("fr")` →
     `Some(Fr)`, and `de` is still `400` with `field: "locale"`.
   - `status_email` gains six `Fr` arms and the greeting "Bonjour".
     `INVOICE_FR` and `QUOTE_FR` are added, and the exhaustive matches
     in `invoice_email` and `quote_email` gain the `Fr` arm.
   - The "every emailing status has copy" test loops over `Locale::ALL`.
     That catches the silent `_ => None` gap for any future locale.
   - Owner emails and API errors stay English.

7. **Admin's WhatsApp quote message gets French** (Phase 23 Decision
   14). `lib/quotes.ts` keys `SHARE_MESSAGES` by `AppLocale` instead of
   `"en" | "nl"`, and falls back to `en` only for a value outside
   `routing.locales`, checked with `hasLocale`. Admin itself stays
   English.

8. **hreflang alternates in metadata and the sitemap** (asked during
   planning: add now).
   - A new `lib/seo.ts` has `localeAlternates(locale, path)`. It returns
     `{ canonical, languages }` with one entry per locale plus
     `x-default` → `/en{path}`.
   - The seven sitemap pages spread it into their `generateMetadata`:
     home, about, services, menu, contact, terms and food regulations.
     Paths are relative and resolve against the layout's `metadataBase`.
   - `sitemap.ts` adds the same `alternates.languages` to each entry.
   - Cart, checkout and the `noindex` token pages (order and quote) get
     no alternates.
   - `routing` sets `alternateLinks: false`. The middleware's `Link`
     header would otherwise give a second, different `x-default` (`/`),
     so metadata and the sitemap become the only hreflang source.

9. **Message-key parity check** (asked during planning: yes, a small
   Node script).
   - `apps/web/scripts/check-messages.mjs`, plain Node with no
     dependency, run as `pnpm check:messages`.
   - It compares every `messages/*.json` with `en.json`. It fails on a
     missing or extra leaf key, and on a value whose `{placeholders}`
     differ from English. ICU plural and select branches are compared by
     the argument names only.
   - The `/validate` skill runs it as part of its web checks.

10. **No new technology.** `tech-stack.md` only changes its i18n text
    and summary row to include `/fr`.

11. **One customer email in all three languages** (asked after planning:
    "instead of sending 3 seperate language email, lets send 1 with the 3
    langueges"). This replaces the one-language emails from Phases
    20, 22 and 23 for every customer email: the invoice, the status emails
    and the quote email.
    - **Body.** One section per language, the customer's stored locale
      first (asked: customer's language first), then the other two in
      `Locale::ALL` order (`en`, `nl`, `fr`).
    - Each section starts with a header line in that language's own name
      (`--- English ---`, `--- Nederlands ---`, `--- Français ---`, the
      same names as the switcher).
    - Each section is that language's complete text, so it reads on its
      own. Its links go to the page in that section's language:
      `/fr/order/{token}` in the French part, `/nl/…` in the Dutch part.
    - `— Zain's Treat n More` appears once, at the very end.
    - **Subject.** All three, joined (asked). Each language gets a short
      subject from `copy.md` §7, joined with ` / ` in the same order as
      the body, then ` — {reference}`. For example: `Paiement confirmé /
      Payment confirmed / Betaling bevestigd — ZTM-12`. The quote uses
      `#{id}`.
    - **The brand moves to the sender name.** It no longer fits in the
      subject. The API sends from `Zain's Treat n More
      <{EMAIL_FROM_ADDRESS}>`, so the inbox still shows the brand. The
      name is a constant in `email.rs`, and `EMAIL_FROM_ADDRESS` stays a
      bare address. *(Changed during implementation: the plan was to
      put the name in the env value, but `_deploy.yml` rejects any value
      containing `'`.)*
    - **The stored locale still matters.** It picks the first section and
      the language of the tracking and quote pages the customer is sent
      to from the site.
    - **Unchanged:**
      - owner emails stay English
      - the admin's WhatsApp quote message stays in the enquiry's
        language only (Decision 7), since a chat message in three
        languages reads badly
      - pages stay single-language
    - **Code shape.**
      - Each email gets a per-locale section function (`status_section`,
        `invoice_section`, `quote_section`) returning the body without
        the sign-off.
      - Each email gets a per-locale short-subject function.
      - One helper, `multilingual(primary, |locale| …)`, orders, labels
        and joins the sections, adds the sign-off, and joins the subjects.
      - The public `status_email`, `invoice_email` and `quote_email` keep
        a primary `Locale`. They take `site_url` and the token instead of
        prebuilt URLs, so each section can build its own.
    - **Rejected:**
      - Shared parts written once (the item list and bank details once,
        only the prose in three languages). It's shorter, but the
        sections no longer read on their own and the layout logic gets
        harder. Revisit if the length is a problem (Risk 5).
      - One email per language (three emails), which the user explicitly
        didn't want.

## Out of scope

- French menu content: menu names and descriptions stay as typed.
  Per-locale menu fields would be their own phase.
- French admin or API error messages. Admin stays English (Phase 3
  Decision 4).
- `openGraph.locale` and other SEO work beyond hreflang: **Phase 26**.
  A page-level `openGraph` would replace the layout's `siteName` and
  `images`, so it needs its own change.
- Phase 25's admin-editable homepage text has to handle three locales.
  That's for Phase 25 to plan.
- Changing the price format per locale (Decision 4).

## Open risks flagged during planning

1. **Machine-quality French.** The copy is Claude's, with no native
   review (Decision 2). Awkward wording on the public site or in emails
   is possible. All French text is in `fr.json` and `copy.md`, so a
   later correction is a text-only change.
2. **Silent missing email.** `status_email`'s `_ => None` means a missing
   `Fr` arm would skip the email without an error. Decision 6's test
   over `Locale::ALL` covers it.
3. **Longer French strings.** French runs about 15–20% longer than
   English. Buttons, the header nav, the mobile menu and the order
   stepper could wrap or overflow. Plan Group 6 checks them at phone
   width.
4. **Down migration.** Reverting the CHECK fails once any `fr` rows
   exist. This is acceptable: a down migration is for dev databases, as
   in Phase 23.
5. **Long emails** (Decision 11). The invoice and quote repeat the item
   list and bank details three times. Putting the customer's language
   first keeps the part they need at the top. If customers find it
   confusing, the fallback is the rejected "shared parts once" layout.
6. **Long subjects** (Decision 11). Three joined subjects are about 60–80
   characters, and phones cut them off. The customer's language comes
   first, so the visible part is theirs. That's why the brand moves to
   the sender name.
7. **Spam scoring.** A longer multi-language plain-text body is a small
   change for SES. Check that staging test emails still land in the
   inbox.
