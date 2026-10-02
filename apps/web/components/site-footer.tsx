import { useTranslations } from "next-intl";
import { Link } from "@/i18n/navigation";

function InstagramIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true">
      <rect x="3" y="3" width="18" height="18" rx="5" />
      <circle cx="12" cy="12" r="4.2" />
      <circle cx="17.2" cy="6.8" r="1" fill="currentColor" stroke="none" />
    </svg>
  );
}

function WhatsAppIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true">
      <path d="M7 16.5 4.5 19l.9-3.3A8 8 0 1 1 7 16.5Z" />
      <path d="M9 10.3c0 2.6 2.1 4.7 4.7 4.7" strokeLinecap="round" />
    </svg>
  );
}

export function SiteFooter() {
  const t = useTranslations("SiteFooter");
  const year = new Date().getFullYear();

  return (
    <footer className="site-footer">
      <div className="container">
        <div className="footer-grid">
          <div>
            <p>{t("tagline")}</p>

            <div className="footer-newsletter">
              <h3>
                {t.rich("newsletterHeading", { em: (chunks) => <em>{chunks}</em> })}
              </h3>
              <form className="footer-newsletter-row">
                <input
                  type="email"
                  name="email"
                  placeholder={t("newsletterPlaceholder")}
                  aria-label={t("newsletterPlaceholder")}
                  disabled
                />
                <button type="submit" disabled aria-label={t("newsletterPlaceholder")}>
                  →
                </button>
              </form>
              <p className="footer-newsletter-note">{t("newsletterComingSoon")}</p>
            </div>

            <div className="footer-socials">
              <a
                href="https://instagram.com/zain_treats"
                target="_blank"
                rel="noopener noreferrer"
                aria-label={t("instagramLabel")}
              >
                <InstagramIcon />
              </a>
              <a
                href="https://wa.me/31630545277"
                target="_blank"
                rel="noopener noreferrer"
                aria-label={t("whatsappLabel")}
              >
                <WhatsAppIcon />
              </a>
            </div>
          </div>

          <nav aria-label={t("footerNavLabel")}>
            <div>
              <p className="footer-nav-heading">{t("footerExploreHeading")}</p>
              <ul>
                <li><Link href="/about">{t("about")}</Link></li>
                <li><Link href="/services">{t("services")}</Link></li>
                <li><Link href="/contact">{t("contact")}</Link></li>
              </ul>
            </div>
            <div>
              <p className="footer-nav-heading">{t("footerSupportHeading")}</p>
              <ul>
                <li><Link href="/terms">{t("terms")}</Link></li>
                <li><Link href="/food-regulations">{t("foodRegulations")}</Link></li>
              </ul>
            </div>
          </nav>
        </div>
        <p className="footer-bottom">{t("copyright", { year })}</p>
      </div>
    </footer>
  );
}
