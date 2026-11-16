import type { Metadata, Viewport } from "next";
import { bodyFont, displayFont } from "@/lib/fonts";
import "../globals.css";

// Installable admin (Phase 27 Decisions 1–3). The manifest is linked here
// only, never from the public site's layout.
export const metadata: Metadata = {
  title: "Admin | Zain's Treat n More",
  manifest: "/admin.webmanifest",
  appleWebApp: { capable: true, title: "Zain's Admin", statusBarStyle: "default" },
  icons: { apple: "/admin-icons/apple-touch-icon.png" },
};

export const viewport: Viewport = {
  themeColor: "#72123C",
  viewportFit: "cover",
};

export default function AdminLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className={`${displayFont.variable} ${bodyFont.variable}`}>
      <body className="bg-background">{children}</body>
    </html>
  );
}
