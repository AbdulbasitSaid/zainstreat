# Phase 25 — Admin-Editable Site Content, Live Preview & Bento Menu: Implementation Plan

**Ordering:**
- Group 1 (email/SES ops), Group 1b (delivery tracking) and Group 2
  (bento grid) don't depend on the draft work, so they can ship first.
  Run `cargo test` after Group 1b.
- Groups 3–6 (drafts, site content, publish) are API-first. Run
  `cargo test` after Groups 3, 4 and 5.
- Groups 7–9 (web) depend on Groups 3–5. Group 10 (simulator) depends on
  Group 8.
- Run `/validate` after Groups 2, 9 and 11.

Paths refer to `master` at `0b7f7c1` (Phase 24 merged).

## Group 0 — Preconditions

Depends on: nothing.

0.1. **Phase 24 is on `master`** (PR #40). Done.
0.2. **Route 53 access** (requirement.md Decision 4). The user gives
     Claude access to the AWS account that hosts the `zainstreat.com`
     zone, through a named AWS profile or a role to assume. Check it with
     a read-only `route53 ListHostedZonesByName --dns-name zainstreat.com`
     using that profile, plus `ListResourceRecordSets` to see the current
     MX, TXT and `_amazonses`/DKIM records. Don't change anything yet.
0.3. **ImprovMX account.** The user signs up at improvmx.com (free plan)
     and adds the domain `zainstreat.com`. Claude can't do this signup.
0.4. **Staging is up** for the email, preview and iPhone checks in
     validation.md.

## Group 1 — `contact@zainstreat.com`, forwarding and SES (ops + repo)

Depends on: Group 0. requirement.md Decisions 1–6.

1.1. **ImprovMX alias:** `contact` → `abdulbasitsaeed619@gmail.com`
     (Decision 3).
1.2. **Route 53 records** (Decision 4). Show the user the before/after
     diff and get confirmation, then send one `ChangeResourceRecordSets`
     batch:
     ```
     UPSERT zainstreat.com. MX  300  10 mx1.improvmx.com.
                                     20 mx2.improvmx.com.
     UPSERT zainstreat.com. TXT 300  "v=spf1 include:spf.improvmx.com include:amazonses.com ~all"
     ```
     - The TXT `UPSERT` replaces the whole record set. If the zone has
       other apex TXT values (for example a site verification), carry
       them over unchanged. Only the `v=spf1` value changes.
     - Wait for `GetChange` to report `INSYNC`.
     - Check with `dig +short MX zainstreat.com` and
       `dig +short TXT zainstreat.com`.
     - ImprovMX's dashboard should show the domain as verified.
1.3. **Send a test mail** from any outside address to
     `contact@zainstreat.com`. It should arrive at
     `abdulbasitsaeed619@gmail.com`.
1.4. **GitHub Environment variables** (confirm with the user before
     changing them, since they're outward-facing config):
     ```sh
     gh variable set EMAIL_FROM_ADDRESS --env staging --body contact@zainstreat.com
     gh variable set EMAIL_BUSINESS_NOTIFY_ADDRESS --env staging --body contact@zainstreat.com
     gh variable set EMAIL_SUBJECT_PREFIX --env staging --body "[STAGING] "
     gh variable set EMAIL_FROM_ADDRESS --env production --body contact@zainstreat.com
     gh variable set EMAIL_BUSINESS_NOTIFY_ADDRESS --env production --body contact@zainstreat.com
     ```
     - Production has no `EMAIL_SUBJECT_PREFIX`.
     - Staging picks the values up on its next deploy. Production only
       picks them up on its next promote (Decision 2).
     - Check that `_deploy.yml`'s env rendering keeps the trailing space
       in `"[STAGING] "`. If the render trims it, the API adds the space
       itself (1.6).
1.5. **SES account details** (AWS MCP `run_script`, `eu-central-1`):
     - `sesv2 PutAccountDetails` with `MailType=TRANSACTIONAL`,
       `WebsiteURL=https://zainstreat.com`, `ContactLanguage=EN`,
       `AdditionalContactEmailAddresses=["contact@zainstreat.com"]`.
     - `UseCaseDescription` should cover:
       - order confirmations, order status updates and catering quotes,
         sent only to customers who placed an order or enquiry
       - no marketing and no purchased lists
       - about 5–30 emails a day
       - bounces and complaints watched through the SES dashboard, with
         failed sends shown to the owner in admin (Decision 6a)
     - Use `ProductionAccessEnabled=true` to re-submit.
     - Record the new case ID in validation.md.
     - **Don't** delete the two sandbox test identities yet
       (Decision 5).
1.6. **`apps/api/src/email.rs`** (Decisions 1a, 1b):
     - `SesSettings` gains `subject_prefix: String`, read with
       `std::env::var("EMAIL_SUBJECT_PREFIX").unwrap_or_default()`. It's
       optional and isn't part of the "all five set" check. If the
       prefix is non-empty and doesn't end in a space, add one.
     - Replace `spawn_send(to, subject, body)` with:
       ```rust
       pub struct OutgoingEmail {
           pub to: String,
           pub subject: String,
           pub body: String,
           /// `None` → the From address (customer emails); owner
           /// notifications pass the customer's address.
           pub reply_to: Option<String>,
           /// Customer emails only (requirement.md Decision 6a).
           pub track: Option<DeliveryRef>,
       }

       pub fn spawn_send(&self, email: OutgoingEmail)
       ```
     - Build the message in a pure `build_message(&SesSettings, &OutgoingEmail) -> Result<Message, String>`:
       ```rust
       let from = sender(&settings.from_address)?;
       let reply_to = match &email.reply_to {
           Some(address) => address.parse::<Mailbox>().map_err(|e| e.to_string())?,
           None => from.clone(),
       };
       Message::builder()
           .from(from)
           .reply_to(reply_to)
           .to(email.to.parse::<Mailbox>().map_err(|e| e.to_string())?)
           .subject(format!("{}{}", settings.subject_prefix, email.subject))
       ```
       An unparsable customer Reply-To is logged, and the From address
       is used instead. It never drops the notification.
     - **Unit tests:**
       - the default Reply-To is the branded From
       - an explicit Reply-To is used
       - `[STAGING] ` is prefixed when set
       - no prefix when unset
       - a prefix without a trailing space gets one
1.7. **Call sites** (Decision 1a). Owner notifications pass
     `reply_to: Some(customer email)`:
     - `orders.rs:370` new order
     - `orders.rs:521` payment claimed
     - `quotes.rs:243` quote accepted, and `quotes.rs:320` declined (the
       enquiry's email)
     - `catering_enquiries.rs:151`
     - `contact_messages.rs:99`

     Customer emails pass `reply_to: None` and a `track` (Group 1b):
     - `send_invoice` (orders and quote acceptance)
     - `admin/orders.rs:336` status
     - `admin/quotes.rs:399` quote
1.8. **Contact page.**
     - `app/[locale]/contact/page.tsx:56`: `mailto:contact@zainstreat.com`.
     - `ContactPage.emailValue` → `contact@zainstreat.com` in
       `messages/{en,nl,fr}.json`.
     - `grep -rn zainstreat@gmail apps/web` should then return nothing.
1.9. **Env wiring for `EMAIL_SUBJECT_PREFIX`:**
     - `docker-compose.yml`: `EMAIL_SUBJECT_PREFIX: ${EMAIL_SUBJECT_PREFIX:-}`
       next to the other email variables.
     - `.github/workflows/_deploy.yml`:
       `V_EMAIL_SUBJECT_PREFIX: ${{ vars.EMAIL_SUBJECT_PREFIX }}`, plus
       its line in the rendered `.env`.
     - `.env.example` gets a commented `# EMAIL_SUBJECT_PREFIX=` with a
       `# [var]` tag.
     - Update `infra/env-parity.sh` if it lists variables.
1.10. **Docs.**
      - `.env.example`: `EMAIL_FROM_ADDRESS=contact@zainstreat.com` and
        `EMAIL_BUSINESS_NOTIFY_ADDRESS=contact@zainstreat.com`. Drop the
        "developer's address on staging" comment.
      - `docs/deployment.md`:
        - In "Staging", replace the "developer's, not the business's"
          sentence: staging notifies `contact@zainstreat.com` with a
          `[STAGING] ` subject prefix (Phase 25).
        - In the email section, document the ImprovMX forwarding (which
          inbox, and how to switch it), the MX and SPF records, the Route
          53 account that holds the zone, Reply-To, and the
          failed-delivery warning.

## Group 1b — Customer email delivery tracking (API + admin web)

Depends on: Group 1 (1.6). requirement.md Decision 6a.

1b.1. **Migration `email_deliveries`** (separate from Group 3's, so this
      can ship first):
      ```sql
      -- Phase 25: record customer email outcomes so admin can flag failures.
      CREATE TABLE email_deliveries (
          id BIGSERIAL PRIMARY KEY,
          order_id BIGINT REFERENCES orders(id) ON DELETE CASCADE,
          quote_id BIGINT REFERENCES catering_quotes(id) ON DELETE CASCADE,
          kind TEXT NOT NULL CHECK (kind IN ('invoice', 'status', 'quote')),
          recipient TEXT NOT NULL,
          status TEXT NOT NULL DEFAULT 'queued' CHECK (status IN ('queued', 'sent', 'failed')),
          error TEXT,
          created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
          updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
          CHECK ((order_id IS NULL) <> (quote_id IS NULL))
      );
      CREATE INDEX idx_email_deliveries_order ON email_deliveries (order_id) WHERE order_id IS NOT NULL;
      CREATE INDEX idx_email_deliveries_quote ON email_deliveries (quote_id) WHERE quote_id IS NOT NULL;
      ```
1b.2. **Tracking in `email.rs`:**
      - `DeliveryRef { pool: PgPool, order_id: Option<i64>, quote_id: Option<i64>, kind: &'static str }`.
      - `spawn_send` inserts a `queued` row inside the spawned task. When
        SMTP returns, it updates the row to `sent`, or to `failed` with
        the error truncated to 500 chars.
      - Errors from the insert or update are `tracing::warn!`ed and never
        surface.
      - With email disabled (no SES config), no row is written, since
        nothing was attempted.
1b.3. **Admin API.** `GET /api/admin/orders/{id}` and the enquiry/quote
      detail response gain
      `customer_email: { status, kind, error, at } | null`, the latest
      row by `created_at`.
1b.4. **Admin web.** On the order detail and enquiry detail pages, a
      `failed` latest delivery shows a warning `Notice`: "Customer email
      may not have been delivered (status update, 14:05). Follow up on
      WhatsApp or copy the link." It sits next to the existing share
      buttons. `queued` older than 5 minutes shows the same notice,
      worded as "not confirmed".
1b.5. **Tests** (`tests/order_submission.rs`, `tests/admin_orders.rs`,
      using `tests/common.rs`'s disabled-email setup):
      - Insert a `failed` row directly and check the admin detail returns
        it.
      - A unit test that `build_message` errors become a `failed` status
        string.

## Group 2 — Bento menu grid (web)

Depends on: Group 0. requirement.md Decisions 7–10.

2.1. **`apps/web/lib/menu-bento.ts`:**
     ```ts
     import type { MenuItem } from "@/lib/api";

     export type BentoSize = "regular" | "tall" | "wide" | "feature";

     // Tuned against real menu content; see Phase 25 requirement.md Decision 7.
     const THRESHOLDS = { feature: 5, wide: 3, tallOptions: 3 } as const;
     const CHARS_PER_POINT = 60;

     export function bentoScore(item: MenuItem): number {
       const descriptionLength = item.description?.trim().length ?? 0;
       return (
         descriptionLength / CHARS_PER_POINT +
         Math.max(0, item.price_options.length - 1) +
         (item.is_featured ? 2 : 0)
       );
     }

     export function bentoSize(item: MenuItem): BentoSize {
       const score = bentoScore(item);
       if (score >= THRESHOLDS.feature) return "feature";
       if (score >= THRESHOLDS.wide) return "wide";
       if (item.price_options.length >= THRESHOLDS.tallOptions) return "tall";
       return "regular";
     }

     export const BENTO_SPAN: Record<BentoSize, string> = {
       regular: "",
       tall: "sm:row-span-2",
       wide: "sm:col-span-2",
       feature: "sm:col-span-2 sm:row-span-2",
     };
     ```
     Write the class strings out in full (as above), never build them
     from pieces, so Tailwind's scanner finds them.
2.2. **`components/menu-bento-grid.tsx`** (server component):
     ```tsx
     export function MenuBentoGrid({ items }: { items: MenuItem[] }) {
       return (
         <div className="grid grid-flow-row-dense auto-rows-[minmax(11rem,auto)] grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
           {items.map((item) => {
             const size = bentoSize(item);
             return (
               <div key={item.id} className={BENTO_SPAN[size]}>
                 <MenuItemCard item={item} size={size} />
               </div>
             );
           })}
         </div>
       );
     }
     ```
     **Few items** (requirement.md Decision 8). Compute
     `totalSpan = Σ (sm column span)`, then pick the column classes from
     a fixed lookup, never string-built:
     ```ts
     const COLUMNS_FOR_SPAN = {
       1: "grid-cols-1 sm:grid-cols-2 sm:max-w-[50%]", // a lone regular card stays card-sized
       2: "grid-cols-1 sm:grid-cols-2",
       3: "grid-cols-1 sm:grid-cols-2 lg:grid-cols-3",
       many: "grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4",
     } as const;
     ```
     `grid-flow-row-dense` stays on in every case (requirement.md
     Decision 8). Check `1`'s `max-w` by eye. If it looks off, drop the
     `max-w` and let the single card span both columns.
2.3. **`components/menu-item-card.tsx`** gains `size?: BentoSize`
     (default `"regular"`). The root gets `h-full`.
     - `tall`: photo `h-[260px]`.
     - `wide`: `sm:flex-row`, photo `sm:w-1/2 sm:h-full`.
     - `feature`: photo `h-[220px] sm:h-[320px]`.
     - Update `sizes` to match: feature and wide use
       `(max-width: 640px) 100vw, 50vw`.
     - `AddToCartControls` is unchanged.
2.4. **Swap the grids** in `app/[locale]/menu/page.tsx` and the
     homepage featured section of `app/[locale]/page.tsx` for
     `<MenuBentoGrid items={…} />`.
2.5. **Check by eye** on `/en/menu` with seed data at 320, 640, 1024
     and 1440px wide (the browser or, once Group 10 exists, the
     simulator). Adjust `THRESHOLDS` if the seed menu comes out all
     `regular` or all `feature`, and note the final values in
     requirement.md Decision 7.

## Group 3 — Draft/publish migration (repo)

Depends on: Group 0. requirement.md Decisions 11, 15.

3.1. **`sqlx migrate add -r site_content_drafts`** in `apps/api`. Up:
     ```sql
     -- Phase 25: draft → preview → publish for site content and the menu.
     CREATE TABLE site_content (
         id BOOLEAN PRIMARY KEY DEFAULT true CHECK (id),
         published JSONB NOT NULL DEFAULT '{}'::jsonb,
         draft JSONB,
         draft_updated_at TIMESTAMPTZ,
         published_at TIMESTAMPTZ,
         updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
     );
     INSERT INTO site_content DEFAULT VALUES;

     ALTER TABLE menu_items
         ADD COLUMN draft JSONB,
         ADD COLUMN draft_updated_at TIMESTAMPTZ,
         ADD COLUMN published_at TIMESTAMPTZ;
     UPDATE menu_items SET published_at = created_at;

     ALTER TABLE categories
         ADD COLUMN draft JSONB,
         ADD COLUMN draft_updated_at TIMESTAMPTZ,
         ADD COLUMN published_at TIMESTAMPTZ;
     UPDATE categories SET published_at = created_at;

     CREATE INDEX idx_menu_items_pending
         ON menu_items (id) WHERE deleted_at IS NULL AND (draft IS NOT NULL OR published_at IS NULL);
     CREATE INDEX idx_categories_pending
         ON categories (id) WHERE deleted_at IS NULL AND (draft IS NOT NULL OR published_at IS NULL);
     ```
     Down: drop the indexes, the columns and `site_content`.
3.2. **`apps/api/seed.sql`**: give seeded rows `published_at = now()`,
     so staging's seed is live.
3.3. Regenerate the offline cache:
     `cargo sqlx prepare -- --all-targets`. Commit `.sqlx/`.

## Group 4 — Public reads and ordering ignore drafts (API)

Depends on: Group 3. requirement.md Decision 17.

4.1. `src/routes/menu_items.rs` (public list) and
     `src/routes/categories.rs`: add `AND published_at IS NOT NULL`. A
     published item in an unpublished category isn't listed either:
     join, and require both.
4.2. `src/routes/orders.rs:193`: add `AND published_at IS NOT NULL` to
     the item lookup. An unpublished id gets the same `400` as an
     archived one.
4.3. **Tests** (`tests/menu_browsing.rs`, `tests/order_submission.rs`):
     - An unpublished item isn't listed.
     - Ordering an unpublished item gives `400`.
     - A published item with a pending draft (draft price 99.00) is
       listed and ordered at its **published** price, and
       `order_items.unit_price` is the published one.

## Group 5 — Draft, site-content, preview and publish endpoints (API)

Depends on: Group 4. requirement.md Decisions 11–13, 15–21.

5.1. **`src/site_content.rs`** — typed document:
     ```rust
     #[derive(Serialize, Deserialize, Default, Clone)]
     #[serde(deny_unknown_fields)]
     pub struct SiteContent {
         #[serde(default)]
         pub images: SiteImages,
         #[serde(default)]
         pub home_text: BTreeMap<String, BTreeMap<String, String>>, // locale → key → text
     }

     #[derive(Serialize, Deserialize, Default, Clone)]
     #[serde(deny_unknown_fields)]
     pub struct SiteImages {
         pub logo: Option<String>,
         pub hero: Option<String>,
         pub category_meals: Option<String>,
         pub category_snacks: Option<String>,
         pub category_catering: Option<String>,
         pub category_event_rentals: Option<String>,
         pub about_hero: Option<String>,
         pub about_mission: Option<String>,
         pub contact: Option<String>,
     }

     pub const HOME_TEXT_KEYS: &[&str] = &[
         "heroEyebrow", "heroHeadline", "heroSupporting", "orderNow", "bookCatering",
         "whatsappUs", "trustHalal", "trustFresh", "trustHygienic", "trustReliable",
         "trustHalalDesc", "trustFreshDesc", "trustHygienicDesc", "trustReliableDesc",
         "servicesMeals", "servicesSnacks", "servicesCatering", "servicesEventRentals",
         "whyChooseUsEyebrow", "whyChooseUs", "cateringCtaHeadline", "requestCateringQuote",
         "aboutPreview", "learnMore", "featuredMenuEyebrow", "featuredMenuHeading", "viewFullMenu",
     ];
     const RICH_KEYS: &[&str] = &["heroHeadline", "whyChooseUs"];
     ```
     `validate(&SiteContent, &media)` rejects these with a `400` naming
     the field (`home_text.fr.heroHeadline`, `images.hero`):
     - a locale outside `Locale::ALL`
     - a key outside `HOME_TEXT_KEYS`
     - a value over 400 characters
     - any `<` in a plain key
     - any tag other than `<em>`/`</em>` (balanced) in a rich key
     - an image URL that's new and fails `is_valid_media_url`

     Empty strings are dropped before saving.
5.2. **Public `GET /api/site-content`**: returns `published`. The web
     caches it with a tag and revalidates on publish (7.3, 9.8), so the API
     sends no cache headers of its own.
5.3. **Admin site content:**
     - `GET /api/admin/site-content` returns `{published, draft, draft_updated_at}`.
     - `PUT /api/admin/site-content/draft` takes
       `{content, draft_updated_at}` (null on the first draft), validates
       it, and checks `draft_updated_at` (`409` on mismatch). It returns
       the new `draft_updated_at`.
5.4. **Menu item and category drafts:**
     - `PUT /api/admin/menu-items/{id}/draft` takes
       `UpsertMenuItemRequest` (its `updated_at` is replaced by
       `draft_updated_at`).
       - It runs `validate_menu_item`, with the grandfathering check
         against the **live** `image_url`.
       - On an **unpublished** row, it writes the live columns (today's
         `update_menu_item` path).
       - On a **published** row, it stores the payload in `draft`.
       - It **never deletes media**.
     - `PUT /api/admin/categories/{id}/draft` works the same way.
     - `POST /api/admin/menu-items` and `POST /api/admin/categories`
       now insert with `published_at = NULL`.
     - Remove the old direct `PATCH /api/admin/menu-items/{id}` and
       `PATCH /api/admin/categories/{id}` live-update routes.
       Availability and archive routes are unchanged (Decision 16),
       except that archive also clears `draft`.
     - Admin list and get responses gain `has_draft`,
       `is_published`, `draft` and `draft_updated_at`.
5.5. **Preview reads** (admin session required), returning the
     **public** response shapes:
     - `GET /api/admin/preview/menu-items?category_id=` lists published
       and unpublished, non-archived items, each overlaid with its
       `draft`. Price options come from the draft when one exists.
     - `GET /api/admin/preview/categories` does the same for categories.
     - `GET /api/admin/preview/site-content` returns `draft` if there is
       one, otherwise `published`.

     Write the overlay as one Rust function shared with publish, so
     preview and publish can't disagree.
5.6. **Publish and discard:**
     - `GET /api/admin/changes` lists every pending entry with
       `{kind, id, label, changed_fields: [{field, before, after}], draft_updated_at, is_new}`.
     - `POST /api/admin/publish` takes `{entries: [{kind, id?, draft_updated_at}]}`.
       In one transaction it locks each row (`FOR UPDATE`), checks
       `draft_updated_at` (any mismatch → `409` and a rollback),
       re-validates, and then:
       - **Site content:** sets `published = draft`, `draft = NULL` and
         `published_at = now()`.
       - **Menu item or category:** applies `draft` to the live columns,
         replaces the price options (Phase 14 Decision 6), sets
         `published_at = COALESCE(published_at, now())`, clears
         `draft` and bumps `updated_at`.

       After commit, it deletes superseded MinIO objects, as
       `update_menu_item` does today. This covers the images swapped out
       by the published versions of site content and menu items.
       It returns the published entries.
     - `DELETE /api/admin/drafts/{kind}/{id}` (`id` is omitted for site
       content) clears the draft. It deletes any image URL in the draft
       that isn't referenced by the live row. On an unpublished row it
       archives the row instead.
5.7. **Route wiring** in `src/routes/admin/mod.rs` and
     `src/routes/mod.rs`.
5.8. **Tests** (`tests/admin_drafts.rs`, `tests/site_content.rs`):
     - A draft save doesn't change `GET /api/menu-items`.
     - A preview read shows the draft.
     - Publish makes it public.
     - Discard restores, and a discarded draft image is deleted from
       MinIO (use the existing MinIO test helper in `tests/common.rs`).
     - A stale `draft_updated_at` gives `409`.
     - A new item is unlisted until it's published.
     - Archive clears a draft.
     - Site content: an unknown key, `<script>` in `heroHeadline`, the
       locale `de` and a foreign image URL each give `400` with the
       right `field`.
     - All preview, draft, changes and publish routes return `401`
       without a session.

## Group 6 — `tech-stack.md` and API docs (repo)

Depends on: Group 5.

6.1. Already done during planning: the "Content preview" section and
     the Site imagery and Content preview Summary Table rows. Check them
     against what was built and amend if anything changed.
6.2. Update the "Admin editing" notes in `docs/` (if any) for the new
     Save draft → Publish flow.

## Group 7 — Web data layer and draft mode (web)

Depends on: Group 5. requirement.md Decisions 11, 21.

7.1. **`lib/site-content.ts`:**
     ```ts
     export const SITE_IMAGE_SLOTS = [
       "logo", "hero", "category_meals", "category_snacks", "category_catering",
       "category_event_rentals", "about_hero", "about_mission", "contact",
     ] as const;
     export type SiteImageSlot = (typeof SITE_IMAGE_SLOTS)[number];

     export interface SiteContent {
       images: Partial<Record<SiteImageSlot, string | null>>;
       home_text: Partial<Record<Locale, Partial<Record<HomeTextKey, string>>>>;
     }
     ```
     - `HomeTextKey` is a union mirroring `HOME_TEXT_KEYS`.
     - Add a parity check to `scripts/check-messages.mjs`: every
       `HomeTextKey` exists in `HomePage`, and the Rust list matches
       (read `site_content.rs` with a regex).
7.2. **`lib/preview.ts`:**
     ```ts
     import { cookies, draftMode } from "next/headers";

     /** In draft mode, public fetchers read drafts from the admin preview API. */
     export async function previewFetchInit(): Promise<RequestInit | null> {
       if (!(await draftMode()).isEnabled) return null;
       const cookie = (await cookies()).toString();
       return { headers: { cookie }, cache: "no-store" };
     }
     ```
7.3. **`lib/api.ts`:**
     - `getMenuItems`, `getCategories` and a new `getSiteContent` check
       `previewFetchInit()`. If it's non-null, they call
       `/api/admin/preview/…` with it, and on `401` fall back to the
       public endpoint.
     - `getSiteContent()` fails soft: an API error returns `{}`, so
       defaults render.
     - The published `getSiteContent()` fetch uses
       `next: { tags: ["site-content"], revalidate: 300 }`, not
       `apiFetch`'s `no-store`. The layout reads it on every page, and
       `no-store` would make about, services and contact dynamic. The
       preview fetch stays `no-store`. Menu fetches already use
       `no-store`, so publishing menu changes needs no revalidation.
7.4. **`app/api/preview/enable/route.ts`** (cms host only; `proxy.ts`
     already passes `/api/` through there):
     - `GET` calls the API's `/api/admin/me` with the request cookie,
       and returns `401` if that fails.
     - Otherwise it runs `(await draftMode()).enable()`, then redirects
       to `/preview/${locale}${path}` taken from validated query params.
       The locale must be in `routing.locales`, and the path must start
       with `/` and be on the allowlist: `"" | /menu | /about | /services | /contact`.
     - On the public host it returns `404` (check the `host` header, as
       `proxy.ts` does).
7.5. **`app/api/preview/disable/route.ts`**: `(await draftMode()).disable()`.
7.6. **`proxy.ts`**, inside the `cms.` branch, before the `/admin`
     rewrite:
     ```ts
     // Live preview (Phase 25): /preview/{locale}/... renders the public
     // page with draft data; draft mode itself is checked by the page.
     if (pathname.startsWith("/preview/")) {
       const url = request.nextUrl.clone();
       url.pathname = pathname.slice("/preview".length);
       return NextResponse.rewrite(url);
     }
     ```
     The public host doesn't rewrite `/preview/` paths, so there they hit
     next-intl and 404.
7.7. **Images.** `SiteImage` accepts `src: StaticImageData | string`. A
     string gets no `placeholder="blur"`, and an `http://` string gets
     `unoptimized`, as in `MenuItemCard`.
     - `Logo` takes an optional `src` from the site content.
     - `[locale]/layout.tsx` fetches site content once, and uses the logo
       for the header and footer and for `openGraph.images`.
7.8. **Home text.** Add `lib/home-text.ts`, with
     `getHomeText(locale, content)`. It returns `t`, `rich` and `markup`
     functions with the same signatures the homepage uses today. They
     check `content.home_text[locale][key]` first and fall back to
     `getTranslations("HomePage")`. Rich overrides go through next-intl's
     `t.rich` parsing by formatting the override string with the same
     tag map, so only `<em>` renders. Use it in `app/[locale]/page.tsx`
     for every `t(...)` call in the edited key list.
7.9. **Image slots in pages.** Home, about, services and contact read
     `content.images.<slot> ?? staticImport`.

## Group 8 — Preview bridge (web)

Depends on: Group 7. requirement.md Decisions 22, 25.

8.1. **`components/preview-bridge.tsx`** (`"use client"`), rendered in
     `[locale]/layout.tsx` only when `(await draftMode()).isEnabled`:
     - Listen for `message` events with `event.origin === location.origin`
       and `data.type === "zt:refresh"` → `router.refresh()`.
     - Use a capture-phase click listener on `document`: for an `<a>`
       with a same-origin `href` whose path starts with a locale prefix,
       `preventDefault()` and `location.assign("/preview" + path)`.
       Hash links stay as they are, and external links open in a new tab.
     - Post `{type: "zt:path", path}` to `window.parent` on mount, and
       after each refresh.
     - Render a fixed ribbon: "Preview — ordering disabled".
8.2. **Disable side effects.**
     - Add a `PreviewContext` (`lib/preview-context.tsx`). When it's
       true, `AddToCartControls`, the order/catering/contact forms and
       the cart badge render disabled, with
       `title="Disabled in preview"`.
     - Cart and checkout routes are already off the allowlist (7.4).
8.3. A `PreviewBridge` in a production build sends no extra JS to normal
     visitors. Check with the build output's route sizes, or by grepping
     the public HTML for `zt:refresh`.

## Group 9 — Admin editing UI: drafts, Changes page, site content (web)

Depends on: Groups 7, 8. requirement.md Decisions 12–13, 18–20, 22, 24.

9.1. **`lib/use-autosave-draft.ts`** — a generic hook:
     ```ts
     export function useAutosaveDraft<T>(opts: {
       value: T;
       isValid: (value: T) => boolean;
       save: (value: T, draftUpdatedAt: string | null) => Promise<{ draft_updated_at: string }>;
       initialDraftUpdatedAt: string | null;
       onSaved: () => void; // posts zt:refresh to preview iframes
       delayMs?: number; // 700
     }): { status: "idle" | "saving" | "saved" | "error" | "conflict"; error: unknown };
     ```
     - Debounce, with only one save in flight. If a newer value arrives
       mid-save, save that once the current save returns (Risk 9).
     - On `409`, set `status: "conflict"` and stop autosaving until the
       admin reloads.
     - The status shows next to the form title: "Draft saved", "Saving…"
       or "Conflict — reload".
9.2. **Menu item and category forms** (`admin-menu-item-form.tsx`,
     `admin-category-form.tsx`):
     - Swap the submit-to-live call for `useAutosaveDraft` against
       `…/draft`.
     - Buttons: **Publish this change** and **Discard draft**, both
       calling the 5.6 endpoints. Ask for confirmation with an in-page
       dialog component (not `window.confirm`).
     - Creating an item still uses the POST, then goes to the edit page,
       which shows an "Unpublished" badge.
     - Field-level validation errors from a draft save show inline, as
       today.
9.3. **Admin lists** (`admin-menu-list.tsx`, `admin-category-list.tsx`):
     add "Draft" and "Unpublished" badges.
9.4. **Site content editor** at `app/admin/(protected)/site/page.tsx`
     (served at `cms.<host>/site`):
     - **Images:** one `AdminImageUpload` per slot, with
       `aspect` from 9.5. Each shows the current image (the draft's,
       the published one, or the bundled default with a "Default" tag)
       and has a "Reset to default" button that sets the slot to `null`.
     - **Homepage text:** en/nl/fr tabs. One input or textarea per
       `HomeTextKey`, with the message-file string as placeholder (import
       the three JSON files on the server side of the page). Rich keys
       get a hint ("Use <em>…</em> for the highlighted word").
     - Autosaves through `useAutosaveDraft`, with Publish and Discard
       buttons, the same as 9.2.
9.5. **`AdminImageUpload`** gains `aspect?: number | null` (default `1`).
     - `null` skips the cropper and uploads the file as-is. The logo
       accepts only `image/png` and `image/webp`.
     - Slot aspects: 4/3 for `hero`, `about_*` and `contact`, 1 for
       `category_*`, `null` for `logo`.
     - Update the helper text to match.
9.6. **Changes page** `app/admin/(protected)/changes/page.tsx`:
     - A table from `GET /api/admin/changes` showing the kind, label, a
       changed-fields diff (old struck through, new beside it, and image
       fields as thumbnails), "New" tags and times.
     - Each row has Preview (opens the full-screen simulator on the
       right page), Publish and Discard.
     - A **Publish all** button sends every entry in one request. On
       `409` it shows "Something changed since you loaded this page —
       reload".
9.7. **Admin nav** (`admin-nav.tsx`): add "Site content" and "Changes"
     links. "Changes" shows a count badge from a small
     `GET /api/admin/changes?count=1` call. Admin is already dynamic, so
     there's no caching to worry about.
9.8. Same-origin Route Handler proxies under `app/api/admin/…` for every
     new endpoint, following the existing `admin-*-api.ts` pattern. The
     publish proxy calls `revalidateTag("site-content")` after a `2xx`
     whose entries include `site_content`, so visitors see published
     content at once rather than after the 300 s fallback.

## Group 10 — Device simulator (web)

Depends on: Group 8. requirement.md Decisions 23–24.

10.1. **`lib/devices.ts`:**
      ```ts
      export interface DevicePreset {
        id: string;
        label: string;
        width: number;
        height: number;
        kind: "phone" | "tablet" | "desktop";
      }

      export const DEVICE_PRESETS: readonly DevicePreset[] = [
        { id: "mobile-s", label: "Mobile S", width: 320, height: 568, kind: "phone" },
        { id: "mobile-m", label: "Mobile M", width: 375, height: 667, kind: "phone" },
        { id: "mobile-l", label: "Mobile L", width: 414, height: 896, kind: "phone" },
        { id: "mobile-xl", label: "Mobile XL", width: 430, height: 932, kind: "phone" },
        { id: "tablet", label: "Tablet", width: 768, height: 1024, kind: "tablet" },
        { id: "tablet-l", label: "Tablet L", width: 1024, height: 1366, kind: "tablet" },
        { id: "laptop", label: "Laptop", width: 1280, height: 800, kind: "desktop" },
        { id: "desktop", label: "Desktop", width: 1440, height: 900, kind: "desktop" },
        { id: "desktop-xl", label: "Desktop XL", width: 1920, height: 1080, kind: "desktop" },
      ];
      ```
10.2. **`components/admin-device-frame.tsx`:** one preset.
      - An `<iframe>` with `width={w}` and `height={h}` (swapped when
        rotated) inside a wrapper sized `w*scale × h*scale`. The iframe
        gets `transform: scale(scale)` with `transform-origin: top left`.
      - `scale = min(1, availableWidth / w, availableHeight / h)` from a
        `ResizeObserver`, unless a fixed zoom (50/75/100%) is chosen.
      - Bezel: rounded-[2.5rem] border for phones, rounded-xl for
        tablets, and a 32px chrome bar with three dots and the current
        path for desktops.
      - The label underneath reads "Mobile S · 320 × 568 · 64%".
      - `title` attribute for accessibility.
      - Exposes `iframeRef` so the parent can post messages.
10.3. **`components/admin-device-simulator.tsx`** (`"use client"`):
      - **Toolbar:**
        - a preset selector: segmented buttons with icons, grouped as
          Phone, Tablet and Desktop
        - "All devices"
        - rotate (phone and tablet only)
        - zoom: Fit, 50%, 75% or 100%
        - the page picker: Home, Menu, About, Services, Contact
        - locale: en, nl or fr
        - "Open in new tab"
      - **Iframe source.** On mount, and whenever the page or locale
        changes, set the src to
        `/api/preview/enable?locale=…&path=…`. That enables draft mode
        and redirects to `/preview/…`.
      - Listens for `zt:path` to sync the page picker.
      - **Startup fallback** (requirement.md Risk 8). If a frame hasn't
        posted `zt:path` within 5 s of its `load` event, replace it with
        the message "Preview couldn't start here", plus an "Open in a new
        tab" button that opens the `/api/preview/enable?...` URL
        top-level.
      - Exposes `refresh()` through a ref or context. It posts
        `{type: "zt:refresh"}` with `targetOrigin` set to
        `location.origin`, to **visible frames only**, 150 ms apart.
        Hidden frames get a `stale` flag and refresh when they become
        visible again (requirement.md Risk 7).
      - **All devices:** a horizontally scrollable row of
        `AdminDeviceFrame`s, all scaled to the same height (say 560px),
        in preset order.
        - Each frame mounts its iframe only once an
          `IntersectionObserver` reports it within 200px of the
          viewport. Until then it shows an empty bezel of the right size.
        - Once mounted, a frame stays mounted. It isn't unmounted on
          scroll-out.
      - Remembers the last preset, zoom, page and locale in
        `localStorage`, wrapped in try/catch.
10.4. **Docked panel**, `components/admin-preview-panel.tsx`:
      - A right-side resizable panel (drag handle, 360–900px) holding
        the simulator.
      - It sits next to the forms on the menu item, category and site
        content pages. A "Preview" toggle shows or hides it, and it's
        hidden by default below `lg`. On a phone, the admin opens it
        full screen.
      - The form's `onSaved` calls the simulator's `refresh()`.
      - The default page is `/` for site content and featured items, and
        `/menu?category={id}` for items and categories. Extend the 7.4
        allowlist to accept `?category=`.
10.5. **Full-screen route** `app/admin/(protected)/preview-all/page.tsx`
      (served at `cms.<host>/preview-all`, because 7.6 claims `/preview/`). The
      simulator fills the viewport. The Changes page links to it with
      `?path=…`.

## Group 11 — Content, docs and validation (repo + staging)

Depends on: Groups 1–10.

11.1. Deploy to staging (merge to `master`, or run Staging up on the
      branch).
11.2. Upload the client's real photos through **Site content**, preview
      them in the simulator at every preset, then **Publish all**.
      List any slot still on stock imagery in validation.md.
11.3. `docs/deployment.md`: add a short "Editing the site" section
      covering Site content, drafts, the Changes page and the simulator.
11.4. Run `/validate` and work through `validation.md`.

## Implementation notes

Recorded while implementing (2026-11-01). Where these differ from the
groups above, these win.

- **Bento thresholds** (2.5): the seed menu came out almost all `regular`
  at `feature: 5, wide: 3`, so they're now `feature: 4, wide: 2.5`
  (requirement.md Decision 7 updated). With seed data, `/en/menu` gets
  wide and feature cards; `tall` needs 3+ price options, which no seed
  item has.
- **`published_at` defaults to `now()`** (Group 3). Hand-inserted rows
  (`seed.sql`, the production menu seed, the API tests) are live, and the
  admin create endpoints insert `NULL` explicitly so admin-created rows
  start unpublished. So `seed.sql` needs no change (3.2). The default is
  its own migration (`20261101184755_published_at_default`), because
  `site_content_drafts` was already committed.
- **Concurrency token.** `draft_updated_at` is the token for every content
  save. An unpublished row's draft save writes the live columns and also
  stamps `draft_updated_at`. Publish and discard clear it to `NULL`, so a
  stale tab's next save or publish gets `409`.
- **Unpublished items when ordering** (4.2/4.3): they get the same
  response as an archived item, which is `409 items_unavailable` with
  `reason: "not_found"`, not `400`.
- **Delivery status field** (1b.3): `GET /api/admin/orders/{id}` already
  has `customer_email` (the address), so the new field is
  `customer_email_delivery: {status, kind, error, at, needs_follow_up}`.
  The enquiry detail uses the same name. The API works out
  `needs_follow_up` (failed, or queued over 5 minutes), because React's
  purity lint rejects `Date.now()` in a server component. The order page
  has no copy-link button, so its notice offers WhatsApp only. The
  enquiry notice points to the quote editor's existing buttons.
- **Routes** (5.4): `GET /api/admin/categories/{id}` was added, so the
  category edit page loads one row. With the PATCH routes gone,
  `/menu-items/{id}` and `/categories/{id}` answer PATCH with `405`.
  Discard of site content is `DELETE /api/admin/drafts/site_content`.
- **Preview locale** (7.6): the `/preview/` rewrite bypasses next-intl's
  middleware, so `proxy.ts` sets next-intl's `X-NEXT-INTL-LOCALE` request
  header from the path. Without it, every preview rendered in English.
  The public host returns a direct `404` for `/preview/…` and
  `/api/preview/…`.
- **Static rendering** (validation, Group 7): on `master` every
  `[locale]` page, about, services and contact included, already builds
  as dynamic (ƒ), because next-intl reads the locale from request headers
  (no `setRequestLocale`). This phase doesn't change that. The tagged
  `getSiteContent()` fetch still avoids a per-request API call, and
  `revalidateTag("site-content", { expire: 0 })` on publish makes the next
  load fresh.
- **Home text overrides** (7.8): `lib/home-text.ts` builds a next-intl
  `createTranslator` over the merged messages. Braces in an override are
  ICU-escaped, and a malformed override falls back to the default string.
- **Simulator extras**: a "Reload" button (also retries failed frames).
  The page picker shows the frame's current path, but in-frame navigation
  doesn't reload the frame.
- **Confirmations** use a native `<dialog>` (`AdminConfirmDialog`), not
  `window.confirm`.
- **DNS is a Lightsail zone, not Route 53** (Groups 0–1): `zainstreat.com`
  is a Lightsail DNS zone in the same account (`686194638565`). The
  `awsdns` nameservers made it look like Route 53. No cross-account access
  was needed. On 2026-11-01 the user added the two MX records and replaced
  the SPF record in the Lightsail console, and Claude checked them with
  `GetDomain` and `dig`. SES identity and DKIM still show `SUCCESS`.
