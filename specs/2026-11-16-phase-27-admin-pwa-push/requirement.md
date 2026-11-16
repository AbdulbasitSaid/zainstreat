# Phase 27 — Installable Admin (PWA) & Push Notifications: Requirements

## Source

`specs/roadmap.md`, Phase 27 (before this planning pass):

> To be investigated first: the client uses an iPhone, and iOS only
> supports web push for home-screen-installed PWAs (iOS 16.4+).
>
> - Web app manifest and service worker so the admin can be installed to
>   the home screen.
> - Web push to the owner for new orders and payment claims, even when the
>   admin isn't open. Tapping a notification opens the order in admin.

The user widened the events, chose the lock-screen content, the session
length and the app badge during planning (2026-11-16, Decisions 6, 8,
11 and 12).

## Context

### The iOS investigation

- **Web push on iPhone works only for a home-screen web app**, on iOS
  16.4 or later. In a normal Safari tab, `PushManager` doesn't exist.
- **Install is manual on iOS.** There's no `beforeinstallprompt`: the
  owner taps Share → Add to Home Screen in Safari. Since iOS 26, any page
  added to the home screen opens as a web app by default. The manifest
  still gives the name, icon, colours and `display: standalone`.
- **Permission must come from a tap.** `Notification.requestPermission()`
  only works inside a user gesture in the installed app, so the admin
  needs an explicit "Turn on notifications" button.
- **Every push must show a notification.** Safari has no silent push. A
  push that doesn't call `showNotification()` counts against the site,
  and repeated misses revoke the subscription. Every push in this phase
  shows one.
- **Apple's push service** is `web.push.apple.com`, reached through the
  standard Web Push protocol (RFC 8030) with VAPID (RFC 8292) and
  `aes128gcm` payload encryption (RFC 8291). No Apple developer account
  or certificate is needed. Chrome (FCM), Firefox and Edge use the same
  protocol, so one sender serves the owner's iPhone and any laptop.
- **App badge.** Installed web apps on iOS 16.4+ support
  `navigator.setAppBadge()`, also from the service worker. The badge is
  only shown if notifications are allowed.
- **Origin binds everything.** The installed app, its service worker and
  its push subscriptions belong to `https://cms.<host>`. Staging's host
  is `cms.<ip>.sslip.io`, and a new staging instance gets a new IP, so
  each one needs a fresh install.

### What exists

- **Admin host (Phase 21).** `proxy.ts` rewrites `cms.<host>` paths onto
  `app/admin/**`. The middleware matcher skips any path with a dot, so
  files in `public/` (such as `/sw.js`) are served as they are, on both
  hosts. `app/admin/layout.tsx` is its own root layout (`<html>`), with
  only a `title` in its metadata. There's no manifest, service worker,
  `theme-color` or Apple meta tag anywhere.
- **Admin icons.** `app/icon.png` is 256 × 256 (the favicon, Phase 25
  kept it a build-time asset). `public/logo.png` is 1254 × 1127 with
  transparency.
- **Admin nav.** `components/admin-nav.tsx` has a disabled "Settings"
  item that points at `/settings`.
- **Admin API access.** The browser never calls the API directly. Each
  admin call goes through a Next route handler in `app/api/admin/**`
  that forwards the session cookie. Server components use
  `lib/admin-api.ts`.
- **Sessions (Phase 11).** `tower-sessions` with
  `Expiry::OnInactivity(Duration::days(7))` in `apps/api/src/lib.rs`.
- **Owner notifications are emails only.** Six call sites, each inside
  `if let Some(business_address) = email.business_notify_address()`:

  | Event | Where | Admin link today |
  |---|---|---|
  | New order | `routes/orders.rs` `create_order` | `/orders/{id}` |
  | Payment claimed | `routes/orders.rs` `claim_payment` | `/orders/{id}` |
  | Quote accepted (creates an order) | `routes/quotes.rs` | `/orders/{id}` |
  | Quote declined | `routes/quotes.rs` | `/enquiries/catering/{enquiry_id}` |
  | New catering enquiry | `routes/catering_enquiries.rs` | none |
  | New contact message | `routes/contact_messages.rs` | none |

- **Order statuses** are `awaiting_payment`, `payment_claimed`,
  `processing`, `ready`, `completed` and `cancelled`.
  `GET /api/admin/orders/summary` already counts them.
- **The API's TLS stack** is rustls. `aws-lc-rs` (from `aws-sdk-s3`) and
  `ring` (from `lettre`) are already in `Cargo.lock`, and there's no
  OpenSSL in the build image (`rust:1-slim-bookworm`). There's no
  general HTTP client crate yet.

## Decisions

### Installable admin

1. **A static manifest for the admin only:
   `public/admin.webmanifest`.** It's linked from the admin root layout
   only. Next's `app/manifest.ts` would put the link in every layout,
   including the public site, which would then install as the admin.
   ```json
   {
     "id": "/",
     "name": "Zain's Admin",
     "short_name": "Zain's Admin",
     "start_url": "/",
     "scope": "/",
     "display": "standalone",
     "background_color": "#FFFFFF",
     "theme_color": "#72123C",
     "icons": [ "192", "512", "512 maskable" ]
   }
   ```
   `start_url` and `scope` are `/` because on `cms.<host>`, `/` is the
   dashboard. The colours are `--color-background` and `--color-primary`
   from `globals.css`. The file is
   also reachable on the public host, but nothing links it there.
2. **Icons are committed PNGs made from `public/logo.png`**, centred on
   the brand background: 192, 512, 512 maskable (logo inside the 80%
   safe zone) and a 180 × 180 `apple-touch-icon`. They live in
   `public/admin-icons/`. Like the favicon, they're build-time assets and
   don't follow the admin-editable logo slot (Phase 25).
3. **Admin root layout metadata** gains `manifest`, `appleWebApp`
   (`capable`, title "Zain's Admin", status bar `default`), the Apple
   touch icon, and a `viewport` export with `themeColor` and
   `viewportFit: "cover"`. The protected layout pads with
   `env(safe-area-inset-*)` so the notch and home indicator don't cover
   the nav or content in standalone mode.
4. **An install hint, no install button on iOS.** On the Settings page
   (Decision 9):
   - iOS Safari, not installed (`navigator.standalone !== true`): a card
     with the steps "Tap Share, then Add to Home Screen", explaining that
     notifications need it.
   - Chromium with a captured `beforeinstallprompt`: an "Install app"
     button.
   - Already installed (`display-mode: standalone`): nothing.
5. **The service worker is `public/sw.js`, plain JavaScript with no
   build step and no library.** It's registered from the protected admin
   layout with scope `/`, only on the cms host. It handles `push`,
   `notificationclick` and `pushsubscriptionchange`. It has **no fetch
   handler and no offline cache**. The admin shows live order data, so
   a cached page would mislead, and installability no longer needs a
   fetch handler. A `skipWaiting()`/`clients.claim()` pair makes updates
   apply on the next launch.

### Push notifications

6. **Push goes to the owner for every event that emails the owner today**
   (asked during planning: all of them):

   | Event | Title | Body | Opens |
   |---|---|---|---|
   | New order | `New order ZT-1042` | `€45.50 from Amina · Pickup` | `/orders/{id}` |
   | Quote accepted | `Quote #12 accepted` | `Order ZT-1043 · €480.00 from Amina` | `/orders/{id}` |
   | Payment claimed | `Payment claimed — ZT-1042` | `Amina says they've paid €45.50` | `/orders/{id}` |
   | New catering enquiry | `New catering enquiry` | `Amina · Wedding · 12-12-2026 · 80 guests` | `/enquiries/catering/{id}` |
   | Quote declined | `Quote #12 declined` | `Amina · <reason, or "No reason given">` | `/enquiries/catering/{enquiry_id}` |
   | New contact message | `New message from Amina` | `<subject>: <first 100 chars of message>` | `/enquiries/contact/{id}` |

   The admin is English only, so the copy is English. Claude writes it
   (`feedback_no_copy_approval`).
7. **Push is independent of email.** It's sent whenever push is
   configured, whether or not `EMAIL_BUSINESS_NOTIFY_ADDRESS` is set.
   Both are fire-and-forget (Phase 17): a failed push is logged and never
   fails or delays the request.
8. **The lock screen shows the customer's name** (asked during
   planning), as in the table above. The trade-off is known: anyone
   near the owner's phone can read it. iOS's "Show Previews: When
   Unlocked" setting hides the body on a locked phone if the owner wants
   that later. Addresses, phone numbers and emails are never in a push.
9. **The Settings page (`cms.<host>/settings`) manages notifications.**
   The nav's disabled "Settings" item is enabled. The page has a
   **Notifications** section:
   - The state on this device: not supported, needs install (iOS),
     blocked (permission denied, with how to fix it in iOS Settings),
     off, or on.
   - **Turn on notifications** (asks permission, subscribes, saves the
     subscription) and **Turn off** (unsubscribes and deletes it).
   - **Send test notification**, to every device of the signed-in admin.
   - **Devices**: every saved subscription with a label from its user
     agent (for example "iPhone · Safari" or "Mac · Chrome"), when it
     was added, when a push last got through, and **Remove**.

   Every device gets every event. Per-device or per-event settings are
   out of scope.
10. **Subscriptions belong to an admin user** and live in a new
    `push_subscriptions` table: `endpoint` (unique), `p256dh`, `auth`,
    `user_id`, `user_agent`, `created_at`, `last_success_at`,
    `last_failure_at`, `failure_count`. Saving an endpoint that already
    exists updates its keys and owner (re-subscribing on the same device
    never creates a duplicate). A push service answering **404 or 410**
    means the subscription is gone, so the row is deleted. Other errors
    are logged, and `failure_count` goes up. Every admin gets every
    event (there's one admin today).
11. **App badge: orders needing action** (asked during planning).
    - The count is orders in `awaiting_payment` or `payment_claimed`.
    - It travels in every push payload (`badge`), so the service worker
      sets it as each notification arrives.
    - The protected admin layout sets it again on every admin page load,
      from the same count, so it goes down as orders are handled.
      Without a notification permission iOS hides the badge; that's
      accepted.
12. **Admin sessions last 30 days of inactivity**, up from 7 (asked
    during planning). Push doesn't depend on the session, but tapping a
    notification after a quiet week shouldn't land on the login page.
    `tech-stack.md`'s Admin Authentication section is updated.
13. **Tapping a notification opens the right admin page.**
    `notificationclick` looks for an open admin window. If there is one,
    it focuses it and navigates it to the URL. If not, it opens one. The
    URL is a path on the admin origin (the payload carries `/orders/42`,
    never a full URL), so a push can't send the app elsewhere. If the
    session has expired, the existing login redirect applies.
14. **Notification options.** Each event uses a `tag` (`order-42`,
    `enquiry-7`, `contact-3`) so a payment claim replaces that order's
    "new order" notification instead of stacking. Push requests use
    `TTL: 86400` (a phone that's off for a day still gets them) and
    `Urgency: high`.

### Sending from the API

15. **Library: `web-push-native` 0.5 plus `reqwest` 0.13 (rustls).**
    - `web-push-native` is pure Rust. It does the VAPID signing (with its
      `vapid` feature) and the `aes128gcm` encryption, and returns an
      `http::Request`. `reqwest` sends it, reusing the rustls stack
      already in the build.
    - _Why not the `web-push` crate (0.11):_ it pulls in the `ece` crate's
      OpenSSL backend, `http` 0.2 and `hyper` 0.14 or `isahc`. That means
      OpenSSL in the Docker build and two old HTTP stacks. Its last
      release was 2025-02.
    - _Why not a hosted push service (OneSignal, FCM directly):_ the Web
      Push protocol is enough and needs no account, no third-party SDK in
      the admin and no extra privacy terms.
16. **VAPID keys are new env variables**: `VAPID_PUBLIC_KEY` (a GitHub
    Environment variable), `VAPID_PRIVATE_KEY` (a secret) and
    `VAPID_SUBJECT` (`mailto:contact@zainstreat.com`). They're wired like
    the email variables. Staging and production get **separate** key
    pairs. If any is unset, push is off: the API logs it once at
    startup, and the Settings page says notifications aren't configured
    on this server.
    - Keys come from a new `apps/api/src/bin/generate_vapid_keys.rs`
      (prints both keys base64url-encoded), so nobody needs a Node tool.
    - The browser gets the public key from `GET
      /api/admin/push/public-key` at subscribe time, not at build time,
      so the web image stays the same across environments.
    - Rotating the keys invalidates every subscription. Devices then
      need "Turn on notifications" again.

## Out of scope

- WhatsApp owner notifications (Phase 28).
- Push to **customers** (order status). Customers aren't installing an
  app, and they get email (Phase 17/22).
- An offline admin, caching or background sync (Decision 5).
- Per-device or per-event notification settings, quiet hours, and
  notification sounds beyond the system default.
- A PWA for the public site. Nothing links its manifest there.
- Making the admin-editable logo (Phase 25) the app icon (Decision 2).
- Production rollout (`project_production_deferred`). This phase
  creates production's VAPID keys and variables only. The promote that
  makes them live is the normal Phase 21 flow.
- Phase 26 (analytics) and Phase 29 (moving AWS accounts). Push uses no
  AWS service, so Phase 29 doesn't touch it beyond copying the env
  variables.

## Open risks flagged during planning

1. **The owner's iPhone may run an iOS version below 16.4**, or Safari
   with a different default browser. *Mitigation:* the Settings page
   shows "not supported" with the reason instead of failing silently.
   Check the iOS version with the owner before the staging test
   (validation).
2. **Staging subscriptions don't survive a new staging instance.** Its
   host changes with its IP, and its database is re-seeded. *Accepted:*
   reinstall after each new staging instance. Production's host is
   fixed.
3. **iOS may drop a subscription** (after a reinstall, an iOS update,
   or when the owner clears website data). *Mitigation:* the service
   worker's `pushsubscriptionchange` re-subscribes and saves the new
   subscription when the browser fires it. The Devices list shows
   "last delivered", and a dead endpoint is removed on the next 404/410.
   Email keeps working either way.
4. **Lock-screen privacy** (Decision 8). *Accepted by the user.*
5. **Env parity** (Phase 21 Decision 14). The three new keys show up as
   changed on the first deploy to each environment. *Mitigation:* pass
   them in the parity allow list for that deploy (plan 6.3).
6. **Push services can be slow or down.** *Mitigation:* sends happen in
   a spawned task with a 10 s timeout per request, so the order request
   never waits on them (Decision 7).
7. **The developer can't test on an iPhone** (found after planning).
   *Mitigation:* the developer tests on Android Chrome and desktop
   Chrome, which use the same Web Push protocol, the same VAPID keys and
   the same service worker. The iOS-only parts are the install steps, the
   permission prompt in the Home Screen app, Apple's push service, the
   safe areas and the app badge. The client tests those on their iPhone
   in a later round (plan 7.3), and the phase isn't done until that
   round passes. The Android badge isn't checked: Android Chrome doesn't
   support `setAppBadge` and shows its own notification dot instead.
