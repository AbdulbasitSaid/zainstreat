# Phase 25 — Admin-Editable Site Content, Live Preview & Bento Menu: Validation

Partly checked on 2026-11-01, against a local stack: the API on a
throwaway Postgres and MinIO, plus `next start`, on the `cms.localhost`
and `localhost` hosts. Ticked items were verified there or by
`cargo test`, `clippy`, `pnpm lint`, `pnpm build` and
`pnpm check:messages`. Unticked items need the ops steps (DNS, ImprovMX,
GitHub variables, SES), staging, or a human eye. Deviations are in
plan.md's "Implementation notes".

## Pass/fail checklist

### Email, forwarding and SES (Groups 1, 1b)

- [ ] `dig +short MX zainstreat.com` returns `10 mx1.improvmx.com.` and
      `20 mx2.improvmx.com.`.
- [ ] `dig +short TXT zainstreat.com` shows
      `v=spf1 include:spf.improvmx.com include:amazonses.com ~all`, any
      other apex TXT values are unchanged, and the Namecheap `efwd`
      include is gone.
- [ ] The SES DKIM CNAMEs still resolve, and SES `GetEmailIdentity
      zainstreat.com` still shows `VerificationStatus: SUCCESS`.
- [ ] ImprovMX's dashboard shows `zainstreat.com` verified. A mail from
      an outside address to `contact@zainstreat.com` arrives at
      `abdulbasitsaeed619@gmail.com`.
- [ ] `gh variable list --env staging` shows `EMAIL_FROM_ADDRESS` and
      `EMAIL_BUSINESS_NOTIFY_ADDRESS` = `contact@zainstreat.com` and
      `EMAIL_SUBJECT_PREFIX` = `[STAGING] `.
- [ ] `--env production` shows both addresses = `contact@zainstreat.com`
      and no `EMAIL_SUBJECT_PREFIX`.
- [ ] `techsihir@gmail.com` appears in neither environment.
- [x] `grep -rn "techsihir" --exclude-dir=specs .` returns nothing.
      (Earlier specs keep their history.)
- [x] `grep -rn "zainstreat@gmail" apps/web` returns nothing. The contact
      page in en, nl and fr shows and links `contact@zainstreat.com`.
- [x] `cargo test email` passes, covering:
      - the default Reply-To is the From address
      - an explicit Reply-To is used
      - the `[STAGING] ` prefix is applied when set, absent when unset,
        and gets a trailing space added
- [ ] `EMAIL_SUBJECT_PREFIX` is in `docker-compose.yml`, `_deploy.yml`
      and `.env.example`, and `infra/env-parity.sh` passes.
- [ ] SES `GetAccount` shows `AdditionalContactEmailAddresses` =
      `["contact@zainstreat.com"]` and a new review case. Record its
      ID and status here.
- [ ] On staging, after a redeploy, place a test order with a verified
      customer address:
      - The owner notification arrives in the forwarded inbox, not in
        spam. Its subject starts with `[STAGING] ` and its sender is
        "Zain's Treat n More" <contact@zainstreat.com>.
      - Gmail's "Show original" shows `DKIM: PASS` and `DMARC: PASS`.
      - Replying to the owner notification addresses the **customer**.
      - The customer email has the same From and prefix, and replying to
        it addresses `contact@zainstreat.com`.
      - The same holds for a catering enquiry and a contact message
        (their Reply-To is the sender).
- [ ] **Delivery tracking** (Group 1b):
      - `sqlx migrate run`/`revert`/`run` passes for `email_deliveries`.
      - On staging, an order for an **unverified** address while SES is
        in the sandbox records a `failed` row with the `554 … not
        verified` error.
      - The admin order detail shows the "may not have been delivered"
        notice next to the WhatsApp and copy-link buttons.
      - A verified address records `sent` and shows no notice.
      - The quote email works the same way on the enquiry detail.
- [x] `cargo test --test order_submission --test admin_orders` passes,
      including the delivery cases.
- [x] `.env.example` and `docs/deployment.md` show `contact@`, the
      ImprovMX setup and the staging prefix, and no longer say staging
      notifies the developer.
- [ ] **Pre-launch, not a blocker for this phase:** before production
      launch, switch the ImprovMX `contact` alias to the owner's inbox
      (requirement.md Risk 1). Record the date here.

### Bento grid (Group 2)

- [x] `lib/menu-bento.ts` is the only place sizes are worked out, and
      `bentoSize` returns all four sizes for suitable inputs. Check with a
      quick `node`/`tsx` script or from the seed data.
- [ ] `/en/menu` at 320 and 375px is one column, with every card full
      width and in `display_order`.
- [ ] At 640, 1024 and 1280px, items with long descriptions, several
      price options or the featured flag span 2 columns and/or 2 rows.
      Short items are 1 × 1. No text is clipped, and no card overflows
      its cell.
- [ ] A category with 1–2 items doesn't leave a mostly empty 4-column
      row at `xl` (the `COLUMNS_FOR_SPAN` lookup).
- [ ] Dense packing leaves no holes at `lg` and `xl` with the seed menu.
      Keyboard tab order still follows `display_order`.
- [x] The homepage featured section uses the same grid.
- [ ] Add-to-cart works from every card size, and the cart count updates.

### Migration and draft-safe public reads (Groups 3–4)

- [x] `sqlx migrate run`, `revert` and `run` succeed on the dev DB.
      Existing items and categories have `published_at` set, and
      `site_content` has one row.
- [x] `SQLX_OFFLINE=true cargo build --release` succeeds.
- [x] `cargo test --test menu_browsing --test order_submission`:
      - an unpublished item isn't listed
      - ordering an unpublished item gives `400`
      - a published item with a pending draft is listed and ordered at
        its published price

### Draft, preview and publish API (Group 5)

- [x] `cargo test --test admin_drafts --test site_content` passes, with
      every case in plan 5.8.
- [x] Every new admin route returns `401` without a session:
      ```sh
      for p in site-content changes preview/menu-items preview/categories preview/site-content; do
        curl -s -o /dev/null -w "%{http_code} $p\n" http://localhost:8080/api/admin/$p; done
      ```
- [x] `curl -s localhost:8080/api/site-content` returns `{}` (or the
      published document). It never contains draft-only values.
- [x] The old `PATCH /api/admin/menu-items/{id}` and
      `PATCH /api/admin/categories/{id}` are gone (`405`/`404`).
- [x] `cargo clippy --all-targets -- -D warnings` passes.

### Site content on the public site (Group 7)

- [ ] With an empty `site_content`, every public page looks the same as
      on `master`. Check the homepage, about, services and contact
      side by side.
- [ ] A published hero image, logo and `fr` `heroHeadline` with `<em>`
      show on `/fr`, while `/en` and `/nl` keep their defaults for that
      key. The logo also changes in the footer and in `og:image`.
- [ ] `pnpm build`'s route table still shows about, services and contact
      as static (○), not dynamic (ƒ). *Not applicable: they are already
      dynamic (ƒ) on `master` (next-intl reads request headers), and this
      phase doesn't change that. See plan.md Implementation notes.*
- [x] After **Publish**, the public homepage shows the new content on the
      next load, without waiting for the 300 s revalidation.
- [x] `pnpm check:messages` passes, and fails if a `HomeTextKey` is
      removed from `HomePage` or from the Rust key list.

### Preview isolation (Groups 7–8)

- [x] `curl -s -o /dev/null -w "%{http_code}" https://<public-host>/preview/en`
      returns `404`.
- [x] `curl -s -o /dev/null -w "%{http_code}" https://<public-host>/api/preview/enable?locale=en&path=/`
      returns `404`.
- [x] `curl` of `https://cms.<host>/api/preview/enable?...` without a
      session returns `401`.
- [x] `https://cms.<host>/preview/en` without draft mode shows published
      content only.
- [x] `/api/preview/enable?locale=en&path=//evil.example` and
      `path=/cart` are rejected (`400`), so there's no open redirect.
- [x] The public HTML of `/en` doesn't contain `zt:refresh` or the
      preview ribbon.
- [ ] In preview, add-to-cart and the contact and catering forms are
      disabled. Clicking header and footer links stays inside
      `/preview/…`, and the page picker follows.

### Admin editing and Changes (Group 9)

- [ ] **Menu item edit:**
      - Changing the description shows "Draft saved" about 0.7 s after
        typing stops.
      - The docked preview updates in about 1.5 s or less.
      - `/en/menu` on the public host is unchanged.
- [ ] An invalid edit (empty name) shows the field error and isn't saved.
      The preview keeps the last valid draft.
- [ ] Two tabs editing the same item: the second save shows
      "Conflict — reload" (`409`).
- [ ] **Publish this change** makes it public. **Discard draft**
      restores it.
- [ ] Replacing a photo and then discarding deletes the uploaded object
      from MinIO, and the live photo stays.
- [ ] A new item shows "Unpublished", is absent from the public menu,
      appears in the preview, and goes public on publish.
- [ ] The availability toggle and archive take effect on the public
      site straight away, without a publish.
- [ ] **Site content:** every image slot uploads with the right crop.
      The logo keeps its transparency, uncropped. "Reset to default"
      works.
- [ ] The en, nl and fr text tabs show defaults as placeholders.
      `<script>` in `heroHeadline` gives an inline error.
- [ ] **Changes page:**
      - It lists every pending entry with a field diff (image fields as
        thumbnails).
      - The nav badge count matches.
      - **Publish all** publishes everything in one request and clears
        the list.
      - A stale entry makes the whole publish fail with the reload
        message.

### Device simulator (Group 10)

- [x] All nine presets are offered (Mobile S 320 through Desktop XL
      1920). Each frame's label shows the real CSS width.
- [ ] Inside each frame, `window.innerWidth` matches the preset (check
      with devtools on the iframe). Tailwind breakpoints switch where
      they should: one column at Mobile S–XL, two at Tablet, three at
      Tablet L (1024), and four at Laptop and up.
- [ ] Rotate swaps width and height for phone and tablet presets, and
      isn't offered for desktop.
- [ ] Fit scales Desktop XL to fit the docked panel. 100% shows it at
      real size with scrolling.
- [ ] **All devices** shows nine frames.
      - Only frames near the viewport have mounted an iframe. Check the
        devtools Elements panel for the iframe count.
      - One autosave refreshes only the visible frames, about 150 ms
        apart.
      - Scrolling a stale frame into view refreshes it.
- [ ] Blocking the preview (for example
      `/api/preview/enable` returning `401` after logout) shows "Preview
      couldn't start here" with a working "Open in a new tab" within
      about 5 s.
- [ ] The page picker and locale picker switch the frame, and the
      choice survives a reload (localStorage).
- [ ] The full-screen simulator opens from the Changes page on the
      right page.
- [ ] The preview works on the owner's iPhone (Safari) in the admin's
      full-screen simulator: draft mode survives inside the iframe
      (Risk 8).

### Content and whole-phase checks (Group 11)

- [ ] The client's real photos are uploaded and published on staging.
      Slots still on stock imagery: *(list here, or "none")*.
- [ ] `docs/deployment.md` has the "Editing the site" section.
- [ ] `specs/tech-stack.md` "Content preview" matches what was built.
- [ ] `/validate` passes (lint, build, `check:messages`, smoke test).

## Definition of done

`contact@zainstreat.com` is the sender and the owner notification
address on staging and production. Customer emails reply to `contact@`,
and owner notifications reply to the customer. Staging subjects carry
`[STAGING] `. Mail sent to `contact@` is forwarded by ImprovMX (MX and SPF
in Route 53), for now to the developer's inbox. SPF authorises SES, DKIM
and DMARC pass, and SES production access has been re-requested with
`contact@` as the contact. Failed customer emails are recorded and shown
in admin next to the WhatsApp follow-up. The public contact page shows `contact@`, and `techsihir`
appears nowhere outside earlier specs. The menu page and the homepage
featured section use a responsive bento grid: one column on phones, up to
four on wide screens, with each card's size worked out from its content.
The owner can replace the logo and site images and edit the homepage
text in en/nl/fr from `cms.<host>/site`. Every content change (site
content, menu items, categories) is saved as a draft and previewed live.
The preview is the real public page with draft data, in a device
simulator covering Mobile S through Desktop XL, one device or all at
once. The draft only reaches the public site through Publish, from the
form or the Changes page, where Publish all applies everything in one
atomic step. Public pages and ordering never read drafts or unpublished
items. Availability and archive stay immediate, and there's no new
library. `cargo test`, `cargo clippy`, lint, build, `check:messages` and
`/validate` pass.
