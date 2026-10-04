import Link from "next/link";
import type { Metadata } from "next";
import { ENQUIRIES_PAGE_SIZE } from "@/lib/admin-enquiries";
import { getAdminCateringEnquiries, getAdminContactMessages } from "@/lib/admin-enquiries-api";
import { formatDateTime } from "@/lib/format";

export const metadata: Metadata = { title: "Enquiries | Zain's Admin" };

export default async function AdminEnquiriesPage({
  searchParams,
}: {
  searchParams: Promise<{ type?: string; page?: string }>;
}) {
  const { type: rawType, page: rawPage } = await searchParams;
  const type = rawType === "contact" ? "contact" : "catering";
  const parsedPage = Number.parseInt(rawPage ?? "1", 10);
  const page = Number.isFinite(parsedPage) && parsedPage > 0 ? parsedPage : 1;

  const [catering, contact] = await Promise.all([
    type === "catering" ? getAdminCateringEnquiries(page) : null,
    type === "contact" ? getAdminContactMessages(page) : null,
  ]);

  const total = catering?.total ?? contact?.total ?? 0;
  const pageCount = Math.max(1, Math.ceil(total / ENQUIRIES_PAGE_SIZE));

  return (
    <div className="flex flex-col gap-8">
      <h1 className="m-0">Enquiries</h1>

      <div className="flex gap-2">
        {(["catering", "contact"] as const).map((value) => (
          <Link
            key={value}
            href={`/admin/enquiries?type=${value}`}
            aria-current={type === value ? "page" : undefined}
            className={`rounded-full px-4 py-2 text-xs font-semibold uppercase tracking-wider ${
              type === value ? "bg-primary text-white" : "bg-card text-text hover:bg-background-soft"
            }`}
          >
            {value === "catering" ? "Catering" : "Contact"}
          </Link>
        ))}
      </div>

      {type === "catering" ? (
        catering!.enquiries.length === 0 ? (
          <p className="m-0 text-text-muted">No catering enquiries yet.</p>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full border-collapse text-sm">
              <thead>
                <tr className="border-b border-text/15 text-left text-xs uppercase tracking-wider text-text-muted">
                  <th scope="col" className="py-3 pr-4 font-semibold">Name</th>
                  <th scope="col" className="py-3 pr-4 font-semibold">Event</th>
                  <th scope="col" className="py-3 pr-4 font-semibold">Date</th>
                  <th scope="col" className="py-3 pr-4 font-semibold">Guests</th>
                  <th scope="col" className="py-3 font-semibold">Submitted</th>
                </tr>
              </thead>
              <tbody>
                {catering!.enquiries.map((e) => (
                  <tr key={e.id} className="border-b border-text/10 hover:bg-background-soft">
                    <td className="py-3 pr-4">
                      <Link href={`/admin/enquiries/catering/${e.id}`} className="font-semibold text-primary underline">
                        {e.name}
                      </Link>
                    </td>
                    <td className="py-3 pr-4">{e.event_type}</td>
                    <td className="py-3 pr-4">{e.event_date}</td>
                    <td className="py-3 pr-4">{e.guest_count}</td>
                    <td className="py-3">{formatDateTime(e.created_at)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )
      ) : contact!.messages.length === 0 ? (
        <p className="m-0 text-text-muted">No contact messages yet.</p>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full border-collapse text-sm">
            <thead>
              <tr className="border-b border-text/15 text-left text-xs uppercase tracking-wider text-text-muted">
                <th scope="col" className="py-3 pr-4 font-semibold">Name</th>
                <th scope="col" className="py-3 pr-4 font-semibold">Subject</th>
                <th scope="col" className="py-3 font-semibold">Submitted</th>
              </tr>
            </thead>
            <tbody>
              {contact!.messages.map((m) => (
                <tr key={m.id} className="border-b border-text/10 hover:bg-background-soft">
                  <td className="py-3 pr-4">
                    <Link href={`/admin/enquiries/contact/${m.id}`} className="font-semibold text-primary underline">
                      {m.name}
                    </Link>
                  </td>
                  <td className="py-3 pr-4">{m.subject}</td>
                  <td className="py-3">{formatDateTime(m.created_at)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {pageCount > 1 && (
        <nav aria-label="Pagination" className="flex items-center justify-center gap-6 text-sm">
          {page > 1 ? (
            <Link href={`/admin/enquiries?type=${type}&page=${page - 1}`} className="font-semibold text-primary underline">
              ‹ Previous
            </Link>
          ) : (
            <span className="text-text-muted opacity-50">‹ Previous</span>
          )}
          <span className="text-text-muted">Page {page} of {pageCount}</span>
          {page < pageCount ? (
            <Link href={`/admin/enquiries?type=${type}&page=${page + 1}`} className="font-semibold text-primary underline">
              Next ›
            </Link>
          ) : (
            <span className="text-text-muted opacity-50">Next ›</span>
          )}
        </nav>
      )}
    </div>
  );
}
