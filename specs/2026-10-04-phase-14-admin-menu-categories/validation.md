# Phase 14 — Admin: Menu & Categories: Validation

## Pass/fail checklist

### Backend

- [ ] `cd apps/api && cargo test` — all cases in `tests/admin_categories.rs`,
      `tests/admin_menu_items.rs` and `tests/admin_media.rs` pass, and
      every pre-existing test file (`admin_auth.rs`, `admin_orders.rs`,
      `menu_browsing.rs`, `order_submission.rs`) still passes unchanged
      after the `AppState`/`FromRef` refactor (Group 3) and the new
      `common.rs` helpers.
- [ ] `cd apps/api && cargo clippy --all-targets -- -D warnings` — clean.
- [ ] `cd apps/api && cargo sqlx prepare --check` — the committed
      `.sqlx/` cache covers every new query in `routes/admin/categories.rs`
      and `routes/admin/menu_items.rs`, including the OCC-conditioned
      `WHERE id = $1 AND updated_at = $2` variants (Decision 17); no
      drift.
- [ ] `git diff --stat apps/api/migrations/` shows exactly one new
      migration pair, `<ts>_guard_menu_item_category_not_archived.{up,down}.sql`
      (plan.md Group 1a, open risk 5) — nothing else. Confirm `sqlx
      migrate run` applies it cleanly against a fresh database and `sqlx
      migrate revert` cleanly undoes it.
- [ ] Confirm by reading `routes/admin/mod.rs` that all nine new routes
      (`categories` ×3, `menu-items` ×4, `media` upload) sit inside the
      `protected` sub-router, above `.route_layer(from_fn(middleware::require_admin))`,
      and that `GET /api/media/{key}` in `routes/mod.rs` is registered
      **outside** `/admin` — not behind the auth middleware.
- [ ] Read `routes/admin/media.rs`'s `upload` and `routes/media.rs`'s
      `get_media`: confirm the `AppError::NotFound` placeholder error
      mapping noted in plan.md Groups 5/6 was replaced with a real
      server-error variant before merging (a MinIO write failure must not
      report itself to the client as `"not_found"`).
- [ ] Read `error.rs`'s `sqlx::Error` → `AppError` conversion: confirm the
      `23514`/"menu item category" branch (plan.md Group 2, open risk 5)
      maps to `AppError::CategoryArchived`, not the generic `Database`
      500 path.
- [ ] Confirm every write handler in `routes/admin/categories.rs` and
      `routes/admin/menu_items.rs` (`update_category`, `archive_category`,
      `update_menu_item`, `update_availability`, `archive_menu_item`)
      conditions its `UPDATE` on `updated_at = $N` and returns
      `AppError::Conflict` (not a silent success or a 404) when the row
      exists but the token didn't match (plan.md Groups 7/8, Decision 17).

### Frontend build

- [ ] `cd apps/web && pnpm lint` — clean, no
      `@typescript-eslint/no-explicit-any` violations on any new file.
- [ ] `cd apps/web && npx tsc --noEmit` — clean.
- [ ] `cd apps/web && pnpm build` — succeeds; the route list shows
      `/admin/menu`, `/admin/menu/new`, `/admin/menu/[id]`,
      `/admin/categories`, `/admin/categories/new`,
      `/admin/categories/[id]`, and the new `/api/admin/*` route handlers
      **without** a locale prefix.
- [ ] `grep -rn "i18n/navigation\|ButtonLink" apps/web/app/admin/\(protected\)/menu apps/web/app/admin/\(protected\)/categories apps/web/components/admin-menu-item-form.tsx apps/web/components/admin-category-form.tsx`
      returns nothing.

### Live dev stack (`docker compose up`) — manual smoke test

Start from the Phase 7 seed data (some flat-priced items, some with
`price_options`) so both price-entry modes have something real to edit.

- [ ] **Nav:** `/admin` shows Menu and Categories as real, clickable links
      (no longer "Soon"); Settings is still disabled.
- [ ] **Categories list (`/admin/categories`):** shows every seeded
      category; creating one with a blank name shows the `name required`
      error inline, not a raw API error.
- [ ] **Category archive, blocked case:** archiving a category that still
      has active menu items shows the specific blocking item names from
      the `409` response, not a generic failure message.
- [ ] **Category archive, success case:** archiving an empty (or
      fully-archived-items) category succeeds and the row shows an
      "Archived" badge.
- [ ] **Menu list (`/admin/menu`):** shows every seeded item, including
      ones with multiple price options rendered as "N sizes" rather than
      a single price.
- [ ] **Availability toggle:** flipping it in the list view updates
      immediately and persists after a hard refresh; `psql` confirms
      `is_available` changed.
- [ ] **Create flat-priced item:** `/admin/menu/new`, fill name/category/
      price, submit → appears in the list with the right price.
- [ ] **Create multi-option item:** switch the price-options editor to
      "multiple sizes", add two label/price rows, submit → the public
      `/en/menu` page (existing Phase 7 UI) renders both options for the
      new item exactly like the seeded multi-option items do.
- [ ] **Image upload + crop:** on the create or edit form, choose a local
      `.jpg`/`.png`/`.webp` file → a `react-easy-crop` square crop UI
      appears over it (pan/zoom both work); confirming the crop uploads
      only the cropped result (visually confirm the saved photo is the
      cropped square, not the full original frame), then the saved item
      shows the uploaded photo (via `next/image`, not `ImageSlot`) on both
      `/admin/menu` and the public menu page. `curl -I` the returned
      `image_url` directly → `200`, correct `Content-Type`, and a
      `Cache-Control: public, max-age=31536000, immutable` header.
- [ ] **Oversized/wrong-type upload:** selecting a file over 5 MiB, or a
      non-image file, shows a clear error in the upload component rather
      than a silent failure or an unhandled JSON-parse exception (the
      `413`/`400` non-JSON-body case from plan.md Group 20).
- [ ] **Oversize bypass is actually closed (open risk 4):** with a valid
      admin session cookie, `curl -i -X PATCH
      http://localhost:8080/api/admin/menu-items/{id} -H 'Content-Type:
      application/json' -d '{"category_id":...,"name":"x","price":"1.00",
      "price_options":[],"image_url":"https://example.com/huge.jpg",
      "is_featured":false,"updated_at":"..."}'` (an `image_url` never
      issued by this API) → `400 validation_error`, `fields[0].field ==
      "image_url"`; `psql` confirms the row's `image_url` is unchanged.
- [ ] **Optimistic concurrency conflict (open risk 3):** open the same
      menu item for edit in two browser tabs, save a small change in the
      first tab, then save a different change in the second (still-stale)
      tab → the second save shows the "changed elsewhere" `Notice`
      instead of silently overwriting the first tab's save; `psql`
      confirms the row reflects the first tab's change. Repeat once for
      the category rename form and once for the list view's availability
      toggle (toggle it in one tab, then try the now-stale toggle control
      in a second tab without refreshing).
- [ ] **Category-archive race is closed (open risk 5):** archive a
      category that has zero active items, then immediately (same `curl`
      session, no UI delay) attempt `POST
      /api/admin/menu-items` with that now-archived `category_id` → `400
      validation_error`, `fields[0].field == "category_id"` (the ordinary
      pre-check catching it in the non-race case); confirmed separately at
      the database level by `admin_menu_items.rs`'s
      `category_archived_trigger_rejects_a_direct_insert_under_an_archived_category`
      test (plan.md Group 13) — no manual race-timing reproduction needed
      here, the trigger's guarantee doesn't depend on timing.
- [ ] **Replacing an image deletes the old object (open risk 2):** edit an
      item that already has an uploaded photo, upload a *different* photo,
      save. Note the old `image_url` from the item's prior state (e.g.
      `psql` or the network tab before saving) → `curl -I` it afterward →
      `404`. The new `image_url` still `200`s. Then edit the same item
      again changing only the name (image untouched) → the current image
      still `200`s (confirms the cleanup only fires on an actual change).
- [ ] **Archiving an item keeps its image (open risk 2 scope):** archive a
      menu item that has an uploaded photo → its `image_url` still
      `curl -I`s `200` afterward (Decision 4 addendum intentionally never
      deletes on archive, only on edit-driven replace/removal).
- [ ] **Edit an item's price options:** change a multi-option item from
      two sizes to one (or to a flat price) → the public menu reflects
      exactly the new set, and `psql` shows the old option rows have
      `deleted_at` set rather than being gone or duplicated.
- [ ] **Archive a menu item:** archived items disappear from the public
      `/en/menu` page but remain visible (marked "Archived") in
      `/admin/menu`; an already-placed order referencing that item (from
      before archiving) still shows the original item name/price on its
      `/admin/orders/[id]` detail page, unaffected.

### Auth gating (the part the frontend must not be trusted for)

- [ ] `curl -i http://localhost:8080/api/admin/menu-items` (no cookie) →
      `401`.
- [ ] `curl -i -X POST http://localhost:8080/api/admin/categories -H
      'Content-Type: application/json' -d '{"name":"x"}'` (no cookie) →
      `401`, and `psql` confirms no row was inserted.
- [ ] `curl -i -X POST http://localhost:8080/api/admin/media -F
      'file=@/path/to/image.jpg'` (no cookie) → `401`.
- [ ] `curl -i http://localhost:8080/api/media/anything.png` (no cookie,
      against a real uploaded key) → `200` — confirms the media
      **serve** route is intentionally public, not an auth bug.

### Regression

- [ ] `/validate`-equivalent sweep (lint + build + live HTTP smoke test) —
      no regression on `/`, `/en`, `/nl`, `/en/menu`, `/en/cart`,
      `/en/order`, `/api/categories`, `/api/menu-items`, `/health`, the
      Phase 11 login/logout loop, and Phase 13's `/admin/orders` flow.
- [ ] Public `GET /api/menu-items` and `GET /api/categories` response
      shapes are byte-for-byte unchanged from `master` (diff a `curl`
      response against both branches) — this phase must not have added
      an admin-only field to the public endpoints by accident.
- [ ] CI green on PR (same open item every prior phase has left unchecked
      at spec-writing time).

## Definition of done

An administrator can run the entire menu without a developer: create a
category, rename it, and archive it once nothing active references it
(with a clear error telling them what still does); create a menu item
with either a single price or several labeled size options, attach a
real photo by cropping and uploading it from their device (served back
out through the API's own public media route, the first real use of the
MinIO container running since Phase 1), edit any of that later —
including switching an item between flat and multi-priced — toggle its
availability with one click, and archive it when it's no longer sold.
Every write is enforced server-side by Phase 11's `require_admin`
middleware independently of the frontend. Historical orders are untouched
by any of this: an archived or edited item's captured name and price on
past orders (`mission.md`'s non-negotiable rule) remain exactly as they
were at purchase time, and the public menu only ever reflects the
current, non-archived state.

Three risks flagged during the original planning pass were fully closed
rather than left accepted (all decided with the user 2026-10-04): an
oversized or arbitrary external image can never reach `image_url` — only
bytes that already passed this API's own upload/size/type check can
(Decision 14 addendum); two overlapping admin sessions editing the same
category or menu item can no longer silently clobber each other — a
stale save is rejected with a `409` naming what actually changed
(Decision 17); and a menu item can never end up pointing at an archived
category, even under concurrent requests, because the database itself
rejects it, not just the application's pre-check (Decision 7 addendum).
