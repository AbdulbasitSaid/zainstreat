# Phase 33 — GDPR Compliance & Security Hardening: Plan

Decisions refer to `requirement.md`. Findings S1–S11 are its audit table.

## Group 0 — Preconditions

0.1. Branch `2026-11-17-phase-33-gdpr-security` from `master` (Phase 32
     at `8183370`).
0.2. Local stack up (`docker compose up -d`). `cargo audit` installed
     (`cargo install cargo-audit --locked`) for Groups 1–2.

## Group 1 — Next.js RCE fix (S1, Decision 1) — implement first

1.1. `apps/web/package.json`: `"next": "16.4.6"` and
     `"eslint-config-next": "16.4.6"` (exact pins; 16.4.6 is the latest
     16.x on 2026-11-17 and is past every advisory's fix). `pnpm install`;
     confirm `node -p "require('next/package.json').version"` → `16.4.6`
     and that `next-intl` resolves the same `next`.
1.2. `apps/web/next.config.ts` `images`: delete `dangerouslyAllowSVG`,
     `contentDispositionType` and `contentSecurityPolicy`, and the
     `placehold.co` entry in `remotePatterns`. Before deleting, confirm
     no `next/image` `src` points at an `.svg` or `placehold.co`:
     `grep -rn "placehold.co\|\.svg" apps/web/app apps/web/components apps/web/lib`.
     Local SVGs that must stay go through a plain `<img>`, not the
     optimizer.
1.3. (Done in planning: `specs/tech-stack.md` Site imagery row no longer lists `placehold.co`.)
1.4. `pnpm audit --prod` shows no `next` advisory. `picomatch` (S9):
     add a `pnpm.overrides` entry to the first patched version if one
     exists; otherwise leave it (dev tooling only) and note it in the
     PR.
1.5. `/validate`, then a manual pass over the homepage hero (three.js),
     menu images, the cms live preview and Draft Mode.

## Group 2 — Rust dependency advisories (S3, Decision 2)

2.1. `cd apps/api && cargo update`, then `cargo audit`.
2.2. For each advisory still listed, `cargo tree -i <crate>@<version>` to
     find the parent and bump it in `Cargo.toml` (expected: `aws-sdk-s3`
     / `aws-config` to a version on `hyper 1` + `rustls 0.23.46+`,
     `lettre` with `tokio1-rustls` on rustls 0.23, `reqwest`, `sqlx`).
     If `aws-sdk-s3` still pulls `rustls 0.21` through the legacy
     `hyper-rustls 0.24` connector, switch it to the
     `default-https-client` feature (aws-smithy-http-client, rustls
     0.23).
2.3. Whatever remains with no fix goes in `apps/api/audit.toml`:
     ```toml
     [advisories]
     ignore = [
       # rsa: Marvin attack needs a decryption oracle; we only sign VAPID
       # JWTs locally (web-push-native). No fixed release on our path.
       "RUSTSEC-2023-0071",
     ]
     ```
2.4. `cargo audit` exits 0; `cargo test` and `cargo clippy -- -D warnings`
     pass; `cargo sqlx prepare` if sqlx moved.

## Group 3 — Admin login hardening (S2, Decisions 3–4)

3.1. Migration `apps/api/migrations/20261118120000_login_attempts.up.sql`:
     ```sql
     -- Phase 33: per-email login lockout (Decision 3).
     CREATE TABLE login_failures (
         id BIGSERIAL PRIMARY KEY,
         email TEXT NOT NULL,
         created_at TIMESTAMPTZ NOT NULL DEFAULT now()
     );
     CREATE INDEX idx_login_failures_email_time ON login_failures (lower(email), created_at);
     ```
     `.down.sql` drops it.
3.2. `apps/api/src/rate_limit.rs`: `login_rate_limiter()` —
     `per_second(60)`, `burst_size(5)`, same `ForwardedForKeyExtractor`
     and cleanup task as `enquiry_rate_limiter`.
3.3. `apps/api/src/routes/admin/mod.rs`:
     `.route("/login", post(auth::login).route_layer(crate::rate_limit::login_rate_limiter()))`.
3.4. `apps/api/src/routes/admin/auth.rs` `login`:
     - before verifying: `SELECT count(*) FROM login_failures WHERE
       lower(email) = lower($1) AND created_at > now() - interval '15
       minutes'`; at 10 or more, still run the argon2 verify against the
       dummy hash (same timing) and return `InvalidCredentials`.
     - on failure: insert a row; `tracing::warn!(ip = %client_ip,
       "admin login failed")` (IP from `client_ip_key`, never the email
       or password).
     - on success: `DELETE FROM login_failures WHERE lower(email) =
       lower($1)`.
     - rows older than 1 day are deleted by the retention sweeper
       (Group 7).
3.5. `apps/web/app/api/admin/login/route.ts`: forward
     `X-Forwarded-For` like `app/api/contact-messages/route.ts` does, so
     the limiter keys on the visitor.
3.6. `Caddyfile`, `api.{$DOMAIN}` block:
     ```caddyfile
     @admin path /api/admin /api/admin/*
     respond @admin 404
     ```
3.7. Tests in `auth.rs`: the 11th attempt with the right password inside
     15 minutes still fails; a success clears the counter.

## Group 4 — Security headers (S4, S10, Decision 5)

4.1. `Caddyfile`: snippets, imported per site block.
     ```caddyfile
     (security-headers) {
         header {
             Strict-Transport-Security "max-age=31536000; includeSubDomains"
             X-Content-Type-Options "nosniff"
             Referrer-Policy "strict-origin-when-cross-origin"
             Permissions-Policy "camera=(), microphone=(), geolocation=()"
             -Server
         }
     }
     # CSP_HEADER is Content-Security-Policy-Report-Only on staging,
     # Content-Security-Policy on production ([var], default report-only).
     (csp-public) {
         header {$CSP_HEADER:Content-Security-Policy-Report-Only} "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob: https://api.{$DOMAIN}; font-src 'self'; connect-src 'self'; media-src 'self' https://api.{$DOMAIN}; worker-src 'self' blob:; frame-src 'none'; frame-ancestors 'none'; base-uri 'self'; form-action 'self'; object-src 'none'; upgrade-insecure-requests"
     }
     (csp-cms) {
         header {$CSP_HEADER:Content-Security-Policy-Report-Only} "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob: https://api.{$DOMAIN}; font-src 'self'; connect-src 'self'; worker-src 'self'; frame-src 'self'; frame-ancestors 'self'; base-uri 'self'; form-action 'self'; object-src 'none'"
     }
     ```
     `{$DOMAIN}` and `cms.{$DOMAIN}` import `security-headers` and their
     CSP; `api.{$DOMAIN}` imports `security-headers` plus
     `header Content-Security-Policy "default-src 'none'; frame-ancestors 'none'"`;
     `logs.{$DOMAIN}` imports `security-headers`. Dev (`localhost`)
     runs without Caddy and isn't affected.
     Why `'unsafe-inline'` for scripts: Next's App Router inlines its
     bootstrap/flight scripts; nonces need `headers()` in a proxy and
     make every page dynamic, losing the static About/Terms/Menu pages
     (Phase 18 memory budget). With no user-generated HTML rendered
     anywhere, the main CSP win here is `frame-ancestors`, `object-src`,
     `base-uri` and `form-action`.
     Also check `'wasm-unsafe-eval'` isn't needed by three.js (it
     isn't, as of r16x) and that GSAP needs nothing beyond
     `'unsafe-inline'` styles.
4.2. `docker-compose.prod.yml` caddy env: `CSP_HEADER: ${CSP_HEADER:-Content-Security-Policy-Report-Only}`.
     `.env.example`: document `CSP_HEADER` as `[var]`.
     `_deploy.yml`: render `CSP_HEADER` from the Environment variable.
4.3. `apps/api/src/routes/media.rs`: add
     `(header::X_CONTENT_TYPE_OPTIONS, "nosniff")` (also set at Caddy,
     but the API shouldn't depend on it).
4.4. On staging, browse every public page and the admin (including the
     device simulator and a push test) with DevTools open; zero CSP
     reports → set `CSP_HEADER=Content-Security-Policy` on staging and
     production.

## Group 5 — Order rate limit and admin CSRF (S5, S6, Decisions 6–7)

5.1. `apps/api/src/routes/mod.rs`:
     `.route("/orders", post(orders::create_order).route_layer(crate::rate_limit::enquiry_rate_limiter()))`.
5.2. `apps/web/app/api/orders/route.ts`: forward `X-Forwarded-For`,
     matching the other public proxies.
5.3. `apps/web/lib/admin-proxy.ts`:
     ```ts
     /** Rejects cross-site writes to the admin proxies (Phase 33 Decision 7). */
     export function rejectCrossSite(request: NextRequest): NextResponse | null {
       if (request.method === "GET" || request.method === "HEAD") return null;
       const origin = request.headers.get("origin");
       if (origin) {
         return origin === request.nextUrl.origin ||
           new URL(origin).host === request.headers.get("host")
           ? null
           : NextResponse.json({ error: "forbidden" }, { status: 403 });
       }
       return request.headers.get("sec-fetch-site") === "same-origin"
         ? null
         : NextResponse.json({ error: "forbidden" }, { status: 403 });
     }
     ```
     Call it at the top of `proxyAdminRequest`, and in every hand-written
     handler: `find apps/web/app/api/admin apps/web/app/api/preview -name route.ts`
     (`login`, `logout`, `media`, `orders/[id]/status`, `events/**`,
     `push/**`, `catering-enquiries/**`, `publish`, `drafts/**`,
     `categories/**`, `menu-items/**`, `site-content/draft`,
     `preview/enable`, `preview/disable`). Behind Caddy, `Host` is the
     public `cms.` host (Caddy passes it through).
5.4. `apps/api/src/lib.rs`: `.with_same_site(SameSite::Strict)`.

## Group 6 — Ops hardening (S7, S8, S11, Decisions 8–10)

6.1. `docker-compose.prod.yml` dozzle: pin `amir20/dozzle:<exact
     version>` (latest release at implementation time, looked up with
     `docker manifest inspect` or the GitHub releases page), add
     `DOZZLE_ENABLE_ACTIONS: "false"` and `DOZZLE_ENABLE_SHELL: "false"`.
6.2. `docs/deployment.md`: Basic Auth password for `logs.` is a 20+
     character random secret; how to rotate it.
6.3. `apps/api/src/lib.rs` `build_app`: after `migrate()`,
     ```rust
     tokio::spawn(
         session_store
             .clone()
             .continuously_delete_expired(tokio::time::Duration::from_secs(3600)),
     );
     ```
6.4. `apps/api/src/email.rs`: replace `to = email.to` in the debug and
     error lines with the delivery id (or nothing when untracked).
     `grep -rn 'to = ' apps/api/src` comes back empty.

## Group 7 — Retention sweeper (Decisions 14–15)

7.1. Migration `apps/api/migrations/20261118120100_privacy.up.sql`:
     ```sql
     -- Phase 33: anonymisation markers and the DSR log (Decisions 15-16).
     ALTER TABLE orders ADD COLUMN anonymised_at TIMESTAMPTZ;
     ALTER TABLE event_bookings ADD COLUMN anonymised_at TIMESTAMPTZ;
     ALTER TABLE catering_enquiries ADD COLUMN anonymised_at TIMESTAMPTZ;
     CREATE TABLE privacy_requests (
         id BIGSERIAL PRIMARY KEY,
         kind TEXT NOT NULL CHECK (kind IN ('export', 'anonymise')),
         email_sha256 TEXT NOT NULL,
         row_counts JSONB NOT NULL,
         admin_user_id BIGINT REFERENCES users(id) ON DELETE SET NULL,
         created_at TIMESTAMPTZ NOT NULL DEFAULT now()
     );
     ```
     `.down.sql` reverses it. Check each personal column's `NOT NULL`
     before choosing `''` vs `NULL` in 7.2 (e.g. `orders.customer_phone`
     is `NOT NULL` → `''`; `delivery_address`, `notes` → `NULL`).
7.2. `apps/api/src/retention.rs`:
     ```rust
     //! Phase 33 Decision 14: how long personal data is kept.
     pub const BOOKKEEPING: &str = "7 years";
     pub const SHORT: &str = "12 months";
     pub const EMAIL_LOG: &str = "90 days";
     pub const LOGIN_FAILURES: &str = "1 day";
     const SWEEP_EVERY: Duration = Duration::from_secs(24 * 3600);
     ```
     - `anonymise_orders(tx, filter)`, `anonymise_bookings(...)`,
       `anonymise_enquiries(...)` — one `UPDATE ... SET customer_name =
       'Anonymised', customer_email = 'anonymised+' || id || '@invalid',
       customer_phone = '', delivery_address = NULL, notes = NULL,
       public_token = gen_random_uuid(), anonymised_at = now() WHERE
       anonymised_at IS NULL AND <filter>` each. Enquiries also clear
       `message`; quotes keep amounts. Event booking guests have no
       personal columns beyond the booking (plate snapshots stay).
     - `sweep(pool)`: one transaction per table with the Decision 14
       filters (status lists from `order_status.rs`, `quote_status.rs`,
       `events.rs`), `DELETE` for `contact_messages` older than 12
       months, `email_deliveries` older than 90 days, `login_failures`
       older than 1 day. Returns counts; logged as one
       `tracing::info!(orders, bookings, enquiries, messages, emails,
       "retention sweep")`.
     - `spawn_retention_sweeper(pool)`, called from `main.rs` only when
       `RETENTION_SWEEP_ENABLED=true` (open risk: accountant sign-off).
     - Tests: the constants match the table; a sqlx test (`#[sqlx::test]`,
       as in existing route tests) inserts an 8-year-old completed order,
       a 2-month-old one and a 13-month-old contact message, sweeps, and
       checks only the right rows changed; a second sweep changes
       nothing.
7.3. `docker-compose.prod.yml` api env: `RETENTION_SWEEP_ENABLED:
     ${RETENTION_SWEEP_ENABLED:-false}`; `.env.example` and
     `_deploy.yml` as in 4.2.
7.4. Admin and public readers handle anonymised rows: the order/booking
     tracking page 404s on the old token (automatic, token changed); the
     admin orders/bookings lists show "Anonymised" as the name with no
     extra code; the bookings CSV includes them as "Anonymised".

## Group 8 — Admin Privacy tool (Decision 16)

8.1. `apps/api/src/routes/admin/privacy.rs`:
     - `GET /api/admin/privacy?email=` → `{ orders: [...], bookings:
       [...], enquiries: [...], contact_messages: [...], blocking:
       [...] }` (summary rows: id, kind, created_at, status, total; plus
       `blocking` = open orders not in a final status and bookings for
       events that haven't happened). Match with `lower(email) =
       lower($1)`.
     - `GET /api/admin/privacy/export?email=` → full rows as JSON,
       `Content-Disposition: attachment; filename="privacy-export.json"`.
       Records a `privacy_requests` row (`kind = 'export'`).
     - `POST /api/admin/privacy/anonymise` `{ email, confirm_email }` →
       422 unless they match (case-insensitive); 409 with the blocking
       list if any; otherwise runs the 7.2 anonymise functions filtered
       to that email plus `DELETE` of contact messages, in one
       transaction, records `kind = 'anonymise'`, returns counts.
     - `email_sha256` = lowercase hex SHA-256 of `lower(trim(email))`
       (`sha2` is already a dependency through argon2/web-push; add it to
       `Cargo.toml` directly if not).
     Register under the protected router in `routes/admin/mod.rs`.
8.2. Web proxies: `apps/web/app/api/admin/privacy/route.ts` (GET),
     `.../privacy/export/route.ts` (GET, passes the attachment headers
     through), `.../privacy/anonymise/route.ts` (POST, uses
     `rejectCrossSite`).
8.3. `apps/web/lib/admin-privacy-api.ts` (types, no `any`) and
     `apps/web/app/admin/(protected)/privacy/page.tsx` +
     `apps/web/components/admin-privacy-search.tsx`: search box, result
     tables linking to the existing detail pages, an Export button, and
     an Anonymise dialog that asks for the email again and shows the
     blocking rows on 409. Nav entry `{ label: "Privacy", href:
     "/privacy", enabled: true }` in `components/admin-nav.tsx`, before
     Settings.
8.4. Tests: anonymise refuses mismatched confirm and blocking rows,
     anonymises the right rows, writes exactly one `privacy_requests`
     row with the hash (not the email).

## Group 9 — Privacy policy page and form notices (Decisions 12–13, 17)

9.1. `apps/web/messages/{en,nl,fr}.json`: a `PrivacyPage` namespace with
     the sections listed in Decision 12 (title, intro, controller,
     dataWeCollect (orders, events, catering, contact, emails),
     whyAndBasis, recipients, retention (the Decision 14 table in plain
     words, plus "backups up to 30 days"), cookies (`NEXT_LOCALE`,
     cart, last order; no tracking), rights, complaints (Autoriteit
     Persoonsgegevens, autoriteitpersoonsgegevens.nl), automatedDecisions,
     changes, lastUpdated). `SiteFooter.privacy` label. A `FormPrivacy`
     namespace with one rich-text line taking a `<link>` tag. Claude
     writes all three languages (no approval gate).
9.2. `apps/web/app/[locale]/privacy/page.tsx`, modelled on
     `app/[locale]/terms/page.tsx` (same layout, `generateMetadata`,
     `setRequestLocale`). Controller block reads `BUSINESS_KVK` at
     render through the same source the invoice uses; KvK line hidden
     when unset.
9.3. `apps/web/lib/seo.ts` `PUBLIC_ROUTES`: add `"/privacy"` (sitemap and
     `hreflang` follow).
9.4. `apps/web/components/site-footer.tsx`: `/privacy` link after
     Terms.
9.5. `apps/web/components/form-privacy-note.tsx`: renders the
     `FormPrivacy` line with a `Link` to `/privacy`. Used under the
     submit button in `order-details-form.tsx`,
     `catering-enquiry-form.tsx`, `contact-form.tsx` and
     `event-booking-form.tsx`.
9.6. `pnpm check:messages` passes for all three locales.

## Group 10 — Docs and CI (Decisions 11, 18)

10.1. `docs/privacy.md`: record of processing (requirement.md Context
      table), processors and DPAs, retention table, the DSR how-to
      (admin Privacy page, answer within one month), and the breach
      checklist.
10.2. `docs/deployment.md` restore section: after a restore, run the
      sweeper once and re-apply anonymisations from `privacy_requests`
      (match by hashing each row's email).
10.3. `.github/workflows/ci.yml`: an `audit` job —
      `pnpm audit --prod --audit-level=high` in `apps/web`, and
      `rustsec/audit-check@v2` (pinned to a commit SHA) in `apps/api`.
10.4. `specs/tech-stack.md` and `specs/roadmap.md` were updated during
      planning (Security & Privacy section, Summary Table row, Phase 26
      note). Revise them if implementation changes a decision.
