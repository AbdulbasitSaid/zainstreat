# Phase 27 — Installable Admin (PWA) & Push Notifications: Implementation Plan

**Ordering:**
- Group 1 (installable admin) is web-only and doesn't depend on push, so
  it can ship first. Run `/validate` after it.
- Groups 2–4 (API) come before Group 5 (web push UI). Run `cargo test`
  after Groups 2, 3 and 4.
- Group 6 (env, keys, docs) can run alongside Group 5. Group 7 needs
  staging and the owner's iPhone.

Paths refer to `2026-11-01-phase-25-site-content-preview` at `835e6af`.

## Group 0 — Preconditions

Depends on: nothing.

0.1. **Phase 25 is merged to `master`** before this branch is rebased
     onto it.
0.2. **Ask the owner their iOS version** (Settings → General → About).
     It must be 16.4 or later (requirement.md risk 1). This doesn't block
     implementation, but it should be known before their test round
     (plan 7.3).
0.3. **Staging is up** for Group 7.

## Group 1 — Installable admin (web)

Depends on: Group 0. requirement.md Decisions 1–5.

1.1. **Icons** in `apps/web/public/admin-icons/`, made once from
     `public/logo.png` and committed:
     - `icon-192.png`, `icon-512.png`: logo centred at about 80% width on
       `#FFFFFF`.
     - `icon-maskable-512.png`: logo inside the central 80% circle (about
       60% width) on `#FFFFFF`.
     - `apple-touch-icon.png`: 180 × 180, same as `icon-192` but no
       transparency (iOS fills transparency with black).

     Make them with macOS `sips` plus a padded canvas, or a one-off
     `node` script using `sharp` from the scratchpad. Don't add a
     dependency or a script to the repo for this.
1.2. **`apps/web/public/admin.webmanifest`** exactly as requirement.md
     Decision 1, with the three icons
     (`"purpose": "any"` for 192/512, `"maskable"` for the maskable one).
1.3. **`apps/web/app/admin/layout.tsx`**:
     ```tsx
     export const metadata: Metadata = {
       title: "Admin | Zain's Treat n More",
       manifest: "/admin.webmanifest",
       appleWebApp: { capable: true, title: "Zain's Admin", statusBarStyle: "default" },
       icons: { apple: "/admin-icons/apple-touch-icon.png" },
     };

     export const viewport: Viewport = {
       themeColor: "#72123C",
       viewportFit: "cover",
     };
     ```
1.4. **Safe areas.** In `app/admin/(protected)/layout.tsx`, the outer
     wrapper gets `pt-[env(safe-area-inset-top)]`,
     `pb-[env(safe-area-inset-bottom)]` and the matching left/right
     insets. Check the login page too (`app/admin/login/page.tsx`).
1.5. **`apps/web/public/sw.js`** (plain JS, no imports). For this group
     it only has:
     ```js
     self.addEventListener("install", () => self.skipWaiting());
     self.addEventListener("activate", (event) => event.waitUntil(self.clients.claim()));
     ```
     Group 5 adds the push handlers. Check that `pnpm lint` either
     ignores `public/` or accepts the file. If ESLint picks it up, add a
     `globals.serviceworker` override for `public/sw.js` in
     `eslint.config.mjs`.
1.6. **`apps/web/components/admin-service-worker.tsx`**, a client
     component rendered in the protected layout. On mount, if
     `"serviceWorker" in navigator` and `location.hostname` starts with
     `cms.`, it calls
     `navigator.serviceWorker.register("/sw.js", { scope: "/", updateViaCache: "none" })`.
     Errors are logged with `console.warn`, never thrown.
1.7. **Enable Settings.** In `components/admin-nav.tsx`, set the
     Settings item to `enabled: true`. Create
     `app/admin/(protected)/settings/page.tsx` with a "Notifications"
     heading and the `AdminInstallHint` component:
     - `components/admin-install-hint.tsx` (client): detects
       standalone (`matchMedia("(display-mode: standalone)")` or
       `navigator.standalone === true`), iOS (`/iPhone|iPad/` in the UA,
       or `MacIntel` with touch points > 1), and a captured
       `beforeinstallprompt` event. Renders the iOS steps card, the
       "Install app" button, or nothing (Decision 4).
     - `navigator.standalone` isn't in TypeScript's DOM lib. Declare it
       with a narrow interface (`interface NavigatorStandalone { standalone?: boolean }`),
       not `any`.
     - `BeforeInstallPromptEvent` isn't in the DOM lib either. Declare
       the two members used (`prompt(): Promise<void>`,
       `userChoice: Promise<{ outcome: "accepted" | "dismissed" }>`).

## Group 2 — Push subscriptions (API)

Depends on: nothing. requirement.md Decisions 9, 10, 12, 16.

2.1. **Dependencies** in `apps/api/Cargo.toml` (check the latest patch
     versions with `cargo search` before pinning):
     ```toml
     web-push-native = { version = "0.5", features = ["vapid", "serialization"] }
     reqwest = { version = "0.13", default-features = false, features = ["rustls", "json"] }
     base64ct = { version = "1", features = ["alloc"] }
     ```
     `reqwest`'s `rustls` feature uses `aws-lc-rs`, which is already in
     `Cargo.lock` through `aws-sdk-s3`, so the Docker build needs no new
     system packages. Confirm with a `docker compose build api`.
2.2. **Migration** `apps/api/migrations/<ts>_push_subscriptions.up.sql`:
     ```sql
     CREATE TABLE push_subscriptions (
         id BIGSERIAL PRIMARY KEY,
         user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
         endpoint TEXT NOT NULL UNIQUE,
         p256dh TEXT NOT NULL,
         auth TEXT NOT NULL,
         user_agent TEXT,
         created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
         last_success_at TIMESTAMPTZ,
         last_failure_at TIMESTAMPTZ,
         failure_count INT NOT NULL DEFAULT 0
     );
     CREATE INDEX push_subscriptions_user_id_idx ON push_subscriptions (user_id);
     ```
     Check `users.id`'s type in the initial schema and match it.
     The `.down.sql` drops the table.
2.3. **`apps/api/src/push.rs`: `PushConfig`.**
     ```rust
     #[derive(Clone)]
     pub struct PushConfig { inner: Option<Arc<PushSettings>> }
     struct PushSettings {
         public_key: String,          // base64url, uncompressed P-256 point
         key_pair: ES256KeyPair,      // from VAPID_PRIVATE_KEY
         subject: String,             // VAPID_SUBJECT
         client: reqwest::Client,     // 10 s timeout
     }
     impl PushConfig {
         pub fn from_env() -> Self;   // None + one `info!` if any var is unset/blank
         pub fn public_key(&self) -> Option<&str>;
         pub fn is_enabled(&self) -> bool;
     }
     ```
     Same `non_blank` helper style as `email.rs`. An unparseable private
     key logs an `error!` and disables push; it doesn't stop the API.
     Add `push: PushConfig` to `AppState` with a `FromRef` impl.
2.4. **Admin endpoints** in `apps/api/src/routes/admin/push.rs`, behind
     `require_admin`, added to `routes/admin/mod.rs`:

     | Method & path | Body / response |
     |---|---|
     | `GET /api/admin/push/public-key` | `{ "public_key": "…" }`, or `404 push_not_configured` |
     | `GET /api/admin/push/subscriptions` | `[{ id, label, created_at, last_success_at, last_failure_at, is_current: false }]` for the signed-in admin |
     | `POST /api/admin/push/subscriptions` | `{ endpoint, keys: { p256dh, auth } }` → `201` (`200` when the endpoint already existed). Upserts on `endpoint`, setting `user_id`, keys and `user_agent` from the request header |
     | `DELETE /api/admin/push/subscriptions/{id}` | `204`. Only the signed-in admin's own rows |
     | `POST /api/admin/push/test` | `202`. Sends "Test notification" / "Notifications are working on this device." to the signed-in admin's devices, URL `/settings` |

     Validation: `endpoint` must be `https://` and at most 1000 chars,
     `p256dh` and `auth` base64url and at most 200 chars, or `400` with
     `field`. `label` is computed server-side from `user_agent` (device:
     iPhone/iPad/Android/Mac/Windows/Linux; browser:
     Safari/Chrome/Firefox/Edge), "Unknown device" otherwise. The web
     marks `is_current` by comparing endpoints.
2.5. **Session length** (Decision 12): `apps/api/src/lib.rs`
     `Expiry::OnInactivity(Duration::days(30))`.
2.6. **`apps/api/src/bin/generate_vapid_keys.rs`**: generates an
     `ES256KeyPair`, prints
     ```
     VAPID_PUBLIC_KEY=<base64url, 65-byte uncompressed point>
     VAPID_PRIVATE_KEY=<base64url, 32-byte scalar>
     ```
     Unpadded base64url, the format `PushManager.subscribe` expects for
     `applicationServerKey`.
2.7. **Tests** in `apps/api/tests/admin_push.rs` (follow
     `tests/common.rs`):
     - every endpoint gives `401` without a session;
     - `public-key` gives `404` when push isn't configured, the key when
       it is;
     - subscribe → list shows it with a label → subscribe again with the
       same endpoint keeps one row → delete removes it;
     - a second admin can't delete the first admin's row (`404`);
     - bad `endpoint` (`http://`, too long) and bad keys give `400`.

## Group 3 — Sending pushes (API)

Depends on: Group 2. requirement.md Decisions 6–8, 10, 11, 13–15.

3.1. **`push.rs`: the payload and the spawn.**
     ```rust
     #[derive(Serialize)]
     pub struct OwnerPush {
         pub title: String,
         pub body: String,
         pub url: String,   // admin path, must start with '/'
         pub tag: String,
     }

     impl PushConfig {
         /// Fire-and-forget (Decision 7). Sends to every admin's devices.
         pub fn spawn_notify_owner(&self, pool: &PgPool, push: OwnerPush);
         /// Same, to one admin's devices (the test button).
         pub fn spawn_notify_user(&self, pool: &PgPool, user_id: i64, push: OwnerPush);
     }
     ```
     Inside the spawned task:
     1. Count orders `WHERE status IN ('awaiting_payment', 'payment_claimed')`
        for `badge` (Decision 11). If the query fails, leave `badge` out.
     2. Serialize `{ title, body, url, tag, badge }`. Truncate `body` to
        200 chars on a char boundary, keeping the payload far below the
        4 KB limit.
     3. Load the subscriptions and send to each one concurrently
        (`futures::future::join_all`, or a `JoinSet`). For each one,
        `WebPushBuilder::new(endpoint, p256dh, auth)
        .with_vapid(&key_pair, &subject).with_ttl(86400)
        .with_urgency(Urgency::High).build(payload)`, then convert the
        `http::Request<Vec<u8>>` into a `reqwest::Request` and send it.
     4. `2xx`: set `last_success_at = now()`, `failure_count = 0`.
        `404`/`410`: delete the row and `info!` it. Anything else, or a
        timeout: `warn!` with the status and the endpoint's host only
        (never the full endpoint, which works as a credential), and set
        `last_failure_at`, `failure_count + 1`.
     The `url` must start with `/` and not `//`. A unit test covers it,
     and it's checked again in the service worker (Group 5).
3.2. **Message builders** in `push.rs`, one per event, so the text lives
     in one place and is unit-tested:
     ```rust
     pub fn new_order(order: &OrderResponse) -> OwnerPush;
     pub fn quote_accepted(quote_id: i64, order: &OrderResponse) -> OwnerPush;
     pub fn payment_claimed(order: &OrderResponse) -> OwnerPush;
     pub fn catering_enquiry(id: i64, name: &str, event_type: &str, event_date: NaiveDate, guests: i32) -> OwnerPush;
     pub fn quote_declined(quote_id: i64, enquiry_id: i64, name: &str, reason: Option<&str>) -> OwnerPush;
     pub fn contact_message(id: i64, name: &str, subject: &str, message: &str) -> OwnerPush;
     ```
     Text and tags as in requirement.md Decisions 6 and 14. Money uses the
     same `€{}` formatting as the owner emails, and dates use `%d-%m-%Y`.
     Match the field types to what each call site actually has.
3.3. **Unit tests** for each builder (title, body, url, tag), body
     truncation, the `/`-path check, and `PushConfig::from_env` with
     missing or blank variables.

## Group 4 — Call sites (API)

Depends on: Group 3.

4.1. Add a `State(push): State<PushConfig>` extractor to each handler
     below and call `push.spawn_notify_owner(&pool, …)` next to the
     owner email. It goes **outside** the
     `if let Some(business_address)` block (Decision 7), and after the
     commit, like the email.

     | Handler | Builder |
     |---|---|
     | `routes/orders.rs` `create_order` | `push::new_order(&response)` |
     | `routes/orders.rs` `claim_payment` | `push::payment_claimed(&order)` (only when this request did the claim, i.e. inside the `claimed` branch) |
     | `routes/quotes.rs` `accept_quote` | `push::quote_accepted(quote.id, &response)` |
     | `routes/quotes.rs` `decline_quote` | `push::quote_declined(…)` (only when this request did the decline) |
     | `routes/catering_enquiries.rs` `create_catering_enquiry` | `push::catering_enquiry(id, …)`; the insert already returns `id` |
     | `routes/contact_messages.rs` `create_contact_message` | `push::contact_message(id, …)` |

4.2. **Tests.** With push not configured (the default in tests), the
     existing order, quote, enquiry and contact tests still pass
     unchanged. Add one test with push configured (a generated key pair)
     and a subscription pointing at a local `axum` listener on
     `127.0.0.1:0` that records requests: creating an order sends one
     `POST` with `Content-Encoding: aes128gcm`, `TTL: 86400`,
     `Urgency: high` and a `vapid` `Authorization` header, and a listener
     answering `410` gets the row deleted. Allow `http://127.0.0.1` in
     the endpoint check under `#[cfg(test)]` only, or insert the row
     directly in the test so the validation stays strict.

## Group 5 — Push in the admin (web)

Depends on: Groups 1–4. requirement.md Decisions 9, 11, 13.

5.1. **Route handlers** in `apps/web/app/api/admin/push/`, same shape as
     `app/api/admin/logout/route.ts` (forward the cookie and the
     `user-agent`, return the API's status and body):
     - `public-key/route.ts` (GET)
     - `subscriptions/route.ts` (GET, POST)
     - `subscriptions/[id]/route.ts` (DELETE)
     - `test/route.ts` (POST)
5.2. **`public/sw.js` push handlers:**
     ```js
     self.addEventListener("push", (event) => {
       const data = event.data ? event.data.json() : {};
       const url = typeof data.url === "string" && data.url.startsWith("/") && !data.url.startsWith("//")
         ? data.url : "/";
       const tasks = [
         self.registration.showNotification(data.title || "Zain's Admin", {
           body: data.body || "",
           tag: data.tag,
           icon: "/admin-icons/icon-192.png",
           badge: "/admin-icons/icon-192.png",
           data: { url },
         }),
       ];
       if (typeof data.badge === "number" && "setAppBadge" in self.navigator) {
         tasks.push(data.badge > 0 ? self.navigator.setAppBadge(data.badge) : self.navigator.clearAppBadge());
       }
       event.waitUntil(Promise.all(tasks));
     });

     self.addEventListener("notificationclick", (event) => {
       event.notification.close();
       const url = new URL(event.notification.data?.url || "/", self.location.origin).href;
       event.waitUntil((async () => {
         const windows = await self.clients.matchAll({ type: "window", includeUncontrolled: true });
         const client = windows.find((c) => new URL(c.url).origin === self.location.origin);
         if (client) { await client.focus(); return client.navigate(url); }
         return self.clients.openWindow(url);
       })());
     });

     self.addEventListener("pushsubscriptionchange", (event) => {
       event.waitUntil((async () => {
         const res = await fetch("/api/admin/push/public-key", { credentials: "same-origin" });
         if (!res.ok) return;
         const { public_key } = await res.json();
         const sub = await self.registration.pushManager.subscribe({
           userVisibleOnly: true, applicationServerKey: public_key,
         });
         await fetch("/api/admin/push/subscriptions", {
           method: "POST", credentials: "same-origin",
           headers: { "Content-Type": "application/json" },
           body: JSON.stringify(sub.toJSON()),
         });
       })());
     });
     ```
     `showNotification` always runs, so no push is ever silent (Context,
     "Every push must show a notification").
5.3. **`lib/push-client.ts`** (client-only helpers, typed, no `any`):
     - `pushSupport(): "supported" | "needs-install" | "unsupported"`:
       `"serviceWorker" in navigator && "PushManager" in window` →
       supported; iOS and not standalone → needs-install; else
       unsupported.
     - `subscribe(): Promise<void>`: must be called from a click. In
       order: `Notification.requestPermission()`, fetch the public key,
       `registration.pushManager.subscribe({ userVisibleOnly: true,
       applicationServerKey })`, POST `subscription.toJSON()`. Throws a
       typed error (`"denied" | "not-configured" | "failed"`) the UI can
       show.
     - `unsubscribe(): Promise<void>`: deletes the matching server row
       (by finding the current endpoint in the list), then
       `subscription.unsubscribe()`.
     - `currentEndpoint(): Promise<string | null>`.
     `applicationServerKey` takes the base64url string directly in
     current Safari and Chromium. If the type checker wants a
     `BufferSource`, convert with a small `base64UrlToUint8Array` helper.
5.4. **`components/admin-notifications.tsx`** (client), on the Settings
     page under `AdminInstallHint`:
     - **State line**: "Notifications are on for this device", "off",
       "blocked" (with "Open Settings → Notifications → Zain's Admin and
       allow notifications"), "Add this app to your Home Screen first"
       (needs-install), "This browser doesn't support notifications", or
       "Notifications aren't set up on this server" (public key `404`).
     - **Buttons**: Turn on / Turn off / Send test notification. Each one
       is disabled while it runs and shows the result inline (no
       `alert()`).
     - **Devices** list from `GET /api/admin/push/subscriptions`: label,
       "This device" for the current endpoint, added date, last
       delivered (or "Never"), a "Delivery failing" note when
       `last_failure_at` is after `last_success_at`, and **Remove**
       (through `AdminConfirmDialog`).
     Dates use the existing admin date helper (Europe/Amsterdam).
5.5. **App badge on page load** (Decision 11). The protected layout
     already loads data server-side. Fetch
     `/api/admin/orders/summary` there (with the same `.catch` fallback
     as the changes count), sum `awaiting_payment` + `payment_claimed`,
     and pass it to `AdminServiceWorker`, which calls
     `navigator.setAppBadge(n)` or `clearAppBadge()` when
     `"setAppBadge" in navigator`. Use a typed feature check, not
     `any`.

## Group 6 — Keys, env and docs (repo + ops)

Depends on: Group 2.

6.1. **Env wiring** for `VAPID_PUBLIC_KEY`, `VAPID_PRIVATE_KEY`,
     `VAPID_SUBJECT`, the same way as `EMAIL_SUBJECT_PREFIX`:
     - `docker-compose.yml` api `environment` (`${VAPID_…:-}`)
     - `.github/workflows/_deploy.yml` "Render .env.next" (`V_VAPID_PUBLIC_KEY`
       and `V_VAPID_SUBJECT` from `vars`, `V_VAPID_PRIVATE_KEY` from
       `secrets`). Not in `required`, so push can stay off.
     - `.env.example`, commented, with `# [var]`/`# [secret]` tags and
       a pointer to `generate_vapid_keys`.
6.2. **Generate two key pairs** with
     `cargo run --bin generate_vapid_keys`, one for staging and one for
     production. Set them with `gh variable set … --env <env>` and
     `gh secret set VAPID_PRIVATE_KEY --env <env>`, plus
     `VAPID_SUBJECT=mailto:contact@zainstreat.com`. The private keys are
     never written to the repo, a spec or a log. Confirm with the user
     before setting production's.
6.3. **First deploy per environment**: run it with
     `allow_env_changes: VAPID_PUBLIC_KEY,VAPID_PRIVATE_KEY,VAPID_SUBJECT`
     (requirement.md risk 5).
6.4. **`specs/tech-stack.md`**: the new "Admin PWA & Web Push" section,
     its Summary Table row, and the Admin Authentication session length
     (30 days). Done during planning; check it still matches what was
     built.
6.5. **`docs/deployment.md`**: the VAPID variables, how to generate and
     rotate them (rotation means every device turns notifications on
     again), and "after a new staging instance, reinstall the admin app
     and turn notifications on again".
6.6. **Admin how-to** for the owner, in `docs/` next to any existing
     admin guide (or a new `docs/admin-app.md`): install on iPhone
     (Safari → Share → Add to Home Screen), open it from the Home Screen,
     Settings → Turn on notifications, and what each notification means.
     Short, with no screenshots needed.

## Group 7 — Staging and device checks

Depends on: Groups 1–6.

The developer has no iPhone (2026-11-16), so testing happens in two
rounds.

7.1. Deploy to staging and run `/validate`.
7.2. **Developer, on Android (Chrome) and a desktop Chrome:** install
     from Chrome's prompt or the Settings page's "Install app" button,
     then run validation.md's shared and Android device checks. Android
     Chrome can also get push without installing. Test the installed
     app, because that's what the owner will use.
7.3. **Client, on their iPhone, later:** send them the owner how-to
     (plan 6.6) and a staging link, then go through validation.md's
     iPhone checks with them (on a call or by screenshots). The phase's
     code can be merged before this round, but the phase isn't done
     until it passes.

## Implementation notes

2026-11-16, Groups 1–5:

- **`web-push-native` has no Urgency option, and ties TTL to the VAPID
  token's lifetime.** The token stays at 12 h (Apple rejects tokens that
  expire more than 24 h out). `TTL: 86400` and `Urgency: high` are set on
  the `reqwest::Request` after the build (`push.rs` `build_request`).
- **`PushConfig::from_keys` checks that the public key matches the private
  key** and turns push off with an error log if not, instead of letting
  every send fail with a 403.
- **The device list returns each row's `endpoint`** (only to the admin
  who owns it), so the web can mark "This device" by comparing endpoints.
  There's no `is_current` field.
- **`lib/admin-proxy.ts` now forwards `User-Agent`** for every admin proxy
  route, so device labels work through the cms proxy.
- **Turning on with an existing browser subscription made with a
  different key** (after a key rotation) unsubscribes it and subscribes
  again (`lib/push-client.ts` `subscribe`).
- **Step 6.1 done early** (`docker-compose.yml`, `.env.example`) so push
  can be tried locally. A local dev key pair is in the gitignored root
  `.env`.

2026-11-17, Group 6:

- `_deploy.yml` renders the three VAPID variables. The private key comes
  from `secrets.` only, and none are in `required`.
- **Staging's** key pair is set (`VAPID_PUBLIC_KEY` and `VAPID_SUBJECT`
  variables, `VAPID_PRIVATE_KEY` secret). The private key was piped from
  `generate_vapid_keys` straight into `gh secret set` and never printed.
  **Production's** pair was set the same way after the user approved it
  (2026-11-17). It's a different pair from staging's. It takes effect at
  the next Promote, with the 6.3 allow list.
- `docs/deployment.md` has a "Push notifications and the admin app"
  section and a staging note. The owner how-to is the new
  `docs/admin-app.md` (iPhone, Android, computer, what each notification
  means, troubleshooting).
- `tests/admin_push.rs` runs a fake push service on `127.0.0.1` and
  decrypts the payload with the browser-side key. It covers headers,
  payload, badge, `last_success_at`, and deleting a row on `410`.
