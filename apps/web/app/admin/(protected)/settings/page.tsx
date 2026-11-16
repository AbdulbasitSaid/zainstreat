import type { Metadata } from "next";
import { AdminInstallHint } from "@/components/admin-install-hint";

export const metadata: Metadata = { title: "Settings | Zain's Admin" };

// Phase 27 Decision 9. The notification controls join the install hint in
// plan Group 5.
export default function AdminSettingsPage() {
  return (
    <div className="flex flex-col gap-8">
      <h1 className="m-0">Settings</h1>
      <section className="flex max-w-2xl flex-col gap-4">
        <h2 className="m-0 text-xl">Notifications</h2>
        <AdminInstallHint />
      </section>
    </div>
  );
}
