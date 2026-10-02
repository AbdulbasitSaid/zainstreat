import { useTranslations } from "next-intl";
import { Link } from "@/i18n/navigation";

function InstagramIcon() {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      aria-hidden="true"
      className="h-[1.05rem] w-[1.05rem]"
    >
      <rect x="3" y="3" width="18" height="18" rx="5" />
      <circle cx="12" cy="12" r="4.2" />
      <circle cx="17.2" cy="6.8" r="1" fill="currentColor" stroke="none" />
    </svg>
  );
}

function WhatsAppIcon() {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      aria-hidden="true"
      className="h-[1.05rem] w-[1.05rem]"
    >
      <path d="M7 16.5 4.5 19l.9-3.3A8 8 0 1 1 7 16.5Z" />
      <path d="M9 10.3c0 2.6 2.1 4.7 4.7 4.7" strokeLinecap="round" />
    </svg>
  );
}

export function SiteFooter() {
  const t = useTranslations("SiteFooter");
  const year = new Date().getFullYear();

  return (
    <footer className="bg-primary-dark pt-16 pb-8 text-text-light [&_a]:text-on-dark-link [&_a:hover]:text-text-light">
      <div className="container">
        <div className="grid grid-cols-1 gap-10 pb-10 md:grid-cols-[2fr_1fr] md:items-start">
          <div>
            <p className="max-w-[42ch] text-on-dark-body">{t("tagline")}</p>

            <div className="mt-6">
              <h3 className="mb-3 text-text-light">
                {t.rich("newsletterHeading", { em: (chunks) => <em>{chunks}</em> })}
              </h3>
              <form className="flex max-w-[360px] items-center gap-2 rounded-full bg-text-light py-1.5 pr-1.5 pl-5">
                <input
                  type="email"
                  name="email"
                  placeholder={t("newsletterPlaceholder")}
                  aria-label={t("newsletterPlaceholder")}
                  disabled
                  className="min-w-0 flex-1 border-none bg-transparent p-0 text-text placeholder:text-text-muted focus-visible:outline focus-visible:outline-2 focus-visible:outline-accent focus-visible:outline-offset-2"
                />
                <button
                  type="submit"
                  disabled
                  aria-label={t("newsletterPlaceholder")}
                  className="flex h-[2.4rem] w-[2.4rem] shrink-0 items-center justify-center rounded-full bg-primary text-text-light disabled:opacity-50 focus-visible:outline focus-visible:outline-2 focus-visible:outline-accent focus-visible:outline-offset-2"
                >
                  →
                </button>
              </form>
              <p className="mt-2.5 max-w-[36ch] text-[0.8rem] text-on-dark-faint">
                {t("newsletterComingSoon")}
              </p>
            </div>

            <div className="mt-7 flex gap-3">
              <a
                href="https://instagram.com/zain_treats"
                target="_blank"
                rel="noopener noreferrer"
                aria-label={t("instagramLabel")}
                className="flex h-9 w-9 items-center justify-center rounded-full border border-white/40 hover:bg-white/10"
              >
                <InstagramIcon />
              </a>
              <a
                href="https://wa.me/31630545277"
                target="_blank"
                rel="noopener noreferrer"
                aria-label={t("whatsappLabel")}
                className="flex h-9 w-9 items-center justify-center rounded-full border border-white/40 hover:bg-white/10"
              >
                <WhatsAppIcon />
              </a>
            </div>
          </div>

          <nav aria-label={t("footerNavLabel")} className="flex flex-wrap gap-10">
            <div>
              <p className="mb-3 text-[0.8rem] font-semibold tracking-[0.12em] text-on-dark-label uppercase">
                {t("footerExploreHeading")}
              </p>
              <ul className="m-0 flex list-none flex-col gap-2.5 p-0">
                <li><Link href="/about">{t("about")}</Link></li>
                <li><Link href="/services">{t("services")}</Link></li>
                <li><Link href="/contact">{t("contact")}</Link></li>
              </ul>
            </div>
            <div>
              <p className="mb-3 text-[0.8rem] font-semibold tracking-[0.12em] text-on-dark-label uppercase">
                {t("footerSupportHeading")}
              </p>
              <ul className="m-0 flex list-none flex-col gap-2.5 p-0">
                <li><Link href="/terms">{t("terms")}</Link></li>
                <li><Link href="/food-regulations">{t("foodRegulations")}</Link></li>
              </ul>
            </div>
          </nav>
        </div>
        <p className="border-t border-white/15 pt-6 text-sm text-on-dark-label">
          {t("copyright", { year })}
        </p>
      </div>
    </footer>
  );
}
