# Phase 38 — Menu Translations & Event Attendee Export: Validation

## Pass/fail checklist

### Build and tests

- [ ] `cargo test` and `cargo clippy -- -D warnings` pass in `apps/api`,
      including the `protect`/`unprotect`/`resolve` unit tests, the
      attachment test, and the localized-order, attendee and
      `claim_closed_events` `#[sqlx::test]`s (no network calls in tests).
- [ ] `apps/api/.sqlx/` is regenerated and the Docker build passes with
      `SQLX_OFFLINE=true`.
- [ ] Exactly one new migration (`…_menu_translations_attendees`) with a
      `.down.sql`; `sqlx migrate run` → `revert` → `run` works on a dev
      database.
- [ ] `pnpm lint` and `pnpm build` pass in `apps/web`, with no `any`. No
      new npm or cargo packages (`git diff apps/api/Cargo.toml
      apps/web/package.json` shows no dependency changes).
- [ ] `DEEPL_API_KEY` is wired in `docker-compose.yml`, `.env.example`,
      `_deploy.yml` and `docs/deployment.md`.
- [ ] `/validate` passes.

### Translations

- [ ] With no `DEEPL_API_KEY`: admin saves work, nl/fr show "Not
      translated yet", and `/nl/menu` shows English (fallback).
- [ ] With a key: within one sweep (≤ 5 min) of startup, every existing
      menu item, category, option label and rental has nl and fr
      (`SELECT count(*) FROM menu_items WHERE deleted_at IS NULL AND NOT
      (translations ? 'nl' AND translations ? 'fr')` → 0, same for the
      other three tables).
- [ ] `curl '/api/menu-items?locale=nl'` returns Dutch `name`/`description`
      and `names.{en,nl,fr}`; `?locale=xx` and no locale return English.
      Same for `/api/categories` and `/api/rentals`.
- [ ] `/nl/menu`, `/fr/menu`, `/nl/rentals` and the home page show
      translated names, descriptions, category headings and option labels.
- [ ] Words on the keep list are unchanged in nl/fr (e.g. "Jollof",
      "Egusi", "Puff Puff", "Small Chops").
- [ ] Editing the Dutch name in admin and publishing shows it on `/nl`.
      Changing the English description afterwards leaves the Dutch name
      as edited and shows "English changed since this was edited".
      Re-translate replaces it.
- [ ] Editing English re-translates non-manual locales on save. The
      preview (Phase 25) shows the new nl/fr before publishing.
- [ ] Adding a word to "Words to keep" leads to re-translated rows within
      one sweep.
- [ ] Spot-check 10 items in nl and fr on staging for sense (risk 2).

### Translation safeguards (Decision 20)

- [ ] Saving a menu item with only a new price, photo, availability or
      order makes no DeepL call (API debug log / DeepL usage unchanged).
- [ ] Settings shows the DeepL usage meter matching DeepL's own
      dashboard (± the 10-minute cache).
- [ ] With "Stop at" set below current usage (e.g. 10 %): saving a changed
      English name makes no DeepL call, the save succeeds, the form shows
      "Automatic translation paused…", and the owner can type nl/fr and
      publish them.
- [ ] With "Automatic translation" off: no DeepL calls from saves, the
      sweep or Re-translate; existing translations still show on `/nl`.
- [ ] With the key set to an invalid value (usage unreadable): nothing is
      sent (fail closed), saves succeed, and forms show a manual-entry
      message.
- [ ] A row that fails 3 times shows "Couldn't translate automatically"
      and the sweep stops retrying it; changing its English or pressing
      Re-translate tries again.
- [ ] One sweep tick sends ≤ 20,000 characters (log line). A save over
      5,000 English characters isn't sent.
- [ ] Changing "Words to keep" shows the "re-translates about {n}
      characters" confirmation before saving.
- [ ] The **Needs translation** filter lists exactly the missing, stale
      and parked rows.

### Cart and orders

- [ ] Add an item on `/en/menu`, switch to `/nl/cart`: the line shows the
      Dutch name. A cart saved before this phase still loads.
- [ ] Place an order from `/nl`: `order_items.item_name_local` is Dutch,
      `item_name` is English. The invoice email, success page and tracking
      page show Dutch. The admin order page, owner email and push show
      English.
- [ ] An `/en` order stores `item_name_local = NULL`. Old orders render
      unchanged.

### Attendee export

- [ ] For an event with confirmed, waitlisted, awaiting-payment and
      cancelled bookings: `GET /api/admin/events/{id}/attendees.csv` lists
      only confirmed guests, one row per person, note on the first row
      only, starts with a UTF-8 BOM, and opens in Excel/Numbers with
      accents intact. A name starting with `=` is prefixed with `'`.
- [ ] Without an admin session, `/api/admin/events/{id}/attendees(.csv)`
      → `401`, and `/admin/events/{id}/attendees` redirects to login.
- [ ] Before the close time: the Bookings tab shows "Attendees so far"
      links; no banner.
- [ ] After the close time (set `rsvp_closes_at` in the past): the banner
      "Registration closed — {n} confirmed guests" with both buttons.
- [ ] `/admin/events/{id}/attendees` shows the logo, brand fonts and
      colours, event header, plate summary and table. **Download PDF** in
      Chrome and Safari gives an A4 PDF with no admin nav or toolbar, the
      table header repeated on page 2+, no split rows, coloured header,
      and the suggested file name `{slug}-attendees`.
- [ ] Within five minutes of the close time, the owner gets one "Attendee
      list" email with the count, plate totals, a working sheet link and
      the CSV attached. Restarting the API afterwards sends nothing more
      (`events.attendees_sent_at` set).
- [ ] An event with no confirmed guests is marked sent but no email goes
      out. Events that closed more than 24 h before deploy get no email.
- [ ] API logs for the sweep contain event ids only, no names or emails.

## Definition of done

Customers on `/nl` and `/fr` read the menu, categories, options and
rentals in their language (DeepL-translated, owner-overridable, dish names
kept), carry those names through the cart, and see them on their invoice
and tracking page while the owner and kitchen keep English. Existing
content is translated automatically after deploy with no manual step. Once
an event's bookings close, the owner gets an email with the confirmed
attendee CSV and a link to a branded, printable check-in sheet, and can
download both from the Bookings tab at any time. All checks above pass on
staging, with `DEEPL_API_KEY` set after the DeepL plan terms are
confirmed (risk 1).
