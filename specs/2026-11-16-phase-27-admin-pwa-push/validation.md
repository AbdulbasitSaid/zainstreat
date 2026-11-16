# Phase 27 — Installable Admin (PWA) & Push Notifications: Validation

## Pass/fail checklist

### Installable admin (Group 1)

- [ ] `curl -s https://cms.<staging-host>/admin.webmanifest` returns the
      JSON from requirement.md Decision 1 with
      `Content-Type: application/manifest+json`, and each icon URL in it
      returns `200 image/png` at the declared size.
- [ ] The admin HTML (`curl -s https://cms.<staging-host>/login`) has
      `<link rel="manifest" href="/admin.webmanifest">`,
      `apple-touch-icon`, `apple-mobile-web-app-capable`,
      `theme-color` `#72123C` and `viewport-fit=cover`.
- [ ] The public site's HTML (`curl -s https://<staging-host>/en`) has
      **no** `rel="manifest"` link and no `apple-mobile-web-app-capable`.
- [ ] `curl -sI https://cms.<staging-host>/sw.js` returns `200` with a
      JavaScript content type.
- [ ] Chrome DevTools → Application on `cms.<staging-host>`: the manifest
      parses with no errors, the service worker is "activated and
      running" with scope `/`, and Chrome offers to install the app.
- [ ] The service worker has no `fetch` listener
      (`grep -c "\"fetch\"" apps/web/public/sw.js` → `0`).
- [ ] The Settings nav item is a link and `cms.<host>/settings` renders.

### Subscriptions API (Group 2)

- [ ] `cargo test` passes, including `tests/admin_push.rs`.
- [ ] `cargo clippy --all-targets -- -D warnings` passes.
- [ ] `curl -s -o /dev/null -w '%{http_code}' https://api.<staging-host>/api/admin/push/subscriptions`
      without a cookie → `401`.
- [ ] `cargo run --bin generate_vapid_keys` prints a 65-byte (87-char)
      public key and a 32-byte (43-char) private key, both unpadded
      base64url.
- [ ] With the VAPID variables unset, the API starts, logs that push is
      off once, and `GET /api/admin/push/public-key` (signed in) → `404`.
- [ ] `apps/api/Cargo.lock` has no `openssl` or `openssl-sys` package
      (`grep -c 'name = "openssl' apps/api/Cargo.lock` → `0`), and
      `docker compose build api` succeeds unchanged.
- [ ] Admin session cookie lifetime: after login, the session row's
      expiry is about 30 days ahead
      (`SELECT expiry_date FROM tower_sessions.session ORDER BY expiry_date DESC LIMIT 1`).

### Sending (Groups 3, 4)

- [ ] The unit tests for each message builder, truncation and the
      `/`-path check pass.
- [ ] The integration test with a local listener shows one encrypted
      `POST` per subscription with `TTL: 86400`, `Urgency: high`, a
      `vapid t=…, k=…` authorization header, and the row deleted on
      `410`.
- [ ] With push configured and no `EMAIL_BUSINESS_NOTIFY_ADDRESS`, a new
      order still sends a push (Decision 7).
- [ ] A push to an unreachable endpoint doesn't slow `POST /api/orders`:
      it still answers in well under a second. The API logs a `warn!`
      with the host only, never the full endpoint.

### Device checks on staging (Groups 5, 7)

Two rounds (plan 7.2–7.3): the developer on Android and desktop Chrome,
then the client on their iPhone.

#### Setup

- [ ] `gh variable list --env staging` shows `VAPID_PUBLIC_KEY` and
      `VAPID_SUBJECT=mailto:contact@zainstreat.com`, and
      `gh secret list --env staging` shows `VAPID_PRIVATE_KEY`.
      Production has its own, different pair.

#### Android, Chrome (developer)

- [ ] Settings shows **Install app**. Installing puts the Zain's Admin
      icon (not a cropped or white-boxed logo) in the launcher, and it
      opens without the browser bar, in the brand theme colour.
- [ ] Installed app → Settings → **Turn on notifications** shows
      Android's permission prompt. After allowing, the state reads "on"
      and the device appears as "Android · Chrome · This device".
- [ ] **Send test notification** arrives with the app closed and the
      screen locked. Tapping it opens the app on Settings.
- [ ] Each event, with the app closed, arrives with the title and body
      from requirement.md Decision 6 (customer name included), and
      tapping it opens the right page:
  - [ ] new order → `/orders/{id}`
  - [ ] payment claimed → same order. It **replaces** that order's
        "new order" notification (same tag) instead of adding a second.
  - [ ] quote accepted → the new order
  - [ ] quote declined → `/enquiries/catering/{enquiry_id}`
  - [ ] new catering enquiry → `/enquiries/catering/{id}`
  - [ ] new contact message → `/enquiries/contact/{id}`
- [ ] With the app already open on another page, tapping a notification
      brings it to the front and navigates it (no second window).
- [ ] **Remove** on a device deletes it, and the next event doesn't
      reach it. **Turn off** on the current device does the same.
- [ ] Denying permission (or blocking it in Android's app settings)
      shows the "blocked" state.

#### Desktop Chrome (developer)

- [ ] Turning on notifications adds a second device ("Mac · Chrome" or
      similar), and a test notification and a new order reach both
      devices.
- [ ] Installed as an app, the dock or taskbar icon shows the badge
      with the number of `awaiting_payment` + `payment_claimed` orders
      after a push, and it goes down after an order is confirmed and the
      admin is opened again.

#### iPhone (client, later)

- [ ] The owner's iPhone runs iOS 16.4 or later (plan 0.2).
- [ ] Safari, not installed: Settings shows the "Share → Add to Home
      Screen" steps, and "Turn on notifications" isn't offered yet.
- [ ] After adding, the Home Screen shows the Zain's Admin icon (not a
      screenshot or black square), and it opens without Safari's
      address bar.
- [ ] The nav and content aren't hidden under the notch or the home
      indicator, on the dashboard, an order and login.
- [ ] **Turn on notifications** shows the iOS prompt. After allowing,
      the device appears as "iPhone · Safari".
- [ ] A test notification, then a real new order and a payment claim,
      arrive with the app closed and the phone locked. Tapping each
      opens the right page.
- [ ] The Home Screen icon badge shows the orders-needing-action count.

#### Both rounds

- [ ] Owner emails still arrive for every event, unchanged.

### Repo and docs (Group 6)

- [ ] `docker-compose.yml`, `_deploy.yml` and `.env.example` include the
      three VAPID variables. The private key is only ever read from
      `secrets.`.
- [ ] `git grep -n "VAPID_PRIVATE_KEY=" -- ':!*.example'` shows no
      real key anywhere in the repo.
- [ ] `specs/tech-stack.md` has the "Admin PWA & Web Push" section and
      Summary Table row, and says 30-day sessions.
- [ ] `docs/deployment.md` covers the VAPID keys, rotation and the
      staging reinstall. The owner how-to exists.
- [ ] `pnpm lint`, `pnpm build` and `pnpm check:messages` pass in
      `apps/web`, with no `any`.
- [ ] `/validate` passes.

## Definition of done

The admin at `cms.<host>` installs to the iPhone Home Screen (and to a
desktop from Chrome) as "Zain's Admin" with the brand icon and colour,
opens full screen without hiding anything under the notch, and the
public site stays a plain website. From Settings the owner can turn on
notifications per device, send a test, and see and remove devices. Every
event that emails the owner (new order, quote accepted, payment claimed,
catering enquiry, quote declined, contact message) also sends a push,
with the customer's name, to every subscribed device, even when the app
is closed. Tapping it opens the matching admin page, and the app icon
shows how many orders are waiting on payment or its confirmation. Push
runs on standard Web Push with VAPID from the Rust API (no third-party
service, no OpenSSL), is fire-and-forget like email, and is off when its
keys aren't set. Admin sessions last 30 days. `cargo test`, clippy,
lint, build and `/validate` pass, and the device checks pass on staging:
Android and desktop Chrome by the developer, then the owner's iPhone.
