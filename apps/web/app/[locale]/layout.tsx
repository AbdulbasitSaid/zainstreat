import type { Metadata } from "next";
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
import { CartProvider } from "@/lib/cart-context";
import { bodyFont, displayFont } from "@/lib/fonts";
import "../globals.css";

export const metadata: Metadata = {
  title: "Zain's Treat n More",
};

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

  return (
    <html lang={locale} className={`${displayFont.variable} ${bodyFont.variable}`}>
      <body>
        <NextIntlClientProvider>
          <CartProvider>
            <MotionProvider />
            <CustomCursor />
            <UtilityBar />
            <SiteHeader />
            <PageTransition>{children}</PageTransition>
            <SiteFooter />
            <WhatsAppFloatButton />
          </CartProvider>
        </NextIntlClientProvider>
      </body>
    </html>
  );
}
