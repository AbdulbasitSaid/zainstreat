import type { Metadata } from "next";
import { AdminInstallHint } from "@/components/admin-install-hint";
import { AdminNotifications } from "@/components/admin-notifications";
import { AdminDeliverySettingsForm } from "@/components/admin-delivery-settings";
import { AdminTranslationSettingsForm } from "@/components/admin-translation-settings";
import { adminApiJson } from "@/lib/admin-api";
import { getAdminDeliverySettings } from "@/lib/admin-orders-api";
import type { TranslationSettingsStatus } from "@/lib/admin-translations";

export const metadata: Metadata = { title: "Settings | Zain's Admin" };

// Phase 27 Decision 9; Delivery from Phase 37 Decision 15; Translations
// from Phase 38 Decision 20.
export default async function AdminSettingsPage() {
  const [delivery, translations] = await Promise.all([
    getAdminDeliverySettings(),
    adminApiJson<TranslationSettingsStatus>("/api/admin/translations/status"),
  ]);

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
      <section id="delivery" className="flex max-w-2xl scroll-mt-20 flex-col gap-4">
        <h2 className="m-0 text-xl">Delivery</h2>
        <p className="m-0 text-sm text-text-muted">
          What customers pay for delivery. They pick their address at checkout and see the fee before
          ordering.
        </p>
        <AdminDeliverySettingsForm initialValue={delivery} />
      </section>
      <section id="translations" className="flex max-w-2xl scroll-mt-20 flex-col gap-4">
        <h2 className="m-0 text-xl">Translations</h2>
        <p className="m-0 text-sm text-text-muted">
          Dutch and French menu text comes from DeepL when you publish (rentals: when you save). Your own
          wording in an item&apos;s Dutch or French tab is never overwritten.
        </p>
        <AdminTranslationSettingsForm initialValue={translations} />
      </section>
    </div>
  );
}
