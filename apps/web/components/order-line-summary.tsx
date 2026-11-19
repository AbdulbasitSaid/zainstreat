import { formatPrice } from "@/lib/format";

export interface OrderSummaryLine {
  name: string;
  optionLabel: string | null;
  unitPrice: string;
  quantity: number;
  // A rental, priced per event (Phase 37 Decision 2).
  perEvent?: boolean;
}

export function OrderLineSummary({ lines, perEventLabel }: { lines: OrderSummaryLine[]; perEventLabel: string }) {
  return (
    <ul className="m-0 list-none p-0">
      {lines.map((line, index) => (
        <li key={index} className="flex items-center justify-between border-b border-text/10 py-3 last:border-0">
          <div>
            <p className="m-0 font-semibold">{line.name}</p>
            {line.optionLabel && <p className="m-0 text-xs text-text-muted">{line.optionLabel}</p>}
          </div>
          <p className="m-0 text-sm text-text-muted">
            {line.quantity} × {formatPrice(line.unitPrice)}
            {line.perEvent && ` ${perEventLabel}`}
          </p>
        </li>
      ))}
    </ul>
  );
}
