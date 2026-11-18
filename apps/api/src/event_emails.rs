//! Customer emails for event bookings (Phase 31 Decision 19). Pure
//! functions returning `(subject, body)`, like `order_emails.rs`, and the
//! same layout: one email with an English, Dutch and French section.
//! Owner emails stay in `events.rs`, in English.

use crate::event_options::{GuestChoice, OptionGroup, choices_summary};
use crate::order_emails::Locale;
use crate::payment::{PaymentInstructions, TikkieInstructions};
use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;
use uuid::Uuid;

const SIGN_OFF: &str = "— Zain's Treat n More";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BookingEmailKind {
    /// Meals booked, seats held until payment. `offered` is a waitlisted
    /// booking the owner offered seats to.
    AwaitingPayment { offered: bool },
    /// Paid, or a free RSVP: carries the address.
    Confirmed { offered: bool },
    Waitlisted,
    /// Unpaid when the hold ran out (Decision 8).
    Released,
    Cancelled,
}

/// One guest's plate, or `None` for attending without a meal.
pub struct GuestLine {
    /// The English name snapshotted at booking time.
    pub plate_name: Option<String>,
    pub unit_price: Option<Decimal>,
    /// The plate's per-locale content while it still exists, to show the
    /// name in the reader's language.
    pub plate_content: Option<serde_json::Value>,
    /// The guest's picks (Phase 32), and the plate's groups to translate them.
    pub choices: Vec<GuestChoice>,
    pub plate_options: Vec<OptionGroup>,
}

impl GuestLine {
    pub fn name_in(&self, locale: Locale) -> Option<String> {
        let snapshot = self.plate_name.clone()?;
        Some(
            self.plate_content
                .as_ref()
                .map(|content| crate::events::localized(content, locale, "name"))
                .filter(|name| !name.is_empty())
                .unwrap_or(snapshot),
        )
    }

    /// "Fried plantain, Chicken", or empty when the plate has no options.
    pub fn choices_in(&self, locale: Locale) -> String {
        choices_summary(&self.choices, &self.plate_options, locale)
    }
}

/// One language's event title and terms.
pub struct LocalizedEvent {
    pub title: String,
    pub terms: String,
}

pub struct BookingEmail<'a> {
    pub kind: BookingEmailKind,
    pub customer_name: &'a str,
    pub token: Uuid,
    /// Indexed like `Locale::ALL`.
    pub event: [LocalizedEvent; 3],
    /// `DD-MM-YYYY HH:MM`, Europe/Amsterdam.
    pub starts_at: &'a str,
    pub area: &'a str,
    /// Only rendered for `Confirmed`.
    pub address: &'a str,
    pub party_size: i32,
    pub guests: &'a [GuestLine],
    pub total: Decimal,
    pub hold_expires_at: Option<&'a str>,
    pub payment: Option<&'a PaymentInstructions>,
    /// Shown first while valid (Phase 34 Decision 17).
    pub tikkie: Option<&'a TikkieInstructions>,
}

struct Copy {
    greeting: &'static str,
    subject_awaiting: &'static str,
    subject_confirmed: &'static str,
    subject_waitlisted: &'static str,
    subject_released: &'static str,
    subject_cancelled: &'static str,
    intro_awaiting: &'static str,
    intro_offered: &'static str,
    intro_confirmed: &'static str,
    intro_waitlisted: &'static str,
    intro_released: &'static str,
    intro_cancelled: &'static str,
    held_until: &'static str,
    when: &'static str,
    area: &'static str,
    address: &'static str,
    guests: &'static str,
    guest: &'static str,
    no_meal: &'static str,
    total: &'static str,
    pay_by_transfer: &'static str,
    pay_by_tikkie: &'static str,
    tikkie_amount: &'static str,
    or_by_transfer: &'static str,
    months: [&'static str; 12],
    account_name: &'static str,
    reference: &'static str,
    use_reference: &'static str,
    whatsapp_fallback: &'static str,
    after_paying: &'static str,
    terms: &'static str,
    view_booking: &'static str,
}

const EN: Copy = Copy {
    greeting: "Hi",
    subject_awaiting: "Your booking — payment request",
    subject_confirmed: "You're confirmed",
    subject_waitlisted: "You're on the waitlist",
    subject_released: "Booking released",
    subject_cancelled: "Booking cancelled",
    intro_awaiting: "Thanks for booking a seat at {title}! Entry is free, and your meals are reserved once payment is received.",
    intro_offered: "Good news: seats have opened up at {title}, and they're yours. Your meals are reserved once payment is received.",
    intro_confirmed: "You're confirmed for {title}. We can't wait to welcome you!",
    intro_waitlisted: "{title} is full right now, so you're on the waitlist for {party} seat(s). We'll email you as soon as seats open up.",
    intro_released: "We didn't receive payment for your booking at {title} in time, so your seats have been released. If you've already paid, please contact us and we'll sort it out.",
    intro_cancelled: "Your booking at {title} has been cancelled. If you already paid, we'll contact you about your refund.",
    held_until: "Your seats are held until",
    when: "When",
    area: "Where",
    address: "Address",
    guests: "Your party",
    guest: "Guest",
    no_meal: "Attending without a meal",
    total: "Total",
    pay_by_transfer: "Please pay by bank transfer:",
    pay_by_tikkie: "Pay with Tikkie:",
    tikkie_amount: "Enter €{amount} in Tikkie. The link is valid until {date}.",
    or_by_transfer: "Or pay by bank transfer:",
    months: [
        "January", "February", "March", "April", "May", "June",
        "July", "August", "September", "October", "November", "December",
    ],
    account_name: "Account name",
    reference: "Reference",
    use_reference: "Please use this exact reference so we can match your payment.",
    whatsapp_fallback: "Please contact us on WhatsApp for our bank details.",
    after_paying: "After paying, let us know on your booking page.",
    terms: "Cancellation and refunds",
    view_booking: "Your booking page:",
};

const NL: Copy = Copy {
    greeting: "Hoi",
    subject_awaiting: "Je reservering — betaalverzoek",
    subject_confirmed: "Je bent bevestigd",
    subject_waitlisted: "Je staat op de wachtlijst",
    subject_released: "Reservering vrijgegeven",
    subject_cancelled: "Reservering geannuleerd",
    intro_awaiting: "Bedankt voor je reservering voor {title}! De toegang is gratis, en je maaltijden zijn gereserveerd zodra je betaling binnen is.",
    intro_offered: "Goed nieuws: er zijn plekken vrijgekomen voor {title}, en ze zijn voor jou. Je maaltijden zijn gereserveerd zodra je betaling binnen is.",
    intro_confirmed: "Je bent bevestigd voor {title}. We kijken ernaar uit je te verwelkomen!",
    intro_waitlisted: "{title} is op dit moment vol, dus je staat op de wachtlijst voor {party} plek(ken). We mailen je zodra er plekken vrijkomen.",
    intro_released: "We hebben je betaling voor {title} niet op tijd ontvangen, dus je plekken zijn vrijgegeven. Heb je al betaald? Neem dan contact met ons op, dan lossen we het op.",
    intro_cancelled: "Je reservering voor {title} is geannuleerd. Heb je al betaald? Dan nemen we contact met je op over de terugbetaling.",
    held_until: "Je plekken worden vastgehouden tot",
    when: "Wanneer",
    area: "Waar",
    address: "Adres",
    guests: "Je gezelschap",
    guest: "Gast",
    no_meal: "Aanwezig zonder maaltijd",
    total: "Totaal",
    pay_by_transfer: "Betaal via bankoverschrijving:",
    pay_by_tikkie: "Betaal met Tikkie:",
    tikkie_amount: "Vul €{amount} in bij Tikkie. De link is geldig t/m {date}.",
    or_by_transfer: "Of betaal via bankoverschrijving:",
    months: [
        "januari", "februari", "maart", "april", "mei", "juni",
        "juli", "augustus", "september", "oktober", "november", "december",
    ],
    account_name: "Naam rekeninghouder",
    reference: "Kenmerk",
    use_reference: "Gebruik precies dit kenmerk, zodat we je betaling kunnen koppelen.",
    whatsapp_fallback: "Neem contact met ons op via WhatsApp voor onze bankgegevens.",
    after_paying: "Heb je betaald? Laat het ons weten op je reserveringspagina.",
    terms: "Annuleren en terugbetaling",
    view_booking: "Je reserveringspagina:",
};

const FR: Copy = Copy {
    greeting: "Bonjour",
    subject_awaiting: "Votre réservation — demande de paiement",
    subject_confirmed: "Votre place est confirmée",
    subject_waitlisted: "Vous êtes sur la liste d'attente",
    subject_released: "Réservation libérée",
    subject_cancelled: "Réservation annulée",
    intro_awaiting: "Merci d'avoir réservé pour {title} ! L'entrée est gratuite, et vos repas sont réservés dès réception de votre paiement.",
    intro_offered: "Bonne nouvelle : des places se sont libérées pour {title}, et elles sont pour vous. Vos repas sont réservés dès réception de votre paiement.",
    intro_confirmed: "Votre place est confirmée pour {title}. Nous avons hâte de vous accueillir !",
    intro_waitlisted: "{title} est complet pour le moment, vous êtes donc sur la liste d'attente pour {party} place(s). Nous vous écrirons dès qu'une place se libère.",
    intro_released: "Nous n'avons pas reçu votre paiement pour {title} à temps, vos places ont donc été libérées. Si vous avez déjà payé, contactez-nous et nous arrangerons cela.",
    intro_cancelled: "Votre réservation pour {title} a été annulée. Si vous avez déjà payé, nous vous contacterons pour le remboursement.",
    held_until: "Vos places sont réservées jusqu'au",
    when: "Quand",
    area: "Où",
    address: "Adresse",
    guests: "Votre groupe",
    guest: "Invité",
    no_meal: "Présent sans repas",
    total: "Total",
    pay_by_transfer: "Veuillez payer par virement bancaire :",
    pay_by_tikkie: "Payer avec Tikkie :",
    tikkie_amount: "Saisissez €{amount} dans Tikkie. Le lien est valable jusqu'au {date}.",
    or_by_transfer: "Ou payez par virement bancaire :",
    months: [
        "janvier", "février", "mars", "avril", "mai", "juin",
        "juillet", "août", "septembre", "octobre", "novembre", "décembre",
    ],
    account_name: "Titulaire du compte",
    reference: "Référence",
    use_reference: "Merci d'utiliser exactement cette référence, afin que nous puissions retrouver votre paiement.",
    whatsapp_fallback: "Contactez-nous sur WhatsApp pour obtenir nos coordonnées bancaires.",
    after_paying: "Après votre paiement, signalez-le-nous sur la page de votre réservation.",
    terms: "Annulation et remboursement",
    view_booking: "La page de votre réservation :",
};

fn copy(locale: Locale) -> &'static Copy {
    match locale {
        Locale::En => &EN,
        Locale::Nl => &NL,
        Locale::Fr => &FR,
    }
}

/// "2 December", "2 december", "2 décembre".
fn day_month(copy: &Copy, date: NaiveDate) -> String {
    format!("{} {}", date.day(), copy.months[date.month0() as usize])
}

/// The guest's booking page, in their language (Decision 11).
pub fn booking_url(site_url: &str, locale: Locale, token: Uuid) -> String {
    format!("{site_url}/{}/events/booking/{token}", locale.as_str())
}

fn subject(copy: &Copy, kind: BookingEmailKind) -> &'static str {
    match kind {
        BookingEmailKind::AwaitingPayment { .. } => copy.subject_awaiting,
        BookingEmailKind::Confirmed { .. } => copy.subject_confirmed,
        BookingEmailKind::Waitlisted => copy.subject_waitlisted,
        BookingEmailKind::Released => copy.subject_released,
        BookingEmailKind::Cancelled => copy.subject_cancelled,
    }
}

fn section(locale: Locale, email: &BookingEmail, event: &LocalizedEvent, site_url: &str) -> String {
    let copy = copy(locale);
    let intro = match email.kind {
        BookingEmailKind::AwaitingPayment { offered: false } => copy.intro_awaiting,
        BookingEmailKind::AwaitingPayment { offered: true } => copy.intro_offered,
        BookingEmailKind::Confirmed { .. } => copy.intro_confirmed,
        BookingEmailKind::Waitlisted => copy.intro_waitlisted,
        BookingEmailKind::Released => copy.intro_released,
        BookingEmailKind::Cancelled => copy.intro_cancelled,
    }
    .replace("{title}", &event.title)
    .replace("{party}", &email.party_size.to_string());

    let mut body = format!(
        "{} {},\n\n{intro}\n\n{}: {}\n{}: {}\n",
        copy.greeting, email.customer_name, copy.when, email.starts_at, copy.area, email.area,
    );
    if matches!(email.kind, BookingEmailKind::Confirmed { .. }) {
        body.push_str(&format!("{}: {}\n", copy.address, email.address));
    }

    if !matches!(email.kind, BookingEmailKind::Released | BookingEmailKind::Cancelled) {
        body.push_str(&format!("\n{}:\n", copy.guests));
        for (index, guest) in email.guests.iter().enumerate() {
            let line = match (guest.name_in(locale), guest.unit_price) {
                (Some(name), Some(price)) => match guest.choices_in(locale) {
                    choices if choices.is_empty() => format!("{name} — €{price}"),
                    choices => format!("{name} ({choices}) — €{price}"),
                },
                _ => copy.no_meal.to_string(),
            };
            body.push_str(&format!("{} {}: {line}\n", copy.guest, index + 1));
        }
        if email.total > Decimal::ZERO {
            body.push_str(&format!("{}: €{}\n", copy.total, email.total));
        }
    }

    if let BookingEmailKind::AwaitingPayment { .. } = email.kind {
        body.push('\n');
        if let Some(tikkie) = email.tikkie {
            let amount = copy
                .tikkie_amount
                .replace("{amount}", &tikkie.amount.to_string())
                .replace("{date}", &day_month(copy, tikkie.valid_until));
            body.push_str(&format!("{} {}\n{amount}\n\n", copy.pay_by_tikkie, tikkie.url));
        }
        match email.payment {
            Some(payment) => {
                let heading = if email.tikkie.is_some() { copy.or_by_transfer } else { copy.pay_by_transfer };
                body.push_str(&format!(
                    "{heading}\n{}: {}\nIBAN: {}\n",
                    copy.account_name, payment.account_name, payment.iban,
                ));
                if let Some(bic) = &payment.bic {
                    body.push_str(&format!("BIC: {bic}\n"));
                }
                body.push_str(&format!(
                    "{}: {}\n\n{}\n",
                    copy.reference, payment.reference, copy.use_reference
                ));
            }
            None if email.tikkie.is_some() => {}
            None => body.push_str(&format!("{}\n", copy.whatsapp_fallback)),
        }
        if let Some(deadline) = email.hold_expires_at {
            body.push_str(&format!("\n{} {deadline}.\n", copy.held_until));
        }
        body.push_str(&format!("{}\n", copy.after_paying));
        if !event.terms.is_empty() {
            body.push_str(&format!("\n{}:\n{}\n", copy.terms, event.terms));
        }
    }

    body.push_str(&format!(
        "\n{}\n{}",
        copy.view_booking,
        booking_url(site_url, locale, email.token)
    ));
    body
}

/// The guest's email for `email.kind`, in every language.
pub fn booking_email(email: &BookingEmail, site_url: &str) -> (String, String) {
    let subject_text = Locale::ALL
        .iter()
        .map(|locale| subject(copy(*locale), email.kind))
        .collect::<Vec<_>>()
        .join(" / ");
    let body = Locale::ALL
        .iter()
        .zip(email.event.iter())
        .map(|(locale, event)| {
            format!("--- {} ---\n\n{}", locale.name(), section(*locale, email, event, site_url))
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    (
        format!("{subject_text} — {}", email.event[0].title),
        format!("{body}\n\n{SIGN_OFF}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event() -> [LocalizedEvent; 3] {
        ["The Abula Experience", "De Abula-ervaring", "L'expérience Abula"].map(|title| {
            LocalizedEvent { title: title.to_string(), terms: "No refunds after 20 December.".to_string() }
        })
    }

    fn email(kind: BookingEmailKind) -> (String, String) {
        email_with(kind, None)
    }

    fn email_with(kind: BookingEmailKind, tikkie: Option<&TikkieInstructions>) -> (String, String) {
        let guests = [
            GuestLine {
                plate_name: Some("Abula".into()),
                unit_price: Some(Decimal::new(1500, 2)),
                plate_content: Some(serde_json::json!({ "en": { "name": "Abula" }, "fr": { "name": "Abula maison" } })),
                choices: Vec::new(),
                plate_options: Vec::new(),
            },
            GuestLine {
                plate_name: None,
                unit_price: None,
                plate_content: None,
                choices: Vec::new(),
                plate_options: Vec::new(),
            },
            GuestLine {
                plate_name: Some("Jollof".into()),
                unit_price: Some(Decimal::new(1500, 2)),
                plate_content: None,
                choices: vec![
                    GuestChoice { group: "side".into(), choice: "plantain".into(), label: "Fried plantain".into() },
                    GuestChoice { group: "protein".into(), choice: "fish".into(), label: "Fish".into() },
                ],
                plate_options: crate::event_options::parse_groups(&serde_json::json!([
                    {"key": "side", "label": {"en": "Side"}, "choices": [
                        {"key": "plantain", "label": {"en": "Fried plantain", "fr": "Bananes plantain frites"}},
                        {"key": "salad", "label": {"en": "Salad"}}]}
                ])),
            },
        ];
        let payment = PaymentInstructions {
            account_name: "Zain's Treat n More".into(),
            iban: "NL91ABNA0417164300".into(),
            bic: None,
            kvk: None,
            reference: "ZTM-7".into(),
        };
        booking_email(
            &BookingEmail {
                kind,
                customer_name: "Amina",
                token: Uuid::nil(),
                event: event(),
                starts_at: "27-12-2026 14:00",
                area: "A home in Utrecht",
                address: "Secret Street 1",
                party_size: 2,
                guests: &guests,
                total: Decimal::new(1500, 2),
                hold_expires_at: Some("19-11-2026 10:00"),
                payment: Some(&payment),
                tikkie,
            },
            "https://example.com",
        )
    }

    #[test]
    fn address_only_in_the_confirmation() {
        assert!(email(BookingEmailKind::Confirmed { offered: false }).1.contains("Secret Street 1"));
        for kind in [
            BookingEmailKind::AwaitingPayment { offered: false },
            BookingEmailKind::Waitlisted,
            BookingEmailKind::Released,
            BookingEmailKind::Cancelled,
        ] {
            assert!(!email(kind).1.contains("Secret Street 1"), "{kind:?}");
        }
    }

    #[test]
    fn payment_request_has_bank_details_terms_and_deadline_in_every_language() {
        let (subject, body) = email(BookingEmailKind::AwaitingPayment { offered: false });
        assert!(subject.ends_with("— The Abula Experience"));
        assert_eq!(body.matches("ZTM-7").count(), 3);
        assert_eq!(body.matches("19-11-2026 10:00").count(), 3);
        assert_eq!(body.matches("No refunds after 20 December.").count(), 3);
        assert!(body.contains("/en/events/booking/"));
        assert!(body.contains("/nl/events/booking/"));
        assert!(body.contains("/fr/events/booking/"));
        assert!(body.contains("De Abula-ervaring"));
    }

    #[test]
    fn lists_each_guest() {
        let (_, body) = email(BookingEmailKind::Confirmed { offered: false });
        assert!(body.contains("Guest 1: Abula — €15.00"));
        assert!(body.contains("Guest 2: Attending without a meal"));
        assert!(body.contains("Gast 2: Aanwezig zonder maaltijd"));
        assert!(body.contains("Invité 1: Abula maison — €15.00"), "translated from the plate");
        assert!(body.contains("Gast 1: Abula — €15.00"), "nl falls back to English");
    }

    #[test]
    fn lists_each_guests_choices_translated() {
        let (_, body) = email(BookingEmailKind::Confirmed { offered: false });
        assert!(body.contains("Guest 3: Jollof (Fried plantain, Fish) — €15.00"));
        assert!(
            body.contains("Invité 3: Jollof (Bananes plantain frites, Fish) — €15.00"),
            "translated while the choice exists, else the English snapshot"
        );
    }

    fn tikkie() -> TikkieInstructions {
        TikkieInstructions {
            url: "https://tikkie.me/pay/test123".into(),
            valid_until: NaiveDate::from_ymd_opt(2026, 12, 2).unwrap(),
            amount: Decimal::new(4500, 2),
        }
    }

    #[test]
    fn tikkie_comes_first_with_the_amount_in_every_language() {
        let tikkie = tikkie();
        let (_, body) = email_with(BookingEmailKind::AwaitingPayment { offered: false }, Some(&tikkie));
        assert_eq!(body.matches("https://tikkie.me/pay/test123").count(), 3);
        assert!(body.contains("Pay with Tikkie: https://tikkie.me/pay/test123\nEnter €45.00 in Tikkie. The link is valid until 2 December."));
        assert!(body.contains("Vul €45.00 in bij Tikkie. De link is geldig t/m 2 december."));
        assert!(body.contains("Saisissez €45.00 dans Tikkie. Le lien est valable jusqu'au 2 décembre."));
        assert!(body.contains("Or pay by bank transfer:"));
        assert!(!body.contains("Please pay by bank transfer:"));
        let tikkie_at = body.find("Pay with Tikkie").unwrap();
        assert!(tikkie_at < body.find("NL91ABNA0417164300").unwrap(), "Tikkie before the bank details");
        assert_eq!(body.matches("ZTM-7").count(), 3, "bank details stay");
    }

    #[test]
    fn no_tikkie_leaves_the_bank_block_as_it_was() {
        let (_, body) = email(BookingEmailKind::AwaitingPayment { offered: false });
        assert!(!body.contains("Tikkie"));
        assert!(body.contains("Please pay by bank transfer:"));
    }

    #[test]
    fn tikkie_only_in_the_payment_request() {
        let tikkie = tikkie();
        let (_, body) = email_with(BookingEmailKind::Confirmed { offered: false }, Some(&tikkie));
        assert!(!body.contains("Tikkie"));
    }
}
