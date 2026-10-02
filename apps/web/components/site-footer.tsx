import { useTranslations } from "next-intl";
import { Link } from "@/i18n/navigation";

export function SiteFooter() {
  const t = useTranslations("SiteFooter");
  const year = new Date().getFullYear();

  return (
    <footer className="site-footer">
      <div className="container">
        <div className="footer-grid">
          <div>
            <p>{t("tagline")}</p>
          </div>
          <nav aria-label={t("footerNavLabel")}>
            <div>
              <p className="footer-nav-heading">{t("footerNavLabel")}</p>
              <ul>
                <li><Link href="/about">{t("about")}</Link></li>
                <li><Link href="/services">{t("services")}</Link></li>
                <li><Link href="/contact">{t("contact")}</Link></li>
              </ul>
            </div>
            <div>
              <p className="footer-nav-heading">{t("terms")}</p>
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
