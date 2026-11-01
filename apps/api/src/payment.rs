use serde::Serialize;
use std::sync::Arc;

/// The business's bank account, shown on every invoice (Phase 20
/// requirement.md Decision 5).
pub struct BankDetails {
    pub account_name: String,
    pub iban: String,
    pub bic: Option<String>,
    /// Chamber of Commerce number. Optional: KOR businesses don't have to
    /// put it on an invoice, but the client may want it (Phase 20 open risk 2).
    pub kvk: Option<String>,
}

#[derive(Clone)]
pub struct PaymentConfig {
    bank: Option<Arc<BankDetails>>,
}

#[derive(Debug, Serialize)]
pub struct PaymentInstructions {
    pub account_name: String,
    pub iban: String,
    pub bic: Option<String>,
    pub kvk: Option<String>,
    pub reference: String,
}

/// Derived from the id, never stored, so it can't drift (Decision 6).
pub fn payment_reference(order_id: i64) -> String {
    format!("ZTM-{order_id}")
}

/// Strips spaces and upper-cases, then checks the ISO 13616 mod-97 checksum.
/// `None` for anything that isn't a valid IBAN, so a typo in the server
/// `.env` can never put a wrong account in front of a customer (Phase 20
/// open risk 1).
pub fn normalize_iban(raw: &str) -> Option<String> {
    let iban: String = raw
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_uppercase();

    let well_formed = (15..=34).contains(&iban.len())
        && iban.chars().all(|c| c.is_ascii_alphanumeric())
        && iban[..2].chars().all(|c| c.is_ascii_alphabetic())
        && iban[2..4].chars().all(|c| c.is_ascii_digit());
    if !well_formed {
        return None;
    }

    // Move the country code and check digits to the end, read letters as
    // A=10..Z=35, and take the whole number mod 97 one digit at a time.
    let remainder = iban[4..]
        .chars()
        .chain(iban[..4].chars())
        .fold(0u32, |acc, c| {
            let value = c.to_digit(36).unwrap_or(0);
            if value >= 10 {
                (acc * 100 + value) % 97
            } else {
                (acc * 10 + value) % 97
            }
        });

    (remainder == 1).then_some(iban)
}

impl PaymentConfig {
    pub fn new(bank: Option<BankDetails>) -> Self {
        Self {
            bank: bank.map(Arc::new),
        }
    }

    pub fn from_env() -> Self {
        // Blank counts as unset, same as `EmailConfig::from_env`.
        let non_blank = |key: &str| std::env::var(key).ok().filter(|v| !v.trim().is_empty());

        let (Some(account_name), Some(raw_iban)) =
            (non_blank("BANK_ACCOUNT_NAME"), non_blank("BANK_IBAN"))
        else {
            tracing::warn!("bank details not set: invoices will show the WhatsApp fallback");
            return Self::new(None);
        };

        let Some(iban) = normalize_iban(&raw_iban) else {
            tracing::error!(
                "BANK_IBAN is not a valid IBAN: invoices will show the WhatsApp fallback"
            );
            return Self::new(None);
        };

        Self::new(Some(BankDetails {
            account_name,
            iban,
            bic: non_blank("BANK_BIC"),
            kvk: non_blank("BUSINESS_KVK"),
        }))
    }

    /// `None` when no bank details are configured — the order still goes
    /// through, the customer just gets the WhatsApp fallback instead.
    pub fn instructions(&self, order_id: i64) -> Option<PaymentInstructions> {
        self.bank.as_ref().map(|bank| PaymentInstructions {
            account_name: bank.account_name.clone(),
            iban: bank.iban.clone(),
            bic: bank.bic.clone(),
            kvk: bank.kvk.clone(),
            reference: payment_reference(order_id),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_ibans_with_or_without_spaces() {
        assert_eq!(
            normalize_iban("NL91ABNA0417164300").as_deref(),
            Some("NL91ABNA0417164300")
        );
        assert_eq!(
            normalize_iban(" nl91 abna 0417 1643 00 ").as_deref(),
            Some("NL91ABNA0417164300")
        );
        assert_eq!(
            normalize_iban("DE89370400440532013000").as_deref(),
            Some("DE89370400440532013000")
        );
    }

    #[test]
    fn rejects_a_single_wrong_digit() {
        assert_eq!(normalize_iban("NL91ABNA0417164301"), None);
        assert_eq!(normalize_iban("NL92ABNA0417164300"), None);
    }

    #[test]
    fn rejects_malformed_input() {
        for raw in [
            "",
            "NL91",
            "91NLABNA0417164300",
            "NL91ABNA04171643!0",
            "<BANK_IBAN>",
        ] {
            assert_eq!(normalize_iban(raw), None, "{raw}");
        }
    }
}
