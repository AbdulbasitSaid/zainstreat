import { notFound } from "next/navigation";
import type { Metadata } from "next";
import { AdminAttendeeSheetToolbar } from "@/components/admin-attendee-sheet-toolbar";
import { getSiteContent } from "@/lib/api";
import { getAdminEventAttendees } from "@/lib/admin-events-api";
import type { AttendeePlateCount } from "@/lib/admin-events";
import { siteImage } from "@/lib/site-content";

function eventId(raw: string): number {
  const id = Number.parseInt(raw, 10);
  if (!Number.isInteger(id) || id < 1) notFound();
  return id;
}

// The title is what "Save as PDF" suggests as the file name.
export async function generateMetadata({ params }: { params: Promise<{ id: string }> }): Promise<Metadata> {
  const data = await getAdminEventAttendees(eventId((await params).id));
  return { title: data ? `${data.event.slug}-attendees` : "Attendees | Zain's Admin" };
}

function plateLabel(count: Pick<AttendeePlateCount, "plate_name" | "choices">): string {
  const name = count.plate_name ?? "No meal";
  return count.choices ? `${name} (${count.choices})` : name;
}

const PRINTED_AT = new Intl.DateTimeFormat("en-GB", {
  timeZone: "Europe/Amsterdam",
  day: "numeric",
  month: "short",
  year: "numeric",
  hour: "2-digit",
  minute: "2-digit",
});

/**
 * Phase 38 Decision 15: the check-in sheet preview, in the site's own
 * fonts and colours; the admin shell is hidden when printing (protected
 * layout). The toolbar's "Download PDF" is the real file (Decision 27).
 */
export default async function AdminAttendeesPage({ params }: { params: Promise<{ id: string }> }) {
  const id = eventId((await params).id);
  const [data, content] = await Promise.all([getAdminEventAttendees(id), getSiteContent()]);
  if (!data) notFound();
  const logo = siteImage(content, "logo") ?? "/logo.png";

  return (
    <div className="mx-auto flex max-w-4xl flex-col gap-6 print:max-w-none print:gap-4">
      <style>{`
        @page { size: A4 portrait; margin: 14mm; }
        @media print {
          html, body { background: #fff; }
          .attendee-sheet * { -webkit-print-color-adjust: exact; print-color-adjust: exact; }
          .attendee-sheet thead { display: table-header-group; }
          .attendee-sheet tr { break-inside: avoid; }
        }
      `}</style>
      <AdminAttendeeSheetToolbar eventId={id} />

      {!data.event.bookings_closed && (
        <p className="m-0 rounded-card bg-accent-light px-5 py-3 text-sm print:hidden">
          Bookings are still open, so this list may still change.
        </p>
      )}

      <article className="attendee-sheet flex flex-col gap-5 text-text">
        <header className="flex items-center gap-5 rounded-card bg-primary px-6 py-5 text-white">
          {/* eslint-disable-next-line @next/next/no-img-element -- print needs the plain image, no lazy loading */}
          <img src={logo} alt="Zain's Treat n More" className="h-16 w-16 shrink-0 rounded-full bg-white object-contain p-1" />
          <div className="flex min-w-0 flex-col gap-1">
            <p className="m-0 text-xs font-semibold uppercase tracking-[0.2em] text-white/80">Check-in sheet</p>
            <h1 className="m-0 text-[clamp(1.6rem,4vw,2.25rem)] text-white">{data.event.title}</h1>
            <p className="m-0 text-sm text-white/90">
              {data.event.starts_at_label} · {data.event.area} · {data.confirmed_guests}{" "}
              {data.confirmed_guests === 1 ? "guest" : "guests"}
            </p>
          </div>
        </header>

        {data.plate_counts.length > 0 && (
          <section aria-label="Plates to cook" className="rounded-card border border-text/15 bg-cream px-5 py-4">
            <h2 className="m-0 mb-2 text-lg">Plates</h2>
            <p className="m-0 text-sm">
              {data.plate_counts.map((count, index) => (
                <span key={`${count.plate_name ?? "none"}-${count.choices}`}>
                  {index > 0 && <span className="text-text-muted"> · </span>}
                  <span className="font-semibold">{plateLabel(count)}</span> × {count.count}
                </span>
              ))}
            </p>
          </section>
        )}

        {data.guests.length === 0 ? (
          <p className="m-0 text-text-muted">No confirmed guests yet.</p>
        ) : (
          <table className="w-full border-collapse text-sm">
            <thead>
              <tr className="border-b-2 border-primary text-left text-xs uppercase tracking-wider text-primary">
                <th scope="col" className="w-10 py-2 pr-2 font-semibold">
                  <span className="sr-only">Checked in</span>✓
                </th>
                <th scope="col" className="py-2 pr-3 font-semibold">Name</th>
                <th scope="col" className="py-2 pr-3 font-semibold">Guest</th>
                <th scope="col" className="py-2 pr-3 font-semibold">Plate</th>
                <th scope="col" className="py-2 pr-3 font-semibold">Phone</th>
                <th scope="col" className="py-2 font-semibold">Note</th>
              </tr>
            </thead>
            <tbody>
              {data.guests.map((guest) => (
                <tr
                  key={`${guest.booker_name}-${guest.payment_reference ?? ""}-${guest.guest_no}`}
                  className={`border-b border-text/10 align-top ${guest.guest_no === 1 ? "border-t border-t-text/25" : ""}`}
                >
                  <td className="py-2 pr-2">
                    <span aria-hidden className="inline-block size-5 rounded border-2 border-primary" />
                  </td>
                  <td className="py-2 pr-3 font-semibold">{guest.guest_no === 1 ? guest.booker_name : ""}</td>
                  <td className="py-2 pr-3 whitespace-nowrap text-text-muted">
                    {guest.guest_no} of {guest.party_size}
                  </td>
                  <td className="py-2 pr-3">{plateLabel(guest)}</td>
                  <td className="py-2 pr-3 whitespace-nowrap">{guest.guest_no === 1 ? guest.phone : ""}</td>
                  <td className="py-2">{guest.note ?? ""}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}

        <footer className="border-t border-text/15 pt-3 text-xs text-text-muted">
          Zain&apos;s Treat n More · printed {PRINTED_AT.format(new Date())}
        </footer>
      </article>
    </div>
  );
}
