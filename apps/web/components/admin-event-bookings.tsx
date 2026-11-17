"use client";

import { useState } from "react";
import Link from "next/link";
import { AdminConfirmDialog } from "@/components/admin-confirm-dialog";
import { Button } from "@/components/button";
import { Notice } from "@/components/notice";
import { BOOKING_STATUS_LABELS, type AdminBooking, type AdminBookingStatus, type AdminBookings } from "@/lib/admin-events";
import { formatDateTime, formatPrice } from "@/lib/format";

const STATUS_CLASSES: Record<AdminBookingStatus, string> = {
  awaiting_payment: "bg-background-soft text-text-muted",
  confirmed: "bg-whatsapp/15 text-whatsapp-dark",
  waitlisted: "bg-accent-light text-primary-dark",
  cancelled: "bg-text/10 text-text-muted",
};

const CANCEL_REASONS = { expired: "unpaid, released", guest: "by guest", admin: "by you" } as const;

type Pending = { booking: AdminBooking; action: "offer" | "cancel" };

function plateSummary(booking: AdminBooking): string {
  const counts = new Map<string, number>();
  for (const guest of booking.guests) {
    const name = guest.plate_name ? (guest.choices ? `${guest.plate_name} (${guest.choices})` : guest.plate_name) : "No meal";
    counts.set(name, (counts.get(name) ?? 0) + 1);
  }
  return [...counts].map(([name, count]) => `${name} × ${count}`).join(", ");
}

/**
 * An event's bookings (Phase 31 Decision 18): what to cook, who's coming,
 * and the waitlist. Payment actions stay on the order page, like food
 * orders; here the owner offers seats and cancels.
 */
export function AdminEventBookings({ eventId, initial }: { eventId: number; initial: AdminBookings }) {
  const [data, setData] = useState(initial);
  const [pending, setPending] = useState<Pending | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  const waitlist = data.bookings.filter((booking) => booking.status === "waitlisted");
  const seatsLeft = Math.max(0, data.capacity - data.seats_taken);

  async function run() {
    if (!pending) return;
    setBusy(true);
    setMessage(null);
    const response = await fetch(`/api/admin/events/bookings/${pending.booking.id}/${pending.action}`, { method: "POST" });
    const body: unknown = await response.json().catch(() => null);
    setBusy(false);
    setPending(null);
    if (response.ok) {
      setData(body as AdminBookings);
      return;
    }
    if (typeof body === "object" && body !== null && "error" in body && body.error === "not_enough_seats") {
      const left = (body as { seats_left?: number }).seats_left ?? 0;
      setMessage(`Not enough seats: ${left} left, and this party needs ${pending.booking.party_size}.`);
      return;
    }
    setMessage("That didn't work. Refresh the page and try again.");
  }

  return (
    <div className="flex flex-col gap-8">
      <section className="grid gap-4 sm:grid-cols-3">
        <div className="rounded-card bg-cream p-5">
          <p className="m-0 text-sm text-text-muted">Seats taken</p>
          <p className="m-0 text-3xl font-semibold">
            {data.seats_taken} <span className="text-lg text-text-muted">/ {data.capacity}</span>
          </p>
          <p className="m-0 text-sm text-text-muted">{seatsLeft} left</p>
        </div>
        <div className="rounded-card bg-cream p-5 sm:col-span-2">
          <p className="m-0 mb-2 text-sm text-text-muted">To cook (paid or free-confirmed · on an unpaid hold)</p>
          {data.plate_counts.length === 0 ? (
            <p className="m-0">No bookings yet.</p>
          ) : (
            <ul className="m-0 flex list-none flex-wrap gap-x-6 gap-y-1 p-0">
              {data.plate_counts.map((count) => (
                <li key={`${count.plate_name ?? "none"}-${count.choices}`}>
                  <span className="font-semibold">{count.plate_name ?? "No meal"}</span>
                  {count.choices && <span> — {count.choices}</span>}: {count.confirmed}
                  {count.held > 0 && <span className="text-text-muted"> + {count.held} held</span>}
                </li>
              ))}
            </ul>
          )}
        </div>
      </section>

      {message && <Notice>{message}</Notice>}

      <div className="flex flex-wrap items-center justify-between gap-3">
        <h2 className="m-0 text-xl">
          Bookings {waitlist.length > 0 && <span className="text-base text-text-muted">· {waitlist.length} on the waitlist</span>}
        </h2>
        <a href={`/api/admin/events/${eventId}/bookings.csv`} className="text-sm font-semibold text-primary underline">
          Download guest list (CSV)
        </a>
      </div>

      {data.bookings.length === 0 ? (
        <p className="m-0 text-text-muted">Nobody has booked yet. Share the link from the Share tab.</p>
      ) : (
        <ul className="m-0 flex list-none flex-col gap-3 p-0">
          {data.bookings.map((booking) => (
            <li key={booking.id} className={`flex flex-col gap-2 rounded-card border border-text/10 p-4 ${booking.status === "cancelled" ? "opacity-70" : ""}`}>
              <div className="flex flex-wrap items-center justify-between gap-2">
                <span className="text-lg font-semibold">
                  {booking.customer_name} <span className="text-sm font-normal text-text-muted">· party of {booking.party_size}</span>
                </span>
                <span className={`rounded-full px-3 py-1 text-xs font-semibold ${STATUS_CLASSES[booking.status]}`}>
                  {BOOKING_STATUS_LABELS[booking.status]}
                  {booking.cancel_reason && ` (${CANCEL_REASONS[booking.cancel_reason]})`}
                </span>
              </div>
              <p className="m-0 text-sm">{plateSummary(booking)}</p>
              <p className="m-0 text-sm text-text-muted">
                <a href={`tel:${booking.customer_phone}`}>{booking.customer_phone}</a> ·{" "}
                <a href={`mailto:${booking.customer_email}`}>{booking.customer_email}</a> · booked {formatDateTime(booking.created_at)}
              </p>
              {booking.notes && <p className="m-0 text-sm">“{booking.notes}”</p>}
              <div className="flex flex-wrap items-center gap-3 text-sm">
                {booking.order_id !== null && (
                  <Link href={`/orders/${booking.order_id}`} className="font-semibold text-primary underline">
                    Order {booking.payment_reference} · {formatPrice(booking.total)}
                    {booking.order_status === "payment_claimed" && " · says paid"}
                  </Link>
                )}
                {booking.status === "awaiting_payment" && booking.hold_expires_at && (
                  <span className="text-text-muted">Held until {formatDateTime(booking.hold_expires_at)}</span>
                )}
                <span className="flex-1" />
                {(booking.status === "waitlisted" || booking.status === "cancelled") && (
                  <Button type="button" variant="outline" className="px-4 py-2" onClick={() => setPending({ booking, action: "offer" })}>
                    Offer seats
                  </Button>
                )}
                {booking.status !== "cancelled" && (
                  <Button type="button" variant="secondary" className="px-4 py-2" onClick={() => setPending({ booking, action: "cancel" })}>
                    Cancel
                  </Button>
                )}
              </div>
            </li>
          ))}
        </ul>
      )}

      <AdminConfirmDialog
        open={pending !== null}
        title={pending?.action === "offer" ? "Offer seats?" : "Cancel this booking?"}
        confirmLabel={pending?.action === "offer" ? "Offer seats" : "Cancel booking"}
        busy={busy}
        onConfirm={() => void run()}
        onCancel={() => setPending(null)}
      >
        {pending?.action === "offer"
          ? `${pending.booking.customer_name} gets ${pending.booking.party_size} seat(s) and an email. With meals, they get a payment request and a new payment window.`
          : `${pending?.booking.customer_name ?? "The guest"}'s seats are freed and they get an email. If they already paid, arrange the refund yourself.`}
      </AdminConfirmDialog>
    </div>
  );
}
