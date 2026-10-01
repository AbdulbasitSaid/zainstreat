# Mission

## Statement

**Keep ordering simple and make managing the business simple.**

Zain's Treat n More needs a digital presence that lets customers discover the
business, browse the menu, and place an order in minutes — and lets the
owner/staff run that menu and those orders without needing a developer. The
website is the business's storefront, order desk, and lightweight back office
in one place.

A customer should be able to go from seeing a meal to submitting an order
with minimal friction. An administrator should be able to add or remove a
menu item in minutes.

## Who It Serves

- **Individual customers** — "I want to see what's available and place an
  order quickly."
- **Event customers** — people organizing a wedding, birthday, corporate
  event, family gathering, or other occasion who need catering and possibly
  event rentals. "I need catering and possibly event services for my
  occasion."
- **Administrators** — the owner or staff managing orders and the menu day
  to day.

## Business Goals

1. Establish a professional online presence.
2. Increase customer enquiries and food orders.
3. Showcase meals, snacks, catering, and event services.
4. Make the menu easily accessible.
5. Reduce repetitive menu enquiries.
6. Provide a simple order-management system.
7. Let staff manage menu items without developer assistance.
8. Communicate halal, hygiene, and food-safety information.
9. Make WhatsApp an easy, low-friction customer-contact channel.

## Customer Goals

1. Understand what the business offers.
2. Find food quickly, with clear descriptions and prices.
3. Add items to an order and submit it easily.
4. Contact the business for catering or event information.
5. Understand food and allergen information before ordering.

## Non-Goals (MVP)

The MVP explicitly does **not** need:

- Enterprise ERP, payroll, or accounting.
- Employee management or complex CRM.
- Multi-vendor marketplace or advanced inventory management.
- Loyalty/rewards system.
- Complex delivery logistics.
- AI chatbot.

These may be reconsidered after the MVP ships — see `roadmap.md` for what's
deliberately out of scope for now.

## Non-Negotiable Rules

A few rules shape every feature decision because they protect trust and
historical accuracy, not just UX:

- Only available menu items can be ordered; archived/unavailable items must
  not be orderable.
- Historical orders must retain the item name and price **as they were at
  the time of purchase**, even if the menu changes later.
- Catering/event enquiries are a quote workflow, separate from the food
  cart — a two-meal order should never require a large catering form.

## North Star

> Discover → Browse → Order → Manage (for food)
> Discover → Enquire → Quote → Confirm (for catering/events)

The site should feel like a real food business — warm, trustworthy,
food-focused, mobile-first — not a generic software product.
