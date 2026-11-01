use chrono::NaiveDate;

/// Mirrors the CHECK constraint on `catering_quotes.status` in
/// `migrations/20261101132739_catering_quotes.up.sql`;
/// `tests/quote_status_schema.rs` fails if the two drift. "Expired" is
/// never stored: it's a `sent` quote past `valid_until` (Phase 23
/// Decision 3).
pub const QUOTE_STATUSES: [&str; 4] = ["draft", "sent", "accepted", "declined"];

/// Anything not yet accepted can be edited; saving moves it back to
/// `draft` (Phase 23 Decision 3).
pub fn admin_can_edit(status: &str) -> bool {
    status != "accepted"
}

/// `today` is the Europe/Amsterdam date, read from the DB.
pub fn is_expired(status: &str, valid_until: NaiveDate, today: NaiveDate) -> bool {
    status == "sent" && valid_until < today
}

pub fn customer_can_accept(status: &str, valid_until: NaiveDate, today: NaiveDate) -> bool {
    status == "sent" && !is_expired(status, valid_until, today)
}

/// An expired quote can still be declined (Phase 23 Decision 4).
pub fn customer_can_decline(status: &str) -> bool {
    status == "sent"
}

/// What admin and the quote page show: the stored status, or `expired`.
pub fn display_status(status: &str, valid_until: NaiveDate, today: NaiveDate) -> &'static str {
    if is_expired(status, valid_until, today) {
        return "expired";
    }
    match status {
        "draft" => "draft",
        "sent" => "sent",
        "accepted" => "accepted",
        _ => "declined",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, day).unwrap()
    }

    #[test]
    fn only_a_sent_quote_in_date_can_be_accepted() {
        let today = date(10);
        for status in QUOTE_STATUSES {
            assert_eq!(customer_can_accept(status, date(10), today), status == "sent", "{status}");
            assert!(!customer_can_accept(status, date(9), today), "{status} expired");
        }
    }

    #[test]
    fn the_valid_until_day_itself_is_still_valid() {
        assert!(customer_can_accept("sent", date(10), date(10)));
        assert!(!customer_can_accept("sent", date(10), date(11)));
    }

    #[test]
    fn only_a_sent_quote_can_be_declined_even_when_expired() {
        for status in QUOTE_STATUSES {
            assert_eq!(customer_can_decline(status), status == "sent", "{status}");
        }
    }

    #[test]
    fn everything_but_accepted_is_editable() {
        for status in QUOTE_STATUSES {
            assert_eq!(admin_can_edit(status), status != "accepted", "{status}");
        }
    }

    #[test]
    fn display_status_marks_only_sent_quotes_as_expired() {
        let today = date(10);
        for status in QUOTE_STATUSES {
            assert_eq!(display_status(status, date(10), today), status);
            let past = display_status(status, date(9), today);
            assert_eq!(past, if status == "sent" { "expired" } else { status });
        }
    }
}
