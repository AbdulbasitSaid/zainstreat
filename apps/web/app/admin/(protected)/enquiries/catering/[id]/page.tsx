import Link from "next/link";
import { notFound } from "next/navigation";
import type { Metadata } from "next";
import { getAdminCateringEnquiry } from "@/lib/admin-enquiries-api";
import { formatDateTime } from "@/lib/format";

export const metadata: Metadata = {
  title: "Catering Enquiry | Zain's Admin",
};

function Field({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="m-0 text-xs font-semibold uppercase tracking-wider text-text-muted">{label}</p>
      <p className="m-0">{value}</p>
    </div>
  );
}

export default async function AdminCateringEnquiryDetailPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = await params;
  const enquiryId = Number.parseInt(id, 10);
  if (!Number.isInteger(enquiryId) || enquiryId < 1) {
    notFound();
  }

  const enquiry = await getAdminCateringEnquiry(enquiryId);

  return (
    <div className="flex flex-col gap-8">
      <Link href="/admin/enquiries?type=catering" className="text-sm font-semibold text-primary underline">
        ‹ Back to enquiries
      </Link>

      <h1 className="m-0">Catering Enquiry #{enquiry.id}</h1>

      <div className="grid gap-5 sm:grid-cols-2 lg:grid-cols-3">
        <Field label="Name" value={enquiry.name} />
        <Field label="Phone" value={enquiry.phone} />
        <Field label="Email" value={enquiry.email} />
        <Field label="Event type" value={enquiry.event_type} />
        <Field label="Event date" value={enquiry.event_date} />
        <Field label="Guests" value={String(enquiry.guest_count)} />
        <Field label="Location" value={enquiry.location} />
        <Field label="Services required" value={enquiry.services_required.join(", ")} />
        <Field label="Submitted" value={formatDateTime(enquiry.created_at)} />
      </div>

      <div>
        <p className="m-0 text-xs font-semibold uppercase tracking-wider text-text-muted">Message</p>
        <p className="m-0 whitespace-pre-wrap">{enquiry.message?.trim() || "—"}</p>
      </div>
    </div>
  );
}
