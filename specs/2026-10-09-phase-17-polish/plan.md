# Phase 17 — Polish & Non-Functional Requirements: Implementation Plan

Six mostly-independent groups. Groups 1–3 and 6 touch only `apps/web`;
Groups 4–5 touch only `apps/api`; Group 6 (email) and its env-var plumbing
touch both `apps/api` and the compose files. None of these groups depend
on each other — implement in any order, but validate each with `/validate`
as you go rather than saving it all for one giant sweep at the end.

## Group 0 — Branch

Branched `2026-10-09-phase-17-polish` off `master` (Phase 16 already
merged).

## Group 1 — SEO

Depends on: nothing.

**1a. `apps/web/app/[locale]/layout.tsx`** — replace the static `metadata`
export:

```tsx
export const metadata: Metadata = {
  metadataBase: new URL(process.env.DOMAIN ? `https://${process.env.DOMAIN}` : "http://localhost:3000"),
  title: "Zain's Treat n More",
  description:
    "Halal meals, snacks, catering, and event rentals from Zain's Treat n More — browse the menu and order online.",
  openGraph: {
    siteName: "Zain's Treat n More",
    images: ["/logo.png"],
  },
};
```

(`process.env.DOMAIN` is already passed into the `web` service's
environment in `docker-compose.prod.yml:41` — Phase 14's wiring, reused
here with no new env var. `/logo.png` resolves against `metadataBase`
automatically; it's the same asset `components/logo.tsx` already uses.)

**1b. New `metaTitle`/`metaDescription` keys** — add to the six
namespaces below, in both `apps/web/messages/en.json` and
`apps/web/messages/nl.json` (alongside each namespace's existing keys,
same `HomePage`/`AboutPage`/etc. objects — don't create new namespaces):

| Namespace | `metaTitle` (en) | `metaDescription` (en) |
|---|---|---|
| `HomePage` | `Zain's Treat n More \| Halal Meals, Snacks & Catering` | `Discover halal meals, snacks, and catering from Zain's Treat n More. Browse the menu and order online in minutes.` |
| `AboutPage` | `About Us \| Zain's Treat n More` | `Learn about Zain's Treat n More — our story, values, and commitment to halal, hygienic, quality food.` |
| `ServicesPage` | `Our Services \| Zain's Treat n More` | `Meals, snacks, catering, and event rental services from Zain's Treat n More — request a quote for your next event.` |
| `ContactPage` | `Contact Us \| Zain's Treat n More` | `Get in touch with Zain's Treat n More for orders, catering enquiries, or general questions.` |
| `TermsPage` | `Terms & Conditions \| Zain's Treat n More` | `Read the terms and conditions for ordering from and using the Zain's Treat n More website.` |
| `FoodRegulationsPage` | `Food Regulations \| Zain's Treat n More` | `Halal certification, hygiene standards, ingredient sourcing, and allergen information for Zain's Treat n More.` |

Dutch (`nl.json`), same six namespaces:

| Namespace | `metaTitle` (nl) | `metaDescription` (nl) |
|---|---|---|
| `HomePage` | `Zain's Treat n More \| Halal Maaltijden, Snacks & Catering` | `Ontdek halal maaltijden, snacks en catering van Zain's Treat n More. Bekijk het menu en bestel binnen enkele minuten online.` |
| `AboutPage` | `Over Ons \| Zain's Treat n More` | `Maak kennis met Zain's Treat n More — ons verhaal, onze waarden en onze toewijding aan halal, hygiënisch en kwalitatief voedsel.` |
| `ServicesPage` | `Onze Diensten \| Zain's Treat n More` | `Maaltijden, snacks, catering en evenementenverhuur van Zain's Treat n More — vraag een offerte aan voor uw volgende evenement.` |
| `ContactPage` | `Neem Contact Op \| Zain's Treat n More` | `Neem contact op met Zain's Treat n More voor bestellingen, cateringaanvragen of algemene vragen.` |
| `TermsPage` | `Algemene Voorwaarden \| Zain's Treat n More` | `Lees de algemene voorwaarden voor het bestellen bij en gebruiken van de website van Zain's Treat n More.` |
| `FoodRegulationsPage` | `Voedselregels \| Zain's Treat n More` | `Halal-certificering, hygiënenormen, ingrediëntenherkomst en allergeneninformatie voor Zain's Treat n More.` |

**1c. `generateMetadata` added to each of the six pages**, matching
`menu/page.tsx:11-23`'s exact shape (these six are currently plain
synchronous components with no metadata export at all — add the function,
don't touch the component body):

```tsx
import type { Metadata } from "next";
import { getTranslations } from "next-intl/server";

export async function generateMetadata({
  params,
}: {
  params: Promise<{ locale: string }>;
}): Promise<Metadata> {
  const { locale } = await params;
  const t = await getTranslations({ locale, namespace: "HomePage" }); // swap namespace per page

  return {
    title: t("metaTitle"),
    description: t("metaDescription"),
  };
}
```

Apply to: `apps/web/app/[locale]/page.tsx` (namespace `HomePage`),
`about/page.tsx` (`AboutPage`), `services/page.tsx` (`ServicesPage`),
`contact/page.tsx` (`ContactPage`), `terms/page.tsx` (`TermsPage`),
`food-regulations/page.tsx` (`FoodRegulationsPage`). `contact/page.tsx`
and `services/page.tsx` are already async server components calling
`getTranslations`/`useTranslations` for their own body content — add
`generateMetadata` as a second, separate export, don't merge it into the
existing component function.

**1d. New `apps/web/app/sitemap.ts`** (outside `[locale]`, Next's
file-convention route — generates `/sitemap.xml`):

```ts
import type { MetadataRoute } from "next";
import { routing } from "@/i18n/routing";

const ROUTES = ["", "/about", "/services", "/menu", "/contact", "/terms", "/food-regulations"];

export default function sitemap(): MetadataRoute.Sitemap {
  const base = process.env.DOMAIN ? `https://${process.env.DOMAIN}` : "http://localhost:3000";

  return routing.locales.flatMap((locale) =>
    ROUTES.map((route) => ({
      url: `${base}/${locale}${route}`,
      lastModified: new Date(),
    })),
  );
}
```

(`/cart` and `/order` deliberately excluded — requirement.md Decision 8.)

**1e. New `apps/web/app/robots.ts`**:

```ts
import type { MetadataRoute } from "next";

export default function robots(): MetadataRoute.Robots {
  const base = process.env.DOMAIN ? `https://${process.env.DOMAIN}` : "http://localhost:3000";

  return {
    rules: { userAgent: "*", allow: "/", disallow: ["/admin", "/api"] },
    sitemap: `${base}/sitemap.xml`,
  };
}
```

## Group 2 — Accessibility

Depends on: nothing.

**2a. `apps/web/components/contact-form.tsx` and
`apps/web/components/order-details-form.tsx`** — every field that renders
a conditional error `<p>` needs `aria-invalid` and `aria-describedby` on
its `<input>`/`<select>`/`<textarea>`, and a matching `id` on the error
`<p>`. Pattern (shown for `contact-form.tsx`'s name field,
lines 181-188 — repeat for every other field with an error in both
files):

```tsx
<input
  id="contact-name"
  className="field"
  value={values.name}
  onChange={(e) => update("name", e.target.value)}
  aria-invalid={!!errors.name}
  aria-describedby={errors.name ? "contact-name-error" : undefined}
/>
{errors.name && (
  <p id="contact-name-error" className={FIELD_ERROR_CLASS}>
    {t(`errors.${errors.name}`, { field: labelFor("name") })}
  </p>
)}
```

Apply the same `{field-id}-error` convention to every field in both
forms: `contact-form.tsx`'s name/email/phone/subject/message, and
`order-details-form.tsx`'s name/email/phone/deliveryAddress (conditional)/
notes (if it has validation — check; if not, skip it). The radio
`<fieldset>` (order-details-form.tsx:150-170) has no per-input error
today, so leave it as-is — its `<legend>` already serves as the
accessible group label.

**2b. `apps/web/components/site-header.tsx`** — add explicit
`focus-visible` styling matching `button.tsx:11-12`'s utility classes:

- Hamburger button (line 99-109): append
  `focus-visible:outline focus-visible:outline-[3px] focus-visible:outline-accent focus-visible:outline-offset-2`
  to its `className`.
- Nav `<Link>` (line 136-144): append the same four utility classes to
  its `className` template string.

## Group 3 — Image optimization

Depends on: nothing.

**3a. `apps/web/next.config.ts`** — add `formats` inside the existing
`images` object (after `remotePatterns`, before `dangerouslyAllowSVG`):

```ts
images: {
  remotePatterns: [ /* unchanged */ ],
  formats: ["image/avif", "image/webp"],
  dangerouslyAllowSVG: true,
  // ...unchanged
},
```

**3b. Narrow the `unoptimized` scheme check** — `apps/api`'s
`PUBLIC_API_URL` makes `image_url` always `http://localhost:8080/...` in
dev and always `https://api.$DOMAIN/...` in prod (confirmed via
`apps/api/src/routes/admin/media.rs:74-76` + both compose files'
`PUBLIC_API_URL` values), so switching the check from
`.startsWith("http")` to `.startsWith("http://")` keeps the dev bypass
and lifts it in production:

- `apps/web/components/menu-item-card.tsx:33` —
  `unoptimized={item.image_url.startsWith("http")}` →
  `unoptimized={item.image_url.startsWith("http://")}`.
- `apps/web/components/admin-menu-list.tsx:115` — same change.
- `apps/web/components/cart-line-item.tsx:19-25` — this file currently
  sets **no** `unoptimized` prop at all (a pre-existing gap, not
  something Phase 14 touched) — add
  `unoptimized={line.imageUrl.startsWith("http://")}` to its `<Image>`
  (guard `line.imageUrl` is non-null first, matching the existing
  conditional at line 18).

**3c. `sizes` props** — add to the same three `<Image>` usages, since
enabling the optimizer in prod (3b) without `sizes` makes Next guess a
default that may over- or under-serve. Use the rendered box size as a
starting point and adjust against the actual grid breakpoints when
implementing (check `menu-item-card.tsx`'s grid container in
`app/[locale]/menu/page.tsx` for the real column-count breakpoints rather
than guessing blind):

- `menu-item-card.tsx` (rendered at `h-[180px] w-full` inside a grid
  cell): `sizes="(max-width: 640px) 100vw, (max-width: 1024px) 50vw, 400px"`
  — tune the vw fractions to the menu grid's actual `grid-cols-*`
  breakpoints.
- `cart-line-item.tsx` (fixed `64×64`): `sizes="64px"`.
- `admin-menu-list.tsx` (fixed small thumbnail, admin-only — lower
  priority, but add `sizes="48px"` for consistency since it's a one-line
  change).

**3d. `priority`/lazy-loading audit** — `Logo` (`logo.tsx:9`) already sets
`priority` correctly (above-the-fold, every page). Check the homepage
hero's image usage (`SiteImage` in `app/[locale]/page.tsx`'s hero
section) — if it's the largest above-the-fold image and doesn't have
`priority` set, add it (improves LCP); every other `SiteImage`/`Image`
usage below the fold should rely on the default lazy behavior (no
`priority` prop) — don't add `priority` anywhere else.

## Group 4 — Input validation hardening

Depends on: nothing. `apps/api/src/validation.rs` only.

Replace `is_valid_email` with real structural checks, same plain
character-matching style as the existing `is_valid_phone` below it (no
new crate):

```rust
pub fn is_valid_email(value: &str) -> bool {
    if value.is_empty() || value.len() > 254 {
        return false;
    }

    let Some((local, domain)) = value.split_once('@') else { return false };
    if local.is_empty() || local.len() > 64 || domain.contains('@') {
        return false;
    }
    if !is_valid_email_local_part(local) || !is_valid_email_domain(domain) {
        return false;
    }

    true
}

fn is_valid_email_local_part(local: &str) -> bool {
    if local.starts_with('.') || local.ends_with('.') || local.contains("..") {
        return false;
    }
    local
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '%' | '+' | '-'))
}

fn is_valid_email_domain(domain: &str) -> bool {
    if domain.starts_with('.') || domain.ends_with('.') || domain.contains("..") {
        return false;
    }
    let labels: Vec<&str> = domain.split('.').collect();
    if labels.len() < 2 {
        return false;
    }
    let Some(tld) = labels.last() else { return false };
    if tld.len() < 2 || !tld.chars().all(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    labels.iter().all(|label| {
        !label.is_empty()
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    })
}
```

Remove the old comment ("Deliberately loose... Decision 10 scopes real
validation hardening to roadmap.md Phase 17") since this phase is that
hardening. Optionally add a `#[cfg(test)] mod tests` block covering a few
known-good (`name@example.com`, `a.b+c@sub.example.co.uk`) and known-bad
(`""`, `"no-at-sign"`, `"a@b"`, `"a@.com"`, `"a@b..com"`,
`"a@b-.com"`) cases — recommended, not blocking.

No other validation/sanitization code changes this phase (requirement.md
Decision 10) — `is_valid_phone` stays as-is, no client-side schema
library is added, React's default escaping is confirmed sufficient as-is.

## Group 5 — Email (Amazon SES via `lettre`)

Depends on: nothing structurally, but touches `Cargo.toml`, `lib.rs`,
`main.rs`, and all three POST route handlers.

**5a. `apps/api/Cargo.toml`** — add to `[dependencies]`:

```toml
lettre = { version = "0.11", default-features = false, features = ["tokio1-rustls-tls", "smtp-transport", "builder"] }
```

(Confirm the exact current `0.11.x` patch via `cargo add lettre` at
implementation time — network access to crates.io wasn't available
during planning to pin it precisely.)

**5b. New `apps/api/src/email.rs`**:

```rust
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use std::sync::Arc;

#[derive(Clone)]
pub struct EmailConfig {
    settings: Option<Arc<SesSettings>>,
}

struct SesSettings {
    smtp_host: String,
    smtp_username: String,
    smtp_password: String,
    from_address: String,
    business_notify_address: String,
}

impl EmailConfig {
    pub fn from_env() -> Self {
        let vars = [
            std::env::var("SES_SMTP_HOST"),
            std::env::var("SES_SMTP_USERNAME"),
            std::env::var("SES_SMTP_PASSWORD"),
            std::env::var("EMAIL_FROM_ADDRESS"),
            std::env::var("EMAIL_BUSINESS_NOTIFY_ADDRESS"),
        ];

        let Ok([smtp_host, smtp_username, smtp_password, from_address, business_notify_address]) =
            vars.into_iter().collect::<Result<Vec<_>, _>>().map(|v| v.try_into().unwrap())
        else {
            tracing::warn!("email sending disabled: SES env vars not fully set");
            return Self { settings: None };
        };

        Self {
            settings: Some(Arc::new(SesSettings {
                smtp_host,
                smtp_username,
                smtp_password,
                from_address,
                business_notify_address,
            })),
        }
    }

    pub fn business_notify_address(&self) -> Option<String> {
        self.settings.as_ref().map(|s| s.business_notify_address.clone())
    }

    /// Fire-and-forget: spawns the send, logs failure, never returns an
    /// error to the caller (requirement.md Decision 3).
    pub fn spawn_send(&self, to: String, subject: String, body: String) {
        let Some(settings) = self.settings.clone() else {
            tracing::debug!(to, "email disabled, skipping send");
            return;
        };

        tokio::spawn(async move {
            if let Err(error) = send(&settings, &to, &subject, body).await {
                tracing::error!(%error, to, subject, "failed to send email");
            }
        });
    }
}

async fn send(settings: &SesSettings, to: &str, subject: &str, body: String) -> Result<(), String> {
    let email = Message::builder()
        .from(settings.from_address.parse::<Mailbox>().map_err(|e| e.to_string())?)
        .to(to.parse::<Mailbox>().map_err(|e| e.to_string())?)
        .subject(subject)
        .body(body)
        .map_err(|e| e.to_string())?;

    let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&settings.smtp_host)
        .map_err(|e| e.to_string())?
        .credentials(Credentials::new(settings.smtp_username.clone(), settings.smtp_password.clone()))
        .build();

    mailer.send(email).await.map_err(|e| e.to_string())?;
    Ok(())
}
```

(The `vars.into_iter().collect::<Result<Vec<_>,_>>()` array-destructure
is one way to require all five vars together — simplify however reads
cleaner in practice; the contract that matters is: any missing var ⇒
`settings: None` ⇒ every `spawn_send` becomes a no-op logged at `debug`.)

**5c. `apps/api/src/lib.rs`** — add `pub mod email;` at the top, add
`email: email::EmailConfig` to `AppState`, and its `FromRef` impl,
mirroring the existing `MediaConfig` pattern exactly:

```rust
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub media: MediaConfig,
    pub email: email::EmailConfig,
}

impl FromRef<AppState> for email::EmailConfig {
    fn from_ref(state: &AppState) -> email::EmailConfig {
        state.email.clone()
    }
}
```

Update `build_app`'s signature to accept `email: email::EmailConfig` and
pass it into `AppState { pool, media, email }`.

**5d. `apps/api/src/main.rs`** — before `build_app(...)`:

```rust
let email = api::email::EmailConfig::from_env();
```

and pass it: `api::build_app(pool, cookie_secure, media, email).await`.

**5e. `apps/api/src/routes/orders.rs`** — add
`State(email): State<email::EmailConfig>` to `create_order`'s extractors
(alongside the existing `State(pool)`), and after `tx.commit().await?`
(line 301), before building `OrderResponse`:

```rust
email.spawn_send(
    payload.customer_email.clone(),
    "Your order confirmation — Zain's Treat n More".to_string(),
    format!(
        "Hi {},\n\nThanks for your order! We've received order #{} and will be in touch shortly.\n\nTotal: €{}\n\n— Zain's Treat n More",
        payload.customer_name, order.id, total,
    ),
);
```

**5f. `apps/api/src/routes/contact_messages.rs`** — add
`State(email): State<email::EmailConfig>` to `create_contact_message`'s
extractors, and after the successful insert (line 94), before the
`Ok((...))` return:

```rust
if let Some(to) = email.business_notify_address() {
    email.spawn_send(
        to,
        format!("New contact message from {}", payload.name.trim()),
        format!(
            "Name: {}\nEmail: {}\nPhone: {}\nSubject: {}\n\n{}",
            payload.name.trim(), payload.email.trim(), payload.phone.trim(), payload.subject, payload.message.trim(),
        ),
    );
}
```

**5g. `apps/api/src/routes/catering_enquiries.rs`** — same pattern as
5f, after the insert (line 133), using the catering fields
(name/phone/email/event_type/event_date/guest_count/location/
services_required/message).

**5h. Env vars** — `.env.example` (new section after the Dozzle block):

```
# Email (Phase 17) — Amazon SES SMTP credentials. Leave all five unset in
# local dev to run with email sending disabled (every send becomes a
# logged no-op). Generate SMTP credentials in the SES console (distinct
# from IAM access keys) after verifying a sending domain; see
# docs/deployment.md.
SES_SMTP_HOST=email-smtp.eu-central-1.amazonaws.com
SES_SMTP_USERNAME=<SES_SMTP_USERNAME>
SES_SMTP_PASSWORD=<SES_SMTP_PASSWORD>
EMAIL_FROM_ADDRESS=no-reply@zainstreat.com
EMAIL_BUSINESS_NOTIFY_ADDRESS=orders@zainstreat.com
```

`docker-compose.yml`'s `api` service `environment:` block (after
`PUBLIC_API_URL`, line 66) — all five default to empty so local dev needs
zero setup:

```yaml
      SES_SMTP_HOST: ${SES_SMTP_HOST:-}
      SES_SMTP_USERNAME: ${SES_SMTP_USERNAME:-}
      SES_SMTP_PASSWORD: ${SES_SMTP_PASSWORD:-}
      EMAIL_FROM_ADDRESS: ${EMAIL_FROM_ADDRESS:-}
      EMAIL_BUSINESS_NOTIFY_ADDRESS: ${EMAIL_BUSINESS_NOTIFY_ADDRESS:-}
```

`docker-compose.prod.yml` needs **no changes** — its `api.environment`
block merges with (doesn't reset) the base file's, same as
`COOKIE_SECURE`/`PUBLIC_API_URL` today; the real values come from the
deploy host's own `.env` file, never committed.

**5i. `docs/deployment.md`** — add a short "Email (Amazon SES)" section:
verify the sending domain in the SES console (DNS records), generate SMTP
credentials (SES console → "SMTP settings" → "Create SMTP credentials" —
distinct from IAM access keys), request production access to leave the
sandbox (new SES accounts can only send to verified recipients until
this is granted), then set the five env vars in the deploy host's `.env`.
Flag this as a manual, pre-launch, do-it-once step — same posture as the
existing GHCR PAT paragraph in this same file.

## Group 6 — Manual verification

No new automated test suite for the frontend groups (1-3) — covered by
`/validate`'s lint/build/smoke sweep plus the manual checks in
`validation.md`. Group 4's email-validation logic gets inline Rust unit
tests (5b note). Run `/validate` once Groups 1–5 are done.
