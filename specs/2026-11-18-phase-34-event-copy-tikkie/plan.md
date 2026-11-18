# Phase 34 — Event Copy to Production & Tikkie Payment: Plan

Decisions refer to `requirement.md`. Groups 0–2 (the copy) don't depend
on Groups 3–9 (Tikkie). On 2026-11-18 the user asked for all groups in
one go.

## Group 0 — Preconditions

0.1. Branch `2026-11-18-phase-34-event-copy-tikkie` from `master`
     (Phase 33 merged in PR #53).
0.2. Local stack up (`docker compose up -d`).
0.3. Check MinIO's client is available in the server image:
     `docker compose exec -T minio sh -c 'command -v mc'`. If it isn't,
     Group 1.4 runs `alpine/mc` pinned to a dated release tag on the
     Compose network instead (open risk 3). Record which in the
     implementation notes.

## Group 1 — `copy-staging-event.yml`

Decisions 1–10. New file `.github/workflows/copy-staging-event.yml`.

1.1. Header comment: what it copies and never copies, the refusals, and
     the staging warning (Decision 11).
     ```yaml
     on:
       workflow_dispatch:
         inputs:
           slug:
             description: 'Event slug, the same on staging and production.'
             default: the-abula-experience-2026
             required: true
           confirm:
             description: 'Type COPY EVENT TO PRODUCTION to confirm. Overwrites the production event with staging''s and publishes it.'
             required: true
     permissions:
       contents: read
       id-token: write
     ```
1.2. Job `check` (no environment): compare the phrase; validate `slug`
     against `^[a-z0-9]+(-[a-z0-9]+)*$` (it ends up in SQL and file
     names).
1.3. Job `export` (needs `check`, environment `staging`):
     - checkout; `./.github/actions/staging-host`; fail with "Staging
       isn't up." when `up != 'true'`; `ssh-target` with the staging IP.
     - Export one JSON document with `psql -tA -v slug=...`:
       ```sql
       SELECT jsonb_build_object(
         'event', to_jsonb(e) - ARRAY['id','slug','published_at','archived_at',
                                      'created_at','updated_at',
                                      'tikkie_url','tikkie_valid_until'],
         'plates', COALESCE((SELECT jsonb_agg(jsonb_build_object(
                      'position', p.position, 'content', p.content, 'options', p.options,
                      'price', p.price, 'is_available', p.is_available) ORDER BY p.position)
                    FROM event_plates p WHERE p.event_id = e.id), '[]'),
         'photos', COALESCE((SELECT jsonb_agg(jsonb_build_object(
                      'position', ph.position, 'image_url', ph.image_url, 'caption', ph.caption)
                      ORDER BY ph.position)
                    FROM event_photos ph WHERE ph.event_id = e.id), '[]'))
       FROM events e WHERE e.slug = :'slug';
       ```
       Subtracting the Tikkie keys works before and after the migration
       (open risk 6). Empty output fails with "No event with that slug
       on staging."
     - Refuse a blank address or `Staging test address, Almere`
       (Decision 6), via `jq`.
     - For each `event.cover_image_url` and `photos[].image_url`
       matching `/api/media/([^/]+)$`: check the key against
       `^[0-9a-f-]{36}\.(jpg|jpeg|png|webp)$`, `curl -fsS -D headers`
       the URL into `media/{key}`, keep the `Content-Type` in
       `media/{key}.type`.
     - `actions/upload-artifact@v4`: `event.json` + `media/`,
       `retention-days: 1`.
     - Job summary (Decision 10): title, start and end in
       Europe/Amsterdam, area, address, capacity and max party size,
       meal-order cutoff, each plate as "name — €price (Protein:
       Assorted meat · Fish)", image count.
1.4. Job `import` (needs `export`, environment `production`):
     - checkout; `actions/download-artifact@v4`; `ssh-target` with
       `vars.DEPLOY_HOST` and `secrets.DEPLOY_SSH_KEY`.
     - Checks, in one `psql -tA` (Decisions 5–6): the production event
       exists ("Run Seed production events first.") and has no rows in
       `event_bookings` ("Production has bookings for this event; it
       can't be overwritten.").
     - `PROD_BASE=$(ssh target "cd /opt/zainstreat && $C exec -T api printenv PUBLIC_API_URL")`;
       fail if empty.
     - Backup: the `pg_dump` step from `seed-production-events.yml`.
     - Upload each image (Decision 8), before the database write:
       ```sh
       ssh target "cd /opt/zainstreat && $C exec -T minio sh -c '
         mc alias set local http://localhost:9000 \"\$MINIO_ROOT_USER\" \"\$MINIO_ROOT_PASSWORD\" >/dev/null &&
         mc pipe --attr \"Content-Type=$TYPE\" \"local/\${MINIO_BUCKET:-menu-images}/$KEY\"'" < "media/$KEY"
       ```
       (or the `alpine/mc` fallback from 0.3). Check `MINIO_BUCKET` is
       set in the minio service's env; if it's only on `api`, read it
       from there like `PUBLIC_API_URL`.
     - Write, in one transaction. Copy `event.json` to the server and
       load it with `\set data \`cat /tmp/event-copy.json\`` (removed
       after), so no shell quoting touches the JSON:
       ```sql
       BEGIN;
       CREATE TEMP TABLE copy AS SELECT :'data'::jsonb AS d;
       -- URL rewrite: staging media → production, relative paths unchanged.
       CREATE FUNCTION pg_temp.media(url TEXT, base TEXT) RETURNS TEXT
         LANGUAGE sql AS $$ SELECT regexp_replace(url, '^https?://[^/]+/api/media/', base || '/api/media/') $$;
       UPDATE events e SET
         content = d->'event'->'content',
         starts_at = (d->'event'->>'starts_at')::timestamptz,
         ends_at = (d->'event'->>'ends_at')::timestamptz,
         area = d->'event'->>'area',
         address = d->'event'->>'address',
         capacity = (d->'event'->>'capacity')::int,
         max_party_size = (d->'event'->>'max_party_size')::int,
         meal_orders_close_at = (d->'event'->>'meal_orders_close_at')::timestamptz,
         rsvp_closes_at = (d->'event'->>'rsvp_closes_at')::timestamptz,
         payment_hold_hours = (d->'event'->>'payment_hold_hours')::int,
         cover_image_url = pg_temp.media(d->'event'->>'cover_image_url', :'base'),
         published_at = COALESCE(e.published_at, now()),
         updated_at = now()
       FROM copy WHERE e.slug = :'slug';
       DELETE FROM event_plates WHERE event_id = (SELECT id FROM events WHERE slug = :'slug');
       INSERT INTO event_plates (event_id, position, content, options, price, is_available)
       SELECT ev.id, (p->>'position')::int, p->'content', p->'options',
              (p->>'price')::numeric, (p->>'is_available')::boolean
       FROM copy, events ev, jsonb_array_elements(d->'plates') p WHERE ev.slug = :'slug';
       -- photos: same DELETE + INSERT, image_url through pg_temp.media.
       -- Re-check no bookings inside the transaction (a guest can't book
       -- a draft, but be exact): raise if any exist.
       COMMIT;
       ```
       `psql -v ON_ERROR_STOP=1 -v slug=… -v base="$PROD_BASE"`.
     - Show the event, as `seed-production-events.yml` does (published,
       has address, plates with price and option count, photo count).
     - Summary: "Copied and published", the public URL
       `https://${vars.DOMAIN}/en/events/{slug}`, and next steps: check
       the page in en/nl/fr, then (after Phase 34 is promoted) add the
       client's Tikkie link in admin.
1.5. `actionlint` on the new file if available locally (read-only
     check; skip if not installed).

## Group 2 — Docs for the copy

2.1. `docs/deployment.md`: a "Copying an event from staging to
     production" section after "Seeding the production menu": when to
     use it, the refusals, the reviewer sees the export summary before
     approving, and the warning (Decision 11) that Staging down, Staging
     up and Seed staging events wipe or overwrite the client's edits.
2.2. A one-line warning comment at the top of `staging-down.yml`,
     `staging-up.yml` and `seed-staging-events.yml` pointing to it.

## Group 3 — Migration (API)

Decision 12. New file; earlier migrations stay untouched.

3.1. `apps/api/migrations/20261118140000_event_tikkie.up.sql`:
     ```sql
     -- Phase 34: an open-amount Tikkie link per event, shown with bank
     -- transfer until the end of tikkie_valid_until (Decisions 12-14).
     ALTER TABLE events
         ADD COLUMN tikkie_url TEXT,
         ADD COLUMN tikkie_valid_until DATE,
         ADD CONSTRAINT events_tikkie_pair
             CHECK ((tikkie_url IS NULL) = (tikkie_valid_until IS NULL));
     ```
3.2. `.down.sql` drops the constraint and both columns.

## Group 4 — API: admin

Decisions 13 and 18.

4.1. `apps/api/src/payment.rs`: `pub fn normalize_tikkie_url(raw: &str) -> Option<String>`
     (trim, then the Decision 13 pattern; no `regex` crate needed:
     `strip_prefix("https://tikkie.me/pay/")` and check the rest is
     non-empty ASCII alphanumeric, `_` or `-`). Unit tests: valid,
     trailing space, `http://`, `tikkie.me.evil.com`, extra path,
     query string, empty token.
4.2. `routes/admin/events.rs`: `EventInput` gains
     `tikkie_url: Option<String>` and `tikkie_valid_until: Option<NaiveDate>`.
     Blank URL and no date → both null. URL without date →
     `field_error("tikkie_valid_until", "required")`; date without URL →
     `field_error("tikkie_url", "required")`; bad URL →
     `field_error("tikkie_url", "invalid")`. Saved in create and update.
4.3. `AdminEvent` gains `tikkie_url`, `tikkie_valid_until` and
     `tikkie_expired: bool` (computed in SQL, Decision 14).
     `AdminEventSummary` gains `tikkie_expired`.
4.4. `cargo sqlx prepare --workspace -- --tests`.

## Group 5 — API: guests

Decisions 14, 15 and 17.

5.1. `payment.rs`:
     ```rust
     #[derive(Debug, Serialize)]
     pub struct TikkieInstructions {
         pub url: String,
         pub valid_until: NaiveDate,
         /// The order total, which the guest types into the open-amount request.
         pub amount: Decimal,
     }
     ```
5.2. One query helper in `events.rs`,
     `async fn tikkie_for_order(pool, order_id) -> Result<Option<TikkieInstructions>, AppError>`:
     ```sql
     SELECT e.tikkie_url AS "url!", e.tikkie_valid_until AS "valid_until!", o.total
     FROM orders o
     JOIN event_bookings b ON b.order_id = o.id
     JOIN events e ON e.id = b.event_id
     WHERE o.id = $1 AND o.status = 'awaiting_payment'
       AND e.tikkie_url IS NOT NULL
       AND now() < ((e.tikkie_valid_until + 1)::timestamp AT TIME ZONE 'Europe/Amsterdam')
     ```
5.3. `routes/orders.rs`: `OrderResponse` gains
     `tikkie: Option<TikkieInstructions>`, filled in
     `load_order_by_token` (the booking page reuses it) and `None` in
     the order-creation response (cart orders are never event orders).
5.4. `event_emails.rs`: `BookingEmail` gains
     `tikkie: Option<&TikkieInstructions>`; `events::notify` loads it
     with 5.2. The payment block puts the Tikkie line first, then
     "Or pay by bank transfer:" and the existing bank lines. New copy
     keys per locale: `pay_by_tikkie` ("Pay with Tikkie: {url}"),
     `tikkie_amount` ("Enter {amount} in Tikkie. The link is valid until
     {date}."), `or_bank` ("Or pay by bank transfer:"). The date is
     formatted per locale ("2 December", "2 december", "2 décembre").
     When there are no bank details, the Tikkie line stands alone.
5.5. Tests in `event_emails.rs`: Tikkie line present and first in en,
     nl and fr; absent when `None`; bank block unchanged when `None`.

## Group 6 — API tests

6.1. `apps/api/tests/events.rs`:
     - admin save: valid link + date stored; bad URL, URL without date
       and date without URL give `400` with the field;
     - the booking's order includes `tikkie` with the order total while
       valid;
     - `tikkie` is `null` after the date (set it to yesterday), when
       unset, after "I've paid", and on a cart order;
     - valid through 23:59 Amsterdam on the date: set the date to today
       and expect it present;
     - `tikkie_expired` on the admin event and summary.
6.2. `cargo test`, `cargo clippy -- -D warnings`.

## Group 7 — Web: guests

Decision 16.

7.1. `lib/orders.ts` (or wherever `OrderResponse` is typed):
     `tikkie: { url: string; valid_until: string; amount: string } | null`.
7.2. `components/order-invoice.tsx`: when `order.tikkie` is set, the
     payment section starts with a `ButtonAnchor` "Pay with Tikkie"
     (`target="_blank" rel="noopener noreferrer"`), then
     `t("tikkieAmount", { amount })` with the amount in bold and
     `t("tikkieValidUntil", { date })`, then a divider and
     `t("orBankTransfer")` heading above the existing bank fields. The
     WhatsApp fallback (no bank details) stays below as today. No change
     when `tikkie` is null.
7.3. Both pages render through `OrderInvoice`, so the booking page and
     `/order/[token]` need no other change. Check the booking page only
     shows it while `awaiting_payment` (it already does).
7.4. Messages en/nl/fr under the invoice namespace: `payWithTikkie`,
     `tikkieAmount` ("Enter {amount} in Tikkie"), `tikkieValidUntil`
     ("Link valid until {date}"), `orBankTransfer` ("Or pay by bank
     transfer"). Dutch: "Betaal met Tikkie", "Vul {amount} in bij
     Tikkie", "Link geldig t/m {date}", "Of betaal per bankoverschrijving".
     French: "Payer avec Tikkie", "Saisissez {amount} dans Tikkie",
     "Lien valable jusqu'au {date}", "Ou payez par virement bancaire".

## Group 8 — Web: admin

Decisions 13 and 18.

8.1. `lib/admin-events.ts`: input and `toInput` gain `tikkie_url`
     (string, `""` when null) and `tikkie_valid_until` (`yyyy-mm-dd` or
     `""`).
8.2. `components/admin-event-form.tsx`: a "Payment" block after the
     meal settings: "Tikkie link" (`type="url"`, placeholder
     `https://tikkie.me/pay/…`) and "Valid until" (`type="date"`), with
     help text "An open-amount Tikkie request. Guests see the exact
     amount to enter. Leave empty to use bank transfer only." Inline
     errors from the API. A warning under the field from
     `tikkie_expired` ("Expired on 2 December: paste the client's new
     link") or, within 3 days, "Expires in N days".
8.3. Admin events list: a "Tikkie expired" badge on an upcoming,
     published event with `tikkie_expired`.
8.4. Admin copy is English only (Phase 3 scope).

## Group 9 — Seed and staging workflows

Decision 20.

9.1. `apps/api/seed-events.sql`: a fourth psql variable `tikkie_url`
     (default empty). When non-empty, set `tikkie_url` and
     `tikkie_valid_until = (now() + interval '1 year')::date` on insert
     and in the upsert. Empty leaves both untouched (null on insert).
9.2. `staging-up.yml` and `seed-staging-events.yml`: add
     `-v tikkie_url=https://tikkie.me/pay/staging-not-a-real-link`.
     `seed-production-events.yml` passes nothing.
9.3. `grep -rn dpmcb8p1hgl6338pqfn6 .` outside `specs/` returns nothing.

## Group 10 — Checks

10.1. `pnpm check:messages`, `pnpm lint`, `pnpm build` in `apps/web`.
10.2. Locally: set a link in admin, book with meals, see Tikkie first on
      the booking page, the tracking page and in the email (Mailpit or
      the logged email); set the date to yesterday and see bank transfer
      only and the admin warning.
10.3. `/validate`.
10.4. On staging after merge: `seed-staging-events.yml` must **not** run
      before the copy has been done (open risk 1).

## Implementation notes

2026-11-18, Groups 0–10:

- **No `mc` in `alpine/minio`, and no trusted `mc` image** (0.3). Images
  move with `infra/minio-object.sh get|put <key>`, which the workflow
  copies to `/tmp` on each server and runs over SSH. It runs pinned
  `curlimages/curl:8.11.1` on the Compose network and signs with
  `--aws-sigv4`. MinIO needs a `Content-Length`, so the body is buffered
  to a file first. The content type comes from the key's extension
  (`jpg`/`png`/`webp`, as `routes/admin/media.rs` names uploads).
- **Images are read from staging's MinIO over SSH**, not from their
  public URL, so the copy doesn't depend on staging's domain or Caddy.
- **The address check is a warning, not a refusal**, for the staging
  test address (Decision 6, changed at the user's request). Blank still
  refuses.
- **The export carries a `preview` block** with the times in Amsterdam,
  for the reviewer's summary; the import ignores it.
- **DATE decodes as `time::Date`** because another crate enables sqlx's
  `time` feature. The queries override it to `chrono::NaiveDate`
  (`as "…: NaiveDate"`, and `as _` on the bind).
- **`tikkie` is filled only in `load_order_by_token`** (the tracking page,
  which the booking page reuses). The order-creation response is for cart
  orders, so it's always `None` there.
- **The invoice's Tikkie amount uses `t.rich`** to bold the amount.
  Admin's expiry warning is computed in the browser from the date
  (`daysUntil`, Amsterdam's today), so it updates as the date is
  edited. The list badge uses the API's `tikkie_expired`.

