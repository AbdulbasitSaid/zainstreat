# Phase 33 — GDPR Compliance & Security Hardening: Validation

## Pass/fail checklist

### Build, tests and audits

- [x] `cargo test` and `cargo clippy -- -D warnings` pass in `apps/api`,
      including the lockout, retention and privacy-tool tests.
- [x] `apps/api/.sqlx/` is regenerated and the Docker build passes with
      `SQLX_OFFLINE=true`.
- [x] Both new migrations apply on top of Phase 32 and their down
      migrations revert them.
- [x] `pnpm check:messages`, `pnpm lint` and `pnpm build` pass in
      `apps/web`, with no `any`.
- [x] `/validate` passes.
- [x] `cargo audit` exits 0 (ignores only in `audit.toml`, each with a
      reason).
- [x] `pnpm audit --prod --audit-level=high` exits 0.
- [ ] The CI `audit` job runs and passes on the PR.

### S1 — Next.js

- [x] `node -p "require('next/package.json').version"` in `apps/web`
      prints `16.4.6` (or later 16.x), and `package.json` pins it exactly.
- [x] `grep -n "dangerouslyAllowSVG\|placehold.co" apps/web/next.config.ts`
      is empty.
- [x] `curl -s -o /dev/null -w '%{http_code}' 'http://localhost:3000/_next/image?url=https%3A%2F%2Fplacehold.co%2F10.svg&w=64&q=75'`
      → `400`.
- [ ] Homepage hero, menu photos, cms live preview and Draft Mode still
      work.

### S2 — Admin login

- [x] Six quick wrong-password logins from one IP: the sixth returns
      `429`.
- [x] Ten wrong passwords for one email (spread across IPs or with the
      limiter relaxed in a test), then the right password, still returns
      `401 invalid_credentials`; after 15 minutes (or in the test, after
      ageing the rows) it succeeds.
- [x] Logs for a failed login show the IP and no email or password.
- [ ] On staging, `curl -s -o /dev/null -w '%{http_code}' -X POST https://api.<staging>/api/admin/login`
      → `404`; `https://api.<staging>/api/menu-items` still `200`.

### S4 — Headers

- [ ] `curl -sI https://<staging>/en` shows `Strict-Transport-Security`,
      `X-Content-Type-Options: nosniff`, `Referrer-Policy`,
      `Permissions-Policy`, a CSP header with `frame-ancestors 'none'`,
      and no `Server`.
- [ ] `curl -sI https://cms.<staging>/` shows `frame-ancestors 'self'`;
      the device simulator still frames pages.
- [ ] `curl -sI https://api.<staging>/api/media/<key>` shows
      `X-Content-Type-Options: nosniff` and `default-src 'none'`.
- [ ] A browse of every public page and the admin on staging shows zero
      CSP violation reports in the console before `CSP_HEADER` is
      switched to enforcing.

### S5/S6 — Orders and CSRF

- [x] Six quick `POST /api/orders` from one IP: the sixth returns `429`.
- [x] `curl -X POST -H 'Origin: https://evil.example' -H 'Cookie: <admin session>' http://cms.localhost:3000/api/admin/publish`
      → `403`. The same request from the admin UI works.
- [x] Every non-GET request to `/api/*` on the `cms.` host passes
      `rejectCrossSite`, called once in `proxy.ts` (as implemented).
- [ ] The admin session cookie stays `SameSite=Lax` (requirement.md
      Decision 7, changed during implementation); logging in, push
      notification taps, owner-email links and the live preview still work.

### S7/S8/S11 — Ops

- [x] `docker-compose.prod.yml` has no `:latest` image, and Dozzle has
      actions and shell disabled.
- [ ] An expired row in `tower_sessions.session` is gone within an hour
      of the API running.
- [x] `grep -rn 'to = ' apps/api/src` is empty.

### Retention

- [x] With `RETENTION_SWEEP_ENABLED=true`, a seeded 8-year-old completed
      order, 13-month-old contact message, 13-month-old unconverted
      enquiry and 100-day-old `email_deliveries` row are anonymised or
      deleted; a 2-month-old order is untouched; amounts and items on
      the anonymised order are unchanged.
- [x] The anonymised order's old tracking link returns not-found.
- [x] A second sweep changes nothing (`anonymised_at` set).
- [ ] With the flag unset, the sweeper doesn't start (log line absent).

### Admin Privacy tool

- [x] `cms.<host>/privacy` is in the nav; searching a test customer's
      email lists their orders, bookings, enquiries and messages.
- [x] Export downloads a JSON file with every row for that address and
      nothing for other addresses.
- [x] Anonymise refuses a mismatched confirmation email, refuses while
      an open order exists (shows it), and otherwise anonymises; a new
      search finds nothing for that email.
- [x] `privacy_requests` has one row per export/anonymise with a SHA-256
      hash, not the email.

### Privacy page and notices

- [x] `/en/privacy`, `/nl/privacy` and `/fr/privacy` render, are in
      `sitemap.xml` with `hreflang`, and are linked from the footer.
- [x] The page names Zain's Treat n More, contact@zainstreat.com and
      WhatsApp, shows the KvK line only when `BUSINESS_KVK` is set, and
      has no street address.
- [x] It covers data collected per form, purposes and lawful bases,
      processors, retention, cookies/storage, every data subject right,
      the Autoriteit Persoonsgegevens, no automated decisions, and a
      last-updated date.
- [ ] The order, catering, contact and event booking forms each show the
      privacy note with a working link, in all three locales, and have
      no consent checkbox.
- [ ] No non-essential cookie is set on a fresh visit (DevTools →
      Application → Cookies shows only `NEXT_LOCALE`).

### Docs

- [x] `docs/privacy.md` exists with the processing record, processors,
      retention, DSR how-to and breach checklist.
- [x] `docs/deployment.md` covers the `logs.` password, `CSP_HEADER`,
      `RETENTION_SWEEP_ENABLED` and the post-restore anonymisation step.
- [x] `specs/tech-stack.md` has the Security & Privacy section and
      Summary Table row.

## Definition of done

The critical Next.js RCE is closed by upgrading to 16.4.x and turning off
SVG optimisation, and `cargo audit` and `pnpm audit` are clean and
enforced in CI. Admin login is rate-limited with a per-email lockout and
unreachable on the public API host, every host sends HSTS, nosniff,
referrer, permissions and CSP headers, public orders are rate-limited,
and admin writes reject cross-site requests. On the GDPR side, the site
has a privacy policy in en/nl/fr that meets Art. 13, every form links to
it, personal data is kept no longer than the documented periods by a
nightly sweeper, and the owner can export or anonymise everything held
for an email address from the admin, with each request logged. Tests,
lint, build, `check:messages` and `/validate` pass.
