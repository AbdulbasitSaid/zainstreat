# Phase 24 — French Locale: Validation

Checked on 2026-11-01 unless noted. Where the implementation deviated from
`plan.md`, see its "Implementation notes" section.

## Pass/fail checklist

### Migration (Group 1)

- [x] `sqlx migrate run`, `sqlx migrate revert`, then `sqlx migrate
      run` succeeds on a fresh dev DB.
      *(Run on the existing dev DB, not a fresh one.)*
- [x] `INSERT … locale = 'fr'` succeeds on `orders` and
      `catering_enquiries`. `locale = 'de'` fails on
      `orders_locale_check` and `catering_enquiries_locale_check`.
      *(Checked with `UPDATE`s in a rolled-back transaction, and `fr`
      through the API.)*
- [x] `SQLX_OFFLINE=true cargo build --release` succeeds.

### API (Group 2)

- [x] `cargo test order_emails` passes:
      - every emailing status has copy for every item in `Locale::ALL`
      - the French quote email matches `copy.md` §4 in full
      - French invoice and status subjects match `copy.md`
      - `/fr/` URLs
      - `parse`/`as_str` round-trip
- [x] `cargo test --test order_submission` and `--test
      catering_enquiries`: `fr` is stored, a missing locale gives `en`,
      and `de` gives `400` with `field: "locale"`.
- [x] `cargo test --test quote_acceptance`: accepting a quote on an
      `fr` enquiry creates an `fr` order.
- [x] `cargo test` (all) and `cargo clippy` pass.
      *(210 passed, 0 failed, up from 194. Clippy clean.)*

### One email in three languages (Group 2b)

- [x] `cargo test order_emails` passes:
      - each language's section text is pinned word for word (`en`/`nl`
        as before, `fr` per `copy.md`)
      - for every primary locale, its section comes first and all three
        `--- {name} ---` headers appear once
      - each section links to its own language's page
      - the sign-off appears once, at the end
      - the subject joins all three short subjects in body order, then
        ` — {reference}`
      - `awaiting_payment` and `payment_claimed` still send nothing
- [x] `email.rs` builds the sender `Zain's Treat n More
      <no-reply@zainstreat.com>` from a bare address, and rejects a
      malformed one. *(The name moved into code; see plan.md's
      implementation notes.)*
- [ ] On staging, with SES (or the API log with email off), one email
      per event, not three:
      - an `nl` order's invoice starts with the Dutch section and its
        subject starts `Factuur / Invoice / Facture — ZTM-…`
      - the sender shows as "Zain's Treat n More"
      - the email lands in the inbox, not spam (Risk 7)
      *(Not checked yet. Local dev has email off, so the API skipped the
      sends. The text is covered by the unit tests. Needs a staging
      deploy.)*
      *(Staging at `0b7f7c1`, SES on: placed `nl` order ZTM-6 and `fr`
      order ZTM-7 to `techsihir@gmail.com`. The inbox check is still
      open.)*

### Web (Groups 3–5)

- [x] `pnpm check:messages` prints `messages OK` for `en`, `fr` and `nl`.
      Deleting a key from `fr.json`, or renaming a `{placeholder}`, makes
      it exit 1. Revert that change afterwards.
      *(A deleted key, a renamed `{placeholder}`, a dropped `<em>` and an
      extra key were each reported, exit 1. Then reverted.)*
- [x] `pnpm lint` and `pnpm build` pass, with no `any`. *(The
      `[locale]` pages render on demand, so nothing is prerendered per
      locale. The live checks below cover `/fr`.)*
- [x] `curl -sI -H 'Accept-Language: fr-BE,fr;q=0.9' localhost:3000/`
      redirects to `/fr`. With `Cookie: NEXT_LOCALE=nl`, it redirects to
      `/nl`.
- [x] `curl -s localhost:3000/fr` → `200` with `<html lang="fr">` and
      French text. The same for `/fr/about`, `/fr/services`, `/fr/menu`,
      `/fr/contact`, `/fr/terms` and `/fr/food-regulations`.
      *(All seven routes return `200` in `en`, `nl` and `fr`.)*
- [x] The switcher shows English / Nederlands / Français on every
      locale, and switching keeps the current path.
      *(Checked in a browser on staging: `/fr/menu` → `/nl/menu` and
      `/en/contact` → `/fr/contact`. After choosing `nl`, `/` with
      `Accept-Language: fr` still went to `/nl`, so the choice sticks.)*
- [x] `curl -s localhost:3000/fr/about` contains `<link
      rel="canonical" href="…/fr/about">` and `<link rel="alternate"
      hreflang=…>` for `en`, `nl`, `fr` and `x-default` (→ `/en/about`).
      `/fr/cart` and `/fr/order/<token>` have no hreflang alternates.
      *(`/fr/cart` has none. `/fr/order/<token>` has no metadata
      alternates, since it isn't in `PUBLIC_ROUTES`.)*
- [x] `curl -sI localhost:3000/fr` has no hreflang `Link` header
      (`alternateLinks: false`).
      *(The only `Link` header is the font/CSS preload.)*
- [x] `curl -s localhost:3000/sitemap.xml` lists the 21 URLs (7 × 3),
      each with `xhtml:link` alternates for the three locales and
      `x-default`.
      *(21 `<loc>`, 84 `xhtml:link`.)*
- [x] The grep in plan 3.4 finds no other two-locale assumptions.

### End to end, French (dev stack)

- [x] A cart order placed on `/fr` stores `locale = 'fr'`. Its tracking
      page `/fr/order/<token>` is fully French, with French dates and the
      stepper labels.
      *(Placed through the `/api/orders` proxy with `locale: "fr"`. Stored
      `fr`. Its tracking page had `lang="fr"`, "Commande n°", "Commande
      passée", "Paiement envoyé", "Comment payer" and "J'ai payé".)*
- [ ] A catering enquiry submitted on `/fr/contact` stores `locale =
      'fr'`. In admin, its quote's "Send via WhatsApp" message is the
      `copy.md` §5 text, and `/fr/quote/<token>` is fully French.
      Accepting it leads to the French order page.
      *(Mostly checked:
      - the enquiry (via the proxy, `locale: "fr"`) stored `fr`
      - the quote was saved on-site and sent, and `quote_url` was
        `/fr/quote/…`
      - the page was French ("Devis n°", "Accepter le devis", "Lieu :
        …")
      - accept created order `fr`/`on_site`
      Not checked: the WhatsApp message in the admin UI. That's a manual
      browser check.)*
- [ ] With SES configured (staging), the invoice, a status email and the
      quote email arrive in French. *(Or, with email off, confirm the
      text through the unit tests above.)*
      *(Email off locally: the text is covered by the unit tests. Staging
      is open.)*

### Layout (Group 6.3)

- [ ] At 375px wide, these have no overflow or clipped buttons:
      - `/fr` home, header and mobile menu
      - menu
      - cart and checkout
      - order tracking stepper
      - quote page
      - contact forms
      *(Checked on staging in a 375px same-origin iframe. No overflow on:
      - `/fr` with the mobile menu open
      - menu
      - `/fr/order` (checkout)
      - the tracking stepper
      - contact, services and about

      **Fails on cart:** with an item in it, `/fr/cart` is 414px wide and
      "Retirer" is off-screen. It isn't French-only: `/en/cart` is 421px
      and `/nl/cart` is 444px. `components/cart-line-item.tsx:22` is a
      single non-wrapping flex row. The quote page isn't checked yet; it
      needs a sent quote.)*

### Sweep

- [x] `/validate` passes, including `pnpm check:messages`.
      *(check:messages, lint, build, cargo check/clippy/build, and the
      live smoke test all pass. The smoke rows and the temporary admin
      were deleted.)*
- [x] `docs/local-development.md` and `specs/tech-stack.md` mention
      `/fr`.

## Definition of done

The public site works in French as well as English and Dutch. A visitor
whose browser prefers French lands on `/fr`. The switcher lists each
language in its own name, and the choice sticks. Every page, the order
tracking page and the quote page are in French. Orders and catering
enquiries placed in French store `fr`. Every customer email (invoice,
status emails, quote) goes out once, carrying all three languages. The
customer's own language comes first, each part links to the page in its
language, and the subject joins the three short subjects. The sender
shows as "Zain's Treat n More". The admin's WhatsApp quote message uses
the French text in `copy.md` for French enquiries. The seven public pages and the sitemap
carry hreflang alternates for the three locales plus `x-default`.
`pnpm check:messages` keeps the three message files in step and runs in
`/validate`. Admin and the API's own messages stay English. There's no
new technology, and `cargo test`, `cargo clippy`, lint, build and
`/validate` pass.
