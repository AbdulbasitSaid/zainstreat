import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { getTranslations } from "next-intl/server";
import { Link } from "@/i18n/navigation";
import { BookingCancelButton } from "@/components/booking-cancel-button";
import { ButtonAnchor } from "@/components/button";
import { Notice } from "@/components/notice";
import { OrderAutoRefresh } from "@/components/order-auto-refresh";
import { OrderInvoice } from "@/components/order-invoice";
import { PageHero } from "@/components/page-hero";
import { PaymentClaimButton } from "@/components/payment-claim-button";
import { eventDateParts, type PublicBooking } from "@/lib/events";
import { getBooking } from "@/lib/events-api";
import { formatPrice } from "@/lib/format";
import { buildWhatsAppLink } from "@/lib/whatsapp";

// The guest's link (Phase 31 Decision 11). Like /order/[token], the token
// is the only protection: no indexing, no Referer on outbound links.
export async function generateMetadata({ params }: { params: Promise<{ locale: string }> }): Promise<Metadata> {
  const { locale } = await params;
  const t = await getTranslations({ locale, namespace: "BookingPage" });
  return { title: t("metaTitle"), robots: { index: false, follow: false }, referrer: "no-referrer" };
}

type StepState = "done" | "current" | "todo";

// Booked → Paid → Confirmed; a free RSVP has no payment step.
function Steps({ booking, labels }: { booking: PublicBooking; labels: [string, string, string] }) {
  const [booked, paidLabel, confirmedLabel] = labels;
  const confirmed = booking.status === "confirmed";
  const claimed = booking.order?.status === "payment_claimed";
  const steps: { label: string; state: StepState }[] = booking.order
    ? [
        { label: booked, state: "done" },
        { label: paidLabel, state: confirmed || claimed ? "done" : "current" },
        { label: confirmedLabel, state: confirmed ? "done" : claimed ? "current" : "todo" },
      ]
    : [
        { label: booked, state: "done" },
        { label: confirmedLabel, state: confirmed ? "done" : "current" },
      ];

  return (
    <ol className="m-0 flex list-none justify-center gap-2 p-0 sm:gap-4">
      {steps.map(({ label, state }, index) => (
        <li key={label} aria-current={state === "current" ? "step" : undefined} className="flex items-center gap-2 text-sm">
          <span
            aria-hidden="true"
            className={`flex size-7 items-center justify-center rounded-full font-bold ${
              state === "done"
                ? "bg-primary text-white"
                : state === "current"
                  ? "border-2 border-primary text-primary"
                  : "border-2 border-text-muted/30"
            }`}
          >
            {state === "done" ? "✓" : index + 1}
          </span>
          <span className={state === "todo" ? "text-text-muted" : "font-semibold"}>{label}</span>
        </li>
      ))}
    </ol>
  );
}

export default async function BookingPage({ params }: { params: Promise<{ locale: string; token: string }> }) {
  const { locale, token } = await params;
  const booking = await getBooking(token, locale);
  if (!booking) notFound();

  const t = await getTranslations("BookingPage");
  const { long, time } = eventDateParts(booking.event.starts_at, locale);
  const deadline = booking.hold_expires_at ? eventDateParts(booking.hold_expires_at, locale) : null;
  const claimed = booking.order?.status === "payment_claimed";
  const paid = booking.order?.status === "processing" || booking.order?.status === "completed";
  const open = booking.status !== "cancelled";

  const headline = {
    waitlisted: t("statusWaitlisted"),
    awaiting_payment: claimed ? t("statusClaimed") : t("statusAwaiting"),
    confirmed: t("statusConfirmed"),
    cancelled: t("statusCancelled"),
  }[booking.status];
  const body =
    booking.status === "awaiting_payment" && !claimed && deadline
      ? t("statusAwaitingBody", { total: formatPrice(booking.total), date: deadline.long, time: deadline.time })
      : booking.status === "awaiting_payment"
        ? t("statusClaimedBody")
        : booking.status === "confirmed"
          ? t("statusConfirmedBody")
          : booking.status === "waitlisted"
            ? t("statusWaitlistedBody", { count: booking.party_size })
            : t(`cancelled.${booking.cancel_reason ?? "admin"}`);

  return (
    <main className="container">
      <PageHero>
        <p className="m-0 text-sm font-semibold tracking-[0.14em] text-primary uppercase">{booking.event.title}</p>
        <h1 className="text-[clamp(2.25rem,5vw,3.75rem)]">{headline}</h1>
        <p className="text-lg text-text-muted">{body}</p>
      </PageHero>

      <div className="mx-auto flex max-w-[680px] flex-col gap-8 pb-16">
        {(booking.status === "awaiting_payment" || booking.status === "confirmed") && (
          <Steps booking={booking} labels={[t("stepBooked"), t("stepPaid"), t("stepConfirmed")]} />
        )}

        <section className="flex flex-col gap-3 rounded-card bg-cream p-6">
          <h2 className="m-0 text-2xl">{booking.event.title}</h2>
          <p className="m-0">
            <span className="capitalize">{long}</span> · {time}
          </p>
          <p className="m-0 text-text-muted">{booking.event.area}</p>
          {booking.address && (
            <div className="mt-2 flex flex-col gap-2 rounded-card bg-background p-4">
              <span className="text-xs font-semibold tracking-wider text-text-muted uppercase">{t("addressHeading")}</span>
              <span className="text-lg font-semibold">{booking.address}</span>
              <a
                href={`https://maps.google.com/?q=${encodeURIComponent(booking.address)}`}
                target="_blank"
                rel="noopener noreferrer"
                className="text-sm font-semibold text-primary"
              >
                {t("openMap")} →
              </a>
            </div>
          )}
          <Link href={`/events/${booking.event.slug}`} className="text-sm font-semibold text-primary">
            {t("viewEvent")}
          </Link>
        </section>

        <section className="flex flex-col gap-3">
          <h2 className="m-0 text-xl">{t("partyHeading", { count: booking.party_size })}</h2>
          <ul className="m-0 flex list-none flex-col gap-2 p-0">
            {booking.guests.map((guest, index) => (
              <li key={index} className="flex justify-between gap-3 border-b border-text/10 pb-2">
                <span>
                  {t("guestN", { n: index + 1 })}: {guest.plate_name ?? t("noMeal")}
                  {guest.choices && <span className="block text-sm text-text-muted">{guest.choices}</span>}
                </span>
                {guest.unit_price && <span className="font-semibold">{formatPrice(guest.unit_price)}</span>}
              </li>
            ))}
          </ul>
          {booking.notes && <p className="m-0 text-sm text-text-muted">“{booking.notes}”</p>}
        </section>

        {booking.order && booking.status === "awaiting_payment" && (
          <section className="flex flex-col gap-6 text-center">
            <PaymentClaimButton token={booking.order.public_token} status={booking.order.status} />
            <OrderInvoice order={booking.order} />
          </section>
        )}

        {booking.event.terms && open && (
          <details className="rounded-card bg-background-soft px-5 py-4">
            <summary className="cursor-pointer font-semibold">{t("termsHeading")}</summary>
            <p className="m-0 mt-3 text-sm whitespace-pre-line">{booking.event.terms}</p>
          </details>
        )}

        {open &&
          (booking.guest_can_cancel ? (
            <div className="flex justify-center">
              <BookingCancelButton token={booking.token} paid={paid || claimed} />
            </div>
          ) : (
            <div className="flex flex-col items-center gap-3 text-center">
              <Notice>{t("cancelAfterCutoff")}</Notice>
              <ButtonAnchor
                href={buildWhatsAppLink(t("whatsappMessage", { title: booking.event.title }))}
                target="_blank"
                rel="noopener noreferrer"
                variant="outline"
                color="whatsapp"
              >
                {t("contactUs")}
              </ButtonAnchor>
            </div>
          ))}

        {open && <OrderAutoRefresh />}
      </div>
    </main>
  );
}
