//! Customer-facing order emails, in the customer's language (Phase 22
//! Decisions 6, 7). Pure functions returning `(subject, body)`, so tests
//! assert on the text directly; sending stays with `EmailConfig`. Owner
//! emails stay in `routes/orders.rs`, in English. Phase 23's quote email
//! lives here too; its text is `specs/2026-11-01-phase-23-catering-quotations/copy.md`.

use crate::routes::orders::OrderResponse;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use uuid::Uuid;

const SIGN_OFF: &str = "— Zain's Treat n More";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Locale {
    En,
    Nl,
}

impl Locale {
    /// Mirrors the CHECK on `orders.locale`.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "en" => Some(Self::En),
            "nl" => Some(Self::Nl),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Nl => "nl",
        }
    }
}

/// The customer's tracking page, in their language.
pub fn tracking_url(site_url: &str, locale: Locale, token: Uuid) -> String {
    format!("{site_url}/{}/order/{token}", locale.as_str())
}

/// The customer's catering quote page (Phase 23 Decision 9).
pub fn quote_url(site_url: &str, locale: Locale, token: Uuid) -> String {
    format!("{site_url}/{}/quote/{token}", locale.as_str())
}

/// The contact page (with WhatsApp), linked from the cancellation email.
pub fn contact_url(site_url: &str, locale: Locale) -> String {
    format!("{site_url}/{}/contact", locale.as_str())
}

/// The email sent when the admin moves an order to `status`, or `None` for
/// statuses that don't email the customer (Phase 22 Decision 6).
pub fn status_email(
    locale: Locale,
    customer_name: &str,
    reference: &str,
    status: &str,
    tracking_url: &str,
    contact_url: &str,
) -> Option<(String, String)> {
    let (subject, message, link_label) = match (locale, status) {
        (Locale::En, "processing") => (
            format!("Payment confirmed for order {reference} — Zain's Treat n More"),
            format!("Good news: we've received your payment for order {reference}, and we're now preparing your food."),
            "Follow your order here:",
        ),
        (Locale::Nl, "processing") => (
            format!("Betaling bevestigd voor bestelling {reference} — Zain's Treat n More"),
            format!("Goed nieuws: we hebben je betaling voor bestelling {reference} ontvangen en we zijn nu je eten aan het bereiden."),
            "Volg je bestelling hier:",
        ),
        (Locale::En, "ready_for_collection") => (
            format!("Your order {reference} is ready for collection"),
            format!("Your order {reference} is ready for collection. See you soon!"),
            "Once you've collected it, you can let us know on your order page:",
        ),
        (Locale::Nl, "ready_for_collection") => (
            format!("Je bestelling {reference} staat klaar om op te halen"),
            format!("Je bestelling {reference} staat klaar om op te halen. Tot zo!"),
            "Heb je hem opgehaald? Laat het ons weten op je bestelpagina:",
        ),
        (Locale::En, "out_for_delivery") => (
            format!("Your order {reference} is on the way"),
            format!("Your order {reference} is on its way to you."),
            "Once it has arrived, you can let us know on your order page:",
        ),
        (Locale::Nl, "out_for_delivery") => (
            format!("Je bestelling {reference} is onderweg"),
            format!("Je bestelling {reference} is onderweg naar je toe."),
            "Is hij aangekomen? Laat het ons weten op je bestelpagina:",
        ),
        (Locale::En, "serving_on_site") => (
            format!("Your catering for order {reference} has started"),
            format!("Our team is at your event and your catering for order {reference} has started. Enjoy your event!"),
            "Afterwards, you can let us know everything went well on your order page:",
        ),
        (Locale::Nl, "serving_on_site") => (
            format!("De catering voor bestelling {reference} is begonnen"),
            format!("Ons team is bij je evenement en de catering voor bestelling {reference} is begonnen. Veel plezier!"),
            "Is alles goed verlopen? Laat het ons weten op je bestelpagina:",
        ),
        (Locale::En, "completed") => (
            format!("Thank you for your order {reference}"),
            format!("Your order {reference} is complete. Thank you for ordering from Zain's Treat n More, and enjoy your meal!"),
            "Your order page:",
        ),
        (Locale::Nl, "completed") => (
            format!("Bedankt voor je bestelling {reference}"),
            format!("Je bestelling {reference} is afgerond. Bedankt dat je bij Zain's Treat n More hebt besteld, en eet smakelijk!"),
            "Je bestelpagina:",
        ),
        (Locale::En, "cancelled") => (
            format!("Your order {reference} has been cancelled"),
            format!("Your order {reference} has been cancelled.\n\nIf you have any questions, or if you already paid, please contact us:\n{contact_url}"),
            "Your order page:",
        ),
        (Locale::Nl, "cancelled") => (
            format!("Je bestelling {reference} is geannuleerd"),
            format!("Je bestelling {reference} is geannuleerd.\n\nHeb je vragen, of heb je al betaald? Neem dan contact met ons op:\n{contact_url}"),
            "Je bestelpagina:",
        ),
        _ => return None,
    };

    let greeting = match locale {
        Locale::En => "Hi",
        Locale::Nl => "Hoi",
    };

    Some((
        subject,
        format!("{greeting} {customer_name},\n\n{message}\n\n{link_label}\n{tracking_url}\n\n{SIGN_OFF}"),
    ))
}

struct InvoiceCopy {
    subject: &'static str,
    greeting: &'static str,
    intro: &'static str,
    order: &'static str,
    date: &'static str,
    pickup: &'static str,
    delivery_to: &'static str,
    served_at: &'static str,
    total: &'static str,
    pay_by_transfer: &'static str,
    account_name: &'static str,
    reference: &'static str,
    use_reference: &'static str,
    whatsapp_fallback: &'static str,
    verified_only: &'static str,
    after_paying: &'static str,
}

const INVOICE_EN: InvoiceCopy = InvoiceCopy {
    subject: "Your invoice for order",
    greeting: "Hi",
    intro: "Thanks for your order! Here is your invoice.",
    order: "Order",
    date: "Date",
    pickup: "Pickup",
    delivery_to: "Delivery to",
    served_at: "Served at",
    total: "Total",
    pay_by_transfer: "Please pay by bank transfer:",
    account_name: "Account name",
    reference: "Reference",
    use_reference: "Please use this exact reference so we can match your payment.",
    whatsapp_fallback: "Please contact us on WhatsApp for our bank details.",
    verified_only: "We only start preparing your order once your payment has been verified.",
    after_paying: "After paying, let us know on your order page:",
};

const INVOICE_NL: InvoiceCopy = InvoiceCopy {
    subject: "Je factuur voor bestelling",
    greeting: "Hoi",
    intro: "Bedankt voor je bestelling! Hier is je factuur.",
    order: "Bestelling",
    date: "Datum",
    pickup: "Afhalen",
    delivery_to: "Bezorgen op",
    served_at: "Locatie",
    total: "Totaal",
    pay_by_transfer: "Betaal via bankoverschrijving:",
    account_name: "Naam rekeninghouder",
    reference: "Kenmerk",
    use_reference: "Gebruik precies dit kenmerk, zodat we je betaling kunnen koppelen.",
    whatsapp_fallback: "Neem contact met ons op via WhatsApp voor onze bankgegevens.",
    verified_only: "We beginnen pas met je bestelling als je betaling is gecontroleerd.",
    after_paying: "Heb je betaald? Laat het ons weten op je bestelpagina:",
};

/// The customer's invoice email (Phase 20 Decision 8), moved here from
/// `routes/orders.rs` and localized (Phase 22 Decision 7). Plain text; no
/// VAT line (KOR).
pub fn invoice_email(
    locale: Locale,
    order: &OrderResponse,
    placed_at: &str,
    tracking_url: &str,
) -> (String, String) {
    let copy = match locale {
        Locale::En => &INVOICE_EN,
        Locale::Nl => &INVOICE_NL,
    };

    let delivery_line = fulfilment_line(copy, &order.delivery_type, order.delivery_address.as_deref());

    let mut body = format!(
        "{} {},\n\n{}\n\n{}: {}\n{}: {}\n{}\n\n",
        copy.greeting,
        order.customer_name,
        copy.intro,
        copy.order,
        order.payment_reference,
        copy.date,
        placed_at,
        delivery_line,
    );

    for item in &order.items {
        let name = match &item.option_label {
            Some(label) => format!("{} ({label})", item.item_name),
            None => item.item_name.clone(),
        };
        body.push_str(&format!("{} x {name} — €{}\n", item.quantity, item.subtotal));
    }
    body.push_str(&format!("\n{}: €{}\n\n", copy.total, order.total));

    match &order.payment {
        Some(payment) => {
            body.push_str(&format!(
                "{}\n{}: {}\nIBAN: {}\n",
                copy.pay_by_transfer, copy.account_name, payment.account_name, payment.iban,
            ));
            if let Some(bic) = &payment.bic {
                body.push_str(&format!("BIC: {bic}\n"));
            }
            body.push_str(&format!("{}: {}\n\n{}\n\n", copy.reference, payment.reference, copy.use_reference));
            if let Some(kvk) = &payment.kvk {
                body.push_str(&format!("KvK: {kvk}\n\n"));
            }
        }
        None => body.push_str(&format!("{}\n\n", copy.whatsapp_fallback)),
    }

    body.push_str(&format!("{}\n\n{}\n{tracking_url}\n\n{SIGN_OFF}", copy.verified_only, copy.after_paying));

    (
        format!("{} {} — Zain's Treat n More", copy.subject, order.payment_reference),
        body,
    )
}

/// "Pickup", "Delivery to: …" or, for on-site catering, "Served at: …".
fn fulfilment_line(copy: &InvoiceCopy, delivery_type: &str, address: Option<&str>) -> String {
    match (delivery_type, address) {
        ("delivery", Some(address)) => format!("{}: {address}", copy.delivery_to),
        ("on_site", Some(address)) => format!("{}: {address}", copy.served_at),
        _ => copy.pickup.to_string(),
    }
}

/// One quote line, for `quote_email`.
pub struct QuoteEmailLine<'a> {
    pub description: &'a str,
    pub quantity: i32,
    pub subtotal: Decimal,
}

/// Everything `quote_email` needs, so the call site names each field.
pub struct QuoteEmail<'a> {
    pub customer_name: &'a str,
    pub quote_id: i64,
    pub event_date: NaiveDate,
    pub guest_count: i32,
    pub delivery_type: &'a str,
    pub delivery_address: Option<&'a str>,
    pub lines: &'a [QuoteEmailLine<'a>],
    pub total: Decimal,
    pub notes: Option<&'a str>,
    pub valid_until: NaiveDate,
    pub url: &'a str,
}

struct QuoteCopy {
    subject: &'static str,
    greeting: &'static str,
    intro: &'static str,
    guests: &'static str,
    quote: &'static str,
    notes: &'static str,
    valid_until: &'static str,
    view: &'static str,
    after_accepting: &'static str,
}

const QUOTE_EN: QuoteCopy = QuoteCopy {
    subject: "Your catering quote",
    greeting: "Hi",
    intro: "Thank you for your enquiry. Here is our quote for your event on",
    guests: "guests",
    quote: "Quote",
    notes: "Notes",
    valid_until: "This quote is valid until",
    view: "View the quote and accept or decline it here:",
    after_accepting: "Once you accept, you'll receive an invoice with our bank details. We start preparing once your payment has been verified.",
};

const QUOTE_NL: QuoteCopy = QuoteCopy {
    subject: "Je cateringofferte",
    greeting: "Hoi",
    intro: "Bedankt voor je aanvraag. Hier is onze offerte voor je evenement op",
    guests: "gasten",
    quote: "Offerte",
    notes: "Opmerkingen",
    valid_until: "Deze offerte is geldig tot en met",
    view: "Bekijk de offerte en accepteer of weiger hem hier:",
    after_accepting: "Na het accepteren ontvang je een factuur met onze bankgegevens. We beginnen met de voorbereiding zodra je betaling is gecontroleerd.",
};

/// The quote sent to the customer when the owner presses "Send" (Phase 23
/// Decision 9). Plain text; no VAT line (KOR).
pub fn quote_email(locale: Locale, quote: &QuoteEmail) -> (String, String) {
    let (copy, invoice) = match locale {
        Locale::En => (&QUOTE_EN, &INVOICE_EN),
        Locale::Nl => (&QUOTE_NL, &INVOICE_NL),
    };

    let mut body = format!(
        "{} {},\n\n{} {} ({} {}).\n\n{}: #{}\n{}\n\n",
        copy.greeting,
        quote.customer_name,
        copy.intro,
        quote.event_date.format("%d-%m-%Y"),
        quote.guest_count,
        copy.guests,
        copy.quote,
        quote.quote_id,
        fulfilment_line(invoice, quote.delivery_type, quote.delivery_address),
    );

    for line in quote.lines {
        body.push_str(&format!("{} x {} — €{}\n", line.quantity, line.description, line.subtotal));
    }
    body.push_str(&format!("\n{}: €{}\n\n", invoice.total, quote.total));

    if let Some(notes) = quote.notes.map(str::trim).filter(|n| !n.is_empty()) {
        body.push_str(&format!("{}:\n{notes}\n\n", copy.notes));
    }

    body.push_str(&format!(
        "{} {}.\n\n{}\n{}\n\n{}\n\n{SIGN_OFF}",
        copy.valid_until,
        quote.valid_until.format("%d-%m-%Y"),
        copy.view,
        quote.url,
        copy.after_accepting,
    ));

    (format!("{} #{} — Zain's Treat n More", copy.subject, quote.quote_id), body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::payment::PaymentInstructions;
    use crate::routes::orders::OrderItemResponse;
    use rust_decimal::Decimal;

    const URL: &str = "https://zainstreat.com/nl/order/x";
    const CONTACT: &str = "https://zainstreat.com/nl/contact";
    const EMAILING: [&str; 6] =
        ["processing", "ready_for_collection", "out_for_delivery", "serving_on_site", "completed", "cancelled"];

    fn order(payment: Option<PaymentInstructions>) -> OrderResponse {
        OrderResponse {
            id: 1042,
            customer_name: "Amina".into(),
            customer_email: "amina@example.com".into(),
            customer_phone: "0612345678".into(),
            delivery_type: "pickup".into(),
            delivery_address: None,
            notes: None,
            subtotal: Decimal::new(3050, 2),
            delivery_fee: Decimal::ZERO,
            total: Decimal::new(3050, 2),
            status: "awaiting_payment".into(),
            created_at: chrono::Utc::now(),
            items: vec![
                OrderItemResponse {
                    item_name: "Jollof Rice".into(),
                    option_label: Some("Large".into()),
                    unit_price: Decimal::new(1200, 2),
                    quantity: 2,
                    subtotal: Decimal::new(2400, 2),
                },
                OrderItemResponse {
                    item_name: "Puff Puff".into(),
                    option_label: None,
                    unit_price: Decimal::new(650, 2),
                    quantity: 1,
                    subtotal: Decimal::new(650, 2),
                },
            ],
            public_token: Uuid::nil(),
            payment_reference: "ZTM-1042".into(),
            payment,
            payment_claimed_at: None,
            payment_confirmed_at: None,
            ready_at: None,
            completed_at: None,
            cancelled_at: None,
            completed_by: None,
        }
    }

    fn bank() -> PaymentInstructions {
        PaymentInstructions {
            account_name: "Zain's Treat n More".into(),
            iban: "NL91ABNA0417164300".into(),
            bic: None,
            kvk: None,
            reference: "ZTM-1042".into(),
        }
    }

    #[test]
    fn every_emailing_status_has_copy_in_both_languages() {
        for locale in [Locale::En, Locale::Nl] {
            for status in EMAILING {
                let (subject, body) = status_email(locale, "Amina", "ZTM-1042", status, URL, CONTACT)
                    .unwrap_or_else(|| panic!("{status} {locale:?}"));
                assert!(subject.contains("ZTM-1042"), "{status} {locale:?}");
                assert!(body.contains("ZTM-1042") && body.contains(URL) && body.contains("Amina"));
            }
        }
    }

    #[test]
    fn statuses_before_payment_confirmation_send_nothing() {
        for status in ["awaiting_payment", "payment_claimed"] {
            assert!(status_email(Locale::En, "Amina", "ZTM-1042", status, URL, CONTACT).is_none());
        }
    }

    #[test]
    fn cancellation_points_to_the_contact_page() {
        let (_, body) = status_email(Locale::Nl, "Amina", "ZTM-1042", "cancelled", URL, CONTACT).unwrap();
        assert!(body.contains(CONTACT));
    }

    #[test]
    fn dutch_status_email_is_dutch() {
        let (subject, body) = status_email(Locale::Nl, "Amina", "ZTM-1042", "out_for_delivery", URL, CONTACT).unwrap();
        assert_eq!(subject, "Je bestelling ZTM-1042 is onderweg");
        assert!(body.starts_with("Hoi Amina,"));
    }

    /// The English invoice is Phase 20's text unchanged, apart from the
    /// locale in the order-page link.
    #[test]
    fn english_invoice_keeps_the_phase_20_text() {
        let (subject, body) = invoice_email(Locale::En, &order(Some(bank())), "31-10-2026 14:05", "https://zainstreat.com/en/order/x");
        assert_eq!(subject, "Your invoice for order ZTM-1042 — Zain's Treat n More");
        assert_eq!(
            body,
            "Hi Amina,\n\nThanks for your order! Here is your invoice.\n\nOrder: ZTM-1042\nDate: 31-10-2026 14:05\nPickup\n\n\
             2 x Jollof Rice (Large) — €24.00\n1 x Puff Puff — €6.50\n\nTotal: €30.50\n\n\
             Please pay by bank transfer:\nAccount name: Zain's Treat n More\nIBAN: NL91ABNA0417164300\n\
             Reference: ZTM-1042\n\nPlease use this exact reference so we can match your payment.\n\n\
             We only start preparing your order once your payment has been verified.\n\n\
             After paying, let us know on your order page:\nhttps://zainstreat.com/en/order/x\n\n— Zain's Treat n More"
        );
    }

    #[test]
    fn dutch_invoice_without_bank_details_falls_back_to_whatsapp() {
        let (subject, body) = invoice_email(Locale::Nl, &order(None), "31-10-2026 14:05", URL);
        assert_eq!(subject, "Je factuur voor bestelling ZTM-1042 — Zain's Treat n More");
        assert!(body.contains("Afhalen") && body.contains("Totaal: €30.50"));
        assert!(body.contains("via WhatsApp") && !body.contains("IBAN"));
    }

    #[test]
    fn on_site_invoice_shows_where_the_food_is_served() {
        let mut on_site = order(None);
        on_site.delivery_type = "on_site".into();
        on_site.delivery_address = Some("Zaal De Linde, Utrecht".into());
        let (_, en) = invoice_email(Locale::En, &on_site, "31-10-2026 14:05", URL);
        assert!(en.contains("Served at: Zaal De Linde, Utrecht"));
        let (_, nl) = invoice_email(Locale::Nl, &on_site, "31-10-2026 14:05", URL);
        assert!(nl.contains("Locatie: Zaal De Linde, Utrecht"));
    }

    const QUOTE_URL: &str = "https://zainstreat.com/en/quote/x";

    fn quote_lines() -> Vec<QuoteEmailLine<'static>> {
        vec![
            QuoteEmailLine { description: "Jollof rice per guest", quantity: 80, subtotal: Decimal::new(96000, 2) },
            QuoteEmailLine { description: "Serving staff", quantity: 2, subtotal: Decimal::new(25000, 2) },
        ]
    }

    fn quote<'a>(lines: &'a [QuoteEmailLine<'a>], notes: Option<&'a str>) -> QuoteEmail<'a> {
        QuoteEmail {
            customer_name: "Amina",
            quote_id: 12,
            event_date: NaiveDate::from_ymd_opt(2026, 12, 5).unwrap(),
            guest_count: 80,
            delivery_type: "on_site",
            delivery_address: Some("Zaal De Linde, Utrecht"),
            lines,
            total: Decimal::new(121000, 2),
            notes,
            valid_until: NaiveDate::from_ymd_opt(2026, 11, 20).unwrap(),
            url: QUOTE_URL,
        }
    }

    /// Pinned to copy.md §1, word for word.
    #[test]
    fn english_quote_email_matches_the_approved_copy() {
        let lines = quote_lines();
        let (subject, body) = quote_email(Locale::En, &quote(&lines, Some("Halal, no pork.")));
        assert_eq!(subject, "Your catering quote #12 — Zain's Treat n More");
        assert_eq!(
            body,
            "Hi Amina,\n\nThank you for your enquiry. Here is our quote for your event on 05-12-2026 (80 guests).\n\n\
             Quote: #12\nServed at: Zaal De Linde, Utrecht\n\n\
             80 x Jollof rice per guest — €960.00\n2 x Serving staff — €250.00\n\nTotal: €1210.00\n\n\
             Notes:\nHalal, no pork.\n\n\
             This quote is valid until 20-11-2026.\n\n\
             View the quote and accept or decline it here:\nhttps://zainstreat.com/en/quote/x\n\n\
             Once you accept, you'll receive an invoice with our bank details. We start preparing once your payment has been verified.\n\n\
             — Zain's Treat n More"
        );
    }

    #[test]
    fn dutch_quote_email_is_dutch_and_skips_empty_notes() {
        let lines = quote_lines();
        let (subject, body) = quote_email(Locale::Nl, &quote(&lines, Some("  ")));
        assert_eq!(subject, "Je cateringofferte #12 — Zain's Treat n More");
        assert!(body.starts_with("Hoi Amina,"));
        assert!(body.contains("(80 gasten)") && body.contains("Locatie: Zaal De Linde, Utrecht"));
        assert!(body.contains("Totaal: €1210.00") && body.contains("geldig tot en met 20-11-2026"));
        assert!(body.contains(QUOTE_URL) && !body.contains("Opmerkingen"));
    }

    #[test]
    fn quote_url_includes_the_locale() {
        assert_eq!(
            quote_url("https://zainstreat.com", Locale::Nl, Uuid::nil()),
            "https://zainstreat.com/nl/quote/00000000-0000-0000-0000-000000000000"
        );
    }

    #[test]
    fn tracking_url_includes_the_locale() {
        assert_eq!(
            tracking_url("https://zainstreat.com", Locale::Nl, Uuid::nil()),
            "https://zainstreat.com/nl/order/00000000-0000-0000-0000-000000000000"
        );
    }
}
