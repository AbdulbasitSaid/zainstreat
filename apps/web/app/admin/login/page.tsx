import type { Metadata } from "next";
import { AdminLoginForm } from "@/components/admin-login-form";

export const metadata: Metadata = {
  title: "Admin Login | Zain's Treat n More",
};

export default function AdminLoginPage() {
  return (
    <main className="container">
      <AdminLoginForm />
    </main>
  );
}
