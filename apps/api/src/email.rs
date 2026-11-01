use lettre::message::Mailbox;
use lettre::Address;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct EmailConfig {
    settings: Option<Arc<SesSettings>>,
    site_url: Arc<str>,
    admin_url: Arc<str>,
}

const DEFAULT_SITE_URL: &str = "http://localhost:3000";
const DEFAULT_ADMIN_URL: &str = "http://cms.localhost:3000";

/// The sender's display name. The subject no longer has room for the brand
/// once it carries three languages (Phase 24 Decision 11), and the deploy
/// workflow rejects a `'` in env values, so the name lives here and
/// `EMAIL_FROM_ADDRESS` stays a bare address.
const SENDER_NAME: &str = "Zain's Treat n More";

struct SesSettings {
    smtp_host: String,
    smtp_username: String,
    smtp_password: String,
    from_address: String,
    business_notify_address: String,
    /// `[STAGING] ` on staging, empty elsewhere (Phase 25 Decision 1b).
    subject_prefix: String,
}

/// One outgoing email. Owner notifications reply to the customer, customer
/// emails reply to the From address (Phase 25 Decision 1a).
pub struct OutgoingEmail {
    pub to: String,
    pub subject: String,
    pub body: String,
    /// `None` → the From address (customer emails); owner notifications
    /// pass the customer's address.
    pub reply_to: Option<String>,
    /// Customer emails only (Phase 25 Decision 6a).
    pub track: Option<DeliveryRef>,
}

/// Where to record a customer email's outcome in `email_deliveries`.
pub struct DeliveryRef {
    pub pool: PgPool,
    pub order_id: Option<i64>,
    pub quote_id: Option<i64>,
    /// `invoice`, `status` or `quote`.
    pub kind: &'static str,
}

const DELIVERY_ERROR_MAX_CHARS: usize = 500;

impl EmailConfig {
    pub fn from_env() -> Self {
        // `docker-compose.yml` defaults each of these to an empty string
        // rather than leaving them unset (so the base compose file is valid
        // without a `.env` at all), so an empty value must be treated the
        // same as a missing one here — `std::env::var` alone would return
        // `Ok("")`, not `Err`, for a var set to "".
        let non_blank = |key: &str| std::env::var(key).ok().filter(|v| !v.is_empty());

        let smtp_host = non_blank("SES_SMTP_HOST");
        let smtp_username = non_blank("SES_SMTP_USERNAME");
        let smtp_password = non_blank("SES_SMTP_PASSWORD");
        let from_address = non_blank("EMAIL_FROM_ADDRESS");
        let business_notify_address = non_blank("EMAIL_BUSINESS_NOTIFY_ADDRESS");
        // Optional, and not part of the "all five set" check below.
        let subject_prefix = normalize_subject_prefix(&std::env::var("EMAIL_SUBJECT_PREFIX").unwrap_or_default());
        // Optional, for links in emails — a missing value never turns email
        // off (Phase 20 Decision 10).
        let base_url = |key: &str, default: &str| -> Arc<str> {
            non_blank(key)
                .map(|url| url.trim_end_matches('/').to_string())
                .unwrap_or_else(|| default.to_string())
                .into()
        };
        let site_url = base_url("PUBLIC_SITE_URL", DEFAULT_SITE_URL);
        // The admin lives on its own host (Phase 21 Decision 20).
        let admin_url = base_url("PUBLIC_ADMIN_URL", DEFAULT_ADMIN_URL);

        let (Some(smtp_host), Some(smtp_username), Some(smtp_password), Some(from_address), Some(business_notify_address)) =
            (smtp_host, smtp_username, smtp_password, from_address, business_notify_address)
        else {
            tracing::warn!("email sending disabled: SES env vars not fully set");
            return Self { settings: None, site_url, admin_url };
        };

        Self {
            settings: Some(Arc::new(SesSettings {
                smtp_host,
                smtp_username,
                smtp_password,
                from_address,
                business_notify_address,
                subject_prefix,
            })),
            site_url,
            admin_url,
        }
    }

    /// The public site's base URL, no trailing slash.
    pub fn site_url(&self) -> &str {
        &self.site_url
    }

    /// The admin's base URL (`cms.` host), no trailing slash.
    pub fn admin_url(&self) -> &str {
        &self.admin_url
    }

    /// False when SES isn't fully configured and every send is skipped, so
    /// admin can offer to share a quote link instead (Phase 23 Decision 14).
    pub fn is_enabled(&self) -> bool {
        self.settings.is_some()
    }

    pub fn business_notify_address(&self) -> Option<String> {
        self.settings.as_ref().map(|s| s.business_notify_address.clone())
    }

    /// Fire-and-forget: spawns the send, logs failure, never returns an
    /// error to the caller (requirement.md Decision 3). A tracked email's
    /// outcome is recorded in `email_deliveries`; a failed insert or update
    /// is logged and never surfaces (Phase 25 Decision 6a).
    pub fn spawn_send(&self, email: OutgoingEmail) {
        let Some(settings) = self.settings.clone() else {
            tracing::debug!(to = email.to, "email disabled, skipping send");
            return;
        };

        tokio::spawn(async move {
            let delivery_id = match &email.track {
                Some(track) => record_queued(track, &email.to).await,
                None => None,
            };

            let result = send(&settings, &email).await;
            if let Err(error) = &result {
                tracing::error!(%error, to = email.to, subject = email.subject, "failed to send email");
            }

            if let (Some(track), Some(id)) = (&email.track, delivery_id) {
                record_outcome(&track.pool, id, &result).await;
            }
        });
    }
}

async fn record_queued(track: &DeliveryRef, recipient: &str) -> Option<i64> {
    let inserted = sqlx::query_scalar!(
        r#"
        INSERT INTO email_deliveries (order_id, quote_id, kind, recipient)
        VALUES ($1, $2, $3, $4)
        RETURNING id
        "#,
        track.order_id,
        track.quote_id,
        track.kind,
        recipient,
    )
    .fetch_one(&track.pool)
    .await;

    match inserted {
        Ok(id) => Some(id),
        Err(err) => {
            tracing::warn!(%err, kind = track.kind, "failed to record queued email delivery");
            None
        }
    }
}

async fn record_outcome(pool: &PgPool, id: i64, result: &Result<(), String>) {
    let (status, error) = delivery_outcome(result);
    if let Err(err) = sqlx::query!(
        "UPDATE email_deliveries SET status = $2, error = $3, updated_at = now() WHERE id = $1",
        id,
        status,
        error,
    )
    .execute(pool)
    .await
    {
        tracing::warn!(%err, id, "failed to record email delivery outcome");
    }
}

/// `sent`, or `failed` with the error truncated for storage.
fn delivery_outcome(result: &Result<(), String>) -> (&'static str, Option<String>) {
    match result {
        Ok(()) => ("sent", None),
        Err(error) => ("failed", Some(error.chars().take(DELIVERY_ERROR_MAX_CHARS).collect())),
    }
}

/// A non-empty prefix always ends in a space, so `[STAGING]` and
/// `[STAGING] ` both read `[STAGING] New order …`.
fn normalize_subject_prefix(raw: &str) -> String {
    if raw.is_empty() || raw.ends_with(' ') {
        raw.to_string()
    } else {
        format!("{raw} ")
    }
}

fn build_message(settings: &SesSettings, email: &OutgoingEmail) -> Result<Message, String> {
    let from = sender(&settings.from_address)?;
    // An unparsable customer address never drops the notification: it
    // just replies to the From address instead.
    let reply_to = match email.reply_to.as_deref().map(str::parse::<Mailbox>) {
        Some(Ok(mailbox)) => mailbox,
        Some(Err(error)) => {
            tracing::warn!(%error, "unparsable reply-to, using the from address");
            from.clone()
        }
        None => from.clone(),
    };

    Message::builder()
        .from(from)
        .reply_to(reply_to)
        .to(email.to.parse::<Mailbox>().map_err(|e| e.to_string())?)
        .subject(format!("{}{}", settings.subject_prefix, email.subject))
        .body(email.body.clone())
        .map_err(|e| e.to_string())
}

async fn send(settings: &SesSettings, email: &OutgoingEmail) -> Result<(), String> {
    let message = build_message(settings, email)?;

    let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&settings.smtp_host)
        .map_err(|e| e.to_string())?
        .credentials(Credentials::new(settings.smtp_username.clone(), settings.smtp_password.clone()))
        .build();

    mailer.send(message).await.map_err(|e| e.to_string())?;
    Ok(())
}

/// `Zain's Treat n More <{EMAIL_FROM_ADDRESS}>`.
fn sender(from_address: &str) -> Result<Mailbox, String> {
    let address = from_address.parse::<Address>().map_err(|e| e.to_string())?;
    Ok(Mailbox::new(Some(SENDER_NAME.to_string()), address))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sender_carries_the_brand_name() {
        let mailbox = sender("no-reply@zainstreat.com").unwrap();
        assert_eq!(mailbox.name.as_deref(), Some("Zain's Treat n More"));
        assert_eq!(mailbox.email.to_string(), "no-reply@zainstreat.com");
        assert!(mailbox.to_string().contains("<no-reply@zainstreat.com>"));
    }

    #[test]
    fn sender_rejects_a_malformed_address() {
        assert!(sender("not an address").is_err());
    }

    fn settings(subject_prefix: &str) -> SesSettings {
        SesSettings {
            smtp_host: "smtp.example.com".into(),
            smtp_username: "user".into(),
            smtp_password: "pass".into(),
            from_address: "contact@zainstreat.com".into(),
            business_notify_address: "contact@zainstreat.com".into(),
            subject_prefix: normalize_subject_prefix(subject_prefix),
        }
    }

    fn outgoing(reply_to: Option<&str>) -> OutgoingEmail {
        OutgoingEmail {
            to: "customer@example.com".into(),
            subject: "New order".into(),
            body: "Hello".into(),
            reply_to: reply_to.map(str::to_string),
            track: None,
        }
    }

    fn header(message: &Message, name: &str) -> Option<String> {
        message.headers().get_raw(name).map(str::to_string)
    }

    #[test]
    fn email_reply_to_defaults_to_the_branded_from() {
        let message = build_message(&settings(""), &outgoing(None)).unwrap();
        let reply_to = header(&message, "Reply-To").unwrap();
        assert!(reply_to.contains("<contact@zainstreat.com>"), "{reply_to}");
        assert!(reply_to.contains("Zain's Treat n More"), "{reply_to}");
    }

    #[test]
    fn email_uses_an_explicit_reply_to() {
        let message = build_message(&settings(""), &outgoing(Some("ada@example.com"))).unwrap();
        assert_eq!(header(&message, "Reply-To").as_deref(), Some("ada@example.com"));
    }

    #[test]
    fn email_unparsable_reply_to_falls_back_to_from() {
        let message = build_message(&settings(""), &outgoing(Some("not an address"))).unwrap();
        assert!(header(&message, "Reply-To").unwrap().contains("<contact@zainstreat.com>"));
    }

    #[test]
    fn email_subject_gets_the_prefix_when_set() {
        let message = build_message(&settings("[STAGING] "), &outgoing(None)).unwrap();
        assert_eq!(header(&message, "Subject").as_deref(), Some("[STAGING] New order"));
    }

    #[test]
    fn email_subject_has_no_prefix_when_unset() {
        let message = build_message(&settings(""), &outgoing(None)).unwrap();
        assert_eq!(header(&message, "Subject").as_deref(), Some("New order"));
    }

    #[test]
    fn email_prefix_without_a_trailing_space_gets_one() {
        assert_eq!(normalize_subject_prefix("[STAGING]"), "[STAGING] ");
        assert_eq!(normalize_subject_prefix("[STAGING] "), "[STAGING] ");
        assert_eq!(normalize_subject_prefix(""), "");
    }

    #[test]
    fn email_send_errors_become_a_failed_delivery() {
        assert_eq!(delivery_outcome(&Ok(())), ("sent", None));

        let error = build_message(&settings(""), &OutgoingEmail { to: "nope".into(), ..outgoing(None) })
            .map(|_| ())
            .unwrap_err();
        let (status, stored) = delivery_outcome(&Err(error.clone()));
        assert_eq!(status, "failed");
        assert_eq!(stored, Some(error));

        let long = "554 Message rejected: not verified ".repeat(40);
        let (_, stored) = delivery_outcome(&Err(long));
        assert_eq!(stored.unwrap().chars().count(), DELIVERY_ERROR_MAX_CHARS);
    }
}
