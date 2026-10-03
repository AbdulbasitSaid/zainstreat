import type { Metadata } from "next";
import "../globals.css";

export const metadata: Metadata = {
  title: "Admin | Zain's Treat n More",
};

export default function AdminLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
