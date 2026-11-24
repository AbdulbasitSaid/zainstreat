# Phase 38 — Menu Translations & Event Attendee Export: Requirements

## Source

A new phase with no earlier roadmap entry. Request (2026-11-24):

> we need to do the following:
>   - fix menu item language change ( its currently all english ). it
>     needs to be trasnlated
>   - export event attendees after the registration is closed

It reverses Phase 24's "French menu content: menu names and descriptions
stay as typed" and Phase 36's "Translating menu or rental item names" (Out
of scope in both).

Answers to the planning questions (2026-11-24):

- **Who writes nl/fr:** auto-translate with the **DeepL API**; the owner
  can override.
- **Scope:** everything customers see — item names and descriptions,
  category names and descriptions, price-option labels, rental names and
  descriptions.
- **Order lines:** in the customer's language on their invoice, emails and
  tracking page; English for admin and the kitchen.
- **Attendee export:** all four — a final list once bookings close,
  confirmed guests one per row, emailed to the owner, and a printable
  check-in sheet. *"The PDF version should have the website design and
  feel."*
- **PDF:** a branded print page saved with the browser's "Save as PDF",
  not a server-generated file.

## Context

### Why now

Phase 24 added `/fr` and Phase 3 `/nl`. All page copy is translated, but
the menu itself (the main thing customers read) is English on every
locale. Phase 31 events ran their first booking round, and the owner wants
a final attendee list to work from on the day.

### What exists

- **Menu storage.** `menu_items.name/description`,
  `categories.name/description`, `menu_item_price_options.label`,
  `rentals.name/description`: one language, as typed. Menu items and
  categories have Phase 25 drafts (`draft JSONB`, publish workflow,
  `routes/admin/drafts.rs`); rentals save live (Phase 36).
- **Per-locale precedent.** Events and plates store
  `content JSONB {"en": {...}, "nl": {...}, "fr": {...}}`, English
  required, nl/fr fall back to English via `events::localized`
  (Phase 31 Decision 15).
- **Public API.** `GET /api/menu-items`, `/api/categories`,
  `/api/rentals` (and `/api/admin/preview/*` for drafts) take no locale.
  `apps/web/lib/api.ts` fetches them with `cache: "no-store"`.
- **Cart.** `CartLine` (`lib/cart-context.tsx`, localStorage
  `zainstreat:cart:v1`) stores `name` and `optionLabel` as they were when
  added.
- **Orders.** `POST /api/orders` already carries `locale` and snapshots
  `order_items.item_name` / `option_label` from the database (English).
- **Outbound HTTP.** `reqwest` (rustls) is already a dependency; Phase 37
  set the pattern for a third-party key (`MAPBOX_ACCESS_TOKEN`: env,
  `docker-compose.yml`, `.env.example`, `_deploy.yml`, `docs/deployment.md`).
- **Guest list.** Phase 31 Decision 18: `GET
  /api/admin/events/{id}/bookings.csv` ("Download guest list (CSV)" on the
  Bookings tab), available at any time, one row per *booking*, every
  status including waitlisted and cancelled.
- **When bookings close.** `now() >= LEAST(meal_orders_close_at,
  rsvp_closes_at, starts_at)` (`routes/events.rs`, `admin/events.rs`).
  `LEAST` ignores nulls, so `starts_at` always bounds it.
- **Background work.** `events::spawn_hold_sweeper` runs every five
  minutes. Owner emails go to `EmailConfig::business_notify_address()`.
  `OutgoingEmail` is plain text with no attachments.

## Decisions

### Menu translations

1. **Translations are stored next to the English columns.** English stays
   in the existing columns (no change to any reader that doesn't care).
   `menu_items`, `categories`, `menu_item_price_options` and `rentals` each
   get `translations JSONB NOT NULL DEFAULT '{}'`:

   ```json
   {"nl": {"name": "…", "description": "…",
           "source": {"name": "<English name>", "description": "<English description>"},
           "manual": false},
    "fr": {…}}
   ```

   Price options use `label` instead of `name`/`description`. `source` is
   the English text the translation was made from (short, so stored as-is
   rather than hashed), so a stale translation is detectable by comparing
   it with the current English. For menu items and categories the draft
   (Phase 25) carries its own `translations`, published with the rest of
   the draft. Rentals save live.
2. **DeepL translates on publish, only when English text changes.** Menu
   item and category forms autosave a draft 700 ms after each pause in
   typing (Phase 25), so autosaves never call DeepL. They only keep the
   owner's own nl/fr edits. **Publish** translates what the item or
   category is missing (its draft if it has one), just before it goes
   live: one DeepL round per published change. Rentals have an explicit
   Save button and translate on save. *(Changed during implementation,
   2026-11-24: translating on every autosave would spend the allowance on
   half-typed text.)* Either way the API calls DeepL for that one item
   only, and only for English name, description or option labels that
   changed (or nl/fr that are missing). Saves that change
   only price, photo, availability, featured flag, order, category or
   minimum quantity send nothing to DeepL. Every call first passes the
   budget guard (Decision 20). The API calls DeepL `POST /v2/translate` (`source_lang=EN`,
   `target_lang=NL` / `FR`, `formality=prefer_more` for French,
   `tag_handling=xml`, `ignore_tags=keep`) for every non-`manual` locale,
   in one request per locale with all of the item's strings (option
   labels included). 5 s timeout. The key is env `DEEPL_API_KEY`; a key
   ending in `:fx` uses `api-free.deepl.com`, otherwise `api.deepl.com`.
   Only menu text is sent, never customer data.
3. **The save never waits on DeepL failing.** If DeepL is down,
   unconfigured or over quota, the save succeeds with nl/fr unchanged (or
   missing), and the admin form shows "Not translated yet — we'll retry
   automatically." A **translation sweep** (Decision 4) fills the gap.
4. **A sweep backfills and retries.** Every five minutes (on the existing
   sweeper's schedule, separate task) and only when `DEEPL_API_KEY` is
   set and automatic translation is on (Decision 20), the API finds rows
   (published or draft) whose nl or fr is missing, or not `manual` with a
   `source` that no longer matches the English, and translates them,
   within the per-run cap and the budget guard. Only live, published rows
   and rentals are swept. Drafts and unpublished rows are translated on
   Publish instead, because an open admin form would overwrite what the
   sweep wrote, and the publish would then pay DeepL again. This is how today's
   menu, categories, options and rentals get translated once after deploy
   (about 15,000 characters at today's size): no data migration, no
   manual step. Afterwards it only picks up saves that DeepL missed.
   Logged with row kind and id only.
5. **The owner can edit every translation.** The admin forms for menu items,
   categories and rentals get **English / Nederlands / Français** tabs
   (the event form's pattern). English is required as today. The nl/fr
   fields are always editable, whether automatic translation is on, off,
   out of allowance or failing. Left blank, the site shows English. nl/fr
   fields show DeepL's text, labelled "Translated automatically". Editing one sets
   that locale's `manual: true`, and DeepL never overwrites it. When the
   English changes later, a manual locale shows "English changed since
   this was edited" with a **Re-translate** button. It drops that
   locale's text, and the next publish (or rental save) translates it
   again. Admin copy is English only (as before).
6. **Dish names stay as they are.** A "Words to keep" list on
   `/admin/settings` (one per line, e.g. Jollof, Puff puff, Suya, Chin
   chin, Abula, Egusi, Small chops) is stored in a single-row
   `translation_settings` table. Before sending, the API wraps each
   case-insensitive whole-word match in `<keep>…</keep>`, so DeepL leaves
   it untouched, then strips the tags. Changing the list re-queues every
   non-manual translation for the sweep (clears `source`). The migration
   seeds the list with the African dish names in the current menu.
7. **Public responses are localized.** `GET /api/menu-items`,
   `/api/categories`, `/api/rentals` and the preview equivalents take
   `?locale=en|nl|fr` (unknown or missing → `en`). `name`, `description`
   and option `label` are resolved for that locale, falling back to
   English per field when blank or missing. The responses also gain
   `names: {en, nl, fr}` (items and rentals) and option
   `labels: {en, nl, fr}` (fallback already applied), so the cart can
   switch language without refetching. `apps/web/lib/api.ts` passes the
   page's locale.
8. **The cart follows the language.** `CartLine` gains
   `names?: Record<Locale, string>` and `optionLabels?: Record<Locale,
   string> | null`. The cart, checkout summary and add-to-cart feedback
   show `names[locale] ?? name`. Carts saved before this phase have no
   `names` and show the stored `name` until the item is added again.
9. **Orders snapshot both languages.** `order_items` gains
   `item_name_local TEXT` and `option_label_local TEXT`, filled from the
   database for the order's `locale` (null for `en`, or when the
   translation is missing, which means "same as English").
   Customer-facing renders (invoice and status emails, success page,
   tracking page, payment-claimed page) show `item_name_local ??
   item_name`. Admin order pages, owner emails and push notifications keep
   `item_name` (English) so the kitchen reads one language. Old orders
   are unchanged (historical accuracy rule).
10. **Search and slugs are untouched.** Menu filtering is by category id;
    nothing keys on names.
11. **Cost.** DeepL API Free, no card and no paid subscription. The user
    confirmed (2026-11-24) that it allows commercial use, with a
    1,000,000-character allowance. DeepL counts the English characters
    sent, once per target language. Today's menu is about 7,600 characters
    (seed data), so a full nl+fr pass is about 15,000 and one item edit
    about 400. The menu's growth is unpredictable, so the limit is enforced
    in code (Decision 20), not by estimate.

### Event attendee export

12. **"Registration closed" is time-based.** An event's bookings are
    closed when `now() >= LEAST(meal_orders_close_at, rsvp_closes_at,
    starts_at)`, the same rule that stops new bookings. A full event isn't
    closed (the waitlist can still move).
13. **The attendee list is confirmed guests, one row per person.** A new
    `GET /api/admin/events/{id}/attendees` returns the event header
    (title, date, area, confirmed party count) and every guest of every
    `confirmed` booking, ordered by booker name then guest position:
    booker name, guest number ("2 of 4"), plate (or "No meal") and
    choices, phone, booking note on the first guest's row, payment
    reference. Waitlisted, awaiting-payment and cancelled bookings are
    left out. Plate totals (as in the Bookings tab, confirmed only) close
    the list. Anonymised bookings (Phase 33) are never confirmed upcoming
    ones, so they don't appear.
14. **Export attendees (CSV).** `GET
    /api/admin/events/{id}/attendees.csv` → `{slug}-attendees.csv` with
    columns `Checked in,Name,Guest,Plate,Choices,Phone,Payment
    reference,Note`. `Checked in` is empty for ticking on paper. It uses
    the existing `csv_field` (formula-safe quoting) and a UTF-8 BOM so
    Excel opens accented names correctly. The existing per-booking
    "Download guest list (CSV)" stays as the working list.
15. **Check-in sheet (branded PDF).** `/admin/events/{id}/attendees`
    (admin host, behind the existing admin auth) is a print-first page in
    the site's design: the site logo (`siteImage(content, "logo")`), the
    display and body fonts and colour tokens from `globals.css`, the event
    title, date and area as the header, then a table with a tick box,
    name, guest, plate and choices, phone and note. It has a plate summary
    box ("Abula × 9 · Jollof × 4 · No meal × 2") and the footer "Zain's
    Treat n More · printed {date}". `@media print`: A4 portrait, the admin
    shell is hidden, rows aren't split across pages, the table header
    repeats on each page, and colours print (`print-color-adjust: exact`).
    On screen a toolbar shows **Download PDF** (`window.print()`, the
    browser's "Save as PDF") and **Download CSV**. The page is in English
    like the rest of the admin. Event titles come from `content.en`.
16. **Bookings tab once closed.** When bookings are closed, the Bookings
    tab opens with a "Registration closed — {n} confirmed guests" banner
    holding **Check-in sheet (PDF)** and **Export attendees (CSV)**. Before
    closing, the same two links sit next to "Download guest list (CSV)",
    labelled "Attendees so far", so the owner can check them early.
17. **The owner is emailed when bookings close.** The sweeper (Decision 4's
    five-minute loop) finds non-archived published events whose bookings
    have closed, whose `starts_at` is still in the future or within the
    last 24 h, and whose new `events.attendees_sent_at` is null. It sets
    `attendees_sent_at` in the same statement (so a restart never sends
    twice), then emails the owner "Attendee list: {title}, {date}" with
    the confirmed count, the plate totals, a link to the check-in sheet
    (`{admin_url}/admin/events/{id}/attendees`) and the attendee CSV
    attached. `OutgoingEmail` gains `attachments: Vec<EmailAttachment {
    filename, content_type, bytes }>` (lettre `MultiPart` +
    `Attachment`). No email when there are no confirmed guests. An event
    whose close time is moved later after sending doesn't send again; the
    owner downloads a fresh copy from the tab.
18. **Personal data.** The attendee CSV is personal data, like the guest
    list. It's only behind admin auth and in the owner email (the owner
    already gets each booking by email). Nothing is stored, and logs print
    event ids only (Phase 33 Decision 10).

### Translation safeguards

20. **The DeepL allowance can't be overrun** (user's request,
    2026-11-24). Every DeepL call, whether a save, the sweep or
    Re-translate, goes through one guard in `translation.rs`:
    - **Live usage.** The API reads `GET /v2/usage` (`character_count`,
      `character_limit`) and caches it for 10 minutes, adding the
      characters it sends in between, so the limit comes from DeepL
      rather than a hard-coded number. If usage can't be read, it
      translates nothing (fail closed).
    - **Stop line.** No call goes out if `count + this request > limit ×
      stop_at_percent`. `stop_at_percent` is an admin setting, default
      **80 %**, keeping the last 20 % in reserve. The owner can raise it
      deliberately.
    - **Per-request and per-run caps.** A single save sends at most 5,000
      characters (a very long description is left for the owner to
      translate). One sweep run sends at most 20,000 characters, so the
      first backfill of a much larger menu spreads over several runs.
    - **No retry loops.** Each locale entry keeps `attempts`. After 3
      failed attempts on the same English text, the row is parked
      ("Couldn't translate automatically — please enter it yourself")
      and the sweep skips it until the English changes or the owner
      presses Re-translate.
    - **Owner switch.** "Automatic translation" on/off in
      `/admin/settings`. Off means DeepL is never called. Existing
      translations stay, and the owner types new ones.
    - **Visibility.** Settings shows "DeepL: {count} of {limit}
      characters used ({pct} %)" and the stop line. Past 60 % the admin
      dashboard shows a notice. At the stop line, forms show
      "Automatic translation paused: allowance nearly used. Enter Dutch
      and French yourself, or raise the limit in Settings." The admin menu
      list has a **Needs translation** filter (missing, stale or parked).
    - Settings live in `translation_settings` (`auto_translate BOOLEAN
      DEFAULT true`, `stop_at_percent INT DEFAULT 80 CHECK (BETWEEN 10 AND
      100)`), next to the keep list.

### Copy

21. No new customer copy beyond the menu content itself (DeepL). Admin copy
    is English. No approval gate (standing rule).

## Out of scope

- **Translating event content.** Events already have per-locale fields
  that the owner fills in. A DeepL button for them can come later.
- **Translating site content blocks** (Phase 25 `site_content`). They're
  already per-locale and owner-written.
- **Translating customer notes or enquiries** for the owner.
- **A server-generated PDF file** (user chose the branded print page). The
  owner email attaches the CSV and links to the sheet.
- **QR or door check-in tracking** in the app. The sheet is ticked on
  paper.
- **DeepL glossaries** (the "keep" word list covers dish names without
  depending on the plan's glossary support).
- **Production rollout steps** beyond setting `DEEPL_API_KEY` as a secret
  (standing rule: staging is the target).

## Open risks flagged during planning

1. **DeepL Free and commercial use.** *Resolved:* the user verified
   (2026-11-24) that DeepL API Free allows commercial use, with a
   1,000,000-character allowance. Google Cloud Translation was rejected:
   its free tier still needs a Google Cloud billing account (the one that
   couldn't be reopened in Phase 37), and the keyless "free" endpoint is
   unofficial and not allowed for a business site. *Remaining:* the menu
   may grow without limit. Decision 20 guarantees the allowance is never
   overrun, and the site degrades to owner-typed translations with
   English as the fallback.
2. **Machine translation quality on food.** DeepL may render dish
   descriptions awkwardly or translate dish names. *Mitigation:* the
   "Words to keep" list (Decision 6), owner overrides that are never
   overwritten (Decision 5), and English fallback. Checking a sample of
   nl/fr output on staging is part of validation.
3. **Drafts and translations.** A draft saved while DeepL is down
   publishes with missing nl/fr, so the public site shows English until
   the sweep runs (≤ 5 min). Acceptable: the same text was English before
   this phase.
4. **Print fidelity across browsers.** "Save as PDF" output differs
   slightly between Chrome, Safari and Firefox. *Mitigation:* print CSS is
   tested in Chrome (desktop) and Safari (macOS and iPhone share sheet),
   which are what the owner uses.
5. **Close time edited after the email.** Handled by Decision 17 (no
   resend, and the tab always has the live list).
