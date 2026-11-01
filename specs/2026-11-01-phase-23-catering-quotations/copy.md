# Phase 23 — Catering Quotations: Customer Copy

This is the exact customer-facing text for this phase, in English and
Dutch (requirement.md Decision 15). **Approved 2026-11-01 by the project
owner, Dutch included.** No separate client sign-off is needed. The code
uses it word for word: `order_emails.rs` for emails, and
`messages/{en,nl}.json` for pages. Any later wording change is made here
first, then in code.

Style follows the existing Phase 20/22 copy: informal Dutch (`je`), plain
text emails, the sign-off `— Zain's Treat n More`, amounts as `€{amount}`
(the same format as the invoice), and dates as `DD-MM-YYYY`.

Placeholders are in `{braces}`.

## 1. Quote email (customer)

Sent when the owner presses "Send to customer". It goes out in the
enquiry's language.

### English

Subject: `Your catering quote #{quote_id} — Zain's Treat n More`

```
Hi {customer_name},

Thank you for your enquiry. Here is our quote for your event on {event_date} ({guest_count} guests).

Quote: #{quote_id}
{fulfilment_line}

{quantity} x {description} — €{subtotal}
{…one line per item}

Total: €{total}

Notes:
{notes}

This quote is valid until {valid_until}.

View the quote and accept or decline it here:
{quote_url}

Once you accept, you'll receive an invoice with our bank details. We start preparing once your payment has been verified.

— Zain's Treat n More
```

`{fulfilment_line}`:
- pickup: `Pickup`
- delivery: `Delivery to: {address}`
- on-site: `Served at: {address}`

Leave out the `Notes:` block, and the blank line after it, when there
are no notes.

### Dutch

Subject: `Je cateringofferte #{quote_id} — Zain's Treat n More`

```
Hoi {customer_name},

Bedankt voor je aanvraag. Hier is onze offerte voor je evenement op {event_date} ({guest_count} gasten).

Offerte: #{quote_id}
{fulfilment_line}

{quantity} x {description} — €{subtotal}
{…one line per item}

Totaal: €{total}

Opmerkingen:
{notes}

Deze offerte is geldig tot en met {valid_until}.

Bekijk de offerte en accepteer of weiger hem hier:
{quote_url}

Na het accepteren ontvang je een factuur met onze bankgegevens. We beginnen met de voorbereiding zodra je betaling is gecontroleerd.

— Zain's Treat n More
```

`{fulfilment_line}`:
- pickup: `Afhalen`
- delivery: `Bezorgen op: {address}`
- on-site: `Locatie: {address}`

## 2. Quote page (`/{locale}/quote/{token}`)

`messages/{en,nl}.json`, `Quote` namespace. The event type labels reuse
the existing `ContactPage.eventType.*` keys.

| Key | English | Dutch |
|---|---|---|
| `metaTitle` | Your quote | Je offerte |
| `heading` | Quote #{id} | Offerte #{id} |
| `eventSummary` | {eventType} on {date} · {guests} guests | {eventType} op {date} · {guests} gasten |
| `fulfilmentPickup` | Pickup | Afhalen |
| `fulfilmentDelivery` | Delivery to {address} | Bezorgen op {address} |
| `fulfilmentOnSite` | Served at {address} | Locatie: {address} |
| `linesCaption` | Quote items | Offerteregels |
| `colDescription` | Description | Omschrijving |
| `colQuantity` | Qty | Aantal |
| `colPrice` | Price | Prijs |
| `colSubtotal` | Subtotal | Subtotaal |
| `total` | Total | Totaal |
| `notes` | Notes from us | Opmerkingen van ons |
| `validUntil` | Valid until {date} | Geldig tot en met {date} |
| `statusSent` | Please review the quote above and accept or decline it. | Bekijk de offerte hierboven en accepteer of weiger hem. |
| `accept` | Accept quote | Offerte accepteren |
| `acceptConfirm` | Yes, accept quote | Ja, accepteren |
| `acceptHint` | After accepting, you'll get an invoice with our bank details. | Na het accepteren krijg je een factuur met onze bankgegevens. |
| `decline` | Decline | Weigeren |
| `declineReasonLabel` | Reason (optional) | Reden (optioneel) |
| `declineReasonPlaceholder` | Let us know what didn't work for you | Laat ons weten wat er niet paste |
| `declineConfirm` | Decline quote | Offerte weigeren |
| `keep` | Cancel | Annuleren |
| `statusDraft` | This quote is being updated. You'll hear from us when it's ready. | Deze offerte wordt bijgewerkt. Je hoort van ons zodra hij klaar is. |
| `statusExpired` | This quote expired on {date}. Contact us and we'll send you an updated one. | Deze offerte is verlopen op {date}. Neem contact met ons op, dan sturen we je een nieuwe. |
| `contactWhatsApp` | Contact us on WhatsApp | Neem contact op via WhatsApp |
| `statusDeclined` | You declined this quote. Changed your mind? Contact us. | Je hebt deze offerte geweigerd. Toch interesse? Neem contact met ons op. |
| `statusAccepted` | You accepted this quote. | Je hebt deze offerte geaccepteerd. |
| `viewOrder` | View your order and payment details | Bekijk je bestelling en betaalgegevens |
| `errorChanged` | This quote has changed. Please refresh the page to see the latest version. | Deze offerte is gewijzigd. Vernieuw de pagina om de nieuwste versie te zien. |
| `errorGeneric` | Something went wrong. Please try again. | Er ging iets mis. Probeer het opnieuw. |

`statusDraft` doesn't promise an email, because the quote may be sent
by WhatsApp instead (requirement.md Decision 14).

## 3. On-site fulfilment (requirement.md Decision 12)

### Tracking page and invoice (`Order` namespace)

| Key | English | Dutch |
|---|---|---|
| `tracking.cateringInProgress` | Catering in progress | Catering bezig |
| `tracking.receivedOnSite` | Our event has been catered | Ons evenement is verzorgd |
| `servedAtLine` | Served at {address} | Locatie: {address} |

The existing `receivedConfirm` ("Yes, I've got it" / "Ja, ik heb hem")
reads oddly for an event, so on-site gets its own:

| Key | English | Dutch |
|---|---|---|
| `tracking.receivedOnSiteConfirm` | Yes, all done | Ja, alles is klaar |

### Status email, `serving_on_site`

English:
- Subject: `Your catering for order {reference} has started`
- Message: `Our team is at your event and your catering for order {reference} has started. Enjoy your event!`
- Link label: `Afterwards, you can let us know everything went well on your order page:`

Dutch:
- Subject: `De catering voor bestelling {reference} is begonnen`
- Message: `Ons team is bij je evenement en de catering voor bestelling {reference} is begonnen. Veel plezier!`
- Link label: `Is alles goed verlopen? Laat het ons weten op je bestelpagina:`

The greeting, layout and sign-off come from the existing `status_email`.

### Invoice email, on-site address line

- `en`: `Served at: {address}`
- `nl`: `Locatie: {address}`

## 4. WhatsApp share message (requirement.md Decision 14)

Prefilled when the owner presses "Send via WhatsApp" in admin. It's in
the enquiry's language.

- `en`: `Hi {customer_name}, here is your quote #{quote_id} from Zain's Treat n More for your event on {event_date}: {quote_url}`
- `nl`: `Hoi {customer_name}, hier is je offerte #{quote_id} van Zain's Treat n More voor je evenement op {event_date}: {quote_url}`

## 5. Owner emails (English only, not for client review)

Quote accepted:
- Subject: `Quote #{quote_id} accepted — order {reference} — €{total}`
- Body:

```
{customer_name} accepted quote #{quote_id} for their {event_type} on {event_date} ({guest_count} guests).

Order {reference} has been created and is awaiting payment.

{admin_url}/orders/{order_id}
```

Quote declined:
- Subject: `Quote #{quote_id} declined by {customer_name}`
- Body:

```
{customer_name} declined quote #{quote_id} for their {event_type} on {event_date}.

Reason: {reason, or "(none given)"}

{admin_url}/enquiries/catering/{enquiry_id}
```
