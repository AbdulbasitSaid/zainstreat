import { useTranslations } from "next-intl";
import { formatPrice } from "@/lib/format";
import type { QuoteItem } from "@/lib/quotes";

// The quote's free-text lines (Phase 23 Decision 2), as a table so the
// quantity, price and subtotal columns line up.
export function QuoteLines({ items, total }: { items: QuoteItem[]; total: string }) {
  const t = useTranslations("Quote");

  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse text-left text-sm">
        <caption className="sr-only">{t("linesCaption")}</caption>
        <thead>
          <tr className="border-b border-text/15 text-xs uppercase tracking-wider text-text-muted">
            <th scope="col" className="py-2 pr-3 font-semibold">{t("colDescription")}</th>
            <th scope="col" className="py-2 pr-3 text-right font-semibold">{t("colQuantity")}</th>
            <th scope="col" className="py-2 pr-3 text-right font-semibold">{t("colPrice")}</th>
            <th scope="col" className="py-2 text-right font-semibold">{t("colSubtotal")}</th>
          </tr>
        </thead>
        <tbody>
          {items.map((item, index) => (
            <tr key={index} className="border-b border-text/10">
              <td className="py-3 pr-3">{item.description}</td>
              <td className="py-3 pr-3 text-right">{item.quantity}</td>
              <td className="py-3 pr-3 text-right">{formatPrice(item.unit_price)}</td>
              <td className="py-3 text-right font-semibold">{formatPrice(item.subtotal)}</td>
            </tr>
          ))}
        </tbody>
        <tfoot>
          <tr>
            <th scope="row" colSpan={3} className="pt-4 pr-3 text-right text-lg font-semibold">
              {t("total")}
            </th>
            <td className="pt-4 text-right text-lg font-semibold text-primary">{formatPrice(total)}</td>
          </tr>
        </tfoot>
      </table>
    </div>
  );
}
