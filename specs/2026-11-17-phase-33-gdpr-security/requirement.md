# Phase 33 — GDPR Compliance & Security Hardening: Requirements

## Source

A new phase with no earlier roadmap entry. On 2026-11-17 the user asked:

> make website GDPR complience and also do a security audit.

The audit was done in the same session (read-only: code review, `pnpm
audit --prod`, `cargo audit`). Its findings are in Context below.

Answers to the planning questions (2026-11-17):

- **Next.js RCE fix:** ships inside this phase (Group 1, first to
  implement), not as a separate hotfix.
- **Retention:** the tax-minimum plan. Orders and other bookkeeping
  records are kept 7 years, then anonymised. Everything else is kept 12
  months. The email log is kept 90 days. A nightly sweeper enforces it.
- **Data subject requests:** an admin **Privacy** tool to look up, export
  and anonymise everything held for an email address.
- **Controller on the privacy page:** business name, KvK number,
  contact@zainstreat.com and WhatsApp. No street address.

## Context

### Why now

Phase 31 events are about to go to production, which means more personal
data (names, emails, phones, party details) from the public. The site
collects personal data on five forms but has no privacy policy, no
retention limits and no way to answer an access or erasure request. The
audit also found a **critical, remotely exploitable** Next.js
vulnerability in the deployed version.

### Personal data the site holds today

| Where | Personal data | Purpose / lawful basis |
| --- | --- | --- |
| `orders` (+ `order_items`) | name, email, phone, delivery address, notes | Contract (Art. 6(1)(b)); tax records (6(1)(c)) |
| `event_bookings` (+ guests) | name, email, phone, notes, party size | Contract / pre-contract steps |
| `catering_enquiries` (+ `catering_quotes`) | name, email, phone, event details, message | Pre-contract steps |
| `contact_messages` | name, email, phone, message | Legitimate interest (answering the sender) |
| `email_deliveries` | recipient email, subject kind | Legitimate interest (delivery troubleshooting) |
| `push_subscriptions` | admin device endpoint, user agent | Admin only, legitimate interest |
| `tower_sessions` | admin session id | Strictly necessary |
| Caddy / container logs | client IP, URL, user agent | Legitimate interest (security). Docker rotates at 3 × 10 MB |
| S3 backups | all of the above | Expire after 30 days (`docs/deployment.md`) |

Processors: AWS (Lightsail and S3 in `eu-central-1` Frankfurt; SES in
`eu-central-1`), Namecheap Private Email (inbound `contact@`), GitHub
(code and container images, no customer data). Admin web push goes
through the browser vendor's push service (Apple, Google or Mozilla) and
only carries the customer's name in the notification.

Cookies and storage: `NEXT_LOCALE` (language preference), the admin
session cookie (`cms.` only), the cart in `localStorage` and the last
order in `sessionStorage`. All are strictly necessary or functional, so
the Dutch Telecommunicatiewet (art. 11.7a) needs no consent banner. Phase
26's analytics will add the banner. Fonts are self-hosted through
`next/font`. The only third-party link is a Google Maps **link** (not an
embed) on a confirmed booking page.

### Security audit findings

Severity uses the usual critical / high / medium / low scale.

| # | Severity | Finding | Where |
| --- | --- | --- | --- |
| S1 | **Critical** | `next@16.3.8` is affected by GHSA advisories for unauthenticated RCE in the image optimizer with SVGs, RCE in `use cache` with typed arrays, Server Actions DoS and two `use cache` cache leaks. The app turns on `dangerouslyAllowSVG`, which is the exact trigger for the first one. Fixed in `>=16.4.5`. | `apps/web/package.json` (`"next": "^16"`, lockfile 16.3.8), `apps/web/next.config.ts` |
| S2 | High | Admin login has no rate limit or lockout, and the whole admin API is also reachable directly at `https://api.<domain>/api/admin/*`, not only through the `cms.` proxy. Passwords can be brute-forced online. | `apps/api/src/routes/admin/mod.rs`, `Caddyfile` |
| S3 | High | Rust advisories: `rustls 0.21/0.23.45` (handshake downgrade, 1 GiB decompression before auth), `rustls-webpki` (accepts distrusted intermediates, name-constraint bypasses), `webpki-roots` (distrusted emSign roots), `h2 0.3` (empty DATA frame flood), `rsa 0.9` (Marvin timing attack), `crypto-bigint 0.5` (timing). Mostly through the old `aws-sdk-s3` / `lettre` / `web-push-native` TLS stacks. 16 in total. | `apps/api/Cargo.lock` |
| S4 | Medium | No security headers on any host: no `Strict-Transport-Security`, `Content-Security-Policy`, `X-Frame-Options`/`frame-ancestors`, `X-Content-Type-Options`, `Referrer-Policy` or `Permissions-Policy`. The admin can be framed (clickjacking). | `Caddyfile`, `apps/web/next.config.ts` |
| S5 | Medium | `POST /api/orders` has no rate limit, and its web proxy doesn't forward `X-Forwarded-For`. Every order sends an invoice email to whatever address was typed, so the site can be used to spam third parties. | `apps/api/src/routes/mod.rs`, `apps/web/app/api/orders/route.ts` |
| S6 | Medium | Admin proxy route handlers don't check `Origin`. The session cookie is `SameSite=Lax`, and `cms.<domain>` and `<domain>` are the **same site**, so any script on the public host (or a future XSS) could make admin `POST`s. | `apps/web/lib/admin-proxy.ts` and the hand-written `app/api/admin/**` routes |
| S7 | Medium | Dozzle uses `amir20/dozzle:latest` (unpinned) with the Docker socket mounted, behind Basic Auth only. A compromised Dozzle is a root shell on the host. | `docker-compose.prod.yml` |
| S8 | Low | Expired admin sessions are never deleted from Postgres (`PostgresStore::continuously_delete_expired` isn't started). | `apps/api/src/lib.rs` |
| S9 | Low | `picomatch <=4.0.7` ReDoS (high in `pnpm audit`, but dev and build tooling only, so low here). | `apps/web/pnpm-lock.yaml` |
| S10 | Low | Media is served with the stored `Content-Type` and no `X-Content-Type-Options: nosniff`. Uploads are admin-only and limited to JPEG, PNG and WebP, so this is defence in depth. | `apps/api/src/routes/media.rs` |
| S11 | Low | Logs write customer email addresses (`email.rs`: `to = email.to` on send failures and the disabled-send debug line). | `apps/api/src/email.rs` |

What the audit checked and found fine: argon2id with a dummy hash on
unknown emails (no user enumeration by timing), session id rotated on
login, `Secure` cookies on servers, every admin route behind
`require_admin`, `sqlx` compile-time-checked queries (no SQL injection
found), public tokens are `gen_random_uuid()` (122 bits), CSV export
defuses formula injection, emails are plain text, uploads are typed and
size-capped, the rate-limit key takes the last `X-Forwarded-For` hop
(not spoofable), honeypots on the public forms, Postgres and MinIO
aren't published on servers, and Caddy only gets the env vars it needs.

## Decisions

### Security

1. **Upgrade Next.js to the latest 16.x (`>=16.4.5`) and turn off
   `dangerouslyAllowSVG`.** Pin `next` and `eslint-config-next` to an
   exact version. Remove `placehold.co` from `images.remotePatterns`: no
   code uses it any more, and the `image_url` fallback is gone. This
   fixes S1 even if a future advisory hits the optimizer again. Done
   first.
2. **Run `cargo update` and bump the crates that pull the old TLS
   stacks** until `cargo audit` shows no vulnerabilities. Any left
   without a fix (e.g. `rsa`, which only signs VAPID JWTs locally and
   isn't reachable as an oracle) go in `apps/api/audit.toml` with a
   one-line reason. Unmaintained-crate warnings are informational. (S3)
3. **Rate-limit admin login: 5 attempts per IP, then one every 60 s**,
   using the existing `tower_governor` setup. Add a **per-email lockout**
   in Postgres: 10 failures in 15 minutes blocks that email for 15
   minutes. The response stays the same `invalid_credentials`, so it
   doesn't reveal the lockout. Login failures are logged with the IP, not
   the password. (S2)
4. **Block `/api/admin/*` on the public `api.<domain>` host in Caddy.**
   The admin only ever talks to the API through the `cms.` proxy over the
   Docker network, so nothing legitimate uses it. (S2)
5. **Security headers in Caddy**, so they cover every host and every
   response, including Next's static files:
   - all hosts: `Strict-Transport-Security: max-age=31536000;
     includeSubDomains`, `X-Content-Type-Options: nosniff`,
     `Referrer-Policy: strict-origin-when-cross-origin`,
     `Permissions-Policy: camera=(), microphone=(), geolocation=()`, and
     `-Server` removed.
   - public and `cms.`: a `Content-Security-Policy` (Group 4 has the
     exact value). It starts in **`Content-Security-Policy-Report-Only`
     on staging**, then is enforced once staging shows no violations.
     `frame-ancestors 'none'` on the public host and `'self'` on `cms.`,
     because the device simulator frames `cms.` itself.
   - `api.`: `Content-Security-Policy: default-src 'none';
     frame-ancestors 'none'`.
   No `preload` on HSTS yet, because preload is hard to undo. (S4, S10)
6. **Rate-limit `POST /api/orders`** with the same limiter as the other
   public forms, and forward `X-Forwarded-For` from its web proxy. (S5)
7. **Same-origin check on every admin write.** A shared
   `assertSameOrigin(request)` in `apps/web/lib/admin-proxy.ts` rejects a
   non-GET admin route handler request with 403 unless `Origin` (or
   `Sec-Fetch-Site: same-origin` when `Origin` is missing) matches the
   `cms.` host. It's applied to `login`, `logout`, `media` and every
   `app/api/admin/**` route, plus `/api/preview/enable` and
   `/api/preview/disable`. The session cookie becomes
   **`SameSite=Strict`**: the admin is opened by typing the URL or from
   a push notification, which are both top-level navigations without a
   cross-site referrer. Push opens `cms.` from the service worker, which
   counts as same-site. (S6)
8. **Pin Dozzle to an exact version and make it read-only**:
   `DOZZLE_ENABLE_ACTIONS=false`, `DOZZLE_ENABLE_SHELL=false` and the
   socket mounted `:ro` (as now). Rate-limit `logs.` Basic Auth through
   Caddy isn't available out of the box, so the Basic Auth password must
   be a 20+ character random secret (documented in
   `docs/deployment.md`). (S7)
9. **Delete expired sessions hourly** with
   `PostgresStore::continuously_delete_expired`. (S8)
10. **No customer emails in logs.** Log the `email_deliveries` id or the
    order/booking id instead of `to`. (S11)
11. **CI audit gate:** `pnpm audit --prod --audit-level=high` and
    `cargo audit` (via `rustsec/audit-check`) run in the CI workflow and
    fail it. Dependabot (or Renovate) is **not** added: the user reviews
    upgrades by hand. (S1, S3, S9)

### GDPR

12. **A privacy policy page at `/{locale}/privacy`**, in en, nl and fr,
    linked from the footer next to Terms and from every form. Written by
    Claude (no copy approval step). It covers what Art. 13 needs:
    - controller: Zain's Treat n More, KvK number (`BUSINESS_KVK`),
      contact@zainstreat.com and WhatsApp. **No street address.**
    - what is collected on each form and why, with the lawful basis from
      the table in Context;
    - recipients and processors (AWS in Frankfurt, Namecheap mail, the
      push service for the owner's phone); no transfers outside the EEA
      that need safeguards beyond the processors' standard contractual
      clauses;
    - retention periods (Decision 14);
    - cookies and local storage, and that there's no tracking;
    - the rights to access, rectification, erasure, restriction,
      objection and portability, how to use them (email contact@), and
      the right to complain to the **Autoriteit Persoonsgegevens**;
    - that no automated decision-making or profiling happens;
    - a "last updated" date.
    The page has the same `metadata`, sitemap entry and `hreflang` as the
    Terms page. It is static copy in `messages/*.json`, not
    admin-editable (Phase 25 site content stays as it is).
13. **A short privacy notice under every form's submit button**: order,
    catering enquiry, contact, event booking. "We use your details only
    to handle your order/enquiry. See our privacy policy." with a link.
    **No consent checkbox**, because the lawful basis is contract or
    legitimate interest, not consent, and a pre-ticked or forced box
    would be wrong under GDPR.
14. **Retention, enforced by a nightly sweeper** in the API
    (`apps/api/src/retention.rs`, same `tokio::spawn` + interval shape
    as `events::spawn_hold_sweeper`):
    | Data | Kept | Then |
    | --- | --- | --- |
    | Orders (any status that reached an invoice), paid or confirmed event bookings, accepted quotes and their enquiry | 7 years from creation | **Anonymised** (Decision 15) |
    | Orders cancelled before an invoice, cancelled or released bookings, waitlist entries, free RSVPs | 12 months after the event or order date | Anonymised |
    | Catering enquiries without an accepted quote, declined or expired quotes | 12 months from the last update | Anonymised |
    | Contact messages | 12 months | **Deleted** |
    | `email_deliveries` | 90 days | Deleted |
    | Expired admin sessions | — | Deleted hourly (Decision 9) |
    | Push subscriptions | Until removed or the push service reports them gone (410), as now | — |
    The sweeper logs counts only. The periods are constants in
    `retention.rs`, with a test that pins them to this table.
15. **Anonymise, don't delete, where amounts must be kept.** Personal
    columns are overwritten in place: name becomes `"Anonymised"`, email
    becomes `anonymised+<id>@invalid`, phone and delivery address become
    empty or `NULL` where the column allows it, and free-text notes and
    messages are cleared. Amounts, items, dates, status and the event
    link stay, so totals and cook counts still add up. A new nullable
    `anonymised_at TIMESTAMPTZ` column on each table records when it
    happened and keeps the sweeper idempotent. Public tracking tokens are
    regenerated so an old link stops showing anything.
16. **Admin Privacy tool** at `cms.<domain>/privacy`: one search box for
    an email address (exact, case-insensitive). It lists every order,
    booking, enquiry, quote and contact message for that address, with
    links to the existing detail pages. Two actions:
    - **Export JSON**: a download of every row for that address, for an
      access or portability request (Art. 15/20).
    - **Anonymise**: for an erasure request (Art. 17). It asks the owner
      to type the email again to confirm. Rows inside the 7-year tax
      period are anonymised under Decision 15. Contact messages are
      deleted. Open orders or upcoming bookings block it with a message
      ("finish or cancel these first").
    Every export and anonymise is recorded in a new `privacy_requests`
    table (`kind`, a SHA-256 hash of the email (not the email itself),
    `row_counts`, `admin_user_id`, `created_at`), so the owner can show
    the request was handled within one month. Admin UI stays English,
    as for the rest of the admin.
17. **No cookie banner in this phase.** Nothing non-essential is set.
    The privacy policy's cookie section lists what is used. Phase 26's
    roadmap entry already covers the consent banner for GA4, and it must
    link to this privacy page.
18. **Processing records and breach steps live in the repo:**
    `docs/privacy.md` holds the Art. 30 record of processing (the
    Context table), the processor list with where each DPA is accepted
    (AWS GDPR DPA is part of the AWS Service Terms; Namecheap's DPA), and
    a short breach checklist (72-hour notice to the Autoriteit
    Persoonsgegevens through its online form, what to collect, who
    decides). No DPIA: the processing is small-scale and not high-risk.

## Out of scope

- **Cookie consent banner and analytics**: Phase 26.
- **Production rollout**: staging is the target (memory: production
  deferred). Enabling HSTS on production is part of the normal promote.
- **Customer accounts and self-service data download**: Beyond MVP.
- **2FA for the admin**: one admin account, now protected by rate limit
  and lockout. Worth revisiting if more staff get accounts (Beyond MVP:
  multiple administrators).
- **A WAF, fail2ban or Cloudflare in front of Caddy.**
- **Penetration test by a third party.**
- **Dependabot/Renovate** (Decision 11).
- **Encrypting columns at rest** beyond Lightsail's disk encryption.

## Open risks flagged during planning

- **The Next.js upgrade crosses minor versions** (16.3 → 16.4+). Phase
  25 relies on Draft Mode and `proxy.ts`, and Phase 30 on GSAP in client
  components. The `/validate` sweep and a manual look at preview and
  the homepage hero are needed before staging.
- **The CSP can break things silently:** GSAP, three.js shaders,
  `next/image`, inline styles from next-intl and Next's inline bootstrap
  scripts. That's why it starts Report-Only on staging. Next's inline
  scripts need either `'unsafe-inline'` for `script-src` or nonces. This
  phase picks `'unsafe-inline'` for scripts to keep pages statically
  rendered; nonces would force every page dynamic (Group 4 notes the
  trade-off).
- **`SameSite=Strict`** means a link to the admin from an email client
  opens logged-out on the first click in some browsers. Owner emails
  link to `cms.`, so the owner may need to log in once from those links.
  If that's annoying, fall back to `Lax`; the Origin check (Decision 7)
  already blocks the CSRF case.
- **Retention defaults are a legal judgment.** 7 years matches the
  Dutch fiscal record-keeping duty (AWR art. 52). The client's
  accountant should confirm before the sweeper runs on production. The
  sweeper ships with a `RETENTION_SWEEP_ENABLED` env flag, off by
  default, so it only runs where it's been turned on (staging first).
- **The KvK number isn't set yet.** `BUSINESS_KVK` is optional. The
  privacy page must say the controller's name and contact details
  either way, and shows the KvK line only when it's set.
- **Backups keep erased data for up to 30 days.** That's allowed if the
  privacy policy says so and erased data is re-applied after a restore.
  The restore runbook in `docs/deployment.md` gets a step to re-run the
  sweeper and replay `privacy_requests` anonymisations after a restore.
