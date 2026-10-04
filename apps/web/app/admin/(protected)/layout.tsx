import { redirect } from "next/navigation";
import { getCurrentAdmin } from "@/lib/admin-auth";
import { AdminNav } from "@/components/admin-nav";
import { AdminLogoutButton } from "@/components/admin-logout-button";

export const dynamic = "force-dynamic";

export default async function ProtectedAdminLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const admin = await getCurrentAdmin();
  if (!admin) {
    redirect("/admin/login");
  }

  return (
    <div className="min-h-dvh md:grid md:grid-cols-[15rem_1fr]">
      <aside className="flex flex-col gap-6 border-b border-text/10 bg-background-soft/40 p-5 md:min-h-dvh md:border-b-0 md:border-r">
        <p className="m-0 text-base font-semibold text-primary">Zain&apos;s Admin</p>
        <AdminNav />
        <div className="flex flex-col items-start gap-3 pt-4 md:mt-auto">
          <p className="m-0 text-xs text-text-muted">Signed in as {admin.name}</p>
          <AdminLogoutButton />
        </div>
      </aside>
      <main className="p-5 md:p-10">{children}</main>
    </div>
  );
}
