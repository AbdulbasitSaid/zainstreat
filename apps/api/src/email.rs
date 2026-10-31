use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use std::sync::Arc;

#[derive(Clone)]
pub struct EmailConfig {
    settings: Option<Arc<SesSettings>>,
    site_url: Arc<str>,
}

const DEFAULT_SITE_URL: &str = "http://localhost:3000";

struct SesSettings {
    smtp_host: String,
    smtp_username: String,
    smtp_password: String,
    from_address: String,
    business_notify_address: String,
}

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
        // Optional, for links in emails — a missing value never turns email
        // off (Phase 20 Decision 10).
        let site_url: Arc<str> = non_blank("PUBLIC_SITE_URL")
            .map(|url| url.trim_end_matches('/').to_string())
            .unwrap_or_else(|| DEFAULT_SITE_URL.to_string())
            .into();

        let (Some(smtp_host), Some(smtp_username), Some(smtp_password), Some(from_address), Some(business_notify_address)) =
            (smtp_host, smtp_username, smtp_password, from_address, business_notify_address)
        else {
            tracing::warn!("email sending disabled: SES env vars not fully set");
            return Self { settings: None, site_url };
        };

        Self {
            settings: Some(Arc::new(SesSettings {
                smtp_host,
                smtp_username,
                smtp_password,
                from_address,
                business_notify_address,
            })),
            site_url,
        }
    }

    /// The public site's base URL, no trailing slash.
    pub fn site_url(&self) -> &str {
        &self.site_url
    }

    pub fn business_notify_address(&self) -> Option<String> {
        self.settings.as_ref().map(|s| s.business_notify_address.clone())
    }

    /// Fire-and-forget: spawns the send, logs failure, never returns an
    /// error to the caller (requirement.md Decision 3).
    pub fn spawn_send(&self, to: String, subject: String, body: String) {
        let Some(settings) = self.settings.clone() else {
            tracing::debug!(to, "email disabled, skipping send");
            return;
        };

        tokio::spawn(async move {
            if let Err(error) = send(&settings, &to, &subject, body).await {
                tracing::error!(%error, to, subject, "failed to send email");
            }
        });
    }
}

async fn send(settings: &SesSettings, to: &str, subject: &str, body: String) -> Result<(), String> {
    let email = Message::builder()
        .from(settings.from_address.parse::<Mailbox>().map_err(|e| e.to_string())?)
        .to(to.parse::<Mailbox>().map_err(|e| e.to_string())?)
        .subject(subject)
        .body(body)
        .map_err(|e| e.to_string())?;

    let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&settings.smtp_host)
        .map_err(|e| e.to_string())?
        .credentials(Credentials::new(settings.smtp_username.clone(), settings.smtp_password.clone()))
        .build();

    mailer.send(email).await.map_err(|e| e.to_string())?;
    Ok(())
}
