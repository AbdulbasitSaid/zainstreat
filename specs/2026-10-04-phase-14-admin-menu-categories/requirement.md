# Phase 14 — Admin: Menu & Categories: Requirements

## Source

Already an original roadmap phase. `specs/roadmap.md`, Phase 14 — the
original four bullets, expanded by this phase's planning to record the
decisions below:

> - Add / edit menu item (name, description, price, category, image,
>   available, featured).
> - Archive menu item (soft delete; remains on historical orders).
> - Availability toggle (available/unavailable).
> - Category management: add, rename, archive.

This is the second half of `mission.md`'s business goal 7 ("Let staff
manage menu items without developer assistance") — Phase 13 gave the
owner visibility into orders; this phase gives them control over what's
being sold.

## Context

Phase 13 built the dashboard shell with Menu and Categories rendered as
disabled nav items specifically so this phase could "flip a flag" rather
than restructure navigation (Phase 13 requirement.md Decision 2). The
pieces already on disk this phase consumes rather than builds:

- `apps/api/src/routes/admin/mod.rs` — the `protected` sub-router this
  phase's new routes nest into, already behind `require_admin`.
- `apps/web/components/admin-nav.tsx` — flip `Menu` and `Categories` from
  `enabled: false` to `enabled: true`.
- `apps/web/lib/admin-api.ts` (`adminApiFetch`/`adminApiJson`) and
  `apps/web/app/admin/(protected)/layout.tsx` — reused as-is.
- The same-origin Route Handler proxy pattern
  (`apps/web/app/api/admin/orders/[id]/status/route.ts`) — this phase's
  writes copy the same cookie-forwarding shape.
- `Button`, `Notice`, `.field`/`.container` classes, `AdminStatusBadge`'s
  `Record<Enum, string>` styling pattern — reused, not rebuilt.

The data model already has everything Phase 7 put in place:
`categories.display_order`, `menu_items.display_order`,
`menu_items.price` (nullable) and `menu_item_price_options` (label,
price, `display_order`, `deleted_at`) for items with multiple sized
options. Phase 7's migration comment explicitly flagged the one thing
nothing has enforced yet: *"the admin menu-editing phase should validate
[the flat-price-XOR-price-options invariant] client- and server-side when
it's built."* This phase is that phase.

One thing research surfaced that the roadmap bullets don't mention:
**menu items have an `image_url` column, but nothing in the codebase has
ever written to it or served an image from MinIO** — the `minio` service
has run since Phase 1 scaffolding, completely unused. `apps/web/next.config.ts`
already carries a comment anticipating this:

> "The production/MinIO host gets added alongside this [placehold.co]
> entry in Phase 14 — don't remove it then."

So "image" in the roadmap bullet is not just a text field — it's the
first real use of MinIO in this codebase. See Decisions 1–4 below, which
were resolved with the user specifically because of this gap (there's no
third-party image host for an admin to paste a URL from — the only
storage the business owns is the MinIO container already running).

Branches off `master` (Phase 13 is already merged; this phase touches no
file unique to any other unmerged branch).

## Decisions

1. **Images are uploaded through a new server-side proxy endpoint, not a
   pasted URL or a direct-to-MinIO presigned upload.** Decided with the
   user. The admin's browser `POST`s the file as `multipart/form-data` to
   a new same-origin Route Handler, which forwards it to a new protected
   Rust endpoint (`POST /api/admin/media`) that streams it into MinIO
   server-side using an S3 client and returns the object's public URL.
   Rejected alternatives: pasting a URL (there is no external image host
   in this stack for an admin to paste from — see Context); a presigned
   direct-to-MinIO `PUT` (would need MinIO CORS configuration and a
   public-read bucket policy, more moving parts than one admin feature
   justifies). The browser never talks to MinIO or to `apps/api` directly
   for this — same posture as every other admin write since Phase 10.

2. **Uploaded images are served back out through a new *public, unauthenticated*
   Rust route, `GET /api/media/{key}`**, not a public MinIO bucket policy
   and not a new Caddy route. `api.{$DOMAIN}` is already a publicly
   reachable Caddy site block (`roadmap.md` Phase 2) used for nothing
   today but admin JSON — this phase gives it its first public,
   unauthenticated consumer. The handler fetches the object from MinIO
   server-side and streams the bytes back with the stored `Content-Type`
   and a long, immutable `Cache-Control` (safe because Decision 4 never
   reuses a key). Rejected: a Caddy route proxying straight to `minio:9000`
   with a public-read bucket policy — functionally equivalent, but it
   would make the MinIO console/API directly fronted by the reverse proxy
   (one more thing to keep locked down) for no benefit over routing
   through the API process already doing everything else.

3. **The Rust API authenticates to MinIO with the existing
   `MINIO_ROOT_USER`/`MINIO_ROOT_PASSWORD` credentials** (new
   `MINIO_ACCESS_KEY`/`MINIO_SECRET_KEY` env vars on the `api` service,
   same values), not a second, narrowly-scoped MinIO user/policy. Rejected
   the scoped-credential approach as not worth the added MinIO policy
   administration for a single-admin, single-bucket, single-box MVP — see
   open risk 1.

4. **Every upload gets a fresh, randomly generated object key
   (`{uuid}.{ext}`), never a key derived from the menu item or overwritten
   in place.** Editing an item's image uploads a new object and replaces
   `image_url`. This is what makes Decision 2's `Cache-Control: immutable`
   safe — a URL, once issued, always points at the same bytes — and
   sidesteps filename collisions without a database lookup before upload.

   **Addendum, decided with the user 2026-10-04 (open risk 2):** when
   `PATCH /api/admin/menu-items/{id}` changes or clears `image_url`
   (confirmed replacing a different, already-committed object — never the
   just-created one), the handler deletes the *previous* object from
   MinIO after the row update commits. This is a best-effort cleanup: a
   failed `DeleteObject` is logged and never fails the request, since the
   database row is already the source of truth and a leftover object is
   harmless. Deletion happens only on edit-driven replacement/removal —
   **archiving an item never deletes its image** (Decision 7's "admin
   detail view must still load an archived item" needs the photo to keep
   resolving, and a fork confirmed during planning that no order-history
   view reads `image_url` either way, so there's no correctness reason to
   delete on archive — just a product choice to keep archived items'
   photos intact). See the narrowed open risk 2 below.

5. **The flat-price/price-options invariant Phase 7 flagged is enforced
   server-side on every create/update**: exactly one of `price` (a
   positive decimal) or `price_options` (a non-empty array, each with a
   non-empty `label` and a positive `price`) must be present — never both,
   never neither. A violation is a `400 validation_error` naming whichever
   field is the problem. **Price options are fully manageable through this
   phase's admin UI** (add/remove/reorder rows, switch an item between
   flat-priced and multi-option) — decided with the user over a
   flat-price-only MVP, since Phase 7's seed data already depends on
   options for some items and leaving them admin-unmanageable would mean
   half the real menu still needs a `psql` session to edit.

6. **Editing `price_options` is a full replace, not a diff.** `PATCH
   /api/admin/menu-items/{id}` soft-deletes (`deleted_at = now()`) every
   existing non-deleted option row for that item and inserts the
   submitted array fresh, in one transaction — regardless of whether a
   submitted option "is the same" as an existing one. This is safe only
   because `order_items` already captures `option_label`/`unit_price` by
   value at order time (`mission.md`'s historical-accuracy rule,
   unaffected either way) and nothing else references a
   `menu_item_price_options.id` by value. Rejected: a diff/upsert that
   preserves ids — meaningfully more complex for a property (price option
   id stability) nothing in the codebase depends on.

7. **Archiving a category is blocked (`409`) while it still has
   non-archived menu items.** Decided with the user over allowing it (and
   letting those items quietly vanish from the public menu with no
   category to render under) and over cascading the archive onto them
   automatically (a single click silently removing N menu items felt too
   blunt). The `409` response lists the blocking items (id + name) so the
   admin UI can tell the admin exactly what to reassign or archive first.
   Un-archiving a category is out of scope (see Out of scope).

   **Addendum, decided with the user 2026-10-04 (open risk 5).** This
   check and `validate_menu_item`'s "category must be non-archived" check
   (Decision 9/Group 8) are each fast and give a clean `400`/`409` in the
   common case, but neither is atomic with the other — a genuine race
   exists where a menu item could be inserted (or re-pointed via edit)
   under a category in the same window that category is being archived.
   The user asked for this to be completely eradicated, not narrowed, so
   it's now closed at the database level: a new Postgres trigger
   (migration `<timestamp>_guard_menu_item_category_not_archived`, the
   one migration this phase now needs) fires `BEFORE INSERT OR UPDATE OF
   category_id ON menu_items`, takes a `FOR SHARE` lock on the referenced
   `categories` row, and raises a `check_violation` if that category is
   archived. Because `archive_category`'s own `UPDATE categories SET
   deleted_at = now() WHERE id = $1` takes an exclusive lock on that same
   row, the trigger and the archive can never interleave — whichever
   transaction locks the category row first determines the outcome the
   other transaction then consistently observes once its own lock
   acquires. The API's error mapping gains a check for this trigger's
   `23514`/`check_violation` and maps it to the same `400
   validation_error` / `category_id` shape the pre-check already returns,
   so a caller never sees a raw `500` for this — only the (now
   vanishingly rare) case where the race is actually hit. *Rejected:
   relying on the application-level transaction/row-lock alone* — doing
   so would avoid a migration, but the user explicitly wants this
   invariant to hold regardless of which code path writes a row (a future
   handler, a maintenance script run directly against the database), not
   just the two call sites planned today; only a database-level
   constraint gives that guarantee.

8. **No reordering UI this phase.** `categories.display_order` and
   `menu_items.display_order` keep whatever Phase 7's seed data set; a
   newly created category or item is appended at `(current MAX
   display_order) + 1` within its scope (globally for categories, per
   `category_id` for items) so it reliably sorts last rather than
   colliding at `0` with everything else. Decided with the user — not in
   the roadmap bullet list, and a drag-reorder control is a meaningfully
   separate UI problem from this phase's CRUD forms.

9. **Two new route modules under `apps/api/src/routes/admin/`:
   `categories.rs` and `menu_items.rs`**, both nested into the existing
   `protected` sub-router (no new auth wiring):
   - `GET /api/admin/categories` — list all (including archived), bare
     array (small table, no pagination — same precedent as the public
     `GET /api/categories`).
   - `POST /api/admin/categories` — create.
   - `PATCH /api/admin/categories/{id}` — rename / edit description.
   - `POST /api/admin/categories/{id}/archive` — soft delete; `409` per
     Decision 7.
   - `GET /api/admin/menu-items` — list all (including archived and
     unavailable), bare array, with `price_options` embedded per item and
     a denormalized `category_name` (one extra join column so the admin
     list doesn't need a second round trip or a client-side category
     lookup).
   - `POST /api/admin/menu-items` — create.
   - `PATCH /api/admin/menu-items/{id}` — full edit (name, description,
     category, price/price_options, image_url, is_featured — **not**
     `is_available`, see next bullet).
   - `PATCH /api/admin/menu-items/{id}/availability` — the dedicated
     one-field toggle the roadmap calls out separately from "add/edit";
     a fast control in the list view shouldn't round-trip the whole item.
   - `POST /api/admin/menu-items/{id}/archive` — soft delete. No
     conflict check needed (unlike categories): nothing references an
     archived item's liveness the way categories reference their items,
     and `order_items` already owns its own captured copy of the name/price.
   - `POST /api/admin/media` — Decision 1's upload endpoint.

10. **`GET /api/media/{key}` is the one new *public* route this phase
    adds**, registered directly on `routes::api_router()` alongside
    `/categories`, `/menu-items` and `/orders` — not nested under
    `/admin`. It needs no session and no admin middleware; an image is
    public by definition the moment it's on the public menu.

11. **`apps/api`'s shared state grows from a bare `PgPool` to a small
    `AppState { pool, media }` struct**, with `impl FromRef<AppState> for
    PgPool` so every existing handler's `State(pool): State<PgPool>`
    keeps compiling unchanged — only `routes::api_router()`'s and
    `admin::admin_router()`'s return types change (`Router<PgPool>` →
    `Router<AppState>`), and only the two new media handlers extract the
    new `MediaConfig` substate. `media` holds the configured
    `aws_sdk_s3::Client`, the bucket name, and the public base URL
    (Decision 12). Chosen over threading a second `State` extractor
    alongside `PgPool` everywhere, or a global `OnceLock` — `FromRef`
    is axum's documented idiom for exactly this "most handlers need the
    pool, two new ones also need something else" shape, and it touches
    zero existing handler bodies.

12. **New env vars, all read once at API startup and validated like
    `DATABASE_URL`** (`std::process::exit(1)` with a logged error if
    missing, not a panic deep in a request handler):
    - `MINIO_ENDPOINT` — `http://minio:9000` in both Compose files
      (Docker-internal; the API talks to MinIO the same way it talks to
      Postgres).
    - `MINIO_ACCESS_KEY` / `MINIO_SECRET_KEY` — Decision 3.
    - `MINIO_BUCKET` — default `menu-images`.
    - `PUBLIC_API_URL` — the API's own externally-reachable origin, used
      to build the absolute URL returned from `POST /api/admin/media` and
      stored in `image_url`. `http://localhost:${API_PORT:-8080}` by
      default in `docker-compose.yml`; `https://api.${DOMAIN}` (no
      default — `DOMAIN` is already required for Caddy) in
      `docker-compose.prod.yml`. Same per-compose-file-default convention
      `tech-stack.md` already documents for `COOKIE_SECURE`/`LOG_FORMAT`.
    - The API creates its bucket idempotently at startup (`CreateBucket`,
      tolerating "already owned by you") — same "fix infrastructure drift
      automatically on boot" spirit as `sqlx::migrate!()` already running
      on every startup.

13. **`apps/web/next.config.ts` gets a second `remotePatterns` entry**
    for the publicly reachable media host — `{ protocol: "http",
    hostname: "localhost", port: "8080" }` for dev (matches the
    convention-default `API_PORT`, no new env plumbing needed into the
    `web` container for this), and, in production, `{ protocol: "https",
    hostname: \`api.${process.env.DOMAIN}\` }` — which requires a new
    `DOMAIN` env var added to the `web` service in
    `docker-compose.prod.yml` (`web` doesn't receive any env today beyond
    `API_BASE_URL`). The existing `placehold.co` entry and its
    SVG-specific image options are untouched — it remains the `ImageSlot`
    fallback's source for items with no `image_url` at all.

14. **Uploads are capped at 5 MiB and restricted to `image/jpeg`,
    `image/png`, and `image/webp`**, enforced in the handler (checked
    `content_type` and buffered byte length) *and* backstopped by an
    `axum::extract::DefaultBodyLimit::max(6 * 1024 * 1024)` layer scoped
    to just the upload route (not the whole app) so a request can't even
    buffer an oversized body before the handler's own check runs. A
    content-type/size rejection from the handler itself uses the normal
    `{"error":"validation_error","fields":[...]}` envelope; a request that
    trips the raw `DefaultBodyLimit` layer gets axum's native `413` — the
    one framework-native exception to this envelope, noted as open risk 4.

    **Addendum, decided with the user 2026-10-04 (open risk 4).** Two
    separate closes, after explicitly considering and rejecting moving
    the crop/resize work itself to the Rust API:
    - **Frontend crop.** The image-upload component lets the admin crop
      the selected photo to a fixed 1:1 square before it's ever uploaded,
      using **`react-easy-crop`** (pinned in `tech-stack.md`) — a small
      component purpose-built for exactly this pan/zoom/crop-to-a-fixed-
      ratio flow, with native touch support. The crop result (a
      canvas-rendered `Blob`) is what's sent to `POST /api/admin/media`;
      the original, uncropped file is never uploaded. This is a product/
      consistency decision (every menu photo is the same shape on every
      card) layered on top of, not a substitute for, the size/type
      enforcement below. *Rejected: doing the crop server-side in Rust* —
      the interactive pan/zoom/crop UI can only run in the browser either
      way (Rust/WASM driving its own canvas UI would mean reimplementing
      `react-easy-crop` from scratch for no benefit), and the crop
      operation itself is a single sub-millisecond `canvas.drawImage`
      call, not a workload Rust would meaningfully speed up. Doing it
      server-side would also require uploading the full, uncropped
      original first — strictly more bytes over the wire and more
      latency than cropping before upload, the opposite of a performance
      win.
    - **Closing the actual bypass.** Nothing before this addendum stopped
      a direct `PATCH /api/admin/menu-items/{id}` (whether from a
      frontend bug or a raw API call reusing a stolen admin cookie) from
      setting `image_url` to an arbitrary string — skipping this
      decision's content-type/size checks entirely, since those only run
      inside the upload endpoint. `validate_menu_item` now additionally
      rejects any non-null `image_url` that doesn't match this API's own
      issued shape (`{PUBLIC_API_URL}/api/media/{uuid}.{ext}`, the same
      prefix Group 8's `media_key_from_url` already strips to recover a
      key for deletion, just checked rather than unwrapped) with a `400
      validation_error` naming `image_url`. This is what actually makes
      "oversized image never gets bypassed" true: the only way to end up
      with a persisted `image_url` is for its bytes to have already
      passed through the checked upload endpoint.

15. **Admin stays English-only; forms use `next/link`/`next/navigation`,
    never `@/i18n/navigation`'s `Link`** — direct continuation of Phase
    11 Decision 8 / Phase 13 Decisions 13–14. No new keys in
    `apps/web/messages/{en,nl}.json`.

16. **Backend gets integration tests** (`apps/api/tests/admin_categories.rs`,
    `apps/api/tests/admin_menu_items.rs`); media upload/serve is covered by
    a focused `apps/api/tests/admin_media.rs` using a tiny in-memory PNG
    fixture. **The frontend continues to have none** — validated manually
    via `/validate`'s live smoke test, same precedent as every prior phase.

17. **Optimistic concurrency control, decided with the user 2026-10-04
    (open risk 3), reusing the existing `updated_at` column as the
    conflict token — no new column, no migration.** Every admin write
    endpoint this phase adds (`PATCH /api/admin/categories/{id}`, `POST
    /api/admin/categories/{id}/archive`, `PATCH
    /api/admin/menu-items/{id}`, `PATCH
    /api/admin/menu-items/{id}/availability`, `POST
    /api/admin/menu-items/{id}/archive`) now requires the caller to echo
    back the `updated_at` it last read for that row — a new required
    `updated_at` field on each request body. `AdminCategoryResponse` and
    `AdminMenuItemResponse` both gain an `updated_at: DateTime<Utc>`
    field so the frontend always has a current token to carry forward.
    Every mutating query's `WHERE` clause becomes `WHERE id = $1 AND
    updated_at = $2`; an update that affects zero rows is disambiguated
    by a follow-up `SELECT`: the row doesn't exist at all → the existing
    `404 not_found`, the row exists but its `updated_at` no longer
    matches → a new `409 {"error": "conflict", "current": {...the
    current row, same shape a GET returns...}}` so the admin's UI can
    show what actually changed and let them reload or retry rather than
    silently clobbering someone else's edit. Applied uniformly to every
    write endpoint, including the single-field availability toggle — no
    endpoint is carved out as "too small to matter," since the point is
    to close the risk, not just the obviously dangerous-looking half of
    it. Decided with the user specifically because one admin account can
    still have two active browser sessions open at once (two tabs, or a
    stale tab left open from an earlier session) — single-admin auth
    doesn't rule out concurrent-edit data loss the way it might first
    appear to (same accepted-risk pattern Phase 13 open risk 2 originally
    flagged for order status, now actually mitigated rather than carried
    forward). Rejected a new `version` integer column: reusing
    `updated_at` needs no migration (every write already sets it to
    `now()`) and avoids a second "did this change" source of truth that
    could drift out of sync with it.

## Out of scope

- Un-archiving a category or menu item. There is no roadmap bullet for
  it; for now it's a direct `psql` edit (`UPDATE ... SET deleted_at =
  NULL`), same escape hatch every other archive-only feature in this repo
  currently has.
- Reordering categories or menu items (Decision 8).
- Deleting an uploaded image from MinIO on item **archive** or category
  archive (Decision 4 addendum, open risk 2) — only an edit-driven
  replace/removal deletes the superseded object now.
- Server-side image resizing or multiple generated sizes. The frontend
  now crops to a fixed 1:1 square before upload (Decision 14 addendum,
  open risk 4), but the API still stores and serves exactly the bytes it
  receives — no server-side resize/thumbnail/format-variant generation;
  `next/image` still handles responsive display sizing and format
  negotiation on the frontend, same as it does for `placehold.co` images
  today.
- Bulk operations (bulk archive, bulk category reassignment, CSV
  import/export of the menu).
- A MinIO bucket lifecycle policy / orphaned-object cleanup job (open
  risk 2).
- Scoped MinIO credentials for the API, separate from the root user
  (open risk 1).
- Any change to the public `GET /api/categories` / `GET /api/menu-items`
  response shapes or the public menu page's rendering — this phase only
  adds admin-side write paths and the new public `GET /api/media/{key}`.

## Open risks flagged during planning

1. **The API authenticates to MinIO as the root user** (Decision 3), so a
   bug in the new upload/serve code has the blast radius of full MinIO
   access, not just one bucket. Acceptable at single-admin, single-box
   scale; revisit if MinIO ever holds more than this one bucket's worth of
   low-sensitivity menu photography.
2. **Archived items' images are never deleted from MinIO** (Decision 4
   addendum) — `minio_data` grows by one object per archived item
   forever, since archiving intentionally leaves the photo resolvable for
   the admin's own archived-item view. (The other half of this risk —
   every *edit's* worth of orphaned objects — is now handled: Decision 4's
   addendum deletes the superseded object on replace/removal.) Harmless at
   menu-photo scale and volume for the foreseeable future; a cleanup job
   (e.g. triggered at archive time once nothing needs the photo anymore)
   is cheap to add later and not worth building speculatively now.
3. **`DefaultBodyLimit`'s `413` on an oversized upload bypasses this
   project's `{"error": "..."}` JSON envelope** (Decision 14) — the
   frontend's upload component needs to handle a non-JSON error response
   specifically for this one route, the one inconsistency of its kind in
   the codebase. (The oversize-bypass question this risk originally
   raised — whether an oversized image could ever actually reach
   storage — is resolved by Decision 14's addendum; what remains here is
   narrowly the non-JSON shape of this one error response.)
4. **The dashboard's "Menu"/"Categories" nav entries flip to enabled with
   no new empty-state design pass** beyond what each new list page does
   on its own — there's no dedicated "first-run, zero categories exist
   yet" onboarding flow beyond an empty-state message, since Phase 4's
   seed data means this condition won't occur in practice before this
   phase ships.

Former open risks 3 (no optimistic-concurrency check) and 5 (category-
archive race) were resolved with the user on 2026-10-04 — see Decision
17 and Decision 7's addendum respectively — rather than carried forward
here.
