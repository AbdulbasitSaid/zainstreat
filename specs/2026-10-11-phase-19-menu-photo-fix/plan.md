# Phase 19 — Menu Photo Fix: Implementation Plan

Groups 1–2 are manual (DNS provider, GitHub settings). Groups 3–5 change
the repo and ship together in one push to `master`. Group 2 must be done
**before** that push, or the Decision 3 guard fails the deploy.

Run `/validate` after Groups 3–5 are implemented locally, before merging.

## Group 0 — Branch

Branched `2026-10-11-phase-19-menu-photo-fix` off `master` (Phase 18
merged as `f8d1759`).

## Group 1 — Remove the dead DNS record (ops, now)

Depends on: nothing. Independent of the code fix, and the most urgent.

1.1. In the DNS provider for `zainstreat.com`, delete every A record
     pointing to `18.192.249.64` for `@`, `www`, `api` and `logs`. Keep
     the `63.185.172.26` records. If `api`/`logs` are CNAMEs to the root,
     only the root records need changing.

1.2. Lightsail console → Networking → Static IPs: if `18.192.249.64` is an
     unattached static IP (left over from Phase 18 Group 2), release it.
     Unattached static IPs are billed.

1.3. Confirm (allow for the record's TTL):

```bash
for h in zainstreat.com www.zainstreat.com api.zainstreat.com logs.zainstreat.com; do
  echo "$h: $(dig +short $h | tr '\n' ' ')"
done
# Expect only 63.185.172.26 (after any CNAME line)
```

## Group 2 — Add the `DOMAIN` repository variable (ops)

Depends on: nothing. Must be done before Group 3 is pushed.

2.1. GitHub → repo → Settings → Secrets and variables → Actions →
     **Variables** tab → New repository variable: `DOMAIN` =
     `zainstreat.com` (bare domain, no scheme, no `api.`).

## Group 3 — Bake `DOMAIN` into the web image (repo)

Depends on: Group 2.

3.1. `.github/workflows/deploy.yml`, the "Build and push web" step: add
     `build-args`:

```yaml
      - name: Build and push web
        uses: docker/build-push-action@v6
        with:
          context: ./apps/web
          push: true
          build-args: |
            DOMAIN=${{ vars.DOMAIN }}
          tags: |
            ...unchanged...
```

3.2. `apps/web/Dockerfile`, `build` stage: declare the arg, fail if it's
     empty, and expose it to `next build` (requirement.md Decisions 1
     and 3):

```dockerfile
FROM node:22-bookworm-slim AS build
WORKDIR /app
# next.config.ts (images.remotePatterns), robots.ts, sitemap.ts and
# metadataBase read DOMAIN at *build* time — standalone output saves the
# result, so a runtime-only DOMAIN is too late (Phase 19).
ARG DOMAIN
RUN test -n "$DOMAIN" || (echo "DOMAIN build arg is required (set the DOMAIN repo variable)" >&2 && exit 1)
ENV DOMAIN=$DOMAIN
COPY --from=deps /app/node_modules ./node_modules
COPY . .
RUN corepack enable && pnpm build
```

     Put the `ARG`/`RUN test` *before* `COPY . .` so a missing arg fails
     in seconds, not after the dependency copy.

3.3. `apps/web/next.config.ts`: rewrite the `remotePatterns` comment.
     "DOMAIN is passed into the web service's environment for exactly
     this (docker-compose.prod.yml)" becomes: DOMAIN must be present at
     `next build` (Dockerfile build arg from `vars.DOMAIN`, Phase 19),
     because standalone output saves this config at build time.
     No logic change.

3.4. `docker-compose.prod.yml`, `web.environment.DOMAIN`: keep it
     (requirement.md Decision 4), with a one-line comment that the
     build-time value (Dockerfile `ARG DOMAIN`) is the one that counts for
     `remotePatterns`, `robots.txt` and `sitemap.xml`. This one only covers
     dynamically rendered metadata.

## Group 4 — Cart: survive a replaced photo (repo)

Depends on: nothing (independent of Group 3).

4.1. `apps/web/components/cart-line-item.tsx`: track a load failure and
     fall back to the existing placeholder (requirement.md Decision 5a):

```tsx
const [imageFailed, setImageFailed] = useState(false);
...
{line.imageUrl && !imageFailed ? (
  <Image
    ...unchanged props...
    onError={() => setImageFailed(true)}
  />
) : (
  <ImageSlot label={line.name} className="h-full min-h-0 p-0 text-[0.6rem]" />
)}
```

     The file is already `"use client"`; just import `useState`.

4.2. `apps/web/lib/cart-context.tsx`, `cartReducer`'s `add` case: when the
     line already exists, also refresh its display fields from the incoming
     line (Decision 5b):

```ts
line.key === key
  ? {
      ...line,
      name: action.line.name,
      imageUrl: action.line.imageUrl,
      quantity: line.quantity + action.quantity,
    }
  : line
```

     Leave `unitPrice` alone. Whether a price change should update an
     existing cart line is a separate product question, not part of this
     bug.

## Group 5 — Validate locally (repo)

Depends on: Groups 3–4.

5.1. `/validate` (lint + build + live smoke test).

5.2. Check the Dockerfile guard locally, both ways:

```bash
cd apps/web
docker build --target build -t zt-web-noarg .            # must FAIL: "DOMAIN build arg is required"
docker build --target build --build-arg DOMAIN=zainstreat.com -t zt-web-arg .   # must succeed
docker run --rm zt-web-arg sh -c 'grep -o "api\.zainstreat\.com" .next/required-server-files.json | head -1'
# → api.zainstreat.com  (the allowlist baked in)
```

## Group 6 — Deploy and verify (ops)

Depends on: Groups 1–5 merged to `master` (CI deploys).

6.1. Run validation.md's production checks.

6.2. If photos still fail with a **500/timeout** instead of 200
     (requirement.md Risk 1, hairpin fetch), stop and plan the follow-up.
     Don't revert Group 3: it also fixes the SEO URLs.
