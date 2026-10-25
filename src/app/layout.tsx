import type { Metadata } from "next";
import { Geist, Geist_Mono } from "next/font/google";
import "./globals.css";
import { Toaster } from "@/components/ui/toaster";

const geistSans = Geist({
  variable: "--font-geist-sans",
  subsets: ["latin"],
});

const geistMono = Geist_Mono({
  variable: "--font-geist-mono",
  subsets: ["latin"],
});

export const metadata: Metadata = {
  title: "قاعدة دوال فورتنايت أندرويد — Fortnite Android Functions DB",
  description:
    "كل دوال فورتنايت أندرويد ARM64 (49,859 دالة native) عبر 11 نسخة من 5.41 إلى 30.00 + منفذ Erbium بأنماط ARM64 مُتحققة",
  keywords: ["Fortnite", "Android", "ARM64", "Erbium", "Reverse Engineering", "UFunction", "Offsets"],
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="ar" dir="rtl" suppressHydrationWarning>
      <body
        className={`${geistSans.variable} ${geistMono.variable} antialiased bg-background text-foreground`}
      >
        {children}
        <Toaster />
      </body>
    </html>
  );
}
