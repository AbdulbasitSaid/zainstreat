use serde::Serialize;
use std::sync::Arc;

/// The business's bank account, shown on every invoice (Phase 20
/// requirement.md Decision 5).
pub struct BankDetails {
    pub account_name: String,
    pub iban: String,
    pub bic: Option<String>,
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
    pub reference: String,
}

/// Derived from the id, never stored, so it can't drift (Decision 6).
pub fn payment_reference(order_id: i64) -> String {
    format!("ZTM-{order_id}")
}

impl PaymentConfig {
    pub fn new(bank: Option<BankDetails>) -> Self {
        Self { bank: bank.map(Arc::new) }
    }

    pub fn from_env() -> Self {
        // Blank counts as unset, same as `EmailConfig::from_env`.
        let non_blank = |key: &str| std::env::var(key).ok().filter(|v| !v.trim().is_empty());

        let (Some(account_name), Some(iban)) = (non_blank("BANK_ACCOUNT_NAME"), non_blank("BANK_IBAN"))
        else {
            tracing::warn!("bank details not set: invoices will show the WhatsApp fallback");
            return Self::new(None);
        };

        Self::new(Some(BankDetails { account_name, iban, bic: non_blank("BANK_BIC") }))
    }

    /// `None` when no bank details are configured — the order still goes
    /// through, the customer just gets the WhatsApp fallback instead.
    pub fn instructions(&self, order_id: i64) -> Option<PaymentInstructions> {
        self.bank.as_ref().map(|bank| PaymentInstructions {
            account_name: bank.account_name.clone(),
            iban: bank.iban.clone(),
            bic: bank.bic.clone(),
            reference: payment_reference(order_id),
        })
    }
}
