//! Customer-facing order emails, in the customer's language (Phase 22
//! Decisions 6, 7). Pure functions returning `(subject, body)`, so tests
//! assert on the text directly; sending stays with `EmailConfig`. Owner
//! emails stay in `routes/orders.rs`, in English.

use crate::routes::orders::OrderResponse;
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

    let delivery_line = match order.delivery_address.as_deref() {
        Some(address) if order.delivery_type == "delivery" => format!("{}: {address}", copy.delivery_to),
        _ => copy.pickup.to_string(),
    };

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::payment::PaymentInstructions;
    use crate::routes::orders::OrderItemResponse;
    use rust_decimal::Decimal;

    const URL: &str = "https://zainstreat.com/nl/order/x";
    const CONTACT: &str = "https://zainstreat.com/nl/contact";
    const EMAILING: [&str; 5] = ["processing", "ready_for_collection", "out_for_delivery", "completed", "cancelled"];

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
    fn tracking_url_includes_the_locale() {
        assert_eq!(
            tracking_url("https://zainstreat.com", Locale::Nl, Uuid::nil()),
            "https://zainstreat.com/nl/order/00000000-0000-0000-0000-000000000000"
        );
    }
}
