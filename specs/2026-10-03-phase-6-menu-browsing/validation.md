# Phase 6 — Menu Browsing (Read-Only): Validation

How to confirm this phase is done and safe to merge. Run these checks after
completing all task groups in `plan.md`.

## Pass/fail checklist

### Backend

- [x] `cd apps/api && cargo test` passes locally (requires `DATABASE_URL`
      pointed at a reachable Postgres with `CREATEDB`; the dev stack's
      `postgres` service works as-is). 4/4 tests pass.
- [x] `SQLX_OFFLINE=true cargo check` (from `apps/api`) succeeds using only
      the committed `.sqlx/` cache — proves the Docker build will too.
      (Verified with `--all-targets`, covering `tests/` too.)
- [x] `docker compose build api` succeeds. (Also verified the production
      `Dockerfile` — the one CI actually uses — builds standalone with
      `SQLX_OFFLINE=true` and no DB access, using only the committed
      `.sqlx/` cache.)
- [ ] CI's `test-api` job is green on a pull request (confirms the
      `pull_request` trigger works) and again on push to `master`. **Not
      checked here** — requires pushing the branch and opening a PR;
      `deploy.yml`'s YAML was validated to parse and the job graph
      (`test-api` gating `build-and-push`/`deploy`, the latter two gated
      to `push` only) was reviewed, but no actual GitHub Actions run was
      observed.
- [x] `curl -s http://localhost:${API_PORT:-8080}/api/categories` against
      the dev stack with `apps/api/seed.sql` loaded returns the seeded
      categories, excluding any soft-deleted ones (exclusion also covered
      by the `categories_excludes_soft_deleted` integration test).
- [x] `curl -s http://localhost:${API_PORT:-8080}/api/menu-items` returns
      all seeded items, including any with `is_available: false`, each
      with `price` as a JSON **string** (e.g. `"6000.00"`, not a bare
      number). Found and fixed a pre-existing bug in `seed.sql`: the
      `INSERT ... SELECT` hardcoded `is_available = true` for every row
      and routed the per-item boolean entirely into `is_featured`, so no
      seeded item was ever unavailable. Fixed to track both columns
      independently (Meat Pie is now the one `is_available: false` item).
- [x] `curl -s "http://localhost:${API_PORT:-8080}/api/menu-items?category_id=<id>"`
      returns only that category's items.
- [x] A request to either endpoint with the DB briefly stopped returns a
      generic `500 {"error":"internal_server_error"}`, not a raw SQL error
      or a panic.

### Frontend

- [x] `pnpm lint` and `pnpm build` (inside `apps/web`) both succeed with no
      `@typescript-eslint/no-explicit-any` violations and no TypeScript
      errors.
- [x] `http://localhost:3000/en/menu` renders: category filter pills, item
      cards (name, description, price, image) grouped by category.
- [x] `http://localhost:3000/nl/menu` renders the same, in Dutch.
- [x] `http://localhost:3000/en/menu?category=<id>` shows only that
      category's items and highlights the matching filter pill
      (`aria-current="page"`).
- [x] A category seeded with zero available/visible items (Drinks, which
      has none) shows the "Nothing available in this category right now"
      `Notice` message instead of an empty grid.
- [x] An item seeded with `is_available: false` (Meat Pie, after the
      `seed.sql` fix above) renders visibly grayed/desaturated with an
      "Currently unavailable" badge — confirms Decision 5's unavailable
      items actually reach the frontend.
- [x] Homepage (`/en`, `/nl`) shows a "Featured Menu" section with items
      where `is_featured = true` and `is_available = true`, each linking
      nowhere (no cart yet) except a "View Full Menu" button to `/menu`.
- [x] If no `menu_items` row has `is_featured = true`, the homepage section
      is hidden entirely — not an empty-state message, not a broken
      section with a heading and no cards. (Verified live by temporarily
      clearing all `is_featured` flags in the dev DB, then restoring them.)
- [x] `/menu` is reachable from both the header nav and the footer, in
      both `/en` and `/nl`.
- [x] Stopping `apps/api` and reloading `/en/menu` shows the
      `menu/error.tsx` friendly fallback ("We couldn't load the menu right
      now...") instead of Next's default error overlay or an unhandled
      exception.
- [x] Menu item images render via `next/image` (not a broken image icon)
      for items with a `placehold.co` `image_url`; items with a null
      `image_url` show the existing `ImageSlot` placeholder. Found and
      fixed a real bug: `placehold.co` serves `image/svg+xml`, which
      `next/image`'s optimizer blocks by default (SVGs can embed
      scripts), so every placeholder image 400'd. Fixed via
      `next.config.ts`'s documented `dangerouslyAllowSVG` + sandboxed
      `contentSecurityPolicy` combination.
- [x] `<html lang="en">` / `<html lang="nl">` still correct on `/menu` and
      the homepage.
- [x] No regressions to Phase 5 pages (`/about`, `/services`, `/contact`,
      `/terms`, `/food-regulations` still return `200` in both locales).

### Scope check

- [x] `git diff master --stat` for this branch touches only:
      `apps/api/{Cargo.toml,Cargo.lock,src/**,tests/**,.sqlx/**,seed.sql}`,
      `.github/workflows/deploy.yml`, `docker-compose.yml`,
      `apps/web/{next.config.ts,lib/**,components/menu-item-card.tsx,
      components/category-filter.tsx,components/site-header.tsx,
      components/site-footer.tsx,app/[locale]/{page.tsx,menu/**},
      messages/{en,nl}.json,public/images/menu/**}`,
      `docs/local-development.md`, `specs/tech-stack.md`, and this phase's
      own `specs/` folder. No unrelated files should appear.
      (`seed.sql` and `public/images/menu/**` were added by the
      real-photos/copy addendum above, not the original Phase 6 scope —
      both are dev-only fixture content, not application code.)

### Implementation notes beyond the plan

Two gaps surfaced only by actually running the stack end-to-end, both
fixed:

1. **Dev `api` container couldn't start on a fresh volume.** `cargo run`'s
   compile step checks `query_as!`/`query!` calls against the *live* DB by
   default; on a brand-new Postgres volume the schema doesn't exist yet
   (migrations only run at program startup, after compilation finishes).
   This was latent before this phase (the old `/health` query touched no
   table), but the new `categories`/`menu_items` queries expose it
   immediately. Fixed by adding `SQLX_OFFLINE: "true"` to the `api`
   service's environment in `docker-compose.yml`, so dev compiles against
   the committed `.sqlx/` cache exactly like the prod Dockerfile does.
2. **`seed.sql`'s `is_available`/`is_featured` column mix-up** and
   **`next.config.ts`'s missing SVG allowance** — both described above.

## Definition of done

Phase 6 is complete when: the two read-only API endpoints are live,
tested, and gated by CI on both PRs and pushes to master; `/menu` and the
homepage's Featured Menu section render real seeded data with correct
empty/unavailable states in both locales; and `/menu` is discoverable from
primary navigation. Phase 7 (Cart) can now build on top of
`GET /api/menu-items`'s `is_available` field and the `MenuItemCard`
component to implement "unavailable items cannot be added to the cart."
