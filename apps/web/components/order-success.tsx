import { useTranslations } from "next-intl";
import { ButtonLink } from "@/components/button";
import { OrderInvoice } from "@/components/order-invoice";
import { PaymentClaimButton } from "@/components/payment-claim-button";
import { Link } from "@/i18n/navigation";
import type { OrderResponse } from "@/lib/orders";

export function OrderSuccess({ order }: { order: OrderResponse }) {
  const t = useTranslations("Order");

  return (
    <div className="mx-auto flex max-w-[640px] flex-col gap-6 py-8 text-center">
      <h2 className="m-0">{t("successHeading", { id: order.id })}</h2>
      <p className="m-0 text-text-muted">{t("successBody")}</p>
      <OrderInvoice order={order} />
      <PaymentClaimButton token={order.public_token} status={order.status} />
      <p className="m-0 text-sm text-text-muted">
        {t("saveLink")}{" "}
        <Link href={`/order/${order.public_token}`} className="font-semibold text-primary underline">
          {t("saveLinkCta")}
        </Link>
      </p>
      <ButtonLink href="/menu" variant="secondary" className="self-center">{t("backToMenu")}</ButtonLink>
    </div>
  );
}
