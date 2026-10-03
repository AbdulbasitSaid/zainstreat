# Zain’s Treat n More — Website & Online Ordering Platform

> **Great Food. Memorable Moments. Exceptional Service.**

Zain’s Treat n More is a food and catering business providing freshly prepared halal meals, delicious snacks, professional catering services, and event rentals. The website will serve as the business's digital presence, online menu, food ordering platform, catering enquiry channel, and lightweight administration system.

---

## 1. Product Overview

The product is a **marketing website + online food ordering system + mini admin dashboard**.

Customers should be able to:

- Learn about Zain’s Treat n More.
- Browse meals and snacks.
- View catering and event-rental services.
- Browse the menu and prices.
- Add food to a cart.
- Place food orders.
- Contact the business.
- Request catering/event information.
- Review terms and conditions.
- Review food regulations, hygiene, halal, and allergen information.
- Contact the business through WhatsApp.

Administrators should be able to:

- Log into a protected dashboard.
- View incoming orders.
- Update order status.
- Add menu items.
- Edit menu items.
- Archive/remove menu items.
- Mark menu items available/unavailable.
- Manage categories.
- View basic order/customer information.

---

# 2. Brand

## Business Name

**Zain’s Treat n More**

## Tagline

**Great Food. Memorable Moments. Exceptional Service.**

## Brand Message

> At Zain’s Treat n More, we believe every occasion deserves great food and exceptional service. From intimate indoor gatherings to lively outdoor celebrations, we provide freshly prepared meals, delicious snacks, professional catering services, and event rentals to make hosting effortless and every moment memorable.

> Our meals are 100% halal, hygienically prepared, and made with fresh, carefully selected ingredients. Whether you’re celebrating a birthday, wedding, corporate event, family gathering, or another special occasion, we’re committed to serving quality food with care and dependable service.

> **You celebrate. We take care of the treats and more.**

## Existing Logo

The website should use the supplied Zain’s Treat n More logo as the primary brand asset.

The logo contains:

- Deep burgundy/red typography
- Crimson/red accents
- Bright pink highlights
- White background
- Chef hat
- Rolling pin
- Circular emblem
- “Indoor & Outdoor Catering • Meal Delivery”
- WhatsApp contact information

The supplied logo should not be recreated unnecessarily. Use the original high-quality asset wherever possible.

---

# 3. Brand Color Palette

The website palette is derived from the supplied logo.

## Primary Palette

| Token | Color | Hex | Usage |
|---|---|---|---|
| `primary-dark` | Deep Burgundy | `#4E0C2A` | Dark sections, footer, headings |
| `primary` | Burgundy | `#72123C` | Primary buttons, links, key UI |
| `primary-light` | Light Burgundy | `#8A1E4E` | Secondary CTA, highlights |
| `accent` | Dusty Pink | `#C66C84` | Small accents, decorative elements |
| `accent-light` | Blush Pink | `#F0AEC0` | Borders, subtle accents |
| `background-soft` | Very Light Pink | `#FAE8EA` | Section backgrounds |
| `white` | White | `#FFFFFF` | Main background, cards |
| `text` | Dark Text | `#2B1114` | Body text |
| `text-muted` | Muted Brown/Red | `#55343A` | Secondary text |
| `whatsapp` | WhatsApp Green | `#1DA332` | WhatsApp actions |

### CSS Variables

```css
:root {
  /* Brand */
  --color-primary: #72123C;
  --color-primary-dark: #4E0C2A;
  --color-primary-light: #8A1E4E;

  /* Accent */
  --color-accent: #C66C84;
  --color-accent-light: #F0AEC0;

  /* Background */
  --color-background: #FFFFFF;
  --color-background-soft: #FAE8EA;
  --color-background-dark: #4E0C2A;

  /* Text */
  --color-text: #2B1114;
  --color-text-muted: #55343A;
  --color-text-light: #FFFFFF;

  /* Functional */
  --color-whatsapp: #1DA332;
}
```

## Color Usage Rules

The website should primarily use:

- White for the main background.
- Deep burgundy and crimson for brand identity.
- Pink as an accent rather than the dominant color.
- Soft pink for occasional section backgrounds.
- WhatsApp green only for WhatsApp-related actions.

Do not make the entire website red or pink.

Food photography should provide much of the visual richness.

---

# 4. Product Vision

> **Make it easy for customers to discover, order, and enjoy great halal food while making it simple for Zain’s Treat n More to manage orders and menu items.**

The website should feel:

- Warm
- Trustworthy
- Professional
- Food-focused
- Family-friendly
- Celebratory
- Easy to use
- Mobile-first

---

# 5. Goals

## Business Goals

1. Establish a professional online presence.
2. Increase customer enquiries and food orders.
3. Showcase meals, snacks, catering, and event services.
4. Make the menu easily accessible.
5. Reduce repetitive menu enquiries.
6. Provide a simple order-management system.
7. Allow staff to manage menu items without developer assistance.
8. Communicate halal, hygiene, and food-safety information.
9. Make WhatsApp an easy customer-contact channel.

## Customer Goals

Customers should be able to:

1. Understand what the business offers.
2. Find food quickly.
3. See food descriptions and prices.
4. Add items to an order.
5. Submit an order easily.
6. Contact the business for catering.
7. Request information for events.
8. Understand food and allergen information.

---

# 6. Non-Goals

The MVP does not need:

- Enterprise ERP.
- Payroll.
- Accounting.
- Employee management.
- Complex CRM.
- Multi-vendor marketplace.
- Advanced inventory management.
- Loyalty/rewards system.
- Complex delivery logistics.
- AI chatbot.

These can be considered later.

---

# 7. Target Users

## 7.1 Individual Customer

A customer who wants to purchase meals or snacks.

**Primary need:**

> “I want to see what's available and place an order quickly.”

## 7.2 Event Customer

A customer organizing:

- Birthday
- Wedding
- Corporate event
- Family gathering
- Outdoor celebration
- Private party
- Community/religious event

**Primary need:**

> “I need catering and possibly event services for my occasion.”

## 7.3 Administrator

The owner or staff member managing the website.

**Primary needs:**

- See new orders.
- Update order status.
- Add food.
- Change prices.
- Archive food.
- Mark food unavailable.

---

# 8. Website Information Architecture

```text
Zain’s Treat n More
│
├── Home
│
├── About
│
├── Services
│   ├── Meals
│   ├── Snacks
│   ├── Catering
│   └── Event Rentals
│
├── Menu
│   ├── Meals
│   ├── Snacks
│   ├── Drinks
│   ├── Desserts
│   └── Specials
│
├── Order
│
├── Contact
│
├── Terms & Conditions
│
├── Food Regulations
│
└── Admin
    ├── Login
    ├── Dashboard
    ├── Orders
    ├── Menu
    ├── Categories
    └── Settings
```

---

# 9. Homepage

## 9.1 Header / Navigation

Navigation:

```text
Home
About
Services
Menu
Contact
[Order Now]
```

On mobile:

- Hamburger menu.
- Persistent Order CTA.
- WhatsApp CTA where appropriate.

---

## 9.2 Hero

### Headline

> **Great Food. Memorable Moments. Exceptional Service.**

### Supporting Copy

> Fresh halal meals, delicious snacks, professional catering, and event services made for your special moments.

### CTAs

Primary:

**Order Now**

Secondary:

**Book Catering**

Optional:

**WhatsApp Us**

Hero should use high-quality food photography rather than a heavily graphical background.

---

# 10. Homepage Sections

Recommended order:

```text
Hero
↓
Quick Trust Indicators
↓
Featured Menu
↓
Services
↓
Why Choose Us
↓
Catering / Events CTA
↓
About Preview
↓
WhatsApp CTA
↓
Footer
```

## Trust Indicators

Use four simple indicators:

- **100% Halal**
- **Fresh Ingredients**
- **Hygienically Prepared**
- **Reliable Service**

These should be presented as factual business claims and kept consistent with the actual business practices.

---

# 11. About Page

## Sections

### Who We Are

Introduce Zain’s Treat n More.

### Our Mission

> To provide delicious, hygienically prepared halal food and dependable catering services while helping customers create memorable occasions.

### Our Values

- Quality
- Hygiene
- Customer satisfaction
- Reliability
- Hospitality
- Halal food standards

### What We Offer

- Meals
- Snacks
- Catering
- Event rentals
- Meal delivery

---

# 12. Services Page

The Services page should provide more detail than the homepage.

## 12.1 Meals

Freshly prepared meals for individuals, families, and groups.

## 12.2 Snacks

Delicious snacks for everyday enjoyment and special occasions.

## 12.3 Catering

Catering for:

- Weddings
- Birthdays
- Corporate events
- Family gatherings
- Outdoor celebrations
- Community/religious events

CTA:

**Request Catering Quote**

## 12.4 Event Rentals

Event-related rental services that help customers organize memorable occasions.

Each service can contain:

```text
Service Name
Description
Image
What's Included
Optional Pricing
[Request a Quote]
```

---

# 13. Menu Page

The Menu is one of the most important parts of the website.

## Categories

Initial categories may include:

```text
Meals
Snacks
Drinks
Desserts
Specials
Event Packages
```

Categories must be manageable from the admin dashboard.

## Menu Item

Each menu item should support:

```text
Name
Description
Image
Category
Price
Availability
Featured
```

Example:

```text
Jollof Rice & Chicken

Classic Nigerian jollof rice served with
seasoned chicken.

₦5,000

[Add to Order]
```

---

# 14. Food Ordering

Food ordering and event/catering enquiries should be separate workflows.

```text
                         ZAIN'S TREAT N MORE
                                  │
                  ┌───────────────┴───────────────┐
                  │                               │
             FOOD ORDERING                   EVENT SERVICES
                  │                               │
             Browse Menu                    Catering
                  │                         Event Rental
               Cart                              │
                  │                         Quote Request
              Customer                           │
              Details                         Enquiry
                  │
             Order Submit
```

A customer ordering two meals should not need to complete a large catering enquiry form.

---

# 15. Customer Ordering Flow

```text
Menu
 ↓
Select Item
 ↓
Add to Cart
 ↓
Review Cart
 ↓
Customer Details
 ↓
Order Review
 ↓
Submit Order
 ↓
Order Confirmation
```

---

# 16. Cart

The cart should allow customers to:

- View items.
- Increase quantity.
- Decrease quantity.
- Remove items.
- See subtotal.
- Continue shopping.
- Proceed to order.

Example:

| Item | Quantity | Unit Price | Total |
|---|---:|---:|---:|
| Jollof Rice & Chicken | 2 | ₦5,000 | ₦10,000 |
| Snack Box | 1 | ₦3,000 | ₦3,000 |

```text
Subtotal: ₦13,000

[Continue Shopping]
[Proceed to Order]
```

Future support may include:

- Delivery fee
- Service fee
- Discount
- Tax

---

# 17. Customer Order Information

## Required

- Full name
- Phone number
- Email address
- Order items
- Quantity

## Optional / Conditional

- Delivery address
- Order notes
- Preferred pickup/delivery time
- Special instructions

If delivery is supported, clearly distinguish:

```text
( ) Pickup

( ) Delivery
```

---

# 18. Order Confirmation

After successful submission:

> **Thank you for your order!**

Example:

```text
Order #ZT-000123

Thank you for your order.

We've received your order and will contact you
to confirm the details.

Order Total: ₦14,000
```

The system should send an order confirmation through the configured communication channel, such as email or WhatsApp where supported.

---

# 19. Order Status

Recommended internal statuses:

```text
New
↓
Confirmed
↓
Preparing
↓
Ready
↓
Completed
```

Alternative:

```text
Cancelled
```

Customer-facing status can eventually become:

```text
Order Received
↓
Confirmed
↓
Preparing
↓
Ready / Out for Delivery
↓
Completed
```

---

# 20. Catering / Event Enquiry

Catering should have a separate enquiry form.

Suggested fields:

```text
Name
Phone
Email
Event Type
Event Date
Number of Guests
Location
Services Required
Message
```

Event types:

- Wedding
- Birthday
- Corporate
- Family Gathering
- Outdoor Event
- Other

Services required:

- Catering
- Meal Delivery
- Event Rental
- Other

CTA:

**Request a Quote**

---

# 21. WhatsApp Integration

WhatsApp should be a prominent customer-contact channel because it is part of the business's existing branding.

## WhatsApp CTA

Examples:

- WhatsApp Us
- Order via WhatsApp
- Ask About Catering
- Contact Us

Use WhatsApp green:

```text
#1DA332
```

WhatsApp should not replace the online ordering system.

The website should support two customer paths:

```text
Quick enquiry / conversation
        ↓
     WhatsApp


Structured food order
        ↓
    Online Order


Large event / catering
        ↓
   Request a Quote
```

---

# 22. Contact Page

The Contact page should provide:

- Phone number
- Email
- WhatsApp
- Business address
- Opening hours
- Social media links
- Contact form
- Map/location

## Contact Form

```text
Name
Email
Phone
Subject
Message

[Send Message]
```

Possible subjects:

- General Enquiry
- Catering
- Event Rental
- Menu Enquiry
- Order Enquiry
- Other

---

# 23. Terms & Conditions

Create a dedicated Terms & Conditions page.

It should cover:

- Ordering
- Payment
- Cancellation
- Refunds
- Delivery/pickup
- Catering orders
- Event bookings
- Customer responsibilities
- Website usage
- Changes to services/prices

The final legal wording should be reviewed by an appropriate legal professional.

---

# 24. Food Regulations Page

This page should communicate food-safety and customer information.

## Sections

### Halal

Information about the business's halal food commitment.

### Food Hygiene

Information about hygienic food preparation and handling.

### Ingredients

Information about ingredient sourcing and preparation.

### Allergens

Customers should be informed about potential allergens in individual menu items.

Example:

> Please inform us about any allergies or dietary requirements before placing your order.

Potential allergens may include:

- Milk
- Eggs
- Wheat/gluten
- Nuts
- Soy

The actual allergen information must reflect the ingredients and preparation processes used by the business.

---

# 25. Admin Dashboard

The dashboard should intentionally remain simple.

## Dashboard

Example:

```text
┌───────────────────────────────────────────────┐
│ Zain's Treat n More                           │
├───────────────┬───────────────────────────────┤
│ Dashboard     │ Today's Orders       12       │
│ Orders        │                               │
│ Menu          │ Pending Orders       5        │
│ Categories    │                               │
│ Settings      │ Menu Items           42       │
└───────────────┴───────────────────────────────┘
```

---

# 26. Admin Authentication

The dashboard must be protected.

MVP:

- Email/username
- Password
- Secure session
- Logout

Future:

- Multiple staff accounts
- Role-based permissions
- Password reset
- Two-factor authentication

---

# 27. Order Management

Admin should see:

```text
Order ID
Customer
Phone
Date
Items
Total
Status
```

Example:

| Order | Customer | Total | Status |
|---|---|---:|---|
| #ZT-001 | Customer A | ₦15,000 | New |
| #ZT-002 | Customer B | ₦8,500 | Preparing |
| #ZT-003 | Customer C | ₦25,000 | Completed |

Clicking an order opens full details.

Admin should be able to:

- View customer information.
- View ordered items.
- View quantities.
- View prices.
- View total.
- View delivery/pickup information.
- View notes.
- Change status.

---

# 28. Menu Management

This is a core admin feature.

## Add Item

Fields:

```text
Name
Description
Price
Category
Image
Available?
Featured?
```

Action:

**Add Item**

## Edit Item

Admin can change:

- Name
- Description
- Price
- Category
- Image
- Availability
- Featured status

## Remove Item

Use **Archive Item** instead of hard deletion.

Example:

```text
Jollof Rice
Status: Archived
```

Archived items should not appear on the active customer menu but should remain available for historical orders.

---

# 29. Menu Availability

Admins should be able to quickly mark an item unavailable.

Example:

```text
Jollof Rice
₦5,000

● Available

[Mark Unavailable]
```

When unavailable:

```text
Jollof Rice
₦5,000

Currently unavailable

[Mark Available]
```

Unavailable items should not be orderable.

---

# 30. Category Management

Admins should be able to:

- Add category.
- Rename category.
- Archive category.
- Reorder categories if needed.

Example:

```text
Meals
Snacks
Drinks
Desserts
Specials
```

A category should not be permanently deleted if it contains historical menu items.

---

# 31. Data Model

A simple relational database model:

```text
users
-----
id
name
email
password_hash
role
created_at
updated_at


categories
----------
id
name
description
created_at
updated_at
deleted_at


menu_items
----------
id
category_id
name
description
price
image_url
is_available
is_featured
created_at
updated_at
deleted_at


orders
------
id
customer_name
customer_email
customer_phone
delivery_type
delivery_address
notes
subtotal
delivery_fee
total
status
created_at
updated_at


order_items
-----------
id
order_id
menu_item_id
item_name
unit_price
quantity
subtotal
```

## Important Order Data Rule

`order_items` should store the item's name and price at the time the order was placed.

Example:

```text
Current menu:
Jollof Rice = ₦6,000

Historical order:
Jollof Rice = ₦5,000
```

Changing the menu price must not change historical orders.

---

# 32. Functional Requirements

## FR-001 — Browse Menu

Customers must be able to browse available menu items.

## FR-002 — Menu Categories

The system must organize menu items by category.

## FR-003 — Add Menu Item

Administrators must be able to create menu items.

## FR-004 — Edit Menu Item

Administrators must be able to modify menu items.

## FR-005 — Archive Menu Item

Administrators must be able to archive menu items.

## FR-006 — Item Availability

Administrators must be able to mark items available/unavailable.

## FR-007 — Cart

Customers must be able to add, remove, and change quantities.

## FR-008 — Place Order

Customers must be able to submit orders with contact information.

## FR-009 — Order Management

Administrators must be able to view orders.

## FR-010 — Order Status

Administrators must be able to update order status.

## FR-011 — Catering Enquiry

Customers must be able to request catering/event information.

## FR-012 — Contact

Customers must be able to submit general enquiries.

## FR-013 — WhatsApp

Customers must be able to contact the business through WhatsApp.

## FR-014 — Responsive Design

The website must work on:

- Mobile
- Tablet
- Desktop

The design should be mobile-first.

---

# 33. Non-Functional Requirements

## Performance

- Optimize food images.
- Use modern image formats where possible.
- Lazy-load images where appropriate.
- Keep initial page load fast.
- Avoid unnecessary JavaScript.

## Security

The system must:

- Protect admin routes.
- Hash passwords.
- Validate user input.
- Sanitize/validate API requests.
- Prevent unauthorized order manipulation.
- Use HTTPS.
- Apply appropriate database access controls.

## Accessibility

Consider:

- Keyboard navigation.
- Readable typography.
- Sufficient contrast.
- Form labels.
- Alt text.
- Screen-reader compatibility.
- Visible focus states.

## SEO

Public pages should include:

- Page title.
- Meta description.
- Open Graph metadata.
- Semantic HTML.
- Search-engine-friendly URLs.
- Structured content.

Important URLs:

```text
/
/about
/services
/menu
/order
/contact
/terms
/food-regulations
```

---

# 34. UX / Visual Design Direction

The logo suggests a visual direction that is:

**Warm + Bold + Elegant + Food-focused + Celebratory**

Avoid making the website:

- Overly corporate.
- Completely minimalist.
- Predominantly pink.
- Visually cluttered.
- Heavy with gradients.

## Recommended Visual Language

### Typography

Use a combination of:

- Strong display typography for major headings.
- Clean sans-serif typography for body copy.
- Optional elegant/script accent typography for small decorative brand moments.

The website should prioritize readability over reproducing the logo's exact script style.

### Shapes

Use:

- Rounded cards.
- Soft corners.
- Circular/curved decorative elements.
- Subtle borders.

The circular logo and rolling-pin motif can inspire decorative elements without repeating the logo everywhere.

### Photography

Food photography should be a major visual element.

Prioritize:

- Fresh food.
- Close-up dishes.
- Catering tables.
- Event setups.
- Happy social occasions.
- Clean preparation environments where appropriate.

Avoid relying entirely on stock photography if real business photography is available.

---

# 35. Recommended Homepage Layout

```text
┌─────────────────────────────────────────────────────┐
│ LOGO          Home About Services Menu   [Order]   │
├─────────────────────────────────────────────────────┤
│                                                     │
│        GREAT FOOD. MEMORABLE MOMENTS.              │
│        EXCEPTIONAL SERVICE.                         │
│                                                     │
│  Fresh halal meals, catering, snacks & events.     │
│                                                     │
│       [ ORDER NOW ]  [ BOOK CATERING ]             │
│                                                     │
│                       FOOD PHOTO                    │
├─────────────────────────────────────────────────────┤
│                                                     │
│       100%       FRESH       HYGIENIC      RELIABLE │
│       HALAL      INGREDIENTS                         │
│                                                     │
├─────────────────────────────────────────────────────┤
│                 FEATURED MENU                       │
│                                                     │
│   [Food]       [Food]       [Food]       [Food]    │
│                                                     │
│                 [VIEW MENU]                         │
├─────────────────────────────────────────────────────┤
│                   OUR SERVICES                      │
│                                                     │
│  Meals    Snacks    Catering    Event Rentals      │
│                                                     │
├─────────────────────────────────────────────────────┤
│                WHY CHOOSE US?                       │
├─────────────────────────────────────────────────────┤
│                                                     │
│             PLAN YOUR NEXT EVENT                   │
│                                                     │
│      [ REQUEST CATERING QUOTE ]                     │
│                                                     │
├─────────────────────────────────────────────────────┤
│                 WHATSAPP US                         │
├─────────────────────────────────────────────────────┤
│ Footer                                              │
└─────────────────────────────────────────────────────┘
```

---

# 36. Primary Customer Journeys

## Journey A — Food Order

```text
Homepage
   ↓
Menu
   ↓
Select Food
   ↓
Add to Cart
   ↓
Cart
   ↓
Customer Details
   ↓
Review
   ↓
Submit
   ↓
Confirmation
   ↓
Admin Receives Order
```

## Journey B — Catering

```text
Homepage
   ↓
Services
   ↓
Catering
   ↓
Request Quote
   ↓
Submit Event Details
   ↓
Business Receives Enquiry
   ↓
Business Contacts Customer
```

## Journey C — WhatsApp

```text
Homepage / Contact
       ↓
   WhatsApp Us
       ↓
WhatsApp Conversation
```

## Journey D — Admin Menu Update

```text
Admin Login
    ↓
Dashboard
    ↓
Menu
    ↓
Add / Edit / Archive
    ↓
Save
    ↓
Customer Menu Updated
```

---

# 37. User Stories

## Customer

> **As a customer, I want to browse the available menu so that I can decide what to order.**

> **As a customer, I want to add food items to a cart so that I can place multiple items in one order.**

> **As a customer, I want to submit my contact details with my order so that Zain’s Treat n More can contact me about my order.**

> **As a customer, I want to learn about catering services so that I can decide whether the business is suitable for my event.**

> **As a customer, I want to request a catering quote so that I can discuss my event requirements with the business.**

> **As a customer, I want to contact Zain’s Treat n More through WhatsApp so that I can quickly ask questions or make an enquiry.**

> **As a customer, I want to understand the business's food and allergen information so that I can make an informed decision.**

## Administrator

> **As an administrator, I want to view incoming orders so that I can process customer requests.**

> **As an administrator, I want to update an order's status so that I can track its progress.**

> **As an administrator, I want to add menu items so that new products can become available to customers.**

> **As an administrator, I want to edit menu items so that food information and prices remain accurate.**

> **As an administrator, I want to archive menu items so that unavailable products no longer appear on the public menu while historical orders remain intact.**

> **As an administrator, I want to mark menu items unavailable so that customers cannot order food that is temporarily unavailable.**

> **As an administrator, I want to manage menu categories so that the menu remains organized.**

---

# 38. MVP Scope

## Public Website

- [ ] Home
- [ ] About
- [ ] Services
- [ ] Menu
- [ ] Order
- [ ] Contact
- [ ] Terms & Conditions
- [ ] Food Regulations

## Ordering

- [ ] Menu browsing
- [ ] Category filtering
- [ ] Cart
- [ ] Quantity management
- [ ] Customer details
- [ ] Order submission
- [ ] Order confirmation

## Catering

- [ ] Catering information
- [ ] Event rental information
- [ ] Quote request form

## Communication

- [ ] WhatsApp CTA
- [ ] Contact form
- [ ] Email notification where configured

## Admin

- [ ] Admin login
- [ ] Dashboard
- [ ] Order list
- [ ] Order details
- [ ] Order status
- [ ] Add menu item
- [ ] Edit menu item
- [ ] Archive menu item
- [ ] Availability toggle
- [ ] Category management

---

# 39. Future Features

## Phase 2

- Online payments.
- WhatsApp order notifications.
- Email notifications.
- Customer order tracking.
- Delivery fee calculation.
- Event booking calendar.
- Discount codes.
- Catering packages.

## Phase 3

- Customer accounts.
- Order history.
- Loyalty program.
- Multiple administrators.
- Staff roles.
- Inventory management.
- Sales reports.
- Revenue dashboard.
- Popular-item analytics.

## Phase 4

A broader food-business operations platform:

```text
Customer
   ↓
Website
   ↓
Order Management
   ↓
Kitchen
   ↓
Pickup / Delivery
   ↓
Customer
```

with notifications and operational workflows integrated throughout.

---

# 40. Suggested Technical Architecture

A simple architecture is preferred for the MVP.

```text
                 ┌────────────────────┐
                 │    Customer Web    │
                 │   Responsive UI    │
                 └─────────┬──────────┘
                           │
                         HTTPS
                           │
                           ▼
                 ┌────────────────────┐
                 │       API          │
                 │ Business Logic     │
                 └─────────┬──────────┘
                           │
              ┌────────────┼────────────┐
              ▼            ▼            ▼
         PostgreSQL     Storage      Email /
                                     WhatsApp
```

A suitable implementation may use:

- Next.js/Astro for the public website if SEO-focused web delivery is desired.
- Rust/Axum or another backend for a custom API if a dedicated backend is required.
- PostgreSQL for persistent data.
- Object storage for menu/food images.
- Secure authentication for the admin dashboard.

The technology should remain secondary to the product requirements. Avoid unnecessary infrastructure for the first release.

---

# 41. Important Business Rules

## Menu

1. Only available menu items can be added to new orders.
2. Archived items should not appear in the active menu.
3. Existing orders must retain historical item information.
4. Price changes must not alter previous orders.

## Orders

1. Every order receives a unique order number.
2. Orders must have a status.
3. Orders should retain the customer information supplied at checkout.
4. Order items should preserve the price at the time of purchase.
5. Cancelled orders should remain in the system for record keeping.

## Catering

1. Catering is handled as an enquiry/quote workflow in the MVP.
2. Catering does not need to use the standard food cart.
3. Event requirements should be captured separately from normal food orders.

---

# 42. Error and Empty States

The application should handle common states.

## Empty Cart

> **Your cart is empty.**

> Explore our menu and find something delicious.

**[View Menu]**

## Unavailable Item

> **Currently unavailable**

Do not allow the customer to add it to the cart.

## Empty Menu Category

> **Nothing available in this category right now.**

## Order Error

> **We couldn't submit your order. Please try again or contact us on WhatsApp.**

## Successful Order

> **Order received!**

Show the order number and next steps.

---

# 43. Definition of Done

The MVP is complete when:

- [ ] Customers can access all public pages.
- [ ] Customers can browse the menu.
- [ ] Customers can filter menu categories.
- [ ] Customers can add/remove items from the cart.
- [ ] Customers can change quantities.
- [ ] Customers can submit an order.
- [ ] Orders are persisted successfully.
- [ ] Customers receive confirmation.
- [ ] Administrators can log in.
- [ ] Administrators can view orders.
- [ ] Administrators can update order status.
- [ ] Administrators can add menu items.
- [ ] Administrators can edit menu items.
- [ ] Administrators can archive menu items.
- [ ] Administrators can toggle availability.
- [ ] Administrators can manage categories.
- [ ] Catering enquiries can be submitted.
- [ ] Contact enquiries can be submitted.
- [ ] WhatsApp contact works.
- [ ] Archived/unavailable items are handled correctly.
- [ ] Previous orders retain their original item names and prices.
- [ ] Website works on mobile, tablet, and desktop.
- [ ] Forms have validation and error handling.
- [ ] Admin functionality is protected.
- [ ] Terms & Conditions are accessible.
- [ ] Food Regulations information is accessible.
- [ ] Website metadata/SEO basics are implemented.
- [ ] Images are optimized.
- [ ] Accessibility basics are implemented.

---

# 44. Recommended MVP Navigation

```text
HOME
ABOUT
SERVICES
MENU
CONTACT

                  [ ORDER NOW ]

Mobile:
☰ Menu
[Order]
[WhatsApp]
```

---

# 45. Final Product Structure

The complete product can be thought of as three connected systems:

```text
┌────────────────────────────────────────────────────┐
│                    PUBLIC WEBSITE                  │
│                                                    │
│ Home • About • Services • Menu • Contact           │
│ Terms • Food Regulations                           │
└───────────────────────┬────────────────────────────┘
                        │
             ┌──────────┴──────────┐
             │                     │
             ▼                     ▼
      FOOD ORDERING          EVENT ENQUIRIES
             │                     │
          Cart                 Catering
             │                Event Rental
          Order                   │
             │                 Quote
             └──────────┬──────────┘
                        │
                        ▼
              ┌───────────────────┐
              │  ADMIN DASHBOARD  │
              │                   │
              │ Orders            │
              │ Menu              │
              │ Categories        │
              │ Availability      │
              └───────────────────┘
```

The core MVP should remain focused on:

> **Discover → Browse → Order → Manage**

while catering and event services follow:

> **Discover → Enquire → Quote → Confirm**

---

# 46. MVP Priority

### P0 — Must Have

- Homepage
- Menu
- Cart
- Food ordering
- Order storage
- Admin login
- Order management
- Menu management
- Add/edit/archive items
- Item availability
- Contact
- WhatsApp
- Mobile responsive design

### P1 — Important

- Catering enquiry
- Event rental information
- Email notifications
- Food regulations
- Allergen information
- SEO
- Accessibility improvements

### P2 — Future

- Online payments
- Customer accounts
- Delivery tracking
- Discounts
- Loyalty
- Inventory
- Analytics
- Staff accounts
- Advanced reporting

---

# 47. Product Principle

The most important principle for the first version is:

> **Keep ordering simple and make managing the business simple.**

A customer should be able to go from seeing a meal to submitting an order with minimal friction.

An administrator should be able to add or remove a menu item in minutes without needing a developer.

The website should feel like a **real food business**, not a generic software product: strong food photography, warm hospitality, clear information, prominent ordering, and easy WhatsApp communication.

