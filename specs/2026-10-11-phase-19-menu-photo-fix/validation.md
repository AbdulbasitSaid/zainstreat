# Phase 19 — Menu Photo Fix: Validation

## Pass/fail checklist

### DNS (Group 1)

- [ ] `dig +short zainstreat.com` (and `www.`, `api.`, `logs.`) returns
      only `63.185.172.26`.
- [ ] `for i in $(seq 10); do curl -s -o /dev/null -w "%{http_code} " https://zainstreat.com/; done`
      gives ten `307`s, no `000`.
- [ ] `18.192.249.64` is released in Lightsail, or confirmed not to be a
      Lightsail static IP.

### GitHub variable (Group 2)

- [ ] Repo → Settings → Actions → Variables lists `DOMAIN` =
      `zainstreat.com`.

### Repo changes, local (Groups 3–5)

- [x] `.github/workflows/deploy.yml`'s web build step passes
      `DOMAIN=${{ vars.DOMAIN }}` in `build-args`. The api step is
      unchanged.
- [x] `docker build --target build apps/web` with **no** `DOMAIN` fails
      with "DOMAIN build arg is required".
- [x] With `--build-arg DOMAIN=zainstreat.com` it succeeds, and
      `.next/required-server-files.json` in that image contains
      `api.zainstreat.com`.
- [x] `next.config.ts`'s `remotePatterns` comment says DOMAIN is needed at
      build time. The logic is unchanged.
- [ ] `cart-line-item.tsx` renders `ImageSlot` when the image fails. Check
      locally: add an item to the cart, edit
      `localStorage["zainstreat:cart:v1"]` so the line's `imageUrl` is
      `http://localhost:8080/api/media/does-not-exist.jpg`, reload
      `/en/order`, and expect the placeholder, not a broken-image icon.
- [ ] `cartReducer` `add` on an existing line updates `imageUrl`/`name`
      and adds to `quantity`. `unitPrice` is untouched.
- [x] `/validate` passes (lint, build, smoke test).

### Production, after deploy (Group 6)

- [ ] The CI "Build and push web" step's log shows the `DOMAIN` build arg,
      and the deploy succeeded.
- [ ] Optimizer accepts uploaded photos:

```bash
IMG=$(curl -s https://api.zainstreat.com/api/menu-items | grep -o 'https://api\.zainstreat\.com/api/media/[^"]*' | head -1)
ENC=$(python3 -c 'import sys,urllib.parse;print(urllib.parse.quote(sys.argv[1],safe=""))' "$IMG")
curl -s -o /dev/null -w "%{http_code} %{content_type}\n" "https://zainstreat.com/_next/image?url=$ENC&w=640&q=75"
# → 200 image/webp   (was: 400 "url" parameter is not allowed)
```

- [ ] End to end: in `/admin`, replace a menu item's photo and save. The
      new photo shows on `/en/menu`, on the home page's featured items
      (if featured), and as the thumbnail in `/admin`'s menu list.
- [ ] A cart that held the item *before* that edit shows the placeholder
      (or the new photo once re-added), never a broken image.
- [ ] `curl -s https://zainstreat.com/robots.txt` →
      `Sitemap: https://zainstreat.com/sitemap.xml`.
- [ ] `curl -s https://zainstreat.com/sitemap.xml | grep -c localhost` →
      `0`.

## Definition of done

An admin can replace a menu photo and the new photo appears on the public
site straight away, with `next/image` optimization still working (WebP,
cached). `robots.txt` and `sitemap.xml` advertise `https://zainstreat.com`.
A visitor whose cart still points at a replaced photo sees a placeholder,
not a broken image. `zainstreat.com` resolves only to the live instance,
so no visitor hits the dead IP. A web image built without `DOMAIN` fails
in CI instead of shipping silently.
