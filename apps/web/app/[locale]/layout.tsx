import type { Metadata } from "next";
import { NextIntlClientProvider, hasLocale } from "next-intl";
import { notFound } from "next/navigation";
import { Fraunces, Work_Sans } from "next/font/google";
import { routing } from "@/i18n/routing";
import { UtilityBar } from "@/components/utility-bar";
import { SiteHeader } from "@/components/site-header";
import { SiteFooter } from "@/components/site-footer";
import { MotionProvider } from "@/components/motion-provider";
import { PageTransition } from "@/components/page-transition";
import "../globals.css";

const displayFont = Fraunces({
  subsets: ["latin"],
  variable: "--font-display",
  weight: ["600", "700"],
});
const bodyFont = Work_Sans({
  subsets: ["latin"],
  variable: "--font-body",
});

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
          <MotionProvider />
          <UtilityBar />
          <SiteHeader />
          <PageTransition>{children}</PageTransition>
          <SiteFooter />
        </NextIntlClientProvider>
      </body>
    </html>
  );
}
