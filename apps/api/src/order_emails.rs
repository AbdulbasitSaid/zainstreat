//! Customer-facing order emails (Phase 22 Decisions 6, 7). Pure functions
//! returning `(subject, body)`, so tests assert on the text directly;
//! sending stays with `EmailConfig`. Owner emails stay in `routes/orders.rs`,
//! in English. Phase 23's quote email lives here too; its text is
//! `specs/2026-11-01-phase-23-catering-quotations/copy.md`, and the French
//! is `specs/2026-11-01-phase-24-french-locale/copy.md`.
//!
//! Every email carries all three languages, the customer's first (Phase 24
//! Decision 11): each `*_section` function is one language's text, and
//! `multilingual` puts them together.

use crate::routes::orders::OrderResponse;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use uuid::Uuid;

const SIGN_OFF: &str = "— Zain's Treat n More";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Locale {
    En,
    Nl,
    Fr,
}

impl Locale {
    /// Every customer locale, in the order the email sections follow. Tests
    /// loop over it so a new locale can't miss copy.
    pub const ALL: [Self; 3] = [Self::En, Self::Nl, Self::Fr];

    /// Mirrors the CHECKs on `orders.locale` and `catering_enquiries.locale`.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "en" => Some(Self::En),
            "nl" => Some(Self::Nl),
            "fr" => Some(Self::Fr),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Nl => "nl",
            Self::Fr => "fr",
        }
    }

    /// The language in its own name, as on the site's language switcher.
    pub fn name(self) -> &'static str {
        match self {
            Self::En => "English",
            Self::Nl => "Nederlands",
            Self::Fr => "Français",
        }
    }

    fn greeting(self) -> &'static str {
        match self {
            Self::En => "Hi",
            Self::Nl => "Hoi",
            Self::Fr => "Bonjour",
        }
    }
}

/// One email from per-language `(locale, short subject, section)` parts, in
/// order: a `--- {name} ---` header per section, one sign-off at the end,
/// and the short subjects joined with " / ", then ` — {suffix}`.
fn multilingual(parts: &[(Locale, &str, String)], suffix: &str) -> (String, String) {
    let subject = parts
        .iter()
        .map(|(_, subject, _)| *subject)
        .collect::<Vec<_>>()
        .join(" / ");
    let body = parts
        .iter()
        .map(|(locale, _, section)| format!("--- {} ---\n\n{section}", locale.name()))
        .collect::<Vec<_>>()
        .join("\n\n");
    (
        format!("{subject} — {suffix}"),
        format!("{body}\n\n{SIGN_OFF}"),
    )
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

/// One language's short subject, message and link label for `status`, or
/// `None` for statuses that don't email the customer (Phase 22 Decision 6).
fn status_text(
    locale: Locale,
    status: &str,
    reference: &str,
    contact_url: &str,
) -> Option<(&'static str, String, &'static str)> {
    Some(match (locale, status) {
        (Locale::En, "processing") => (
            "Payment confirmed",
            format!(
                "Good news: we've received your payment for order {reference}, and we're now preparing your food."
            ),
            "Follow your order here:",
        ),
        (Locale::Nl, "processing") => (
            "Betaling bevestigd",
            format!(
                "Goed nieuws: we hebben je betaling voor bestelling {reference} ontvangen en we zijn nu je eten aan het bereiden."
            ),
            "Volg je bestelling hier:",
        ),
        (Locale::Fr, "processing") => (
            "Paiement confirmé",
            format!(
                "Bonne nouvelle : nous avons bien reçu votre paiement pour la commande {reference}, et nous préparons maintenant votre repas."
            ),
            "Suivez votre commande ici :",
        ),
        (Locale::En, "ready_for_collection") => (
            "Ready for collection",
            format!("Your order {reference} is ready for collection. See you soon!"),
            "Once you've collected it, you can let us know on your order page:",
        ),
        (Locale::Nl, "ready_for_collection") => (
            "Klaar om op te halen",
            format!("Je bestelling {reference} staat klaar om op te halen. Tot zo!"),
            "Heb je hem opgehaald? Laat het ons weten op je bestelpagina:",
        ),
        (Locale::Fr, "ready_for_collection") => (
            "Prête à être retirée",
            format!("Votre commande {reference} est prête à être retirée. À bientôt !"),
            "Une fois votre commande retirée, vous pouvez nous le signaler sur la page de votre commande :",
        ),
        (Locale::En, "out_for_delivery") => (
            "On the way",
            format!("Your order {reference} is on its way to you."),
            "Once it has arrived, you can let us know on your order page:",
        ),
        (Locale::Nl, "out_for_delivery") => (
            "Onderweg",
            format!("Je bestelling {reference} is onderweg naar je toe."),
            "Is hij aangekomen? Laat het ons weten op je bestelpagina:",
        ),
        (Locale::Fr, "out_for_delivery") => (
            "En route",
            format!("Votre commande {reference} est en route vers vous."),
            "Une fois votre commande arrivée, vous pouvez nous le signaler sur la page de votre commande :",
        ),
        (Locale::En, "serving_on_site") => (
            "Catering started",
            format!(
                "Our team is at your event and your catering for order {reference} has started. Enjoy your event!"
            ),
            "Afterwards, you can let us know everything went well on your order page:",
        ),
        (Locale::Nl, "serving_on_site") => (
            "Catering begonnen",
            format!(
                "Ons team is bij je evenement en de catering voor bestelling {reference} is begonnen. Veel plezier!"
            ),
            "Is alles goed verlopen? Laat het ons weten op je bestelpagina:",
        ),
        (Locale::Fr, "serving_on_site") => (
            "Service traiteur commencé",
            format!(
                "Notre équipe est à votre événement et le service traiteur de la commande {reference} a commencé. Profitez bien de votre événement !"
            ),
            "Ensuite, vous pouvez nous confirmer que tout s'est bien passé sur la page de votre commande :",
        ),
        (Locale::En, "completed") => (
            "Thank you",
            format!(
                "Your order {reference} is complete. Thank you for ordering from Zain's Treat n More, and enjoy your meal!"
            ),
            "Your order page:",
        ),
        (Locale::Nl, "completed") => (
            "Bedankt",
            format!(
                "Je bestelling {reference} is afgerond. Bedankt dat je bij Zain's Treat n More hebt besteld, en eet smakelijk!"
            ),
            "Je bestelpagina:",
        ),
        (Locale::Fr, "completed") => (
            "Merci",
            format!(
                "Votre commande {reference} est terminée. Merci d'avoir commandé chez Zain's Treat n More, et bon appétit !"
            ),
            "La page de votre commande :",
        ),
        (Locale::En, "cancelled") => (
            "Order cancelled",
            format!(
                "Your order {reference} has been cancelled.\n\nIf you have any questions, or if you already paid, please contact us:\n{contact_url}"
            ),
            "Your order page:",
        ),
        (Locale::Nl, "cancelled") => (
            "Bestelling geannuleerd",
            format!(
                "Je bestelling {reference} is geannuleerd.\n\nHeb je vragen, of heb je al betaald? Neem dan contact met ons op:\n{contact_url}"
            ),
            "Je bestelpagina:",
        ),
        (Locale::Fr, "cancelled") => (
            "Commande annulée",
            format!(
                "Votre commande {reference} a été annulée.\n\nSi vous avez des questions, ou si vous avez déjà payé, contactez-nous :\n{contact_url}"
            ),
            "La page de votre commande :",
        ),
        _ => return None,
    })
}

/// One language's status email text, without the sign-off.
fn status_section(
    locale: Locale,
    customer_name: &str,
    reference: &str,
    status: &str,
    site_url: &str,
    token: Uuid,
) -> Option<(&'static str, String)> {
    let (subject, message, link_label) =
        status_text(locale, status, reference, &contact_url(site_url, locale))?;
    let url = tracking_url(site_url, locale, token);
    Some((
        subject,
        format!(
            "{} {customer_name},\n\n{message}\n\n{link_label}\n{url}",
            locale.greeting()
        ),
    ))
}

/// The email sent when the admin moves an order to `status`, or `None` for
/// statuses that don't email the customer (Phase 22 Decision 6).
pub fn status_email(
    customer_name: &str,
    reference: &str,
    status: &str,
    site_url: &str,
    token: Uuid,
) -> Option<(String, String)> {
    let parts = Locale::ALL
        .into_iter()
        .map(|locale| {
            let (subject, section) =
                status_section(locale, customer_name, reference, status, site_url, token)?;
            Some((locale, subject, section))
        })
        .collect::<Option<Vec<_>>>()?;
    Some(multilingual(&parts, reference))
}

struct InvoiceCopy {
    /// The short subject, joined with the other languages' (copy.md §7).
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
    subject: "Invoice",
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
    subject: "Factuur",
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

const INVOICE_FR: InvoiceCopy = InvoiceCopy {
    subject: "Facture",
    greeting: "Bonjour",
    intro: "Merci pour votre commande ! Voici votre facture.",
    order: "Commande",
    date: "Date",
    pickup: "À retirer",
    delivery_to: "Livraison à",
    served_at: "Lieu",
    total: "Total",
    pay_by_transfer: "Veuillez payer par virement bancaire :",
    account_name: "Titulaire du compte",
    reference: "Référence",
    use_reference: "Merci d'utiliser exactement cette référence, afin que nous puissions retrouver votre paiement.",
    whatsapp_fallback: "Contactez-nous sur WhatsApp pour obtenir nos coordonnées bancaires.",
    verified_only: "Nous commençons à préparer votre commande uniquement après vérification de votre paiement.",
    after_paying: "Après votre paiement, signalez-le-nous sur la page de votre commande :",
};

fn invoice_copy(locale: Locale) -> &'static InvoiceCopy {
    match locale {
        Locale::En => &INVOICE_EN,
        Locale::Nl => &INVOICE_NL,
        Locale::Fr => &INVOICE_FR,
    }
}

/// One language's invoice (Phase 20 Decision 8), without the sign-off.
/// Plain text; no VAT line (KOR).
fn invoice_section(
    locale: Locale,
    order: &OrderResponse,
    placed_at: &str,
    tracking_url: &str,
) -> String {
    let copy = invoice_copy(locale);

    let delivery_line = fulfilment_line(
        copy,
        &order.delivery_type,
        order.delivery_address.as_deref(),
    );

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
        body.push_str(&format!(
            "{} x {name} — €{}\n",
            item.quantity, item.subtotal
        ));
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
            body.push_str(&format!(
                "{}: {}\n\n{}\n\n",
                copy.reference, payment.reference, copy.use_reference
            ));
            if let Some(kvk) = &payment.kvk {
                body.push_str(&format!("KvK: {kvk}\n\n"));
            }
        }
        None => body.push_str(&format!("{}\n\n", copy.whatsapp_fallback)),
    }

    body.push_str(&format!(
        "{}\n\n{}\n{tracking_url}",
        copy.verified_only, copy.after_paying
    ));
    body
}

/// The customer's invoice email, in every language (Phase 24 Decision 11).
pub fn invoice_email(
    order: &OrderResponse,
    placed_at: &str,
    site_url: &str,
    token: Uuid,
) -> (String, String) {
    let parts: Vec<_> = Locale::ALL
        .into_iter()
        .map(|locale| {
            let url = tracking_url(site_url, locale, token);
            (
                locale,
                invoice_copy(locale).subject,
                invoice_section(locale, order, placed_at, &url),
            )
        })
        .collect();
    multilingual(&parts, &order.payment_reference)
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
}

struct QuoteCopy {
    /// The short subject, joined with the other languages' (copy.md §7).
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
    subject: "Catering quote",
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
    subject: "Cateringofferte",
    greeting: "Hoi",
    intro: "Bedankt voor je aanvraag. Hier is onze offerte voor je evenement op",
    guests: "gasten",
    quote: "Offerte",
    notes: "Opmerkingen",
    valid_until: "Deze offerte is geldig tot en met",
    view: "Bekijk de offerte en accepteer of weiger hem hier:",
    after_accepting: "Na het accepteren ontvang je een factuur met onze bankgegevens. We beginnen met de voorbereiding zodra je betaling is gecontroleerd.",
};

const QUOTE_FR: QuoteCopy = QuoteCopy {
    subject: "Devis traiteur",
    greeting: "Bonjour",
    intro: "Merci pour votre demande. Voici notre devis pour votre événement du",
    guests: "invités",
    quote: "Devis",
    notes: "Remarques",
    valid_until: "Ce devis est valable jusqu'au",
    view: "Consultez le devis, puis acceptez-le ou refusez-le ici :",
    after_accepting: "Après acceptation, vous recevrez une facture avec nos coordonnées bancaires. Nous commençons la préparation dès que votre paiement a été vérifié.",
};

fn quote_copy(locale: Locale) -> &'static QuoteCopy {
    match locale {
        Locale::En => &QUOTE_EN,
        Locale::Nl => &QUOTE_NL,
        Locale::Fr => &QUOTE_FR,
    }
}

/// One language's quote (Phase 23 Decision 9), without the sign-off. Plain
/// text; no VAT line (KOR).
fn quote_section(locale: Locale, quote: &QuoteEmail, url: &str) -> String {
    let copy = quote_copy(locale);
    let invoice = invoice_copy(locale);

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
        body.push_str(&format!(
            "{} x {} — €{}\n",
            line.quantity, line.description, line.subtotal
        ));
    }
    body.push_str(&format!("\n{}: €{}\n\n", invoice.total, quote.total));

    if let Some(notes) = quote.notes.map(str::trim).filter(|n| !n.is_empty()) {
        body.push_str(&format!("{}:\n{notes}\n\n", copy.notes));
    }

    body.push_str(&format!(
        "{} {}.\n\n{}\n{url}\n\n{}",
        copy.valid_until,
        quote.valid_until.format("%d-%m-%Y"),
        copy.view,
        copy.after_accepting,
    ));
    body
}

/// The quote sent to the customer when the owner presses "Send", in every
/// language (Phase 24 Decision 11).
pub fn quote_email(quote: &QuoteEmail, site_url: &str, token: Uuid) -> (String, String) {
    let parts: Vec<_> = Locale::ALL
        .into_iter()
        .map(|locale| {
            let url = quote_url(site_url, locale, token);
            (
                locale,
                quote_copy(locale).subject,
                quote_section(locale, quote, &url),
            )
        })
        .collect();
    multilingual(&parts, &format!("#{}", quote.quote_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::payment::PaymentInstructions;
    use crate::routes::orders::OrderItemResponse;
    use rust_decimal::Decimal;

    const SITE: &str = "https://zainstreat.com";
    const TOKEN: Uuid = Uuid::nil();
    const ORDER_URL: &str = "https://zainstreat.com/nl/order/00000000-0000-0000-0000-000000000000";
    const EMAILING: [&str; 6] = [
        "processing",
        "ready_for_collection",
        "out_for_delivery",
        "serving_on_site",
        "completed",
        "cancelled",
    ];

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

    fn status(status: &str) -> Option<(String, String)> {
        status_email("Amina", "ZTM-1042", status, SITE, TOKEN)
    }

    /// The headers of an email body, in order.
    fn headers(body: &str) -> Vec<&str> {
        body.lines()
            .filter(|l| l.starts_with("--- ") && l.ends_with(" ---"))
            .collect()
    }

    #[test]
    fn locale_parse_and_as_str_round_trip() {
        for locale in Locale::ALL {
            assert_eq!(Locale::parse(locale.as_str()), Some(locale));
        }
        assert_eq!(Locale::parse("fr"), Some(Locale::Fr));
        assert_eq!(Locale::parse("de"), None);
    }

    #[test]
    fn every_emailing_status_has_copy_in_every_language() {
        for locale in Locale::ALL {
            for s in EMAILING {
                let (subject, section) =
                    status_section(locale, "Amina", "ZTM-1042", s, SITE, TOKEN)
                        .unwrap_or_else(|| panic!("{s} {locale:?}"));
                assert!(!subject.is_empty(), "{s} {locale:?}");
                let url = tracking_url(SITE, locale, TOKEN);
                assert!(
                    section.contains("ZTM-1042")
                        && section.contains(&url)
                        && section.contains("Amina")
                );
            }
        }
    }

    #[test]
    fn statuses_before_payment_confirmation_send_nothing() {
        for s in ["awaiting_payment", "payment_claimed"] {
            assert!(status(s).is_none());
        }
    }

    #[test]
    fn sections_are_always_english_dutch_french() {
        let (_, body) = status("processing").unwrap();
        assert_eq!(
            headers(&body),
            ["--- English ---", "--- Nederlands ---", "--- Français ---"]
        );
        assert!(body.starts_with("--- English ---\n\nHi Amina,"));
    }

    #[test]
    fn each_section_links_to_its_own_language() {
        let (_, body) = status("cancelled").unwrap();
        let sections: Vec<&str> = body.split("\n--- ").collect();
        assert_eq!(sections.len(), 3);
        for (section, locale) in sections.iter().zip(Locale::ALL) {
            assert!(
                section.contains(&tracking_url(SITE, locale, TOKEN)),
                "{locale:?}"
            );
            assert!(section.contains(&contact_url(SITE, locale)), "{locale:?}");
            for other in Locale::ALL.into_iter().filter(|l| *l != locale) {
                assert!(
                    !section.contains(&format!("/{}/", other.as_str())),
                    "{locale:?} links to {other:?}"
                );
            }
        }
    }

    #[test]
    fn sign_off_appears_once_at_the_end() {
        let (_, body) = status("completed").unwrap();
        assert_eq!(body.matches(SIGN_OFF).count(), 1);
        assert!(body.ends_with(&format!("\n\n{SIGN_OFF}")));
        let (_, invoice) = invoice_email(&order(None), "31-10-2026 14:05", SITE, TOKEN);
        assert_eq!(invoice.matches(SIGN_OFF).count(), 1);
    }

    #[test]
    fn subject_joins_all_three_in_body_order() {
        let (subject, _) = status("processing").unwrap();
        assert_eq!(
            subject,
            "Payment confirmed / Betaling bevestigd / Paiement confirmé — ZTM-1042"
        );
        let (subject, _) = invoice_email(&order(None), "31-10-2026 14:05", SITE, TOKEN);
        assert_eq!(subject, "Invoice / Factuur / Facture — ZTM-1042");
    }

    #[test]
    fn cancellation_points_to_the_contact_page() {
        let (_, section) =
            status_section(Locale::Nl, "Amina", "ZTM-1042", "cancelled", SITE, TOKEN).unwrap();
        assert!(section.contains("https://zainstreat.com/nl/contact"));
    }

    #[test]
    fn dutch_status_section_is_dutch() {
        let (subject, section) = status_section(
            Locale::Nl,
            "Amina",
            "ZTM-1042",
            "out_for_delivery",
            SITE,
            TOKEN,
        )
        .unwrap();
        assert_eq!(subject, "Onderweg");
        assert_eq!(
            section,
            format!(
                "Hoi Amina,\n\nJe bestelling ZTM-1042 is onderweg naar je toe.\n\nIs hij aangekomen? Laat het ons weten op je bestelpagina:\n{ORDER_URL}"
            )
        );
    }

    /// Pinned to the Phase 24 copy.md §2.
    #[test]
    fn french_status_section_matches_the_copy() {
        let (subject, section) =
            status_section(Locale::Fr, "Amina", "ZTM-1042", "processing", SITE, TOKEN).unwrap();
        assert_eq!(subject, "Paiement confirmé");
        assert_eq!(
            section,
            "Bonjour Amina,\n\nBonne nouvelle : nous avons bien reçu votre paiement pour la commande ZTM-1042, et nous préparons maintenant votre repas.\n\n\
             Suivez votre commande ici :\nhttps://zainstreat.com/fr/order/00000000-0000-0000-0000-000000000000"
        );
    }

    /// The English invoice section is Phase 20's text, minus the sign-off,
    /// which `multilingual` adds once.
    #[test]
    fn english_invoice_keeps_the_phase_20_text() {
        let body = invoice_section(
            Locale::En,
            &order(Some(bank())),
            "31-10-2026 14:05",
            "https://zainstreat.com/en/order/x",
        );
        assert_eq!(
            body,
            "Hi Amina,\n\nThanks for your order! Here is your invoice.\n\nOrder: ZTM-1042\nDate: 31-10-2026 14:05\nPickup\n\n\
             2 x Jollof Rice (Large) — €24.00\n1 x Puff Puff — €6.50\n\nTotal: €30.50\n\n\
             Please pay by bank transfer:\nAccount name: Zain's Treat n More\nIBAN: NL91ABNA0417164300\n\
             Reference: ZTM-1042\n\nPlease use this exact reference so we can match your payment.\n\n\
             We only start preparing your order once your payment has been verified.\n\n\
             After paying, let us know on your order page:\nhttps://zainstreat.com/en/order/x"
        );
    }

    #[test]
    fn dutch_invoice_without_bank_details_falls_back_to_whatsapp() {
        let body = invoice_section(Locale::Nl, &order(None), "31-10-2026 14:05", ORDER_URL);
        assert!(body.starts_with("Hoi Amina,"));
        assert!(body.contains("Afhalen") && body.contains("Totaal: €30.50"));
        assert!(body.contains("via WhatsApp") && !body.contains("IBAN"));
    }

    /// Pinned to the Phase 24 copy.md §3.
    #[test]
    fn french_invoice_matches_the_copy() {
        let body = invoice_section(
            Locale::Fr,
            &order(Some(bank())),
            "31-10-2026 14:05",
            "https://zainstreat.com/fr/order/x",
        );
        assert_eq!(
            body,
            "Bonjour Amina,\n\nMerci pour votre commande ! Voici votre facture.\n\nCommande: ZTM-1042\nDate: 31-10-2026 14:05\nÀ retirer\n\n\
             2 x Jollof Rice (Large) — €24.00\n1 x Puff Puff — €6.50\n\nTotal: €30.50\n\n\
             Veuillez payer par virement bancaire :\nTitulaire du compte: Zain's Treat n More\nIBAN: NL91ABNA0417164300\n\
             Référence: ZTM-1042\n\nMerci d'utiliser exactement cette référence, afin que nous puissions retrouver votre paiement.\n\n\
             Nous commençons à préparer votre commande uniquement après vérification de votre paiement.\n\n\
             Après votre paiement, signalez-le-nous sur la page de votre commande :\nhttps://zainstreat.com/fr/order/x"
        );
    }

    #[test]
    fn invoice_email_has_every_language_with_bank_details() {
        let (_, body) = invoice_email(&order(Some(bank())), "31-10-2026 14:05", SITE, TOKEN);
        assert_eq!(
            headers(&body),
            ["--- English ---", "--- Nederlands ---", "--- Français ---"]
        );
        assert_eq!(body.matches("IBAN: NL91ABNA0417164300").count(), 3);
    }

    #[test]
    fn on_site_invoice_shows_where_the_food_is_served() {
        let mut on_site = order(None);
        on_site.delivery_type = "on_site".into();
        on_site.delivery_address = Some("Zaal De Linde, Utrecht".into());
        let (_, body) = invoice_email(&on_site, "31-10-2026 14:05", SITE, TOKEN);
        assert!(body.contains("Served at: Zaal De Linde, Utrecht"));
        assert!(body.contains("Locatie: Zaal De Linde, Utrecht"));
        assert!(body.contains("Lieu: Zaal De Linde, Utrecht"));
    }

    fn quote_lines() -> Vec<QuoteEmailLine<'static>> {
        vec![
            QuoteEmailLine {
                description: "Jollof rice per guest",
                quantity: 80,
                subtotal: Decimal::new(96000, 2),
            },
            QuoteEmailLine {
                description: "Serving staff",
                quantity: 2,
                subtotal: Decimal::new(25000, 2),
            },
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
        }
    }

    /// Pinned to the Phase 23 copy.md §1, minus the sign-off.
    #[test]
    fn english_quote_section_matches_the_approved_copy() {
        let lines = quote_lines();
        let body = quote_section(
            Locale::En,
            &quote(&lines, Some("Halal, no pork.")),
            "https://zainstreat.com/en/quote/x",
        );
        assert_eq!(
            body,
            "Hi Amina,\n\nThank you for your enquiry. Here is our quote for your event on 05-12-2026 (80 guests).\n\n\
             Quote: #12\nServed at: Zaal De Linde, Utrecht\n\n\
             80 x Jollof rice per guest — €960.00\n2 x Serving staff — €250.00\n\nTotal: €1210.00\n\n\
             Notes:\nHalal, no pork.\n\n\
             This quote is valid until 20-11-2026.\n\n\
             View the quote and accept or decline it here:\nhttps://zainstreat.com/en/quote/x\n\n\
             Once you accept, you'll receive an invoice with our bank details. We start preparing once your payment has been verified."
        );
    }

    #[test]
    fn dutch_quote_section_is_dutch_and_skips_empty_notes() {
        let lines = quote_lines();
        let body = quote_section(
            Locale::Nl,
            &quote(&lines, Some("  ")),
            "https://zainstreat.com/nl/quote/x",
        );
        assert!(body.starts_with("Hoi Amina,"));
        assert!(body.contains("(80 gasten)") && body.contains("Locatie: Zaal De Linde, Utrecht"));
        assert!(body.contains("Totaal: €1210.00") && body.contains("geldig tot en met 20-11-2026"));
        assert!(body.contains("/nl/quote/x") && !body.contains("Opmerkingen"));
    }

    /// Pinned to the Phase 24 copy.md §4.
    #[test]
    fn french_quote_section_matches_the_copy() {
        let lines = quote_lines();
        let body = quote_section(
            Locale::Fr,
            &quote(&lines, Some("Halal, sans porc.")),
            "https://zainstreat.com/fr/quote/x",
        );
        assert_eq!(
            body,
            "Bonjour Amina,\n\nMerci pour votre demande. Voici notre devis pour votre événement du 05-12-2026 (80 invités).\n\n\
             Devis: #12\nLieu: Zaal De Linde, Utrecht\n\n\
             80 x Jollof rice per guest — €960.00\n2 x Serving staff — €250.00\n\nTotal: €1210.00\n\n\
             Remarques:\nHalal, sans porc.\n\n\
             Ce devis est valable jusqu'au 20-11-2026.\n\n\
             Consultez le devis, puis acceptez-le ou refusez-le ici :\nhttps://zainstreat.com/fr/quote/x\n\n\
             Après acceptation, vous recevrez une facture avec nos coordonnées bancaires. Nous commençons la préparation dès que votre paiement a été vérifié."
        );
    }

    #[test]
    fn quote_email_joins_subjects_and_links_each_language() {
        let lines = quote_lines();
        let (subject, body) = quote_email(&quote(&lines, None), SITE, TOKEN);
        assert_eq!(
            subject,
            "Catering quote / Cateringofferte / Devis traiteur — #12"
        );
        for locale in Locale::ALL {
            assert!(body.contains(&quote_url(SITE, locale, TOKEN)));
        }
        assert_eq!(body.matches(SIGN_OFF).count(), 1);
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
            tracking_url("https://zainstreat.com", Locale::Fr, Uuid::nil()),
            "https://zainstreat.com/fr/order/00000000-0000-0000-0000-000000000000"
        );
    }
}
