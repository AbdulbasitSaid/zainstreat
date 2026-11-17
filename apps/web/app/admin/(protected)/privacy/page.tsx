import type { Metadata } from "next";
import { AdminPrivacySearch } from "@/components/admin-privacy-search";

export const metadata: Metadata = { title: "Privacy | Zain's Admin" };

// Phase 33 Decision 16: answering GDPR access and erasure requests.
export default function AdminPrivacyPage() {
  return (
    <div className="flex flex-col gap-8">
      <h1 className="m-0">Privacy</h1>
      <p className="m-0 max-w-2xl text-sm text-text-muted">
        When a customer asks what you hold about them, or asks you to delete it, search for their
        email address here. <strong>Export</strong> downloads everything as a file you can send them.{" "}
        <strong>Anonymise</strong> removes their name and contact details for good; amounts stay
        for your books. Answer within one month of the request.
      </p>
      <AdminPrivacySearch />
    </div>
  );
}
