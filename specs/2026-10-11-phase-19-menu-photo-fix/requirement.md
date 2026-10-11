# Phase 19 — Menu Photo Fix: Requirements

## Source

New phase, not in the original roadmap. It comes from a production bug
reported on 2026-10-11, after Phase 18 merged (`f8d1759`):

> Editing a menu photo breaks the photo link. The new photo shows in the
> database and on the admin page, but not on the main website.

## Context

### The bug: confirmed against production

Read-only check against `https://zainstreat.com` on 2026-10-11:

```
IMG=https://api.zainstreat.com/api/media/8d897c79-24f8-4dd4-9103-02a793d6ff28.jpg
direct from API:      200 image/jpeg
through /_next/image: 400  "url" parameter is not allowed
```

The API stores and serves the new photo correctly. Next's image optimizer
refuses to load it.

### Why

1. **Seed photos vs uploaded photos.** Phase 7's seed photos are same-site
   paths (`/assets/...`), which `next/image` always accepts. Since
   Phase 14, an admin upload is saved as a full address,
   `https://api.$DOMAIN/api/media/<uuid>` (`apps/api/src/routes/admin/media.rs`).
   So the first time an admin *edits* a seed item's photo, it moves from a
   working path to an absolute URL. That's why the bug looks like
   "editing breaks the photo".
2. **Phase 17 Decision 9** (commit `6176352`) changed
   `unoptimized={url.startsWith("http")}` to
   `unoptimized={url.startsWith("http://")}` in `menu-item-card.tsx`,
   `admin-menu-list.tsx` and `cart-line-item.tsx`. Since then, production's
   `https://` uploads go through the optimizer.
3. The optimizer only loads hosts listed in `images.remotePatterns`.
   `apps/web/next.config.ts` adds `api.${process.env.DOMAIN}` only when
   `DOMAIN` is set.
4. **`output: "standalone"` reads `next.config.ts` once, at `next build`,
   and saves the result into `server.js`.** `.github/workflows/deploy.yml`
   builds the web image without `DOMAIN`, so the shipped allowlist is just
   `placehold.co` and `localhost:8080`. The runtime `DOMAIN` in
   `docker-compose.prod.yml`'s `web.environment` arrives too late to
   change anything.

Why it looked fine elsewhere:
- **Database:** the new URL is saved correctly.
- **Admin edit form:** previews the photo with a plain `<img>`
  (`admin-image-upload.tsx:175`), which skips the optimizer.
- **Admin menu list** thumbnails *are* broken too, because they use
  `next/image`. They're just small and easy to miss.

### Same cause, second symptom: SEO URLs point to localhost

Phase 17's `app/robots.ts`, `app/sitemap.ts` and `app/[locale]/layout.tsx`
(`metadataBase`) also read `process.env.DOMAIN` and fall back to
`http://localhost:3000`. `robots.ts` and `sitemap.ts` are rendered
statically at build time, so production serves this today (checked
2026-10-11):

```
robots.txt:  Sitemap: http://localhost:3000/sitemap.xml
sitemap.xml: <loc>http://localhost:3000/en</loc> ...
```

Search engines are given a sitemap they can't use. Decision 1 fixes this
too, with no extra code.

### A related bug: stale cart photos

When an edit replaces a photo, `update_menu_item`
(`apps/api/src/routes/admin/menu_items.rs:425`) deletes the old MinIO
object, which is intended (Phase 14 Decision 4 addendum). But a visitor's
cart in `localStorage` (`apps/web/lib/cart-context.tsx`) keeps the old
`imageUrl` forever. Their cart thumbnail then points at a deleted object
and breaks.

### Found during research: a dead IP in DNS

`zainstreat.com`, `www.` and `api.` each resolve to two A records:
`63.185.172.26` responds, and `18.192.249.64` refuses connections. Clients
pick one at random, so some visitors get
`SSL_ERROR_SYSCALL` on any page. The dead address is most likely left over
from the instance Phase 18 Group 2 replaced. It's fixed in DNS, not in the
repo, but it's included here because it fails every page.

## Decisions

1. **Pass `DOMAIN` into the web image at build time.** CI sends it as a
   Docker build arg, and the `Dockerfile`'s `build` stage sets it as an
   env var before `pnpm build`, so the existing `remotePatterns` code in
   `next.config.ts` finally sees it. This keeps Phase 17 Decision 9's WebP
   conversion and Phase 18's 30-day optimizer cache.
   - **Rejected: go back to `unoptimized` for every `https://` URL.** It
     loses the optimization Phase 17/18 rely on.
   - **Rejected: allow any hostname for `/api/media/**`.** That turns the
     optimizer into an open image proxy.
   - **Rejected: serve media from the main site's own address.** It needs
     Caddy and API changes, too much for a bug fix.
2. **`DOMAIN` comes from a GitHub Actions repository *variable*
   (`vars.DOMAIN`), not a secret.** The domain is public, and a variable
   stays readable in the workflow logs for debugging.
3. **Fail the Docker build if `DOMAIN` is empty.** The check goes in
   `apps/web/Dockerfile`, not `next.config.ts`. Only CI builds that
   `Dockerfile`, because dev's `docker-compose.yml` runs `pnpm dev` on
   plain `node:22`. So a guard there can't break local `pnpm build` or
   `/validate`, and it stops this bug from coming back silently.
4. **Keep the runtime `DOMAIN` in `docker-compose.prod.yml`'s
   `web.environment`.** Any route that renders dynamically reads
   `metadataBase` at request time, so build time and runtime should agree.
   Correct the `next.config.ts` comment, which currently suggests the
   runtime value is enough, and add a comment in compose saying the
   build-time value is the one that counts for `remotePatterns`,
   `robots.txt` and `sitemap.xml`.
5. **Cart: two small client-side changes, no extra network request.**
   - (a) If a cart thumbnail fails to load, `cart-line-item.tsx` falls back
     to `ImageSlot` (`onError`), the same placeholder used for items with
     no photo.
   - (b) When the item is added to the cart again, the reducer's `add`
     case refreshes `imageUrl` (and `name`) on an existing line, so it
     picks up the current values.

   Re-fetching every cart item from the menu API on load was rejected:
   it adds a request on every page for an edge case (b) and (a) already
   cover.
6. **The DNS cleanup is a manual step** (plan Group 1), done first because
   it affects every page.

## Out of scope

- Deleting orphaned MinIO objects or adding versioned media keys. Keys are
  already unique UUIDs, so caching isn't involved here.
- Moving the seed photos to MinIO. They work as same-site paths.
- Swapping the admin form's `<img>` preview for `next/image`. That
  `<img>` is intentional (see its eslint-disable comment).

## Open risks flagged during planning

1. **Hairpin fetch.** After the fix, the optimizer in the `web` container
   fetches `https://api.zainstreat.com`. That request goes out to the
   public IP and back in through Caddy. The current 400 proves the
   allowlist rejection, not that this fetch will work. If photos still
   fail after deploy with a 500 or a timeout, the follow-up is a
   `loader`/rewrite that points the optimizer at `http://api:8080`
   internally.
2. **`vars.DOMAIN` missing.** If the repository variable isn't created
   before the first push, the Decision 3 guard fails the deploy. That's
   intended, but it blocks deploys until the variable is set
   (plan Group 2).
3. **Old optimizer 400s in `web_cache`.** Next doesn't cache error
   responses, so no purge should be needed. Validation checks this.
