# Phase 36 — Snacks, Rentals & Email Encoding Fix: Validation

## Pass/fail checklist

### Build and tests

- [ ] `cargo test` and `cargo clippy -- -D warnings` pass in `apps/api`,
      including the new email header, min-quantity, rentals and data
      migration tests.
- [ ] `apps/api/.sqlx/` is regenerated and the Docker build passes with
      `SQLX_OFFLINE=true`.
- [ ] Exactly two new migrations under `apps/api/migrations/`
      (`…_min_quantity_rentals`, `…_snacks_rentals_data`), each with a
      `.down.sql`; `sqlx migrate run` then `sqlx migrate revert` twice
      then `run` again works on a dev database.
- [ ] `pnpm check:messages`, `pnpm lint` and `pnpm build` pass in
      `apps/web`, with no `any`. No new packages.
- [ ] `/validate` passes.

### Email encoding

- [ ] A new order email and a new event booking email (staging) show
      "€115.00", "—", "you’re" and the emoji correctly in Gmail (web
      and iOS) and Apple Mail.
- [ ] The raw message source has `Content-Type: text/plain;
      charset=utf-8`.
- [ ] Subjects with non-ASCII still display correctly.

### Snacks

- [ ] `/en/menu` shows a **Snacks** category after the existing ones
      with the 13 items of Decision 6, prices matching the client's PDF;
      Puff Puff offers "50 pieces €30.00" and "100 pieces €45.00".
- [ ] Meat Pie, Snack Pack and Meat Skewer show "Minimum order: 10";
      Snack Combos 1–5 show "Minimum order: 5"; the platters show no
      minimum line.
- [ ] Adding Meat Pie puts 10 in the cart; the cart's − at 10 removes
      the line; + goes to 11.
- [ ] `curl -X POST /api/orders` with 9 meat pies → `400`,
      `items[0].quantity` `below_minimum`; with 10 → `201`.
- [ ] Existing items still add at 1 and order as before.
- [ ] Admin menu item form shows "Minimum order"; 0 and 1000 are
      rejected inline; changing it to 3 on a published item goes through
      Changes → Publish and the menu then shows "Minimum order: 3".
- [ ] Re-running the data migration SQL by hand inserts no rows
      (`INSERT 0 0` for every statement).

### Rentals

- [ ] `GET /api/rentals` returns the seven rentals of Decision 12 in
      order.
- [ ] `/en/rentals`, `/nl/rentals`, `/fr/rentals` show heading, intro,
      seven cards with prices per event ("€15 per event" /
      "€15 per evenement" / "€15 par événement"), "Reserve on WhatsApp"
      buttons, and the
      booking note; each button opens
      `https://wa.me/31630545277?text=…` naming that rental.
- [ ] The header shows "Rentals" (Verhuur / Location) on desktop and in
      the mobile menu; the Services page's Event Rentals section links
      to `/rentals`.
- [ ] `/sitemap.xml` lists the rentals page in all three locales.
- [ ] Admin `/rentals`: create a rental with a photo, edit its price,
      toggle it unavailable (it disappears from the public page), set it
      available again, archive it (gone from admin list and public page).
- [ ] Saving an empty name, a negative price or a foreign image URL
      shows inline errors; saving a stale form shows the conflict
      message.
- [ ] Unauthenticated `GET/POST /api/admin/rentals` → `401`.

## Definition of done

Every email the site sends declares UTF-8 and renders €, dashes,
apostrophes and emoji correctly; the menu has a Snacks category with
the client's 13 snack items and per-item minimum orders that the cart,
admin and API all enforce; the owner manages rentals in admin, and
customers see them on a translated `/rentals` page linked from the
header and Services, reserving by WhatsApp — all with passing tests,
`pnpm build` and `/validate` on staging.
