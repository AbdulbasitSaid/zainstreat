# Phase 6 — Menu Browsing (Read-Only): Validation

How to confirm this phase is done and safe to merge. Run these checks after
completing all task groups in `plan.md`.

## Pass/fail checklist

### Backend

- [ ] `cd apps/api && cargo test` passes locally (requires `DATABASE_URL`
      pointed at a reachable Postgres with `CREATEDB`; the dev stack's
      `postgres` service works as-is).
- [ ] `SQLX_OFFLINE=true cargo check` (from `apps/api`) succeeds using only
      the committed `.sqlx/` cache — proves the Docker build will too.
- [ ] `docker compose build api` succeeds.
- [ ] CI's `test-api` job is green on a pull request (confirms the
      `pull_request` trigger works) and again on push to `master`.
- [ ] `curl -s http://localhost:${API_PORT:-8080}/api/categories` against
      the dev stack with `apps/api/seed.sql` loaded returns the seeded
      categories, excluding any soft-deleted ones.
- [ ] `curl -s http://localhost:${API_PORT:-8080}/api/menu-items` returns
      all seeded items, including any with `is_available: false`, each
      with `price` as a JSON **string** (e.g. `"6000.00"`, not a bare
      number).
- [ ] `curl -s "http://localhost:${API_PORT:-8080}/api/menu-items?category_id=<id>"`
      returns only that category's items.
- [ ] A request to either endpoint with the DB briefly stopped returns a
      generic `500 {"error":"internal_server_error"}`, not a raw SQL error
      or a panic.

### Frontend

- [ ] `pnpm lint` and `pnpm build` (inside `apps/web`) both succeed with no
      `@typescript-eslint/no-explicit-any` violations and no TypeScript
      errors.
- [ ] `http://localhost:3000/en/menu` renders: category filter pills, item
      cards (name, description, price, image) grouped by category.
- [ ] `http://localhost:3000/nl/menu` renders the same, in Dutch.
- [ ] `http://localhost:3000/en/menu?category=<id>` shows only that
      category's items and highlights the matching filter pill
      (`aria-current="page"`).
- [ ] A category seeded with zero available/visible items shows the
      "Nothing available in this category right now" `Notice` message
      instead of an empty grid.
- [ ] An item seeded with `is_available: false` renders visibly
      grayed/desaturated with an "Currently unavailable" badge — confirms
      Decision 5's unavailable items actually reach the frontend.
- [ ] Homepage (`/en`, `/nl`) shows a "Featured Menu" section with items
      where `is_featured = true` and `is_available = true`, each linking
      nowhere (no cart yet) except a "View Full Menu" button to `/menu`.
- [ ] If no `menu_items` row has `is_featured = true`, the homepage section
      is hidden entirely — not an empty-state message, not a broken
      section with a heading and no cards.
- [ ] `/menu` is reachable from both the header nav and the footer, in
      both `/en` and `/nl`.
- [ ] Stopping `apps/api` and reloading `/en/menu` shows the
      `menu/error.tsx` friendly fallback ("We couldn't load the menu right
      now...") instead of Next's default error overlay or an unhandled
      exception.
- [ ] Menu item images render via `next/image` (not a broken image icon)
      for items with a `placehold.co` `image_url`; items with a null
      `image_url` show the existing `ImageSlot` placeholder.
- [ ] `<html lang="en">` / `<html lang="nl">` still correct on `/menu` and
      the homepage.
- [ ] No regressions to Phase 5 pages (`/about`, `/services`, `/contact`,
      `/terms`, `/food-regulations` still return `200` in both locales).

### Scope check

- [ ] `git diff master --stat` for this branch touches only:
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

## Definition of done

Phase 6 is complete when: the two read-only API endpoints are live,
tested, and gated by CI on both PRs and pushes to master; `/menu` and the
homepage's Featured Menu section render real seeded data with correct
empty/unavailable states in both locales; and `/menu` is discoverable from
primary navigation. Phase 7 (Cart) can now build on top of
`GET /api/menu-items`'s `is_available` field and the `MenuItemCard`
component to implement "unavailable items cannot be added to the cart."
