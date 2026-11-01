# Phase 24 — French Locale: Customer Copy

The French text for the customer emails and the admin's WhatsApp quote
message (requirement.md Decisions 2, 3, 6 and 7). There's no approval
step. The code uses this word for word: `order_emails.rs` for emails,
`lib/quotes.ts` for WhatsApp. Any later wording change is made here
first, then in code. Page copy is `messages/fr.json`, translated from
`en.json` during implementation (plan Group 4).

Style (Decision 3):
- formal `vous`
- French spacing before `:` `!` `?` in sentences
- plain-text emails, the sign-off `— Zain's Treat n More`, amounts as
  `€{amount}`, dates as `DD-MM-YYYY`
- lines that code builds as `{label}: {value}` keep `: `

Placeholders are in `{braces}`. Each block mirrors the English text in
`order_emails.rs` field for field.

**Every customer email now carries all three languages** (requirement.md
Decision 11). §2–4 are the French *section* text. The old per-language
subjects (`INVOICE_*.subject`, `QUOTE_*.subject` and the long
`status_email` subjects) go: §7 has the short subjects for all three
languages. §8 shows how the sections fit together.

## 1. Greeting

`Bonjour {customer_name},` (English `Hi`, Dutch `Hoi`).

## 2. Status emails (`status_email`)

The French section is `Bonjour {customer_name},\n\n{message}\n\n{link_label}\n{tracking_url}`,
as in English. The sign-off is added once, after all sections (§8).

### `processing`

- Message: `Bonne nouvelle : nous avons bien reçu votre paiement pour la commande {reference}, et nous préparons maintenant votre repas.`
- Link label: `Suivez votre commande ici :`

### `ready_for_collection`

- Message: `Votre commande {reference} est prête à être retirée. À bientôt !`
- Link label: `Une fois votre commande retirée, vous pouvez nous le signaler sur la page de votre commande :`

### `out_for_delivery`

- Message: `Votre commande {reference} est en route vers vous.`
- Link label: `Une fois votre commande arrivée, vous pouvez nous le signaler sur la page de votre commande :`

### `serving_on_site`

- Message: `Notre équipe est à votre événement et le service traiteur de la commande {reference} a commencé. Profitez bien de votre événement !`
- Link label: `Ensuite, vous pouvez nous confirmer que tout s'est bien passé sur la page de votre commande :`

### `completed`

- Message: `Votre commande {reference} est terminée. Merci d'avoir commandé chez Zain's Treat n More, et bon appétit !`
- Link label: `La page de votre commande :`

### `cancelled`

- Message: `Votre commande {reference} a été annulée.\n\nSi vous avez des questions, ou si vous avez déjà payé, contactez-nous :\n{contact_url}`
- Link label: `La page de votre commande :`

## 3. Invoice email (`INVOICE_FR`)

| Field | French |
|---|---|
| `greeting` | `Bonjour` |
| `intro` | `Merci pour votre commande ! Voici votre facture.` |
| `order` | `Commande` |
| `date` | `Date` |
| `pickup` | `À retirer` |
| `delivery_to` | `Livraison à` |
| `served_at` | `Lieu` |
| `total` | `Total` |
| `pay_by_transfer` | `Veuillez payer par virement bancaire :` |
| `account_name` | `Titulaire du compte` |
| `reference` | `Référence` |
| `use_reference` | `Merci d'utiliser exactement cette référence, afin que nous puissions retrouver votre paiement.` |
| `whatsapp_fallback` | `Contactez-nous sur WhatsApp pour obtenir nos coordonnées bancaires.` |
| `verified_only` | `Nous commençons à préparer votre commande uniquement après vérification de votre paiement.` |
| `after_paying` | `Après votre paiement, signalez-le-nous sur la page de votre commande :` |

## 4. Quote email (`QUOTE_FR`)

| Field | French |
|---|---|
| `greeting` | `Bonjour` |
| `intro` | `Merci pour votre demande. Voici notre devis pour votre événement du` |
| `guests` | `invités` |
| `quote` | `Devis` |
| `notes` | `Remarques` |
| `valid_until` | `Ce devis est valable jusqu'au` |
| `view` | `Consultez le devis, puis acceptez-le ou refusez-le ici :` |
| `after_accepting` | `Après acceptation, vous recevrez une facture avec nos coordonnées bancaires. Nous commençons la préparation dès que votre paiement a été vérifié.` |

Rendered, the opening reads:

```
Bonjour {customer_name},

Merci pour votre demande. Voici notre devis pour votre événement du {event_date} ({guest_count} invités).

Devis: #{quote_id}
{fulfilment_line}
```

## 5. WhatsApp quote message (`lib/quotes.ts`)

```
Bonjour {name}, voici votre devis #{id} de Zain's Treat n More pour votre événement du {date} : {url}
```

## 6. Language name

The switcher's French option: `Français` (Decision 5).

## 7. Short subjects (all three languages)

Joined with ` / `, customer's language first, then ` — {reference}`
(` — #{quote_id}` for the quote).

| Email | English | Nederlands | Français |
|---|---|---|---|
| Invoice | `Invoice` | `Factuur` | `Facture` |
| `processing` | `Payment confirmed` | `Betaling bevestigd` | `Paiement confirmé` |
| `ready_for_collection` | `Ready for collection` | `Klaar om op te halen` | `Prête à être retirée` |
| `out_for_delivery` | `On the way` | `Onderweg` | `En route` |
| `serving_on_site` | `Catering started` | `Catering begonnen` | `Service traiteur commencé` |
| `completed` | `Thank you` | `Bedankt` | `Merci` |
| `cancelled` | `Order cancelled` | `Bestelling geannuleerd` | `Commande annulée` |
| Quote | `Catering quote` | `Cateringofferte` | `Devis traiteur` |

Example for a French customer: `Paiement confirmé / Payment confirmed / Betaling bevestigd — ZTM-12`.

## 8. Assembled email

For a customer whose locale is `nl`:

```
--- Nederlands ---

Hoi {customer_name},

{Dutch section, with links to /nl/…}

--- English ---

Hi {customer_name},

{English section, with links to /en/…}

--- Français ---

Bonjour {customer_name},

{French section, with links to /fr/…}

— Zain's Treat n More
```

- Each section is the full single-language body from before, without its
  sign-off.
- Sections are separated by one blank line.
- The header names (`English`, `Nederlands`, `Français`) are the same in
  every email, and the same as the site's language switcher.
- The sender shows as `Zain's Treat n More <{address}>`
  (`EMAIL_FROM_ADDRESS`).
