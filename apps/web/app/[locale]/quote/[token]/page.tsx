import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { getTranslations } from "next-intl/server";
import { buttonClasses, ButtonLink } from "@/components/button";
import { Notice } from "@/components/notice";
import { PageHero } from "@/components/page-hero";
import { QuoteActions } from "@/components/quote-actions";
import { QuoteLines } from "@/components/quote-lines";
import { getQuoteByToken } from "@/lib/api";
import { buildWhatsAppLink } from "@/lib/whatsapp";

// The link in the quote email (Phase 23 Decision 4). Like the order page, the
// token is the only thing protecting it: no indexing, no Referer.
export async function generateMetadata({
  params,
}: {
  params: Promise<{ locale: string }>;
}): Promise<Metadata> {
  const { locale } = await params;
  const t = await getTranslations({ locale, namespace: "Quote" });

  return {
    title: t("metaTitle"),
    robots: { index: false, follow: false },
    referrer: "no-referrer",
  };
}

export default async function QuotePage({
  params,
}: {
  params: Promise<{ locale: string; token: string }>;
}) {
  const { locale, token } = await params;
  const quote = await getQuoteByToken(token);
  if (!quote) {
    notFound();
  }

  const t = await getTranslations("Quote");
  const tContact = await getTranslations("ContactPage");

  // Date-only values: format in UTC so the day never shifts.
  const formatDate = (isoDate: string) =>
    new Intl.DateTimeFormat(locale, { timeZone: "UTC", dateStyle: "long" }).format(new Date(`${isoDate}T00:00:00Z`));

  const fulfilment =
    quote.delivery_type === "pickup" || !quote.delivery_address
      ? t("fulfilmentPickup")
      : quote.delivery_type === "on_site"
        ? t("fulfilmentOnSite", { address: quote.delivery_address })
        : t("fulfilmentDelivery", { address: quote.delivery_address });

  const contactLink = (
    <a
      href={buildWhatsAppLink(t("heading", { id: quote.id }))}
      target="_blank"
      rel="noopener noreferrer"
      className={buttonClasses({ variant: "outline", color: "whatsapp" })}
    >
      {t("contactWhatsApp")}
    </a>
  );

  return (
    <main className="container">
      <PageHero>
        <h1 className="text-[clamp(2.5rem,5vw,4.25rem)]">{t("heading", { id: quote.id })}</h1>
      </PageHero>
      <div className="mx-auto flex max-w-[640px] flex-col gap-6 pb-16 text-center">
        <div className="flex flex-col gap-1 text-left text-sm text-text-muted">
          <p className="m-0 text-base font-semibold text-text">
            {t("eventSummary", {
              eventType: tContact(`eventType.${quote.event_type}`),
              date: formatDate(quote.event_date),
              guests: quote.guest_count,
            })}
          </p>
          <p className="m-0">{fulfilment}</p>
          <p className="m-0">{t("validUntil", { date: formatDate(quote.valid_until) })}</p>
        </div>

        <QuoteLines items={quote.items} total={quote.total} />

        {quote.notes && (
          <section className="rounded-card bg-background-soft p-5 text-left">
            <h2 className="m-0 mb-2 text-lg font-semibold">{t("notes")}</h2>
            <p className="m-0 whitespace-pre-wrap">{quote.notes}</p>
          </section>
        )}

        {quote.display_status === "sent" && (
          <>
            <p className="m-0">{t("statusSent")}</p>
            <QuoteActions token={token} version={quote.version} />
          </>
        )}
        {quote.display_status === "draft" && <Notice>{t("statusDraft")}</Notice>}
        {quote.display_status === "expired" && (
          <div className="flex flex-col items-center gap-3">
            <Notice>{t("statusExpired", { date: formatDate(quote.valid_until) })}</Notice>
            {contactLink}
          </div>
        )}
        {quote.display_status === "declined" && (
          <div className="flex flex-col items-center gap-3">
            <Notice>{t("statusDeclined")}</Notice>
            {contactLink}
          </div>
        )}
        {quote.display_status === "accepted" && quote.order_token && (
          <div className="flex flex-col items-center gap-3">
            <Notice>{t("statusAccepted")}</Notice>
            <ButtonLink href={`/order/${quote.order_token}`}>{t("viewOrder")}</ButtonLink>
          </div>
        )}
      </div>
    </main>
  );
}
