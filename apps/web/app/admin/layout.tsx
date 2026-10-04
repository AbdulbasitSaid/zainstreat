import type { Metadata } from "next";
import { bodyFont, displayFont } from "@/lib/fonts";
import "../globals.css";

export const metadata: Metadata = {
  title: "Admin | Zain's Treat n More",
};

export default function AdminLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className={`${displayFont.variable} ${bodyFont.variable}`}>
      <body className="bg-background">{children}</body>
    </html>
  );
}
