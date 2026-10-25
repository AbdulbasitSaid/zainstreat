# Client call — 2026-10-18

Notes extracted from a ~17-minute recorded call with the client
(`Abuja.m4a`), recorded "to remember all the things we said we would do".
Personal/off-topic parts of the recording are omitted. Feeds Phases 20–28
in `specs/roadmap.md`.

## Goal

Official launch / start sharing the website publicly **the weekend after
the call** (≈ 2026-10-24/25). Work on the items below is targeted for the
week of 2026-10-19; the client is available to answer questions during
the week.

## Payments

- **Bank transfer first.** "It's food — you pay before we process your
  order." Cash is ruled out (no way to verify it; people in NL rarely pay
  cash). Payment options shown to the customer at checkout: **bank
  transfer** now, **iDEAL** later.
- **iDEAL is deferred**, not dropped: it needs a payment-provider account
  opened with the client's business details, which can't happen quickly.
  The business is registered.
- **VAT:** the client has applied for the **KOR** (Dutch small-business
  VAT exemption, < €20,000 revenue) → **no VAT on bank-transfer orders**.
  Once a payment platform like iDEAL is used, the platform reports
  revenue, so VAT must be handled (added to the price or absorbed) — a
  decision to settle when iDEAL is built.

## Order flow (agreed)

1. Customer places an order (choosing **pickup** or **delivery**) and
   receives an **invoice** with the bank account details.
2. The customer is told clearly that **only verified payments are
   processed**, and can follow their order's **progress**.
3. Customer pays, then **marks the order as paid** on the site.
4. The owner gets an **email** for the order/payment, checks the bank
   alert, and **manually confirms payment in admin** → order becomes
   *Confirmed*.
5. Client's preference: confirming payment should **automatically move
   the order to *Processing*** (no separate manual step).
6. When the food is done:
   - **Delivery:** owner sets *On the way* → customer gets an email.
   - **Pickup:** owner sets *Ready for collection* → customer comes to
     collect.
7. Order is **completed** once the customer has confirmed receipt/
   collection (verified with the owner).

Customer status changes are notified by **email**.

## Catering / events

Event orders do **not** go through the food-order flow (they involve
rentals, on-site serving, etc.). The customer **requests a catering
quotation**; the owner sends a quote; if accepted the customer pays with
the **same payment method**, the owner confirms, and it proceeds like an
order (incl. pickup/delivery). Builds on Phase 15's catering enquiry form.

## Notifications for the owner

- **Now:** email only, plus in-admin notifications while the admin page
  is open (it's not a mobile app).
- **Next:** **WhatsApp** notifications via the WhatsApp Business API
  (client has a WhatsApp Business account) — tapping through leads to the
  admin.
- **Investigate:** make the admin an installable **PWA** with push
  notifications so the owner is notified even when not on the site. The
  client uses an **iPhone** (iOS web push only works for home-screen–
  installed PWAs).

## Content & languages

- Add **French** alongside Dutch and English.
- Replace **all homepage images** with the right (real) food photos; the
  client has done some and will check/send the rest.
- Images the client can't currently change from admin (e.g. the kitchen
  photo, the contact-us photo) and **the logo** should become editable
  from admin; homepage text too.

## Analytics & search

- Add **Google Analytics** — traffic, pages, visitors who don't convert
  to an order, so drop-off can be investigated.
- Client agreed the site **should be findable on Google** (business is
  registered), but **the address should not be published** for now.
- Keep improving SEO.

## Hosting

The developer's AWS **free trial expires in a few weeks** → move
everything to the **client's own server/account** before then.
