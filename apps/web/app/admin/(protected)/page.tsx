import type { Metadata } from "next";
import { getCurrentAdmin } from "@/lib/admin-auth";
import { AdminLogoutButton } from "@/components/admin-logout-button";

export const metadata: Metadata = {
  title: "Admin | Zain's Treat n More",
};

export default async function AdminHomePage() {
  const admin = await getCurrentAdmin();

  return (
    <main className="container flex flex-col items-start gap-4 py-16">
      <h1 className="m-0 text-2xl font-semibold">Welcome, {admin?.name}</h1>
      <p className="m-0 text-text-muted">Logged in as {admin?.email}.</p>
      <AdminLogoutButton />
    </main>
  );
}
