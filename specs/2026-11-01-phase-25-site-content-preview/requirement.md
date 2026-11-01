# Phase 25 — Admin-Editable Site Content, Live Preview & Bento Menu: Requirements

## Source

`specs/roadmap.md`, Phase 25 (before this planning pass):

> - Admin can replace site imagery that is currently hardcoded: homepage
>   images, the kitchen photo, the contact-page photo, and the **logo**,
>   all uploaded to MinIO like menu photos (Phase 14).
> - Admin can edit the homepage text, per locale.
> - Swap all remaining stock/placeholder homepage imagery for the client's
>   real food photos (some supplied already; the client is sending the
>   rest).

Phase 24's requirement.md, "Out of scope": "Phase 25's admin-editable
homepage text has to handle three locales. That's for Phase 25 to plan."

The user added these requirements when planning started (2026-11-01):

> - update email address from techsihir to contact@zainstreat.com for
>   staging and production.
> - lets update things with SES to contact@zainstreat.com
> - use a responsive bento grid for menu items based on the Item list:
>   items more content should take more space and smaller for the later.
> - we need to add realtime preview in the admin page for when making
>   changes to the website. the preview should be in a device simulator
>   with all possible device types to capture responsiveness (mobile-s -
>   mobile xl to desktop xl)
> - we should also have preview after changes before committing the
>   changes.

## Context

### What exists

- **Email (Phases 17, 22, 24).**
  - `apps/api/src/email.rs` reads `SES_SMTP_HOST`, `SES_SMTP_USERNAME`,
    `SES_SMTP_PASSWORD`, `EMAIL_FROM_ADDRESS` and
    `EMAIL_BUSINESS_NOTIFY_ADDRESS`. `sender()` adds the display name
    "Zain's Treat n More". No email sets a `Reply-To`.
  - GitHub Environment variables (checked 2026-11-01):

    | Variable | staging | production |
    |---|---|---|
    | `EMAIL_FROM_ADDRESS` | `no-reply@zainstreat.com` | `no-reply@zainstreat.com` |
    | `EMAIL_BUSINESS_NOTIFY_ADDRESS` | `techsihir@gmail.com` | unset |

    Production's unset notify address keeps **all** production email off
    (Phase 22 plan 0.3).
  - SES `eu-central-1` (checked 2026-11-01): the domain identity
    `zainstreat.com` is verified (DKIM), plus `techsihir@gmail.com` and
    `abdulbasitsaeed619@gmail.com` as sandbox test recipients. Production
    access is still `DENIED` (case 179346172100474), 200 sends a day.
  - DNS: Namecheap is the registrar, but the nameservers are
    **Route 53**, in an AWS account other than the one the AWS MCP tools
    reach. `zainstreat.com` has **no MX record**, so nothing at
    `@zainstreat.com` can receive mail. The SPF record
    (`include:spf.efwd.registrar-servers.com`) is Namecheap's
    email-forwarding SPF, which can't work without Namecheap DNS. DMARC
    is `p=none`.
  - The public contact page shows `zainstreat@gmail.com`
    (`app/[locale]/contact/page.tsx:56` and `ContactPage.emailValue` in
    the three message files).
  - `docs/deployment.md` says staging's notify address is "the
    developer's, not the business's".
- **Menu grid.** `app/[locale]/menu/page.tsx` renders each category as
  `grid-cols-[repeat(auto-fit,minmax(260px,1fr))]` of identical
  `MenuItemCard`s (180px photo, name, description, price or price
  options, add-to-cart). The homepage's "featured" section reuses
  `MenuItemCard` for up to 6 featured items. Every item has a photo
  (`image_url` is required by `validate_menu_item`). What varies per item
  is description length, the number of price options (0, or 1 or more)
  and `is_featured`.
- **Site imagery.** Eight static imports from `apps/web/assets/images/`
  (`hero`, four `category-*`, `about-hero`, `about-mission`, `contact`),
  used by the home, about, services and contact pages through
  `SiteImage` (`StaticImageData` with a blur placeholder and a GSAP
  float). The logo is `public/logo.png` (`components/logo.tsx`), also
  used as the Open Graph image in `[locale]/layout.tsx`.
- **Homepage text.** The `HomePage` namespace in
  `messages/{en,nl,fr}.json`: 29 keys, two of them (`heroHeadline`,
  `whyChooseUs`) with `<em>` rich-text tags.
- **Admin editing (Phase 14).** Menu items and categories save straight
  to the live rows. `update_menu_item` uses optimistic concurrency on
  `updated_at`, fully replaces the price options, and deletes the
  superseded MinIO photo after commit. `AdminImageUpload` always crops to
  a 1:1 square and keeps the file's type (JPEG/PNG/WebP).
- **Admin host (Phase 21).** `proxy.ts` rewrites every non-`/api/` path
  on `cms.<host>` onto `app/admin/**`. The admin session cookie is
  host-only on `cms.<host>`. Neither Caddy nor Next sets
  `X-Frame-Options` or `frame-ancestors`, so the app can be framed by
  itself.
- **Public pages.** `/menu` is `force-dynamic`, and the homepage is
  dynamic too, because `apiFetch` uses `cache: "no-store"` for its menu
  fetch. About, services and contact are static.

## Decisions

### Email and SES

1. **`contact@zainstreat.com` replaces the email addresses in four
   places** (asked during planning: all four):
   - `EMAIL_BUSINESS_NOTIFY_ADDRESS` on **staging** (was
     `techsihir@gmail.com`) and **production** (was unset).
   - `EMAIL_FROM_ADDRESS` on both (was `no-reply@zainstreat.com`), so
     customer replies reach the business.
   - `Reply-To` on every email (Decision 1a).
   - The public contact page (`mailto:` and `ContactPage.emailValue` in
     en/nl/fr), replacing `zainstreat@gmail.com`.

1a. **Reply-To depends on who the email is for** (asked after planning).
    - **Customer emails** (invoice, status updates, quote) get
      `Reply-To: contact@zainstreat.com`, which is the
      `EMAIL_FROM_ADDRESS` value.
    - **Owner notifications** get the **customer's** address as
      Reply-To, so the owner can reply to the customer directly. These
      are: new order, payment claimed, quote accepted or declined, new
      catering enquiry and new contact message.
    - `spawn_send` takes the Reply-To as a parameter, so each call site
      chooses.
1b. **Staging mail goes to the business inbox, with a `[STAGING]`
    subject prefix** (asked after planning).
    - A new optional `EMAIL_SUBJECT_PREFIX` variable is set to
      `[STAGING] ` on staging and unset on production. The API puts it in
      front of every subject.
    - This replaces the "staging notifies the developer" rule in
      `docs/deployment.md`.
2. **Setting production's notify address turns production email on**
   at the next promote. That's intended (the user asked for production).
   `project_production_deferred` keeps production rollout out of phase
   specs, so this phase only sets the variable. The promote that makes it
   live is the normal Phase 21 flow, outside this phase's checklist.
3. **Mail to `contact@` is received through ImprovMX forwarding**
   (asked after planning).
   - DNS for `zainstreat.com` is served by **Route 53**. Namecheap is only
     the registrar. Namecheap's free forwarding expects Namecheap DNS,
     which is why the old `spf.efwd.registrar-servers.com` include never
     led to working mail.
   - ImprovMX is free and works with any DNS: two MX records, an SPF
     include and an alias.
   - The alias forwards `contact@` to **`abdulbasitsaeed619@gmail.com`**
     (the developer) for now (asked after planning). Before production
     launch, switch it to the owner's inbox. That's an ImprovMX
     dashboard edit with no code or DNS change, and validation.md tracks
     it.
4. **DNS records** (Route 53):
   ```
   zainstreat.com.  MX   10 mx1.improvmx.com.
   zainstreat.com.  MX   20 mx2.improvmx.com.
   zainstreat.com.  TXT  "v=spf1 include:spf.improvmx.com include:amazonses.com ~all"
   ```
   - The Namecheap include goes, and SES is added.
   - SES mail already passes DMARC through its DKIM signature, which
     ImprovMX leaves alone when it forwards. That's why forwarded owner
     notifications don't land in spam.
   - DMARC stays `p=none`.
   - **Claude makes the change, with access** (asked after planning).
     The hosted zone isn't in the AWS account the AWS MCP tools use
     (`ListHostedZones` there returns nothing). The user grants access
     to the account that owns the zone, for example a named AWS profile
     or a role Claude can assume. Claude shows the record diff and gets
     confirmation before calling `ChangeResourceRecordSets`.
   - The **ImprovMX account** is created by the user, because it's a
     signup on an outside service. Claude gives the steps.
5. **SES console updates.**
   - No new identity is needed: the verified `zainstreat.com` domain
     covers sending from, and sandbox delivery to, `contact@`.
   - Set the account's contact email (`PutAccountDetails`
     `AdditionalContactEmailAddresses`) to `contact@zainstreat.com`.
   - Re-submit the production-access request from that account. The
     2026-10-31 request was denied. Use `contact@` as the contact, the
     live site URL, and a description of transactional order and quote
     mail with no marketing.
   - Keep `techsihir@gmail.com` and `abdulbasitsaeed619@gmail.com` as
     sandbox test recipients until production access is granted, then
     delete them.
6. **One new env variable**, `EMAIL_SUBJECT_PREFIX` (Decision 1b),
   optional and empty by default. It's wired the same way as the other
   email variables: `docker-compose.yml`, `_deploy.yml`'s env rendering,
   `.env.example` and `infra/env-parity.sh` if it lists variables. The
   address changes are GitHub Environment variable edits.
6a. **The admin can see failed customer emails** (asked after planning,
    as the fallback in case SES stays in the sandbox).
    - A new `email_deliveries` table records each **customer** email:
      `order_id` or `quote_id`, `kind` (invoice, status or quote),
      `recipient`, `status` (`queued`, `sent` or `failed`), `error`
      and timestamps.
    - The spawned send task updates the row. An SES sandbox rejection
      (`554 … not verified`) fails at SMTP time, so it's recorded as
      `failed` with the error.
    - The admin order detail and the enquiry/quote detail show "Customer
      email may not have been delivered" with the time and kind, next to
      the existing WhatsApp and copy-link buttons, so the owner can
      follow up there.
    - Owner notifications aren't tracked. The owner would be the one
      reading the warning anyway.
    - It's still fire-and-forget. A failed insert or update is logged and
      never fails the request (Phase 17).

### Bento menu grid

7. **Card size comes from the item's content**, computed by a pure
   function `bentoSize(item)` in `apps/web/lib/menu-bento.ts`. There's no
   admin field. Four sizes:

   | Size | Span (from `sm`) | When |
   |---|---|---|
   | `feature` | 2 cols × 2 rows | score ≥ 5 |
   | `wide` | 2 cols × 1 row | score ≥ 3 |
   | `tall` | 1 col × 2 rows | 3 or more price options and score < 3 |
   | `regular` | 1 × 1 | otherwise |

   `score = descriptionLength / 60 + max(0, priceOptions − 1) + (is_featured ? 2 : 0)`.
   Featured items count for 2 points, so the owner's picks get more
   room. The thresholds live in one constant so they're easy to tune
   once real content is in.
8. **Responsive columns.** 1 column below `sm` (640px), 2 from `sm`, 3
   from `lg` (1024px), 4 from `xl` (1280px).
   - **Dense packing** (`grid-flow-row-dense`, asked after planning), so
     the grid has no holes. Dense packing can show a later small card
     before an earlier large one. DOM order still drives reading and tab
     order (WCAG 1.3.2 is about meaning, and a menu list's order carries
     none).
   - Spans only apply from `sm`. On a phone every card is full width,
     in list order. A 2-column span on a 2-column grid is full width,
     which is intended.
   - Rows use `auto-rows-[minmax(11rem,auto)]` so text never clips.
   - **Few items:** a category whose spans add up to less than the
     column count gets fewer columns. 1–2 regular items use at most 2
     columns, and a lone `feature` item uses 2. Each section stays full
     width, with no mostly empty rows. The column class comes from a
     fixed lookup.
9. **Size changes the layout, not just the box.**
   - `regular`: today's card.
   - `tall`: a taller photo with the full price-option list.
   - `wide`: photo on the left and text on the right (from `sm`).
   - `feature`: a large photo with the text below and the description
     not truncated.

   `MenuItemCard` takes a `size` prop. The add-to-cart controls stay the
   same at every size.
10. **Used on `/menu` and the homepage featured section** (asked during
    planning). Both render through one `MenuBentoGrid` component. Order
    still follows `display_order`. Dense packing can move a small card
    up into a gap (Decision 8).

### Editable site content

11. **One site-content document, stored with a published and a draft
    copy.** A new single-row table `site_content` holds `published JSONB`
    and `draft JSONB NULL`. The JSON shape is typed on both sides (a serde
    struct and a TS interface):

    ```jsonc
    {
      "images": {            // MinIO URLs, null = bundled default
        "logo": null, "hero": null,
        "category_meals": null, "category_snacks": null,
        "category_catering": null, "category_event_rentals": null,
        "about_hero": null, "about_mission": null, "contact": null
      },
      "home_text": {         // per-locale overrides of HomePage keys
        "en": { "heroEyebrow": "…" }, "nl": {}, "fr": {}
      }
    }
    ```

    A missing or null value falls back to today's static image or message
    string. An empty database therefore renders exactly as today.
    The web caches the published document under a `site-content` tag and
    revalidates it when a publish includes site content. That keeps about,
    services and contact static even though the layout reads the logo.
    "Kitchen photo" in the roadmap is `about_hero`, which the homepage's
    about preview also uses.
12. **Editable homepage text** is the `HomePage` keys except
    `metaTitle`/`metaDescription` (SEO, Phase 26). There's one tab per
    locale (en/nl/fr), and each field shows the message-file default as
    its placeholder. `heroHeadline` and `whyChooseUs` accept only
    `<em>…</em>` markup. The API rejects any other tag or a key outside
    the allowed list (`400` with `field`). Values are trimmed. An empty
    value removes the override.
13. **Image slots reuse the menu upload path** (`POST /api/admin/media`,
    MinIO, immutable keys). `AdminImageUpload` gains an `aspect` prop:
    - `hero`, `about_*`, `contact`: 4:3
    - `category_*`: 1:1 (they render as circles)
    - `logo`: no crop, PNG or WebP only, so transparency survives

    `SiteImage` accepts a URL as well as `StaticImageData`. Remote images
    have no blur placeholder and keep the float. The Open Graph image
    follows the logo slot.
14. **Real photos.** The client's photos are uploaded through the new
    admin screen on staging. That's a content task in validation, not
    code. Any slot still holding stock imagery at the end of the phase is
    listed in validation.md.

### Draft → preview → publish

15. **Every content edit goes to a draft first** (asked during planning:
    site content and the menu). This covers site content, menu items and
    categories. The public site only ever reads published data. A
    **Publish** step makes the draft live.
    - **Site content:** `site_content.draft`.
    - **Published menu items and categories:** a nullable `draft JSONB`
      column holding the same payload as today's upsert request
      (`UpsertMenuItemRequest` / the category upsert), plus
      `draft_updated_at`.
    - **New menu items and categories** are created **unpublished**: a
      new `published_at TIMESTAMPTZ NULL` column, backfilled to
      `created_at` for existing rows. An unpublished row's live columns
      are its content, so edits write them directly. Publishing sets
      `published_at`.
16. **What stays immediate.** The availability toggle and archive are
    day-to-day operations, not content changes, so they keep writing the
    live row at once. The owner marking a dish sold out mid-service can't
    wait for a publish. Archiving an item with a pending draft also
    discards the draft.
17. **Public reads and ordering ignore drafts and unpublished rows.**
    These all add `published_at IS NOT NULL` and never read `draft`:
    - `GET /api/menu-items` and `GET /api/categories`
    - order submission's item lookup (`create_order`)
    - the featured query

    An unpublished item can't be ordered, which extends `mission.md`'s
    "only available items can be ordered". Prices on orders come from the
    published row, so historical-order accuracy is unaffected.
18. **Drafts are validated like today's saves.** `validate_menu_item`
    and the category validation run on every draft save. An invalid form
    shows field errors and isn't saved, so the preview keeps the last
    valid draft. A saved draft is always publishable. Publish re-runs the
    validation, because the category could have been archived since the
    draft was saved.
19. **Publish is atomic and checks concurrency.**
    - `POST /api/admin/publish` takes a list of entries
      (`{kind: "site_content" | "menu_item" | "category", id?, draft_updated_at}`)
      and applies them in one transaction: draft → live columns, price
      options fully replaced, `published_at` set if null, `draft` cleared.
    - A `draft_updated_at` mismatch returns `409` for the whole request,
      as Phase 14 does for `updated_at`.
    - Superseded MinIO images are deleted after commit, exactly like
      `update_menu_item` today.
    - **Discard**
      (`DELETE /api/admin/drafts/{kind}/{id}`) clears a draft. It also
      deletes the draft's own uploaded image if that image isn't the live
      one. Discarding an unpublished new item archives it.
20. **An admin "Changes" page** (`cms.<host>/changes`) lists every
    pending draft and unpublished row: what it is, what changed (a
    field-level before/after diff) and when. It has per-entry **Preview**,
    **Publish** and **Discard**, and a **Publish all** button. The admin
    nav shows a badge with the pending count.
    - Menu and category forms change their button from "Save" to "Save
      draft" (drafts autosave, Decision 22).
    - Forms also get "Publish this change" and "Discard draft". An item
      with a pending draft shows a "Draft" badge in the admin list.

### Live preview in a device simulator

21. **The preview renders the real public pages with draft data, framed
    on the admin host.**
    - `cms.<host>/preview/{locale}/{path}` is exempted from the `/admin`
      rewrite. When Next.js draft mode is on, `proxy.ts` rewrites it to
      the public `/{locale}/{path}` route.
    - Draft mode is enabled by `GET /api/preview/enable` on the cms host.
      That route checks the admin session with the API first, then calls
      `(await draftMode()).enable()`.
    - In draft mode, `lib/api.ts`'s public fetchers call new admin
      endpoints (`GET /api/admin/preview/menu-items`, `…/categories`,
      `…/site-content`), forwarding the request's cookie. Those endpoints
      return the public response shape with drafts overlaid and
      unpublished rows included.
    - Without an admin session these return `401`, and the page falls
      back to published data.
    - Why this design:
      - It needs no token or new secret. The draft-mode cookie is signed
        by Next and host-only on `cms.`, and the data still needs the
        admin session.
      - Static public pages stay static for real visitors. Draft mode
        only bypasses prerendering for that request.
      - The same-origin iframe can talk to the admin page with
        `postMessage`.
    - Public pages never see draft data (`proxy.ts` only rewrites
      `/preview/` on the cms host).
22. **"Realtime" means autosave plus refresh.**
    - Admin forms autosave the draft 700 ms after the last valid change.
    - On a successful save, the editor posts `{type: "zt:refresh"}` to
      every preview iframe.
    - A `PreviewBridge` client component calls `router.refresh()`. It's
      rendered in `[locale]/layout.tsx` only when draft mode is on, so
      the server components re-render with the new draft and keep their
      scroll position.
    - Typical edit-to-preview time is about 1–1.5 s on staging. Pushing
      unsaved form state straight into the iframe was rejected. Every
      server-rendered section would need a client-side twin, and the
      preview would stop being the real page.
23. **Device simulator presets** (CSS px, portrait):

    | Preset | Size | Covers |
    |---|---|---|
    | Mobile S | 320 × 568 | iPhone SE (1st gen), small Android |
    | Mobile M | 375 × 667 | iPhone SE 2/3, 8 |
    | Mobile L | 414 × 896 | iPhone 11 / XR, large Android |
    | Mobile XL | 430 × 932 | iPhone Pro Max / Plus |
    | Tablet | 768 × 1024 | iPad mini / iPad portrait |
    | Tablet L | 1024 × 1366 | iPad Pro 12.9" portrait |
    | Laptop | 1280 × 800 | small laptop (`xl` breakpoint) |
    | Desktop | 1440 × 900 | standard desktop |
    | Desktop XL | 1920 × 1080 | full HD |

    - The iframe is always at the preset's **real width**, so CSS media
      queries and Tailwind breakpoints fire exactly as on the device. It
      is scaled down with `transform: scale()` to fit the pane.
    - Mobile and tablet presets have a rotate toggle (landscape). A
      zoom control offers fit, 50%, 75% and 100%.
    - An **"All devices"** view shows every preset side by side, each
      scaled to the same height, and refreshes them together.
    - Phone presets get a simple rounded bezel. Desktop presets get a
      plain browser-chrome bar showing the path.
24. **Where the simulator appears.**
    - As a docked, resizable side panel in the site-content editor and
      in the menu item and category forms. It opens on the page the
      content affects: home for site content and featured items, `/menu`
      for items and categories.
    - Full screen from the Changes page, to review everything before
      **Publish all**.
    - A page picker (Home, Menu, About, Services, Contact) and a locale
      picker (en/nl/fr) sit above it.
25. **In-preview navigation and side effects.**
    - `PreviewBridge` catches link clicks inside the iframe and keeps
      navigation under `/preview/…`. It reports the current path to the
      parent, which keeps the page picker in sync.
    - Add-to-cart, checkout, order, quote and contact-form submissions
      are disabled in preview, with a small "Preview — ordering disabled"
      ribbon. The cart, order and quote routes aren't in the page picker.
26. **No new library.** The simulator is plain React and CSS, and draft
    mode is built into Next.js. `tech-stack.md` gains a "Content
    preview" section and updates its Site imagery row.

## Out of scope

- Draft/preview for **orders, quotes, enquiries or the availability
  toggle**. They aren't site content (Decision 16).
- Per-locale **menu** names and descriptions (still Phase 24's
  "out of scope"). Bento sizing uses the single description.
- Editable text on pages other than the homepage, including SEO
  `metaTitle`/`metaDescription` (Phase 26).
- The favicon (`app/icon.png`). It stays a build-time asset.
- Scheduled publishing, version history or rollback of published
  content beyond Publish/Discard.
- Real device or screenshot testing services (BrowserStack and so on).
  The simulator emulates viewport size, not device browsers.
- A custom SES MAIL FROM domain, BIMI, or tightening DMARC beyond
  `p=none`.
- Production rollout and promote (`project_production_deferred`). This
  phase sets production's variables only (Decision 2).
- Moving SES to the client's AWS account (Phase 29).

## Open risks flagged during planning

All the original risks and assumptions were resolved with the user on
2026-11-01. Each one keeps its resolution so the reasoning stays
traceable.

1. **Forwarding target.** *Resolved:* forward to
   `abdulbasitsaeed619@gmail.com` for now (Decision 3). **Still open:**
   switch the ImprovMX alias to the owner's inbox before production
   launch. validation.md tracks it as a pre-launch item.
2. **Forwarded SES mail filed oddly or marked as spam by Gmail.**
   *Resolved:* ImprovMX forwards without changing the body, so SES's
   DKIM signature still verifies and DMARC passes (Decision 4). Owner
   notifications now also carry the customer's Reply-To (Decision 1a),
   so they read as normal messages. The first staging order is still
   checked in the real inbox (validation).
3. **SES production access may be denied again.** *Mitigated:* failed
   customer emails are recorded and shown in admin with the WhatsApp
   follow-up buttons (Decision 6a). Owner notifications to `contact@`
   work in the sandbox, because the domain is verified. **Still open:**
   the AWS decision itself, which is outside our control.
4. **Dense packing changes visual order.** *Resolved:* dense packing is
   kept on purpose (Decision 8).
5. **Bento with very few items.** *Resolved:* the column count follows
   the category's total span (Decision 8, plan 2.2).
6. **The draft model touches order-critical queries.** *Resolved:* the
   API tests cover them (plan 4.3). An unpublished item isn't listed and
   gives `400` when ordered. A published item with a pending draft is
   ordered at its **published** price.
7. **Preview cost.** *Resolved:*
   - "All devices" mounts each frame only when it scrolls into view
     (`IntersectionObserver`).
   - Only visible frames get a `zt:refresh`, staggered 150 ms apart.
   - Off-screen frames refresh when they come back into view.
   - The docked panel shows one frame.

   So one autosave costs at most a few server renders, never nine at
   once (plan 10.3).
8. **iOS Safari and draft mode inside the iframe.** *Resolved:*
   - The iframe is same-origin with the admin page, so it isn't a
     third-party cookie and WebKit's ITP doesn't block it.
   - As a fallback, if a frame hasn't posted `zt:path` within 5 s of
     loading, the simulator shows "Preview couldn't start here — open in
     a new tab". That opens `/api/preview/enable?...` top-level (plan
     10.3).
   - Checked on the owner's iPhone (validation).
9. **Autosave and optimistic concurrency.** *Resolved:* the client keeps
   one save in flight, queues only the latest value, and sends and
   receives `draft_updated_at` on every save. A real conflict stops
   autosave with "Conflict — reload" (plan 9.1).
10. **DNS access** (found after planning). *Resolved:* the user grants
    Claude access to the Route 53 account, and Claude confirms each
    record change before making it (Decision 4).
11. **ImprovMX is a third-party dependency.** *Accepted:* if it ever
    stops working, `contact@` stops receiving, but sending (SES) is
    unaffected. The free tier's limits are far above this site's volume.
    Moving to Zoho or Google Workspace later is a DNS change.
