import type { Metadata } from "next";
import { AdminInstallHint } from "@/components/admin-install-hint";
import { AdminNotifications } from "@/components/admin-notifications";

export const metadata: Metadata = { title: "Settings | Zain's Admin" };

// Phase 27 Decision 9.
export default function AdminSettingsPage() {
  return (
    <div className="flex flex-col gap-8">
      <h1 className="m-0">Settings</h1>
      <section className="flex max-w-2xl flex-col gap-4">
        <h2 className="m-0 text-xl">Notifications</h2>
        <p className="m-0 text-sm text-text-muted">
          Get a notification for new orders, payment claims, quote replies, catering enquiries and
          contact messages, even when the admin is closed. Turn it on separately on each phone or
          computer.
        </p>
        <AdminInstallHint />
        <AdminNotifications />
      </section>
    </div>
  );
}
