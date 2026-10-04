import { Fraunces, Work_Sans } from "next/font/google";

export const displayFont = Fraunces({
  subsets: ["latin"],
  variable: "--font-display",
  weight: ["600", "700"],
});

export const bodyFont = Work_Sans({
  subsets: ["latin"],
  variable: "--font-body",
});
