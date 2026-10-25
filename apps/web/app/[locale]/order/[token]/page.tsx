import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { getTranslations } from "next-intl/server";
import { OrderInvoice } from "@/components/order-invoice";
import { PageHero } from "@/components/page-hero";
import { PaymentClaimButton } from "@/components/payment-claim-button";
import { getOrderByToken } from "@/lib/api";

// The link in the invoice email (Phase 20 Decision 4). The token is the only
// thing protecting this page, so keep it out of search results and out of
// the Referer header on outbound links (e.g. WhatsApp).
export async function generateMetadata({
  params,
}: {
  params: Promise<{ locale: string }>;
}): Promise<Metadata> {
  const { locale } = await params;
  const t = await getTranslations({ locale, namespace: "Order" });

  return {
    title: t("trackMetaTitle"),
    robots: { index: false, follow: false },
    referrer: "no-referrer",
  };
}

export default async function OrderStatusPage({
  params,
}: {
  params: Promise<{ token: string }>;
}) {
  const { token } = await params;
  const order = await getOrderByToken(token);
  if (!order) {
    notFound();
  }

  const t = await getTranslations("Order");

  return (
    <main className="container">
      <PageHero>
        <h1 className="text-[clamp(2.5rem,5vw,4.25rem)]">{t("trackHeading", { id: order.id })}</h1>
      </PageHero>
      <div className="mx-auto flex max-w-[640px] flex-col gap-6 pb-16 text-center">
        <p className="m-0 text-lg">
          <span className="text-text-muted">{t("statusLabel")}:</span>{" "}
          <span className="font-semibold">{t(`status.${order.status}`)}</span>
        </p>
        <OrderInvoice order={order} />
        <PaymentClaimButton token={order.public_token} status={order.status} />
      </div>
    </main>
  );
}
