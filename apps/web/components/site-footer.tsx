import { useTranslations } from "next-intl";
import { Link } from "@/i18n/navigation";

export function SiteFooter() {
  const t = useTranslations("SiteFooter");
  const year = new Date().getFullYear();

  return (
    <footer className="site-footer">
      <div className="container">
        <p>{t("tagline")}</p>
        <nav aria-label={t("footerNavLabel")}>
          <ul>
            <li><Link href="/about">{t("about")}</Link></li>
            <li><Link href="/services">{t("services")}</Link></li>
            <li><Link href="/contact">{t("contact")}</Link></li>
            <li><Link href="/terms">{t("terms")}</Link></li>
            <li><Link href="/food-regulations">{t("foodRegulations")}</Link></li>
          </ul>
        </nav>
        <p>{t("copyright", { year })}</p>
      </div>
    </footer>
  );
}
