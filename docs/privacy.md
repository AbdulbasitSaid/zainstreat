# Privacy (GDPR)

Phase 33. The public policy is `/{locale}/privacy` (copy in
`apps/web/messages/*.json`, `PrivacyPage`). This file is the business's
own record: what is processed and why (GDPR Art. 30), who processes it,
how long it's kept, how to answer requests, and what to do after a
breach. Keep it in step with the public page.

## Controller

Zain's Treat n More, the Netherlands. KvK number: `BUSINESS_KVK`.
Contact: contact@zainstreat.com, WhatsApp +31 6 3054 5277. No data
protection officer is required (small scale, no special-category data,
no large-scale monitoring).

## Record of processing (Art. 30)

| Activity | Data subjects | Personal data | Purpose | Lawful basis | Kept |
| --- | --- | --- | --- | --- | --- |
| Food and rental orders (`orders`, `order_items`) | Customers | Name, email, phone, delivery address and distance, rental date, notes, items, amounts | Fulfil and invoice the order | Contract (6(1)(b)); tax records (6(1)(c)) | 7 years, then anonymised; unpaid cancelled orders 12 months |
| Event bookings (`event_bookings`, guests) | Guests | Name, email, phone, party size, meal choices, notes | Seat guests, cook, invoice | Contract (6(1)(b)); tax records (6(1)(c)) | Paid: 7 years. Free RSVP, waitlist, cancelled: 12 months after the event |
| Catering enquiries and quotes | Prospective customers | Name, email, phone, event details, message, quote | Answer and quote | Pre-contract steps (6(1)(b)) | Accepted: 7 years. Otherwise 12 months after the last update |
| Contact messages | Anyone who writes | Name, email, phone, subject, message | Answer the message | Legitimate interest (6(1)(f)) | 12 months, then deleted |
| Outgoing email log (`email_deliveries`) | Customers | Email address, delivery status | Troubleshoot delivery | Legitimate interest | 90 days, then deleted |
| Admin login (`users`, sessions, `login_failures`) | The owner | Name, email, password hash, session id | Secure the admin | Legitimate interest | Sessions until expiry (30 days idle); failed logins 24 hours |
| Admin push devices (`push_subscriptions`) | The owner | Device endpoint, browser name | Owner notifications | Legitimate interest | Until removed |
| Server logs (Caddy, containers) | Visitors | IP address, URL, browser | Security, debugging | Legitimate interest | Rotated at 3 × 10 MB per container (days) |
| Backups (S3) | All of the above | All of the above | Disaster recovery | Same as source | 30 days |
| GDPR request log (`privacy_requests`) | Requesters | SHA-256 of the email, row counts | Show requests were handled | Legal obligation (6(1)(c)) | Kept |

No special-category data is asked for. Allergy notes typed into "notes"
are only used to prepare the food.

The periods are constants in `apps/api/src/retention.rs`, enforced
nightly when `RETENTION_SWEEP_ENABLED=true`. A test pins them; change the
code, this table and the public page together.

## Processors

| Processor | What | Where | Agreement |
| --- | --- | --- | --- |
| Amazon Web Services (Lightsail, S3, SES) | Hosting, database, backups, outgoing email | `eu-central-1`, Frankfurt | AWS GDPR Data Processing Addendum, part of the AWS Service Terms (accepted with the account; download from AWS Artifact) |
| Namecheap (Private Email) | The `contact@` mailbox | Namecheap's mail infrastructure | Namecheap's DPA (accept in the account) |
| Apple / Google / Mozilla push services | Deliver admin notifications (customer name in the text) | May be outside the EEA | Browser vendor terms, standard contractual clauses |
| PDOK (Kadaster, Dutch government) | Address suggestions and lookups for delivery (Phase 37): the address text a customer types | Netherlands | Public open-data service; no account, no DPA |
| Google (Maps Platform, Routes API) | Driving distance for the delivery fee (Phase 37): **coordinates only**, no name or address text | May be outside the EEA | Google Maps Platform Data Processing Terms (accept in the client's Google Cloud account) |
| GitHub | Code and container images | US | No customer data |

**To do before production:** the client accepts both DPAs in their own
accounts after Phase 29 moves AWS to them, and keeps a copy.

## Answering a request

Customers write to contact@ (the public page says so). Answer within one
month. Check the request comes from the email address on file; if not,
ask for confirmation from that address first.

1. Open the admin's **Privacy** page and search the email.
2. **Access / portability:** click **Export everything (JSON)** and send
   the file.
3. **Rectification:** correct the record from its normal admin page, or
   ask the developer.
4. **Erasure:** finish or cancel any open order or upcoming booking
   first, then **Anonymise…** and type the email again. Orders keep their
   amounts (tax law); names and contact details go, contact messages are
   deleted, and tracking links stop working.
5. Reply to the customer saying what was done.

Each export and erasure is logged in `privacy_requests` by a hash of the
email, so there's proof without keeping the address.

## Data breach checklist

A breach is any loss, leak or unauthorised access to personal data (a
stolen admin password, a leaked backup, a hacked server).

1. **Contain it:** rotate the affected secret (see `docs/deployment.md`,
   "Changing a secret or setting"), and stop or snapshot the server if
   it's compromised.
2. **Write down:** what happened, when, which data and roughly how many
   people, and what you did. Keep this even if you don't report.
3. **Report within 72 hours** of finding out to the Autoriteit
   Persoonsgegevens through its online "Datalek melden" form, unless the
   breach is unlikely to put anyone at risk.
4. **Tell the affected customers** without delay if the risk to them is
   high (for example, addresses and phone numbers leaked).
5. **Fix the cause**, and note it here or in the phase specs.

The owner decides on reporting; the developer supplies the facts.
