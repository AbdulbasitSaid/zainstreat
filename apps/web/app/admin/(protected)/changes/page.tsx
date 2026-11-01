import type { Metadata } from "next";
import { adminApiJson } from "@/lib/admin-api";
import { AdminChangesTable } from "@/components/admin-changes-table";
import type { ChangeEntry } from "@/lib/admin-changes";

export const metadata: Metadata = {
  title: "Changes | Zain's Admin",
};

export default async function AdminChangesPage() {
  const entries = await adminApiJson<ChangeEntry[]>("/api/admin/changes");

  return (
    <div className="flex flex-col gap-8">
      <div>
        <h1 className="m-0">Changes</h1>
        <p className="m-0 mt-2 text-text-muted">
          Edits to the site content, menu and categories wait here until they&apos;re published.
        </p>
      </div>
      <AdminChangesTable entries={entries} />
    </div>
  );
}
