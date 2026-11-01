import { useLocale, useTranslations } from "next-intl";
import { buttonClasses } from "@/components/button";
import { Notice } from "@/components/notice";
import { OrderLineSummary } from "@/components/order-line-summary";
import { formatPrice } from "@/lib/format";
import type { OrderResponse } from "@/lib/orders";
import { buildWhatsAppLink } from "@/lib/whatsapp";

// IBANs are easier to read back and type in groups of four.
function formatIban(iban: string): string {
  return iban.replace(/\s+/g, "").replace(/(.{4})(?=.)/g, "$1 ");
}

function PaymentField({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex flex-wrap justify-between gap-x-4 gap-y-1">
      <dt className="text-text-muted">{label}</dt>
      <dd className="m-0 font-semibold">{value}</dd>
    </div>
  );
}

// Shared by the post-checkout confirmation and /order/[token] (Phase 20
// Decision 8). Server-compatible: no state, no effects. No VAT line — the
// business is under the KOR scheme.
export function OrderInvoice({ order }: { order: OrderResponse }) {
  const t = useTranslations("Order");
  const locale = useLocale();

  // Explicit timeZone, same reason as lib/format.ts's formatDateTime: the
  // server renders in UTC, the browser in the visitor's zone.
  const placedOn = new Intl.DateTimeFormat(locale, {
    timeZone: "Europe/Amsterdam",
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(order.created_at));

  return (
    <div className="flex flex-col gap-6 text-left">
      <div className="flex flex-col gap-1 text-sm text-text-muted">
        <p className="m-0">{t("placedOn", { date: placedOn })}</p>
        <p className="m-0">
          {order.delivery_type === "delivery" && order.delivery_address
            ? t("deliveryLine", { address: order.delivery_address })
            : order.delivery_type === "on_site" && order.delivery_address
              ? t("servedAtLine", { address: order.delivery_address })
              : t("pickupLine")}
        </p>
      </div>

      <OrderLineSummary
        lines={order.items.map((item) => ({
          name: item.item_name,
          optionLabel: item.option_label,
          unitPrice: item.unit_price,
          quantity: item.quantity,
        }))}
      />
      <p className="m-0 text-right text-lg font-semibold">
        {t("total")}: <span className="text-primary">{formatPrice(order.total)}</span>
      </p>

      <section className="flex flex-col gap-4 rounded-card bg-background-soft p-5">
        <h3 className="m-0 text-lg font-semibold">{t("howToPay")}</h3>
        {order.payment ? (
          <>
            <p className="m-0">{t("payByTransfer", { total: formatPrice(order.total) })}</p>
            <dl className="m-0 flex flex-col gap-2 text-sm">
              <PaymentField label={t("accountName")} value={order.payment.account_name} />
              <PaymentField label={t("iban")} value={formatIban(order.payment.iban)} />
              {order.payment.bic && <PaymentField label={t("bic")} value={order.payment.bic} />}
            </dl>
            <div className="rounded-card border-2 border-primary bg-card px-4 py-3 text-center">
              <p className="m-0 text-xs font-semibold uppercase tracking-wider text-text-muted">
                {t("reference")}
              </p>
              <p className="m-0 text-2xl font-semibold tracking-wide text-primary">
                {order.payment.reference}
              </p>
              <p className="m-0 mt-1 text-xs text-text-muted">{t("referenceHint")}</p>
            </div>
            {order.payment.kvk && (
              <p className="m-0 text-xs text-text-muted">{t("kvk", { number: order.payment.kvk })}</p>
            )}
          </>
        ) : (
          <>
            <p className="m-0">{t("noBankDetails", { reference: order.payment_reference })}</p>
            <a
              href={buildWhatsAppLink(t("noBankDetailsMessage", { reference: order.payment_reference }))}
              target="_blank"
              rel="noopener noreferrer"
              className={buttonClasses({ variant: "outline", color: "whatsapp", className: "self-start" })}
            >
              {t("noBankDetailsCta")}
            </a>
          </>
        )}
      </section>

      <Notice>{t("verifiedOnly")}</Notice>
    </div>
  );
}
