import type { Metadata } from "next";
import { draftMode } from "next/headers";
import { NextIntlClientProvider, hasLocale } from "next-intl";
import { notFound } from "next/navigation";
import { routing } from "@/i18n/routing";
import { UtilityBar } from "@/components/utility-bar";
import { SiteHeader } from "@/components/site-header";
import { SiteFooter } from "@/components/site-footer";
import { MotionProvider } from "@/components/motion-provider";
import { PageTransition } from "@/components/page-transition";
import { CustomCursor } from "@/components/custom-cursor";
import { WhatsAppFloatButton } from "@/components/whatsapp-float-button";
import { PreviewBridge } from "@/components/preview-bridge";
import { CartProvider } from "@/lib/cart-context";
import { getSiteContent } from "@/lib/api";
import { PreviewProvider } from "@/lib/preview-context";
import { siteImage } from "@/lib/site-content";
import { bodyFont, displayFont } from "@/lib/fonts";
import "../globals.css";

export async function generateMetadata(): Promise<Metadata> {
  // The Open Graph image follows the admin's logo slot (Phase 25 Decision 13).
  const logo = siteImage(await getSiteContent(), "logo");

  return {
    metadataBase: new URL(process.env.DOMAIN ? `https://${process.env.DOMAIN}` : "http://localhost:3000"),
    title: "Zain's Treat n More",
    description:
      "Halal meals, snacks, catering, and event rentals from Zain's Treat n More — browse the menu and order online.",
    openGraph: {
      siteName: "Zain's Treat n More",
      images: [logo ?? "/logo.png"],
    },
  };
}

export function generateStaticParams() {
  return routing.locales.map((locale) => ({ locale }));
}

export default async function LocaleLayout({
  children,
  params,
}: {
  children: React.ReactNode;
  params: Promise<{ locale: string }>;
}) {
  const { locale } = await params;
  if (!hasLocale(routing.locales, locale)) {
    notFound();
  }
  const [content, { isEnabled: preview }] = await Promise.all([getSiteContent(), draftMode()]);

  return (
    <html lang={locale} className={`${displayFont.variable} ${bodyFont.variable}`}>
      <body>
        <NextIntlClientProvider>
          <PreviewProvider preview={preview}>
            <CartProvider>
              <MotionProvider />
              <CustomCursor />
              <UtilityBar />
              <SiteHeader logoSrc={siteImage(content, "logo")} />
              <PageTransition>{children}</PageTransition>
              <SiteFooter />
              <WhatsAppFloatButton />
              {/* Draft mode only (cms host): no extra JS for visitors. */}
              {preview && <PreviewBridge />}
            </CartProvider>
          </PreviewProvider>
        </NextIntlClientProvider>
      </body>
    </html>
  );
}
