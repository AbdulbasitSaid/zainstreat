"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { ORDER_STATUSES, type OrderStatus } from "@/lib/admin-orders";
import { Notice } from "@/components/notice";

export function AdminOrderStatusSelect({
  orderId,
  status,
}: {
  orderId: number;
  status: OrderStatus;
}) {
  const router = useRouter();
  const [value, setValue] = useState<OrderStatus>(status);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleChange(next: OrderStatus) {
    const previous = value;
    setValue(next);
    setError(null);
    setSaving(true);

    const response = await fetch(`/api/admin/orders/${orderId}/status`, {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ status: next }),
    });

    setSaving(false);

    if (!response.ok) {
      setValue(previous);
      setError("Could not update the status. Please try again.");
      return;
    }

    router.refresh();
  }

  return (
    <div className="flex flex-col gap-2">
      <label htmlFor="order-status" className="text-xs font-semibold uppercase tracking-wider text-text-muted">
        Status
      </label>
      <select
        id="order-status"
        className="field max-w-[16rem] capitalize"
        value={value}
        disabled={saving}
        onChange={(event) => handleChange(event.target.value as OrderStatus)}
      >
        {ORDER_STATUSES.map((option) => (
          <option key={option} value={option} className="capitalize">
            {option}
          </option>
        ))}
      </select>
      {error && <Notice>{error}</Notice>}
    </div>
  );
}
