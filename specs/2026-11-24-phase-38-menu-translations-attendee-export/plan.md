# Phase 38 — Menu Translations & Event Attendee Export: Plan

One migration, no new packages (`reqwest` and `lettre` are already in the
API; lettre's `builder` feature already includes `MultiPart`/`Attachment`).
One new secret, `DEEPL_API_KEY`. Decisions refer to `requirement.md`.

The two halves are independent: Groups 1–6 (translations) and Groups 7–9
(attendees) can be built in either order. Group 0 is shared.

## Group 0 — Migration (Decisions 1, 6, 9, 17)

`apps/api/migrations/20261124120000_menu_translations_attendees.up.sql`:

```sql
-- Phase 38: per-locale menu text (Decision 1), the DeepL keep list
-- (Decision 6), localized order-line snapshots (Decision 9) and the
-- once-only attendee email (Decision 17).
ALTER TABLE menu_items
    ADD COLUMN translations JSONB NOT NULL DEFAULT '{}' CHECK (jsonb_typeof(translations) = 'object');
ALTER TABLE categories
    ADD COLUMN translations JSONB NOT NULL DEFAULT '{}' CHECK (jsonb_typeof(translations) = 'object');
ALTER TABLE menu_item_price_options
    ADD COLUMN translations JSONB NOT NULL DEFAULT '{}' CHECK (jsonb_typeof(translations) = 'object');
ALTER TABLE rentals
    ADD COLUMN translations JSONB NOT NULL DEFAULT '{}' CHECK (jsonb_typeof(translations) = 'object');

CREATE TABLE translation_settings (
    id BOOLEAN PRIMARY KEY DEFAULT true CHECK (id),
    -- Decision 6: whole words DeepL must leave alone, one per entry.
    keep_words TEXT[] NOT NULL DEFAULT '{}',
    -- Decision 20: the owner's switch and the stop line on DeepL's allowance.
    auto_translate BOOLEAN NOT NULL DEFAULT true,
    stop_at_percent INT NOT NULL DEFAULT 80 CHECK (stop_at_percent BETWEEN 10 AND 100),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
INSERT INTO translation_settings (keep_words) VALUES (ARRAY[
    'Jollof', 'Egusi', 'Efo Riro', 'Ogbono', 'Ofada', 'Puff Puff',
    'Small Chops', 'Suya', 'Chin Chin', 'Abula', 'Amala', 'Gbegiri',
    'Fufu', 'Moi Moi', 'Dodo', 'Asun', 'Zobo', 'Meat Pie'
]);

ALTER TABLE order_items
    ADD COLUMN item_name_local TEXT,
    ADD COLUMN option_label_local TEXT;

ALTER TABLE events ADD COLUMN attendees_sent_at TIMESTAMPTZ;
```

`.down.sql` drops all of the above. Mirror the columns in `seed.sql` only
if it lists columns explicitly. The defaults cover it otherwise, and the
sweep translates dev data when a key is set.

## Group 1 — Translation core (Decisions 1–3, 6, 20)

New `apps/api/src/translation.rs`:

```rust
pub struct DeeplConfig { key: Option<Arc<str>>, base: &'static str, client: reqwest::Client }
impl DeeplConfig {
    pub fn from_env() -> Self; // DEEPL_API_KEY; ":fx" suffix → https://api-free.deepl.com, else https://api.deepl.com
    pub fn is_enabled(&self) -> bool;
    /// One request per target; returns texts in input order. 5 s timeout.
    pub async fn translate(&self, texts: &[String], target: Locale, keep: &[String]) -> Result<Vec<String>, TranslateError>;
}
pub fn is_stale(locale_entry: &Value, english: &Value) -> bool; // entry["source"] != current English fields
pub fn protect(text: &str, keep: &[String]) -> String; // whole-word, case-insensitive → <keep>…</keep>, XML-escapes the rest
pub fn unprotect(text: &str) -> String;
pub fn resolve(translations: &Value, locale: Locale, key: &str, english: &str) -> String; // fallback per field
```

- Request: `POST {base}/v2/translate`, header `Authorization: DeepL-Auth-Key {key}`,
  JSON `{ text: [...], source_lang: "EN", target_lang: "NL"|"FR",
  tag_handling: "xml", ignore_tags: ["keep"], formality: "prefer_more"
  (FR only), preserve_formatting: true }`. Response `translations[].text`.
- 456 (quota exceeded), 429, 5xx, timeout → `TranslateError`, logged as
  `deepl failed status=…` with no text.
- `source` stores the English fields verbatim (Decision 1), so no hash
  crate is needed (`sha2` is only transitive in `Cargo.lock`).
- `Locale` lives in `order_emails.rs`. Reuse it.
- A `translate_fields(cfg, keep, english: Fields, existing: &Value) -> Value`
  helper builds the new `translations` object: for each of nl/fr, keep the
  locale if `manual` or `source` matches, or if it's parked (`attempts >=
  3` with the same `source`); otherwise translate. On error, keep what was
  there and increment `attempts`; on success, reset `attempts` to 0.
- **Budget guard (Decision 20).** All DeepL traffic goes through one
  `Budget` held in `DeeplConfig` (`tokio::sync::Mutex<UsageCache { count,
  limit, fetched_at }>`):

  ```rust
  pub enum Refusal { Disabled, Paused { count: u64, limit: u64 }, TooLong, UsageUnknown }
  /// Reserves `chars` (English chars × number of targets) or refuses.
  /// Refreshes from GET {base}/v2/usage when the cache is >10 min old.
  async fn reserve(&self, settings: &TranslationSettings, chars: u64) -> Result<(), Refusal>;
  ```

  - `auto_translate = false` → `Disabled`. Usage fetch fails →
    `UsageUnknown` (fail closed). `count + chars > limit *
    stop_at_percent / 100` → `Paused`. A single save over 5,000 chars →
    `TooLong`.
  - On a successful reserve, `count += chars` locally until the next
    refresh, so a burst of saves can't overshoot between refreshes.
  - `translate` never calls DeepL without a successful `reserve`. A 456
    from DeepL sets the cache to `count = limit` until the next refresh.
  - A refusal is not an error for the save: the form shows the matching
    message (Group 6).
  - Unit tests: stop line at 80 % (refuse at 800,001 of 1,000,000), local
    increment, disabled, unknown usage, `TooLong`, parked after 3 failures.
- Unit tests: `protect`/`unprotect` round-trip (including "Jollof Rice" →
  `<keep>Jollof</keep> Rice`, XML special chars), `resolve` fallback on
  missing/blank, host choice from key suffix. No network in tests.

Wire `DeeplConfig` into app state alongside `EmailConfig`/`PushConfig`.
Add `DEEPL_API_KEY: ${DEEPL_API_KEY:-}` to `docker-compose.yml` (api
service), `# DEEPL_API_KEY=  # [secret]` to `.env.example`, the secret to
`.github/workflows/_deploy.yml` (same pattern as `V_MAPBOX_ACCESS_TOKEN`),
and a step in `docs/deployment.md` next to the Mapbox one.

## Group 2 — Admin save paths (Decisions 1–3, 5)

- `routes/admin/menu_items.rs`: `MenuItemFields` gains
  `#[serde(default)] translations: Value`; `PriceOptionInput` gains
  `#[serde(default)] translations: Value`. The request may carry owner
  edits as `{"nl": {"name": "…", "manual": true}}`. On `save_menu_item_draft`
  and `create_menu_item`: merge the owner's manual locales, then call
  `translate_fields` for the rest, then store in the draft. Publishing
  (`drafts.rs`) copies `translations` to `menu_items.translations` and each
  option's to `menu_item_price_options.translations` (options are
  re-created on publish: carry translations by position).
- `routes/admin/categories.rs`: same for `CategoryFields`.
- `routes/admin/rentals.rs`: same on create/update, written straight to
  `rentals.translations`.
- Responses for admin GETs include `translations` and a computed
  `translation_status: { nl: "ok"|"missing"|"stale"|"manual"|"manual_stale", fr: … }`.
- *(As built)* No separate re-translate route: the form drops the
  locale's entry and saves, and the next publish (rental: save) fills it.
- *(As built)* Menu item and category saves (autosave) only
  `translation::accept_edits`. `drafts::publish` calls
  `translation_sweep::translate_for_publish` for each menu item and
  category entry before its transaction.
- `drafts.rs` change list (Phase 25 preview diff): include nl/fr name and
  description so the owner sees translated changes before publishing.
- Translation runs inside the request with the 5 s timeout. A failure
  never fails the save or publish (Decision 3).

## Group 3 — Translation sweep (Decision 4)

In `translation.rs`, `spawn_translation_sweeper(pool, deepl)`: a separate
`tokio::spawn` loop, every 300 s, no-op unless `deepl.is_enabled()`.

Per tick, no-op unless `auto_translate` is on. Rows across the four tables
(menu items and categories: the draft if one exists, else the live row;
options with their item) where nl or fr is missing, or `manual` is not
true and `source` ≠ the current English fields, and the locale isn't
parked. Translate rows in id order until **20,000 characters** have been
sent this tick or the budget guard refuses, then `UPDATE … SET
translations = $1 WHERE id = $2 AND translations = $old AND name = $name …`
(drafts: `WHERE draft = $old_draft`). Neither concurrency token is
touched, so an open admin form never gets a 409 from the sweep, and an
unrelated change (an availability toggle) doesn't waste characters
already sent. Drafts and unpublished rows are skipped (Publish translates them). *(As built; the plan first said `updated_at = $3`.)* Stop the tick on the first refusal or quota error (456). Log counts
and characters sent only.

Call it from `main.rs` next to `spawn_hold_sweeper`.

## Group 4 — Public API (Decision 7)

- `routes/menu_items.rs`, `routes/categories.rs`, `routes/rentals.rs`, and
  `drafts::preview_menu_items` / `preview_categories`: accept
  `Query { locale: Option<String> }` (`Locale::parse`, default `En`).
  Select `translations`, resolve `name`/`description`/`label` with
  `translation::resolve`. Add `names: {en, nl, fr}` to items and rentals
  and `labels: {en, nl, fr}` to options.
- `apps/web/lib/api.ts`: `getCategories(locale)`, `getMenuItems(locale,
  categoryId?)`, `getRentals(locale)` append `locale=`; types gain
  `names: Record<Locale, string>` and option `labels`. Update every caller
  (`app/[locale]/page.tsx`, `menu`, `rentals`, `services`, anything else
  `grep -rn "getMenuItems\|getCategories\|getRentals" apps/web`), passing
  the route's `locale`.

## Group 5 — Cart and orders (Decisions 8, 9)

- `lib/cart-context.tsx`: `CartLine` gains `names?: Record<Locale,
  string>` and `optionLabels?: Record<Locale, string> | null`; add-to-cart
  passes them from the menu/rental data. A `lineName(line, locale)` helper
  returns `line.names?.[locale] ?? line.name` (same for option label). Use
  it in the cart page, checkout summary, add-to-cart feedback and the
  header cart count tooltip if any. Old saved lines still parse (both
  optional).
- `routes/orders.rs`: when pricing menu lines and rentals, also select
  `translations` (item and option) and, for the order's locale ≠ en, set
  `item_name_local` / `option_label_local` (null if missing or same as
  English). Insert both new columns.
- Customer renders use `item_name_local ?? item_name`: `order_emails.rs`
  (invoice, status, fee-confirmed emails), the public order/tracking
  responses (add `display_name`/`display_option_label` resolved
  server-side, so the web doesn't need to know), `order-invoice.tsx`,
  `order-view.tsx`. Admin and owner paths stay on `item_name`.
- `#[sqlx::test]`: an `nl` order on a translated item stores the Dutch
  snapshot, an `en` order stores null, and the owner email body is English.

## Group 6 — Admin UI (Decisions 5, 6, 20)

- New `components/admin-locale-tabs.tsx` (extract from the event form if
  its tabs are inline): English / Nederlands / Français, with a status dot
  per tab (missing / stale).
- `admin-menu-item-form.tsx`, `admin-category-form.tsx`,
  `admin-rental-form.tsx`: wrap name/description (and option labels) in
  the tabs. nl/fr inputs are pre-filled from `translations`; editing one
  marks it manual in the payload. Under nl/fr: "Translated automatically",
  or "Edited by you" + Re-translate, or "English changed since this was
  edited" + Re-translate, or "Not translated yet — we'll retry
  automatically." Refusals show instead: `Paused` → "Automatic translation
  paused: allowance nearly used. Enter Dutch and French yourself, or raise
  the limit in Settings."; `Disabled` → "Automatic translation is off.
  Enter Dutch and French yourself."; `TooLong` / parked → "Couldn't
  translate automatically. Please enter it yourself." nl/fr inputs are
  always editable.
- `/admin/settings`: a "Translations" section with:
  - **Automatic translation** on/off switch.
  - **Usage meter:** "DeepL: {count} of {limit} characters used ({pct} %)"
    with the stop line marked, and a "Stop at" percentage input (10–100,
    default 80).
  - **Words to keep** textarea (one per line, ≤ 100 words, each ≤ 40
    chars).
  - A status line: "Menu translations: 42 up to date, 3 waiting, 1 needs
    you".

  Data comes from a new `GET /api/admin/translations/status` (usage from
  the guard's cache, counts from the tables).
  `PUT /api/admin/translations/settings` saves all three (with
  `updated_at` concurrency). Changing the keep list clears `source` on
  every non-manual translation so the sweep redoes them (Decision 6). The
  form first shows "This re-translates about {n} characters ({pct} % of
  your allowance)" and asks to confirm (an in-page confirm, not
  `window.confirm`).
- Admin dashboard: a notice when usage passes 60 % ("Translation
  allowance {pct} % used — see Settings").
- Admin list pages (`admin-menu-list.tsx`, `admin-category-list.tsx`,
  `admin-rental-list.tsx`): a "NL/FR missing" or "Needs translation"
  badge on rows that are missing, stale or parked, and a **Needs
  translation** filter on the menu list.

## Group 7 — Attendee data and CSV (Decisions 12–14)

`routes/admin/events.rs`:

- `load_attendees(pool, event_id) -> AttendeesResponse { event: { id,
  slug, title, starts_at, area, bookings_closed, bookings_close_at },
  guests: [{ booker_name, guest_no, party_size, plate_name, choices,
  phone, note, payment_reference }], plate_counts, confirmed_guests }`.
  Confirmed bookings only. `note` only on `guest_no = 1`. Reuse the
  choices formatting from `bookings_csv`. `bookings_closed` uses the
  Decision 12 SQL expression.
- `GET /events/{id}/attendees` (JSON) and `GET /events/{id}/attendees.csv`
  (header `Checked in,Name,Guest,Plate,Choices,Phone,Payment
  reference,Note`, body prefixed with `\u{feff}`, `csv_field` per cell,
  `Content-Disposition: attachment; filename="{slug}-attendees.csv"`).
  Factor the CSV builder into `fn attendees_csv(&AttendeesResponse) ->
  String` for Group 9.
- Register both in `routes/admin/mod.rs` next to `bookings.csv`.
- Web proxies: `app/api/admin/events/[id]/attendees.csv/route.ts` (copy
  `bookings.csv/route.ts`); the JSON goes through `lib/admin-events-api.ts`
  (`getAdminEventAttendees`).
- Tests: `csv_field` reuse; a `#[sqlx::test]` with one confirmed party of
  3, one waitlisted and one cancelled → 3 rows, first row carries the
  note.

## Group 8 — Check-in sheet and Bookings tab (Decisions 15, 16)

- `app/admin/(protected)/events/[id]/attendees/page.tsx` (server
  component): fetches `getAdminEventAttendees` and `getSiteContent()` for
  the logo. Renders the branded header (logo, event title in
  `font-display`, date in Europe/Amsterdam, area, "{n} guests"), the plate
  summary box, the table (☐, Name, Guest, Plate & choices, Phone, Note)
  and the footer. Toolbar (client component `attendee-sheet-toolbar.tsx`):
  **Download PDF** → `window.print()`, **Download CSV** → the proxy link,
  **Back to bookings**. Empty state: "No confirmed guests yet."
- Print CSS in a co-located `attendees.module.css` (or `print:` Tailwind
  variants): `@page { size: A4 portrait; margin: 14mm }`, hide the admin
  nav/toolbar (`print:hidden` on the admin shell; check
  `app/admin/(protected)/layout.tsx` for its wrapper),
  `thead { display: table-header-group }`, `tr { break-inside: avoid }`,
  `print-color-adjust: exact` on the header band, brand colours from the
  existing tokens. Set `<title>` to `{slug}-attendees` so "Save as PDF"
  suggests that file name.
- `components/admin-event-bookings.tsx`: if `bookings_closed` (add it to
  `AdminBookingsResponse` / `load_bookings`), a top banner
  "Registration closed — {n} confirmed guests" with **Check-in sheet
  (PDF)** (`/admin/events/{id}/attendees`) and **Export attendees (CSV)**.
  Otherwise the same two links beside "Download guest list (CSV)" under
  the label "Attendees so far".

## Group 9 — Owner email when bookings close (Decision 17)

- `email.rs`: `pub struct EmailAttachment { pub filename: String, pub
  content_type: String, pub bytes: Vec<u8> }`; `OutgoingEmail` gains
  `attachments: Vec<EmailAttachment>` (all existing constructors pass
  `vec![]`, or add `Default`). `send` builds a `MultiPart::mixed()` with
  the text body plus `Attachment::new(filename).body(bytes,
  ContentType::parse(..))` when non-empty, and keeps today's single-part
  message otherwise. Unit test: a message with one attachment renders
  `Content-Disposition: attachment; filename="x.csv"`.
- `events.rs`: `claim_closed_events(pool) -> Vec<i64>`:

  ```sql
  UPDATE events SET attendees_sent_at = now()
  WHERE attendees_sent_at IS NULL AND archived_at IS NULL AND published_at IS NOT NULL
    AND now() >= LEAST(meal_orders_close_at, rsvp_closes_at, starts_at)
    AND starts_at > now() - interval '24 hours'
  RETURNING id
  ```

  For each id: `load_attendees`; if `confirmed_guests > 0` and
  `business_notify_address()` is set, send subject "Attendee list:
  {title} ({date})", a body with the count, plate totals and the sheet link
  (`{admin_url}/admin/events/{id}/attendees`), and the CSV attached.
  Called from the existing `spawn_hold_sweeper` loop after
  `release_expired_holds` (so holds released at the deadline are already
  gone).
- The migration sets nothing for past events: the `starts_at > now() - 24h`
  guard stops a burst of emails for old events on deploy.
- `#[sqlx::test]`: claim returns an event once and never again; an
  archived or unpublished event is never claimed.
