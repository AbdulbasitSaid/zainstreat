import { useTranslations } from "next-intl";
import { Link } from "@/i18n/navigation";
import { buildWhatsAppLink } from "@/lib/whatsapp";
import { LiquidSurface } from "@/components/liquid-surface";

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
  const tw = useTranslations("Whatsapp");
  const year = new Date().getFullYear();

  return (
    <footer className="bg-primary-dark pt-16 pb-8 text-text-light [&_a]:text-on-dark-link [&_a:hover]:text-text-light">
      <div className="container">
        <div className="grid grid-cols-1 gap-10 pb-10 md:grid-cols-[2fr_1fr] md:items-start">
          <div>
            <p className="max-w-[42ch] text-on-dark-body">{t("tagline")}</p>

            <h3 className="mt-6 mb-3 text-text-light">
              {t.rich("followHeading", { em: (chunks) => <em>{chunks}</em> })}
            </h3>
            <div className="flex gap-3">
              <a
                href="https://instagram.com/zain_treats"
                target="_blank"
                rel="noopener noreferrer"
                aria-label={t("instagramLabel")}
                className="group/liquid relative isolate flex h-9 w-9 items-center justify-center rounded-full border border-white/40 hover:bg-white/10"
              >
                <InstagramIcon />
                <SocialLiquid />
              </a>
              <a
                href={buildWhatsAppLink(tw("prefilledMessage"))}
                target="_blank"
                rel="noopener noreferrer"
                aria-label={t("whatsappLabel")}
                className="group/liquid relative isolate flex h-9 w-9 items-center justify-center rounded-full border border-white/40 hover:bg-white/10"
              >
                <WhatsAppIcon />
                <SocialLiquid />
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
                <li><Link href="/menu">{t("menu")}</Link></li>
                <li><Link href="/rentals">{t("rentals")}</Link></li>
                <li><Link href="/contact">{t("contact")}</Link></li>
              </ul>
            </div>
            <div>
              <p className="mb-3 text-[0.8rem] font-semibold tracking-[0.12em] text-on-dark-label uppercase">
                {t("footerSupportHeading")}
              </p>
              <ul className="m-0 flex list-none flex-col gap-2.5 p-0">
                <li><Link href="/terms">{t("terms")}</Link></li>
                <li><Link href="/privacy">{t("privacy")}</Link></li>
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

// The social icons' border and hover tint, as a liquid surface.
function SocialLiquid() {
  return (
    <LiquidSurface
      strokeWidth={1}
      pathClassName="fill-white/0 stroke-white/40 transition-[fill] duration-150 group-hover/liquid:fill-white/10"
      splashClassName="fill-white"
    />
  );
}
