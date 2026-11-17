# Phase 30 — Add-to-Cart Feedback: Implementation Plan

**Ordering:** web only, no API change. Groups 1–4 in order. Run
`/validate` after Group 4.

Paths refer to `2026-11-16-phase-27-admin-pwa-push` at `ccff0e7`.

## Group 0 — Preconditions

0.1. Phase 27 is merged (PR #50). Done.

## Group 1 — Mobile cart icon (web)

requirement.md Decision 1.

1.1. In `components/site-header.tsx`, extract the cart link into a small
     `HeaderCartLink({ className })` inside the same file, so both copies
     stay identical:
     ```tsx
     <Link href="/cart" aria-label={t("cart")} data-cart-target
           className={`group/liquid relative isolate inline-flex h-10 w-10 items-center justify-center rounded-full text-primary hover:bg-background-soft ${className}`}>
       <CartIcon className="h-5 w-5" />
       <CartBadge />
       <LiquidSurface pathClassName={GHOST_LIQUID_PATH} splashClassName="fill-primary" />
     </Link>
     ```
1.2. Render one copy before the hamburger button, wrapped with the
     button in a `flex items-center gap-2` group, with
     `className="min-[769px]:hidden"`. Keep the copy in the nav list, but
     hide its `<li>` below 769px (`max-[768px]:hidden`), so a phone's
     open menu doesn't repeat it.
1.3. Check the header at 320px: logo, cart and menu fit on one row.

## Group 2 — Feedback provider: toast, fly, badge (web)

Decisions 2, 4–8.

2.1. **`lib/cart-feedback.tsx`** (client):
     ```tsx
     type AddAnnouncement = {
       name: string;
       optionLabel: string | null;
       imageUrl: string | null;
       origin: DOMRect;           // the button that was clicked
     };
     type CartFeedbackValue = { announceAdd: (a: AddAnnouncement) => void };
     export function CartFeedbackProvider({ children }: { children: ReactNode }): JSX.Element;
     export function useCartFeedback(): CartFeedbackValue; // no-op outside the provider
     ```
     The provider renders `{children}`, `<AddToast …/>` and a fly layer
     `<div aria-hidden className="pointer-events-none fixed inset-0 z-[60]">`.
     `announceAdd` sets the toast state (with an incrementing `id`, so the
     same item twice still restarts it) and calls `flyToCart(origin,
     imageUrl)` unless reduced motion is on.
2.2. **`flyToCart`** (same file):
     - Find the visible target:
       `[...document.querySelectorAll<HTMLElement>("[data-cart-target]")].find((el) => el.getClientRects().length > 0)`.
       If there's none, return.
     - Create a 48px round element at the origin's centre (`<img>` with
       `object-cover` when `imageUrl` is set, else a `bg-primary` div) in
       the fly layer.
     - Run a GSAP timeline: `x` to the target centre over 0.65 s with
       `power1.out`, `y` over 0.65 s with `power2.in` (in parallel, so it
       curves), then `scale: 1/3` and `opacity: 0` over the last 0.2 s.
       `onComplete` removes the element and dispatches
       `window.dispatchEvent(new CustomEvent("zt:cart-landed"))`.
     - Clean up any flies still running when the provider unmounts.
2.3. **`components/add-toast.tsx`** (client):
     - An always-rendered `<div role="status" aria-live="polite"
       className="sr-only">` holding the plain-text message, so screen
       readers hear each add.
     - The visible card: `fixed z-[60]`. Below 769px it's
       `bottom-[calc(1.5rem+env(safe-area-inset-bottom))]
       left-1/2 -translate-x-1/2 w-[min(22rem,calc(100vw-7rem))]`. From
       769px up it's `min-[769px]:top-20 min-[769px]:right-6
       min-[769px]:bottom-auto min-[769px]:left-auto
       min-[769px]:translate-x-0 min-[769px]:w-80`.
     - Contents: a small round photo (if any), the message via
       `t.rich("added" | "addedOption", { name, option, strong: (c) =>
       <strong>{c}</strong> })`, a **View cart** `Link` (`/cart`), and a ×
       close button (`aria-label={t("dismiss")}`).
     - Show and hide with GSAP (`y: 16 → 0`, `opacity`, 0.25 s
       `outExpo`), or instantly with reduced motion.
     - The 3.5 s auto-hide timer pauses on `pointerenter`/`focusin` and
       resumes on `pointerleave`/`focusout`. A new `id` restarts it. The
       card is `inert` while hidden.
2.4. **Badge bounce** in `components/cart-badge.tsx`:
     - Keep the previous `itemCount` in a ref, and skip until the cart is
       hydrated. Expose `isHydrated` from `useCart()` if it isn't already.
     - When the count goes up, bounce with GSAP
       (`scale: 1.35, yoyo, repeat: 1, ease: "back.out(3)", duration: 0.2`).
       If a `zt:cart-landed` event arrives within 900 ms, bounce on it
       instead. Otherwise bounce at once (no fly, e.g. reduced motion or
       no visible target). Skip it with reduced motion.
     - The badge only mounts once the count is above 0, so the first add
       bounces it from mount (`gsap.from(scale: 0)`).
2.5. **Layout:** in `app/[locale]/layout.tsx`, wrap the existing
     `CartProvider` children in `<CartFeedbackProvider>`.

## Group 3 — Button splash and ✓ (web)

Decision 3.

3.1. In `components/add-to-cart-controls.tsx`:
     - Add `const anchorRef = useRef<HTMLButtonElement>(null)` on the
       tile's main "+" button, and `const [justAdded, setJustAdded] =
       useState<number | null>(null)` (the price option id, or `0` for a
       flat price).
     - A `confirm(option)` helper calls `addOption`, then
       `announceAdd({ name: item.name, optionLabel: option?.label ?? null,
       imageUrl: item.image_url, origin: anchorRef.current!.getBoundingClientRect() })`,
       sets `justAdded`, and (re)starts a 1.2 s timeout that clears it.
       Clear it on unmount.
     - `AddGlyph` takes a `checked` prop: it renders "✓" instead of "+"
       (both `aria-hidden`), with a quick scale-in (CSS
       `animate-[pop_200ms_ease-out]`).
     - The ring: a sibling `<span aria-hidden className="pointer-events-none
       absolute inset-0 -z-10 rounded-full border-2 border-primary
       motion-safe:animate-[ring_450ms_ease-out_forwards]">`, re-keyed
       per add (`key={addCount}`) so it replays. It isn't rendered with
       reduced motion.
3.2. In `app/globals.css`, add `@keyframes pop` (scale 0.4 → 1) and
     `@keyframes ring` (scale 1 → 2.2, opacity 0.6 → 0). Use the `onBrand`
     colour (`border-white`) on brand tiles.

## Group 4 — Messages and checks (web)

Decision 8.

4.1. Add a `CartFeedback` namespace to `messages/{en,nl,fr}.json`:

     | key | en | nl | fr |
     |---|---|---|---|
     | `added` | `<strong>{name}</strong> added to your order` | `<strong>{name}</strong> toegevoegd aan je bestelling` | `<strong>{name}</strong> ajouté à votre commande` |
     | `addedOption` | `<strong>{name} ({option})</strong> added to your order` | `<strong>{name} ({option})</strong> toegevoegd aan je bestelling` | `<strong>{name} ({option})</strong> ajouté à votre commande` |
     | `viewCart` | `View cart` | `Bekijk winkelwagen` | `Voir le panier` |
     | `dismiss` | `Close` | `Sluiten` | `Fermer` |

     Match the existing nl/fr terms for "cart" and "order" in the message
     files (`SiteHeader.cart`, `CartPage`). If they differ from this
     table, the files win.
4.2. Run `pnpm check:messages`, `pnpm lint` and `pnpm build`, then
     `/validate`.
4.3. Run validation.md's manual checks in the Phase 25 device simulator
     (Mobile S through Desktop XL) and on a real phone.

## Implementation notes

2026-11-17, Groups 1–4:

- **The ✓ for an option shows on the tile's "+"**, not on the option's
  button inside the dialog. The dialog closes right after the add, so a
  ✓ there would never be seen. The ring and the fly start there too, as
  planned.
- **The badge bounces on events, not on count changes.**
  `CartFeedbackProvider` dispatches `zt:cart-added` (`{ flying }`) for
  each add, and `zt:cart-landed` when the photo arrives. `CartBadge`
  bounces on the landing, after 900 ms at most, or at once when nothing
  flies. Loading a saved cart or editing on `/cart` never fires these, so
  `useCart()` didn't need an `isHydrated` flag (2.4).
- The keyframes are Tailwind theme tokens (`--animate-add-pop`,
  `--animate-add-ring` → `animate-add-pop` / `animate-add-ring`), like
  the existing `--animate-marquee`.
- The toast's hover/focus pause is tied to the toast's id, so closing it
  while hovering never leaves the next toast paused.
