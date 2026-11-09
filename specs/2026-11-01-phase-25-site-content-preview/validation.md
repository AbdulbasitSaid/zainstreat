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

- [x] `dig +short MX zainstreat.com` returns `10 mx1.improvmx.com.` and
      `20 mx2.improvmx.com.`.
- [x] `dig +short TXT zainstreat.com` shows
      `v=spf1 include:spf.improvmx.com include:amazonses.com ~all`, any
      other apex TXT values are unchanged, and the Namecheap `efwd`
      include is gone.
- [x] The SES DKIM CNAMEs still resolve, and SES `GetEmailIdentity
      zainstreat.com` still shows `VerificationStatus: SUCCESS`.
- [x] ImprovMX's dashboard shows `zainstreat.com` verified. A mail from
      an outside address to `contact@zainstreat.com` arrives at
      `abdulbasitsaeed619@gmail.com`. *2026-11-09: forwarding works,
      confirmed by the user.*
- [x] `gh variable list --env staging` shows `EMAIL_FROM_ADDRESS` and
      `EMAIL_BUSINESS_NOTIFY_ADDRESS` = `contact@zainstreat.com` and
      `EMAIL_SUBJECT_PREFIX` = `[STAGING] `.
- [x] `--env production` shows both addresses = `contact@zainstreat.com`
      and no `EMAIL_SUBJECT_PREFIX`.
- [x] `techsihir@gmail.com` appears in neither environment.
- [x] `grep -rn "techsihir" --exclude-dir=specs .` returns nothing.
      (Earlier specs keep their history.)
- [x] `grep -rn "zainstreat@gmail" apps/web` returns nothing. The contact
      page in en, nl and fr shows and links `contact@zainstreat.com`.
- [x] `cargo test email` passes, covering:
      - the default Reply-To is the From address
      - an explicit Reply-To is used
      - the `[STAGING] ` prefix is applied when set, absent when unset,
        and gets a trailing space added
- [x] `EMAIL_SUBJECT_PREFIX` is in `docker-compose.yml`, `_deploy.yml`
      and `.env.example`, and `infra/env-parity.sh` passes.
      *2026-11-09: in all three (commented out in `.env.example`, since
      it's unset by default). The script only runs on the server; the
      staging deploy (run 41343934662, with `[STAGING] ` set) printed
      "Env parity: no changes".*
- [x] SES `GetAccount` shows `ProductionAccessEnabled: true` with the
      review `GRANTED`. Record the case ID and quota here. *(Reworded
      2026-11-09. It used to require `AdditionalContactEmailAddresses` =
      `contact@` and a new review case. See plan.md Implementation notes.)*
      *2026-11-01: `PutAccountDetails` returned `ConflictException` while
      case 179346172100474 was `DENIED`, so the re-request went in as a
      reply to that case in AWS Support Center.*
      *2026-11-09: granted on case 179346172100474. Quota is 50,000/day
      at 14/s, and `EnforcementStatus` is `HEALTHY`.*
- [x] AWS account Operations alternate contact = `contact@zainstreat.com`
      (`aws account get-alternate-contact --alternate-contact-type
      OPERATIONS`). This replaces the SES `AdditionalContactEmailAddresses`
      setting. *2026-11-09: the user set it, and Claude confirmed it shows
      "Zain's Treat n More", Operations, `contact@zainstreat.com`.*
- [x] On staging, after a redeploy, place a test order with a verified
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
      *2026-11-09, Claude in Chrome on staging (SES out of sandbox):
      order ZTM-10 plus "I've paid", one contact message and one catering
      enquiry. Customer addresses were Gmail plus-aliases
      (`abdulbasitsaeed619+customer@`, `+contact@`, `+catering@`). All
      five emails arrived in `abdulbasitsaeed619@gmail.com`: the new
      order, the payment claim, the ZTM-10 invoice, the contact message
      and the catering enquiry. Each has From "Zain's Treat n More"
      <contact@zainstreat.com> and a `[STAGING] ` subject, with SPF,
      DKIM (`zainstreat.com` and `amazonses.com`) and DMARC passing.
      Reply-To on the three owner notifications is the customer's
      `+customer@` address, and on the contact and catering emails it's
      the sender's own alias. The invoice's Reply-To is
      `contact@zainstreat.com`. The four mails forwarded through
      ImprovMX also carry an `improvmx.net` DKIM pass. One exception:
      Gmail put the **catering enquiry** in Spam, though it passed every
      check like the others. Probably content-based. Mark it "Not spam"
      once, and check again after launch.*
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

- *Staging run 2026-11-01 (`51-102-179-66.sslip.io`, from `master` at
  333e7ff): placed order ZTM-8 (verified Gmail), order ZTM-9
  (`staging-unverified-test@example.com`) and one contact message. SES
  `SentLast24Hours` went 38 → 42, which matches 4 sends with ZTM-9's
  invoice rejected by the sandbox. Still to check by eye: the Gmail
  inbox (prefix, sender, DKIM/DMARC, Reply-To) and the ZTM-9 admin page
  ("may not have been delivered"). Production stays on the SES sandbox
  for now (user, 2026-11-01). Out of sandbox since 2026-11-09 (see
  the SES item above). Staging now delivers to any address, so test
  orders use our own inboxes or the SES mailbox simulator. The
  simulator's bounce comes back after SES accepts the mail, so it
  records `sent`, not `failed`. ZTM-9 is the only `failed` example.*

### Bento grid (Group 2)

*Revised 2026-11-09 for the pattern layout (requirement.md Decisions
7–10). The first version's checks are replaced.*

- [ ] `lib/menu-bento.ts` is the only place placement, shape and tint
      are worked out. `pnpm check:bento` (plan 2.1) passes for counts 1–17:
      no overlapping cells, no holes except beside a lone tail item, and
      no touching `lg` tiles with the same tint.
- [ ] `/en/menu` at 320 and 375px is one column, every card full width
      and in `display_order`.
- [ ] At 640 and 768px the 2-column pattern shows, and at 1024, 1280
      and 1440px the 4-column pattern does. Each category's grid has no
      holes, and no tile grows past its rows.
- [ ] No text is clipped mid-word except clamped descriptions, at every
      width above, including items with long names and several price
      options.
- [ ] Every tint passes WCAG AA contrast for its name, description,
      price, button and the "Unavailable" badge.
- [ ] Keyboard tab order follows `display_order` and matches the visual
      order at every width.
- [x] The homepage featured section uses the same grid.
- [ ] Add-to-cart works from every tile shape. For a multi-price item
      the dialog opens, lists every option with its price, adds the one
      chosen, closes, and the cart count updates. Esc closes it too.
- [ ] `pnpm check:messages` passes with `fromPrice` and `chooseOption`
      in en, nl and fr.

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
- [x] `docs/deployment.md` has the "Editing the site" section.
- [x] `specs/tech-stack.md` "Content preview" matches what was built.
- [x] `/validate` passes (lint, build, `check:messages`, smoke test).
      *2026-11-09, on the docker compose stack: `check:messages` (331
      keys), `pnpm lint`, `pnpm build`, `cargo clippy -D warnings`,
      `cargo build` and the full `cargo test` all pass. Smoke test:
      `/health` 200, `/` → `/en`, `/en` `/nl` `/fr` 200 with the right
      `lang`, the menu, about, services and contact pages 200, the
      contact page shows `contact@zainstreat.com`, and the preview
      isolation checks above give the same 401/404/400 results.*

## Definition of done

`contact@zainstreat.com` is the sender and the owner notification
address on staging and production. Customer emails reply to `contact@`,
and owner notifications reply to the customer. Staging subjects carry
`[STAGING] `. Mail sent to `contact@` is forwarded by ImprovMX (MX and SPF
in Route 53), for now to the developer's inbox. SPF authorises SES, DKIM
and DMARC pass, and SES production access is granted, with `contact@`
as the AWS account's Operations contact. Failed customer emails are recorded and shown
in admin next to the WhatsApp follow-up. The public contact page shows `contact@`, and `techsihir`
appears nowhere outside earlier specs. The menu page and the homepage
featured section use a responsive bento grid: one column on phones, then
a repeating, hand-designed tile pattern (2 columns on tablets, 4 on
desktops) with brand-tinted tiles of fixed height.
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
