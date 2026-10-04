import Link from "next/link";
import { notFound } from "next/navigation";
import type { Metadata } from "next";
import { getAdminContactMessage } from "@/lib/admin-enquiries-api";
import { formatDateTime } from "@/lib/format";

export const metadata: Metadata = {
  title: "Contact Message | Zain's Admin",
};

function Field({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="m-0 text-xs font-semibold uppercase tracking-wider text-text-muted">{label}</p>
      <p className="m-0">{value}</p>
    </div>
  );
}

export default async function AdminContactMessageDetailPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = await params;
  const messageId = Number.parseInt(id, 10);
  if (!Number.isInteger(messageId) || messageId < 1) {
    notFound();
  }

  const message = await getAdminContactMessage(messageId);

  return (
    <div className="flex flex-col gap-8">
      <Link href="/admin/enquiries?type=contact" className="text-sm font-semibold text-primary underline">
        ‹ Back to enquiries
      </Link>

      <h1 className="m-0">Contact Message #{message.id}</h1>

      <div className="grid gap-5 sm:grid-cols-2 lg:grid-cols-3">
        <Field label="Name" value={message.name} />
        <Field label="Email" value={message.email} />
        <Field label="Phone" value={message.phone} />
        <Field label="Subject" value={message.subject} />
        <Field label="Submitted" value={formatDateTime(message.created_at)} />
      </div>

      <div>
        <p className="m-0 text-xs font-semibold uppercase tracking-wider text-text-muted">Message</p>
        <p className="m-0 whitespace-pre-wrap">{message.message}</p>
      </div>
    </div>
  );
}
